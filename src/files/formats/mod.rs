//! File format handlers
//!
//! Each format handler implements file format-specific logic for embedding
//! and extracting XMP metadata. All handlers are pure Rust implementations
//! that work across all platforms.

#[cfg(feature = "gif")]
pub mod gif;
#[cfg(feature = "jpeg")]
pub mod jpeg;
#[cfg(feature = "mp3")]
pub mod mp3;
#[cfg(feature = "mp4")]
pub mod mp4;
#[cfg(feature = "png")]
pub mod png;
#[cfg(feature = "tiff")]
pub mod tiff;
