//! WebP file format handler
//!
//! This module provides functionality for reading and writing XMP metadata
//! in WebP files. The implementation is pure Rust and cross-platform compatible.
//!
//! WebP XMP Storage:
//! - WebP uses RIFF container format
//! - XMP is stored in a chunk with FourCC "XMP " (note the trailing space)
//! - Each chunk has: 4-byte ID, 4-byte size (little-endian), data, optional padding byte
//!
//! Reference: RFC 9649 - WebP Image Format

use crate::core::error::{XmpError, XmpResult};
use crate::core::metadata::XmpMeta;
use crate::files::handler::{FileHandler, XmpOptions};
use std::io::{Read, Seek, SeekFrom, Write};

/// RIFF file signature
const RIFF_SIGNATURE: &[u8; 4] = b"RIFF";

/// WebP format identifier
const WEBP_SIGNATURE: &[u8; 4] = b"WEBP";

/// XMP chunk FourCC (note the trailing space)
const XMP_CHUNK_ID: &[u8; 4] = b"XMP ";

/// VP8X chunk FourCC (extended format)
const VP8X_CHUNK_ID: &[u8; 4] = b"VP8X";

/// VP8 chunk FourCC (lossy format)
const VP8_CHUNK_ID: &[u8; 4] = b"VP8 ";

/// VP8L chunk FourCC (lossless format)
const VP8L_CHUNK_ID: &[u8; 4] = b"VP8L";

/// RIFF header size (RIFF + size + WEBP)
const RIFF_HEADER_SIZE: u64 = 12;

/// Chunk header size (ID + size)
const CHUNK_HEADER_SIZE: u64 = 8;

/// VP8X flags bit for XMP metadata
const VP8X_XMP_FLAG: u8 = 0x04;

/// WebP file handler for XMP metadata
#[derive(Debug, Clone, Copy)]
pub struct WebpHandler;

impl FileHandler for WebpHandler {
    /// Check if this is a valid WebP file:
    /// 1. File length >= 12 bytes (RIFF header)
    /// 2. Check "RIFF" signature at offset 0
    /// 3. Check "WEBP" signature at offset 8
    fn can_handle<R: Read + Seek>(&self, reader: &mut R) -> XmpResult<bool> {
        let pos = reader.stream_position()?;

        // Check minimum file length
        let file_len = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(pos))?;
        if file_len < 12 {
            return Ok(false);
        }

        let mut header = [0u8; 12];
        match reader.read_exact(&mut header) {
            Ok(_) => {
                reader.seek(SeekFrom::Start(pos))?;
                Ok(&header[0..4] == RIFF_SIGNATURE && &header[8..12] == WEBP_SIGNATURE)
            }
            Err(_) => {
                reader.seek(SeekFrom::Start(pos))?;
                Ok(false)
            }
        }
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
        "WebP"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["webp"]
    }
}

/// Information about a chunk in the file
#[derive(Debug, Clone)]
struct ChunkInfo {
    /// Chunk FourCC ID
    id: [u8; 4],
    /// Chunk data size (excluding header and padding)
    size: u32,
    /// Position of chunk header in file
    offset: u64,
}

impl WebpHandler {
    /// Read XMP metadata from a WebP file
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
    pub fn read_xmp<R: Read + Seek>(mut reader: R) -> XmpResult<Option<XmpMeta>> {
        // Validate WebP header
        Self::validate_webp_header(&mut reader)?;

        // Search for XMP chunk
        reader.seek(SeekFrom::Start(RIFF_HEADER_SIZE))?;

        while let Ok(chunk) = Self::read_chunk_header(&mut reader) {
            if chunk.id == *XMP_CHUNK_ID {
                // Found XMP chunk, read its data
                let mut xmp_data = vec![0u8; chunk.size as usize];
                reader.read_exact(&mut xmp_data)?;

                let xmp_str = String::from_utf8(xmp_data)
                    .map_err(|e| XmpError::ParseError(format!("Invalid UTF-8 in XMP: {}", e)))?;

                return XmpMeta::parse(&xmp_str).map(Some);
            }

            // Skip this chunk (data + padding)
            Self::skip_chunk_data(&mut reader, chunk.size)?;
        }

        Ok(None)
    }

