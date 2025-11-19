//! WebAssembly bindings for XMP value types

use wasm_bindgen::prelude::*;

/// XMP value type kind
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum XmpValueKind {
    /// String value
    String,
    /// Integer value
    Integer,
    /// Boolean value
    Boolean,
    /// Date/time value
    DateTime,
}

/// XMP property value types
///
/// Represents different types of values that can be stored in XMP properties.
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct XmpValue {
    kind: XmpValueKind,
    string_value: Option<String>,
    integer_value: Option<i64>,
    boolean_value: Option<bool>,
}

#[wasm_bindgen]
impl XmpValue {
    /// Create a string value
    #[wasm_bindgen(constructor)]
    pub fn string(s: String) -> XmpValue {
        XmpValue {
            kind: XmpValueKind::String,
            string_value: Some(s),
            integer_value: None,
            boolean_value: None,
        }
    }

    /// Create an integer value
    pub fn integer(i: i64) -> XmpValue {
        XmpValue {
            kind: XmpValueKind::Integer,
            string_value: None,
            integer_value: Some(i),
            boolean_value: None,
        }
    }

    /// Create a boolean value
    pub fn boolean(b: bool) -> XmpValue {
        XmpValue {
            kind: XmpValueKind::Boolean,
            string_value: None,
            integer_value: None,
            boolean_value: Some(b),
        }
    }

    /// Create a date/time value
    pub fn date_time(dt: String) -> XmpValue {
        XmpValue {
            kind: XmpValueKind::DateTime,
            string_value: Some(dt),
            integer_value: None,
            boolean_value: None,
        }
    }

    /// Get the value kind
    #[wasm_bindgen(getter)]
    pub fn kind(&self) -> XmpValueKind {
        self.kind
    }

    /// Get the value as a string, if it is a string type
    pub fn as_string(&self) -> Option<String> {
        self.string_value.clone()
    }

    /// Get the value as an integer, if it is an integer type
    pub fn as_integer(&self) -> Option<i64> {
        self.integer_value
    }

    /// Get the value as a boolean, if it is a boolean type
    pub fn as_boolean(&self) -> Option<bool> {
        self.boolean_value
    }

    /// Get the value as a date/time string, if it is a date/time type
    pub fn as_date_time(&self) -> Option<String> {
        if self.kind == XmpValueKind::DateTime {
            self.string_value.clone()
        } else {
            None
        }
    }
}
