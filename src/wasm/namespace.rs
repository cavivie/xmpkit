//! WebAssembly bindings for namespace management

use crate::core::namespace;
use crate::wasm::error::{xmp_error_to_wasm_error, XmpError};
use wasm_bindgen::prelude::*;

/// Built-in XMP namespaces (enum for JavaScript)
///
/// These enum values can be used in JavaScript to reference standard XMP namespaces.
///
/// # Example
///
/// ```javascript
/// import { Namespace, namespace_uri } from './pkg/xmpkit.js';
/// meta.set_property(namespace_uri(Namespace.Xmp), "CreatorTool", "MyApp");
/// ```
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Namespace {
    /// XMP Basic namespace
    Xmp,
    /// Dublin Core namespace
    Dc,
    /// EXIF namespace
    Exif,
    /// EXIF Aux namespace
    ExifAux,
    /// IPTC Core namespace
    IptcCore,
    /// IPTC Extension namespace
    IptcExt,
    /// Photoshop namespace
    Photoshop,
    /// Camera Raw namespace
    CameraRaw,
    /// XMP Rights namespace
    XmpRights,
    /// XMP Media Management namespace
    XmpMm,
    /// XMP Basic Job Ticket namespace
    XmpBj,
    /// TIFF namespace
    Tiff,
    /// PDF namespace
    Pdf,
    /// PDF/X namespace
    Pdfx,
    /// PDF/A namespace
    Pdfa,
    /// XMP Dynamic Media namespace
    XmpDm,
    /// XMP PagedText namespace
    XmpPaged,
    /// XMP Graphics namespace
    XmpGraphics,
    /// XMP Image namespace
    XmpImage,
    /// RDF namespace
    Rdf,
    /// XML namespace
    Xml,
}

/// Get the namespace URI for a Namespace enum value
///
/// # Example
///
/// ```javascript
/// import { Namespace, namespace_uri } from './pkg/xmpkit.js';
/// meta.set_property(namespace_uri(Namespace.Xmp), "CreatorTool", "MyApp");
/// ```
#[wasm_bindgen]
pub fn namespace_uri(ns: Namespace) -> String {
    match ns {
        Namespace::Xmp => namespace::ns::XMP.to_string(),
        Namespace::Dc => namespace::ns::DC.to_string(),
        Namespace::Exif => namespace::ns::EXIF.to_string(),
        Namespace::ExifAux => namespace::ns::EXIF_AUX.to_string(),
        Namespace::IptcCore => namespace::ns::IPTC_CORE.to_string(),
        Namespace::IptcExt => namespace::ns::IPTC_EXT.to_string(),
        Namespace::Photoshop => namespace::ns::PHOTOSHOP.to_string(),
        Namespace::CameraRaw => namespace::ns::CAMERA_RAW.to_string(),
        Namespace::XmpRights => namespace::ns::XMP_RIGHTS.to_string(),
        Namespace::XmpMm => namespace::ns::XMP_MM.to_string(),
        Namespace::XmpBj => namespace::ns::XMP_BJ.to_string(),
        Namespace::Tiff => namespace::ns::TIFF.to_string(),
        Namespace::Pdf => namespace::ns::PDF.to_string(),
        Namespace::Pdfx => namespace::ns::PDFX.to_string(),
        Namespace::Pdfa => namespace::ns::PDFA.to_string(),
        Namespace::XmpDm => namespace::ns::XMP_DM.to_string(),
        Namespace::XmpPaged => namespace::ns::XMP_PAGED.to_string(),
        Namespace::XmpGraphics => namespace::ns::XMP_GRAPHICS.to_string(),
        Namespace::XmpImage => namespace::ns::XMP_IMAGE.to_string(),
        Namespace::Rdf => namespace::ns::RDF.to_string(),
        Namespace::Xml => namespace::ns::XML.to_string(),
    }
}

