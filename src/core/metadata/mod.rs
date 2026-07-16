//! XMP Metadata
//!
//! This module provides the main XmpMeta struct for working with XMP metadata.

use crate::core::error::{XmpError, XmpResult};
use crate::core::namespace::NamespaceMap;
use crate::core::node::{Node, StructureNode};
use crate::core::parser::XmpParser;
use crate::core::serializer::XmpSerializer;
use crate::core::xpath::PathComponent;
use crate::types::value::XmpValue;
use std::str::FromStr;

mod node;
#[macro_use]
mod macros;

use node::{new_root_node, root_read_with, RootNode};

/// Main structure for working with XMP metadata
#[derive(Debug, Clone)]
pub struct XmpMeta {
    /// Root structure node containing all properties
    root: RootNode,
    /// Namespace map
    namespaces: NamespaceMap,
    /// About URI (typically empty string for main metadata)
    about_uri: Option<String>,
}

/// A property entry produced by iterating an [`XmpMeta`] instance.
#[derive(Debug, Clone)]
pub struct XmpProperty {
    /// Expanded namespace URI for the property (e.g., `http://ns.adobe.com/xap/1.0/`)
    pub namespace_uri: String,
    /// Property name (e.g., "CreatorTool", "creator", "Flash")
    pub name: String,
    /// Property value
    pub value: XmpValue,
}

impl XmpMeta {
    /// Create a new empty XMP metadata object
    pub fn new() -> Self {
        Self {
            root: new_root_node(StructureNode::new()),
            namespaces: NamespaceMap::new(),
            about_uri: None,
        }
    }

    /// Returns all top-level properties in this metadata object.
    ///
    /// This method returns an owned iterator (it snapshots the current state), so it can
    /// be used safely without holding internal borrows/locks across iteration.
    pub fn all_properties(&self) -> Vec<XmpProperty> {
        root_read_with(&self.root, |root| {
            let mut out = Vec::with_capacity(root.fields.len());

            for key in root.fields.keys() {
                // Keys are stored as "namespace_uri:property_name".
                let Some(colon_pos) = key.rfind(':') else {
                    continue;
                };
                let namespace_uri = key[..colon_pos].to_string();
                let name = key[colon_pos + 1..].to_string();

                if let Some(value) = self.get_property(&namespace_uri, &name) {
                    out.push(XmpProperty {
                        namespace_uri,
                        name,
                        value,
                    });
                }
            }
            out.sort_by(|a, b| {
                (a.namespace_uri.as_str(), a.name.as_str())
                    .cmp(&(b.namespace_uri.as_str(), b.name.as_str()))
            });

            out
        })
    }

    /// Resolve namespace URI from namespace parameter (URI or prefix)
    ///
    /// Returns the URI if namespace is already a URI, or resolves the prefix to URI.
    /// Returns None if namespace is a prefix that is not registered.
    fn resolve_namespace_uri(&self, namespace: &str) -> Option<String> {
        if namespace.starts_with("http://") {
            Some(namespace.to_string())
        } else {
            self.namespaces
                .get_uri(namespace)
                .map(|s| s.to_string())
                .or_else(|| {
                    use crate::core::namespace::get_global_namespace_uri;
                    get_global_namespace_uri(namespace)
                })
        }
    }

    /// Resolve namespace URI from namespace parameter (URI or prefix) with error handling
    ///
    /// Returns the URI if namespace is already a URI, or resolves the prefix to URI.
    /// Returns an error if namespace is a prefix that is not registered.
    ///
    /// **Note**: SetProperty requires the namespace to be registered first,
    /// even when using a full URI. This matches that behavior.
    fn resolve_namespace_uri_or_error(&self, namespace: &str) -> XmpResult<String> {
        if namespace.starts_with("http://") {
            // Even for URIs, check if they're registered
            // First check instance namespace map, then global registry
            if self.namespaces.has_uri(namespace) {
                Ok(namespace.to_string())
            } else {
                // Check global registry
                use crate::core::namespace::is_namespace_registered;
                if is_namespace_registered(namespace) {
                    Ok(namespace.to_string())
                } else {
                    Err(XmpError::BadSchema(format!(
                        "Unregistered schema namespace URI '{}'. Register the namespace first using register_namespace().",
                        namespace
                    )))
                }
            }
        } else {
            // Try instance namespace map first
            if let Some(uri) = self.namespaces.get_uri(namespace) {
                Ok(uri.to_string())
            } else {
                // Try global registry
                use crate::core::namespace::get_global_namespace_uri;
                if let Some(uri) = get_global_namespace_uri(namespace) {
                    Ok(uri)
                } else {
                    Err(XmpError::BadSchema(format!(
                        "Unknown namespace prefix '{}'. Use a full URI (e.g., 'http://ns.adobe.com/xap/1.0/') or register the namespace first.",
                        namespace
                    )))
                }
            }
        }
    }

    /// Parse XMP metadata from a string
    ///
    /// The string should contain a complete XMP Packet (with or without
    /// the `<?xpacket>` wrapper).
    pub fn parse(s: &str) -> XmpResult<Self> {
        let mut parser = XmpParser::new();
        let root_node = parser.parse_packet(s)?;
        let namespaces = parser.namespace_map();

        Ok(Self {
            root: new_root_node(root_node),
            namespaces,
            about_uri: None,
        })
    }

    /// Check if a property exists
    ///
    /// # Arguments
    ///
    /// * `namespace` - The namespace URI or prefix
    /// * `path` - The property path
    pub fn has_property(&self, namespace: &str, path: &str) -> bool {
        root_read_with(&self.root, |root| {
            self.get_node_by_path(root, namespace, path).is_some()
        })
    }

    /// Get a property value
    ///
    /// # Arguments
    ///
    /// * `namespace` - The namespace URI or prefix
    /// * `path` - The property path (e.g., "CreatorTool" or "creator\[1\]")
    pub fn get_property(&self, namespace: &str, path: &str) -> Option<XmpValue> {
        let root = root_read_opt!(self.root);
        let (node, _) = self.get_node_by_path(&root, namespace, path)?;

        // Handle simple node
        if let Some(simple_node) = node.as_simple() {
            return Some(XmpValue::String(simple_node.value.clone()));
        }

        // Handle structure node: return empty string
        // Arrays and non-leaf levels of structs do not have values.
        // Use get_struct_field() to access individual fields.
        if node.as_structure().is_some() {
            return Some(XmpValue::String(String::new()));
        }

        None
    }

