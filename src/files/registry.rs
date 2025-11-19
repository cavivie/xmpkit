//! File handler registry for XMP metadata
//!
//! This module provides a registry system for managing file format handlers.
//! Handlers can be registered and looked up by file extension or format detection.

use crate::core::error::XmpResult;
use crate::files::handler::FileHandler;
use std::io::{Read, Seek, Write};

/// Enum of supported file handlers
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Handler {
    #[cfg(feature = "gif")]
    Gif(crate::files::formats::gif::GifHandler),
    #[cfg(feature = "jpeg")]
    Jpeg(crate::files::formats::jpeg::JpegHandler),
    #[cfg(feature = "mp3")]
    Mp3(crate::files::formats::mp3::Mp3Handler),
    #[cfg(feature = "mp4")]
    Mp4(crate::files::formats::mp4::Mp4Handler),
    #[cfg(feature = "png")]
    Png(crate::files::formats::png::PngHandler),
    #[cfg(feature = "tiff")]
    Tiff(crate::files::formats::tiff::TiffHandler),
}

impl FileHandler for Handler {
    fn can_handle<R: Read + Seek>(&self, reader: &mut R) -> XmpResult<bool> {
        match self {
            #[cfg(feature = "gif")]
            Handler::Gif(h) => h.can_handle(reader),
            #[cfg(feature = "jpeg")]
            Handler::Jpeg(h) => h.can_handle(reader),
            #[cfg(feature = "mp3")]
            Handler::Mp3(h) => h.can_handle(reader),
            #[cfg(feature = "mp4")]
            Handler::Mp4(h) => h.can_handle(reader),
            #[cfg(feature = "png")]
            Handler::Png(h) => h.can_handle(reader),
            #[cfg(feature = "tiff")]
            Handler::Tiff(h) => h.can_handle(reader),
        }
    }

    fn read_xmp<R: Read + Seek>(
        &self,
        reader: &mut R,
    ) -> XmpResult<Option<crate::core::metadata::XmpMeta>> {
        match self {
            #[cfg(feature = "gif")]
            Handler::Gif(h) => h.read_xmp(reader),
            #[cfg(feature = "jpeg")]
            Handler::Jpeg(h) => h.read_xmp(reader),
            #[cfg(feature = "mp3")]
            Handler::Mp3(h) => h.read_xmp(reader),
            #[cfg(feature = "mp4")]
            Handler::Mp4(h) => h.read_xmp(reader),
            #[cfg(feature = "png")]
            Handler::Png(h) => h.read_xmp(reader),
            #[cfg(feature = "tiff")]
            Handler::Tiff(h) => h.read_xmp(reader),
        }
    }

    fn write_xmp<R: Read + Seek, W: Seek + Write>(
        &self,
        reader: &mut R,
        writer: &mut W,
        meta: &crate::core::metadata::XmpMeta,
    ) -> XmpResult<()> {
        match self {
            #[cfg(feature = "gif")]
            Handler::Gif(h) => h.write_xmp(reader, writer, meta),
            #[cfg(feature = "jpeg")]
            Handler::Jpeg(h) => h.write_xmp(reader, writer, meta),
            #[cfg(feature = "mp3")]
            Handler::Mp3(h) => h.write_xmp(reader, writer, meta),
            #[cfg(feature = "mp4")]
            Handler::Mp4(h) => h.write_xmp(reader, writer, meta),
            #[cfg(feature = "png")]
            Handler::Png(h) => h.write_xmp(reader, writer, meta),
            #[cfg(feature = "tiff")]
            Handler::Tiff(h) => h.write_xmp(reader, writer, meta),
        }
    }

    fn format_name(&self) -> &'static str {
        match self {
            #[cfg(feature = "gif")]
            Handler::Gif(h) => h.format_name(),
            #[cfg(feature = "jpeg")]
            Handler::Jpeg(h) => h.format_name(),
            #[cfg(feature = "mp3")]
            Handler::Mp3(h) => h.format_name(),
            #[cfg(feature = "mp4")]
            Handler::Mp4(h) => h.format_name(),
            #[cfg(feature = "png")]
            Handler::Png(h) => h.format_name(),
            #[cfg(feature = "tiff")]
            Handler::Tiff(h) => h.format_name(),
        }
    }

    fn extensions(&self) -> &'static [&'static str] {
        match self {
            #[cfg(feature = "gif")]
            Handler::Gif(h) => h.extensions(),
            #[cfg(feature = "jpeg")]
            Handler::Jpeg(h) => h.extensions(),
            #[cfg(feature = "mp3")]
            Handler::Mp3(h) => h.extensions(),
            #[cfg(feature = "mp4")]
            Handler::Mp4(h) => h.extensions(),
            #[cfg(feature = "png")]
            Handler::Png(h) => h.extensions(),
            #[cfg(feature = "tiff")]
            Handler::Tiff(h) => h.extensions(),
        }
    }
}

