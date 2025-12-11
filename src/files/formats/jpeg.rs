//! JPEG file format handler
//!
//! This module provides functionality for reading and writing XMP metadata
//! in JPEG files. The implementation is pure Rust and cross-platform compatible.
//!
//! JPEG XMP Storage:
//! - XMP Packet is stored in APP1 segment with identifier `<http://ns.adobe.com/xap/1.0/>\0`
//! - Extended XMP (if needed) uses GUID-based chunking in additional APP1 segments
//! - Standard APP1 segment size limit: 64KB (65535 bytes including header)

use crate::core::error::{XmpError, XmpResult};
use crate::core::metadata::XmpMeta;
use crate::files::handler::{FileHandler, XmpOptions};
use std::io::{Read, Seek, SeekFrom, Write};

/// JPEG segment markers
const MARKER_SOI: u8 = 0xD8; // Start of Image
const MARKER_APP0: u8 = 0xE0;
const MARKER_APP1: u8 = 0xE1;
const MARKER_APP15: u8 = 0xEF;
const MARKER_SOS: u8 = 0xDA; // Start of Scan
const MARKER_EOI: u8 = 0xD9; // End of Image

/// XMP namespace identifier in APP1 segment
const XMP_NAMESPACE: &[u8] = b"http://ns.adobe.com/xap/1.0/\0";

/// Extended XMP namespace identifier
const EXTENDED_XMP_NAMESPACE: &[u8] = b"http://ns.adobe.com/xap/1.0/ext/\0";

/// Exif signature in APP1 segment
const EXIF_SIGNATURE: &[u8] = b"Exif\0\x00";
const EXIF_SIGNATURE_ALT: &[u8] = b"Exif\0\xFF";
const EXIF_SIGNATURE_LENGTH: usize = 6;

/// Maximum size of a standard APP1 segment (64KB - 2 bytes for length)
const MAX_APP1_SIZE: usize = 65533;

/// JPEG file handler for XMP metadata
#[derive(Debug, Clone, Copy)]
pub struct JpegHandler;

impl FileHandler for JpegHandler {
    /// Check if this is a valid JPEG file:
    /// 1. Check for SOI marker (0xFFD8) at offset 0
    /// 2. Skip any 0xFF padding bytes
    /// 3. Validate the second marker ID
    fn can_handle<R: Read + Seek>(&self, reader: &mut R) -> XmpResult<bool> {
        let pos = reader.stream_position()?;

        // Need at least the SOI marker
        let file_len = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(pos))?;
        if file_len < 2 {
            return Ok(false);
        }

        // Read up to 100 bytes for validation
        let mut buffer = [0u8; 100];
        let bytes_read = reader.read(&mut buffer)?;
        reader.seek(SeekFrom::Start(pos))?;

        if bytes_read < 2 {
            return Ok(false);
        }

        // Offset 0 must have the SOI marker (0xFFD8)
        if buffer[0] != 0xFF || buffer[1] != MARKER_SOI {
            return Ok(false);
        }

        // Skip 0xFF padding and high order 0xFF of next marker
        let mut buffer_pos = 2;
        while buffer_pos < bytes_read && buffer[buffer_pos] == 0xFF {
            buffer_pos += 1;
        }

        // Nothing but 0xFF bytes after SOI, close enough
        if buffer_pos >= bytes_read {
            return Ok(true);
        }

        // Check the ID of the second marker
        let id = buffer[buffer_pos];

        // Most probable cases: RST markers, SOI, SOS, etc.
        if id >= 0xDD {
            return Ok(true);
        }

        // Invalid markers: standalone markers (0xD0-0xD7 RST, 0xD8 SOI, 0xDA SOS, 0xDC DNL)
        // and anything below 0xC0
        if id < 0xC0 || (id & 0xF8) == 0xD0 || id == 0xD8 || id == 0xDA || id == 0xDC {
            return Ok(false);
        }

        Ok(true)
    }

    fn read_xmp<R: Read + Seek>(
        &self,
        reader: &mut R,
        _options: &XmpOptions,
    ) -> XmpResult<Option<XmpMeta>> {
        Self::read_xmp(reader)
    }

    fn write_xmp<R: Read + Seek, W: Write + Seek>(
        &self,
        reader: &mut R,
        writer: &mut W,
        meta: &XmpMeta,
    ) -> XmpResult<()> {
        Self::write_xmp(reader, writer, meta)
    }

    fn format_name(&self) -> &'static str {
        "JPEG"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["jpg", "jpeg"]
    }
}