/// Get the namespace prefix for a Namespace enum value
#[wasm_bindgen]
pub fn namespace_prefix(ns: Namespace) -> String {
    match ns {
        Namespace::Xmp => namespace::ns::XMP_PREFIX.to_string(),
        Namespace::Dc => namespace::ns::DC_PREFIX.to_string(),
        Namespace::Exif => namespace::ns::EXIF_PREFIX.to_string(),
        Namespace::ExifAux => namespace::ns::EXIF_AUX_PREFIX.to_string(),
        Namespace::IptcCore => namespace::ns::IPTC_CORE_PREFIX.to_string(),
        Namespace::IptcExt => namespace::ns::IPTC_EXT_PREFIX.to_string(),
        Namespace::Photoshop => namespace::ns::PHOTOSHOP_PREFIX.to_string(),
        Namespace::CameraRaw => namespace::ns::CAMERA_RAW_PREFIX.to_string(),
        Namespace::XmpRights => namespace::ns::XMP_RIGHTS_PREFIX.to_string(),
        Namespace::XmpMm => namespace::ns::XMP_MM_PREFIX.to_string(),
        Namespace::XmpBj => namespace::ns::XMP_BJ_PREFIX.to_string(),
        Namespace::Tiff => namespace::ns::TIFF_PREFIX.to_string(),
        Namespace::Pdf => namespace::ns::PDF_PREFIX.to_string(),
        Namespace::Pdfx => namespace::ns::PDFX_PREFIX.to_string(),
        Namespace::Pdfa => namespace::ns::PDFA_PREFIX.to_string(),
        Namespace::XmpDm => namespace::ns::XMP_DM_PREFIX.to_string(),
        Namespace::XmpPaged => namespace::ns::XMP_PAGED_PREFIX.to_string(),
        Namespace::XmpGraphics => namespace::ns::XMP_GRAPHICS_PREFIX.to_string(),
        Namespace::XmpImage => namespace::ns::XMP_IMAGE_PREFIX.to_string(),
        Namespace::Rdf => namespace::ns::RDF_PREFIX.to_string(),
        Namespace::Xml => namespace::ns::XML_PREFIX.to_string(),
    }
}

/// Register a namespace URI with a prefix
///
/// # Arguments
/// * `uri` - Namespace URI (e.g., "http://ns.adobe.com/xap/1.0/")
/// * `prefix` - Namespace prefix (e.g., "xmp")
///
/// # Example
///
/// ```javascript
/// import { register_namespace } from './pkg/xmpkit.js';
/// register_namespace("http://ns.adobe.com/xap/1.0/", "xmp");
/// ```
#[wasm_bindgen]
pub fn register_namespace(uri: &str, prefix: &str) -> Result<(), XmpError> {
    namespace::register_namespace(uri, prefix).map_err(xmp_error_to_wasm_error)
}

/// Check if a namespace URI is registered
#[wasm_bindgen]
pub fn is_namespace_registered(uri: &str) -> bool {
    namespace::is_namespace_registered(uri)
}

/// Get the prefix for a namespace URI
#[wasm_bindgen]
pub fn get_namespace_prefix(uri: &str) -> Option<String> {
    namespace::get_global_namespace_prefix(uri)
}

/// Get the URI for a namespace prefix
#[wasm_bindgen]
pub fn get_namespace_uri(prefix: &str) -> Option<String> {
    namespace::get_global_namespace_uri(prefix)
}

/// Get all registered namespaces
///
/// Returns a JavaScript object mapping URI to prefix.
/// # Example
///
/// ```javascript
/// import { get_all_registered_namespaces } from './pkg/xmpkit.js';
/// const namespaces = get_all_registered_namespaces();
/// // namespaces is an object like { "http://ns.adobe.com/xap/1.0/": "xmp", ... }
/// ```
#[wasm_bindgen]
pub fn get_all_registered_namespaces() -> JsValue {
    use js_sys::Object;
    use wasm_bindgen::JsValue;
    let namespaces = namespace::get_all_registered_namespaces();
    let obj = Object::new();
    for (uri, prefix) in namespaces {
        js_sys::Reflect::set(&obj, &JsValue::from_str(&uri), &JsValue::from_str(&prefix))
            .expect("Failed to set namespace mapping");
    }
    JsValue::from(obj)
}

/// Get all built-in namespace URIs
///
/// Returns a JavaScript array of built-in namespace URIs.
/// # Example
///
/// ```javascript
/// import { get_builtin_namespace_uris } from './pkg/xmpkit.js';
/// const builtinUris = get_builtin_namespace_uris();
/// // builtinUris is an array like ["http://ns.adobe.com/xap/1.0/", ...]
/// ```
#[wasm_bindgen]
pub fn get_builtin_namespace_uris() -> Vec<String> {
    namespace::get_builtin_namespace_uris()
}
