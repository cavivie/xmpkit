//! WebP file format handler
//!
//! WebP uses RIFF container format with form type "WEBP".
//! XMP is stored in a chunk with FourCC "XMP " (note the trailing space).
//!
//! Reference: RFC 9649 - WebP Image Format

use super::{
    chunk_total_size, copy_chunk, read_all_chunks, read_chunk_header, skip_chunk_data,
    validate_riff_header, write_chunk, write_riff_header, RiffChunk, CHUNK_HEADER_SIZE,
    RIFF_HEADER_SIZE,
};
use crate::core::error::{XmpError, XmpResult};
use crate::core::metadata::XmpMeta;
use crate::files::handler::{FileHandler, XmpOptions};
use std::io::{Read, Seek, SeekFrom, Write};

// ============================================================================
// Constants
// ============================================================================

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

/// VP8X flags bit for XMP metadata
const VP8X_XMP_FLAG: u8 = 0x04;

// ============================================================================
// Handler
// ============================================================================

/// WebP file handler for XMP metadata
#[derive(Debug, Clone, Copy, Default)]
pub struct WebpHandler;

impl FileHandler for WebpHandler {
    fn can_handle<R: Read + Seek>(&self, reader: &mut R) -> XmpResult<bool> {
        let pos = reader.stream_position()?;

        // Check minimum file length
        let file_len = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(pos))?;
        if file_len < 12 {
            return Ok(false);
        }

