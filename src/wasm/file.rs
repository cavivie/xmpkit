//! WebAssembly bindings for XMP file operations

use crate::files::handler::XmpOptions as RustXmpOptions;
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
/// // For read-only operations (memory efficient)
/// const file = new XmpFile();
/// file.from_bytes(data);
/// const meta = file.get_xmp();
///
/// // For read and write operations
/// const options = new XmpOptions();
/// options.for_update();  // Required if you want to write changes
/// file.from_bytes_with(data, options);
/// // ... modify metadata ...
/// const modifiedData = file.write_to_bytes();
/// ```
#[derive(Default)]
#[wasm_bindgen]
pub struct XmpOptions {
    inner: RustXmpOptions,
}

#[wasm_bindgen]
impl XmpOptions {
    /// Create default options
    #[wasm_bindgen(constructor)]
    pub fn new() -> XmpOptions {
        XmpOptions::default()
    }

    /// Open for reading and writing
    ///
    /// This option is **required** if you want to use `write_to_bytes()` later.
    /// When enabled, the original file data is stored in memory for later writing.
    ///
    /// If you only need to read XMP metadata, you can skip this option to save memory.
    pub fn for_update(&mut self) {
        self.inner = self.inner.for_update();
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
/// import init, { XmpFile, XmpOptions } from './pkg/xmpkit.js';
/// await init();
///
/// // Read-only mode (memory efficient)
/// const file = new XmpFile();
/// file.from_bytes(fileData);
/// const meta = file.get_xmp();
///
/// // Read and write mode
/// const file2 = new XmpFile();
/// const options = new XmpOptions();
/// options.for_update();  // Required for write_to_bytes()
/// file2.from_bytes_with(fileData, options);
/// const meta2 = file2.get_xmp();
/// if (meta2) {
///     meta2.set_property("http://ns.adobe.com/xap/1.0/", "CreatorTool", "MyApp");
///     file2.put_xmp(meta2);
/// }
/// const modifiedData = file2.write_to_bytes();
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
    /// const options = new XmpOptions();
    /// options.use_packet_scanning();
    /// file.from_bytes_with(data, options);
    /// ```
    pub fn from_bytes_with(&mut self, data: &[u8], options: &XmpOptions) -> Result<(), XmpError> {
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
