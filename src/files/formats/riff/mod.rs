//! RIFF (Resource Interchange File Format) support
//!
//! This module provides common utilities for handling RIFF-based file formats:
//! - WebP (image)
//! - WAV (audio)
//! - AVI (video)
//!
//! RIFF Structure:
//! ```text
//! RIFF <file_size> <form_type>
//!   <chunk_id> <chunk_size> <chunk_data> [padding]
//!   <chunk_id> <chunk_size> <chunk_data> [padding]
//!   ...
//! ```
//!
//! - All multi-byte integers are little-endian
//! - Chunk data is padded to even byte boundary
//! - file_size = total file size - 8 (excludes "RIFF" and size field)

use crate::core::error::{XmpError, XmpResult};
use std::io::{Read, Seek, SeekFrom, Write};

// Re-export handlers
#[cfg(feature = "webp")]
pub mod webp;

#[cfg(feature = "wav")]
pub mod wav;

#[cfg(feature = "avi")]
pub mod avi;

// ============================================================================
// Constants
// ============================================================================

/// RIFF file signature
pub const RIFF_SIGNATURE: &[u8; 4] = b"RIFF";

/// RIFF header size (RIFF + size + form_type)
pub const RIFF_HEADER_SIZE: u64 = 12;

/// Chunk header size (id + size)
pub const CHUNK_HEADER_SIZE: u64 = 8;

/// LIST chunk ID
pub const LIST_CHUNK_ID: &[u8; 4] = b"LIST";

/// INFO list type (used in WAV/AVI for metadata)
pub const INFO_LIST_TYPE: &[u8; 4] = b"INFO";

// ============================================================================
// Types
// ============================================================================

/// Information about a RIFF chunk
#[derive(Debug, Clone)]
pub struct RiffChunk {
    /// Chunk FourCC ID
    pub id: [u8; 4],
    /// Chunk data size (excluding header and padding)
    pub size: u32,
    /// Position of chunk header in file
    pub offset: u64,
}

impl RiffChunk {
    /// Calculate total chunk size including header and padding
    pub fn total_size(&self) -> u64 {
        chunk_total_size(self.size)
    }

    /// Get the data offset (after the header)
    pub fn data_offset(&self) -> u64 {
        self.offset + CHUNK_HEADER_SIZE
    }
}

/// Data for writing a chunk
#[derive(Debug, Clone)]
pub struct ChunkData<'a> {
    /// Chunk FourCC ID
    pub id: [u8; 4],
    /// Chunk data
    pub data: &'a [u8],
}

// ============================================================================
// Reading Functions
// ============================================================================

/// Validate RIFF file header and return the form type
///
/// Returns the 4-byte form type (e.g., "WEBP", "WAVE", "AVI ")
pub fn validate_riff_header<R: Read + Seek>(reader: &mut R) -> XmpResult<[u8; 4]> {
    reader.seek(SeekFrom::Start(0))?;
    let mut header = [0u8; 12];
    reader.read_exact(&mut header)?;

    if &header[0..4] != RIFF_SIGNATURE {
        return Err(XmpError::BadValue("Not a valid RIFF file".to_string()));
    }

    let mut form_type = [0u8; 4];
    form_type.copy_from_slice(&header[8..12]);
    Ok(form_type)
}

/// Read RIFF file header and return (file_size, form_type)
pub fn read_riff_header<R: Read + Seek>(reader: &mut R) -> XmpResult<(u32, [u8; 4])> {
    reader.seek(SeekFrom::Start(0))?;
    let mut header = [0u8; 12];
    reader.read_exact(&mut header)?;

    if &header[0..4] != RIFF_SIGNATURE {
        return Err(XmpError::BadValue("Not a valid RIFF file".to_string()));
    }

    let file_size = u32::from_le_bytes([header[4], header[5], header[6], header[7]]);
    let mut form_type = [0u8; 4];
    form_type.copy_from_slice(&header[8..12]);

    Ok((file_size, form_type))
}

/// Read a chunk header at the current position
pub fn read_chunk_header<R: Read + Seek>(reader: &mut R) -> XmpResult<RiffChunk> {
    let offset = reader.stream_position()?;

    let mut id = [0u8; 4];
    reader.read_exact(&mut id)?;

    let mut size_bytes = [0u8; 4];
    reader.read_exact(&mut size_bytes)?;
    let size = u32::from_le_bytes(size_bytes);

    Ok(RiffChunk { id, size, offset })
}

