//! XMP XML/RDF serializer
//!
//! This module provides functionality for serializing XMP metadata to XML/RDF format.

use crate::core::error::{XmpError, XmpResult};
use crate::core::namespace::{ns, NamespaceMap};
use crate::core::node::{ArrayNode, ArrayType, Node, StructureNode};
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use std::io::Cursor;

/// Serializer for XMP Packets
pub struct XmpSerializer {
    namespaces: NamespaceMap,
}

impl XmpSerializer {
    /// Create a new XMP serializer
    pub fn new() -> Self {
        Self {
            namespaces: NamespaceMap::new(),
        }
    }

    /// Serialize a StructureNode to RDF/XML
    pub fn serialize_rdf(&self, root: &StructureNode) -> XmpResult<String> {
        let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 2);

        // Collect namespaces used in the metadata
        let mut used_namespaces = std::collections::HashMap::new();

        // Collect simple nodes as attributes and complex nodes as elements
        let mut simple_attrs = Vec::new();
        let mut complex_nodes = Vec::new();

        for (key, node) in &root.fields {
            if self.should_serialize_as_element(key, node) {
                complex_nodes.push((key.clone(), node.clone()));
            } else if let Some((prefix, prop_name, ns_uri)) = self.parse_path_with_namespace(key) {
                // Record namespace usage
                used_namespaces.insert(ns_uri.clone(), prefix.clone());

                if let Node::Simple(simple) = node {
                    simple_attrs.push((format!("{}:{}", prefix, prop_name), simple.value.clone()));
                } else {
                    complex_nodes.push((key.clone(), node.clone()));
                }
            }
        }

        // Write RDF root element with namespaces
        let mut rdf_start = BytesStart::new("rdf:RDF");
        rdf_start.push_attribute(("xmlns:rdf", "http://www.w3.org/1999/02/22-rdf-syntax-ns#"));
        rdf_start.push_attribute(("xmlns:xmp", "http://ns.adobe.com/xap/1.0/"));
        rdf_start.push_attribute(("xmlns:dc", "http://purl.org/dc/elements/1.1/"));
        rdf_start.push_attribute(("xmlns:exif", "http://ns.adobe.com/exif/1.0/"));
        rdf_start.push_attribute(("xmlns:xml", ns::XML));

        // Add dynamically discovered namespaces
        for (ns_uri, prefix) in &used_namespaces {
            // Skip namespaces already declared above
            match ns_uri.as_str() {
                "http://www.w3.org/1999/02/22-rdf-syntax-ns#" => continue,
                "http://ns.adobe.com/xap/1.0/" => continue,
                "http://purl.org/dc/elements/1.1/" => continue,
                "http://ns.adobe.com/exif/1.0/" => continue,
                ns::XML => continue,
                _ => {
                    rdf_start
                        .push_attribute((format!("xmlns:{}", prefix).as_str(), ns_uri.as_str()));
                }
            }
        }

        writer.write_event(Event::Start(rdf_start))?;

        // Write Description element with attributes and nested elements
        let mut desc_start = BytesStart::new("rdf:Description");
        desc_start.push_attribute(("rdf:about", ""));

        // Add simple attributes to Description
        for (attr_name, attr_value) in &simple_attrs {
            desc_start.push_attribute((attr_name.as_str(), attr_value.as_str()));
        }

        // If there are no complex nodes, use Empty (self-closing) tag
        // Otherwise use Start/End tags
        if complex_nodes.is_empty() {
            writer.write_event(Event::Empty(desc_start))?;
        } else {
            writer.write_event(Event::Start(desc_start))?;

            // Serialize complex nodes as nested elements
            for (key, node) in &complex_nodes {
                self.serialize_node(&mut writer, key, node)?;
            }

            writer.write_event(Event::End(BytesEnd::new("rdf:Description")))?;
        }
        writer.write_event(Event::End(BytesEnd::new("rdf:RDF")))?;

