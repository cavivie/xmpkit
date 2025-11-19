//! Namespace management for XMP
//!
//! This module handles XMP namespace registration, lookup, and management.
//! XMP uses namespaces to organize properties into schemas.

use crate::core::error::{XmpError, XmpResult};
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

// Global namespace registry for thread safety
static GLOBAL_NAMESPACE_MAP: OnceLock<RwLock<NamespaceMap>> = OnceLock::new();

/// Built-in XMP namespaces
pub mod ns {
    /// XMP Basic namespace
    pub const XMP: &str = "http://ns.adobe.com/xap/1.0/";
    /// Dublin Core namespace
    pub const DC: &str = "http://purl.org/dc/elements/1.1/";
    /// EXIF namespace
    pub const EXIF: &str = "http://ns.adobe.com/exif/1.0/";
    /// EXIF Aux namespace
    pub const EXIF_AUX: &str = "http://ns.adobe.com/exif/1.0/aux/";
    /// IPTC Core namespace
    pub const IPTC_CORE: &str = "http://iptc.org/std/Iptc4xmpCore/1.0/xmlns/";
    /// IPTC Extension namespace
    pub const IPTC_EXT: &str = "http://iptc.org/std/Iptc4xmpExt/2008-02-29/";
    /// Photoshop namespace
    pub const PHOTOSHOP: &str = "http://ns.adobe.com/photoshop/1.0/";
    /// Camera Raw namespace
    pub const CAMERA_RAW: &str = "http://ns.adobe.com/camera-raw-settings/1.0/";
    /// XMP Rights namespace
    pub const XMP_RIGHTS: &str = "http://ns.adobe.com/xap/1.0/rights/";
    /// XMP Media Management namespace
    pub const XMP_MM: &str = "http://ns.adobe.com/xap/1.0/mm/";
    /// XMP Basic Job Ticket namespace
    pub const XMP_BJ: &str = "http://ns.adobe.com/xap/1.0/bj/";
    /// TIFF namespace
    pub const TIFF: &str = "http://ns.adobe.com/tiff/1.0/";
    /// PDF namespace
    pub const PDF: &str = "http://ns.adobe.com/pdf/1.3/";
    /// PDF/X namespace
    pub const PDFX: &str = "http://ns.adobe.com/pdfx/1.3/";
    /// PDF/A namespace
    pub const PDFA: &str = "http://www.aiim.org/pdfa/ns/id/";
    /// XMP Dynamic Media namespace
    pub const XMP_DM: &str = "http://ns.adobe.com/xmp/1.0/DynamicMedia/";
    /// XMP PagedText namespace
    pub const XMP_PAGED: &str = "http://ns.adobe.com/xap/1.0/t/pg/";
    /// XMP Graphics namespace
    pub const XMP_GRAPHICS: &str = "http://ns.adobe.com/xap/1.0/g/";
    /// XMP Image namespace
    pub const XMP_IMAGE: &str = "http://ns.adobe.com/xap/1.0/g/img/";
    /// RDF namespace
    pub const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
    /// XML namespace (for xml:lang, etc.)
    pub const XML: &str = "http://www.w3.org/XML/1998/namespace";
    /// XMP namespace prefix
    pub const XMP_PREFIX: &str = "xmp";
    /// Dublin Core prefix
    pub const DC_PREFIX: &str = "dc";
    /// EXIF prefix
    pub const EXIF_PREFIX: &str = "exif";
    /// RDF prefix
    pub const RDF_PREFIX: &str = "rdf";
    /// XML prefix
    pub const XML_PREFIX: &str = "xml";
    /// EXIF Aux prefix
    pub const EXIF_AUX_PREFIX: &str = "exifEX";
    /// IPTC Core prefix
    pub const IPTC_CORE_PREFIX: &str = "Iptc4xmpCore";
    /// IPTC Extension prefix
    pub const IPTC_EXT_PREFIX: &str = "Iptc4xmpExt";
    /// Photoshop prefix
    pub const PHOTOSHOP_PREFIX: &str = "photoshop";
    /// Camera Raw prefix
    pub const CAMERA_RAW_PREFIX: &str = "crs";
    /// XMP Rights prefix
    pub const XMP_RIGHTS_PREFIX: &str = "xmpRights";
    /// XMP Media Management prefix
    pub const XMP_MM_PREFIX: &str = "xmpMM";
    /// XMP Basic Job Ticket prefix
    pub const XMP_BJ_PREFIX: &str = "xmpBJ";
    /// TIFF prefix
    pub const TIFF_PREFIX: &str = "tiff";
    /// PDF prefix
    pub const PDF_PREFIX: &str = "pdf";
    /// PDF/X prefix
    pub const PDFX_PREFIX: &str = "pdfx";
    /// PDF/A prefix
    pub const PDFA_PREFIX: &str = "pdfaid";
    /// XMP Dynamic Media prefix
    pub const XMP_DM_PREFIX: &str = "xmpDM";
    /// XMP PagedText prefix
    pub const XMP_PAGED_PREFIX: &str = "xmpTPg";
    /// XMP Graphics prefix
    pub const XMP_GRAPHICS_PREFIX: &str = "xmpG";
    /// XMP Image prefix
    pub const XMP_IMAGE_PREFIX: &str = "xmpGImg";
}