/// Read all chunks in the file (starting after RIFF header)
pub fn read_all_chunks<R: Read + Seek>(reader: &mut R) -> XmpResult<Vec<RiffChunk>> {
    reader.seek(SeekFrom::Start(RIFF_HEADER_SIZE))?;
    let mut chunks = Vec::new();

    while let Ok(chunk) = read_chunk_header(reader) {
        chunks.push(chunk.clone());
        skip_chunk_data(reader, chunk.size)?;
    }

    Ok(chunks)
}

/// Find a chunk by ID
pub fn find_chunk<'a>(chunks: &'a [RiffChunk], id: &[u8; 4]) -> Option<&'a RiffChunk> {
    chunks.iter().find(|c| &c.id == id)
}

/// Read chunk data
pub fn read_chunk_data<R: Read + Seek>(reader: &mut R, chunk: &RiffChunk) -> XmpResult<Vec<u8>> {
    reader.seek(SeekFrom::Start(chunk.data_offset()))?;
    let mut data = vec![0u8; chunk.size as usize];
    reader.read_exact(&mut data)?;
    Ok(data)
}

/// Skip chunk data (including padding byte if odd size)
pub fn skip_chunk_data<R: Read + Seek>(reader: &mut R, size: u32) -> XmpResult<()> {
    let padded_size = padded_size(size);
    reader.seek(SeekFrom::Current(padded_size as i64))?;
    Ok(())
}

// ============================================================================
// Writing Functions
// ============================================================================

/// Write RIFF file header
pub fn write_riff_header<W: Write>(
    writer: &mut W,
    file_size: u32,
    form_type: &[u8; 4],
) -> XmpResult<()> {
    writer.write_all(RIFF_SIGNATURE)?;
    writer.write_all(&file_size.to_le_bytes())?;
    writer.write_all(form_type)?;
    Ok(())
}

/// Write a chunk
pub fn write_chunk<W: Write>(writer: &mut W, id: &[u8; 4], data: &[u8]) -> XmpResult<()> {
    let size = data.len() as u32;

    writer.write_all(id)?;
    writer.write_all(&size.to_le_bytes())?;
    writer.write_all(data)?;

    // Add padding byte if odd size
    if size % 2 == 1 {
        writer.write_all(&[0])?;
    }

    Ok(())
}

