//! OpenHarmony bindings for namespace management

use crate::core::namespace;
use crate::ohos::error::xmp_error_to_ohos_error;
use napi_derive_ohos::napi;
use napi_ohos::bindgen_prelude::*;

/// Built-in XMP namespaces
#[napi]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Namespace {
    Xmp,
    Dc,
    Exif,
    ExifAux,
    IptcCore,
    IptcExt,
    Photoshop,
    CameraRaw,
    XmpRights,
    XmpMm,
    XmpBj,
    Tiff,
    Pdf,
    Pdfx,
    Pdfa,
    XmpDm,
    XmpPaged,
    XmpGraphics,
    XmpImage,
    Rdf,
    Xml,
}

#[napi]
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

#[napi]
pub fn register_namespace(uri: String, prefix: String) -> Result<()> {
    namespace::register_namespace(&uri, &prefix)
        .map_err(|e| Error::from_reason(format!("{}", xmp_error_to_ohos_error(e))))
}

#[napi]
pub fn is_namespace_registered(uri: String) -> bool {
    namespace::is_namespace_registered(&uri)
}

#[napi]
pub fn get_namespace_prefix(uri: String) -> Option<String> {
    namespace::get_global_namespace_prefix(&uri)
}

#[napi]
pub fn get_namespace_uri(prefix: String) -> Option<String> {
    namespace::get_global_namespace_uri(&prefix)
}

#[napi]
pub fn get_all_registered_namespaces() -> std::collections::HashMap<String, String> {
    namespace::get_all_registered_namespaces()
        .into_iter()
        .collect()
}

#[napi]
pub fn get_builtin_namespace_uris() -> Vec<String> {
    namespace::get_builtin_namespace_uris()
}
