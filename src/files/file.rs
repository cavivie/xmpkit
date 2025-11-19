//! XMP File API
//!
//! This module provides a high-level API for working with XMP metadata in files,
//! similar to the original xmp-toolkit-rs API, but with Wasm compatibility.

use crate::core::error::{XmpError, XmpResult};
use crate::core::metadata::XmpMeta;
use crate::files::handler::FileHandler;
use crate::files::registry::default_registry;
use std::io::{Cursor, Read, Seek, Write};

/// Options for reading XMP metadata from files or memory.
///
/// Use the builder pattern to configure options. These options apply to both
/// file operations (`open_with`) and memory operations (`from_bytes_with`, `from_reader_with`).
///
/// # Example
///
/// ```rust,no_run
/// use xmpkit::{XmpFile, ReadOptions};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut file = XmpFile::new();
/// file.open_with("image.jpg", ReadOptions::default().for_update())?;
/// // ... modify metadata ...
/// file.try_close()?;
/// # Ok(())
/// # }
/// ```
#[derive(Default, Clone, Copy, Debug)]
pub struct ReadOptions {
    /// Open for reading and writing (default: read-only)
    pub(crate) for_update: bool,
    /// Only the XMP is wanted (allows optimizations)
    pub(crate) only_xmp: bool,
    /// Force use of the given handler (format)
    pub(crate) force_given_handler: bool,
    /// Be strict about only attempting to use the designated file handler
    pub(crate) strict: bool,
    /// Require the use of a smart handler
    pub(crate) use_smart_handler: bool,
    /// Force packet scanning (do not use smart handler)
    pub(crate) use_packet_scanning: bool,
    /// Only packet scan files "known" to need scanning
    pub(crate) limited_scanning: bool,
}

impl ReadOptions {
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
    /// This allows space/time optimizations by skipping other metadata.
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
    pub fn use_smart_handler(mut self) -> Self {
        self.use_smart_handler = true;
        self.use_packet_scanning = false; // Mutually exclusive
        self
    }

    /// Force packet scanning.
    ///
    /// Do not use a smart handler.
    pub fn use_packet_scanning(mut self) -> Self {
        self.use_packet_scanning = true;
        self.use_smart_handler = false; // Mutually exclusive
        self
    }

    /// Only packet scan files "known" to need scanning.
    pub fn limited_scanning(mut self) -> Self {
        self.limited_scanning = true;
        self
    }
}

/// High-level API for working with XMP metadata in files
///
/// This struct provides a file-like API similar to the original xmp-toolkit-rs,
/// but works across all platforms including Wasm.
///
/// # Platform Support
///
/// - **Native platforms** (iOS, Android, macOS, Windows): Can use `open_file()`
/// - **Wasm**: Use `from_bytes()` or `from_reader()` with in-memory data
///
/// # File Update Behavior
///
/// When a file is opened with [`ReadOptions::for_update`], changes made via
/// [`XmpFile::put_xmp`] are not written to disk immediately. The file remains open
/// and changes are only written when [`XmpFile::close`] or [`XmpFile::try_close`] is called.
///
/// # Example
///
/// ```rust,no_run
/// use xmpkit::{XmpFile, ReadOptions, XmpMeta, XmpValue};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut file = XmpFile::new();
/// file.open_with("image.jpg", ReadOptions::default().for_update())?;
///
/// if let Some(mut meta) = file.get_xmp().cloned() {
///     meta.set_property(
///         "http://ns.adobe.com/xap/1.0/",
///         "CreatorTool",
///         XmpValue::String("MyApp".to_string()),
///     )?;
///     file.put_xmp(meta);
/// }
///
/// // Changes are written to disk when try_close() is called
/// file.try_close()?;
/// # Ok(())
/// # }
/// ```
pub struct XmpFile {
    meta: Option<XmpMeta>,
    /// Original file path (for native platforms)
    #[cfg(not(target_arch = "wasm32"))]
    file_path: Option<std::path::PathBuf>,
    /// Original file data (for in-memory operations)
    #[allow(dead_code)] // Used in native code paths (open_with, try_close)
    file_data: Option<Vec<u8>>,
    /// Handler used to read/write the file
    #[allow(dead_code)] // Used in native code paths (open_with, try_close)
    handler: Option<crate::files::registry::Handler>,
    /// Open options
    #[allow(dead_code)] // Used in native code paths (open_with, try_close)
    options: ReadOptions,
    /// Whether the file is open
    is_open: bool,
}

