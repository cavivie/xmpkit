//! WebAssembly bindings for XMP file operations

use crate::files::file::ReadOptions as RustReadOptions;
use crate::wasm::error::{xmp_error_to_wasm_error, XmpError};
use crate::wasm::meta::XmpMeta;
use crate::XmpFile as RustXmpFile;
use wasm_bindgen::prelude::*;

/// Options for reading XMP metadata from files or memory (WebAssembly)
///
/// Configure how XMP data is read and processed.
///
/// # Example
///
/// ```javascript
/// const options = new ReadOptions();
/// options.use_packet_scanning();
/// options.limited_scanning();
/// file.from_bytes_with(data, options);
/// ```
#[derive(Default)]
#[wasm_bindgen]
pub struct ReadOptions {
    inner: RustReadOptions,
}

#[wasm_bindgen]
impl ReadOptions {
    /// Create default options
    #[wasm_bindgen(constructor)]
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

/// XmpFile for WebAssembly
///
/// Provides the same API as Rust's `XmpFile`.
///
/// # Example
///
/// ```javascript
/// import init, { XmpFile, ReadOptions } from './pkg/xmpkit.js';
/// await init();
///
/// const file = new XmpFile();
/// file.from_bytes(fileData);
/// const meta = file.get_xmp();
/// if (meta) {
///     meta.set_property("http://ns.adobe.com/xap/1.0/", "CreatorTool", "MyApp");
///     file.put_xmp(meta);
/// }
/// const modifiedData = file.write_to_bytes();
/// ```
#[derive(Default)]
#[wasm_bindgen]
pub struct XmpFile {
    inner: RustXmpFile,
}

#[wasm_bindgen]
impl XmpFile {
    /// Create a new XmpFile instance
    #[wasm_bindgen(constructor)]
    pub fn new() -> XmpFile {
        XmpFile {
            inner: RustXmpFile::new(),
        }
    }

    /// Load XMP from file bytes
    pub fn from_bytes(&mut self, data: &[u8]) -> Result<(), XmpError> {
        self.inner.from_bytes(data).map_err(xmp_error_to_wasm_error)
    }

    /// Load XMP from file bytes with options
    ///
    /// # Arguments
    /// * `data` - File data as Uint8Array
    /// * `options` - Opening options (e.g., use_packet_scanning, limited_scanning)
    ///
    /// # Example
    ///
    /// ```javascript
    /// const options = new ReadOptions();
    /// options.use_packet_scanning();
    /// file.from_bytes_with(data, options);
    /// ```
    pub fn from_bytes_with(&mut self, data: &[u8], options: &ReadOptions) -> Result<(), XmpError> {
        self.inner
            .from_bytes_with(data, options.inner)
            .map_err(xmp_error_to_wasm_error)
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
    pub fn write_to_bytes(&mut self) -> Result<Vec<u8>, XmpError> {
        self.inner.write_to_bytes().map_err(xmp_error_to_wasm_error)
    }
}
