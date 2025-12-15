//! ISO Base Media File Format (BMFF) support
//!
//! This module provides common utilities for handling BMFF-based file formats:
//! - MPEG-4/QuickTime family: MP4, MOV, M4A, M4V
//! - HEIF family: HEIC, AVIF
//!
//! BMFF Structure:
//! - Files are composed of "boxes" (also called "atoms" in QuickTime)
//! - Each box has: 4-byte size, 4-byte type, optional extended size, data
//! - All multi-byte integers are big-endian

use crate::core::error::XmpResult;
use std::io::{Read, Seek, SeekFrom};

// MPEG-4 / QuickTime family
#[cfg(feature = "mpeg4")]
pub mod mpeg4;

// MPEG-H family handlers (HEIF, AVIF)
#[cfg(feature = "mpegh")]
pub mod mpegh;

// Re-export handlers
#[cfg(feature = "mpeg4")]
pub use mpeg4::Mpeg4Handler;
#[cfg(feature = "mpegh")]
pub use mpegh::MpeghHandler;

// ============================================================================
// Constants
// ============================================================================

/// ftyp box type (file type box)
pub const FTYP_BOX: &[u8; 4] = b"ftyp";

/// UUID box type
pub const UUID_BOX: &[u8; 4] = b"uuid";

/// XMP UUID for BMFF-based formats (MPEG-4, QuickTime, HEIF)
/// UUID: BE7ACFCB-97A9-42E8-9C71-999491E3AFAC (from ISOBaseMedia_Support.hpp k_xmpUUID)
pub const XMP_UUID: &[u8] = &[
    0xBE, 0x7A, 0xCF, 0xCB, 0x97, 0xA9, 0x42, 0xE8, 0x9C, 0x71, 0x99, 0x94, 0x91, 0xE3, 0xAF, 0xAC,
];

// ============================================================================
// Types
// ============================================================================

/// BMFF box information
#[derive(Debug, Clone)]
pub struct BmffBox {
    /// Box size (including header)
    pub size: u64,
    /// Box type (4-byte FourCC)
    pub box_type: [u8; 4],
    /// Offset where box data starts (after header)
    pub data_offset: u64,
    /// Offset where box header starts
    pub header_offset: u64,
}

impl BmffBox {
    /// Get the size of the box header (8 or 16 bytes for extended size)
    pub fn header_size(&self) -> u64 {
        self.data_offset - self.header_offset
    }

    /// Get the size of the box data (excluding header)
    pub fn data_size(&self) -> u64 {
        self.size - self.header_size()
    }
}

// ============================================================================
// Reading Functions
// ============================================================================

/// Check if this is a valid BMFF file
pub fn is_bmff<R: Read + Seek>(reader: &mut R) -> XmpResult<bool> {
    let pos = reader.stream_position()?;

    // Check minimum file length
    let file_len = reader.seek(SeekFrom::End(0))?;
    reader.seek(SeekFrom::Start(pos))?;
    if file_len < 8 {
        return Ok(false);
    }

    // Read first box header
    let mut header = [0u8; 8];
    if reader.read_exact(&mut header).is_err() {
        reader.seek(SeekFrom::Start(pos))?;
        return Ok(false);
    }
    reader.seek(SeekFrom::Start(pos))?;

    let box_size = u32::from_be_bytes([header[0], header[1], header[2], header[3]]);
    let box_type = &header[4..8];

    // Box size must be at least 8 (header size)
    // Special case: size 0 means "extends to EOF", size 1 means 64-bit extended size
    if box_size != 0 && box_size != 1 && box_size < 8 {
        return Ok(false);
    }

    // Check for 'ftyp' box (ISO Base Media File Format)
    if box_type == FTYP_BOX {
        return Ok(true);
    }

    // Also accept QuickTime files that may start with other boxes
    let qt_boxes: &[&[u8; 4]] = &[b"moov", b"mdat", b"wide", b"free", b"skip", b"pnot"];
    for qt_box in qt_boxes {
        if box_type == *qt_box {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Read a box header at the current position
pub fn read_box<R: Read + Seek>(reader: &mut R) -> std::io::Result<BmffBox> {
    let header_offset = reader.stream_position()?;

    // Read box size (4 bytes, big-endian)
    let mut size_bytes = [0u8; 4];
    reader.read_exact(&mut size_bytes)?;
    let size = u32::from_be_bytes(size_bytes) as u64;

    // Read box type (4 bytes)
    let mut box_type = [0u8; 4];
    reader.read_exact(&mut box_type)?;

    // Handle extended size (size == 1 means extended size follows)
    let (actual_size, data_offset) = if size == 1 {
        let mut ext_size_bytes = [0u8; 8];
        reader.read_exact(&mut ext_size_bytes)?;
        (u64::from_be_bytes(ext_size_bytes), header_offset + 16)
    } else {
        (size, header_offset + 8)
    };

    Ok(BmffBox {
        size: actual_size,
        box_type,
        data_offset,
        header_offset,
    })
}

/// Skip to the next box (move past current box)
pub fn skip_box<R: Read + Seek>(reader: &mut R, box_info: &BmffBox) -> std::io::Result<()> {
    reader.seek(SeekFrom::Start(box_info.header_offset + box_info.size))?;
    Ok(())
}

/// Read box data
pub fn read_box_data<R: Read + Seek>(
    reader: &mut R,
    box_info: &BmffBox,
) -> std::io::Result<Vec<u8>> {
    reader.seek(SeekFrom::Start(box_info.data_offset))?;
    let mut data = vec![0u8; box_info.data_size() as usize];
    reader.read_exact(&mut data)?;
    Ok(data)
}

/// Copy bytes from reader to writer
pub fn copy_bytes<R: Read, W: std::io::Write>(
    reader: &mut R,
    writer: &mut W,
    count: u64,
) -> std::io::Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn create_minimal_bmff() -> Vec<u8> {
        let mut data = Vec::new();
        // ftyp box
        data.extend_from_slice(&20u32.to_be_bytes()); // size
        data.extend_from_slice(FTYP_BOX); // type
        data.extend_from_slice(b"isom"); // brand
        data.extend_from_slice(&0u32.to_be_bytes()); // version
        data.extend_from_slice(b"isom"); // compatible brand
        data
    }

    #[test]
    fn test_is_bmff() {
        let data = create_minimal_bmff();
        let mut reader = Cursor::new(data);
        assert!(is_bmff(&mut reader).unwrap());
    }

    #[test]
    fn test_is_bmff_invalid() {
        let data = vec![0x00, 0x01, 0x02, 0x03];
        let mut reader = Cursor::new(data);
        assert!(!is_bmff(&mut reader).unwrap());
    }

    #[test]
    fn test_read_box() {
        let data = create_minimal_bmff();
        let mut reader = Cursor::new(data);
        let box_info = read_box(&mut reader).unwrap();
        assert_eq!(box_info.size, 20);
        assert_eq!(&box_info.box_type, FTYP_BOX);
        assert_eq!(box_info.header_offset, 0);
        assert_eq!(box_info.data_offset, 8);
    }
}
