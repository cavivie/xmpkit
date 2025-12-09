//! TIFF file format handler
//!
//! This module provides functionality for reading and writing XMP metadata
//! in TIFF files. The implementation is pure Rust and cross-platform compatible.
//!
//! TIFF XMP Storage:
//! - XMP Packet is stored in Tag 700 (kTIFF_XMP) in the Primary IFD (0th IFD)
//! - Tag type is typically BYTE (1) or UNDEFINED (7)
//! - Value is stored inline if <= 4 bytes, otherwise as an offset to the data

use crate::core::error::{XmpError, XmpResult};
use crate::core::metadata::XmpMeta;
use crate::files::handler::{FileHandler, XmpOptions};
use std::io::{Read, Seek, SeekFrom, Write};

/// TIFF file header signatures
const TIFF_SIGNATURE_LE: &[u8] = &[0x49, 0x49, 0x2A, 0x00]; // II/42 (little-endian)
const TIFF_SIGNATURE_BE: &[u8] = &[0x4D, 0x4D, 0x00, 0x2A]; // MM/42 (big-endian)

/// TIFF Tag IDs
const TAG_XMP: u16 = 700;

/// TIFF Data Types
const TYPE_BYTE: u16 = 1;
const TYPE_ASCII: u16 = 2;
const TYPE_UNDEFINED: u16 = 7;

/// Size of an IFD entry in bytes
const IFD_ENTRY_SIZE: usize = 12;

/// TIFF file handler for XMP metadata
#[derive(Debug, Clone, Copy)]
pub struct TiffHandler;

impl FileHandler for TiffHandler {
    fn can_handle<R: Read + Seek>(&self, reader: &mut R) -> XmpResult<bool> {
        let mut header = [0u8; 4];
        reader.read_exact(&mut header)?;
        reader.rewind()?;
        Ok(header[0..4] == *TIFF_SIGNATURE_LE || header[0..4] == *TIFF_SIGNATURE_BE)
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
        "TIFF"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["tif", "tiff"]
    }
}

/// Byte order for TIFF file
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ByteOrder {
    LittleEndian,
    BigEndian,
}

/// IFD Entry structure
struct IfdEntry {
    tag: u16,
    type_: u16,
    count: u32,
    value_or_offset: u32,
}

impl TiffHandler {
    /// Read XMP metadata from a TIFF file
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
        // Read and verify TIFF header
        let mut header = [0u8; 8];
        reader.read_exact(&mut header)?;

        let byte_order = if header[0..4] == *TIFF_SIGNATURE_LE {
            ByteOrder::LittleEndian
        } else if header[0..4] == *TIFF_SIGNATURE_BE {
            ByteOrder::BigEndian
        } else {
            return Err(XmpError::BadValue("Not a valid TIFF file".to_string()));
        };

        // Read first IFD offset (bytes 4-7)
        let first_ifd_offset = Self::read_u32(&header[4..8], byte_order)?;

        // Seek to first IFD
        reader.seek(SeekFrom::Start(first_ifd_offset as u64))?;

        // Read Primary IFD (0th IFD)
        let xmp_data = Self::read_ifd_for_xmp(&mut reader, byte_order)?;

        if xmp_data.is_empty() {
            return Ok(None);
        }

        // Parse XMP Packet
        let xmp_str = String::from_utf8(xmp_data)
            .map_err(|e| XmpError::ParseError(format!("Invalid UTF-8 in XMP: {}", e)))?;