impl JpegHandler {
    /// Read XMP metadata from a JPEG file
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
    ///
    /// # Platform Compatibility
    ///
    /// This function uses only standard Rust I/O traits (`Read`, `Seek`),
    /// making it compatible with all platforms including Wasm.
    pub fn read_xmp<R: Read + Seek>(mut reader: R) -> XmpResult<Option<XmpMeta>> {
        // Check JPEG file header (SOI marker)
        let mut header = [0u8; 2];
        reader.read_exact(&mut header)?;

        if header[0] != 0xFF || header[1] != MARKER_SOI {
            return Err(XmpError::BadValue("Not a valid JPEG file".to_string()));
        }

        // Search for APP1 segments containing XMP
        let mut xmp_data = Vec::new();
        let mut extended_xmp_parts: Vec<(u32, Vec<u8>)> = Vec::new();

        loop {
            // Find next marker
            let marker = Self::find_marker(&mut reader)?;
            if marker == MARKER_EOI || marker == MARKER_SOS {
                break;
            }

            // Process APP1 segments
            if (MARKER_APP0..=MARKER_APP15).contains(&marker) {
                Self::process_app_segment(
                    &mut reader,
                    marker,
                    &mut xmp_data,
                    &mut extended_xmp_parts,
                )?;
            } else {
                // Skip other segments
                let length = Self::read_segment_length(&mut reader)?;
                reader.seek(SeekFrom::Current(length as i64 - 2))?;
            }
        }

        // Reconstruct Extended XMP if present
        if !extended_xmp_parts.is_empty() {
            xmp_data = Self::reconstruct_extended_xmp(extended_xmp_parts)?;
        }

        if xmp_data.is_empty() {
            return Ok(None);
        }

        // Parse XMP Packet
        let xmp_str = String::from_utf8(xmp_data)
            .map_err(|e| XmpError::ParseError(format!("Invalid UTF-8 in XMP: {}", e)))?;

        XmpMeta::parse(&xmp_str).map(Some)
    }

