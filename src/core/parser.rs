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

struct StackFrame {
    key: String,
    node: Node,
    accumulated_text: String,
}

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
        let mut stack: Vec<StackFrame> = Vec::new();
        let mut current_qualifiers: Vec<Qualifier> = Vec::new();
        let mut description_depth: usize = 0;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    self.handle_start_element(
                        &e,
                        &mut root,
                        &mut stack,
                        &mut current_qualifiers,
                        &mut description_depth,
                    )?;
                }
                Ok(Event::Text(e)) => {
                    if description_depth > 0 {
                        if let Some(frame) = stack.last_mut() {
                            if let Node::Simple(_) = frame.node {
                                let raw_text = String::from_utf8_lossy(e.as_ref());
                                frame.accumulated_text.push_str(&raw_text);
                            }
                        }
                    }
                }
                Ok(Event::GeneralRef(e)) => {
                    if description_depth > 0 {
                        if let Some(frame) = stack.last_mut() {
                            if let Node::Simple(_) = frame.node {
                                let name = String::from_utf8_lossy(e.as_ref());
                                frame.accumulated_text.push('&');
                                frame.accumulated_text.push_str(&name);
                                frame.accumulated_text.push(';');
                            }
                        }
                    }
                }
                Ok(Event::End(e)) => {
                    self.handle_end_element(&e, &mut root, &mut stack, &mut description_depth)?;
                }
                Ok(Event::Empty(e)) => {
                    self.handle_empty_element(
                        &e,
                        &mut root,
                        &mut stack,
                        &mut current_qualifiers,
                        description_depth,
                    )?;
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
        stack: &mut [StackFrame],
        key: String,
        node: Node,
    ) {
        if let Some(frame) = stack.last_mut() {
            let parent = &mut frame.node;
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

    /// Make an array node at `frame` with element `name`.
    /// This will replace the frame node.
    fn make_array_node(name: &str, frame: &mut StackFrame) {
        use crate::core::node::{ArrayNode, ArrayType};

        let parent = &mut frame.node;
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

    /// Handle Start element event during RDF parsing
    fn handle_start_element(
        &mut self,
        e: &quick_xml::events::BytesStart<'_>,
        root: &mut StructureNode,
        stack: &mut Vec<StackFrame>,
        current_qualifiers: &mut Vec<Qualifier>,
        description_depth: &mut usize,
    ) -> XmpResult<()> {
        let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
        let attrs = Self::collect_attributes(e);
        self.process_attributes(&attrs, current_qualifiers);

        // Handle RDF Description
        if self.is_description_element(&name) {
            *description_depth += 1;
            if *description_depth > 1 {
                if let Some(frame) = stack.last_mut() {
                    self.populate_node_from_attributes(
                        &attrs,
                        &mut frame.node,
                        current_qualifiers,
                    )?;
                }
            } else {
                self.handle_description_attributes(&attrs, root, current_qualifiers)?;
            }
        }
        // Handle RDF containers (Seq, Bag, Alt)
        else if *description_depth > 0 && self.is_array_container(&name) {
            if let Some(frame) = stack.last_mut() {
                Self::make_array_node(&name, frame);
            }
        }
        // Handle li (list item) - add to current array
        else if *description_depth > 0 && self.is_li_element(&name) {
            let mut node = Node::simple("");
            if let Node::Simple(sn) = &mut node {
                sn.qualifiers.extend(current_qualifiers.clone());
            }
            self.populate_node_from_attributes(&attrs, &mut node, current_qualifiers)?;
            stack.push(StackFrame {
                key: "".to_string(),
                node,
                accumulated_text: String::new(),
            });
        } else if *description_depth > 0 && !self.is_rdf_element(&name) {
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
            self.populate_node_from_attributes(&attrs, &mut node, current_qualifiers)?;
            stack.push(StackFrame {
                key,
                node,
                accumulated_text: String::new(),
            });
        }
        Ok(())
    }

    /// Handle End element event during RDF parsing
    fn handle_end_element(
        &self,
        e: &quick_xml::events::BytesEnd<'_>,
        root: &mut StructureNode,
        stack: &mut Vec<StackFrame>,
        description_depth: &mut usize,
    ) -> XmpResult<()> {
        let name = String::from_utf8_lossy(e.name().as_ref()).to_string();

        if self.is_description_element(&name) {
            *description_depth = description_depth.saturating_sub(1);
        } else if *description_depth > 0
            && !self.is_array_container(&name)
            && !self.is_rdf_element(&name)
        {
            if let Some(frame) = stack.pop() {
                let StackFrame {
                    key,
                    mut node,
                    accumulated_text,
                } = frame;
                if let Node::Simple(ref mut simple) = node {
                    if !accumulated_text.is_empty() {
                        let text = match unescape(&accumulated_text) {
                            Ok(unescaped) => unescaped.to_string(),
                            Err(_) => accumulated_text.to_string(),
                        };
                        simple.value = text.trim().to_string();
                    }
                }
                Self::insert_node_into_parent(root, stack, key, node);
            }
        }
        Ok(())
    }

    /// Handle Empty element event during RDF parsing
    fn handle_empty_element(
        &mut self,
        e: &quick_xml::events::BytesStart<'_>,
        root: &mut StructureNode,
        stack: &mut [StackFrame],
        current_qualifiers: &mut Vec<Qualifier>,
        description_depth: usize,
    ) -> XmpResult<()> {
        let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
        let attrs = Self::collect_attributes_empty(e);
        self.process_attributes(&attrs, current_qualifiers);

        // Handle RDF Description
        if self.is_description_element(&name) {
            if description_depth > 0 {
                if let Some(frame) = stack.last_mut() {
                    self.populate_node_from_attributes(
                        &attrs,
                        &mut frame.node,
                        current_qualifiers,
                    )?;
                }
            } else {
                self.handle_description_attributes(&attrs, root, current_qualifiers)?;
            }
        } else if description_depth > 0 && !self.is_rdf_element(&name) {
            if self.is_array_container(&name) {
                if let Some(frame) = stack.last_mut() {
                    Self::make_array_node(&name, frame);
                }
            } else {
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
                self.populate_node_from_attributes(&attrs, &mut node, current_qualifiers)?;

                if name == "li" || name.ends_with(":li") {
                    Self::insert_node_into_parent(root, stack, "".to_string(), node);
                } else {
                    Self::insert_node_into_parent(root, stack, key, node);
                }
            }
        }
        Ok(())
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
        let colon_pos = name.find(':');
        let (prefix, local_name) = if let Some(pos) = colon_pos {
            (&name[..pos], &name[pos + 1..])
        } else {
            ("", name)
        };

        if local_name != "Description" {
            return false;
        }

        if let Some(ns_uri) = self.namespaces.get_uri(prefix) {
            ns_uri == crate::core::namespace::ns::RDF
        } else {
            prefix == "rdf" || (prefix.is_empty() && name == "Description")
        }
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

    /// Populate a node's fields from XML attributes if they represent properties.
    /// If the node is Simple and attributes are present, it will be converted to Structure.
    fn populate_node_from_attributes(
        &self,
        attrs: &[(String, String)],
        node: &mut Node,
        qualifiers: &[Qualifier],
    ) -> XmpResult<()> {
        let has_properties = attrs.iter().any(|(k, _)| {
            if self.should_skip_attribute(k) {
                return false;
            }
            if let Some(pos) = k.find(':') {
                let prefix = &k[..pos];
                self.namespaces.get_uri(prefix).is_some()
            } else {
                false
            }
        });

        if !has_properties {
            return Ok(());
        }

        // Convert Node to Structure if it is not already.
        if !node.is_structure() {
            let mut struct_node = StructureNode::new();
            match node {
                Node::Simple(sn) => {
                    struct_node.qualifiers = std::mem::take(&mut sn.qualifiers);
                }
                Node::Array(sn) => {
                    struct_node.qualifiers = std::mem::take(&mut sn.qualifiers);
                }
                _ => {}
            }
            *node = Node::Structure(struct_node);
        }

        if let Node::Structure(ref mut struct_node) = node {
            self.handle_description_attributes(attrs, struct_node, qualifiers)?;
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
            || attr_name == "rdf:parseType"
            || attr_name.ends_with(":parseType")
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

    #[test]
    fn test_parse_attributes_on_li() {
        let mut parser = XmpParser::new();
        let xml = r#"
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:xmpMM="http://ns.adobe.com/xap/1.0/mm/"
         xmlns:stEvt="http://ns.adobe.com/xap/1.0/sType/ResourceEvent#">
  <rdf:Description rdf:about="">
    <xmpMM:History>
      <rdf:Seq>
        <rdf:li stEvt:action="saved"/>
      </rdf:Seq>
    </xmpMM:History>
  </rdf:Description>
</rdf:RDF>"#;

        let root = parser.parse_rdf(xml).unwrap();
        let xmp_mm_uri = "http://ns.adobe.com/xap/1.0/mm/";
        let st_evt_uri = "http://ns.adobe.com/xap/1.0/sType/ResourceEvent#";

        let history_key = format!("{}:History", xmp_mm_uri);
        let history = root.get_field(&history_key).unwrap().as_array().unwrap();
        assert_eq!(history.len(), 1);

        let li = history.get(0).unwrap().as_structure().unwrap();
        let action_key = format!("{}:action", st_evt_uri);
        let action = li.get_field(&action_key).unwrap().as_simple().unwrap();
        assert_eq!(action.value, "saved");
    }

    #[test]
    fn test_parse_empty_array() {
        let mut parser = XmpParser::new();
        let xml = r#"
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:xmpMM="http://ns.adobe.com/xap/1.0/mm/">
  <rdf:Description rdf:about="">
    <xmpMM:History>
      <rdf:Seq/>
    </xmpMM:History>
  </rdf:Description>
</rdf:RDF>"#;

        let root = parser.parse_rdf(xml).unwrap();
        let xmp_mm_uri = "http://ns.adobe.com/xap/1.0/mm/";

        let history_key = format!("{}:History", xmp_mm_uri);
        let history = root
            .get_field(&history_key)
            .unwrap()
            .as_array()
            .expect("History is not an array");
        assert_eq!(history.len(), 0);
    }

    #[test]
    fn test_parse_attributes_on_nested_element() {
        let mut parser = XmpParser::new();
        let xml = r#"
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:Container="http://ns.google.com/photos/1.0/container/"
         xmlns:Item="http://ns.google.com/photos/1.0/container/item/">
  <rdf:Description rdf:about="">
    <Container:Directory>
      <rdf:Seq>
        <rdf:li rdf:parseType="Resource">
          <Container:Item Item:Semantic="Primary"/>
        </rdf:li>
      </rdf:Seq>
    </Container:Directory>
  </rdf:Description>
</rdf:RDF>"#;

        let root = parser.parse_rdf(xml).unwrap();
        let container_uri = "http://ns.google.com/photos/1.0/container/";
        let item_uri = "http://ns.google.com/photos/1.0/container/item/";

        let directory_key = format!("{}:Directory", container_uri);
        let directory = root.get_field(&directory_key).unwrap().as_array().unwrap();
        assert_eq!(directory.len(), 1);

        let li = directory.get(0).unwrap().as_structure().unwrap();
        let item_key = format!("{}:Item", container_uri);
        let item = li.get_field(&item_key).unwrap().as_structure().unwrap();

        let semantic_key = format!("{}:Semantic", item_uri);
        let semantic = item.get_field(&semantic_key).unwrap().as_simple().unwrap();
        assert_eq!(semantic.value, "Primary");
    }

    #[test]
    fn test_exif_ex_namespace() {
        let mut parser = XmpParser::new();
        let xml = r#"
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:exifEX="http://cipa.jp/exif/1.0/">
  <rdf:Description rdf:about=""
                   exifEX:LensMake="Google"/>
</rdf:RDF>"#;

        let root = parser.parse_rdf(xml).unwrap();
        let lens_make_key = "http://cipa.jp/exif/1.0/:LensMake";
        assert!(root.has_field(lens_make_key));

        let lens_make = root.get_field(lens_make_key).unwrap().as_simple().unwrap();
        assert_eq!(lens_make.value, "Google");
    }
}