        XmpMeta::parse(&xmp_str).map(Some)
    }

    /// Write XMP metadata to a TIFF file
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

        // Read TIFF header
        let mut header = [0u8; 8];
        reader.read_exact(&mut header)?;

        let byte_order = if header[0..4] == *TIFF_SIGNATURE_LE {
            ByteOrder::LittleEndian
        } else if header[0..4] == *TIFF_SIGNATURE_BE {
            ByteOrder::BigEndian
        } else {
            return Err(XmpError::BadValue("Not a valid TIFF file".to_string()));
        };

        // Write header
        writer.write_all(&header)?;

        // Read first IFD offset
        let first_ifd_offset = Self::read_u32(&header[4..8], byte_order)?;

        // Copy file up to first IFD
        reader.seek(SeekFrom::Start(8))?;
        let mut buffer = vec![0u8; (first_ifd_offset as usize).saturating_sub(8)];
        if !buffer.is_empty() {
            reader.read_exact(&mut buffer)?;
            writer.write_all(&buffer)?;
        }

        // Read and modify Primary IFD
        reader.seek(SeekFrom::Start(first_ifd_offset as u64))?;
        Self::write_ifd_with_xmp(&mut reader, &mut writer, byte_order, xmp_bytes)?;

        // Copy rest of file
        let current_pos = reader.stream_position()?;
        reader.seek(SeekFrom::End(0))?;
        let file_size = reader.stream_position()?;
        reader.seek(SeekFrom::Start(current_pos))?;

        let mut buffer = vec![0u8; 8192];
        let remaining = file_size - current_pos;
        let mut copied = 0u64;

        while copied < remaining {
            let to_read = ((remaining - copied) as usize).min(buffer.len());
            reader.read_exact(&mut buffer[..to_read])?;
            writer.write_all(&buffer[..to_read])?;
            copied += to_read as u64;
        }

        Ok(())
    }

    /// Read IFD and extract XMP tag (Tag 700)
    fn read_ifd_for_xmp<R: Read + Seek>(
        reader: &mut R,
        byte_order: ByteOrder,
    ) -> XmpResult<Vec<u8>> {
        // Read entry count
        let mut count_bytes = [0u8; 2];
        reader.read_exact(&mut count_bytes)?;
        let entry_count = Self::read_u16(&count_bytes, byte_order)?;

        // Read all entries
        for _ in 0..entry_count {
            let entry = Self::read_ifd_entry(reader, byte_order)?;

            if entry.tag == TAG_XMP {
                // Found XMP tag
                return Self::read_tag_value(reader, &entry, byte_order);
            }
        }

        // Read next IFD offset (we don't need it for XMP in Primary IFD)
        let mut _next_ifd = [0u8; 4];
        reader.read_exact(&mut _next_ifd)?;

        Ok(Vec::new())
    }

    /// Read an IFD entry
    fn read_ifd_entry<R: Read>(reader: &mut R, byte_order: ByteOrder) -> XmpResult<IfdEntry> {
        let mut entry_bytes = [0u8; IFD_ENTRY_SIZE];
        reader.read_exact(&mut entry_bytes)?;

        let tag = Self::read_u16(&entry_bytes[0..2], byte_order)?;
        let type_ = Self::read_u16(&entry_bytes[2..4], byte_order)?;
        let count = Self::read_u32(&entry_bytes[4..8], byte_order)?;
        let value_or_offset = Self::read_u32(&entry_bytes[8..12], byte_order)?;

        Ok(IfdEntry {
            tag,
            type_,
            count,
            value_or_offset,
        })
    }

    /// Read tag value (handles inline values and offsets)
    fn read_tag_value<R: Read + Seek>(
        reader: &mut R,
        entry: &IfdEntry,
        _byte_order: ByteOrder,
    ) -> XmpResult<Vec<u8>> {
        let type_size = Self::get_type_size(entry.type_)?;
        let data_size = (entry.count as usize)
            .checked_mul(type_size)
            .ok_or_else(|| XmpError::BadValue("Tag count overflow".to_string()))?;

        if data_size <= 4 {
            // Value is stored inline in value_or_offset field
            let mut data = vec![0u8; data_size];
            let value_bytes = entry.value_or_offset.to_ne_bytes();
            data.copy_from_slice(&value_bytes[..data_size]);
            Ok(data)
        } else {
            // Value is stored at offset
            let saved_pos = reader.stream_position()?;
            reader.seek(SeekFrom::Start(entry.value_or_offset as u64))?;

            let mut data = vec![0u8; data_size];
            reader.read_exact(&mut data)?;

            reader.seek(SeekFrom::Start(saved_pos))?;
            Ok(data)
        }
    }

    /// Write IFD with XMP tag
    fn write_ifd_with_xmp<R: Read + Seek, W: Write + Seek>(
        reader: &mut R,
        writer: &mut W,
        byte_order: ByteOrder,
        xmp_bytes: &[u8],
    ) -> XmpResult<()> {
        // Save IFD position (where we'll write the IFD later)
        let ifd_start = writer.stream_position()?;

        // Read entry count
        let mut count_bytes = [0u8; 2];
        reader.read_exact(&mut count_bytes)?;
        let entry_count = Self::read_u16(&count_bytes, byte_order)?;

        // Read all entries
        let mut entries = Vec::new();
        let mut xmp_found = false;
        let mut xmp_entry_index = None;

        for i in 0..entry_count {
            let entry = Self::read_ifd_entry(reader, byte_order)?;
            if entry.tag == TAG_XMP {
                xmp_found = true;
                xmp_entry_index = Some(i as usize);
            }
            entries.push(entry);
        }

        // Read next IFD offset
        let mut next_ifd_bytes = [0u8; 4];
        reader.read_exact(&mut next_ifd_bytes)?;
        let next_ifd_offset = Self::read_u32(&next_ifd_bytes, byte_order)?;

        // Calculate IFD size: 2 (entry count) + entries.len() * 12 + 4 (next IFD offset)
        // Note: We need to account for the new XMP entry if not found
        let entries_count = if xmp_found {
            entries.len()
        } else {
            entries.len() + 1
        };
        let ifd_size = 2 + (entries_count as u32) * 12 + 4;

        // Determine where to write XMP data
        let xmp_data_offset = if xmp_found {
            // Replace existing XMP - use same offset if possible
            entries[xmp_entry_index.unwrap()].value_or_offset
        } else {
            // Append new XMP entry after IFD
            (ifd_start + ifd_size as u64) as u32
        };

        // Update or add XMP entry
        if xmp_found {
            let entry = &mut entries[xmp_entry_index.unwrap()];
            entry.count = xmp_bytes.len() as u32;
            entry.type_ = TYPE_BYTE;
            if xmp_bytes.len() <= 4 {
                // Store inline
                let mut value_bytes = [0u8; 4];
                value_bytes[..xmp_bytes.len()].copy_from_slice(xmp_bytes);
                entry.value_or_offset = Self::read_u32(&value_bytes, byte_order)?;
            } else {
                entry.value_or_offset = xmp_data_offset;
            }
        } else {
            // Add new XMP entry
            let new_entry = IfdEntry {
                tag: TAG_XMP,
                type_: TYPE_BYTE,
                count: xmp_bytes.len() as u32,
                value_or_offset: if xmp_bytes.len() <= 4 {
                    let mut value_bytes = [0u8; 4];
                    value_bytes[..xmp_bytes.len()].copy_from_slice(xmp_bytes);
                    Self::read_u32(&value_bytes, byte_order)?
                } else {
                    xmp_data_offset
                },
            };
            entries.push(new_entry);
        }

        // Write updated IFD at the saved position first
        writer.seek(SeekFrom::Start(ifd_start))?;

        // Write entry count
        let count = entries.len() as u16;
        writer.write_all(&Self::write_u16(count, byte_order))?;

        // Write entries
        for entry in &entries {
            Self::write_ifd_entry(writer, entry, byte_order)?;
        }

        // Write next IFD offset
        writer.write_all(&Self::write_u32(next_ifd_offset, byte_order))?;

        // Write XMP data if needed (after IFD is written)
        if !xmp_found || xmp_bytes.len() > 4 {
            writer.seek(SeekFrom::Start(xmp_data_offset as u64))?;
            writer.write_all(xmp_bytes)?;
        }

        Ok(())
    }

    /// Write an IFD entry
    fn write_ifd_entry<W: Write>(
        writer: &mut W,
        entry: &IfdEntry,
        byte_order: ByteOrder,
    ) -> XmpResult<()> {
        writer.write_all(&Self::write_u16(entry.tag, byte_order))?;
        writer.write_all(&Self::write_u16(entry.type_, byte_order))?;
        writer.write_all(&Self::write_u32(entry.count, byte_order))?;
        writer.write_all(&Self::write_u32(entry.value_or_offset, byte_order))?;
        Ok(())
    }

    /// Get size of a TIFF data type
    fn get_type_size(type_: u16) -> XmpResult<usize> {
        match type_ {
            TYPE_BYTE | TYPE_ASCII | TYPE_UNDEFINED => Ok(1),
            3 | 8 => Ok(2),  // SHORT, SSHORT
            4 | 9 => Ok(4),  // LONG, SLONG
            5 | 10 => Ok(8), // RATIONAL, SRATIONAL
            11 => Ok(4),     // FLOAT
            12 => Ok(8),     // DOUBLE
            _ => Err(XmpError::BadValue(format!("Unknown TIFF type: {}", type_))),
        }
    }

    /// Read u16 with byte order
    fn read_u16(bytes: &[u8], byte_order: ByteOrder) -> XmpResult<u16> {
        if bytes.len() < 2 {
            return Err(XmpError::BadValue("Not enough bytes for u16".to_string()));
        }
        Ok(match byte_order {
            ByteOrder::LittleEndian => u16::from_le_bytes([bytes[0], bytes[1]]),
            ByteOrder::BigEndian => u16::from_be_bytes([bytes[0], bytes[1]]),
        })
    }

    /// Read u32 with byte order
    fn read_u32(bytes: &[u8], byte_order: ByteOrder) -> XmpResult<u32> {
        if bytes.len() < 4 {
            return Err(XmpError::BadValue("Not enough bytes for u32".to_string()));
        }
        Ok(match byte_order {
            ByteOrder::LittleEndian => u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            ByteOrder::BigEndian => u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        })
    }

    /// Write u16 with byte order
    fn write_u16(value: u16, byte_order: ByteOrder) -> [u8; 2] {
        match byte_order {
            ByteOrder::LittleEndian => value.to_le_bytes(),
            ByteOrder::BigEndian => value.to_be_bytes(),
        }
    }

    /// Write u32 with byte order
    fn write_u32(value: u32, byte_order: ByteOrder) -> [u8; 4] {
        match byte_order {
            ByteOrder::LittleEndian => value.to_le_bytes(),
            ByteOrder::BigEndian => value.to_be_bytes(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::metadata::XmpMeta;
    use crate::core::namespace::ns;
    use crate::types::value::XmpValue;
    use std::io::Cursor;

    // Minimal valid TIFF file (little-endian) with no XMP
    fn create_minimal_tiff_le() -> Vec<u8> {
        let mut tiff = Vec::new();
        // Header: II/42 + first IFD offset (8)
        tiff.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00, 0x08, 0x00, 0x00, 0x00]);
        // IFD: entry count (0)
        tiff.extend_from_slice(&[0x00, 0x00]);
        // Next IFD offset (0 = end)
        tiff.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        tiff
    }

    #[test]
    fn test_read_xmp_no_xmp() {
        let tiff_data = create_minimal_tiff_le();
        let reader = Cursor::new(tiff_data);
        let result = TiffHandler::read_xmp(reader).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_invalid_tiff() {
        let invalid_data = vec![0x00, 0x01, 0x02, 0x03];
        let reader = Cursor::new(invalid_data);
        let result = TiffHandler::read_xmp(reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_write_xmp() {
        // Create minimal TIFF with a dummy IFD entry to make write_xmp work
        let mut tiff = Vec::new();
        // Header: II/42 + first IFD offset (8)
        tiff.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00, 0x08, 0x00, 0x00, 0x00]);
        // IFD: entry count (1) - need at least one entry for write_xmp to work
        tiff.extend_from_slice(&[0x01, 0x00]);
        // Dummy entry: tag (256 = ImageWidth), type (3 = SHORT), count (1), value (100)
        tiff.extend_from_slice(&[0x00, 0x01]); // tag
        tiff.extend_from_slice(&[0x00, 0x03]); // type
        tiff.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // count
        tiff.extend_from_slice(&[0x64, 0x00, 0x00, 0x00]); // value (100 as u16 in little-endian)
                                                           // Next IFD offset (0 = end)
        tiff.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);

        let reader = Cursor::new(tiff);
        let mut writer = Cursor::new(Vec::new());

        // Create XMP metadata
        let mut meta = XmpMeta::new();
        meta.set_property(ns::DC, "title", XmpValue::String("Test Image".to_string()))
            .unwrap();

        // Write XMP
        TiffHandler::write_xmp(reader, &mut writer, &meta).unwrap();

        // Read back XMP
        writer.set_position(0);
        let result = TiffHandler::read_xmp(writer).unwrap();
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
}
