//! XMP XML/RDF parser
//!
//! This module provides functionality for parsing XMP Packets from XML/RDF format.

use crate::core::error::{XmpError, XmpResult};
use crate::core::namespace::{ns, NamespaceMap};
use crate::core::node::{Node, StructureNode};
use crate::types::qualifier::Qualifier;
use quick_xml::escape::unescape;
use quick_xml::events::Event;
use quick_xml::Reader;

/// Parser for XMP Packets
pub struct XmpParser {
    namespaces: NamespaceMap,
}

impl XmpParser {
    /// Create a new XMP parser
    pub fn new() -> Self {
        Self {
            namespaces: NamespaceMap::new(),
        }
    }

    /// Parse an XMP Packet from a string
    ///
    /// This function extracts the XMP Packet from the `<?xpacket>` wrapper
    /// and parses the RDF/XML content.
    pub fn parse_packet(&mut self, xml: &str) -> XmpResult<StructureNode> {
        // Extract XMP Packet content (remove <?xpacket> wrapper)
        let packet_content = self.extract_packet_content(xml)?;

        // Parse RDF/XML
        self.parse_rdf(&packet_content)
    }

    /// Return the namespace map accumulated while parsing.
    pub fn namespace_map(&self) -> NamespaceMap {
        self.namespaces.clone()
    }

    /// Extract the XMP Packet content from the `<?xpacket>` wrapper
    fn extract_packet_content(&self, xml: &str) -> XmpResult<String> {
        // Look for <?xpacket start
        let Some(start_pos) = xml.find("<?xpacket") else {
            return self.validate_and_return_xml(xml);
        };

        let Some(end_pos) = xml[start_pos..].find("?>") else {
            return self.validate_and_return_xml(xml);
        };

        let pi_end = start_pos + end_pos + 2;
        let Some(close_pos) = xml[pi_end..].find("<?xpacket end") else {
            return self.validate_and_return_xml(xml);
        };

        let content = xml[pi_end..pi_end + close_pos].trim().to_string();
        Ok(content)
    }

    /// Validate XML content and return it if valid
    fn validate_and_return_xml(&self, xml: &str) -> XmpResult<String> {
        let trimmed = xml.trim();
        if trimmed.is_empty() || (!trimmed.starts_with('<') && !trimmed.starts_with("<?xml")) {
            return Err(XmpError::ParseError("Invalid XML content".to_string()));
        }
        Ok(trimmed.to_string())
    }