        let result = writer.into_inner().into_inner();
        String::from_utf8(result)
            .map_err(|e| XmpError::SerializationError(format!("UTF-8 encoding error: {}", e)))
    }

    /// Parse a path in format "namespace_uri:property_name" into (prefix, property_name, namespace_uri)
    ///
    /// This function converts the internal path format (namespace URI:property) to
    /// the serialization format (prefix:property). It follows C++ SDK behavior:
    /// - First checks instance namespace map
    /// - Then checks global namespace registry
    /// - Returns None if namespace is not registered (does not infer prefix from URI)
    fn parse_path_with_namespace(&self, path: &str) -> Option<(String, String, String)> {
        // Find the last colon (to handle URIs that contain colons like http://...)
        let colon_pos = path.rfind(':')?;
        let ns_uri = &path[..colon_pos];
        let prop_name = &path[colon_pos + 1..];

        // Try to get prefix from instance namespace map first
        if let Some(prefix) = self.namespaces.get_prefix(ns_uri) {
            return Some((
                prefix.to_string(),
                prop_name.to_string(),
                ns_uri.to_string(),
            ));
        }

        // Fallback: check global namespace registry
        use crate::core::namespace::get_global_namespace_prefix;
        if let Some(prefix) = get_global_namespace_prefix(ns_uri) {
            return Some((prefix, prop_name.to_string(), ns_uri.to_string()));
        }

        // Namespace not registered - return None (following C++ SDK behavior)
        // In C++ SDK, unregistered namespaces would cause an error during serialization
        None
    }

    /// Parse a path in format "namespace_uri:property_name" into (prefix, property_name)
    /// This is a compatibility method that calls parse_path_with_namespace
    fn parse_path(&self, path: &str) -> Option<(String, String)> {
        self.parse_path_with_namespace(path)
            .map(|(prefix, prop_name, _)| (prefix, prop_name))
    }

    /// Serialize a node
    fn serialize_node(
        &self,
        writer: &mut Writer<Cursor<Vec<u8>>>,
        path: &str,
        node: &Node,
    ) -> XmpResult<()> {
        match node {
            Node::Simple(simple) => {
                self.serialize_simple_node(writer, path, simple)?;
            }
            Node::Array(array) => {
                self.serialize_array_node(writer, path, array)?;
            }
            Node::Structure(structure) => {
                self.serialize_structure_node(writer, path, structure)?;
            }
        }
        Ok(())
    }

    /// Serialize a simple node
    fn serialize_simple_node(
        &self,
        writer: &mut Writer<Cursor<Vec<u8>>>,
        path: &str,
        node: &crate::core::node::SimpleNode,
    ) -> XmpResult<()> {
        let (prefix, prop_name) = self
            .parse_path(path)
            .ok_or_else(|| XmpError::BadXPath(format!("Invalid path format: {}", path)))?;

        let elem_name = format!("{}:{}", prefix, prop_name);
        let mut elem_start = BytesStart::new(&elem_name);

        // Add qualifiers as attributes (e.g., xml:lang)
        self.add_lang_qualifier_attributes(&Node::Simple(node.clone()), &mut elem_start);

        writer.write_event(Event::Start(elem_start))?;
        writer.write_event(Event::Text(BytesText::new(&node.value)))?;
        writer.write_event(Event::End(BytesEnd::new(&elem_name)))?;

        Ok(())
    }

    /// Serialize an array node
    fn serialize_array_node(
        &self,
        writer: &mut Writer<Cursor<Vec<u8>>>,
        path: &str,
        node: &ArrayNode,
    ) -> XmpResult<()> {
        let (prefix, prop_name) = self
            .parse_path(path)
            .ok_or_else(|| XmpError::BadXPath(format!("Invalid path format: {}", path)))?;

        let container_name = match node.array_type {
            ArrayType::Ordered => "rdf:Seq",
            ArrayType::Unordered => "rdf:Bag",
            ArrayType::Alternative => "rdf:Alt",
        };

        // Write property element containing the container
        let prop_elem = format!("{}:{}", prefix, prop_name);
        writer.write_event(Event::Start(BytesStart::new(&prop_elem)))?;

        // Write container element
        writer.write_event(Event::Start(BytesStart::new(container_name)))?;

        // Write list items
        for item in &node.items {
            let mut li_start = BytesStart::new("rdf:li");
            self.add_lang_qualifier_attributes(item, &mut li_start);
            writer.write_event(Event::Start(li_start))?;

            self.serialize_array_item(writer, item)?;

            writer.write_event(Event::End(BytesEnd::new("rdf:li")))?;
        }

        writer.write_event(Event::End(BytesEnd::new(container_name)))?;
        writer.write_event(Event::End(BytesEnd::new(&prop_elem)))?;
        Ok(())
    }

    /// Serialize a structure node
    fn serialize_structure_node(
        &self,
        writer: &mut Writer<Cursor<Vec<u8>>>,
        path: &str,
        node: &StructureNode,
    ) -> XmpResult<()> {
        let (prefix, prop_name) = self
            .parse_path(path)
            .ok_or_else(|| XmpError::BadXPath(format!("Invalid path format: {}", path)))?;

        // Write property element containing the structure
        let prop_elem = format!("{}:{}", prefix, prop_name);
        writer.write_event(Event::Start(BytesStart::new(&prop_elem)))?;

        // Write structure as nested Description with rdf:parseType="Resource"
        let mut desc_start = BytesStart::new("rdf:Description");
        desc_start.push_attribute(("rdf:parseType", "Resource"));
        writer.write_event(Event::Start(desc_start))?;

        // Write fields
        for (key, value) in &node.fields {
            self.serialize_node(writer, key, value)?;
        }

        writer.write_event(Event::End(BytesEnd::new("rdf:Description")))?;
        writer.write_event(Event::End(BytesEnd::new(&prop_elem)))?;
        Ok(())
    }

    /// Check if a node should be serialized as an element (not attribute)
    fn should_serialize_as_element(&self, _key: &str, node: &Node) -> bool {
        let Node::Simple(simple) = node else {
            // Arrays and structures are always elements
            return true;
        };

        // Simple nodes with xml:lang qualifier must be elements
        simple
            .qualifiers
            .iter()
            .any(|q| q.namespace == ns::XML && q.name == "lang")
    }

    /// Add language qualifier attributes to an element
    fn add_lang_qualifier_attributes(&self, node: &Node, elem_start: &mut BytesStart) {
        let Node::Simple(simple) = node else {
            return;
        };

        for qualifier in &simple.qualifiers {
            if qualifier.namespace == ns::XML && qualifier.name == "lang" {
                elem_start.push_attribute(("xml:lang", qualifier.value.as_str()));
            }
        }
    }

    /// Serialize an array item
    fn serialize_array_item(
        &self,
        writer: &mut Writer<Cursor<Vec<u8>>>,
        item: &Node,
    ) -> XmpResult<()> {
        match item {
            Node::Simple(simple) => {
                writer.write_event(Event::Text(BytesText::new(&simple.value)))?;
            }
            Node::Structure(structure) => {
                writer.write_event(Event::Start(BytesStart::new("rdf:Description")))?;
                for (key, value) in &structure.fields {
                    self.serialize_node(writer, key, value)?;
                }
                writer.write_event(Event::End(BytesEnd::new("rdf:Description")))?;
            }
            Node::Array(_) => {
                return Err(XmpError::NotSupported(
                    "Nested arrays not yet supported".to_string(),
                ));
            }
        }
        Ok(())
    }

    /// Serialize to XMP Packet format
    pub fn serialize_packet(&self, root: &StructureNode) -> XmpResult<String> {
        let rdf_content = self.serialize_rdf(root)?;

        // Wrap in xpacket
        let packet = format!(
            r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
{}
<?xpacket end="w"?>"#,
            rdf_content
        );

        Ok(packet)
    }
}

impl Default for XmpSerializer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_rdf() {
        let serializer = XmpSerializer::new();
        let root = StructureNode::new();
        let result = serializer.serialize_rdf(&root);
        assert!(result.is_ok());
    }

    #[test]
    fn test_serialize_packet() {
        let serializer = XmpSerializer::new();
        let mut root = StructureNode::new();
        root.set_field(
            "http://ns.adobe.com/xap/1.0/:CreatorTool".to_string(),
            Node::simple("TestApp".to_string()),
        );
        let result = serializer.serialize_packet(&root);
        assert!(result.is_ok());
        let packet = result.unwrap();
        eprintln!("Serialized packet:\n{}", packet);
        assert!(packet.contains("<?xpacket"));
        assert!(packet.contains("rdf:RDF"));
        assert!(packet.contains("xmp:CreatorTool"));
    }
}
