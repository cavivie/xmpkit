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
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();
        let mut root = StructureNode::new();
        let mut stack: Vec<StructureNode> = Vec::new();
        let mut current_path: Vec<String> = Vec::new();
        let mut current_qualifiers: Vec<Qualifier> = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let attrs = Self::collect_attributes(&e);
                    self.process_attributes(&attrs, &mut current_qualifiers);

                    // Handle RDF Description
                    if self.is_description_element(&name) {
                        self.handle_description_attributes(&attrs, &mut root, &current_qualifiers)?;
                    }
                    // Handle RDF containers (Seq, Bag, Alt)
                    else if self.is_array_container(&name) {
                        self.handle_array_container(&name, &mut root, &mut current_path)?;
                    }
                    // Handle li (list item) - add to current array
                    // Note: li elements don't push to current_path, they add items to the current array
                    else if self.is_li_element(&name) {
                        // Extract qualifiers (xml:lang) for the li element
                        // These will be used when we encounter the text content
                        // Don't push to current_path - we're already in an array context
                    } else if !self.is_rdf_element(&name) {
                        self.push_element_to_path(&name, &mut current_path);
                    }
                }
                Ok(Event::Text(e)) => {
                    // Decode XML entities (e.g., &quot; -> ")
                    let raw_text = String::from_utf8_lossy(e.as_ref());
                    let text = match unescape(&raw_text) {
                        Ok(unescaped) => unescaped.to_string(),
                        Err(_) => raw_text.to_string(),
                    };

                    let trimmed_text = text.trim();
                    if trimmed_text.is_empty() {
                        continue;
                    }

                    // Check if we're inside an array (current_path ends with "__array__")
                    let Some(last_path) = current_path.last() else {
                        continue;
                    };

                    if last_path == "__array__" {
                        // We're in an array, add item to the array
                        self.handle_array_text_item(
                            &mut root,
                            &current_path,
                            trimmed_text,
                            &current_qualifiers,
                        )?;
                    } else {
                        // Not in array, set as field
                        self.handle_simple_text_item(
                            &mut root,
                            &mut stack,
                            last_path,
                            trimmed_text,
                            &current_qualifiers,
                        )?;
                    }
                }
                Ok(Event::End(e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                    if name == "Seq"
                        || name == "Bag"
                        || name == "Alt"
                        || name.ends_with(":Seq")
                        || name.ends_with(":Bag")
                        || name.ends_with(":Alt")
                    {
                        // End of array container, pop "__array__" marker
                        if current_path.last() == Some(&"__array__".to_string()) {
                            current_path.pop();
                        }
                    } else if name != "Description"
                        && !name.ends_with(":Description")
                        && name != "RDF"
                        && !name.ends_with(":RDF")
                        && name != "li"
                        && !name.ends_with(":li")
                    {
                        current_path.pop();
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(XmpError::ParseError(format!("XML parsing error: {}", e)));
                }
                Ok(Event::Empty(e)) => {
                    // Handle empty/self-closing elements the same way as Start events
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let attrs = Self::collect_attributes_empty(&e);
                    self.process_attributes(&attrs, &mut current_qualifiers);

                    // Handle RDF Description
                    if self.is_description_element(&name) {
                        self.handle_description_attributes(&attrs, &mut root, &current_qualifiers)?;
                    }
                }
                _ => {}
            }
            buf.clear();
        }

        Ok(root)
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

    /// Handle array container (Seq, Bag, Alt)
    fn handle_array_container(
        &self,
        name: &str,
        root: &mut StructureNode,
        current_path: &mut Vec<String>,
    ) -> XmpResult<()> {
        use crate::core::node::{ArrayNode, ArrayType};

        let array_type = if name.contains("Seq") {
            ArrayType::Ordered
        } else if name.contains("Bag") {
            ArrayType::Unordered
        } else {
            ArrayType::Alternative
        };

        let array_node = ArrayNode::new(array_type);
        let array_node_wrapper = Node::Array(array_node);

        // Set array to the current path (property name)
        let Some(last_path) = current_path.last() else {
            return Ok(());
        };

        let full_path = self.resolve_path_to_full_format(last_path);
        root.set_field(full_path.clone(), array_node_wrapper);

        // Mark that we're in an array for adding items
        // Store the full path so we can reference it later
        current_path.push("__array__".to_string());
        Ok(())
    }

    /// Push element name to current path, resolving namespace if needed
    fn push_element_to_path(&self, name: &str, current_path: &mut Vec<String>) {
        let Some(colon_pos) = name.find(':') else {
            current_path.push(name.to_string());
            return;
        };

        let ns_prefix = &name[..colon_pos];
        let prop_name = &name[colon_pos + 1..];

        if let Some(ns_uri) = self.namespaces.get_uri(ns_prefix) {
            let full_path = format!("{}:{}", ns_uri, prop_name);
            current_path.push(full_path);
        } else {
            current_path.push(name.to_string());
        }
    }

    /// Resolve path to full format (namespace URI:property)
    fn resolve_path_to_full_format(&self, path: &str) -> String {
        if path.starts_with("http://") {
            return path.to_string();
        }

        let Some(colon_pos) = path.find(':') else {
            return path.to_string();
        };

        let ns_prefix = &path[..colon_pos];
        let prop_name = &path[colon_pos + 1..];

        self.namespaces
            .get_uri(ns_prefix)
            .map(|ns_uri| format!("{}:{}", ns_uri, prop_name))
            .unwrap_or_else(|| path.to_string())
    }

    /// Handle text item in an array context
    fn handle_array_text_item(
        &self,
        root: &mut StructureNode,
        current_path: &[String],
        text: &str,
        qualifiers: &[Qualifier],
    ) -> XmpResult<()> {
        // Get the property path (the element before "__array__")
        if current_path.len() < 2 {
            return Ok(());
        }

        let prop_path = &current_path[current_path.len() - 2];
        let full_path = prop_path.clone();

        let Some(Node::Array(ref mut arr)) = root.get_field_mut(&full_path) else {
            return Ok(());
        };

        let mut simple_node = Node::simple(text);
        // Add qualifiers to the node
        if let Node::Simple(ref mut sn) = simple_node {
            for qual in qualifiers {
                sn.add_qualifier(qual.clone());
            }
        }
        arr.append(simple_node);
        Ok(())
    }

    /// Handle simple text item (not in array)
    fn handle_simple_text_item(
        &self,
        root: &mut StructureNode,
        stack: &mut [StructureNode],
        last_path: &str,
        text: &str,
        qualifiers: &[Qualifier],
    ) -> XmpResult<()> {
        // Resolve path to full format
        let path_to_check = if last_path.starts_with("http://") {
            last_path.to_string()
        } else if let Some(colon_pos) = last_path.find(':') {
            let ns_prefix = &last_path[..colon_pos];
            let prop_name = &last_path[colon_pos + 1..];
            self.namespaces
                .get_uri(ns_prefix)
                .map(|ns_uri| format!("{}:{}", ns_uri, prop_name))
                .unwrap_or_else(|| last_path.to_string())
        } else {
            last_path.to_string()
        };

        // Only set as simple node if there's no existing array
        let has_array = root
            .get_field(&path_to_check)
            .map(|n| n.is_array())
            .unwrap_or(false);
        if has_array {
            return Ok(());
        }

        let mut simple_node = Node::simple(text);
        // Add qualifiers to the node
        if let Node::Simple(ref mut sn) = simple_node {
            for qual in qualifiers {
                sn.add_qualifier(qual.clone());
            }
        }

        if let Some(parent) = stack.last_mut() {
            parent.set_field(path_to_check.clone(), simple_node);
        } else {
            root.set_field(path_to_check.clone(), simple_node);
        }
        Ok(())
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
}
