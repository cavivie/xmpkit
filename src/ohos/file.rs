//! OpenHarmony bindings for XMP file operations

use crate::files::file::ReadOptions as RustReadOptions;
use crate::ohos::error::xmp_error_to_ohos_error;
use crate::ohos::meta::XmpMeta;
use crate::XmpFile as RustXmpFile;
use napi_derive_ohos::napi;
use napi_ohos::bindgen_prelude::*;

/// Options for reading XMP metadata from files or memory (OpenHarmony)
///
/// Configure how XMP data is read and processed.
#[derive(Default)]
#[napi]
pub struct ReadOptions {
    inner: RustReadOptions,
}

#[napi]
impl ReadOptions {
    /// Create default options
    #[napi(constructor)]
    pub fn new() -> ReadOptions {
        ReadOptions::default()
    }

    /// Force packet scanning (do not use smart handler)
    pub fn use_packet_scanning(&mut self) {
        self.inner = self.inner.use_packet_scanning();
    }

    /// Only packet scan files "known" to need scanning
    pub fn limited_scanning(&mut self) {
        self.inner = self.inner.limited_scanning();
    }

    /// Require the use of a smart handler
    pub fn use_smart_handler(&mut self) {
        self.inner = self.inner.use_smart_handler();
    }

    /// Be strict about only attempting to use the designated file handler
    pub fn strict(&mut self) {
        self.inner = self.inner.strict();
    }

    /// Only the XMP is wanted (allows optimizations)
    pub fn only_xmp(&mut self) {
        self.inner = self.inner.only_xmp();
    }
}

/// XmpFile for OpenHarmony
///
/// Provides the same API as Rust's `XmpFile`.
#[derive(Default)]
#[napi]
pub struct XmpFile {
    inner: RustXmpFile,
}

#[napi]
impl XmpFile {
    /// Create a new XmpFile instance
    #[napi(constructor)]
    pub fn new() -> XmpFile {
        XmpFile {
            inner: RustXmpFile::new(),
        }
    }

    /// Load XMP from file bytes
    pub fn from_bytes(&mut self, data: Buffer) -> Result<()> {
        self.inner
            .from_bytes(data.as_ref())
            .map_err(|e| Error::from_reason(format!("{}", xmp_error_to_ohos_error(e))))
    }

    /// Load XMP from file bytes with options
    pub fn from_bytes_with(&mut self, data: Buffer, options: &ReadOptions) -> Result<()> {
        self.inner
            .from_bytes_with(data.as_ref(), options.inner)
            .map_err(|e| Error::from_reason(format!("{}", xmp_error_to_ohos_error(e))))
    }

    /// Get XMP metadata (returns an XmpMeta instance)
    pub fn get_xmp(&self) -> Option<XmpMeta> {
        self.inner.get_xmp().map(|meta| XmpMeta {
            inner: meta.clone(),
        })
    }

    /// Set XMP metadata
    pub fn put_xmp(&mut self, meta: XmpMeta) {
        self.inner.put_xmp(meta.inner);
    }

    /// Write file to bytes
    pub fn write_to_bytes(&mut self) -> Result<Buffer> {
        self.inner
            .write_to_bytes()
            .map(Buffer::from)
            .map_err(|e| Error::from_reason(format!("{}", xmp_error_to_ohos_error(e))))
    }
}
