//! XMP value types
//!
//! This module defines the value types that can be stored in XMP properties.

use std::fmt;

/// XMP property value types
#[derive(Debug, Clone, PartialEq)]
pub enum XmpValue {
    /// String value
    String(String),
    /// Integer value
    Integer(i64),
    /// Boolean value
    Boolean(bool),
    /// Date/time value (ISO 8601 format)
    DateTime(String),
    /// Array of values
    Array(Vec<XmpValue>),
    /// Structure (key-value pairs)
    Structure(std::collections::HashMap<String, XmpValue>),
}

impl XmpValue {
    /// Get the value as a string, if it is a string type
    pub fn as_str(&self) -> Option<&str> {
        match self {
            XmpValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// Get the value as an integer, if it is an integer type
    pub fn as_int(&self) -> Option<i64> {
        match self {
            XmpValue::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// Get the value as a boolean, if it is a boolean type
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            XmpValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }
}

impl fmt::Display for XmpValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            XmpValue::String(s) => write!(f, "{}", s),
            XmpValue::Integer(i) => write!(f, "{}", i),
            XmpValue::Boolean(b) => write!(f, "{}", b),
            XmpValue::DateTime(dt) => write!(f, "{}", dt),
            XmpValue::Array(_) => write!(f, "[Array]"),
            XmpValue::Structure(_) => write!(f, "[Structure]"),
        }
    }
}

#[cfg(feature = "serde")]
impl serde::ser::Serialize for XmpValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        match self {
            XmpValue::String(s) => serializer.serialize_str(s),
            XmpValue::Integer(i) => serializer.serialize_i64(*i),
            XmpValue::Boolean(b) => serializer.serialize_bool(*b),
            XmpValue::DateTime(dt) => serializer.serialize_str(dt),
            XmpValue::Array(arr) => arr.serialize(serializer),
            XmpValue::Structure(structure) => structure.serialize(serializer),
        }
    }
}

impl From<String> for XmpValue {
    fn from(s: String) -> Self {
        XmpValue::String(s)
    }
}

impl From<&str> for XmpValue {
    fn from(s: &str) -> Self {
        XmpValue::String(s.to_string())
    }
}

impl From<i64> for XmpValue {
    fn from(i: i64) -> Self {
        XmpValue::Integer(i)
    }
}

impl From<bool> for XmpValue {
    fn from(b: bool) -> Self {
        XmpValue::Boolean(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xmp_value_string() {
        let value = XmpValue::String("test".to_string());
        assert_eq!(value.as_str(), Some("test"));
        assert_eq!(value.to_string(), "test"); // Display trait
    }

    #[test]
    fn test_xmp_value_integer() {
        let value = XmpValue::Integer(42);
        assert_eq!(value.as_int(), Some(42));
        assert_eq!(value.to_string(), "42"); // Display trait
    }

    #[test]
    fn test_xmp_value_boolean() {
        let value = XmpValue::Boolean(true);
        assert_eq!(value.as_bool(), Some(true));
        assert_eq!(value.to_string(), "true"); // Display trait
    }

    #[test]
    fn test_xmp_value_from() {
        let value: XmpValue = "test".into();
        assert_eq!(value.as_str(), Some("test"));

        let value: XmpValue = 42.into();
        assert_eq!(value.as_int(), Some(42));

        let value: XmpValue = true.into();
        assert_eq!(value.as_bool(), Some(true));
    }
}
