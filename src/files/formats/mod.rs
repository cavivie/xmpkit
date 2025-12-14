//! File format handlers
//!
//! Each format handler implements file format-specific logic for embedding
//! and extracting XMP metadata. All handlers are pure Rust implementations
//! that work across all platforms.
//!
//! ## Module Organization
//!
//! Formats are organized by their container type:
//! - `riff/` - RIFF-based formats (WebP, WAV, AVI)
//! - `bmff/` - BMFF-based formats (MP4, MOV)
//! - Individual modules for standalone formats

#[cfg(feature = "gif")]
pub mod gif;
#[cfg(feature = "jpeg")]
pub mod jpeg;
#[cfg(feature = "mp3")]
pub mod mp3;
#[cfg(feature = "pdf")]
pub mod pdf;
#[cfg(feature = "png")]
pub mod png;
#[cfg(feature = "psd")]
pub mod psd;
#[cfg(feature = "svg")]
pub mod svg;
#[cfg(feature = "tiff")]
pub mod tiff;

// RIFF-based formats
#[cfg(any(feature = "webp", feature = "wav", feature = "avi"))]
pub mod riff;

// BMFF-based formats
#[cfg(any(feature = "mpeg4", feature = "heif"))]
pub mod bmff;
