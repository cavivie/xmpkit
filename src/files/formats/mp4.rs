//! MP4 file format handler
//!
//! This module provides functionality for reading and writing XMP metadata
//! in MP4 files. The implementation is pure Rust and cross-platform compatible.
//!
//! MP4 XMP Storage:
//! - XMP Packet is stored in a UUID box (user data box)
//! - UUID: BE7ACFCB-97A9-42E8-9C71-999FBE5EFFDB
//! - The XMP data is stored directly in the UUID box data

use crate::core::error::{XmpError, XmpResult};
use crate::core::metadata::XmpMeta;
use crate::files::handler::FileHandler;
use std::io::{Read, Seek, SeekFrom, Write};

/// MP4 file signature (ftyp box)
const MP4_SIGNATURE: &[u8] = b"ftyp";

/// XMP UUID for MP4 files
/// UUID: BE7ACFCB-97A9-42E8-9C71-999491E3AFAC (from ISOBaseMedia_Support.hpp k_xmpUUID)
const XMP_UUID: &[u8] = &[
    0xBE, 0x7A, 0xCF, 0xCB, 0x97, 0xA9, 0x42, 0xE8, 0x9C, 0x71, 0x99, 0x94, 0x91, 0xE3, 0xAF, 0xAC,
];

/// Box type for user data
const BOX_TYPE_UDTA: &[u8] = b"udta";
/// Box type for UUID
const BOX_TYPE_UUID: &[u8] = b"uuid";

/// MP4 file handler for XMP metadata
#[derive(Debug, Clone, Copy)]
pub struct Mp4Handler;

impl FileHandler for Mp4Handler {
    fn can_handle<R: Read + Seek>(&self, reader: &mut R) -> XmpResult<bool> {
        // MP4 file format: first 4 bytes are box size, next 4 bytes are box type "ftyp"
        let pos = reader.stream_position()?;

        // Read box size (4 bytes, big-endian)
        let mut size_bytes = [0u8; 4];
        match reader.read_exact(&mut size_bytes) {
            Ok(_) => {}
            Err(_) => {
                reader.seek(SeekFrom::Start(pos))?;
                return Ok(false);
            }
        }

        // Read box type (4 bytes)
        let mut box_type = [0u8; 4];
        match reader.read_exact(&mut box_type) {
            Ok(_) => {
                reader.seek(SeekFrom::Start(pos))?;
                Ok(box_type == *MP4_SIGNATURE)
            }
            Err(_) => {
                reader.seek(SeekFrom::Start(pos))?;
                Ok(false)
            }
        }
    }

    fn read_xmp<R: Read + Seek>(&self, reader: &mut R) -> XmpResult<Option<XmpMeta>> {
        Self::read_xmp(reader)
    }

    fn write_xmp<R: Read + Seek, W: Write + Seek>(
        &self,
        reader: &mut R,
        writer: &mut W,
        meta: &XmpMeta,
    ) -> XmpResult<()> {
        // Create a mutable reference that can be moved
        Self::write_xmp(reader, writer, meta)
    }

    fn format_name(&self) -> &'static str {
        "MP4"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["mp4", "m4a", "m4v"]
    }
}

#[derive(Debug)]
struct Mp4Box {
    size: u64,
    box_type: [u8; 4],
    #[allow(dead_code)]
    data_offset: u64,
}

impl Mp4Handler {
    /// Read XMP metadata from an MP4 file
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
        // Read ftyp box (first box in MP4 file)
        let ftyp_box = Self::read_box(&mut reader)?;
        if ftyp_box.box_type != *MP4_SIGNATURE {
            return Err(XmpError::BadValue("Not a valid MP4 file".to_string()));
        }

        // Skip ftyp box data (size includes header, so skip size - 8 bytes for header)
        let ftyp_data_size = ftyp_box.size - 8;
        reader.seek(SeekFrom::Current(ftyp_data_size as i64))?;