    /// Write XMP metadata to a WebP file
    ///
    /// # Arguments
    ///
    /// * `reader` - A reader implementing `Read + Seek` for the source file
    /// * `writer` - A writer implementing `Write + Seek` for the output file
    /// * `meta` - The XMP metadata to write
    pub fn write_xmp<R: Read + Seek, W: Write + Seek>(
        mut reader: R,
        mut writer: W,
        meta: &XmpMeta,
    ) -> XmpResult<()> {
        // Validate WebP header
        Self::validate_webp_header(&mut reader)?;

        // Serialize XMP metadata
        let xmp_packet = meta.serialize_packet()?;
        let xmp_bytes = xmp_packet.as_bytes();

        // Read all chunks and find relevant information
        reader.seek(SeekFrom::Start(RIFF_HEADER_SIZE))?;
        let chunks = Self::read_all_chunks(&mut reader)?;

        // Find existing XMP chunk and VP8X chunk
        let xmp_chunk = chunks.iter().find(|c| c.id == *XMP_CHUNK_ID);
        let vp8x_chunk = chunks.iter().find(|c| c.id == *VP8X_CHUNK_ID);

        // Calculate new file size
        let old_xmp_size = xmp_chunk
            .map(|c| Self::chunk_total_size(c.size))
            .unwrap_or(0);
        let new_xmp_size = Self::chunk_total_size(xmp_bytes.len() as u32);

        reader.seek(SeekFrom::Start(0))?;

        // Read original RIFF header
        let mut riff_header = [0u8; 4];
        reader.read_exact(&mut riff_header)?;
        let mut old_file_size_bytes = [0u8; 4];
        reader.read_exact(&mut old_file_size_bytes)?;
        let old_file_size = u32::from_le_bytes(old_file_size_bytes);
        let mut webp_sig = [0u8; 4];
        reader.read_exact(&mut webp_sig)?;

        // Calculate new RIFF size
        let new_file_size = if xmp_chunk.is_some() {
            // Replace existing XMP
            old_file_size - old_xmp_size as u32 + new_xmp_size as u32
        } else {
            // Add new XMP chunk
            let vp8x_addition = if vp8x_chunk.is_none() {
                // Need to add VP8X chunk (header + 10 bytes data)
                Self::chunk_total_size(10) as u32
            } else {
                0
            };
            old_file_size + new_xmp_size as u32 + vp8x_addition
        };

        // Write new RIFF header
        writer.write_all(RIFF_SIGNATURE)?;
        writer.write_all(&new_file_size.to_le_bytes())?;
        writer.write_all(WEBP_SIGNATURE)?;

        // Process chunks
        let needs_vp8x = vp8x_chunk.is_none();
        let mut xmp_written = false;
        let mut vp8x_written = false;

        for chunk in &chunks {
            if chunk.id == *XMP_CHUNK_ID {
                // Skip old XMP chunk, write new one at appropriate position
                continue;
            }

            if chunk.id == *VP8X_CHUNK_ID {
                // Update VP8X chunk with XMP flag
                reader.seek(SeekFrom::Start(chunk.offset + CHUNK_HEADER_SIZE))?;
                let mut vp8x_data = vec![0u8; chunk.size as usize];
                reader.read_exact(&mut vp8x_data)?;

                // Set XMP flag (bit 2)
                if !vp8x_data.is_empty() {
                    vp8x_data[0] |= VP8X_XMP_FLAG;
                }

                // Write updated VP8X chunk
                writer.write_all(VP8X_CHUNK_ID)?;
                writer.write_all(&chunk.size.to_le_bytes())?;
                writer.write_all(&vp8x_data)?;
                if chunk.size % 2 == 1 {
                    writer.write_all(&[0])?;
                }
                vp8x_written = true;

                // Write XMP chunk right after VP8X
                Self::write_xmp_chunk(&mut writer, xmp_bytes)?;
                xmp_written = true;
                continue;
            }

            // For VP8/VP8L (simple WebP without VP8X), insert VP8X and XMP before it
            if needs_vp8x
                && !vp8x_written
                && (chunk.id == *VP8_CHUNK_ID || chunk.id == *VP8L_CHUNK_ID)
            {
                // Need to create VP8X chunk first
                // Read image dimensions from VP8/VP8L chunk
                let (width, height) = Self::read_image_dimensions(&mut reader, chunk)?;

                // Write VP8X chunk
                Self::write_vp8x_chunk(&mut writer, width, height, VP8X_XMP_FLAG)?;
                vp8x_written = true;

                // Write XMP chunk
                Self::write_xmp_chunk(&mut writer, xmp_bytes)?;
                xmp_written = true;
            }

            // Copy chunk as-is
            reader.seek(SeekFrom::Start(chunk.offset))?;
            let total_size = Self::chunk_total_size(chunk.size);
            Self::copy_bytes(&mut reader, &mut writer, total_size)?;
        }

        // If XMP wasn't written yet (no VP8X, no VP8/VP8L found), append at end
        if !xmp_written {
            Self::write_xmp_chunk(&mut writer, xmp_bytes)?;
        }

        Ok(())
    }