/// Registry for file format handlers
pub struct HandlerRegistry {
    handlers: Vec<Handler>,
}

impl HandlerRegistry {
    /// Create a new handler registry with default handlers registered
    pub fn new() -> Self {
        let mut registry = Self {
            handlers: Vec::new(),
        };
        registry.register_defaults();
        registry
    }

    /// Register a file handler
    pub fn register(&mut self, handler: Handler) {
        self.handlers.push(handler);
    }

    /// Register default handlers (GIF, JPEG, MP3, MP4, PNG, TIFF)
    fn register_defaults(&mut self) {
        #[cfg(feature = "gif")]
        self.register(Handler::Gif(crate::files::formats::gif::GifHandler));
        #[cfg(feature = "jpeg")]
        self.register(Handler::Jpeg(crate::files::formats::jpeg::JpegHandler));
        #[cfg(feature = "mp3")]
        self.register(Handler::Mp3(crate::files::formats::mp3::Mp3Handler));
        #[cfg(feature = "mp4")]
        self.register(Handler::Mp4(crate::files::formats::mp4::Mp4Handler));
        #[cfg(feature = "png")]
        self.register(Handler::Png(crate::files::formats::png::PngHandler));
        #[cfg(feature = "tiff")]
        self.register(Handler::Tiff(crate::files::formats::tiff::TiffHandler));
    }

    /// Find a handler by file extension
    ///
    /// # Arguments
    ///
    /// * `extension` - File extension (e.g., "jpg", "png", "tiff")
    ///
    /// # Returns
    ///
    /// * `Some(&Handler)` if a handler is found
    /// * `None` if no handler matches the extension
    pub fn find_by_extension(&self, extension: &str) -> Option<&Handler> {
        let ext_lower = extension.to_lowercase();
        self.handlers
            .iter()
            .find(|h| h.extensions().iter().any(|e| e.to_lowercase() == ext_lower))
    }

    /// Find a handler by format detection
    ///
    /// This method tries each registered handler's `can_handle` method
    /// to determine which handler can process the file.
    ///
    /// # Arguments
    ///
    /// * `reader` - A reader implementing `Read + Seek`
    ///
    /// # Returns
    ///
    /// * `Ok(Some(&Handler))` if a handler is found
    /// * `Ok(None)` if no handler can handle the file
    /// * `Err(XmpError)` if an error occurs during detection
    pub fn find_by_detection<R: Read + Seek>(&self, reader: &mut R) -> XmpResult<Option<&Handler>> {
        let saved_pos = reader.stream_position()?;

        for handler in &self.handlers {
            reader.seek(std::io::SeekFrom::Start(saved_pos))?;
            if handler.can_handle(reader)? {
                reader.seek(std::io::SeekFrom::Start(saved_pos))?;
                return Ok(Some(handler));
            }
        }

        reader.seek(std::io::SeekFrom::Start(saved_pos))?;
        Ok(None)
    }

    /// Get all registered handlers
    pub fn handlers(&self) -> &[Handler] {
        &self.handlers
    }
}

impl Default for HandlerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Global default registry instance
///
/// This provides a convenient way to access the default handler registry
/// without needing to create a new instance.
pub fn default_registry() -> HandlerRegistry {
    HandlerRegistry::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_registry_new() {
        let registry = HandlerRegistry::new();
        assert!(!registry.handlers().is_empty());
    }

    #[test]
    fn test_find_by_extension() {
        let registry = HandlerRegistry::new();
        assert!(registry.find_by_extension("jpg").is_some());
        assert!(registry.find_by_extension("png").is_some());
        assert!(registry.find_by_extension("tiff").is_some());
        assert!(registry.find_by_extension("unknown").is_none());
    }

    #[test]
    fn test_find_by_detection_jpeg() {
        let registry = HandlerRegistry::new();
        let jpeg_data = vec![0xFF, 0xD8, 0xFF, 0xE0];
        let mut reader = Cursor::new(jpeg_data);
        let handler = registry.find_by_detection(&mut reader).unwrap();
        assert!(handler.is_some());
        assert_eq!(handler.unwrap().format_name(), "JPEG");
    }

    #[test]
    fn test_find_by_detection_png() {
        let registry = HandlerRegistry::new();
        let png_data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        let mut reader = Cursor::new(png_data);
        let handler = registry.find_by_detection(&mut reader).unwrap();
        assert!(handler.is_some());
        assert_eq!(handler.unwrap().format_name(), "PNG");
    }
}