        // Search for top-level uuid box with XMP UUID first (ISO Base Media format)
        // Then search for moov/udta/XMP_ box (QuickTime format)
        loop {
            let box_start = reader.stream_position()?;
            let box_info = match Self::read_box(&mut reader) {
                Ok(b) => b,
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    return Ok(None);
                }
                Err(e) => return Err(e.into()),
            };

            // Check for top-level XMP UUID box (ISO Base Media format)
            if box_info.box_type == *BOX_TYPE_UUID {
                if let Some(xmp) = Self::read_xmp_from_uuid_box(&mut reader, &box_info)? {
                    return Ok(Some(xmp));
                }
            } else if box_info.box_type == *b"moov" {
                // Search inside moov for udta/XMP_ (QuickTime format)
                let moov_end = box_start + box_info.size;
                if let Some(xmp) = Self::search_udta_for_xmp(&mut reader, moov_end)? {
                    return Ok(Some(xmp));
                }
            } else {
                // Skip other boxes
                let remaining = box_info.size - 8;
                reader.seek(SeekFrom::Current(remaining as i64))?;
            }
        }
    }

    /// Search for udta box and XMP UUID box within a parent box
    fn search_udta_for_xmp<R: Read + Seek>(
        reader: &mut R,
        parent_end: u64,
    ) -> XmpResult<Option<XmpMeta>> {
        let start_pos = reader.stream_position()?;

        while reader.stream_position()? < parent_end {
            let box_start = reader.stream_position()?;
            let box_info = match Self::read_box(reader) {
                Ok(b) => b,
                Err(_) => break,
            };

            if box_info.box_type == *BOX_TYPE_UDTA {
                // Search inside udta for XMP UUID box
                // XMP UUID box can be:
                // 1. Direct child of udta (most common)
                // 2. Inside meta box (QuickTime format)
                let udta_end = box_start + box_info.size;

                // First, try to find UUID box directly in udta
                if let Some(xmp) = Self::search_uuid_for_xmp(reader, udta_end)? {
                    return Ok(Some(xmp));
                }

                // If not found, try searching in meta box
                reader.seek(SeekFrom::Start(box_start + 8))?; // Reset to start of udta content
                if let Some(xmp) = Self::search_meta_for_xmp(reader, udta_end)? {
                    return Ok(Some(xmp));
                }
            } else {
                // Skip this box
                reader.seek(SeekFrom::Start(box_start + box_info.size))?;
            }
        }

        reader.seek(SeekFrom::Start(start_pos))?;
        Ok(None)
    }

    /// Search for meta box and XMP UUID box within a parent box
    fn search_meta_for_xmp<R: Read + Seek>(
        reader: &mut R,
        parent_end: u64,
    ) -> XmpResult<Option<XmpMeta>> {
        let start_pos = reader.stream_position()?;

        while reader.stream_position()? < parent_end {
            let box_start = reader.stream_position()?;
            let box_info = match Self::read_box(reader) {
                Ok(b) => b,
                Err(_) => break,
            };

            if box_info.box_type == *b"meta" {
                // MP4 meta box: first 4 bytes after box header are version/flags (usually 0)
                // Skip version/flags and search for uuid box
                let version_flags_size = 4u64;
                reader.seek(SeekFrom::Current(version_flags_size as i64))?;

                let meta_end = box_start + box_info.size;
                if let Some(xmp) = Self::search_uuid_for_xmp(reader, meta_end)? {
                    return Ok(Some(xmp));
                }
            } else {
                // Skip this box
                reader.seek(SeekFrom::Start(box_start + box_info.size))?;
            }
        }

        reader.seek(SeekFrom::Start(start_pos))?;
        Ok(None)
    }

    /// Search for UUID box with XMP UUID
    fn search_uuid_for_xmp<R: Read + Seek>(
        reader: &mut R,
        parent_end: u64,
    ) -> XmpResult<Option<XmpMeta>> {
        let start_pos = reader.stream_position()?;

        while reader.stream_position()? < parent_end {
            let box_start = reader.stream_position()?;
            let box_info = match Self::read_box(reader) {
                Ok(b) => b,
                Err(_) => break,
            };

            if box_info.box_type == *BOX_TYPE_UUID {
                if let Some(xmp) = Self::read_xmp_from_uuid_box(reader, &box_info)? {
                    return Ok(Some(xmp));
                }
            } else {
                // Skip this box
                reader.seek(SeekFrom::Start(box_start + box_info.size))?;
            }
        }

        reader.seek(SeekFrom::Start(start_pos))?;
        Ok(None)
    }

    /// Read XMP from UUID box if it matches XMP UUID
    fn read_xmp_from_uuid_box<R: Read + Seek>(
        reader: &mut R,
        box_info: &Mp4Box,
    ) -> XmpResult<Option<XmpMeta>> {
        // Read UUID (16 bytes)
        let mut uuid = [0u8; 16];
        reader.read_exact(&mut uuid)?;

        if uuid != *XMP_UUID {
            // Skip this UUID box
            let remaining = box_info.size - 8 - 16;
            reader.seek(SeekFrom::Current(remaining as i64))?;
            return Ok(None);
        }

        // Found XMP UUID box
        let xmp_data_size = box_info.size - 8 - 16; // size - box header - UUID
        let mut xmp_data = vec![0u8; xmp_data_size as usize];
        reader.read_exact(&mut xmp_data)?;

        let xmp_str = String::from_utf8(xmp_data)
            .map_err(|e| XmpError::ParseError(format!("Invalid UTF-8: {}", e)))?;
        Ok(Some(XmpMeta::parse(&xmp_str)?))
    }

    /// Read an MP4 box header
    fn read_box<R: Read + Seek>(reader: &mut R) -> std::io::Result<Mp4Box> {
        let data_offset = reader.stream_position()?;

        // Read box size (4 bytes, big-endian)
        let mut size_bytes = [0u8; 4];
        reader.read_exact(&mut size_bytes)?;
        let size = u32::from_be_bytes(size_bytes) as u64;

        // Read box type (4 bytes)
        let mut box_type = [0u8; 4];
        reader.read_exact(&mut box_type)?;

        // Handle extended size (size == 1 means extended size follows)
        let actual_size = if size == 1 {
            let mut ext_size_bytes = [0u8; 8];
            reader.read_exact(&mut ext_size_bytes)?;
            u64::from_be_bytes(ext_size_bytes)
        } else {
            size
        };

        Ok(Mp4Box {
            size: actual_size,
            box_type,
            data_offset,
        })
    }

    /// Write XMP metadata to an MP4 file
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
    ///
    /// # Note
    ///
    /// This implementation currently has limitations:
    /// - When the moov box size changes, chunk offset tables (stco/co64) are not updated
    /// - This may cause media playback issues for some MP4 files
    /// - Full implementation requires updating all chunk offsets when moov size changes
    pub fn write_xmp<R: Read + Seek, W: Write + Seek>(
        mut reader: R,
        mut writer: W,
        meta: &XmpMeta,
    ) -> XmpResult<()> {
        // Serialize XMP Packet
        let xmp_packet = meta.serialize_packet()?;
        let xmp_bytes = xmp_packet.as_bytes();

        // Read ftyp box
        let ftyp_box = Self::read_box(&mut reader)?;
        if ftyp_box.box_type != *MP4_SIGNATURE {
            return Err(XmpError::BadValue("Not a valid MP4 file".to_string()));
        }

        // Determine file format: ISO Base Media or QuickTime
        // Read ftyp brand to determine format
        reader.seek(SeekFrom::Start(8))?; // Skip ftyp header
        let mut brand_bytes = [0u8; 4];
        reader.read_exact(&mut brand_bytes)?;
        let brand = u32::from_be_bytes(brand_bytes);

        // ISO Base Media brands: isom, iso2, mp41, mp42, avc1, f4v, 3gp4, 3g2a, 3g2b, 3g2c
        // QuickTime brand: qt
        let is_iso_base_media = brand == 0x69736F6D // isom
            || brand == 0x69736F32 // iso2
            || brand == 0x6D703431 // mp41
            || brand == 0x6D703432 // mp42
            || brand == 0x61766331 // avc1
            || brand == 0x66347620 // f4v
            || brand == 0x33677034 // 3gp4
            || brand == 0x33673261 // 3g2a
            || brand == 0x33673262 // 3g2b
            || brand == 0x33673263; // 3g2c

        // Copy ftyp box
        reader.seek(SeekFrom::Start(0))?;
        let mut ftyp_data = vec![0u8; ftyp_box.size as usize];
        reader.read_exact(&mut ftyp_data)?;
        writer.write_all(&ftyp_data)?;

        let mut xmp_written = false;
        let mut moov_found = false;
        let mut xmp_box_pos = None::<u64>; // For ISO Base Media: top-level UUID box position

        // Process boxes
        loop {
            let box_start = reader.stream_position()?;
            let box_info = match Self::read_box(&mut reader) {
                Ok(b) => b,
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e.into()),
            };

            // After read_box, reader is positioned after the box header
            // We need to seek back to box_start to copy the entire box
            reader.seek(SeekFrom::Start(box_start))?;

            if box_info.box_type == *b"moov" {
                moov_found = true;
                let old_moov_size = box_info.size;

                // Write moov box to a temporary buffer first
                // This allows us to update chunk offsets before writing to the final writer
                let mut moov_buffer = Vec::new();
                let mut moov_header = [0u8; 8];

                {
                    use std::io::Cursor;
                    let mut cursor = Cursor::new(&mut moov_buffer);

                    // Write moov box header placeholder (will update size later)
                    reader.seek(SeekFrom::Start(box_start))?;
                    reader.read_exact(&mut moov_header)?;
                    cursor.write_all(&moov_header)?;

                    // Check for extended size
                    let has_extended_size = box_info.size > u32::MAX as u64;
                    if has_extended_size {
                        let mut ext_size = vec![0u8; 8];
                        reader.read_exact(&mut ext_size)?;
                        cursor.write_all(&ext_size)?;
                    }

                    // Process moov children
                    let moov_end = box_start + box_info.size;
                    // For ISO Base Media format, don't write UUID box in moov/udta
                    // Instead, write it as top-level box after moov
                    let xmp_bytes_option = if is_iso_base_media {
                        None
                    } else {
                        Some(xmp_bytes)
                    };
                    Self::write_moov_with_xmp(
                        &mut reader,
                        &mut cursor,
                        moov_end,
                        xmp_bytes_option,
                        &mut xmp_written,
                    )?;
                }

                // Update moov box size in buffer
                let new_moov_size = moov_buffer.len() as u64;
                let moov_size_delta = new_moov_size as i64 - old_moov_size as i64;

                // Update moov box header size
                if new_moov_size <= u32::MAX as u64 {
                    moov_buffer[0..4].copy_from_slice(&(new_moov_size as u32).to_be_bytes());
                } else {
                    moov_buffer[0..4].copy_from_slice(&1u32.to_be_bytes()); // extended size marker
                    if moov_buffer.len() < 16 {
                        return Err(XmpError::BadValue("Invalid moov box structure".to_string()));
                    }
                    moov_buffer[8..16].copy_from_slice(&new_moov_size.to_be_bytes());
                }

                // For ISO Base Media format, we'll insert UUID box after moov
                // This will shift mdat box position, so we need to update chunk offsets
                let uuid_box_size = if is_iso_base_media && !xmp_written {
                    8 + 16 + xmp_bytes.len() as u64 // box header + UUID + XMP data
                } else {
                    0
                };

                // Update chunk offsets if moov size changed OR if UUID box will be inserted
                let total_offset_delta = moov_size_delta + uuid_box_size as i64;
                if total_offset_delta != 0 {
                    Self::update_chunk_offsets_in_buffer(&mut moov_buffer, total_offset_delta)?;
                }

                // Write the updated moov box buffer to the final writer
                writer.write_all(&moov_buffer)?;

                // For ISO Base Media format, write UUID box immediately after moov
                // (before any free boxes or mdat)
                if is_iso_base_media && !xmp_written {
                    Self::write_xmp_uuid_box(&mut writer, xmp_bytes)?;
                    xmp_written = true;
                }

                // Reader is already at box_start from above, now seek past the box
                reader.seek(SeekFrom::Start(box_start + box_info.size))?;
            } else if box_info.box_type == *BOX_TYPE_UUID && is_iso_base_media {
                // Check if this is an existing top-level XMP UUID box
                reader.seek(SeekFrom::Start(box_start + 8))?; // Skip box header
                let mut uuid = [0u8; 16];
                reader.read_exact(&mut uuid)?;

                if uuid == *XMP_UUID {
                    // Skip old XMP UUID box
                    let remaining = box_info.size - 8 - 16;
                    reader.seek(SeekFrom::Current(remaining as i64))?;

                    // Record position for writing new UUID box later
                    xmp_box_pos = Some(writer.stream_position()?);
                } else {
                    // Copy other UUID boxes
                    reader.seek(SeekFrom::Start(box_start))?;
                    let mut box_data = vec![0u8; box_info.size as usize];
                    reader.read_exact(&mut box_data)?;
                    writer.write_all(&box_data)?;
                }
            } else {
                // Copy other boxes as-is
                // Reader is already at box_start from above
                let mut box_data = vec![0u8; box_info.size as usize];
                reader.read_exact(&mut box_data)?;
                writer.write_all(&box_data)?;
            }
        }

        // Write XMP box based on file format
        // Note: For ISO Base Media format, UUID box is written immediately after moov box
        // (handled in the moov box processing above)
        if !xmp_written {
            if is_iso_base_media {
                // This should not happen if moov box was found
                // But handle the case where moov box doesn't exist
                if let Some(pos) = xmp_box_pos {
                    // Replace existing UUID box
                    writer.seek(SeekFrom::Start(pos))?;
                    Self::write_xmp_uuid_box(&mut writer, xmp_bytes)?;
                } else {
                    // No moov box found - write UUID box at current position
                    Self::write_xmp_uuid_box(&mut writer, xmp_bytes)?;
                }
            } else {
                // QuickTime format: should write moov/udta/XMP_ box
                // But we already handled this in write_moov_with_xmp
                if moov_found {
                    return Err(XmpError::NotSupported(
                        "Adding XMP to QuickTime files without existing udta box not yet implemented".to_string(),
                    ));
                }
            }
        }

        Ok(())
    }

    /// Write moov box with XMP UUID box
    /// For ISO Base Media format, xmp_bytes should be None (XMP goes in top-level UUID box)
    /// For QuickTime format, xmp_bytes should be Some (XMP goes in moov/udta/XMP_ box)
    fn write_moov_with_xmp<R: Read + Seek, W: Write + Seek>(
        reader: &mut R,
        writer: &mut W,
        moov_end: u64,
        xmp_bytes: Option<&[u8]>,
        xmp_written: &mut bool,
    ) -> XmpResult<()> {
        while reader.stream_position()? < moov_end {
            let box_start = reader.stream_position()?;
            let box_info = match Self::read_box(reader) {
                Ok(b) => b,
                Err(_) => break,
            };

            if box_info.box_type == *BOX_TYPE_UDTA {
                // Record udta box start position in writer
                let udta_writer_start = writer.stream_position()?;

                // Process udta box
                reader.seek(SeekFrom::Start(box_start))?;
                let mut udta_header = [0u8; 8];
                reader.read_exact(&mut udta_header)?;
                writer.write_all(&udta_header)?;

                // Check for extended size
                let has_extended_size = box_info.size > u32::MAX as u64;
                if has_extended_size {
                    let mut ext_size = vec![0u8; 8];
                    reader.read_exact(&mut ext_size)?;
                    writer.write_all(&ext_size)?;
                }

                let udta_end = box_start + box_info.size;
                if let Some(xmp_data) = xmp_bytes {
                    Self::write_udta_with_xmp(reader, writer, udta_end, xmp_data, xmp_written)?;
                } else {
                    // ISO Base Media format: just copy udta as-is (XMP goes in top-level UUID box)
                    reader.seek(SeekFrom::Start(box_start))?;
                    let mut box_data = vec![0u8; box_info.size as usize];
                    reader.read_exact(&mut box_data)?;
                    writer.write_all(&box_data)?;
                }

                // Update udta box size
                let udta_writer_end = writer.stream_position()?;
                let new_udta_size = udta_writer_end - udta_writer_start;
                writer.seek(SeekFrom::Start(udta_writer_start))?;
                if new_udta_size <= u32::MAX as u64 {
                    writer.write_all(&(new_udta_size as u32).to_be_bytes())?;
                    writer.write_all(&udta_header[4..8])?; // box type
                } else {
                    writer.write_all(&1u32.to_be_bytes())?; // extended size marker
                    writer.write_all(&udta_header[4..8])?; // box type
                    writer.write_all(&new_udta_size.to_be_bytes())?;
                }
                writer.seek(SeekFrom::Start(udta_writer_end))?;
            } else {
                // Copy other moov children
                reader.seek(SeekFrom::Start(box_start))?;
                let mut box_data = vec![0u8; box_info.size as usize];
                reader.read_exact(&mut box_data)?;
                writer.write_all(&box_data)?;
            }
        }

        Ok(())
    }

    /// Write udta box with XMP UUID box
    fn write_udta_with_xmp<R: Read + Seek, W: Write + Seek>(
        reader: &mut R,
        writer: &mut W,
        udta_end: u64,
        xmp_bytes: &[u8],
        xmp_written: &mut bool,
    ) -> XmpResult<()> {
        while reader.stream_position()? < udta_end {
            let box_start = reader.stream_position()?;
            let box_info = match Self::read_box(reader) {
                Ok(b) => b,
                Err(_) => break,
            };

            if box_info.box_type == *BOX_TYPE_UUID {
                // Check if it's XMP UUID
                let mut uuid = [0u8; 16];
                reader.read_exact(&mut uuid)?;

                if uuid == *XMP_UUID {
                    // Skip old XMP UUID box
                    let remaining = box_info.size - 8 - 16;
                    reader.seek(SeekFrom::Current(remaining as i64))?;

                    // Write new XMP UUID box
                    if !*xmp_written {
                        Self::write_xmp_uuid_box(writer, xmp_bytes)?;
                        *xmp_written = true;
                    }
                } else {
                    // Copy other UUID boxes
                    reader.seek(SeekFrom::Start(box_start))?;
                    let mut box_data = vec![0u8; box_info.size as usize];
                    reader.read_exact(&mut box_data)?;
                    writer.write_all(&box_data)?;
                }
            } else {
                // Copy other udta children
                reader.seek(SeekFrom::Start(box_start))?;
                let mut box_data = vec![0u8; box_info.size as usize];
                reader.read_exact(&mut box_data)?;
                writer.write_all(&box_data)?;
            }
        }

        // If XMP wasn't written yet, add it at the end of udta
        if !*xmp_written {
            Self::write_xmp_uuid_box(writer, xmp_bytes)?;
            *xmp_written = true;
        }

        Ok(())
    }

    /// Write XMP UUID box
    fn write_xmp_uuid_box<W: Write>(writer: &mut W, xmp_bytes: &[u8]) -> XmpResult<()> {
        // Box size: 8 (header) + 16 (UUID) + xmp_bytes.len()
        let box_size = 8 + 16 + xmp_bytes.len() as u64;

        // Write box size (4 bytes, big-endian)
        if box_size <= u32::MAX as u64 {
            writer.write_all(&(box_size as u32).to_be_bytes())?;
        } else {
            // Extended size
            writer.write_all(&1u32.to_be_bytes())?;
            writer.write_all(&box_size.to_be_bytes())?;
        }

        // Write box type (uuid)
        writer.write_all(BOX_TYPE_UUID)?;

        // Write UUID
        writer.write_all(XMP_UUID)?;

        // Write XMP data
        writer.write_all(xmp_bytes)?;

        Ok(())
    }

    /// Update chunk offsets in stco/co64 boxes when moov box size changes
    ///
    /// When moov box size changes, all chunk offsets that point to data after moov need to be adjusted
    fn update_chunk_offsets_in_buffer(buffer: &mut [u8], moov_size_delta: i64) -> XmpResult<()> {
        // Search for stco boxes (4-byte offsets)
        let mut offset = 0;
        while offset + 4 < buffer.len() {
            if &buffer[offset..offset + 4] == b"stco" {
                // Found stco box
                // stco format: size (4) + type (4) + version/flags (4) + entry_count (4) + offsets (4 bytes each)
                // But we need to find the actual box start (with size field)
                // Search backwards for the size field (4 bytes before "stco")
                if offset >= 4 && offset + 12 < buffer.len() {
                    let box_start = offset - 4;
                    let box_size = u32::from_be_bytes([
                        buffer[box_start],
                        buffer[box_start + 1],
                        buffer[box_start + 2],
                        buffer[box_start + 3],
                    ]) as usize;

                    // Verify this is a valid stco box
                    if box_size >= 12 && box_start + box_size <= buffer.len() {
                        let entry_count = u32::from_be_bytes([
                            buffer[offset + 8],
                            buffer[offset + 9],
                            buffer[offset + 10],
                            buffer[offset + 11],
                        ]) as usize;

                        // Update each chunk offset
                        let table_start = offset + 12;
                        if table_start + entry_count * 4 <= buffer.len() {
                            for i in 0..entry_count {
                                let offset_pos = table_start + i * 4;
                                let old_offset = u32::from_be_bytes([
                                    buffer[offset_pos],
                                    buffer[offset_pos + 1],
                                    buffer[offset_pos + 2],
                                    buffer[offset_pos + 3],
                                ]) as i64;

                                // Update all offsets (they all point to data after moov)
                                let new_offset = old_offset + moov_size_delta;
                                if new_offset >= 0 && new_offset <= u32::MAX as i64 {
                                    buffer[offset_pos..offset_pos + 4]
                                        .copy_from_slice(&(new_offset as u32).to_be_bytes());
                                }
                            }
                        }
                    }
                }
            } else if &buffer[offset..offset + 4] == b"co64" {
                // Found co64 box (8-byte offsets)
                if offset >= 4 && offset + 12 < buffer.len() {
                    let box_start = offset - 4;
                    let box_size = u32::from_be_bytes([
                        buffer[box_start],
                        buffer[box_start + 1],
                        buffer[box_start + 2],
                        buffer[box_start + 3],
                    ]) as usize;

                    // Verify this is a valid co64 box
                    if box_size >= 12 && box_start + box_size <= buffer.len() {
                        let entry_count = u32::from_be_bytes([
                            buffer[offset + 8],
                            buffer[offset + 9],
                            buffer[offset + 10],
                            buffer[offset + 11],
                        ]) as usize;

                        // Update each chunk offset
                        let table_start = offset + 12;
                        if table_start + entry_count * 8 <= buffer.len() {
                            for i in 0..entry_count {
                                let offset_pos = table_start + i * 8;
                                let old_offset = u64::from_be_bytes([
                                    buffer[offset_pos],
                                    buffer[offset_pos + 1],
                                    buffer[offset_pos + 2],
                                    buffer[offset_pos + 3],
                                    buffer[offset_pos + 4],
                                    buffer[offset_pos + 5],
                                    buffer[offset_pos + 6],
                                    buffer[offset_pos + 7],
                                ]) as i64;

                                // Update all offsets
                                let new_offset = old_offset + moov_size_delta;
                                if new_offset >= 0 {
                                    buffer[offset_pos..offset_pos + 8]
                                        .copy_from_slice(&new_offset.to_be_bytes());
                                }
                            }
                        }
                    }
                }
            }
            offset += 1;
        }

        Ok(())
    }
}