/// Map of namespace URI to prefix
#[derive(Debug, Clone, Default)]
pub struct NamespaceMap {
    uri_to_prefix: HashMap<String, String>,
    prefix_to_uri: HashMap<String, String>,
}

impl NamespaceMap {
    /// Create a new namespace map with built-in namespaces registered
    pub fn new() -> Self {
        let mut map = Self::default();
        map.register_builtin_namespaces();
        map
    }

    /// Register a namespace URI with a prefix
    ///
    /// # Arguments
    ///
    /// * `uri` - The namespace URI
    /// * `prefix` - The namespace prefix
    ///
    /// # Returns
    ///
    /// Returns an error if the prefix is already registered to a different URI
    pub fn register(&mut self, uri: &str, prefix: &str) -> XmpResult<()> {
        // Check if prefix is already registered to a different URI
        if let Some(existing_uri) = self.prefix_to_uri.get(prefix) {
            if existing_uri != uri {
                return Err(XmpError::BadParam(format!(
                    "Prefix '{}' is already registered to '{}'",
                    prefix, existing_uri
                )));
            }
            // Already registered to the same URI, no-op
            return Ok(());
        }

        self.uri_to_prefix
            .insert(uri.to_string(), prefix.to_string());
        self.prefix_to_uri
            .insert(prefix.to_string(), uri.to_string());
        Ok(())
    }

    /// Get the prefix for a namespace URI
    pub fn get_prefix(&self, uri: &str) -> Option<&str> {
        self.uri_to_prefix.get(uri).map(|s| s.as_str())
    }

    /// Get the URI for a namespace prefix
    pub fn get_uri(&self, prefix: &str) -> Option<&str> {
        self.prefix_to_uri.get(prefix).map(|s| s.as_str())
    }

    /// Check if a namespace URI is registered
    pub fn has_uri(&self, uri: &str) -> bool {
        self.uri_to_prefix.contains_key(uri)
    }

    /// Check if a namespace prefix is registered
    pub fn has_prefix(&self, prefix: &str) -> bool {
        self.prefix_to_uri.contains_key(prefix)
    }

    /// Get all registered namespaces as a vector of (uri, prefix) tuples
    pub fn get_all_namespaces(&self) -> Vec<(String, String)> {
        self.uri_to_prefix
            .iter()
            .map(|(uri, prefix)| (uri.clone(), prefix.clone()))
            .collect()
    }

    /// Register built-in XMP namespaces
    fn register_builtin_namespaces(&mut self) {
        // These should never fail, so we use unwrap
        self.register(ns::XMP, ns::XMP_PREFIX).unwrap();
        self.register(ns::DC, ns::DC_PREFIX).unwrap();
        self.register(ns::EXIF, ns::EXIF_PREFIX).unwrap();
        self.register(ns::RDF, ns::RDF_PREFIX).unwrap();
        self.register(ns::XML, ns::XML_PREFIX).unwrap();
        // Register others without prefix conflicts
        self.register(ns::EXIF_AUX, ns::EXIF_AUX_PREFIX).unwrap();
        self.register(ns::IPTC_CORE, ns::IPTC_CORE_PREFIX).unwrap();
        self.register(ns::IPTC_EXT, ns::IPTC_EXT_PREFIX).unwrap();
        self.register(ns::PHOTOSHOP, ns::PHOTOSHOP_PREFIX).unwrap();
        self.register(ns::CAMERA_RAW, ns::CAMERA_RAW_PREFIX)
            .unwrap();
        self.register(ns::XMP_RIGHTS, ns::XMP_RIGHTS_PREFIX)
            .unwrap();
        self.register(ns::XMP_MM, ns::XMP_MM_PREFIX).unwrap();
        self.register(ns::XMP_BJ, ns::XMP_BJ_PREFIX).unwrap();
        self.register(ns::TIFF, ns::TIFF_PREFIX).unwrap();
        self.register(ns::PDF, ns::PDF_PREFIX).unwrap();
        self.register(ns::PDFX, ns::PDFX_PREFIX).unwrap();
        self.register(ns::PDFA, ns::PDFA_PREFIX).unwrap();
        self.register(ns::XMP_DM, ns::XMP_DM_PREFIX).unwrap();
        self.register(ns::XMP_PAGED, ns::XMP_PAGED_PREFIX).unwrap();
        self.register(ns::XMP_GRAPHICS, ns::XMP_GRAPHICS_PREFIX)
            .unwrap();
        self.register(ns::XMP_IMAGE, ns::XMP_IMAGE_PREFIX).unwrap();
    }
}

fn get_global_namespace_map() -> &'static RwLock<NamespaceMap> {
    GLOBAL_NAMESPACE_MAP.get_or_init(|| RwLock::new(NamespaceMap::new()))
}

