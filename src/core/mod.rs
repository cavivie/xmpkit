//! XMP Core module
//!
//! This module contains the core functionality for XMP metadata processing,
//! including parsing, manipulation, and serialization.

pub mod error;
pub mod metadata;
pub mod namespace;
pub mod node;
pub mod parser;
pub mod serializer;
pub mod xpath;

pub use error::{XmpError, XmpResult};
pub use metadata::XmpMeta;
pub use namespace::{
    get_all_registered_namespaces, get_builtin_namespace_uris, get_global_namespace_prefix,
    get_global_namespace_uri, register_namespace, NamespaceMap,
};
pub use node::{ArrayNode, ArrayType, Node, SimpleNode, StructureNode};
pub use parser::XmpParser;
pub use serializer::XmpSerializer;
pub use xpath::{build_path, parse_path, PathComponent, PathComponents};