    /// Validate WebP file header
    fn validate_webp_header<R: Read + Seek>(reader: &mut R) -> XmpResult<()> {
        reader.seek(SeekFrom::Start(0))?;
        let mut header = [0u8; 12];
        reader.read_exact(&mut header)?;

        if &header[0..4] != RIFF_SIGNATURE {
            return Err(XmpError::BadValue("Not a valid RIFF file".to_string()));
        }

        if &header[8..12] != WEBP_SIGNATURE {
            return Err(XmpError::BadValue("Not a valid WebP file".to_string()));
        }

        Ok(())
    }

    /// Read chunk header with proper offset tracking
    fn read_chunk_header<R: Read + Seek>(reader: &mut R) -> XmpResult<ChunkInfo> {
        let offset = reader.stream_position()?;

        let mut id = [0u8; 4];
        reader.read_exact(&mut id)?;

        let mut size_bytes = [0u8; 4];
        reader.read_exact(&mut size_bytes)?;
        let size = u32::from_le_bytes(size_bytes);

        Ok(ChunkInfo { id, size, offset })
    }

    /// Read all chunks in the file
    fn read_all_chunks<R: Read + Seek>(reader: &mut R) -> XmpResult<Vec<ChunkInfo>> {
        let mut chunks = Vec::new();

        while let Ok(chunk) = Self::read_chunk_header(reader) {
            chunks.push(chunk.clone());
            Self::skip_chunk_data(reader, chunk.size)?;
        }

        Ok(chunks)
    }

    /// Skip chunk data (including padding byte if odd size)
    fn skip_chunk_data<R: Read + Seek>(reader: &mut R, size: u32) -> XmpResult<()> {
        let padded_size = if size % 2 == 1 { size + 1 } else { size };
        reader.seek(SeekFrom::Current(padded_size as i64))?;
        Ok(())
    }

    /// Calculate total chunk size including header and padding
    fn chunk_total_size(data_size: u32) -> u64 {
        let padded_data = if data_size % 2 == 1 {
            data_size + 1
        } else {
            data_size
        };
        CHUNK_HEADER_SIZE + padded_data as u64
    }

    /// Write XMP chunk
    fn write_xmp_chunk<W: Write>(writer: &mut W, xmp_data: &[u8]) -> XmpResult<()> {
        let size = xmp_data.len() as u32;

        writer.write_all(XMP_CHUNK_ID)?;
        writer.write_all(&size.to_le_bytes())?;
        writer.write_all(xmp_data)?;

        // Add padding byte if odd size
        if size % 2 == 1 {
            writer.write_all(&[0])?;
        }

        Ok(())
    }

    /// Write VP8X chunk
    fn write_vp8x_chunk<W: Write>(
        writer: &mut W,
        width: u32,
        height: u32,
        flags: u8,
    ) -> XmpResult<()> {
        // VP8X chunk is always 10 bytes
        let size: u32 = 10;

        writer.write_all(VP8X_CHUNK_ID)?;
        writer.write_all(&size.to_le_bytes())?;

        // Flags (1 byte) + Reserved (3 bytes)
        writer.write_all(&[flags, 0, 0, 0])?;

        // Canvas width - 1 (3 bytes, little-endian)
        let w = (width.saturating_sub(1)) & 0xFFFFFF;
        writer.write_all(&[w as u8, (w >> 8) as u8, (w >> 16) as u8])?;

        // Canvas height - 1 (3 bytes, little-endian)
        let h = (height.saturating_sub(1)) & 0xFFFFFF;
        writer.write_all(&[h as u8, (h >> 8) as u8, (h >> 16) as u8])?;

        Ok(())
    }

