//! File format support for XMP
//!
//! This module provides functionality for reading and writing XMP metadata
//! in various file formats. All implementations are pure Rust and cross-platform
//! compatible (iOS, Android, HarmonyOS, macOS, Windows, Wasm).

pub mod file;
pub mod formats;
pub mod handler;
pub mod registry;

pub use file::XmpFile;
#[cfg(feature = "mpeg4")]
pub use formats::bmff::Mpeg4Handler;
#[cfg(feature = "mpegh")]
pub use formats::bmff::MpeghHandler;
#[cfg(feature = "gif")]
pub use formats::gif::GifHandler;
#[cfg(feature = "jpeg")]
pub use formats::jpeg::JpegHandler;
#[cfg(feature = "mp3")]
pub use formats::mp3::Mp3Handler;
#[cfg(feature = "pdf")]
pub use formats::pdf::PdfHandler;
#[cfg(feature = "png")]
pub use formats::png::PngHandler;
#[cfg(feature = "psd")]
pub use formats::psd::PsdHandler;
#[cfg(feature = "avi")]
pub use formats::riff::avi::AviHandler;
#[cfg(feature = "wav")]
pub use formats::riff::wav::WavHandler;
#[cfg(feature = "webp")]
pub use formats::riff::webp::WebpHandler;
#[cfg(feature = "svg")]
pub use formats::svg::SvgHandler;
#[cfg(feature = "tiff")]
pub use formats::tiff::TiffHandler;
pub use handler::FileHandler;
pub use handler::XmpOptions;
pub use registry::{default_registry, Handler, HandlerRegistry};