        // Validate RIFF header and check form type
        match validate_riff_header(reader) {
            Ok(form_type) => {
                reader.seek(SeekFrom::Start(pos))?;
                Ok(&form_type == WEBP_SIGNATURE)
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

impl WebpHandler {
    /// Read XMP metadata from a WebP file
    pub fn read_xmp<R: Read + Seek>(mut reader: R) -> XmpResult<Option<XmpMeta>> {
        // Validate WebP header
        let form_type = validate_riff_header(&mut reader)?;
        if &form_type != WEBP_SIGNATURE {
            return Err(XmpError::BadValue("Not a valid WebP file".to_string()));
        }

        // Search for XMP chunk
        reader.seek(SeekFrom::Start(RIFF_HEADER_SIZE))?;

        while let Ok(chunk) = read_chunk_header(&mut reader) {
            if chunk.id == *XMP_CHUNK_ID {
                // Found XMP chunk, read its data
                let mut xmp_data = vec![0u8; chunk.size as usize];
                reader.read_exact(&mut xmp_data)?;

                let xmp_str = String::from_utf8(xmp_data)
                    .map_err(|e| XmpError::ParseError(format!("Invalid UTF-8 in XMP: {}", e)))?;

                return XmpMeta::parse(&xmp_str).map(Some);
            }

            // Skip this chunk
            skip_chunk_data(&mut reader, chunk.size)?;
        }

        Ok(None)
    }

    /// Write XMP metadata to a WebP file
    pub fn write_xmp<R: Read + Seek, W: Write + Seek>(
        mut reader: R,
        mut writer: W,
        meta: &XmpMeta,
    ) -> XmpResult<()> {
        // Validate WebP header
        let form_type = validate_riff_header(&mut reader)?;
        if &form_type != WEBP_SIGNATURE {
            return Err(XmpError::BadValue("Not a valid WebP file".to_string()));
        }

        // Serialize XMP metadata
        let xmp_packet = meta.serialize_packet()?;
        let xmp_bytes = xmp_packet.as_bytes();

        // Read all chunks
        let chunks = read_all_chunks(&mut reader)?;

        // Find existing XMP chunk and VP8X chunk
        let xmp_chunk = chunks.iter().find(|c| c.id == *XMP_CHUNK_ID);
        let vp8x_chunk = chunks.iter().find(|c| c.id == *VP8X_CHUNK_ID);

        // Calculate new file size
        let old_xmp_size = xmp_chunk.map(|c| c.total_size()).unwrap_or(0);
        let new_xmp_size = chunk_total_size(xmp_bytes.len() as u32);

        // Read original RIFF header
        reader.seek(SeekFrom::Start(4))?;
        let mut old_file_size_bytes = [0u8; 4];
        reader.read_exact(&mut old_file_size_bytes)?;
        let old_file_size = u32::from_le_bytes(old_file_size_bytes);

        // Calculate new RIFF size
        let new_file_size = if xmp_chunk.is_some() {
            old_file_size - old_xmp_size as u32 + new_xmp_size as u32
        } else {
            let vp8x_addition = if vp8x_chunk.is_none() {
                chunk_total_size(10) as u32
            } else {
                0
            };
            old_file_size + new_xmp_size as u32 + vp8x_addition
        };

        // Write new RIFF header
        write_riff_header(&mut writer, new_file_size, WEBP_SIGNATURE)?;

        // Process chunks
        let needs_vp8x = vp8x_chunk.is_none();
        let mut xmp_written = false;
        let mut vp8x_written = false;

        for chunk in &chunks {
            if chunk.id == *XMP_CHUNK_ID {
                continue; // Skip old XMP chunk
            }

            if chunk.id == *VP8X_CHUNK_ID {
                // Update VP8X chunk with XMP flag
                reader.seek(SeekFrom::Start(chunk.offset + CHUNK_HEADER_SIZE))?;
                let mut vp8x_data = vec![0u8; chunk.size as usize];
                reader.read_exact(&mut vp8x_data)?;

                if !vp8x_data.is_empty() {
                    vp8x_data[0] |= VP8X_XMP_FLAG;
                }

                write_chunk(&mut writer, VP8X_CHUNK_ID, &vp8x_data)?;
                vp8x_written = true;

                // Write XMP chunk right after VP8X
                write_chunk(&mut writer, XMP_CHUNK_ID, xmp_bytes)?;
                xmp_written = true;
                continue;
            }

            // For VP8/VP8L (simple WebP), insert VP8X and XMP before it
            if needs_vp8x
                && !vp8x_written
                && (chunk.id == *VP8_CHUNK_ID || chunk.id == *VP8L_CHUNK_ID)
            {
                let (width, height) = Self::read_image_dimensions(&mut reader, chunk)?;
                Self::write_vp8x_chunk(&mut writer, width, height, VP8X_XMP_FLAG)?;
                vp8x_written = true;

                write_chunk(&mut writer, XMP_CHUNK_ID, xmp_bytes)?;
                xmp_written = true;
            }

            // Copy chunk as-is
            copy_chunk(&mut reader, &mut writer, chunk)?;
        }

        // If XMP wasn't written yet, append at end
        if !xmp_written {
            write_chunk(&mut writer, XMP_CHUNK_ID, xmp_bytes)?;
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
        let mut data = [0u8; 10];

        // Flags (1 byte) + Reserved (3 bytes)
        data[0] = flags;

        // Canvas width - 1 (3 bytes, little-endian)
        let w = width.saturating_sub(1) & 0xFFFFFF;
        data[4] = w as u8;
        data[5] = (w >> 8) as u8;
        data[6] = (w >> 16) as u8;

        // Canvas height - 1 (3 bytes, little-endian)
        let h = height.saturating_sub(1) & 0xFFFFFF;
        data[7] = h as u8;
        data[8] = (h >> 8) as u8;
        data[9] = (h >> 16) as u8;

        write_chunk(writer, VP8X_CHUNK_ID, &data)
    }

    /// Read image dimensions from VP8 or VP8L chunk
    fn read_image_dimensions<R: Read + Seek>(
        reader: &mut R,
        chunk: &RiffChunk,
    ) -> XmpResult<(u32, u32)> {
        reader.seek(SeekFrom::Start(chunk.offset + CHUNK_HEADER_SIZE))?;

        if chunk.id == *VP8_CHUNK_ID {
            let mut header = [0u8; 10];
            reader.read_exact(&mut header)?;

            if header[3] == 0x9D && header[4] == 0x01 && header[5] == 0x2A {
                let width = u16::from_le_bytes([header[6], header[7]]) & 0x3FFF;
                let height = u16::from_le_bytes([header[8], header[9]]) & 0x3FFF;
                return Ok((width as u32, height as u32));
            }
        } else if chunk.id == *VP8L_CHUNK_ID {
            let mut header = [0u8; 5];
            reader.read_exact(&mut header)?;

            if header[0] == 0x2F {
                let bits = u32::from_le_bytes([header[1], header[2], header[3], header[4]]);
                let width = (bits & 0x3FFF) + 1;
                let height = ((bits >> 14) & 0x3FFF) + 1;
                return Ok((width, height));
            }
        }

        Ok((1, 1)) // Fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::namespace::ns;
    use crate::types::value::XmpValue;
    use std::io::Cursor;

    fn create_minimal_webp() -> Vec<u8> {
        let mut webp = Vec::new();

        webp.extend_from_slice(b"RIFF");

        let vp8l_data: Vec<u8> = vec![
            0x2F, 0x00, 0x00, 0x00, 0x00, 0x10, 0x07, 0x10, 0x11, 0x11, 0x88, 0x88, 0x08, 0x08,
        ];

        let file_size = 4 + 8 + vp8l_data.len();
        webp.extend_from_slice(&(file_size as u32).to_le_bytes());
        webp.extend_from_slice(WEBP_SIGNATURE);
        webp.extend_from_slice(VP8L_CHUNK_ID);
        webp.extend_from_slice(&(vp8l_data.len() as u32).to_le_bytes());
        webp.extend_from_slice(&vp8l_data);

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
    fn test_write_and_read_xmp() {
        let webp_data = create_minimal_webp();
        let reader = Cursor::new(webp_data);
        let mut writer = Cursor::new(Vec::new());

        let mut meta = XmpMeta::new();
        meta.set_property(ns::DC, "title", XmpValue::String("Test WebP".to_string()))
            .unwrap();

        WebpHandler::write_xmp(reader, &mut writer, &meta).unwrap();

        writer.set_position(0);
        let result = WebpHandler::read_xmp(writer).unwrap();
        assert!(result.is_some());

        let read_meta = result.unwrap();
        let title = read_meta.get_property(ns::DC, "title");
        assert!(matches!(title, Some(XmpValue::String(s)) if s == "Test WebP"));
    }

    #[test]
    fn test_format_info() {
        let handler = WebpHandler;
        assert_eq!(handler.format_name(), "WebP");
        assert_eq!(handler.extensions(), &["webp"]);
    }
}