impl XmpFile {
    /// Create a new empty XmpFile
    ///
    /// Use `open_file()` or `from_*()` methods to load metadata from a file.
    pub fn new() -> Self {
        Self {
            meta: None,
            #[cfg(not(target_arch = "wasm32"))]
            file_path: None,
            file_data: None,
            handler: None,
            options: ReadOptions::default(),
            is_open: false,
        }
    }

    /// Open a file from a path with options (native platforms only)
    ///
    /// # Platform Support
    ///
    /// - Native platforms (iOS, Android, macOS, Windows)
    /// - Wasm: Not supported (use `from_bytes()` or `from_reader()` instead)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use xmpkit::{XmpFile, ReadOptions};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut file = XmpFile::new();
    /// file.open_with("image.jpg", ReadOptions::default().for_update())?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(not(target_arch = "wasm32"))]
    pub fn open_with<P: AsRef<std::path::Path>>(
        &mut self,
        path: P,
        options: ReadOptions,
    ) -> XmpResult<()> {
        use std::fs;
        let path = path.as_ref();

        // Check limited_scanning: only scan known file types
        // This check needs to happen before reading the file, so we do it here
        // rather than in from_reader_with_options
        if options.use_packet_scanning && options.limited_scanning {
            let file_ext = path
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("")
                .to_lowercase();
            // Known file types that need scanning
            const KNOWN_SCANNED_FILES: &[&str] = &["txt", "xml", "html", "htm"];
            if !KNOWN_SCANNED_FILES.contains(&file_ext.as_str()) {
                return Err(XmpError::NotSupported(format!(
                    "File type '{}' not in limited scanning list",
                    file_ext
                )));
            }
        }

        // Read file and use from_reader_with
        let file = fs::File::open(path)?;
        self.file_path = Some(path.to_path_buf());
        self.from_reader_with(file, options)
    }