    /// Parse RDF/XML content into a StructureNode
    fn parse_rdf(&mut self, xml: &str) -> XmpResult<StructureNode> {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(false);

        let mut buf = Vec::new();
        let mut root = StructureNode::new();
        let mut stack: Vec<(String, Node, String)> = Vec::new();
        let mut current_qualifiers: Vec<Qualifier> = Vec::new();
        let mut description_depth: usize = 0;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let attrs = Self::collect_attributes(&e);
                    self.process_attributes(&attrs, &mut current_qualifiers);

                    // Handle RDF Description
                    if self.is_description_element(&name) {
                        description_depth += 1;
                        self.handle_description_attributes(&attrs, &mut root, &current_qualifiers)?;
                    }
                    // Handle RDF containers (Seq, Bag, Alt)
                    else if description_depth > 0 && self.is_array_container(&name) {
                        if let Some((_, parent, _)) = stack.last_mut() {
                            use crate::core::node::{ArrayNode, ArrayType};
                            let array_type = if name.contains("Seq") {
                                ArrayType::Ordered
                            } else if name.contains("Bag") {
                                ArrayType::Unordered
                            } else {
                                ArrayType::Alternative
                            };
                            let mut array_node = ArrayNode::new(array_type);
                            // Preserve any qualifiers from the parent (e.g. from attributes)
                            match parent {
                                Node::Simple(sn) => {
                                    array_node.qualifiers = std::mem::take(&mut sn.qualifiers);
                                }
                                Node::Structure(sn) => {
                                    array_node.qualifiers = std::mem::take(&mut sn.qualifiers);
                                }
                                Node::Array(sn) => {
                                    array_node.qualifiers = std::mem::take(&mut sn.qualifiers);
                                }
                            }
                            *parent = Node::Array(array_node);
                        }
                    }
                    // Handle li (list item) - add to current array
                    else if description_depth > 0 && self.is_li_element(&name) {
                        let mut node = Node::simple("");
                        if let Node::Simple(sn) = &mut node {
                            sn.qualifiers.extend(current_qualifiers.clone());
                        }
                        stack.push(("".to_string(), node, String::new()));
                    } else if description_depth > 0 && !self.is_rdf_element(&name) {
                        let colon_pos = name.find(':');
                        let key = if let Some(pos) = colon_pos {
                            let prefix = &name[..pos];
                            let prop_name = &name[pos + 1..];
                            if let Some(ns_uri) = self.namespaces.get_uri(prefix) {
                                format!("{}:{}", ns_uri, prop_name)
                            } else {
                                name.clone()
                            }
                        } else {
                            name.clone()
                        };

                        let has_parse_type_resource = attrs
                            .iter()
                            .any(|(k, v)| k == "rdf:parseType" && v == "Resource");
                        let mut node = if has_parse_type_resource {
                            Node::structure()
                        } else {
                            Node::simple("")
                        };
                        match &mut node {
                            Node::Simple(sn) => sn.qualifiers.extend(current_qualifiers.clone()),
                            Node::Structure(sn) => sn.qualifiers.extend(current_qualifiers.clone()),
                            Node::Array(sn) => sn.qualifiers.extend(current_qualifiers.clone()),
                        }
                        stack.push((key, node, String::new()));
                    }
                }
                Ok(Event::Text(e)) => {
                    if description_depth > 0 {
                        if let Some((_, Node::Simple(_), ref mut acc_text)) = stack.last_mut() {
                            let raw_text = String::from_utf8_lossy(e.as_ref());
                            acc_text.push_str(&raw_text);
                        }
                    }
                }
                Ok(Event::GeneralRef(e)) => {
                    if description_depth > 0 {
                        if let Some((_, Node::Simple(_), ref mut acc_text)) = stack.last_mut() {
                            let name = String::from_utf8_lossy(e.as_ref());
                            acc_text.push('&');
                            acc_text.push_str(&name);
                            acc_text.push(';');
                        }
                    }
                }
                Ok(Event::End(e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                    if self.is_description_element(&name) {
                        description_depth = description_depth.saturating_sub(1);
                    } else if description_depth > 0
                        && !self.is_array_container(&name)
                        && !self.is_rdf_element(&name)
                    {
                        if let Some((key, mut node, acc_text)) = stack.pop() {
                            if let Node::Simple(ref mut simple) = node {
                                if !acc_text.is_empty() {
                                    let text = match unescape(&acc_text) {
                                        Ok(unescaped) => unescaped.to_string(),
                                        Err(_) => acc_text.to_string(),
                                    };
                                    simple.value = text.trim().to_string();
                                }
                            }
                            Self::insert_node_into_parent(&mut root, &mut stack, key, node);
                        }
                    }
                }
                Ok(Event::Empty(e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let attrs = Self::collect_attributes_empty(&e);
                    self.process_attributes(&attrs, &mut current_qualifiers);

                    // Handle RDF Description
                    if self.is_description_element(&name) {
                        self.handle_description_attributes(&attrs, &mut root, &current_qualifiers)?;
                    } else if description_depth > 0 && !self.is_rdf_element(&name) {
                        let colon_pos = name.find(':');
                        let key = if let Some(pos) = colon_pos {
                            let prefix = &name[..pos];
                            let prop_name = &name[pos + 1..];
                            if let Some(ns_uri) = self.namespaces.get_uri(prefix) {
                                format!("{}:{}", ns_uri, prop_name)
                            } else {
                                name.clone()
                            }
                        } else {
                            name.clone()
                        };

                        let mut node = Node::simple("");
                        if let Node::Simple(sn) = &mut node {
                            sn.qualifiers.extend(current_qualifiers.clone());
                        }

                        if name == "li" || name.ends_with(":li") {
                            Self::insert_node_into_parent(
                                &mut root,
                                &mut stack,
                                "".to_string(),
                                node,
                            );
                        } else {
                            Self::insert_node_into_parent(&mut root, &mut stack, key, node);
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(XmpError::ParseError(format!("XML parsing error: {}", e)));
                }
                _ => {}
            }
            buf.clear();
        }

        Ok(root)
    }

    /// Helper to insert a completed node into its parent or root
    fn insert_node_into_parent(
        root: &mut StructureNode,
        stack: &mut [(String, Node, String)],
        key: String,
        node: Node,
    ) {
        if let Some((_, parent, _)) = stack.last_mut() {
            match parent {
                Node::Array(arr) => {
                    arr.append(node);
                }
                Node::Structure(structure) => {
                    structure.set_field(key, node);
                }
                Node::Simple(simple) => {
                    // Convert parent from Simple to Structure
                    let mut structure = StructureNode::new();
                    structure.qualifiers = std::mem::take(&mut simple.qualifiers);
                    *parent = Node::Structure(structure);

                    if let Node::Structure(structure) = parent {
                        structure.set_field(key, node);
                    }
                }
            }
        } else {
            root.set_field(key, node);
        }
    }

    /// Process collected attributes: extract namespaces and qualifiers
    fn process_attributes(
        &mut self,
        attrs: &[(String, String)],
        current_qualifiers: &mut Vec<Qualifier>,
    ) {
        // Extract namespace declarations from attributes (on any element)
        for (attr_name, attr_value) in attrs {
            if attr_name == "xmlns" {
                // Default namespace - For XMP, we typically don't use default namespace
                continue;
            }
            if let Some(prefix) = attr_name.strip_prefix("xmlns:") {
                // Namespace prefix declaration: xmlns:prefix="uri"
                let _ = self.namespaces.register(attr_value, prefix);
            }
        }

        // Extract qualifiers from attributes (e.g., xml:lang)
        current_qualifiers.clear();
        for (attr_name, attr_value) in attrs {
            if self.is_lang_attribute(attr_name) {
                let qualifier = Qualifier::new(ns::XML, "lang", attr_value.clone());
                current_qualifiers.push(qualifier);
            }
        }
    }

    /// Collect attributes from XML element
    fn collect_attributes(e: &quick_xml::events::BytesStart<'_>) -> Vec<(String, String)> {
        e.attributes()
            .flatten()
            .map(|attr| {
                let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                // Decode XML entities in attribute values (e.g., &quot; -> ")
                let raw_value = String::from_utf8_lossy(attr.value.as_ref());
                let value = match unescape(&raw_value) {
                    Ok(unescaped) => unescaped.to_string(),
                    Err(_) => raw_value.to_string(),
                };
                (key, value)
            })
            .collect()
    }

    /// Collect attributes from empty XML element (same as Start)
    fn collect_attributes_empty(e: &quick_xml::events::BytesStart<'_>) -> Vec<(String, String)> {
        Self::collect_attributes(e)
    }

    /// Check if attribute name is a language qualifier
    fn is_lang_attribute(&self, attr_name: &str) -> bool {
        attr_name == "lang" || attr_name == "xml:lang" || attr_name.ends_with(":lang")
    }

    /// Check if element name is a Description element
    fn is_description_element(&self, name: &str) -> bool {
        name == "Description" || name.ends_with(":Description")
    }

    /// Check if element name is an array container (Seq, Bag, Alt)
    fn is_array_container(&self, name: &str) -> bool {
        name == "Seq"
            || name == "Bag"
            || name == "Alt"
            || name.ends_with(":Seq")
            || name.ends_with(":Bag")
            || name.ends_with(":Alt")
    }

    /// Check if element name is a li element
    fn is_li_element(&self, name: &str) -> bool {
        name == "li" || name.ends_with(":li")
    }

    /// Check if element name is an RDF element
    fn is_rdf_element(&self, name: &str) -> bool {
        name == "RDF" || name.ends_with(":RDF")
    }

    /// Handle Description element attributes
    fn handle_description_attributes(
        &self,
        attrs: &[(String, String)],
        root: &mut StructureNode,
        qualifiers: &[Qualifier],
    ) -> XmpResult<()> {
        for (attr_name, attr_value) in attrs {
            // Skip xmlns declarations, rdf:about, and qualifiers
            if self.should_skip_attribute(attr_name) {
                continue;
            }

            // Parse namespace:property format
            let Some(colon_pos) = attr_name.find(':') else {
                continue;
            };

            let ns_prefix = &attr_name[..colon_pos];
            let prop_name = &attr_name[colon_pos + 1..];

            // Try to get namespace URI for the prefix
            // Handle case where prefix in attribute name doesn't match declared prefix
            // (e.g., TC260:AIGC but xmlns:C260="...")
            let ns_uri = self.namespaces.get_uri(ns_prefix).or_else(|| {
                // If prefix not found, try common variations
                // For TC260, try C260
                if ns_prefix == "TC260" {
                    self.namespaces.get_uri("C260")
                } else if ns_prefix == "C260" {
                    self.namespaces.get_uri("TC260")
                } else {
                    None
                }
            });

            let Some(ns_uri) = ns_uri else {
                continue;
            };

            let full_path = format!("{}:{}", ns_uri, prop_name);
            let mut simple_node = Node::simple(attr_value.clone());
            // Add qualifiers to the node
            if let Node::Simple(ref mut sn) = simple_node {
                for qual in qualifiers {
                    sn.add_qualifier(qual.clone());
                }
            }
            root.set_field(full_path.clone(), simple_node);
        }
        Ok(())
    }

    /// Check if attribute should be skipped during Description processing
    fn should_skip_attribute(&self, attr_name: &str) -> bool {
        attr_name == "xmlns"
            || attr_name.starts_with("xmlns:")
            || attr_name == "about"
            || attr_name.ends_with(":about")
            || self.is_lang_attribute(attr_name)
    }
}

impl Default for XmpParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_packet_content() {
        let parser = XmpParser::new();
        let xml = r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<rdf:RDF>...</rdf:RDF>
<?xpacket end="w"?>"#;