    /// Write XMP metadata to a JPEG file
    ///
    /// # Arguments
    ///
    /// * `reader` - A reader implementing `Read + Seek` for the source file
    /// * `writer` - A writer implementing `Write + Seek` for the output file
    /// * `meta` - The XMP metadata to write
    ///
    /// # Platform Compatibility
    ///
    /// This function uses only standard Rust I/O traits (`Read`, `Seek`, `Write`),
    /// making it compatible with all platforms including Wasm.
    pub fn write_xmp<R: Read + Seek, W: Write + Seek>(
        mut reader: R,
        mut writer: W,
        meta: &XmpMeta,
    ) -> XmpResult<()> {
        // Serialize XMP metadata
        let xmp_packet = meta.serialize_packet()?;
        let xmp_bytes = xmp_packet.as_bytes();

        // Check if we need Extended XMP
        if xmp_bytes.len() > MAX_APP1_SIZE {
            return Err(XmpError::NotSupported(
                "Extended XMP not yet implemented".to_string(),
            ));
        }

        // Read source file header
        let mut header = [0u8; 2];
        reader.read_exact(&mut header)?;
        writer.write_all(&header)?;

        if header[0] != 0xFF || header[1] != MARKER_SOI {
            return Err(XmpError::BadValue("Not a valid JPEG file".to_string()));
        }

        // Copy any leading APP0 marker segments
        // Reference: xmp-toolkit-rs/external/xmp_toolkit/XMPFiles/source/FileHandlers/JPEG_Handler.cpp
        while let Ok(marker) = Self::find_marker(&mut reader) {
            if marker != MARKER_APP0 {
                // Not an APP0 segment, back up to the marker
                reader.seek(SeekFrom::Current(-2))?;
                break;
            }

            // Copy APP0 segment
            writer.write_all(&[0xFF, MARKER_APP0])?;
            let length = Self::read_segment_length(&mut reader)?;
            writer.write_all(&length.to_be_bytes())?;

            let mut buffer = vec![0u8; length as usize - 2];
            reader.read_exact(&mut buffer)?;
            writer.write_all(&buffer)?;
        }

        // Write XMP APP1 segment
        Self::write_app1_xmp_segment(&mut writer, xmp_bytes)?;

        // Copy remaining segments, skipping old XMP segments, until SOS or EOI
        // The APP0 copy loop already read the next marker and backed up, so we're at the start of the next segment
        loop {
            let marker = match Self::find_marker(&mut reader) {
                Ok(m) => m,
                Err(e)
                    if e.to_string().contains("UnexpectedEof")
                        || e.to_string().contains("failed to fill") =>
                {
                    // Reached end of file unexpectedly
                    break;
                }
                Err(e) => return Err(e),
            };

            // Quit at the first SOS marker or at EOI
            if marker == MARKER_SOS || marker == MARKER_EOI {
                // Back up to the marker (find_marker already read past it)
                reader.seek(SeekFrom::Current(-2))?;
                break;
            }

            if (MARKER_APP0..=MARKER_APP15).contains(&marker) {
                Self::process_app_segment_write(&mut reader, marker, &mut writer)?;
            } else {
                // Copy other segments
                writer.write_all(&[0xFF, marker])?;
                let length = Self::read_segment_length(&mut reader)?;
                writer.write_all(&length.to_be_bytes())?;

                let mut buffer = vec![0u8; length as usize - 2];
                reader.read_exact(&mut buffer)?;
                writer.write_all(&buffer)?;
            }
        }

        // Copy the remainder of the source file (from current position to end)
        // This includes SOS segment, scan data, and EOI marker
        let current_pos = reader.stream_position()?;
        reader.seek(SeekFrom::End(0))?;
        let file_end = reader.stream_position()?;
        reader.seek(SeekFrom::Start(current_pos))?;

        let remaining = file_end - current_pos;
        if remaining > 0 {
            // Copy in chunks to avoid loading entire file into memory
            let mut buffer = vec![0u8; 64 * 1024]; // 64KB buffer
            let mut copied = 0u64;
            while copied < remaining {
                let to_read = std::cmp::min(buffer.len() as u64, remaining - copied) as usize;
                let n = reader.read(&mut buffer[..to_read])?;
                if n == 0 {
                    break;
                }
                writer.write_all(&buffer[..n])?;
                copied += n as u64;
            }
        }

        Ok(())
    }

    /// Process an APP segment during read operation
    fn process_app_segment<R: Read>(
        reader: &mut R,
        marker: u8,
        xmp_data: &mut Vec<u8>,
        extended_xmp_parts: &mut Vec<(u32, Vec<u8>)>,
    ) -> XmpResult<()> {
        let Some(segment_data) = Self::read_app_segment(reader, marker)? else {
            return Ok(());
        };

        if Self::is_xmp_segment(&segment_data) {
            *xmp_data = Self::extract_xmp_data(&segment_data)?;
        } else if Self::is_extended_xmp_segment(&segment_data) {
            if let Some((guid, data)) = Self::extract_extended_xmp_data(&segment_data)? {
                extended_xmp_parts.push((guid, data));
            }
        }

        Ok(())
    }

