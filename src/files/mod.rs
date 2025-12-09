//! File format support for XMP
//!
//! This module provides functionality for reading and writing XMP metadata
//! in various file formats. All implementations are pure Rust and cross-platform
//! compatible (iOS, Android, HarmonyOS, macOS, Windows, Wasm).

pub mod file;
pub mod formats;
pub mod handler;
pub mod registry;

pub use file::{ReadOptions, XmpFile};
#[cfg(feature = "gif")]
pub use formats::gif::GifHandler;
#[cfg(feature = "jpeg")]
pub use formats::jpeg::JpegHandler;
#[cfg(feature = "mp3")]
pub use formats::mp3::Mp3Handler;
#[cfg(feature = "mpeg4")]
pub use formats::mpeg4::Mpeg4Handler;
#[cfg(feature = "png")]
pub use formats::png::PngHandler;
#[cfg(feature = "tiff")]
pub use formats::tiff::TiffHandler;
pub use handler::FileHandler;
pub use registry::{default_registry, Handler, HandlerRegistry};
