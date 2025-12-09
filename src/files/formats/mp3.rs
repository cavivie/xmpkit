//! MP3 file format handler
//!
//! This module provides functionality for reading and writing XMP metadata
//! in MP3 files. The implementation is pure Rust and cross-platform compatible.
//!
//! MP3 XMP Storage:
//! - XMP Packet is stored in ID3v2 PRIV frame (ID3v2.3/2.4) or PRV frame (ID3v2.2)
//! - Frame content format: "XMP\0" + XMP Packet
//! - ID3v2 tag header is 10 bytes at the start of the file

use crate::core::error::{XmpError, XmpResult};
use crate::core::metadata::XmpMeta;
use crate::files::handler::{FileHandler, XmpOptions};
use std::io::{Read, Seek, SeekFrom, Write};

/// ID3v2 tag header size (same for v2.2, v2.3, v2.4)
const ID3_TAG_HEADER_SIZE: usize = 10;

/// ID3v2.2 frame header size
const ID3V22_FRAME_HEADER_SIZE: usize = 6;

/// ID3v2.3/2.4 frame header size
const ID3V23_FRAME_HEADER_SIZE: usize = 10;

/// XMP frame ID for ID3v2.3/2.4 (PRIV)
const XMP_V23_ID: &[u8] = b"PRIV";

/// XMP frame ID for ID3v2.2 (PRV)
const XMP_V22_ID: &[u8] = b"PRV\0";

/// XMP frame content prefix
const XMP_PREFIX: &[u8] = b"XMP\0";

/// MP3 file handler for XMP metadata
#[derive(Debug, Clone, Copy)]
pub struct Mp3Handler;

impl FileHandler for Mp3Handler {
    fn can_handle<R: Read + Seek>(&self, reader: &mut R) -> XmpResult<bool> {
        let mut header = [0u8; 3];
        reader.read_exact(&mut header)?;
        reader.rewind()?;
        Ok(header == *b"ID3")
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
        "MP3"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["mp3"]
    }
}

impl Mp3Handler {
    /// Read XMP metadata from an MP3 file
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
        // Check ID3v2 tag header
        let mut header = [0u8; ID3_TAG_HEADER_SIZE];
        reader.read_exact(&mut header)?;

        if &header[0..3] != b"ID3" {
            return Ok(None); // No ID3v2 tag
        }

        // Parse version
        let major_version = header[3];
        let minor_version = header[4];
        let flags = header[5];

        // Validate version (support v2.2, v2.3, v2.4)
        if !(2..=4).contains(&major_version) || minor_version == 0xFF {
            return Err(XmpError::BadValue(format!(
                "Unsupported ID3v2 version: {}.{}",
                major_version, minor_version
            )));
        }

        // Check flags
        if (flags & 0x10) != 0 {
            return Err(XmpError::NotSupported(
                "ID3v2 footer not supported".to_string(),
            ));
        }
        if (flags & 0x80) != 0 {
            return Err(XmpError::NotSupported(
                "Unsynchronized ID3v2 tags not supported".to_string(),
            ));
        }

        // Read tag size (synchsafe integer, big-endian)
        let tag_size = Self::read_synchsafe_u32(&header[6..10])?;

        // Skip extended header if present
        if (flags & 0x40) != 0 {
            let ext_header_size = Self::read_synchsafe_u32_from_reader(&mut reader)?;
            let skip_size = if major_version < 4 {
                ext_header_size - 4 // v2.3 doesn't include size in the size field
            } else {
                ext_header_size
            };
            reader.seek(SeekFrom::Current(skip_size as i64 - 4))?;
        }

        // Determine frame header size and XMP frame ID
        let frame_header_size = if major_version == 2 {
            ID3V22_FRAME_HEADER_SIZE
        } else {
            ID3V23_FRAME_HEADER_SIZE
        };
        let xmp_frame_id = if major_version == 2 {
            XMP_V22_ID
        } else {
            XMP_V23_ID
        };

        // Read frames until we find XMP frame or reach end of tag
        let tag_start = reader.stream_position()?;
        let tag_end = tag_start + tag_size as u64;

        while reader.stream_position()? < tag_end {
            let current_pos = reader.stream_position()?;
            if tag_end - current_pos < frame_header_size as u64 {
                break; // Not enough space for another frame
            }

            // Read frame header
            let mut frame_header = vec![0u8; frame_header_size];
            reader.read_exact(&mut frame_header)?;

            // Check if this is a padding frame (all zeros)
            if frame_header.iter().all(|&b| b == 0) {
                break;
            }

            // Parse frame ID and size
            let (frame_id, frame_size) = Self::parse_frame_header(&frame_header, major_version)?;

            // Check if this is the XMP frame
            if frame_id == xmp_frame_id {
                if let Some(meta) = Self::read_xmp_frame_content(&mut reader, frame_size)? {
                    return Ok(Some(meta));
                }
            } else {
                // Skip this frame
                reader.seek(SeekFrom::Current(frame_size as i64))?;
            }
        }