/// Copy bytes from reader to writer
pub fn copy_bytes<R: Read, W: Write>(reader: &mut R, writer: &mut W, count: u64) -> XmpResult<()> {
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

/// Copy a chunk from reader to writer
pub fn copy_chunk<R: Read + Seek, W: Write>(
    reader: &mut R,
    writer: &mut W,
    chunk: &RiffChunk,
) -> XmpResult<()> {
    reader.seek(SeekFrom::Start(chunk.offset))?;
    copy_bytes(reader, writer, chunk.total_size())
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Calculate padded size (rounded up to even boundary)
pub fn padded_size(size: u32) -> u32 {
    if size % 2 == 1 {
        size + 1
    } else {
        size
    }
}

/// Calculate total chunk size including header and padding
pub fn chunk_total_size(data_size: u32) -> u64 {
    CHUNK_HEADER_SIZE + padded_size(data_size) as u64
}

// ============================================================================
// INFO Metadata Support (for WAV/AVI)
// ============================================================================

pub mod info {
    use super::*;
    use crate::core::metadata::XmpMeta;
    use crate::core::namespace::ns;

    /// INFO chunk IDs and their XMP mappings
    pub const INAM: &[u8; 4] = b"INAM"; // Title -> dc:title
    pub const IART: &[u8; 4] = b"IART"; // Artist -> dc:creator
    pub const ICRD: &[u8; 4] = b"ICRD"; // Date -> xmp:CreateDate
    pub const ICOP: &[u8; 4] = b"ICOP"; // Copyright -> dc:rights
    pub const ICMT: &[u8; 4] = b"ICMT"; // Comment -> dc:description
    pub const IGNR: &[u8; 4] = b"IGNR"; // Genre -> xmpDM:genre
    pub const ISFT: &[u8; 4] = b"ISFT"; // Software -> xmp:CreatorTool

    /// An INFO metadata item
    #[derive(Debug, Clone)]
    pub struct InfoItem {
        pub id: [u8; 4],
        pub value: String,
    }

    /// Read INFO list from a LIST chunk
    pub fn read_info_list<R: Read + Seek>(
        reader: &mut R,
        list_chunk: &RiffChunk,
    ) -> XmpResult<Vec<InfoItem>> {
        let mut items = Vec::new();

        // Seek to LIST chunk data (after header)
        reader.seek(SeekFrom::Start(list_chunk.data_offset()))?;

        // Read list type
        let mut list_type = [0u8; 4];
        reader.read_exact(&mut list_type)?;

        if &list_type != INFO_LIST_TYPE {
            return Ok(items); // Not an INFO list
        }

        // Read sub-chunks within LIST
        let list_end = list_chunk.data_offset() + list_chunk.size as u64;
        while reader.stream_position()? < list_end {
            match read_chunk_header(reader) {
                Ok(sub_chunk) => {
                    // Read null-terminated string
                    let mut data = vec![0u8; sub_chunk.size as usize];
                    reader.read_exact(&mut data)?;

                    // Remove null terminator if present
                    if let Some(pos) = data.iter().position(|&b| b == 0) {
                        data.truncate(pos);
                    }

                    if let Ok(value) = String::from_utf8(data) {
                        if !value.is_empty() {
                            items.push(InfoItem {
                                id: sub_chunk.id,
                                value,
                            });
                        }
                    }

                    // Skip padding
                    if sub_chunk.size % 2 == 1 {
                        reader.seek(SeekFrom::Current(1))?;
                    }
                }
                Err(_) => break,
            }
        }

        Ok(items)
    }

    /// Reconcile INFO metadata into XMP
    ///
    /// Only adds properties that don't already exist in XMP.
    pub fn reconcile_to_xmp(meta: &mut XmpMeta, items: &[InfoItem]) {
        for item in items {
            match &item.id {
                id if id == INAM => {
                    // Title -> dc:title (as lang alt)
                    if meta
                        .get_localized_text(ns::DC, "title", "", "x-default")
                        .is_none()
                    {
                        let _ =
                            meta.set_localized_text(ns::DC, "title", "", "x-default", &item.value);
                    }
                }
                id if id == IART => {
                    // Artist -> dc:creator (as array)
                    if meta.get_property(ns::DC, "creator").is_none() {
                        let _ = meta.set_property(
                            ns::DC,
                            "creator",
                            crate::types::value::XmpValue::Array(vec![
                                crate::types::value::XmpValue::String(item.value.clone()),
                            ]),
                        );
                    }
                }
                id if id == ICOP => {
                    // Copyright -> dc:rights (as lang alt)
                    if meta
                        .get_localized_text(ns::DC, "rights", "", "x-default")
                        .is_none()
                    {
                        let _ =
                            meta.set_localized_text(ns::DC, "rights", "", "x-default", &item.value);
                    }
                }
                id if id == ICMT => {
                    // Comment -> dc:description (as lang alt)
                    if meta
                        .get_localized_text(ns::DC, "description", "", "x-default")
                        .is_none()
                    {
                        let _ = meta.set_localized_text(
                            ns::DC,
                            "description",
                            "",
                            "x-default",
                            &item.value,
                        );
                    }
                }
                id if id == ISFT => {
                    // Software -> xmp:CreatorTool
                    if meta.get_property(ns::XMP, "CreatorTool").is_none() {
                        let _ = meta.set_property(
                            ns::XMP,
                            "CreatorTool",
                            crate::types::value::XmpValue::String(item.value.clone()),
                        );
                    }
                }
                _ => {} // Ignore other INFO chunks
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn create_minimal_riff(form_type: &[u8; 4]) -> Vec<u8> {
        let mut data = Vec::new();

        // RIFF header
        data.extend_from_slice(RIFF_SIGNATURE);
        data.extend_from_slice(&4u32.to_le_bytes()); // file size (just form type)
        data.extend_from_slice(form_type);

        data
    }

    #[test]
    fn test_read_riff_header() {
        let data = create_minimal_riff(b"TEST");
        let mut reader = Cursor::new(data);

        let (size, form_type) = read_riff_header(&mut reader).unwrap();
        assert_eq!(size, 4);
        assert_eq!(&form_type, b"TEST");
    }

    #[test]
    fn test_validate_riff_header() {
        let data = create_minimal_riff(b"WAVE");
        let mut reader = Cursor::new(data);

        let form_type = validate_riff_header(&mut reader).unwrap();
        assert_eq!(&form_type, b"WAVE");
    }

    #[test]
    fn test_invalid_riff() {
        let data = vec![0x00, 0x01, 0x02, 0x03];
        let mut reader = Cursor::new(data);

        assert!(validate_riff_header(&mut reader).is_err());
    }

    #[test]
    fn test_chunk_total_size() {
        assert_eq!(chunk_total_size(10), 18); // 8 header + 10 data
        assert_eq!(chunk_total_size(11), 20); // 8 header + 11 data + 1 padding
    }

    #[test]
    fn test_padded_size() {
        assert_eq!(padded_size(10), 10);
        assert_eq!(padded_size(11), 12);
    }
}
