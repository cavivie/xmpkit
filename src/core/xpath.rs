//! XPath path handling for XMP
//!
//! This module provides functionality for parsing and building XPath expressions
//! used in XMP property access.

use crate::core::error::{XmpError, XmpResult};

/// Parse an XPath-like path expression
///
/// Supports formats like:
/// - `xmp:CreatorTool` - simple property
/// - `dc:creator[1]` - array item with index
/// - `exif:Flash/Fired` - structure field
/// - `dc:creator[1]/title` - nested path
pub fn parse_path(path: &str) -> XmpResult<PathComponents> {
    let mut components = Vec::new();
    let mut current = String::new();
    let mut in_brackets = false;

    for ch in path.chars() {
        match ch {
            '[' => {
                if !current.is_empty() {
                    components.push(PathComponent::Name(current.clone()));
                    current.clear();
                }
                in_brackets = true;
            }
            ']' => {
                if in_brackets {
                    let index = current.parse::<usize>().map_err(|_| {
                        XmpError::BadXPath(format!("Invalid array index: {}", current))
                    })?;
                    components.push(PathComponent::Index(index));
                    current.clear();
                    in_brackets = false;
                } else {
                    return Err(XmpError::BadXPath("Unexpected ']'".to_string()));
                }
            }
            '/' => {
                if !current.is_empty() && !in_brackets {
                    components.push(PathComponent::Name(current.clone()));
                    current.clear();
                }
            }
            _ => {
                if !in_brackets || ch.is_ascii_digit() {
                    current.push(ch);
                } else {
                    return Err(XmpError::BadXPath(format!(
                        "Invalid character in index: {}",
                        ch
                    )));
                }
            }
        }
    }

    if !current.is_empty() && !in_brackets {
        components.push(PathComponent::Name(current));
    }

    if in_brackets {
        return Err(XmpError::BadXPath("Unclosed bracket".to_string()));
    }

    if components.is_empty() {
        return Err(XmpError::BadXPath("Empty path".to_string()));
    }

    Ok(PathComponents { components })
}

/// Build a path from components
pub fn build_path(components: &PathComponents) -> String {
    let mut result = String::new();
    for (i, comp) in components.components.iter().enumerate() {
        if i > 0 {
            match comp {
                PathComponent::Name(_) => result.push('/'),
                PathComponent::Index(_) => {}
            }
        }
        match comp {
            PathComponent::Name(name) => result.push_str(name),
            PathComponent::Index(idx) => {
                result.push('[');
                result.push_str(&idx.to_string());
                result.push(']');
            }
        }
    }
    result
}

/// A component of an XPath expression
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathComponent {
    /// A property or field name
    Name(String),
    /// An array index (1-based in XMP, but we use 0-based internally)
    Index(usize),
}

/// Parsed path components
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathComponents {
    pub components: Vec<PathComponent>,
}

impl PathComponents {
    /// Get the first component as a name
    pub fn first_name(&self) -> Option<&str> {
        self.components.first().and_then(|c| match c {
            PathComponent::Name(n) => Some(n.as_str()),
            _ => None,
        })
    }

    /// Get the last component
    pub fn last(&self) -> Option<&PathComponent> {
        self.components.last()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_path() {
        let path = parse_path("CreatorTool").unwrap();
        assert_eq!(path.components.len(), 1);
        assert_eq!(
            path.components[0],
            PathComponent::Name("CreatorTool".to_string())
        );
    }

    #[test]
    fn test_parse_array_path() {
        let path = parse_path("creator[1]").unwrap();
        assert_eq!(path.components.len(), 2);
        assert_eq!(
            path.components[0],
            PathComponent::Name("creator".to_string())
        );
        assert_eq!(path.components[1], PathComponent::Index(1));
    }

    #[test]
    fn test_parse_nested_path() {
        let path = parse_path("Flash/Fired").unwrap();
        assert_eq!(path.components.len(), 2);
        assert_eq!(path.components[0], PathComponent::Name("Flash".to_string()));
        assert_eq!(path.components[1], PathComponent::Name("Fired".to_string()));
    }

    #[test]
    fn test_build_path() {
        let components = PathComponents {
            components: vec![
                PathComponent::Name("creator".to_string()),
                PathComponent::Index(1),
            ],
        };
        assert_eq!(build_path(&components), "creator[1]");
    }
}