    /// Read image dimensions from VP8 or VP8L chunk
    fn read_image_dimensions<R: Read + Seek>(
        reader: &mut R,
        chunk: &ChunkInfo,
    ) -> XmpResult<(u32, u32)> {
        reader.seek(SeekFrom::Start(chunk.offset + CHUNK_HEADER_SIZE))?;

        if chunk.id == *VP8_CHUNK_ID {
            // VP8 bitstream format
            // Skip frame tag (3 bytes) if keyframe
            let mut header = [0u8; 10];
            reader.read_exact(&mut header)?;

            // Check for VP8 signature (0x9D 0x01 0x2A)
            if header[3] == 0x9D && header[4] == 0x01 && header[5] == 0x2A {
                let width = u16::from_le_bytes([header[6], header[7]]) & 0x3FFF;
                let height = u16::from_le_bytes([header[8], header[9]]) & 0x3FFF;
                return Ok((width as u32, height as u32));
            }
        } else if chunk.id == *VP8L_CHUNK_ID {
            // VP8L bitstream format
            let mut header = [0u8; 5];
            reader.read_exact(&mut header)?;

            // Check signature byte (0x2F)
            if header[0] == 0x2F {
                // Width and height are encoded in 14 bits each
                let bits = u32::from_le_bytes([header[1], header[2], header[3], header[4]]);
                let width = (bits & 0x3FFF) + 1;
                let height = ((bits >> 14) & 0x3FFF) + 1;
                return Ok((width, height));
            }
        }

        // Default fallback
        Ok((1, 1))
    }

