//! File handler trait for XMP metadata
//!
//! This module defines the trait that all file format handlers must implement.
//! This allows for a unified interface across different file formats.

use crate::core::error::XmpResult;
use crate::core::metadata::XmpMeta;
use std::io::{Read, Seek, Write};

/// Options for XMP file operations.
///
/// Use the builder pattern to configure options. These options control how
/// file handlers read and process XMP metadata.
///
/// # Example
///
/// ```rust,no_run
/// use xmpkit::{XmpFile, XmpOptions};
///
/// let mut file = XmpFile::new();
/// // Open for update with strict mode
/// file.open_with("photo.jpg", XmpOptions::default().for_update().strict())?;
/// // ... modify metadata ...
/// file.try_close()?;
/// # Ok::<(), xmpkit::XmpError>(())
/// ```
#[derive(Default, Clone, Copy, Debug)]
pub struct XmpOptions {
    /// Open for reading and writing (default: read-only)
    pub for_update: bool,
    /// Only the XMP is wanted, skip reconciliation with native metadata
    pub only_xmp: bool,
    /// Force use of the given handler (format)
    pub force_given_handler: bool,
    /// Be strict about only attempting to use the designated file handler
    pub strict: bool,
    /// Require the use of a smart handler
    pub use_smart_handler: bool,
    /// Force packet scanning (do not use smart handler)
    pub use_packet_scanning: bool,
    /// Only packet scan files "known" to need scanning
    pub limited_scanning: bool,
}

impl XmpOptions {
    /// Open for read-only access (default).
    pub fn for_read(mut self) -> Self {
        self.for_update = false;
        self
    }

    /// Open for reading and writing.
    ///
    /// Files opened for update are written to only when closing.
    pub fn for_update(mut self) -> Self {
        self.for_update = true;
        self
    }

    /// Only the XMP is wanted.
    ///
    /// This allows space/time optimizations by skipping reconciliation
    /// with native metadata formats (e.g., QuickTime metadata in MPEG4).
    pub fn only_xmp(mut self) -> Self {
        self.only_xmp = true;
        self
    }

    /// Force use of the given handler (format).
    ///
    /// Do not even verify the format.
    pub fn force_given_handler(mut self) -> Self {
        self.force_given_handler = true;
        self
    }

    /// Be strict about only attempting to use the designated file handler.
    ///
    /// Do not fall back to other handlers.
    pub fn strict(mut self) -> Self {
        self.strict = true;
        self
    }

    /// Require the use of a smart handler.
    ///
    /// Do not fall back to packet scanning.
    pub fn use_smart_handler(mut self) -> Self {
        self.use_smart_handler = true;
        self
    }

    /// Force packet scanning.
    ///
    /// Do not use a smart handler.
    pub fn use_packet_scanning(mut self) -> Self {
        self.use_packet_scanning = true;
        self
    }

    /// Only packet scan files "known" to need scanning.
    pub fn limited_scanning(mut self) -> Self {
        self.limited_scanning = true;
        self
    }
}

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
    /// * `options` - Options controlling how XMP is read
    ///
    /// # Returns
    ///
    /// * `Ok(Some(XmpMeta))` if XMP metadata is found
    /// * `Ok(None)` if no XMP metadata is found
    /// * `Err(XmpError)` if an error occurs
    fn read_xmp<R: Read + Seek>(
        &self,
        reader: &mut R,
        options: &XmpOptions,
    ) -> XmpResult<Option<XmpMeta>>;

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
