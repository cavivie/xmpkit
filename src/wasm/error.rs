//! WebAssembly error handling for XMP operations

use crate::core::error::XmpError as RustXmpError;
use wasm_bindgen::prelude::*;

/// WebAssembly error type for XMP operations
///
/// This provides structured error information that JavaScript can inspect.
///
/// # Example
///
/// ```javascript
/// import { XmpErrorKind } from './pkg/xmpkit.js';
/// try {
///     file.from_bytes(data);
/// } catch (error) {
///     if (error instanceof XmpError) {
///         if (error.kind === XmpErrorKind.BadParam) {
///             console.log("Bad parameter error:", error.message);
///         }
///     }
/// }
/// ```
#[wasm_bindgen]
pub struct XmpError {
    kind: XmpErrorKind,
    message: String,
}

impl std::fmt::Display for XmpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

/// XMP Error kinds (exported enum for JavaScript)
///
/// These enum values can be used in JavaScript to check error types:
///
/// ```javascript
/// import { XmpErrorKind } from './pkg/xmpkit.js';
/// try {
///     file.from_bytes(data);
/// } catch (error) {
///     if (error.kind() === XmpErrorKind.BadParam) {
///         console.log("Bad parameter error");
///     }
/// }
/// ```
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum XmpErrorKind {
    /// Bad parameter error
    BadParam,
    /// Bad value error
    BadValue,
    /// Bad schema error
    BadSchema,
    /// Bad XPath error
    BadXPath,
    /// Parse error
    ParseError,
    /// Serialization error
    SerializationError,
    /// IO error
    IoError,
    /// Internal error
    InternalError,
    /// Not found error
    NotFound,
    /// Not supported error
    NotSupported,
}

#[wasm_bindgen]
impl XmpError {
    /// Get the error kind enum value
    #[wasm_bindgen(getter)]
    pub fn kind(&self) -> XmpErrorKind {
        self.kind
    }

    /// Get the error message
    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.message.clone()
    }
}

/// Convert Rust XmpError to WebAssembly XmpError
pub(crate) fn xmp_error_to_wasm_error(err: RustXmpError) -> XmpError {
    let (kind, message) = match &err {
        RustXmpError::BadParam(msg) => (XmpErrorKind::BadParam, msg.clone()),
        RustXmpError::BadValue(msg) => (XmpErrorKind::BadValue, msg.clone()),
        RustXmpError::BadSchema(msg) => (XmpErrorKind::BadSchema, msg.clone()),
        RustXmpError::BadXPath(msg) => (XmpErrorKind::BadXPath, msg.clone()),
        RustXmpError::ParseError(msg) => (XmpErrorKind::ParseError, msg.clone()),
        RustXmpError::SerializationError(msg) => (XmpErrorKind::SerializationError, msg.clone()),
        RustXmpError::IoError(io_err) => (XmpErrorKind::IoError, io_err.to_string()),
        RustXmpError::InternalError(msg) => (XmpErrorKind::InternalError, msg.clone()),
        RustXmpError::NotFound(msg) => (XmpErrorKind::NotFound, msg.clone()),
        RustXmpError::NotSupported(msg) => (XmpErrorKind::NotSupported, msg.clone()),
    };
    XmpError { kind, message }
}