    /// Set a property value
    ///
    /// # Arguments
    ///
    /// * `namespace` - The namespace URI or prefix
    /// * `path` - The property path
    /// * `value` - The value to set
    pub fn set_property(&mut self, namespace: &str, path: &str, value: XmpValue) -> XmpResult<()> {
        let ns_uri = self.resolve_namespace_uri_or_error(namespace)?;
        let parsed = crate::core::xpath::parse_path(path)?;
        let mut root = root_write!(self.root);

        if parsed.components.len() == 1 {
            let name = match &parsed.components[0] {
                PathComponent::Name(name) => name,
                _ => {
                    return Err(XmpError::BadXPath(
                        "Path must start with a name".to_string(),
                    ))
                }
            };
            let key = format!("{}:{}", ns_uri, name);
            let node = value_to_node(value, &ns_uri, &self.namespaces)?;
            root.set_field(key, node);
        } else {
            let parent_components = &parsed.components[..parsed.components.len() - 1];
            let parent_node =
                get_or_create_node(&mut root, &self.namespaces, &ns_uri, parent_components)?;
            let last_comp = parsed.components.last().unwrap();
            match last_comp {
                PathComponent::Name(name) => {
                    let structure = parent_node.as_structure_mut().ok_or_else(|| {
                        XmpError::BadValue("Parent is not a structure".to_string())
                    })?;
                    let parent_ns_uri = {
                        let mut resolved_uri = ns_uri;
                        if let Some(PathComponent::Name(pname)) = parent_components.last() {
                            if let Some(colon_pos) = pname.find(':') {
                                let prefix = &pname[..colon_pos];
                                if let Some(uri) = self.resolve_namespace_uri(prefix) {
                                    resolved_uri = uri;
                                }
                            }
                        }
                        resolved_uri
                    };
                    let key = self.resolve_field_key(&parent_ns_uri, name);
                    let node = value_to_node(value, &parent_ns_uri, &self.namespaces)?;
                    structure.set_field(key, node);
                }
                PathComponent::Index(idx) => {
                    if *idx == 0 {
                        return Err(XmpError::BadXPath(
                            "Array index must be 1 or greater".to_string(),
                        ));
                    }
                    let idx_0 = *idx - 1;
                    let array = parent_node
                        .as_array_mut()
                        .ok_or_else(|| XmpError::BadValue("Parent is not an array".to_string()))?;
                    let node = value_to_node(value, &ns_uri, &self.namespaces)?;
                    while array.len() <= idx_0 {
                        array.append(Node::simple(""));
                    }
                    if let Some(item) = array.get_mut(idx_0) {
                        *item = node;
                    }
                }
            }
        }
        Ok(())
    }

