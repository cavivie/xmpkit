//! OpenHarmony bindings for XMP metadata operations

use crate::ohos::error::xmp_error_to_ohos_error;
use crate::{XmpMeta as RustXmpMeta, XmpValue};
use napi_derive_ohos::napi;
use napi_ohos::bindgen_prelude::*;

/// XmpMeta for OpenHarmony
#[derive(Default)]
#[napi]
pub struct XmpMeta {
    pub(crate) inner: RustXmpMeta,
}

#[napi]
impl XmpMeta {
    /// Create a new empty XmpMeta instance
    #[napi(constructor)]
    pub fn new() -> XmpMeta {
        XmpMeta::default()
    }

    /// Parse XMP packet string
    pub fn parse(xmp_packet: String) -> Result<XmpMeta> {
        RustXmpMeta::parse(&xmp_packet)
            .map(|meta| XmpMeta { inner: meta })
            .map_err(|e| Error::from_reason(format!("{}", xmp_error_to_ohos_error(e))))
    }

    /// Get a property value
    pub fn get_property(&self, namespace: String, property: String) -> Option<String> {
        self.inner
            .get_property(&namespace, &property)
            .map(|value| match value {
                XmpValue::String(s) => s,
                XmpValue::Integer(i) => i.to_string(),
                XmpValue::Boolean(b) => b.to_string(),
                XmpValue::DateTime(d) => d,
                _ => format!("{:?}", value),
            })
    }

    /// Set a property value
    pub fn set_property(
        &mut self,
        namespace: String,
        property: String,
        value: String,
    ) -> Result<()> {
        self.inner
            .set_property(&namespace, &property, XmpValue::String(value))
            .map_err(|e| Error::from_reason(format!("{}", xmp_error_to_ohos_error(e))))
    }

    /// Serialize to RDF/XML string
    pub fn serialize(&self) -> Result<String> {
        self.inner
            .serialize()
            .map_err(|e| Error::from_reason(format!("{}", xmp_error_to_ohos_error(e))))
    }

    /// Serialize to XMP packet string
    pub fn serialize_packet(&self) -> Result<String> {
        self.inner
            .serialize_packet()
            .map_err(|e| Error::from_reason(format!("{}", xmp_error_to_ohos_error(e))))
    }

    /// Check if a property exists
    pub fn has_property(&self, namespace: String, path: String) -> bool {
        self.inner.has_property(&namespace, &path)
    }

    /// Delete a property
    pub fn delete_property(&mut self, namespace: String, path: String) -> Result<()> {
        self.inner
            .delete_property(&namespace, &path)
            .map_err(|e| Error::from_reason(format!("{}", xmp_error_to_ohos_error(e))))
    }
}