    /// Process an APP segment during write operation
    fn process_app_segment_write<R: Read + Seek, W: Write>(
        reader: &mut R,
        marker: u8,
        writer: &mut W,
    ) -> XmpResult<()> {
        // Read segment length first
        let length = Self::read_segment_length(reader)?;
        if length < 2 {
            return Ok(());
        }
        let content_len = length - 2; // Content length (excluding the 2 bytes for length itself)

        // Save current position (start of segment content)
        let content_origin = reader.stream_position()?;

        // Read signature to check segment type
        // For APP1, we need to check for Exif, XMP, or Extended XMP
        let mut copy_segment = true;

        if marker == MARKER_APP1 && content_len >= EXIF_SIGNATURE_LENGTH as u16 {
            // Read enough bytes to check for the longest signature (Extended XMP)
            let max_sig_len = std::cmp::max(
                std::cmp::max(EXIF_SIGNATURE_LENGTH, XMP_NAMESPACE.len()),
                EXTENDED_XMP_NAMESPACE.len(),
            );
            let sig_len = std::cmp::min(content_len as usize, max_sig_len);
            let mut signature = vec![0u8; sig_len];
            reader.read_exact(&mut signature)?;

            // Check for Exif signature
            if sig_len >= EXIF_SIGNATURE_LENGTH
                && (signature[..EXIF_SIGNATURE_LENGTH] == *EXIF_SIGNATURE
                    || signature[..EXIF_SIGNATURE_LENGTH] == *EXIF_SIGNATURE_ALT)
            {
                // Keep Exif segments - we're not writing new Exif, so preserve the original
                copy_segment = true;
            }

            // Check for XMP signatures
            if sig_len >= XMP_NAMESPACE.len() && signature[..XMP_NAMESPACE.len()] == *XMP_NAMESPACE
            {
                copy_segment = false; // Skip old XMP
            }

            if sig_len >= EXTENDED_XMP_NAMESPACE.len()
                && signature[..EXTENDED_XMP_NAMESPACE.len()] == *EXTENDED_XMP_NAMESPACE
            {
                copy_segment = false; // Skip old Extended XMP
            }

            // Seek back to content origin to read the full segment if we're copying it
            reader.seek(SeekFrom::Start(content_origin))?;
        }

        if !copy_segment {
            // Skip this segment - seek past it
            reader.seek(SeekFrom::Start(content_origin + content_len as u64))?;
            return Ok(());
        }

        // Copy the segment: write marker, length, and content
        writer.write_all(&[0xFF, marker])?;
        writer.write_all(&length.to_be_bytes())?;

        // Copy segment content
        let mut buffer = vec![0u8; content_len as usize];
        reader.read_exact(&mut buffer)?;
        writer.write_all(&buffer)?;

        Ok(())
    }

    /// Find the next JPEG marker
    fn find_marker<R: Read>(reader: &mut R) -> XmpResult<u8> {
        let mut buffer = [0u8; 1];
        loop {
            reader.read_exact(&mut buffer)?;
            if buffer[0] == 0xFF {
                reader.read_exact(&mut buffer)?;
                if buffer[0] != 0x00 && buffer[0] != 0xFF {
                    return Ok(buffer[0]);
                }
            }
        }
    }

    /// Read segment length (2 bytes, big-endian)
    fn read_segment_length<R: Read>(reader: &mut R) -> XmpResult<u16> {
        let mut length_bytes = [0u8; 2];
        reader.read_exact(&mut length_bytes)?;
        Ok(u16::from_be_bytes(length_bytes))
    }

    /// Read an APP segment
    fn read_app_segment<R: Read>(reader: &mut R, _marker: u8) -> XmpResult<Option<Vec<u8>>> {
        let length = Self::read_segment_length(reader)?;
        if length < 2 {
            return Ok(None);
        }

        let mut data = vec![0u8; length as usize - 2];
        reader.read_exact(&mut data)?;
        Ok(Some(data))
    }

    /// Check if a segment is an XMP segment
    fn is_xmp_segment(segment_data: &[u8]) -> bool {
        segment_data.len() >= XMP_NAMESPACE.len()
            && segment_data[..XMP_NAMESPACE.len()] == *XMP_NAMESPACE
    }

    /// Check if a segment is an Extended XMP segment
    fn is_extended_xmp_segment(segment_data: &[u8]) -> bool {
        segment_data.len() >= EXTENDED_XMP_NAMESPACE.len()
            && segment_data[..EXTENDED_XMP_NAMESPACE.len()] == *EXTENDED_XMP_NAMESPACE
    }

    /// Extract XMP data from APP1 segment
    fn extract_xmp_data(segment_data: &[u8]) -> XmpResult<Vec<u8>> {
        if segment_data.len() < XMP_NAMESPACE.len() {
            return Err(XmpError::BadValue("Invalid XMP segment".to_string()));
        }

        Ok(segment_data[XMP_NAMESPACE.len()..].to_vec())
    }