    /// Delete a property
    ///
    /// # Arguments
    ///
    /// * `namespace` - The namespace URI or prefix
    /// * `path` - The property path
    pub fn delete_property(&mut self, namespace: &str, path: &str) -> XmpResult<()> {
        let ns_uri = self.resolve_namespace_uri_or_error(namespace)?;
        let parsed = crate::core::xpath::parse_path(path)?;
        let mut root = root_write!(self.root);

        if parsed.components.is_empty() {
            return Ok(());
        }

        if parsed.components.len() == 1 {
            let name = match &parsed.components[0] {
                PathComponent::Name(name) => name,
                _ => {
                    return Err(XmpError::BadXPath(
                        "Path must start with a name".to_string(),
                    ))
                }
            };
            let key = format!("{}:{}", ns_uri, name);
            root.remove_field(&key);
        } else {
            let parent_components = &parsed.components[..parsed.components.len() - 1];
            if let Some((parent_node, _)) =
                self.get_node_by_components_mut(&mut root, namespace, parent_components)
            {
                let last_comp = parsed.components.last().unwrap();
                match last_comp {
                    PathComponent::Name(name) => {
                        if let Some(structure) = parent_node.as_structure_mut() {
                            let parent_ns_uri = {
                                let mut resolved_uri = ns_uri;
                                if let Some(PathComponent::Name(pname)) = parent_components.last() {
                                    if let Some(colon_pos) = pname.find(':') {
                                        let prefix = &pname[..colon_pos];
                                        if let Some(uri) = self.resolve_namespace_uri(prefix) {
                                            resolved_uri = uri;
                                        }
                                    }
                                }
                                resolved_uri
                            };
                            let key = self.resolve_field_key(&parent_ns_uri, name);
                            structure.remove_field(&key);
                        }
                    }
                    PathComponent::Index(idx) => {
                        if *idx > 0 {
                            if let Some(array) = parent_node.as_array_mut() {
                                if array.len() >= *idx {
                                    let _ = array.remove(*idx - 1);
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Get the about URI
    pub fn about_uri(&self) -> Option<&str> {
        self.about_uri.as_deref()
    }

    /// Set the about URI
    pub fn set_about_uri(&mut self, uri: impl Into<String>) {
        self.about_uri = Some(uri.into());
    }

    /// Serialize to RDF/XML string
    pub fn serialize(&self) -> XmpResult<String> {
        let serializer = XmpSerializer::with_namespaces(self.namespaces.clone());
        let root = root_read!(self.root);
        serializer.serialize_rdf(&root)
    }

    /// Serialize to XMP Packet format
    pub fn serialize_packet(&self) -> XmpResult<String> {
        let serializer = XmpSerializer::with_namespaces(self.namespaces.clone());
        let root = root_read!(self.root);
        serializer.serialize_packet(&root)
    }

    /// Serialize to XMP Packet format with padding to reach a target length
    ///
    /// This is useful for in-place updates where the new packet needs to fit
    /// within the space of an existing packet.
    ///
    /// # Arguments
    ///
    /// * `target_length` - The desired total packet length in bytes
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - The serialized packet with padding
    /// * `Err(XmpError)` - If the serialized packet exceeds target_length
    pub fn serialize_packet_with_padding(&self, target_length: usize) -> XmpResult<String> {
        let serializer = XmpSerializer::with_namespaces(self.namespaces.clone());
        let root = root_read!(self.root);
        serializer.serialize_packet_with_padding(&root, target_length)
    }

    /// Get an array item by index
    ///
    /// # Arguments
    ///
    /// * `namespace` - The namespace URI or prefix
    /// * `path` - The array property path (e.g., "creator")
    /// * `index` - The array index (0-based)
    pub fn get_array_item(&self, namespace: &str, path: &str, index: usize) -> Option<XmpValue> {
        let root = root_read_opt!(self.root);
        let (node, _) = self.get_node_by_path(&root, namespace, path)?;
        let array = node.as_array()?;
        let item = array.get(index)?;
        let simple = item.as_simple()?;
        Some(XmpValue::String(simple.value.clone()))
    }

    /// Get the size of an array property
    ///
    /// # Arguments
    ///
    /// * `namespace` - The namespace URI or prefix
    /// * `path` - The array property path
    pub fn get_array_size(&self, namespace: &str, path: &str) -> Option<usize> {
        let root = root_read_opt!(self.root);
        let (node, _) = self.get_node_by_path(&root, namespace, path)?;
        let array = node.as_array()?;
        Some(array.len())
    }

    /// Append an item to an array property
    ///
    /// # Arguments
    ///
    /// * `namespace` - The namespace URI or prefix
    /// * `path` - The array property path
    /// * `value` - The value to append
    pub fn append_array_item(
        &mut self,
        namespace: &str,
        path: &str,
        value: XmpValue,
    ) -> XmpResult<()> {
        let ns_uri = self.resolve_namespace_uri_or_error(namespace)?;
        let parsed = crate::core::xpath::parse_path(path)?;
        let mut root = root_write!(self.root);

        let array_node =
            get_or_create_node(&mut root, &self.namespaces, &ns_uri, &parsed.components)?;
        if !array_node.is_array() {
            *array_node = Node::array(crate::core::node::ArrayType::Ordered);
        }
        let array = array_node.as_array_mut().unwrap();
        let item_node = value_to_node(value, &ns_uri, &self.namespaces)?;
        array.append(item_node);
        Ok(())
    }

    /// Insert an item into an array property at a specific index
    ///
    /// # Arguments
    ///
    /// * `namespace` - The namespace URI or prefix
    /// * `path` - The array property path
    /// * `index` - The index to insert at (0-based)
    /// * `value` - The value to insert
    pub fn insert_array_item(
        &mut self,
        namespace: &str,
        path: &str,
        index: usize,
        value: XmpValue,
    ) -> XmpResult<()> {
        let ns_uri = self.resolve_namespace_uri_or_error(namespace)?;
        let parsed = crate::core::xpath::parse_path(path)?;
        let mut root = root_write!(self.root);

        let (array_node, _) = self
            .get_node_by_components_mut(&mut root, namespace, &parsed.components)
            .ok_or_else(|| {
                XmpError::BadValue(format!("Property '{}:{}' not found", ns_uri, path))
            })?;
        let array = array_node.as_array_mut().ok_or_else(|| {
            XmpError::BadValue(format!(
                "Property '{}:{}' exists but is not an array. Use get_property() or get_struct_field() instead.",
                ns_uri, path
            ))
        })?;

        let item_node = value_to_node(value, &ns_uri, &self.namespaces)?;
        array.insert(index, item_node)
    }

    /// Delete an item from an array property
    ///
    /// # Arguments
    ///
    /// * `namespace` - The namespace URI or prefix
    /// * `path` - The array property path
    /// * `index` - The index to delete (0-based)
    pub fn delete_array_item(
        &mut self,
        namespace: &str,
        path: &str,
        index: usize,
    ) -> XmpResult<()> {
        let ns_uri = self.resolve_namespace_uri_or_error(namespace)?;
        let parsed = crate::core::xpath::parse_path(path)?;
        let mut root = root_write!(self.root);

        if let Some((array_node, _)) =
            self.get_node_by_components_mut(&mut root, namespace, &parsed.components)
        {
            let array = array_node.as_array_mut().ok_or_else(|| {
                XmpError::BadValue(format!(
                    "Property '{}:{}' exists but is not an array. Use get_property() or get_struct_field() instead.",
                    ns_uri, path
                ))
            })?;
            if index < array.len() {
                array.remove(index).map(|_| ())
            } else {
                Err(XmpError::BadValue(format!(
                    "Array index {} out of bounds",
                    index
                )))
            }
        } else {
            Ok(())
        }
    }

    /// Get a structure field value
    ///
    /// # Arguments
    ///
    /// * `namespace` - The namespace URI or prefix
    /// * `struct_path` - The structure property path
    /// * `field_name` - The field name within the structure
    pub fn get_struct_field(
        &self,
        namespace: &str,
        struct_path: &str,
        field_name: &str,
    ) -> Option<XmpValue> {
        let root = root_read_opt!(self.root);
        let (struct_node, struct_ns_uri) = self.get_node_by_path(&root, namespace, struct_path)?;
        let structure = struct_node.as_structure()?;
        let field_key = self.resolve_field_key(&struct_ns_uri, field_name);
        structure
            .get_field(&field_key)
            .and_then(|field_node| field_node.as_simple())
            .map(|n| XmpValue::String(n.value.clone()))
    }

    /// Set a structure field value
    ///
    /// # Arguments
    ///
    /// * `namespace` - The namespace URI or prefix
    /// * `struct_path` - The structure property path
    /// * `field_name` - The field name within the structure
    /// * `value` - The value to set
    pub fn set_struct_field(
        &mut self,
        namespace: &str,
        struct_path: &str,
        field_name: &str,
        value: XmpValue,
    ) -> XmpResult<()> {
        let ns_uri = self.resolve_namespace_uri_or_error(namespace)?;
        let parsed = crate::core::xpath::parse_path(struct_path)?;
        let mut root = root_write!(self.root);

        let struct_node =
            get_or_create_node(&mut root, &self.namespaces, &ns_uri, &parsed.components)?;
        let structure = struct_node.as_structure_mut().ok_or_else(|| {
            XmpError::BadValue(format!("Property '{}' is not a structure", struct_path))
        })?;

        let struct_ns_uri = {
            let mut resolved_uri = ns_uri;
            if let Some(PathComponent::Name(name)) = parsed.components.last() {
                if let Some(colon_pos) = name.find(':') {
                    let prefix = &name[..colon_pos];
                    if let Some(uri) = self.resolve_namespace_uri(prefix) {
                        resolved_uri = uri;
                    }
                }
            }
            resolved_uri
        };

        let field_key = self.resolve_field_key(&struct_ns_uri, field_name);
        let field_node = value_to_node(value, &struct_ns_uri, &self.namespaces)?;
        structure.set_field(field_key, field_node);
        Ok(())
    }

    /// Delete a structure field
    ///
    /// # Arguments
    ///
    /// * `namespace` - The namespace URI or prefix
    /// * `struct_path` - The structure property path
    /// * `field_name` - The field name to delete
    pub fn delete_struct_field(
        &mut self,
        namespace: &str,
        struct_path: &str,
        field_name: &str,
    ) -> XmpResult<()> {
        let ns_uri = self.resolve_namespace_uri_or_error(namespace)?;
        let parsed = crate::core::xpath::parse_path(struct_path)?;
        let mut root = root_write!(self.root);

        if let Some((struct_node, _)) =
            self.get_node_by_components_mut(&mut root, namespace, &parsed.components)
        {
            let structure = struct_node.as_structure_mut().ok_or_else(|| {
                XmpError::BadValue(format!(
                    "Property '{}:{}' exists but is not a structure. Use get_property() or get_array_item() instead.",
                    ns_uri, struct_path
                ))
            })?;
            let struct_ns_uri = {
                let mut resolved_uri = ns_uri;
                if let Some(PathComponent::Name(name)) = parsed.components.last() {
                    if let Some(colon_pos) = name.find(':') {
                        let prefix = &name[..colon_pos];
                        if let Some(uri) = self.resolve_namespace_uri(prefix) {
                            resolved_uri = uri;
                        }
                    }
                }
                resolved_uri
            };
            let field_key = self.resolve_field_key(&struct_ns_uri, field_name);
            structure.remove_field(&field_key);
        }
        Ok(())
    }

    /// Navigate the path and return the node and the last resolved namespace URI
    fn get_node_by_path<'a>(
        &self,
        root: &'a StructureNode,
        namespace: &str,
        path: &str,
    ) -> Option<(&'a Node, String)> {
        let ns_uri = self.resolve_namespace_uri(namespace)?;
        let parsed = crate::core::xpath::parse_path(path).ok()?;
        if parsed.components.is_empty() {
            return None;
        }

        let mut current_ns_uri = ns_uri;
        let first_name = match &parsed.components[0] {
            PathComponent::Name(name) => name,
            _ => return None,
        };
        let first_key = format!("{}:{}", current_ns_uri, first_name);
        let mut current = root.get_field(&first_key)?;

        for i in 1..parsed.components.len() {
            match &parsed.components[i] {
                PathComponent::Index(idx) => {
                    if *idx == 0 {
                        return None;
                    }
                    let array = current.as_array()?;
                    current = array.get(*idx - 1)?;
                }
                PathComponent::Name(name) => {
                    let structure = current.as_structure()?;
                    let key = if let Some(colon_pos) = name.find(':') {
                        let prefix = &name[..colon_pos];
                        let local_name = &name[colon_pos + 1..];
                        if let Some(uri) = self.resolve_namespace_uri(prefix) {
                            current_ns_uri = uri;
                            format!("{}:{}", current_ns_uri, local_name)
                        } else {
                            name.clone()
                        }
                    } else {
                        format!("{}:{}", current_ns_uri, name)
                    };
                    current = structure.get_field(&key)?;
                }
            }
        }

        let mut resolved_uri = current_ns_uri;
        if let Some(PathComponent::Name(name)) = parsed.components.last() {
            if let Some(colon_pos) = name.find(':') {
                let prefix = &name[..colon_pos];
                if let Some(uri) = self.resolve_namespace_uri(prefix) {
                    resolved_uri = uri;
                }
            }
        }

        Some((current, resolved_uri))
    }

    /// Navigate the path and return a mutable reference to the node and the last resolved namespace URI
    fn get_node_by_components_mut<'a>(
        &self,
        root: &'a mut StructureNode,
        namespace: &str,
        components: &[PathComponent],
    ) -> Option<(&'a mut Node, String)> {
        if components.is_empty() {
            return None;
        }

        let ns_uri = self.resolve_namespace_uri(namespace)?;
        let mut current_ns_uri = ns_uri;
        let first_name = match &components[0] {
            PathComponent::Name(name) => name,
            _ => return None,
        };
        let first_key = format!("{}:{}", current_ns_uri, first_name);
        let mut current = root.get_field_mut(&first_key)?;

        for component in components.iter().skip(1) {
            let next = match component {
                PathComponent::Index(idx) => {
                    if *idx == 0 {
                        return None;
                    }
                    let array = current.as_array_mut()?;
                    array.get_mut(*idx - 1)?
                }
                PathComponent::Name(name) => {
                    let structure = current.as_structure_mut()?;
                    let key = if let Some(colon_pos) = name.find(':') {
                        let prefix = &name[..colon_pos];
                        let local_name = &name[colon_pos + 1..];
                        if let Some(uri) = self.resolve_namespace_uri(prefix) {
                            current_ns_uri = uri;
                            format!("{}:{}", current_ns_uri, local_name)
                        } else {
                            name.clone()
                        }
                    } else {
                        format!("{}:{}", current_ns_uri, name)
                    };
                    structure.get_field_mut(&key)?
                }
            };
            current = next;
        }

        let mut resolved_uri = current_ns_uri;
        if let Some(PathComponent::Name(name)) = components.last() {
            if let Some(colon_pos) = name.find(':') {
                let prefix = &name[..colon_pos];
                if let Some(uri) = self.resolve_namespace_uri(prefix) {
                    resolved_uri = uri;
                }
            }
        }

        Some((current, resolved_uri))
    }

    /// Resolve field name key inside structure
    fn resolve_field_key(&self, struct_ns_uri: &str, field_name: &str) -> String {
        if let Some(colon_pos) = field_name.find(':') {
            let prefix = &field_name[..colon_pos];
            let name = &field_name[colon_pos + 1..];
            if let Some(uri) = self.resolve_namespace_uri(prefix) {
                return format!("{}:{}", uri, name);
            }
        }
        format!("{}:{}", struct_ns_uri, field_name)
    }

    /// Set a localized text property
    ///
    /// Localized text properties are stored as `rdf:Alt` arrays, where each item
    /// has an `xml:lang` qualifier indicating its language.
    ///
    /// # Arguments
    ///
    /// * `namespace` - The namespace URI or prefix
    /// * `property` - The property name
    /// * `generic_lang` - Generic language code (e.g., "en"), can be empty string
    /// * `specific_lang` - Specific language code (e.g., "en-US"), required
    /// * `value` - The text value to set
    ///
    /// # Example
    ///
    /// ```rust
    /// use xmpkit::{XmpMeta, XmpValue};
    ///
    /// let mut meta = XmpMeta::new();
    /// meta.set_localized_text(
    ///     "http://purl.org/dc/elements/1.1/",
    ///     "title",
    ///     "",
    ///     "x-default",
    ///     "Default Title"
    /// ).unwrap();
    /// ```
    pub fn set_localized_text(
        &mut self,
        namespace: &str,
        property: &str,
        _generic_lang: &str,
        specific_lang: &str,
        value: &str,
    ) -> XmpResult<()> {
        use crate::core::namespace::ns;
        use crate::core::node::{ArrayNode, ArrayType, Node};
        use crate::types::qualifier::Qualifier;

        let ns_uri = self.resolve_namespace_uri_or_error(namespace)?;

        let full_path = format!("{}:{}", ns_uri, property);
        let mut root = root_write!(self.root);

        // Get or create Alt array
        let array_node = root
            .get_field_mut(&full_path)
            .and_then(|node| node.as_array_mut());

        let array = if let Some(array) = array_node {
            // Ensure it's an Alt array
            if array.array_type != ArrayType::Alternative {
                return Err(XmpError::BadValue(format!(
                    "Property '{}:{}' exists but is not a localized text array (rdf:Alt). Expected array type: Alternative",
                    ns_uri, property
                )));
            }
            array
        } else {
            // Create new Alt array
            let new_array = ArrayNode::new(ArrayType::Alternative);
            root.set_field(full_path.clone(), Node::Array(new_array));
            root.get_field_mut(&full_path)
                .and_then(|node| node.as_array_mut())
                .ok_or_else(|| {
                    XmpError::InternalError("Failed to create localized text array".to_string())
                })?
        };

        // Find existing item with matching specific_lang
        let mut found = false;
        for item in &mut array.items {
            let Some(simple) = item.as_simple_mut() else {
                continue;
            };
            let Some(lang_qual) = simple.get_qualifier(ns::XML, "lang") else {
                continue;
            };
            if lang_qual.value == specific_lang {
                // Update existing item
                simple.value = value.to_string();
                found = true;
                break;
            }
        }

        // If not found, create new item
        if !found {
            let mut simple_node = Node::simple(value.to_string());
            if let Node::Simple(ref mut sn) = simple_node {
                let lang_qualifier = Qualifier::new(ns::XML, "lang", specific_lang.to_string());
                sn.add_qualifier(lang_qualifier);
            }
            array.append(simple_node);
        }

        Ok(())
    }

    /// Get a localized text property
    ///
    /// This method searches for a localized text value matching the specified
    /// language codes. It follows XMP language matching rules:
    /// 1. Exact match for specific_lang
    /// 2. Match for generic_lang if specific_lang not found
    /// 3. Fallback to "x-default" if neither found
    ///
    /// # Arguments
    ///
    /// * `namespace` - The namespace URI or prefix
    /// * `property` - The property name
    /// * `generic_lang` - Generic language code (e.g., "en"), can be empty string
    /// * `specific_lang` - Specific language code (e.g., "en-US"), required
    ///
    /// # Returns
    ///
    /// Returns `Some((value, actual_lang))` if found, where:
    /// - `value` is the text value
    /// - `actual_lang` is the actual language code used (may differ from requested)
    ///
    /// Returns `None` if the property doesn't exist or no matching language found.
    ///
    /// # Example
    ///
    /// ```rust
    /// use xmpkit::XmpMeta;
    ///
    /// let mut meta = XmpMeta::new();
    /// meta.set_localized_text(
    ///     "http://purl.org/dc/elements/1.1/",
    ///     "title",
    ///     "",
    ///     "x-default",
    ///     "Default Title"
    /// ).unwrap();
    ///
    /// let (value, lang) = meta.get_localized_text(
    ///     "http://purl.org/dc/elements/1.1/",
    ///     "title",
    ///     "",
    ///     "x-default"
    /// ).unwrap();
    /// assert_eq!(value, "Default Title");
    /// assert_eq!(lang, "x-default");
    /// ```
    pub fn get_localized_text(
        &self,
        namespace: &str,
        property: &str,
        generic_lang: &str,
        specific_lang: &str,
    ) -> Option<(String, String)> {
        use crate::core::namespace::ns;

        let ns_uri = self.resolve_namespace_uri(namespace)?;

        let full_path = format!("{}:{}", ns_uri, property);
        let root = root_read_opt!(self.root);

        let array = root
            .get_field(&full_path)
            .and_then(|node| node.as_array())?;

        // Ensure it's an Alt array
        if array.array_type != crate::core::node::ArrayType::Alternative {
            return None;
        }

        // Try exact match for specific_lang
        for item in &array.items {
            let Some(simple) = item.as_simple() else {
                continue;
            };
            let Some(lang_qual) = simple.get_qualifier(ns::XML, "lang") else {
                continue;
            };
            if lang_qual.value == specific_lang {
                return Some((simple.value.clone(), lang_qual.value.clone()));
            }
        }

        // Try match for generic_lang (if provided)
        if !generic_lang.is_empty() {
            for item in &array.items {
                let Some(simple) = item.as_simple() else {
                    continue;
                };
                let Some(lang_qual) = simple.get_qualifier(ns::XML, "lang") else {
                    continue;
                };
                if lang_qual.value.starts_with(generic_lang) {
                    return Some((simple.value.clone(), lang_qual.value.clone()));
                }
            }
        }

        // Fallback to x-default
        for item in &array.items {
            let Some(simple) = item.as_simple() else {
                continue;
            };
            let Some(lang_qual) = simple.get_qualifier(ns::XML, "lang") else {
                continue;
            };
            if lang_qual.value == "x-default" {
                return Some((simple.value.clone(), lang_qual.value.clone()));
            }
        }

        // If no x-default, return first item
        let first_item = array.items.first()?;
        let simple = first_item.as_simple()?;
        let lang = simple
            .get_qualifier(ns::XML, "lang")
            .map(|q| q.value.clone())
            .unwrap_or_else(|| "".to_string());
        Some((simple.value.clone(), lang))
    }

    /// Set a date/time property
    ///
    /// This is a convenience method that validates and formats the date/time value
    /// before setting it as a property.
    ///
    /// # Arguments
    ///
    /// * `namespace` - The namespace URI or prefix
    /// * `path` - The property path
    /// * `dt` - The date/time value
    ///
    /// # Example
    ///
    /// ```rust
    /// use xmpkit::{XmpMeta, utils::datetime::XmpDateTime};
    ///
    /// let mut meta = XmpMeta::new();
    /// let mut dt = XmpDateTime::new();
    /// dt.has_date = true;
    /// dt.has_time = true;
    /// dt.year = 2023;
    /// dt.month = 12;
    /// dt.day = 25;
    /// dt.hour = 10;
    /// dt.minute = 30;
    /// dt.second = 0;
    /// dt.has_timezone = true;
    /// dt.tz_sign = 0; // UTC
    ///
    /// meta.set_date_time("http://ns.adobe.com/xap/1.0/", "ModifyDate", &dt).unwrap();
    /// ```
    pub fn set_date_time(
        &mut self,
        namespace: &str,
        path: &str,
        dt: &crate::utils::datetime::XmpDateTime,
    ) -> XmpResult<()> {
        dt.validate()?;
        let formatted = dt.format();
        self.set_property(namespace, path, XmpValue::DateTime(formatted))
    }

    /// Get a date/time property
    ///
    /// This is a convenience method that parses a date/time property value
    /// and returns it as an `XmpDateTime`.
    ///
    /// # Arguments
    ///
    /// * `namespace` - The namespace URI or prefix
    /// * `path` - The property path
    ///
    /// # Returns
    ///
    /// Returns `Some(XmpDateTime)` if the property exists and can be parsed,
    /// `None` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// use xmpkit::{XmpMeta, XmpValue, utils::datetime::XmpDateTime};
    ///
    /// let mut meta = XmpMeta::new();
    /// meta.set_property(
    ///     "http://ns.adobe.com/xap/1.0/",
    ///     "ModifyDate",
    ///     XmpValue::DateTime("2023-12-25T10:30:00Z".to_string())
    /// ).unwrap();
    ///
    /// let dt = meta.get_date_time("http://ns.adobe.com/xap/1.0/", "ModifyDate").unwrap();
    /// assert_eq!(dt.year, 2023);
    /// assert_eq!(dt.month, 12);
    /// assert_eq!(dt.day, 25);
    /// ```
    pub fn get_date_time(
        &self,
        namespace: &str,
        path: &str,
    ) -> Option<crate::utils::datetime::XmpDateTime> {
        self.get_property(namespace, path)
            .and_then(|v| match v {
                XmpValue::DateTime(s) => Some(s),
                XmpValue::String(s) => Some(s),
                _ => None,
            })
            .and_then(|s| crate::utils::datetime::XmpDateTime::parse(&s).ok())
    }
}

/// Convert XmpValue to Node
fn value_to_node(
    value: XmpValue,
    default_ns_uri: &str,
    namespaces: &NamespaceMap,
) -> XmpResult<Node> {
    match value {
        XmpValue::String(s) => Ok(Node::simple(s)),
        XmpValue::Integer(i) => Ok(Node::simple(i.to_string())),
        XmpValue::Boolean(b) => Ok(Node::simple(if b { "True" } else { "False" })),
        XmpValue::DateTime(dt) => Ok(Node::simple(dt)),
        XmpValue::Array(arr) => {
            use crate::core::node::{ArrayNode, ArrayType};
            // Note: XmpValue::Array does not carry the specific array type (Seq, Bag, or Alt).
            // We default to ArrayType::Unordered (Bag) here. As a limitation, round-trips of
            // Seq or Alt arrays parsed and then set/serialized via XmpValue may lose their type info.
            let mut array_node = ArrayNode::new(ArrayType::Unordered);
            for item in arr {
                let item_node = value_to_node(item, default_ns_uri, namespaces)?;
                array_node.append(item_node);
            }
            Ok(Node::Array(array_node))
        }
        XmpValue::Structure(structure) => {
            let mut structure_node = crate::core::node::StructureNode::new();
            for (key, val) in structure {
                let resolved_key = if let Some(colon_pos) = key.find(':') {
                    let prefix = &key[..colon_pos];
                    let local_name = &key[colon_pos + 1..];
                    if let Some(uri) = namespaces.get_uri(prefix) {
                        format!("{}:{}", uri, local_name)
                    } else {
                        // Check global namespaces
                        use crate::core::namespace::get_global_namespace_uri;
                        if let Some(uri) = get_global_namespace_uri(prefix) {
                            format!("{}:{}", uri, local_name)
                        } else {
                            key.clone()
                        }
                    }
                } else {
                    format!("{}:{}", default_ns_uri, key)
                };
                let val_node = value_to_node(val, default_ns_uri, namespaces)?;
                structure_node.set_field(resolved_key, val_node);
            }
            Ok(Node::Structure(structure_node))
        }
    }
}

/// Helper function to traverse or create nested nodes
fn get_or_create_node<'a>(
    root: &'a mut StructureNode,
    namespaces: &NamespaceMap,
    ns_uri: &str,
    components: &[crate::core::xpath::PathComponent],
) -> XmpResult<&'a mut Node> {
    use crate::core::xpath::PathComponent;
    if components.is_empty() {
        return Err(XmpError::BadXPath("Empty path components".to_string()));
    }

    let mut current_ns_uri = ns_uri.to_string();

    // Handle first component
    let first_name = match &components[0] {
        PathComponent::Name(name) => name,
        _ => {
            return Err(XmpError::BadXPath(
                "Path must start with a property name".to_string(),
            ))
        }
    };

    let first_key = format!("{}:{}", current_ns_uri, first_name);

    // Determine default node
    let default_node = if components.len() > 1 {
        match &components[1] {
            PathComponent::Index(_) => Node::array(crate::core::node::ArrayType::Ordered),
            _ => Node::structure(),
        }
    } else {
        Node::structure()
    };

    let mut current = root.fields.entry(first_key).or_insert(default_node);

    // Handle remaining components
    for i in 1..components.len() {
        let next_is_index =
            i + 1 < components.len() && matches!(&components[i + 1], PathComponent::Index(_));
        let default_child = if next_is_index {
            Node::array(crate::core::node::ArrayType::Ordered)
        } else {
            Node::structure()
        };

        match &components[i] {
            PathComponent::Index(idx) => {
                if *idx == 0 {
                    return Err(XmpError::BadXPath(
                        "Array index must be 1 or greater".to_string(),
                    ));
                }
                let idx_0 = *idx - 1;

                if !matches!(current, Node::Array(_)) {
                    *current = Node::array(crate::core::node::ArrayType::Ordered);
                }

                let array = current.as_array_mut().unwrap();
                while array.len() <= idx_0 {
                    array.append(default_child.clone());
                }
                current = array.get_mut(idx_0).unwrap();
            }
            PathComponent::Name(name) => {
                if !matches!(current, Node::Structure(_)) {
                    *current = Node::structure();
                }

                let structure = current.as_structure_mut().unwrap();

                let key = if let Some(colon_pos) = name.find(':') {
                    let prefix = &name[..colon_pos];
                    let local_name = &name[colon_pos + 1..];
                    if let Some(uri) = namespaces.get_uri(prefix) {
                        current_ns_uri = uri.to_string();
                        format!("{}:{}", current_ns_uri, local_name)
                    } else {
                        // Check global namespaces
                        use crate::core::namespace::get_global_namespace_uri;
                        if let Some(uri) = get_global_namespace_uri(prefix) {
                            current_ns_uri = uri;
                            format!("{}:{}", current_ns_uri, local_name)
                        } else {
                            name.clone()
                        }
                    }
                } else {
                    format!("{}:{}", current_ns_uri, name)
                };

                current = structure.fields.entry(key).or_insert(default_child);
            }
        }
    }

    Ok(current)
}

impl Default for XmpMeta {
    fn default() -> Self {
        Self::new()
    }
}

impl FromStr for XmpMeta {
    type Err = XmpError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xmp_meta_new() {
        let meta = XmpMeta::new();
        assert!(meta.about_uri().is_none());
    }

    #[test]
    fn test_xmp_meta_from_str() {
        let xml = r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:xmp="http://ns.adobe.com/xap/1.0/">
  <rdf:Description rdf:about=""
                   xmp:CreatorTool="MyApp"/>
</rdf:RDF>
<?xpacket end="w"?>"#;

        let result = XmpMeta::parse(xml);
        assert!(result.is_ok());

        // Test FromStr trait
        let result2 = xml.parse::<XmpMeta>();
        assert!(result2.is_ok());
    }

    #[test]
    fn test_set_and_get_property() {
        let mut meta = XmpMeta::new();
        meta.set_property(
            "http://ns.adobe.com/xap/1.0/",
            "CreatorTool",
            XmpValue::String("TestApp".to_string()),
        )
        .unwrap();

        let value = meta.get_property("http://ns.adobe.com/xap/1.0/", "CreatorTool");
        assert_eq!(value, Some(XmpValue::String("TestApp".to_string())));
    }

    #[test]
    fn test_serialize() {
        let mut meta = XmpMeta::new();
        meta.set_property(
            "http://ns.adobe.com/xap/1.0/",
            "CreatorTool",
            XmpValue::String("TestApp".to_string()),
        )
        .unwrap();

        let serialized = meta.serialize().unwrap();
        assert!(serialized.contains("rdf:RDF"));
        assert!(serialized.contains("rdf:Description"));
    }

    #[test]
    fn test_serialize_packet() {
        let mut meta = XmpMeta::new();
        meta.set_property(
            "http://ns.adobe.com/xap/1.0/",
            "CreatorTool",
            XmpValue::String("TestApp".to_string()),
        )
        .unwrap();

        let packet = meta.serialize_packet().unwrap();
        assert!(packet.contains("<?xpacket"));
        assert!(packet.contains("rdf:RDF"));
    }

    #[test]
    fn test_has_property() {
        let mut meta = XmpMeta::new();
        assert!(!meta.has_property("http://ns.adobe.com/xap/1.0/", "CreatorTool"));

        meta.set_property(
            "http://ns.adobe.com/xap/1.0/",
            "CreatorTool",
            XmpValue::String("TestApp".to_string()),
        )
        .unwrap();

        assert!(meta.has_property("http://ns.adobe.com/xap/1.0/", "CreatorTool"));
    }

    #[test]
    fn test_array_operations() {
        let mut meta = XmpMeta::new();

        // Append items
        meta.append_array_item(
            "http://purl.org/dc/elements/1.1/",
            "creator",
            XmpValue::String("Author1".to_string()),
        )
        .unwrap();
        meta.append_array_item(
            "http://purl.org/dc/elements/1.1/",
            "creator",
            XmpValue::String("Author2".to_string()),
        )
        .unwrap();

        // Check size
        assert_eq!(
            meta.get_array_size("http://purl.org/dc/elements/1.1/", "creator"),
            Some(2)
        );

        // Get item
        assert_eq!(
            meta.get_array_item("http://purl.org/dc/elements/1.1/", "creator", 0),
            Some(XmpValue::String("Author1".to_string()))
        );

        // Insert item
        meta.insert_array_item(
            "http://purl.org/dc/elements/1.1/",
            "creator",
            1,
            XmpValue::String("Author1.5".to_string()),
        )
        .unwrap();

        assert_eq!(
            meta.get_array_size("http://purl.org/dc/elements/1.1/", "creator"),
            Some(3)
        );

        // Delete item
        meta.delete_array_item("http://purl.org/dc/elements/1.1/", "creator", 1)
            .unwrap();

        assert_eq!(
            meta.get_array_size("http://purl.org/dc/elements/1.1/", "creator"),
            Some(2)
        );
    }

    #[test]
    fn test_struct_operations() {
        let mut meta = XmpMeta::new();

        // Set struct field
        meta.set_struct_field(
            "http://ns.adobe.com/exif/1.0/",
            "Flash",
            "Fired",
            XmpValue::Boolean(true),
        )
        .unwrap();

        // Get struct field
        assert_eq!(
            meta.get_struct_field("http://ns.adobe.com/exif/1.0/", "Flash", "Fired"),
            Some(XmpValue::String("True".to_string()))
        );

        // Delete struct field
        meta.delete_struct_field("http://ns.adobe.com/exif/1.0/", "Flash", "Fired")
            .unwrap();

        assert_eq!(
            meta.get_struct_field("http://ns.adobe.com/exif/1.0/", "Flash", "Fired"),
            None
        );
    }

    #[test]
    fn test_localized_text_set_and_get() {
        let mut meta = XmpMeta::new();
        let ns = "http://purl.org/dc/elements/1.1/";
        let property = "title";

        // Set default title
        meta.set_localized_text(ns, property, "", "x-default", "Default Title")
            .unwrap();

        // Get default title
        let (value, lang) = meta
            .get_localized_text(ns, property, "", "x-default")
            .unwrap();
        assert_eq!(value, "Default Title");
        assert_eq!(lang, "x-default");

        // Set English title
        meta.set_localized_text(ns, property, "en", "en-US", "English Title")
            .unwrap();

        // Get English title
        let (value, lang) = meta
            .get_localized_text(ns, property, "en", "en-US")
            .unwrap();
        assert_eq!(value, "English Title");
        assert_eq!(lang, "en-US");

        // Set Chinese title
        meta.set_localized_text(ns, property, "zh", "zh-CN", "中文标题")
            .unwrap();

        // Get Chinese title
        let (value, lang) = meta
            .get_localized_text(ns, property, "zh", "zh-CN")
            .unwrap();
        assert_eq!(value, "中文标题");
        assert_eq!(lang, "zh-CN");

        // Test fallback to x-default when specific language not found
        let (value, lang) = meta
            .get_localized_text(ns, property, "fr", "fr-FR")
            .unwrap();
        assert_eq!(value, "Default Title");
        assert_eq!(lang, "x-default");
    }

    #[test]
    fn test_localized_text_update_existing() {
        let mut meta = XmpMeta::new();
        let ns = "http://purl.org/dc/elements/1.1/";
        let property = "title";

        // Set initial value
        meta.set_localized_text(ns, property, "", "x-default", "Initial Title")
            .unwrap();

        // Update existing value
        meta.set_localized_text(ns, property, "", "x-default", "Updated Title")
            .unwrap();

        // Verify update
        let (value, _) = meta
            .get_localized_text(ns, property, "", "x-default")
            .unwrap();
        assert_eq!(value, "Updated Title");
    }

    #[test]
    fn test_localized_text_serialize_round_trip() {
        let mut meta = XmpMeta::new();
        let ns = "http://purl.org/dc/elements/1.1/";
        let property = "title";

        // Set localized texts
        meta.set_localized_text(ns, property, "", "x-default", "Default Title")
            .unwrap();
        meta.set_localized_text(ns, property, "en", "en-US", "English Title")
            .unwrap();

        // Serialize
        let serialized = meta.serialize_packet().unwrap();

        // Parse back
        let meta2 = XmpMeta::parse(&serialized).unwrap();

        // Verify round-trip
        let (value1, lang1) = meta2
            .get_localized_text(ns, property, "", "x-default")
            .expect("Failed to get localized text for x-default");
        assert_eq!(value1, "Default Title");
        assert_eq!(lang1, "x-default");

        let (value2, lang2) = meta2
            .get_localized_text(ns, property, "en", "en-US")
            .unwrap();
        assert_eq!(value2, "English Title");
        assert_eq!(lang2, "en-US");
    }

    #[test]
    fn test_parse_preserves_custom_namespaces_for_round_trip() {
        let xml = r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about=""
    xmlns:xmp="http://ns.adobe.com/xap/1.0/"
    xmlns:lightroom="http://ns.adobe.com/lightroom/1.0/">
   <xmp:CreatorTool>NIKON Z 7 Ver.03.40</xmp:CreatorTool>
   <lightroom:hierarchicalSubject>
    <rdf:Bag>
     <rdf:li>Thailand</rdf:li>
     <rdf:li>Thailand|Phuket</rdf:li>
    </rdf:Bag>
   </lightroom:hierarchicalSubject>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#;

        let mut meta = XmpMeta::parse(xml).unwrap();

        assert_eq!(
            meta.get_property("xmp", "CreatorTool"),
            Some(XmpValue::String("NIKON Z 7 Ver.03.40".to_string()))
        );
        assert_eq!(
            meta.get_array_size("lightroom", "hierarchicalSubject"),
            Some(2)
        );

        meta.set_property(
            "xmp",
            "ModifyDate",
            XmpValue::DateTime("2024-01-01T00:00:00Z".to_string()),
        )
        .unwrap();

        let serialized = meta.serialize_packet().unwrap();
        assert!(serialized.contains("xmlns:lightroom=\"http://ns.adobe.com/lightroom/1.0/\""));
        assert!(serialized.contains("<lightroom:hierarchicalSubject>"));

        let reparsed = XmpMeta::parse(&serialized).unwrap();
        assert_eq!(
            reparsed.get_property("xmp", "ModifyDate"),
            Some(XmpValue::String("2024-01-01T00:00:00Z".to_string()))
        );
    }

    #[test]
    fn test_date_time_set_and_get() {
        use crate::utils::datetime::XmpDateTime;

        let mut meta = XmpMeta::new();
        let ns = "http://ns.adobe.com/xap/1.0/";
        let property = "ModifyDate";

        // Create a date/time value
        let mut dt = XmpDateTime::new();
        dt.has_date = true;
        dt.has_time = true;
        dt.year = 2023;
        dt.month = 12;
        dt.day = 25;
        dt.hour = 10;
        dt.minute = 30;
        dt.second = 0;
        dt.has_timezone = true;
        dt.tz_sign = 0; // UTC

        // Set date/time
        meta.set_date_time(ns, property, &dt).unwrap();

        // Get date/time
        let retrieved_dt = meta.get_date_time(ns, property).unwrap();
        assert_eq!(retrieved_dt.year, 2023);
        assert_eq!(retrieved_dt.month, 12);
        assert_eq!(retrieved_dt.day, 25);
        assert_eq!(retrieved_dt.hour, 10);
        assert_eq!(retrieved_dt.minute, 30);
        assert_eq!(retrieved_dt.second, 0);
        assert!(retrieved_dt.has_timezone);
        assert_eq!(retrieved_dt.tz_sign, 0);
    }

    #[test]
    fn test_date_time_serialize_round_trip() {
        let mut meta = XmpMeta::new();
        let ns = "http://ns.adobe.com/xap/1.0/";
        let property = "ModifyDate";

        // Set date/time via string
        meta.set_property(
            ns,
            property,
            XmpValue::DateTime("2023-12-25T10:30:00Z".to_string()),
        )
        .unwrap();

        // Serialize
        let serialized = meta.serialize_packet().unwrap();

        // Parse back
        let meta2 = XmpMeta::parse(&serialized).unwrap();

        // Verify round-trip
        let dt = meta2.get_date_time(ns, property).unwrap();
        assert_eq!(dt.year, 2023);
        assert_eq!(dt.month, 12);
        assert_eq!(dt.day, 25);
        assert_eq!(dt.hour, 10);
        assert_eq!(dt.minute, 30);
        assert_eq!(dt.second, 0);
    }

    #[test]
    fn test_date_time_partial_dates() {
        use crate::utils::datetime::XmpDateTime;

        let mut meta = XmpMeta::new();
        let ns = "http://purl.org/dc/elements/1.1/";
        let property = "date";

        // Test year only
        let mut dt = XmpDateTime::new();
        dt.has_date = true;
        dt.year = 2023;
        meta.set_date_time(ns, property, &dt).unwrap();
        let retrieved = meta.get_date_time(ns, property).unwrap();
        assert_eq!(retrieved.year, 2023);
        assert_eq!(retrieved.month, 0);
        assert_eq!(retrieved.day, 0);

        // Test year-month
        let mut dt = XmpDateTime::new();
        dt.has_date = true;
        dt.year = 2023;
        dt.month = 12;
        meta.set_date_time(ns, property, &dt).unwrap();
        let retrieved = meta.get_date_time(ns, property).unwrap();
        assert_eq!(retrieved.year, 2023);
        assert_eq!(retrieved.month, 12);
        assert_eq!(retrieved.day, 0);
    }
}