/// Register a namespace URI with a prefix
///
/// This is a convenience function that uses a global namespace map.
/// For per-instance namespace management, use `NamespaceMap` directly.
///
/// This function registers namespaces globally (per thread) for convenience.
pub fn register_namespace(uri: &str, prefix: &str) -> XmpResult<()> {
    if uri.is_empty() {
        return Err(XmpError::BadParam("URI cannot be empty".to_string()));
    }
    if prefix.is_empty() {
        return Err(XmpError::BadParam("Prefix cannot be empty".to_string()));
    }

    let map = get_global_namespace_map();
    // RwLock::write() only fails if the lock is poisoned (another thread panicked while holding the lock)
    // In wasm (single-threaded) or normal usage, this should never happen
    let mut guard = map.write().expect("Namespace registry lock poisoned");
    guard.register(uri, prefix)
}

/// Check if a namespace URI is registered globally
pub fn is_namespace_registered(uri: &str) -> bool {
    let map = get_global_namespace_map();
    // RwLock::read() only fails if the lock is poisoned
    let guard = map.read().expect("Namespace registry lock poisoned");
    guard.has_uri(uri)
}

/// Get the prefix for a namespace URI from global registry
pub fn get_global_namespace_prefix(uri: &str) -> Option<String> {
    let map = get_global_namespace_map();
    let guard = map.read().expect("Namespace registry lock poisoned");
    guard.get_prefix(uri).map(|s| s.to_string())
}

/// Get the URI for a namespace prefix from global registry
pub fn get_global_namespace_uri(prefix: &str) -> Option<String> {
    let map = get_global_namespace_map();
    let guard = map.read().expect("Namespace registry lock poisoned");
    guard.get_uri(prefix).map(|s| s.to_string())
}

/// Get all registered namespaces from global registry
///
/// Returns a vector of (uri, prefix) tuples for all registered namespaces.
pub fn get_all_registered_namespaces() -> Vec<(String, String)> {
    let map = get_global_namespace_map();
    let guard = map.read().expect("Namespace registry lock poisoned");
    guard.get_all_namespaces()
}

/// Get all built-in namespace URIs
///
/// Returns a vector of built-in namespace URIs.
pub fn get_builtin_namespace_uris() -> Vec<String> {
    vec![
        ns::XMP.to_string(),
        ns::DC.to_string(),
        ns::EXIF.to_string(),
        ns::EXIF_AUX.to_string(),
        ns::IPTC_CORE.to_string(),
        ns::IPTC_EXT.to_string(),
        ns::PHOTOSHOP.to_string(),
        ns::CAMERA_RAW.to_string(),
        ns::XMP_RIGHTS.to_string(),
        ns::XMP_MM.to_string(),
        ns::XMP_BJ.to_string(),
        ns::TIFF.to_string(),
        ns::PDF.to_string(),
        ns::PDFX.to_string(),
        ns::PDFA.to_string(),
        ns::XMP_DM.to_string(),
        ns::XMP_PAGED.to_string(),
        ns::XMP_GRAPHICS.to_string(),
        ns::XMP_IMAGE.to_string(),
        ns::RDF.to_string(),
        ns::XML.to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_namespace_map_new() {
        let map = NamespaceMap::new();
        assert!(map.has_uri(ns::XMP));
        assert!(map.has_uri(ns::DC));
        assert!(map.has_prefix(ns::XMP_PREFIX));
    }

    #[test]
    fn test_namespace_map_register() {
        let mut map = NamespaceMap::new();
        assert!(map.register("http://example.com/ns", "ex").is_ok());
        assert_eq!(map.get_prefix("http://example.com/ns"), Some("ex"));
        assert_eq!(map.get_uri("ex"), Some("http://example.com/ns"));
    }

    #[test]
    fn test_namespace_map_duplicate_prefix() {
        let mut map = NamespaceMap::new();
        assert!(map.register("http://example.com/ns1", "ex").is_ok());
        assert!(map.register("http://example.com/ns2", "ex").is_err());
    }

    #[test]
    fn test_namespace_map_same_uri_prefix() {
        let mut map = NamespaceMap::new();
        assert!(map.register("http://example.com/ns", "ex").is_ok());
        // Registering again with same URI and prefix should succeed
        assert!(map.register("http://example.com/ns", "ex").is_ok());
    }

    #[test]
    fn test_get_global_namespace_prefix() {
        assert_eq!(
            get_global_namespace_prefix(ns::XMP),
            Some(ns::XMP_PREFIX.to_string())
        );
        assert_eq!(
            get_global_namespace_prefix(ns::DC),
            Some(ns::DC_PREFIX.to_string())
        );
        assert_eq!(get_global_namespace_prefix("http://unknown.com/ns"), None);

        // Test dynamically registered namespace
        register_namespace("http://example.com/ns", "ex").unwrap();
        assert_eq!(
            get_global_namespace_prefix("http://example.com/ns"),
            Some("ex".to_string())
        );
    }
}