    /// Scan file content for XMP packet (packet scanning mode)
    ///
    /// This method searches for XMP packets in file content by looking for
    /// the `<?xpacket` marker. Used when packet scanning is requested.
    pub fn scan_for_xmp_packet(file_data: &[u8]) -> XmpResult<Option<XmpMeta>> {
        // Use byte search to find XMP packet (files may contain binary data)
        // Look for "<?xpacket" pattern
        let xpacket_start = b"<?xpacket";
        let mut search_pos = 0;

        while search_pos + xpacket_start.len() <= file_data.len() {
            // Find next occurrence of "<?xpacket"
            let Some(pos) = file_data[search_pos..]
                .windows(xpacket_start.len())
                .position(|window| window == xpacket_start)
            else {
                break;
            };

            let start_pos = search_pos + pos;

            // Find the end of the packet ("<?xpacket end")
            let xpacket_end_marker = b"<?xpacket end";
            let Some(packet_end_offset) = file_data[start_pos..]
                .windows(xpacket_end_marker.len())
                .position(|window| window.starts_with(xpacket_end_marker))
            else {
                search_pos = start_pos + 1;
                continue;
            };

            // Find the actual end: "<?xpacket end=\"w\"?>" or "<?xpacket end=\"r\"?>"
            // Search for "?>" after the end marker (should be close after "end=")
            let end_marker_start = start_pos + packet_end_offset;
            // Look for "?>" after "<?xpacket end" - it should be within a reasonable distance
            // (typically "<?xpacket end=\"w\"?>" or "<?xpacket end=\"r\"?>")
            let Some(close_pos) = file_data[end_marker_start..]
                .iter()
                .enumerate()
                .find(|(_, &b)| b == b'?')
                .and_then(|(q_pos, _)| {
                    if end_marker_start + q_pos + 1 < file_data.len()
                        && file_data[end_marker_start + q_pos + 1] == b'>'
                    {
                        // Verify this is actually the end of <?xpacket end (not just any ?>)
                        // Check that we have "end=" before the ?>
                        let before_close = &file_data[end_marker_start..end_marker_start + q_pos];
                        if before_close.ends_with(b"\"w\"") || before_close.ends_with(b"\"r\"") {
                            Some(q_pos + 2)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
            else {
                search_pos = start_pos + 1;
                continue;
            };

            let packet_end_pos = end_marker_start + close_pos;

            // Extract packet as string (XMP content should be valid UTF-8)
            if let Ok(packet_str) = std::str::from_utf8(&file_data[start_pos..packet_end_pos]) {
                // Try to parse the packet
                match XmpMeta::parse(packet_str) {
                    Ok(meta) => return Ok(Some(meta)),
                    Err(_) => {
                        // If parsing fails, continue searching for another packet
                        search_pos = start_pos + 1;
                        continue;
                    }
                }
            }

            search_pos = start_pos + 1;
        }

        Ok(None)
    }

    /// Open a file from a path (native platforms only)
    ///
    /// # Platform Support
    ///
    /// - Native platforms (iOS, Android, macOS, Windows)
    /// - Wasm: Not supported (use `from_bytes()` or `from_reader()` instead)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use xmpkit::XmpFile;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut file = XmpFile::new();
    /// file.open("image.jpg")?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(not(target_arch = "wasm32"))]
    pub fn open<P: AsRef<std::path::Path>>(&mut self, path: P) -> XmpResult<()> {
        use std::fs::File;
        let file = File::open(path)?;
        self.from_reader(file)
    }

    /// Open a file from bytes (all platforms, including Wasm)
    ///
    /// This is the recommended method for Wasm environments.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use xmpkit::XmpFile;
    ///
    /// let jpeg_data: &[u8] = /* your JPEG file data */;
    /// let mut file = XmpFile::new();
    /// file.from_bytes(jpeg_data)?;
    /// ```
    pub fn from_bytes(&mut self, data: &[u8]) -> XmpResult<()> {
        self.from_bytes_with(data, ReadOptions::default())
    }

    /// Open a file from bytes with options (all platforms, including Wasm)
    ///
    /// This method allows you to specify opening options, such as forcing packet scanning
    /// or requiring a smart handler.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use xmpkit::{XmpFile, ReadOptions};
    ///
    /// let data: &[u8] = /* your file data */;
    /// let mut file = XmpFile::new();
    /// file.from_bytes_with(data, ReadOptions::default().use_packet_scanning())?;
    /// ```
    pub fn from_bytes_with(&mut self, data: &[u8], options: ReadOptions) -> XmpResult<()> {
        let cursor = Cursor::new(data);
        self.from_reader_with(cursor, options)
    }

    /// Open a file from a reader (all platforms, including Wasm)
    ///
    /// This is the most flexible method, accepting any type that implements
    /// `Read + Seek`. Use this when you have a custom reader or need maximum flexibility.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use std::io::Cursor;
    /// use xmpkit::XmpFile;
    ///
    /// let data: Vec<u8> = /* your JPEG file data */;
    /// let cursor = Cursor::new(data);
    /// let mut file = XmpFile::new();
    /// file.from_reader(cursor)?;
    /// ```
    pub fn from_reader<R: Read + Seek>(&mut self, reader: R) -> XmpResult<()> {
        self.from_reader_with(reader, ReadOptions::default())
    }

    /// Open a file from a reader with options (all platforms, including Wasm)
    ///
    /// This method allows you to specify opening options when reading from a reader.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use std::io::Cursor;
    /// use xmpkit::{XmpFile, ReadOptions};
    ///
    /// let data: Vec<u8> = /* your file data */;
    /// let cursor = Cursor::new(data);
    /// let mut file = XmpFile::new();
    /// file.from_reader_with(cursor, ReadOptions::default().strict())?;
    /// ```
    pub fn from_reader_with<R: Read + Seek>(
        &mut self,
        mut reader: R,
        options: ReadOptions,
    ) -> XmpResult<()> {
        // Reset state before opening (in case of retry)
        self.meta = None;
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.handler = None;
            self.is_open = false;
        }
        self.options = options;

        // Read file data for potential packet scanning or handler operations
        // Store file_data for writing (needed on all platforms including Wasm)
        let mut file_data = Vec::new();
        reader.read_to_end(&mut file_data)?;
        self.file_data = Some(file_data.clone());

        // If packet scanning is requested, search for XMP packet in file content
        // Note: limited_scanning check is done in open_with (for file paths) before calling this
        if options.use_packet_scanning {
            self.meta = Self::scan_for_xmp_packet(&file_data)?;
            #[cfg(not(target_arch = "wasm32"))]
            {
                self.is_open = true;
            }
            return Ok(());
        }

        // Detect handler
        let registry = default_registry();
        let mut reader_cursor = Cursor::new(&file_data);

        // Handle force_given_handler: skip format detection, use handler directly
        // Note: This requires a handler to be specified, which we don't currently support
        // in from_reader_with_options. For now, we'll just proceed with normal detection.
        let handler = if options.force_given_handler {
            // Force given handler: try all handlers without format check
            // This is a simplified version - in full implementation, handler would be specified
            registry.find_by_detection(&mut reader_cursor)?
        } else {
            registry.find_by_detection(&mut reader_cursor)?
        };

        // Handle use_smart_handler: if set and no handler found, return error
        if options.use_smart_handler {
            let handler = handler.ok_or_else(|| {
                XmpError::NotSupported("No smart file handler available to handle file".to_string())
            })?;
            // Read XMP (only_xmp flag is passed implicitly - handlers already only read XMP)
            reader_cursor.set_position(0);
            self.meta = handler.read_xmp(&mut reader_cursor)?;
            #[cfg(not(target_arch = "wasm32"))]
            {
                self.handler = Some(handler.clone());
                self.is_open = true;
            }
            return Ok(());
        }

        // Handle strict: if set and no handler found, return error (don't fall back)
        if options.strict {
            let handler = handler.ok_or_else(|| {
                XmpError::NotSupported("No handler available for file format".to_string())
            })?;
            // Read XMP (only_xmp flag is passed implicitly)
            reader_cursor.set_position(0);
            self.meta = handler.read_xmp(&mut reader_cursor)?;
            #[cfg(not(target_arch = "wasm32"))]
            {
                self.handler = Some(handler.clone());
                self.is_open = true;
            }
            return Ok(());
        }

        // Normal case: try to find handler, fall back to packet scanning if not found
        if let Some(handler) = handler {
            // Read XMP
            // Note: only_xmp flag is currently not used in our handlers as they already
            // only read XMP metadata. This flag is kept for API compatibility and future
            // optimizations where handlers might skip reading other metadata (Exif, IPTC, etc.)
            reader_cursor.set_position(0);
            self.meta = handler.read_xmp(&mut reader_cursor)?;
            #[cfg(not(target_arch = "wasm32"))]
            {
                self.handler = Some(handler.clone());
                self.is_open = true;
            }
            Ok(())
        } else {
            // No handler found, try packet scanning as fallback
            self.meta = Self::scan_for_xmp_packet(&file_data)?;
            #[cfg(not(target_arch = "wasm32"))]
            {
                self.is_open = true;
            }
            Ok(())
        }
    }

    /// Get the XMP metadata
    ///
    /// Returns `None` if no metadata has been loaded or found.
    pub fn get_xmp(&self) -> Option<&XmpMeta> {
        self.meta.as_ref()
    }

    /// Get mutable reference to XMP metadata
    ///
    /// Returns `None` if no metadata has been loaded or found.
    pub fn get_xmp_mut(&mut self) -> Option<&mut XmpMeta> {
        self.meta.as_mut()
    }

    /// Put XMP metadata
    ///
    /// Replaces any existing metadata.
    ///
    /// # Update Behavior
    ///
    /// - If the file was opened with [`ReadOptions::for_update`], changes are
    ///   not written to disk immediately. Call [`XmpFile::close`] or [`XmpFile::try_close`]
    ///   to write changes to disk.
    /// - If the file was opened read-only, this only updates the in-memory metadata.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use xmpkit::{XmpFile, ReadOptions, XmpMeta, XmpValue};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut file = XmpFile::new();
    /// file.open_with("image.jpg", ReadOptions::default().for_update())?;
    ///
    /// let mut meta = file.get_xmp().cloned().unwrap_or_else(XmpMeta::new);
    /// meta.set_property(
    ///     "http://ns.adobe.com/xap/1.0/",
    ///     "CreatorTool",
    ///     XmpValue::String("MyApp".to_string()),
    /// )?;
    /// file.put_xmp(meta);
    ///
    /// // Write changes to disk
    /// file.try_close()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn put_xmp(&mut self, meta: XmpMeta) {
        self.meta = Some(meta);
        // Note: Changes are written to disk when close() or try_close() is called
    }

    /// Explicitly closes an opened file.
    ///
    /// Performs any necessary output to the file and closes it. Files that are
    /// opened for update are written to only when closing.
    ///
    /// If the file is opened for read-only access (using
    /// [`ReadOptions::for_read`]), the disk file is closed
    /// immediately after reading the data from it; the `XmpFile`
    /// struct, however, remains in the open state. You must call
    /// [`XmpFile::close`] when finished using it.
    ///
    /// # Platform Support
    ///
    /// - **Native platforms**: Writes changes to disk if opened for update
    /// - **Wasm**: Only cleans up internal state (file writing not supported)
    ///
    /// # Errors
    ///
    /// This method ignores errors for backward compatibility. If you want to
    /// handle errors, use [`XmpFile::try_close`] instead.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use xmpkit::{XmpFile, ReadOptions};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut file = XmpFile::new();
    /// file.open_with("image.jpg", ReadOptions::default().for_update())?;
    /// // ... modify metadata ...
    /// file.close(); // Ignores errors
    /// # Ok(())
    /// # }
    /// ```
    pub fn close(&mut self) {
        let _ = self.try_close();
        // Ignore error for backward compatibility
    }

    /// Explicitly closes an opened file with error handling.
    ///
    /// Performs any necessary output to the file and closes it. Files that are
    /// opened for update are written to only when closing.
    ///
    /// If the file is opened for read-only access (using
    /// [`ReadOptions::for_read`]), the disk file is closed
    /// immediately after reading the data from it; the `XmpFile`
    /// struct, however, remains in the open state. You must call
    /// [`XmpFile::try_close`] when finished using it.
    ///
    /// # Platform Support
    ///
    /// - **Native platforms**: Writes changes to disk if opened for update
    /// - **Wasm**: Only cleans up internal state (file writing not supported)
    ///
    /// # Errors
    ///
    /// Returns an error if writing the file fails (native platforms only).
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use xmpkit::{XmpFile, ReadOptions};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut file = XmpFile::new();
    /// file.open_with("image.jpg", ReadOptions::default().for_update())?;
    /// // ... modify metadata ...
    /// file.try_close()?; // Returns error if write fails
    /// # Ok(())
    /// # }
    /// ```
    pub fn try_close(&mut self) -> XmpResult<()> {
        if !self.is_open {
            return Ok(());
        }

        // On native, if opened for update, write changes to disk
        #[cfg(not(target_arch = "wasm32"))]
        {
            if self.options.for_update {
                if let Some(ref path) = self.file_path {
                    if let Some(ref meta) = self.meta {
                        use std::fs::File;
                        use std::io::BufWriter;

                        // If handler is None (e.g., packet scanning mode), detect handler from file data
                        let handler = if let Some(ref h) = self.handler {
                            h.clone()
                        } else {
                            // Use file_data to detect handler (avoid re-opening file)
                            let registry = default_registry();
                            let mut reader =
                                Cursor::new(self.file_data.as_ref().ok_or_else(|| {
                                    XmpError::BadValue(
                                        "File data not available for handler detection".to_string(),
                                    )
                                })?);
                            registry
                                .find_by_detection(&mut reader)?
                                .ok_or_else(|| {
                                    XmpError::NotSupported(
                                        "Unsupported file format for writing".to_string(),
                                    )
                                })?
                                .clone()
                        };

                        // Read original file content first (before creating new file)
                        let file_data = self
                            .file_data
                            .as_ref()
                            .ok_or_else(|| {
                                XmpError::BadValue(
                                    "File data not available for writing".to_string(),
                                )
                            })?
                            .clone();
                        let mut reader = Cursor::new(&file_data);

                        // Write to same file (or create new one)
                        let mut writer = BufWriter::new(File::create(path)?);

                        // Write XMP
                        handler.write_xmp(&mut reader, &mut writer, meta)?;
                        writer.flush()?;
                    }
                }
            }
        }

        // On Wasm, we can't write to files, so just clean up state
        #[cfg(target_arch = "wasm32")]
        {
            // No-op
        }

        self.is_open = false;
        Ok(())
    }

    /// Write XMP metadata to a file path (native platforms only)
    ///
    /// # Platform Support
    ///
    /// - Native platforms (iOS, Android, macOS, Windows)
    /// - Wasm: Not supported (use `write_to_bytes()` or `write_to_writer()` instead)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use xmpkit::{XmpFile, XmpMeta};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut file = XmpFile::new();
    /// file.open("image.jpg")?;
    /// // ... modify metadata ...
    /// file.save("output.jpg")?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save<P: AsRef<std::path::Path>>(&self, path: P) -> XmpResult<()> {
        use std::fs::File;
        let file = File::create(path)?;
        self.write_to_writer(file)
    }

    /// Write XMP metadata to bytes (all platforms, including Wasm)
    ///
    /// This is the recommended method for Wasm environments.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use xmpkit::XmpFile;
    ///
    /// let input_data: &[u8] = /* your JPEG file data */;
    /// let mut file = XmpFile::new();
    /// file.from_bytes(input_data)?;
    /// // ... modify metadata ...
    /// let output_data = file.write_to_bytes()?;
    /// ```
    pub fn write_to_bytes(&self) -> XmpResult<Vec<u8>> {
        let mut buffer = Vec::new();
        let cursor = Cursor::new(&mut buffer);
        self.write_to_writer(cursor)?;
        Ok(buffer)
    }

    /// Write XMP metadata to a writer (all platforms, including Wasm)
    ///
    /// This is the most flexible method, accepting any type that implements
    /// `Write + Seek`.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use std::io::Cursor;
    /// use xmpkit::XmpFile;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut file = XmpFile::new();
    /// // ... load metadata ...
    /// let mut output = Vec::new();
    /// let cursor = Cursor::new(&mut output);
    /// file.write_to_writer(cursor)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn write_to_writer<W: Write + Seek>(&self, mut writer: W) -> XmpResult<()> {
        // Get XMP metadata
        let meta = self.meta.as_ref().ok_or_else(|| {
            XmpError::BadValue("No XMP metadata available for writing".to_string())
        })?;

        // Get original file data
        let file_data = self.file_data.as_ref().ok_or_else(|| {
            XmpError::BadValue("Original file data not available for writing".to_string())
        })?;

        // Detect handler from file data
        let registry = default_registry();
        let mut reader = Cursor::new(file_data);
        let handler = registry.find_by_detection(&mut reader)?.ok_or_else(|| {
            XmpError::NotSupported("Unsupported file format for writing".to_string())
        })?;

        // Reset reader position
        reader.set_position(0);

        // Write XMP using handler
        handler.write_xmp(&mut reader, &mut writer, meta)?;
        writer.flush()?;

        Ok(())
    }
}

impl Default for XmpFile {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let file = XmpFile::new();
        assert!(file.get_xmp().is_none());
    }

    #[test]
    fn test_from_bytes_empty() {
        let mut file = XmpFile::new();
        // Empty data should fail format detection
        let result = file.from_bytes(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_put_and_get_xmp() {
        let mut file = XmpFile::new();
        let meta = XmpMeta::new();
        file.put_xmp(meta);
        assert!(file.get_xmp().is_some());
    }
}
