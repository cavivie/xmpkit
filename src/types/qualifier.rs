//! Qualifier types for XMP
//!
//! Qualifiers provide additional information about XMP properties.
//! They can be used to add language information, type information, etc.

use std::fmt;

/// A qualifier for an XMP property
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Qualifier {
    /// The namespace URI of the qualifier
    pub namespace: String,
    /// The name of the qualifier
    pub name: String,
    /// The value of the qualifier
    pub value: String,
}

impl Qualifier {
    /// Create a new qualifier
    pub fn new(
        namespace: impl Into<String>,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        Self {
            namespace: namespace.into(),
            name: name.into(),
            value: value.into(),
        }
    }

    /// Get the full path of the qualifier (namespace:name)
    pub fn path(&self) -> String {
        format!("{}:{}", self.namespace, self.name)
    }
}

impl fmt::Display for Qualifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}={}", self.namespace, self.name, self.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qualifier_new() {
        let qual = Qualifier::new("http://ns.adobe.com/xap/1.0/", "lang", "en-US");
        assert_eq!(qual.namespace, "http://ns.adobe.com/xap/1.0/");
        assert_eq!(qual.name, "lang");
        assert_eq!(qual.value, "en-US");
    }

    #[test]
    fn test_qualifier_path() {
        let qual = Qualifier::new("http://ns.adobe.com/xap/1.0/", "lang", "en-US");
        assert_eq!(qual.path(), "http://ns.adobe.com/xap/1.0/:lang");
    }
}