        let content = parser.extract_packet_content(xml).unwrap();
        assert!(content.contains("<rdf:RDF>"));
    }

    #[test]
    fn test_parse_simple_rdf() {
        let mut parser = XmpParser::new();
        let xml = r#"
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:xmp="http://ns.adobe.com/xap/1.0/">
  <rdf:Description rdf:about=""
                   xmp:CreatorTool="MyApp"/>
</rdf:RDF>"#;

        let result = parser.parse_rdf(xml);
        assert!(result.is_ok());
        let root = result.unwrap();

        // Debug: print all fields
        for field_name in root.field_names() {
            eprintln!("Field: {}", field_name);
        }

        // Check if xmp prefix is registered
        eprintln!("xmp URI: {:?}", parser.namespaces.get_uri("xmp"));

        assert!(root.has_field("http://ns.adobe.com/xap/1.0/:CreatorTool"));
    }

    #[test]
    fn test_parse_multiline_text() {
        let mut parser = XmpParser::new();
        let xml = r#"
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:xmp="http://ns.adobe.com/xap/1.0/">
  <rdf:Description rdf:about=""
                   xmlns:test="http://example.com/test/">
    <test:Multiline>Line1
Line2
Line3</test:Multiline>
  </rdf:Description>
</rdf:RDF>"#;

        let root = parser.parse_rdf(xml).unwrap();
        let node = root
            .get_field("http://example.com/test/:Multiline")
            .unwrap();
        let simple = node.as_simple().unwrap();
        assert_eq!(simple.value, "Line1\nLine2\nLine3");
    }