    /// Copy bytes from reader to writer
    fn copy_bytes<R: Read, W: Write>(reader: &mut R, writer: &mut W, count: u64) -> XmpResult<()> {
        let mut buffer = [0u8; 8192];
        let mut remaining = count;

        while remaining > 0 {
            let to_read = (remaining as usize).min(buffer.len());
            let n = reader.read(&mut buffer[..to_read])?;
            if n == 0 {
                break;
            }
            writer.write_all(&buffer[..n])?;
            remaining -= n as u64;
        }

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

    /// Create a minimal valid WebP file (simple format with VP8L)
    fn create_minimal_webp() -> Vec<u8> {
        let mut webp = Vec::new();

        // RIFF header
        webp.extend_from_slice(RIFF_SIGNATURE);

        // VP8L chunk data (minimal lossless image)
        // Signature (1 byte) + image size info (4 bytes) = 5 bytes minimum
        // But we need valid VP8L data, so let's create a 1x1 image
        let vp8l_data: Vec<u8> = vec![
            0x2F, // VP8L signature
            0x00, 0x00, 0x00, 0x00, // Width=1, Height=1, alpha=0, version=0
            0x10, 0x07, 0x10, 0x11, 0x11, 0x88, 0x88, 0x08, 0x08, // Minimal image data
        ];

        // File size = WEBP(4) + VP8L chunk header(8) + VP8L data
        let file_size = 4 + 8 + vp8l_data.len();
        webp.extend_from_slice(&(file_size as u32).to_le_bytes());

        // WEBP signature
        webp.extend_from_slice(WEBP_SIGNATURE);

        // VP8L chunk
        webp.extend_from_slice(VP8L_CHUNK_ID);
        webp.extend_from_slice(&(vp8l_data.len() as u32).to_le_bytes());
        webp.extend_from_slice(&vp8l_data);

        // Add padding if odd size
        if vp8l_data.len() % 2 == 1 {
            webp.push(0);
        }

        webp
    }

    /// Create a WebP file with VP8X (extended format)
    fn create_extended_webp() -> Vec<u8> {
        let mut webp = Vec::new();

        // RIFF header
        webp.extend_from_slice(RIFF_SIGNATURE);

        // VP8X chunk (10 bytes)
        let vp8x_data: Vec<u8> = vec![
            0x00, 0x00, 0x00, 0x00, // flags + reserved
            0x00, 0x00, 0x00, // width - 1 (1 pixel)
            0x00, 0x00, 0x00, // height - 1 (1 pixel)
        ];

        // VP8L chunk data
        let vp8l_data: Vec<u8> = vec![
            0x2F, // VP8L signature
            0x00, 0x00, 0x00, 0x00, // Width=1, Height=1
            0x10, 0x07, 0x10, 0x11, 0x11, 0x88, 0x88, 0x08, 0x08,
        ];

        // File size = WEBP(4) + VP8X chunk(18) + VP8L chunk
        let file_size = 4 + 8 + vp8x_data.len() + 8 + vp8l_data.len();
        webp.extend_from_slice(&(file_size as u32).to_le_bytes());

        // WEBP signature
        webp.extend_from_slice(WEBP_SIGNATURE);

        // VP8X chunk
        webp.extend_from_slice(VP8X_CHUNK_ID);
        webp.extend_from_slice(&(vp8x_data.len() as u32).to_le_bytes());
        webp.extend_from_slice(&vp8x_data);

        // VP8L chunk
        webp.extend_from_slice(VP8L_CHUNK_ID);
        webp.extend_from_slice(&(vp8l_data.len() as u32).to_le_bytes());
        webp.extend_from_slice(&vp8l_data);

        // Add padding if odd size
        if vp8l_data.len() % 2 == 1 {
            webp.push(0);
        }

        webp
    }

    #[test]
    fn test_can_handle_webp() {
        let handler = WebpHandler;
        let webp_data = create_minimal_webp();
        let mut reader = Cursor::new(webp_data);
        assert!(handler.can_handle(&mut reader).unwrap());
    }

    #[test]
    fn test_can_handle_non_webp() {
        let handler = WebpHandler;
        let non_webp_data = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
        let mut reader = Cursor::new(non_webp_data);
        assert!(!handler.can_handle(&mut reader).unwrap());
    }

    #[test]
    fn test_read_xmp_no_xmp() {
        let webp_data = create_minimal_webp();
        let reader = Cursor::new(webp_data);
        let result = WebpHandler::read_xmp(reader).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_invalid_webp() {
        let invalid_data = vec![0x00, 0x01, 0x02, 0x03];
        let reader = Cursor::new(invalid_data);
        let result = WebpHandler::read_xmp(reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_write_and_read_xmp_simple() {
        // Create minimal WebP
        let webp_data = create_minimal_webp();
        let reader = Cursor::new(webp_data);
        let mut writer = Cursor::new(Vec::new());

        // Create XMP metadata
        let mut meta = XmpMeta::new();
        meta.set_property(ns::DC, "title", XmpValue::String("Test WebP".to_string()))
            .unwrap();

        // Write XMP
        WebpHandler::write_xmp(reader, &mut writer, &meta).unwrap();

        // Read back XMP
        writer.set_position(0);
        let result = WebpHandler::read_xmp(writer).unwrap();
        assert!(result.is_some());

        let read_meta = result.unwrap();
        let title_value = read_meta.get_property(ns::DC, "title");
        assert!(title_value.is_some());
        if let Some(XmpValue::String(title)) = title_value {
            assert_eq!(title, "Test WebP");
        } else {
            panic!("Expected string value");
        }
    }

    #[test]
    fn test_write_and_read_xmp_extended() {
        // Create extended WebP with VP8X
        let webp_data = create_extended_webp();
        let reader = Cursor::new(webp_data);
        let mut writer = Cursor::new(Vec::new());

        // Create XMP metadata
        let mut meta = XmpMeta::new();
        meta.set_property(
            ns::DC,
            "creator",
            XmpValue::String("Test Creator".to_string()),
        )
        .unwrap();

        // Write XMP
        WebpHandler::write_xmp(reader, &mut writer, &meta).unwrap();

        // Read back XMP
        writer.set_position(0);
        let result = WebpHandler::read_xmp(writer).unwrap();
        assert!(result.is_some());

        let read_meta = result.unwrap();
        let creator_value = read_meta.get_property(ns::DC, "creator");
        assert!(creator_value.is_some());
    }

    #[test]
    fn test_format_info() {
        let handler = WebpHandler;
        assert_eq!(handler.format_name(), "WebP");
        assert_eq!(handler.extensions(), &["webp"]);
    }
}
