//! Error types for XMP operations
//!
//! This module defines all error types used throughout the XMP Toolkit.

use thiserror::Error;

/// Error types for XMP operations
#[derive(Debug, Error)]
pub enum XmpError {
    /// Bad parameter provided to a function
    #[error("Bad parameter: {0}")]
    BadParam(String),

    /// Bad value provided (e.g., invalid property value)
    #[error("Bad value: {0}")]
    BadValue(String),

    /// Bad schema URI or namespace
    #[error("Bad schema: {0}")]
    BadSchema(String),

    /// Bad XPath expression
    #[error("Bad XPath: {0}")]
    BadXPath(String),

    /// Parse error (XML/RDF parsing failed)
    #[error("Parse error: {0}")]
    ParseError(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Internal error (should not occur in normal operation)
    #[error("Internal error: {0}")]
    InternalError(String),

    /// Resource not found
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Operation not supported
    #[error("Operation not supported: {0}")]
    NotSupported(String),
}

/// Result type alias for XMP operations
pub type XmpResult<T> = Result<T, XmpError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = XmpError::BadParam("test".to_string());
        assert!(err.to_string().contains("Bad parameter: test"));
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let xmp_err: XmpError = io_err.into();
        assert!(matches!(xmp_err, XmpError::IoError(_)));
    }
}
