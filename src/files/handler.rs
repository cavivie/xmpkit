//! File handler trait for XMP metadata
//!
//! This module defines the trait that all file format handlers must implement.
//! This allows for a unified interface across different file formats.

use crate::core::error::XmpResult;
use crate::core::metadata::XmpMeta;
use std::io::{Read, Seek, Write};

/// Trait for file format handlers
///
/// All file format handlers (JPEG, PNG, TIFF, etc.) must implement this trait
/// to provide a unified interface for reading and writing XMP metadata.
pub trait FileHandler: Send + Sync {
    /// Check if this handler can handle the given file
    ///
    /// This method should peek at the file header to determine if it matches
    /// the expected format. It should not consume the reader.
    ///
    /// # Arguments
    ///
    /// * `reader` - A reader implementing `Read + Seek`
    ///
    /// # Returns
    ///
    /// * `true` if this handler can handle the file format
    /// * `false` otherwise
    fn can_handle<R: Read + Seek>(&self, reader: &mut R) -> XmpResult<bool>;

    /// Read XMP metadata from a file
    ///
    /// # Arguments
    ///
    /// * `reader` - A reader implementing `Read + Seek`
    ///
    /// # Returns
    ///
    /// * `Ok(Some(XmpMeta))` if XMP metadata is found
    /// * `Ok(None)` if no XMP metadata is found
    /// * `Err(XmpError)` if an error occurs
    fn read_xmp<R: Read + Seek>(&self, reader: &mut R) -> XmpResult<Option<XmpMeta>>;

    /// Write XMP metadata to a file
    ///
    /// # Arguments
    ///
    /// * `reader` - A reader implementing `Read + Seek` for the source file
    /// * `writer` - A writer implementing `Write + Seek` for the output file
    /// * `meta` - The XMP metadata to write
    ///
    /// # Returns
    ///
    /// * `Ok(())` if successful
    /// * `Err(XmpError)` if an error occurs
    fn write_xmp<R: Read + Seek, W: Write + Seek>(
        &self,
        reader: &mut R,
        writer: &mut W,
        meta: &XmpMeta,
    ) -> XmpResult<()>;

    /// Get the name of the file format this handler supports
    ///
    /// # Returns
    ///
    /// A static string describing the format (e.g., "JPEG", "PNG", "TIFF")
    fn format_name(&self) -> &'static str;

    /// Get the file extensions this handler supports
    ///
    /// # Returns
    ///
    /// A slice of file extensions (e.g., &["jpg", "jpeg"] for JPEG)
    fn extensions(&self) -> &'static [&'static str];
}
