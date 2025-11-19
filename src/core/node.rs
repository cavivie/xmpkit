//! XMP node types
//!
//! This module defines the node types used in the XMP data model:
//! - SimpleNode: A simple value node
//! - ArrayNode: An array of nodes (ordered, unordered, or alternative)
//! - StructureNode: A structure containing named fields

use crate::core::error::{XmpError, XmpResult};
use crate::types::qualifier::Qualifier;
use std::collections::HashMap;

/// Type of array node
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrayType {
    /// Ordered array (rdf:Seq)
    Ordered,
    /// Unordered array (rdf:Bag)
    Unordered,
    /// Alternative array (rdf:Alt)
    Alternative,
}

impl ArrayType {
    /// Get the RDF type name for this array type
    pub fn rdf_type(&self) -> &'static str {
        match self {
            ArrayType::Ordered => "Seq",
            ArrayType::Unordered => "Bag",
            ArrayType::Alternative => "Alt",
        }
    }
}

/// A simple value node
#[derive(Debug, Clone)]
pub struct SimpleNode {
    /// The value of the node
    pub value: String,
    /// Qualifiers attached to this node
    pub qualifiers: Vec<Qualifier>,
}

impl SimpleNode {
    /// Create a new simple node
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            qualifiers: Vec::new(),
        }
    }

    /// Add a qualifier to this node
    pub fn add_qualifier(&mut self, qualifier: Qualifier) {
        self.qualifiers.push(qualifier);
    }

    /// Get a qualifier by name
    pub fn get_qualifier(&self, namespace: &str, name: &str) -> Option<&Qualifier> {
        self.qualifiers
            .iter()
            .find(|q| q.namespace == namespace && q.name == name)
    }

    /// Remove a qualifier
    pub fn remove_qualifier(&mut self, namespace: &str, name: &str) -> bool {
        let initial_len = self.qualifiers.len();
        self.qualifiers
            .retain(|q| !(q.namespace == namespace && q.name == name));
        self.qualifiers.len() < initial_len
    }
}

/// An array node containing multiple child nodes
#[derive(Debug, Clone)]
pub struct ArrayNode {
    /// The items in the array
    pub items: Vec<Node>,
    /// The type of array
    pub array_type: ArrayType,
    /// Qualifiers attached to this node
    pub qualifiers: Vec<Qualifier>,
}

impl ArrayNode {
    /// Create a new array node
    pub fn new(array_type: ArrayType) -> Self {
        Self {
            items: Vec::new(),
            array_type,
            qualifiers: Vec::new(),
        }
    }

    /// Get the number of items in the array
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if the array is empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Get an item by index
    pub fn get(&self, index: usize) -> Option<&Node> {
        self.items.get(index)
    }

    /// Get a mutable reference to an item by index
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Node> {
        self.items.get_mut(index)
    }

    /// Append an item to the array
    pub fn append(&mut self, node: Node) {
        self.items.push(node);
    }

    /// Insert an item at a specific index
    pub fn insert(&mut self, index: usize, node: Node) -> XmpResult<()> {
        if index > self.items.len() {
            return Err(XmpError::BadParam(format!(
                "Index {} out of bounds for array of length {}",
                index,
                self.items.len()
            )));
        }
        self.items.insert(index, node);
        Ok(())
    }

    /// Remove an item at a specific index
    pub fn remove(&mut self, index: usize) -> XmpResult<Node> {
        if index >= self.items.len() {
            return Err(XmpError::BadParam(format!(
                "Index {} out of bounds for array of length {}",
                index,
                self.items.len()
            )));
        }
        Ok(self.items.remove(index))
    }

    /// Add a qualifier to this node
    pub fn add_qualifier(&mut self, qualifier: Qualifier) {
        self.qualifiers.push(qualifier);
    }

    /// Get a qualifier by name
    pub fn get_qualifier(&self, namespace: &str, name: &str) -> Option<&Qualifier> {
        self.qualifiers
            .iter()
            .find(|q| q.namespace == namespace && q.name == name)
    }
}

/// A structure node containing named fields
#[derive(Debug, Clone)]
pub struct StructureNode {
    /// The fields in the structure
    pub fields: HashMap<String, Node>,
    /// Qualifiers attached to this node
    pub qualifiers: Vec<Qualifier>,
}

impl StructureNode {
    /// Create a new structure node
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
            qualifiers: Vec::new(),
        }
    }

    /// Get a field by name
    pub fn get_field(&self, name: &str) -> Option<&Node> {
        self.fields.get(name)
    }

    /// Get a mutable reference to a field by name
    pub fn get_field_mut(&mut self, name: &str) -> Option<&mut Node> {
        self.fields.get_mut(name)
    }

    /// Set a field
    pub fn set_field(&mut self, name: impl Into<String>, node: Node) {
        self.fields.insert(name.into(), node);
    }

    /// Remove a field
    pub fn remove_field(&mut self, name: &str) -> Option<Node> {
        self.fields.remove(name)
    }

    /// Check if a field exists
    pub fn has_field(&self, name: &str) -> bool {
        self.fields.contains_key(name)
    }

    /// Get all field names
    pub fn field_names(&self) -> impl Iterator<Item = &String> {
        self.fields.keys()
    }

    /// Add a qualifier to this node
    pub fn add_qualifier(&mut self, qualifier: Qualifier) {
        self.qualifiers.push(qualifier);
    }

    /// Get a qualifier by name
    pub fn get_qualifier(&self, namespace: &str, name: &str) -> Option<&Qualifier> {
        self.qualifiers
            .iter()
            .find(|q| q.namespace == namespace && q.name == name)
    }
}

