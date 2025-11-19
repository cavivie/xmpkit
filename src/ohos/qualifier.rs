//! OpenHarmony bindings for XMP qualifiers

use crate::types::qualifier::Qualifier as RustQualifier;
use napi_derive_ohos::napi;

#[napi]
#[derive(Clone)]
pub struct Qualifier {
    inner: RustQualifier,
}

#[napi]
impl Qualifier {
    #[napi(constructor)]
    pub fn new(namespace: String, name: String, value: String) -> Qualifier {
        Qualifier {
            inner: RustQualifier::new(namespace, name, value),
        }
    }

    #[napi(getter)]
    pub fn namespace(&self) -> String {
        self.inner.namespace.clone()
    }
}
