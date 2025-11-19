//! WebAssembly bindings for XMP metadata operations

use crate::wasm::error::{xmp_error_to_wasm_error, XmpError};
use crate::{XmpMeta as RustXmpMeta, XmpValue};
use wasm_bindgen::prelude::*;

/// XmpMeta for WebAssembly
///
/// Provides the same API as Rust's `XmpMeta`.
///
/// # Example
///
/// ```javascript
/// import init, { XmpMeta } from './pkg/xmpkit.js';
/// await init();
///
/// const meta = XmpMeta.parse(xmpPacketString);
/// const creatorTool = meta.get_property("http://ns.adobe.com/xap/1.0/", "CreatorTool");
/// meta.set_property("http://ns.adobe.com/xap/1.0/", "CreatorTool", "MyApp");
/// const serialized = meta.serialize();
/// ```
#[wasm_bindgen]
#[derive(Default)]
pub struct XmpMeta {
    pub(crate) inner: RustXmpMeta,
}

#[wasm_bindgen]
impl XmpMeta {
    /// Create a new empty XmpMeta instance
    #[wasm_bindgen(constructor)]
    pub fn new() -> XmpMeta {
        XmpMeta::default()
    }

    /// Parse XMP packet string
    pub fn parse(xmp_packet: &str) -> Result<XmpMeta, XmpError> {
        RustXmpMeta::parse(xmp_packet)
            .map(|meta| XmpMeta { inner: meta })
            .map_err(xmp_error_to_wasm_error)
    }

    /// Get a property value
    ///
    /// Returns the property value as a string, or null if not found.
    /// For complex types, returns a JSON string representation.
    ///
    /// # Arguments
    /// * `namespace` - Namespace URI (e.g., "http://ns.adobe.com/xap/1.0/")
    /// * `property` - Property name (e.g., "CreatorTool", "title")
    pub fn get_property(&self, namespace: &str, property: &str) -> Option<String> {
        self.inner.get_property(namespace, property).map(|value| {
            match value {
                XmpValue::String(s) => s,
                XmpValue::Integer(i) => i.to_string(),
                XmpValue::Boolean(b) => b.to_string(),
                XmpValue::DateTime(d) => d,
                _ => format!("{:?}", value), // Fallback for complex types
            }
        })
    }

    /// Set a property value
    ///
    /// # Arguments
    /// * `namespace` - Namespace URI (e.g., "http://ns.adobe.com/xap/1.0/")
    /// * `property` - Property name (e.g., "CreatorTool", "title")
    /// * `value` - Property value as string
    pub fn set_property(
        &mut self,
        namespace: &str,
        property: &str,
        value: &str,
    ) -> Result<(), XmpError> {
        self.inner
            .set_property(namespace, property, XmpValue::String(value.to_string()))
            .map_err(xmp_error_to_wasm_error)
    }

    /// Serialize to RDF/XML string
    pub fn serialize(&self) -> Result<String, XmpError> {
        self.inner.serialize().map_err(xmp_error_to_wasm_error)
    }

    /// Serialize to XMP packet string (with <?xpacket> wrapper)
    pub fn serialize_packet(&self) -> Result<String, XmpError> {
        self.inner
            .serialize_packet()
            .map_err(xmp_error_to_wasm_error)
    }

    /// Check if a property exists
    pub fn has_property(&self, namespace: &str, path: &str) -> bool {
        self.inner.has_property(namespace, path)
    }

    /// Delete a property
    pub fn delete_property(&mut self, namespace: &str, path: &str) -> Result<(), XmpError> {
        self.inner
            .delete_property(namespace, path)
            .map_err(xmp_error_to_wasm_error)
    }

    /// Get an array item by index
    pub fn get_array_item(&self, namespace: &str, path: &str, index: usize) -> Option<String> {
        self.inner
            .get_array_item(namespace, path, index)
            .map(|value| match value {
                XmpValue::String(s) => s,
                XmpValue::Integer(i) => i.to_string(),
                XmpValue::Boolean(b) => b.to_string(),
                XmpValue::DateTime(d) => d,
                _ => format!("{:?}", value),
            })
    }

    /// Get the size of an array property
    pub fn get_array_size(&self, namespace: &str, path: &str) -> Option<usize> {
        self.inner.get_array_size(namespace, path)
    }

    /// Append an item to an array property
    pub fn append_array_item(
        &mut self,
        namespace: &str,
        path: &str,
        value: &str,
    ) -> Result<(), XmpError> {
        self.inner
            .append_array_item(namespace, path, XmpValue::String(value.to_string()))
            .map_err(xmp_error_to_wasm_error)
    }

    /// Insert an item into an array property at a specific index
    pub fn insert_array_item(
        &mut self,
        namespace: &str,
        path: &str,
        index: usize,
        value: &str,
    ) -> Result<(), XmpError> {
        self.inner
            .insert_array_item(namespace, path, index, XmpValue::String(value.to_string()))
            .map_err(xmp_error_to_wasm_error)
    }

    /// Delete an item from an array property
    pub fn delete_array_item(
        &mut self,
        namespace: &str,
        path: &str,
        index: usize,
    ) -> Result<(), XmpError> {
        self.inner
            .delete_array_item(namespace, path, index)
            .map_err(xmp_error_to_wasm_error)
    }

    /// Get a struct field value
    pub fn get_struct_field(
        &self,
        namespace: &str,
        struct_path: &str,
        field: &str,
    ) -> Option<String> {
        self.inner
            .get_struct_field(namespace, struct_path, field)
            .map(|value| match value {
                XmpValue::String(s) => s,
                XmpValue::Integer(i) => i.to_string(),
                XmpValue::Boolean(b) => b.to_string(),
                XmpValue::DateTime(d) => d,
                _ => format!("{:?}", value),
            })
    }

    /// Set a struct field value
    pub fn set_struct_field(
        &mut self,
        namespace: &str,
        struct_path: &str,
        field: &str,
        value: &str,
    ) -> Result<(), XmpError> {
        self.inner
            .set_struct_field(
                namespace,
                struct_path,
                field,
                XmpValue::String(value.to_string()),
            )
            .map_err(xmp_error_to_wasm_error)
    }

    /// Delete a struct field
    pub fn delete_struct_field(
        &mut self,
        namespace: &str,
        struct_path: &str,
        field: &str,
    ) -> Result<(), XmpError> {
        self.inner
            .delete_struct_field(namespace, struct_path, field)
            .map_err(xmp_error_to_wasm_error)
    }

    /// Get the about URI
    pub fn about_uri(&self) -> Option<String> {
        self.inner.about_uri().map(|s| s.to_string())
    }

    /// Set the about URI
    pub fn set_about_uri(&mut self, uri: &str) {
        self.inner.set_about_uri(uri);
    }
}