impl Default for StructureNode {
    fn default() -> Self {
        Self::new()
    }
}

/// A node in the XMP data model
#[derive(Debug, Clone)]
pub enum Node {
    /// A simple value node
    Simple(SimpleNode),
    /// An array node
    Array(ArrayNode),
    /// A structure node
    Structure(StructureNode),
}

impl Node {
    /// Create a new simple node
    pub fn simple(value: impl Into<String>) -> Self {
        Node::Simple(SimpleNode::new(value))
    }

    /// Create a new array node
    pub fn array(array_type: ArrayType) -> Self {
        Node::Array(ArrayNode::new(array_type))
    }

    /// Create a new structure node
    pub fn structure() -> Self {
        Node::Structure(StructureNode::new())
    }

    /// Check if this is a simple node
    pub fn is_simple(&self) -> bool {
        matches!(self, Node::Simple(_))
    }

    /// Check if this is an array node
    pub fn is_array(&self) -> bool {
        matches!(self, Node::Array(_))
    }

    /// Check if this is a structure node
    pub fn is_structure(&self) -> bool {
        matches!(self, Node::Structure(_))
    }

    /// Get the simple node, if this is a simple node
    pub fn as_simple(&self) -> Option<&SimpleNode> {
        match self {
            Node::Simple(node) => Some(node),
            _ => None,
        }
    }

    /// Get the array node, if this is an array node
    pub fn as_array(&self) -> Option<&ArrayNode> {
        match self {
            Node::Array(node) => Some(node),
            _ => None,
        }
    }

    /// Get the structure node, if this is a structure node
    pub fn as_structure(&self) -> Option<&StructureNode> {
        match self {
            Node::Structure(node) => Some(node),
            _ => None,
        }
    }

    /// Get a mutable reference to the simple node, if this is a simple node
    pub fn as_simple_mut(&mut self) -> Option<&mut SimpleNode> {
        match self {
            Node::Simple(node) => Some(node),
            _ => None,
        }
    }

    /// Get a mutable reference to the array node, if this is an array node
    pub fn as_array_mut(&mut self) -> Option<&mut ArrayNode> {
        match self {
            Node::Array(node) => Some(node),
            _ => None,
        }
    }

    /// Get a mutable reference to the structure node, if this is a structure node
    pub fn as_structure_mut(&mut self) -> Option<&mut StructureNode> {
        match self {
            Node::Structure(node) => Some(node),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_node() {
        let mut node = SimpleNode::new("test");
        assert_eq!(node.value, "test");
        assert_eq!(node.qualifiers.len(), 0);

        let qual = Qualifier::new("http://ns.adobe.com/xap/1.0/", "lang", "en-US");
        node.add_qualifier(qual.clone());
        assert_eq!(node.qualifiers.len(), 1);
        assert_eq!(
            node.get_qualifier("http://ns.adobe.com/xap/1.0/", "lang"),
            Some(&qual)
        );
    }

    #[test]
    fn test_array_node() {
        let mut array = ArrayNode::new(ArrayType::Ordered);
        assert_eq!(array.len(), 0);
        assert!(array.is_empty());

        array.append(Node::simple("item1"));
        array.append(Node::simple("item2"));
        assert_eq!(array.len(), 2);

        assert_eq!(
            array.get(0).and_then(|n| n.as_simple()).map(|n| &n.value),
            Some(&"item1".to_string())
        );
        assert_eq!(
            array.get(1).and_then(|n| n.as_simple()).map(|n| &n.value),
            Some(&"item2".to_string())
        );

        let removed = array.remove(0).unwrap();
        assert_eq!(array.len(), 1);
        assert_eq!(
            removed.as_simple().map(|n| &n.value),
            Some(&"item1".to_string())
        );
    }

    #[test]
    fn test_structure_node() {
        let mut structure = StructureNode::new();
        assert!(!structure.has_field("field1"));

        structure.set_field("field1", Node::simple("value1"));
        assert!(structure.has_field("field1"));
        assert_eq!(
            structure
                .get_field("field1")
                .and_then(|n| n.as_simple())
                .map(|n| &n.value),
            Some(&"value1".to_string())
        );

        structure.remove_field("field1");
        assert!(!structure.has_field("field1"));
    }

    #[test]
    fn test_node_creation() {
        let simple = Node::simple("test");
        assert!(simple.is_simple());

        let array = Node::array(ArrayType::Ordered);
        assert!(array.is_array());

        let structure = Node::structure();
        assert!(structure.is_structure());
    }

    #[test]
    fn test_array_type_rdf() {
        assert_eq!(ArrayType::Ordered.rdf_type(), "Seq");
        assert_eq!(ArrayType::Unordered.rdf_type(), "Bag");
        assert_eq!(ArrayType::Alternative.rdf_type(), "Alt");
    }
}