        Ok(None)
    }

    /// Write XMP metadata to an MP3 file
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
    pub fn write_xmp<R: Read + Seek, W: Write + Seek>(
        mut reader: R,
        writer: &mut W,
        meta: &XmpMeta,
    ) -> XmpResult<()> {
        // Serialize XMP Packet
        let xmp_packet = meta.serialize_packet()?;
        let xmp_bytes = xmp_packet.as_bytes();

        // Create XMP frame content: "XMP\0" + XMP Packet
        let mut frame_content = Vec::with_capacity(4 + xmp_bytes.len());
        frame_content.extend_from_slice(XMP_PREFIX);
        frame_content.extend_from_slice(xmp_bytes);

        // Read existing ID3v2 tag header
        let mut header = [0u8; ID3_TAG_HEADER_SIZE];
        reader.read_exact(&mut header)?;

        if &header[0..3] != b"ID3" {
            // No existing ID3v2 tag, create a new one
            return Self::write_new_id3v2_tag(writer, &frame_content);
        }

        // Parse existing tag
        let major_version = header[3];
        let flags = header[5];
        let tag_size = Self::read_synchsafe_u32(&header[6..10])?;

        // Determine frame header size and XMP frame ID
        let frame_header_size = if major_version == 2 {
            ID3V22_FRAME_HEADER_SIZE
        } else {
            ID3V23_FRAME_HEADER_SIZE
        };
        let xmp_frame_id = if major_version == 2 {
            XMP_V22_ID
        } else {
            XMP_V23_ID
        };

        // Save header position to update tag size later
        let header_pos = writer.stream_position()?;

        // Copy tag header (will update size later)
        writer.write_all(&header)?;

        // Skip extended header if present
        if (flags & 0x40) != 0 {
            let ext_header_size = Self::read_synchsafe_u32_from_reader(&mut reader)?;
            let skip_size = if major_version < 4 {
                ext_header_size - 4
            } else {
                ext_header_size
            };
            let mut ext_header = vec![0u8; skip_size as usize - 4];
            reader.read_exact(&mut ext_header)?;
            writer.write_all(&ext_header)?;
        }

        // Read and process frames
        let tag_start = reader.stream_position()?;
        let tag_end = tag_start + tag_size as u64;
        let mut other_frames = Vec::new();

        while reader.stream_position()? < tag_end {
            let current_pos = reader.stream_position()?;
            if tag_end - current_pos < frame_header_size as u64 {
                break;
            }

            // Read frame header
            let mut frame_header = vec![0u8; frame_header_size];
            reader.read_exact(&mut frame_header)?;

            // Check for padding
            if frame_header.iter().all(|&b| b == 0) {
                break;
            }

            // Parse frame ID and size
            let (frame_id, frame_size) = Self::parse_frame_header(&frame_header, major_version)?;

            // Check if this is the XMP frame
            if frame_id == xmp_frame_id {
                // Skip old XMP frame
                reader.seek(SeekFrom::Current(frame_size as i64))?;
            } else {
                // Copy other frames
                let mut frame_content = vec![0u8; frame_size as usize];
                reader.read_exact(&mut frame_content)?;
                other_frames.push((frame_header, frame_content));
            }
        }

        // Calculate new tag size
        let mut new_tag_size = 0u32;
        for (frame_header, frame_content) in &other_frames {
            new_tag_size += frame_header.len() as u32 + frame_content.len() as u32;
        }
        // Add XMP frame size
        let xmp_frame_size = frame_header_size as u32 + frame_content.len() as u32;
        new_tag_size += xmp_frame_size;

        // Write all other frames
        for (frame_header, frame_content) in &other_frames {
            writer.write_all(frame_header)?;
            writer.write_all(frame_content)?;
        }

        // Write XMP frame
        Self::write_xmp_frame(writer, major_version, &frame_content)?;

        // Update tag size in header
        let current_pos = writer.stream_position()?;
        writer.seek(SeekFrom::Start(header_pos))?;
        writer.write_all(&header[0..6])?; // Write ID3 + version + flags
        Self::write_synchsafe_u32(&mut header[6..10], new_tag_size)?;
        writer.write_all(&header[6..10])?; // Write updated size
        writer.seek(SeekFrom::Start(current_pos))?;

        // Copy rest of file
        reader.seek(SeekFrom::Start(tag_start + tag_size as u64))?;
        std::io::copy(&mut reader, writer)?;

        Ok(())
    }

    /// Write a new ID3v2 tag with XMP frame
    fn write_new_id3v2_tag<W: Write + Seek>(writer: &mut W, frame_content: &[u8]) -> XmpResult<()> {
        // Create ID3v2.3 header (most compatible)
        let mut header = [0u8; ID3_TAG_HEADER_SIZE];
        header[0..3].copy_from_slice(b"ID3");
        header[3] = 3; // Major version 3
        header[4] = 0; // Minor version 0
        header[5] = 0; // Flags

        // Calculate tag size (frame size + frame header)
        let frame_size = frame_content.len() as u32;
        let tag_size = ID3V23_FRAME_HEADER_SIZE as u32 + frame_size;

        // Write synchsafe size
        Self::write_synchsafe_u32(&mut header[6..10], tag_size)?;

        writer.write_all(&header)?;

        // Write XMP frame
        Self::write_xmp_frame(writer, 3, frame_content)?;

        Ok(())
    }

    /// Write an XMP frame
    fn write_xmp_frame<W: Write + Seek>(
        writer: &mut W,
        major_version: u8,
        frame_content: &[u8],
    ) -> XmpResult<()> {
        let frame_header_size = if major_version == 2 {
            ID3V22_FRAME_HEADER_SIZE
        } else {
            ID3V23_FRAME_HEADER_SIZE
        };
        let xmp_frame_id = if major_version == 2 {
            XMP_V22_ID
        } else {
            XMP_V23_ID
        };

        let mut frame_header = vec![0u8; frame_header_size];
        frame_header[0..xmp_frame_id.len()].copy_from_slice(xmp_frame_id);

        let frame_size = frame_content.len() as u32;

        // Write frame size
        if major_version == 2 {
            // v2.2: 3 bytes, big-endian
            frame_header[3] = ((frame_size >> 16) & 0xFF) as u8;
            frame_header[4] = ((frame_size >> 8) & 0xFF) as u8;
            frame_header[5] = (frame_size & 0xFF) as u8;
        } else if major_version == 4 {
            // v2.4: synchsafe integer
            Self::write_synchsafe_u32(&mut frame_header[4..8], frame_size)?;
        } else {
            // v2.3: 4 bytes, big-endian
            frame_header[4] = ((frame_size >> 24) & 0xFF) as u8;
            frame_header[5] = ((frame_size >> 16) & 0xFF) as u8;
            frame_header[6] = ((frame_size >> 8) & 0xFF) as u8;
            frame_header[7] = (frame_size & 0xFF) as u8;
        }

        writer.write_all(&frame_header)?;
        writer.write_all(frame_content)?;

        Ok(())
    }

    /// Read a synchsafe 32-bit integer from bytes (big-endian)
    fn read_synchsafe_u32(bytes: &[u8]) -> XmpResult<u32> {
        if bytes.len() < 4 {
            return Err(XmpError::BadValue(
                "Not enough bytes for synchsafe integer".to_string(),
            ));
        }

        let raw = u32::from(bytes[0]) << 24
            | u32::from(bytes[1]) << 16
            | u32::from(bytes[2]) << 8
            | u32::from(bytes[3]);

        // Check that it's synchsafe (no bit 7 set in any byte)
        if (raw & 0x80808080) != 0 {
            return Err(XmpError::BadValue("Invalid synchsafe integer".to_string()));
        }

        // Decode synchsafe integer
        Ok((raw & 0x7F)
            | ((raw >> 1) & 0x3F80)
            | ((raw >> 2) & 0x1FC000)
            | ((raw >> 3) & 0x0FE00000))
    }

    /// Read a synchsafe 32-bit integer from reader
    fn read_synchsafe_u32_from_reader<R: Read>(reader: &mut R) -> XmpResult<u32> {
        let mut bytes = [0u8; 4];
        reader.read_exact(&mut bytes)?;
        Self::read_synchsafe_u32(&bytes)
    }

    /// Parse frame header to extract frame ID and size
    fn parse_frame_header(frame_header: &[u8], major_version: u8) -> XmpResult<(&[u8], u32)> {
        let frame_id = if major_version == 2 {
            &frame_header[0..3]
        } else {
            &frame_header[0..4]
        };

        let frame_size = if major_version == 2 {
            // v2.2: 3 bytes, big-endian
            u32::from(frame_header[3]) << 16
                | u32::from(frame_header[4]) << 8
                | u32::from(frame_header[5])
        } else if major_version == 4 {
            // v2.4: synchsafe integer
            Self::read_synchsafe_u32(&frame_header[4..8])?
        } else {
            // v2.3: 4 bytes, big-endian
            u32::from(frame_header[4]) << 24
                | u32::from(frame_header[5]) << 16
                | u32::from(frame_header[6]) << 8
                | u32::from(frame_header[7])
        };

        Ok((frame_id, frame_size))
    }

    /// Read XMP frame content and parse it
    fn read_xmp_frame_content<R: Read + Seek>(
        reader: &mut R,
        frame_size: u32,
    ) -> XmpResult<Option<XmpMeta>> {
        // Read frame content
        let mut frame_content = vec![0u8; frame_size as usize];
        reader.read_exact(&mut frame_content)?;

        // Check for XMP prefix
        if frame_content.len() < 4 || &frame_content[0..4] != b"XMP\0" {
            return Ok(None);
        }

        // Extract XMP Packet
        let xmp_packet = &frame_content[4..];
        let xmp_str = String::from_utf8(xmp_packet.to_vec())
            .map_err(|e| XmpError::ParseError(format!("Invalid UTF-8 in XMP: {}", e)))?;

        // Parse XMP Packet
        Ok(Some(XmpMeta::parse(&xmp_str)?))
    }

    /// Write a synchsafe 32-bit integer to bytes (big-endian)
    fn write_synchsafe_u32(bytes: &mut [u8], value: u32) -> XmpResult<()> {
        if bytes.len() < 4 {
            return Err(XmpError::BadValue(
                "Not enough bytes for synchsafe integer".to_string(),
            ));
        }

        if value > 0x0FFFFFFF {
            return Err(XmpError::BadValue(
                "Value too large for synchsafe integer".to_string(),
            ));
        }

        // Encode synchsafe integer
        let encoded = (value & 0x7F)
            | ((value & 0x3F80) << 1)
            | ((value & 0x1FC000) << 2)
            | ((value & 0x0FE00000) << 3);

        bytes[0] = ((encoded >> 24) & 0xFF) as u8;
        bytes[1] = ((encoded >> 16) & 0xFF) as u8;
        bytes[2] = ((encoded >> 8) & 0xFF) as u8;
        bytes[3] = (encoded & 0xFF) as u8;

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

    // Minimal valid MP3 file with ID3v2 header but no XMP
    fn create_minimal_mp3() -> Vec<u8> {
        let mut mp3 = Vec::new();
        // ID3v2.3 header: "ID3" + version (03 00) + flags (00) + size (00 00 00 00)
        mp3.extend_from_slice(b"ID3");
        mp3.extend_from_slice(&[0x03, 0x00]); // version 2.3
        mp3.push(0x00); // flags
        mp3.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // size (synchsafe, 0 = no frames)
        mp3
    }

    #[test]
    fn test_read_xmp_no_xmp() {
        let mp3_data = create_minimal_mp3();
        let reader = Cursor::new(mp3_data);
        let result = Mp3Handler::read_xmp(reader).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_invalid_mp3() {
        // Test with data that's too short to read ID3v2 header (10 bytes)
        let invalid_data = vec![0x00, 0x01, 0x02, 0x03];
        let reader = Cursor::new(invalid_data);
        let result = Mp3Handler::read_xmp(reader);
        // MP3 handler returns error when data is too short to read header
        assert!(result.is_err());

        // Test with data that has enough bytes but no ID3 tag
        let no_id3_data = vec![0x00; 10];
        let reader2 = Cursor::new(no_id3_data);
        let result2 = Mp3Handler::read_xmp(reader2);
        // MP3 handler returns Ok(None) for files without ID3 tag
        assert!(result2.is_ok());
        assert!(result2.unwrap().is_none());
    }

    #[test]
    fn test_write_xmp() {
        // Create minimal MP3 (with empty ID3v2 tag)
        let mp3_data = create_minimal_mp3();
        let reader = Cursor::new(mp3_data.clone());
        let mut writer = Cursor::new(Vec::new());

        // Create XMP metadata
        let mut meta = XmpMeta::new();
        meta.set_property(ns::DC, "title", XmpValue::String("Test Audio".to_string()))
            .unwrap();

        // Write XMP
        Mp3Handler::write_xmp(reader, &mut writer, &meta).unwrap();

        // Read back XMP
        writer.set_position(0);
        let result = Mp3Handler::read_xmp(writer).unwrap();
        assert!(result.is_some(), "XMP should be readable after write");

        let read_meta = result.unwrap();
        let title_value = read_meta.get_property(ns::DC, "title");
        assert!(title_value.is_some());
        if let Some(XmpValue::String(title)) = title_value {
            assert_eq!(title, "Test Audio");
        } else {
            panic!("Expected string value");
        }
    }

    #[test]
    fn test_synchsafe_u32() {
        // Test synchsafe encoding/decoding
        let test_values = vec![0, 1, 255, 256, 1000, 1000000];

        for value in test_values {
            let mut bytes = [0u8; 4];
            Mp3Handler::write_synchsafe_u32(&mut bytes, value).unwrap();
            let decoded = Mp3Handler::read_synchsafe_u32(&bytes).unwrap();
            assert_eq!(value, decoded);
        }
    }
}