    /// Extract Extended XMP data from APP1 segment
    fn extract_extended_xmp_data(segment_data: &[u8]) -> XmpResult<Option<(u32, Vec<u8>)>> {
        if segment_data.len() < EXTENDED_XMP_NAMESPACE.len() + 36 {
            return Ok(None);
        }

        // GUID is 32 bytes (128 bits) after namespace
        let guid_start = EXTENDED_XMP_NAMESPACE.len();
        let _guid_bytes = &segment_data[guid_start..guid_start + 32];

        // Read chunk info (offset and total size)
        let offset_start = guid_start + 32;
        if segment_data.len() < offset_start + 8 {
            return Ok(None);
        }

        let offset = u32::from_be_bytes([
            segment_data[offset_start],
            segment_data[offset_start + 1],
            segment_data[offset_start + 2],
            segment_data[offset_start + 3],
        ]);

        let _total_size = u32::from_be_bytes([
            segment_data[offset_start + 4],
            segment_data[offset_start + 5],
            segment_data[offset_start + 6],
            segment_data[offset_start + 7],
        ]);

        // Extract chunk data
        let data_start = offset_start + 8;
        if segment_data.len() < data_start {
            return Ok(None);
        }

        let data = segment_data[data_start..].to_vec();
        Ok(Some((offset, data)))
    }

    /// Reconstruct Extended XMP from chunks
    fn reconstruct_extended_xmp(chunks: Vec<(u32, Vec<u8>)>) -> XmpResult<Vec<u8>> {
        // Sort chunks by offset
        let mut sorted_chunks = chunks;
        sorted_chunks.sort_by_key(|(offset, _)| *offset);

        // Concatenate chunks
        let mut result = Vec::new();
        for (_, data) in sorted_chunks {
            result.extend_from_slice(&data);
        }

        Ok(result)
    }

    /// Write APP1 XMP segment
    fn write_app1_xmp_segment<W: Write>(writer: &mut W, xmp_data: &[u8]) -> XmpResult<()> {
        // Write marker
        writer.write_all(&[0xFF, MARKER_APP1])?;

        // Calculate segment length (namespace + data + 2 bytes for length)
        let segment_length = (XMP_NAMESPACE.len() + xmp_data.len() + 2) as u16;
        writer.write_all(&segment_length.to_be_bytes())?;

        // Write namespace identifier
        writer.write_all(XMP_NAMESPACE)?;

        // Write XMP data
        writer.write_all(xmp_data)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::metadata::XmpMeta;
    use crate::core::namespace::ns;
    use crate::types::value::XmpValue;
    use std::io::Cursor;

    // Minimal valid JPEG file with no XMP (SOI + EOI)
    fn create_minimal_jpeg() -> Vec<u8> {
        vec![0xFF, MARKER_SOI, 0xFF, MARKER_EOI]
    }

    #[test]
    fn test_read_xmp_no_xmp() {
        let jpeg_data = create_minimal_jpeg();
        let reader = Cursor::new(jpeg_data);
        let result = JpegHandler::read_xmp(reader).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_invalid_jpeg() {
        let invalid_data = vec![0x00, 0x01, 0x02, 0x03];
        let reader = Cursor::new(invalid_data);
        let result = JpegHandler::read_xmp(reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_write_xmp() {
        // Create minimal JPEG
        let jpeg_data = create_minimal_jpeg();
        let reader = Cursor::new(jpeg_data);
        let mut writer = Cursor::new(Vec::new());

        // Create XMP metadata
        let mut meta = XmpMeta::new();
        meta.set_property(ns::DC, "title", XmpValue::String("Test Image".to_string()))
            .unwrap();

        // Write XMP
        JpegHandler::write_xmp(reader, &mut writer, &meta).unwrap();

        // Read back XMP
        writer.set_position(0);
        let result = JpegHandler::read_xmp(writer).unwrap();
        assert!(result.is_some());

        let read_meta = result.unwrap();
        let title_value = read_meta.get_property(ns::DC, "title");
        assert!(title_value.is_some());
        if let Some(XmpValue::String(title)) = title_value {
            assert_eq!(title, "Test Image");
        } else {
            panic!("Expected string value");
        }
    }

    #[test]
    fn test_is_xmp_segment() {
        let mut segment = XMP_NAMESPACE.to_vec();
        segment.extend_from_slice(b"<rdf:RDF>...</rdf:RDF>");
        assert!(JpegHandler::is_xmp_segment(&segment));

        let other_segment = b"JFIF\0";
        assert!(!JpegHandler::is_xmp_segment(other_segment));
    }

    #[test]
    fn test_extract_xmp_data() {
        let mut segment = XMP_NAMESPACE.to_vec();
        let xmp_content = b"<rdf:RDF>test</rdf:RDF>";
        segment.extend_from_slice(xmp_content);

        let extracted = JpegHandler::extract_xmp_data(&segment).unwrap();
        assert_eq!(extracted, xmp_content);
    }
}
