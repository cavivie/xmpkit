//! WebAssembly bindings for XMP qualifiers

use crate::types::qualifier::Qualifier as RustQualifier;
use wasm_bindgen::prelude::*;

/// A qualifier for an XMP property
///
/// Qualifiers provide additional information about XMP properties.
/// They can be used to add language information, type information, etc.
#[wasm_bindgen]
#[derive(Clone)]
pub struct Qualifier {
    inner: RustQualifier,
}

#[wasm_bindgen]
impl Qualifier {
    /// Create a new qualifier
    ///
    /// # Arguments
    /// * `namespace` - Namespace URI (e.g., "http://ns.adobe.com/xap/1.0/")
    /// * `name` - Qualifier name (e.g., "lang")
    /// * `value` - Qualifier value (e.g., "en-US")
    #[wasm_bindgen(constructor)]
    pub fn new(namespace: String, name: String, value: String) -> Qualifier {
        Qualifier {
            inner: RustQualifier::new(namespace, name, value),
        }
    }

    /// Get the namespace URI of the qualifier
    #[wasm_bindgen(getter)]
    pub fn namespace(&self) -> String {
        self.inner.namespace.clone()
    }

    /// Get the name of the qualifier
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.inner.name.clone()
    }

    /// Get the value of the qualifier
    #[wasm_bindgen(getter)]
    pub fn value(&self) -> String {
        self.inner.value.clone()
    }

    /// Get the full path of the qualifier (namespace:name)
    pub fn path(&self) -> String {
        self.inner.path()
    }
}