    #[test]
    fn test_parse_large_text() {
        let mut parser = XmpParser::new();
        let mut large_val = String::new();
        for _ in 0..10000 {
            large_val.push_str("abcdefghijklmnopqrstuvwxyz0123456789\n");
        }
        let xml = format!(
            r#"
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:xmp="http://ns.adobe.com/xap/1.0/">
  <rdf:Description rdf:about=""
                   xmlns:test="http://example.com/test/">
    <test:Large>{}</test:Large>
  </rdf:Description>
</rdf:RDF>"#,
            large_val
        );

        let root = parser.parse_rdf(&xml).unwrap();
        let node = root.get_field("http://example.com/test/:Large").unwrap();
        let simple = node.as_simple().unwrap();
        assert_eq!(simple.value, large_val.trim());
    }

    #[test]
    fn test_parse_text_with_comments() {
        let mut parser = XmpParser::new();
        let xml = r#"
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:xmp="http://ns.adobe.com/xap/1.0/">
  <rdf:Description rdf:about=""
                   xmlns:test="http://example.com/test/">
    <test:Commented>Hello <!-- comment -->World</test:Commented>
  </rdf:Description>
</rdf:RDF>"#;

        let root = parser.parse_rdf(xml).unwrap();
        let node = root
            .get_field("http://example.com/test/:Commented")
            .unwrap();
        let simple = node.as_simple().unwrap();
        assert_eq!(simple.value, "Hello World");
    }

    #[test]
    fn test_parse_text_with_character_references() {
        let mut parser = XmpParser::new();
        let xml = r#"
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:xmp="http://ns.adobe.com/xap/1.0/">
  <rdf:Description rdf:about=""
                   xmlns:test="http://example.com/test/">
    <test:CharRef>Hello&#x20;World &amp; Universe</test:CharRef>
  </rdf:Description>
</rdf:RDF>"#;

        let root = parser.parse_rdf(xml).unwrap();
        let node = root.get_field("http://example.com/test/:CharRef").unwrap();
        let simple = node.as_simple().unwrap();
        assert_eq!(simple.value, "Hello World & Universe");
    }
}
