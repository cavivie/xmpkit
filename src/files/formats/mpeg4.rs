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
pub struct Mpeg4Handler;

impl FileHandler for Mpeg4Handler {
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
struct Mpeg4Box {
    size: u64,
    box_type: [u8; 4],
    #[allow(dead_code)]
    data_offset: u64,
}

/// Box layout information for optimize-file-layout mode (matches Adobe C++ LayoutInfo)
#[cfg(feature = "optimize-file-layout")]
#[derive(Debug, Clone)]
struct BoxLayoutInfo {
    box_type: [u8; 4],
    box_size: u64,
    old_offset: u64,
    new_offset: u64,
}

impl Mpeg4Handler {
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
        box_info: &Mpeg4Box,
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
    fn read_box<R: Read + Seek>(reader: &mut R) -> std::io::Result<Mpeg4Box> {
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

        Ok(Mpeg4Box {
            size: actual_size,
            box_type,
            data_offset,
        })
    }

    /// Scan all top-level boxes and build layout map (matches Adobe C++ OptimizeFileLayout)
    /// Returns: (boxes, moov_index, xmp_index, needs_optimization)
    #[cfg(feature = "optimize-file-layout")]
    #[allow(clippy::type_complexity)]
    fn scan_boxes_for_optimization<R: Read + Seek>(
        reader: &mut R,
    ) -> XmpResult<(Vec<BoxLayoutInfo>, Option<usize>, Option<usize>, bool)> {
        let file_size = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(0))?;

        let mut boxes = Vec::new();
        let mut moov_index = None;
        let mut xmp_index = None;
        let mut moov_found = false;
        let mut xmp_found = false;
        let mut mdat_found = false;
        let mut needs_optimization = false;

        let mut curr_pos = 0u64;
        while curr_pos < file_size {
            reader.seek(SeekFrom::Start(curr_pos))?;
            let box_info = match Self::read_box(reader) {
                Ok(b) => b,
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e.into()),
            };

            // Skip free/skip/wide boxes (they will be removed in optimized layout)
            if box_info.box_type == *b"free"
                || box_info.box_type == *b"skip"
                || box_info.box_type == *b"wide"
            {
                curr_pos += box_info.size;
                continue;
            }

            let box_index = boxes.len();
            boxes.push(BoxLayoutInfo {
                box_type: box_info.box_type,
                box_size: box_info.size,
                old_offset: curr_pos,
                new_offset: 0, // Will be set later
            });

            if box_info.box_type == *b"mdat" {
                mdat_found = true;
            } else if box_info.box_type == *b"moov" {
                moov_found = true;
                moov_index = Some(box_index);
                needs_optimization = mdat_found;
                if xmp_found {
                    break;
                }
            } else if box_info.box_type == *BOX_TYPE_UUID {
                // Check if this is XMP UUID box
                let uuid_pos = reader.stream_position()?;
                let mut uuid = [0u8; 16];
                if reader.read_exact(&mut uuid).is_ok() && uuid == *XMP_UUID {
                    xmp_found = true;
                    xmp_index = Some(box_index);
                    needs_optimization = mdat_found;
                    if moov_found {
                        break;
                    }
                }
                reader.seek(SeekFrom::Start(uuid_pos))?;
            }

            curr_pos += box_info.size;
        }

        Ok((boxes, moov_index, xmp_index, needs_optimization))
    }

    /// Adjust offset based on layout map (matches Adobe C++ AdjustOffset)
    /// Uses oldEndMap approach: find box by checking which box's range contains the offset
    #[cfg(feature = "optimize-file-layout")]
    fn adjust_offset(old_offset: u64, boxes: &[BoxLayoutInfo]) -> XmpResult<u64> {
        // Find the box that contains this offset
        // Chunk offsets point to data within boxes (typically mdat boxes)
        // We need to find which box contains this offset
        for box_info in boxes {
            // Check if offset is within this box's range
            // Note: chunk offsets can point to any byte within the box
            if old_offset >= box_info.old_offset
                && old_offset < box_info.old_offset + box_info.box_size
            {
                // Calculate relative offset within the box
                let relative_offset = old_offset - box_info.old_offset;
                // Map to new position
                return Ok(box_info.new_offset + relative_offset);
            }
        }
        // If not found, try to find the closest box (for debugging)
        Err(XmpError::BadValue(format!(
            "Offset {} not found in any box. Available boxes: {}",
            old_offset,
            boxes
                .iter()
                .map(|b| format!(
                    "{:?} [{}, {})",
                    std::str::from_utf8(&b.box_type).unwrap_or("???"),
                    b.old_offset,
                    b.old_offset + b.box_size
                ))
                .collect::<Vec<_>>()
                .join(", ")
        )))
    }

    /// Write XMP with optimized file layout (matches Adobe C++ OptimizeFileLayout)
    /// Completely rewrites the file in optimized order: ftyp -> moov -> XMP uuid -> other non-mdat -> all mdat
    #[cfg(feature = "optimize-file-layout")]
    fn write_xmp_optimized_layout<R: Read + Seek, W: Write + Seek>(
        mut reader: R,
        mut writer: W,
        meta: &XmpMeta,
        ftyp_size: u64,
    ) -> XmpResult<()> {
        // Serialize XMP Packet
        let xmp_packet = meta.serialize_packet()?;
        let xmp_bytes = xmp_packet.as_bytes();

        // Scan all boxes and build layout map
        let (mut boxes, moov_index, _xmp_index, _needs_optimization) =
            Self::scan_boxes_for_optimization(&mut reader)?;

        if boxes.is_empty() {
            return Err(XmpError::BadValue("No boxes found in file".to_string()));
        }

        // Calculate UUID box size
        let uuid_box_data_size = 16 + xmp_bytes.len() as u64;
        let uuid_box_total_size = 8 + uuid_box_data_size;
        let uuid_box_size = if uuid_box_total_size > u32::MAX as u64 {
            16 + uuid_box_data_size // extended size header
        } else {
            uuid_box_total_size
        };

        // Build new layout: ftyp -> moov -> XMP uuid -> other non-mdat -> all mdat
        // First, process moov box to get its new size
        let (moov_buffer, new_moov_size) = if let Some(moov_idx) = moov_index {
            reader.seek(SeekFrom::Start(boxes[moov_idx].old_offset))?;
            let old_moov_data = {
                let mut data = vec![0u8; boxes[moov_idx].box_size as usize];
                reader.read_exact(&mut data)?;
                data
            };
            let mut moov_buffer = Vec::new();
            {
                use std::io::Cursor;
                let mut cursor = Cursor::new(&mut moov_buffer);
                let mut xmp_written_in_moov = false;
                Self::write_moov_with_xmp(
                    &mut Cursor::new(&old_moov_data),
                    &mut cursor,
                    boxes[moov_idx].box_size,
                    None, // ISO Base Media: don't write UUID in moov/udta
                    &mut xmp_written_in_moov,
                )?;
            }
            let new_moov_size = moov_buffer.len() as u64;
            // Update moov box header size
            if new_moov_size <= u32::MAX as u64 {
                moov_buffer[0..4].copy_from_slice(&(new_moov_size as u32).to_be_bytes());
            } else {
                moov_buffer[0..4].copy_from_slice(&1u32.to_be_bytes());
                if moov_buffer.len() < 16 {
                    return Err(XmpError::BadValue("Invalid moov box structure".to_string()));
                }
                moov_buffer[8..16].copy_from_slice(&new_moov_size.to_be_bytes());
            }
            boxes[moov_idx].box_size = new_moov_size;
            (Some(moov_buffer), new_moov_size)
        } else {
            (None, 0)
        };

        // Now calculate new layout with correct moov size
        let mut new_size = ftyp_size;

        // 1. ftyp (already written)
        // new_size already includes ftyp_size

        // 2. moov (if found)
        if let Some(moov_idx) = moov_index {
            boxes[moov_idx].new_offset = new_size;
            new_size += new_moov_size;
        }

        // 3. XMP uuid box
        new_size += uuid_box_size;

        // 4. Other non-mdat boxes (skip ftyp, moov, and mdat)
        for (i, box_info) in boxes.iter_mut().enumerate() {
            if Some(i) == moov_index {
                continue;
            }
            if box_info.box_type == *b"mdat" {
                continue;
            }
            if box_info.box_type == *MP4_SIGNATURE {
                // Skip ftyp - already written at the beginning
                continue;
            }
            box_info.new_offset = new_size;
            new_size += box_info.box_size;
        }

        // 5. All mdat boxes
        for box_info in &mut boxes {
            if box_info.box_type == *b"mdat" {
                box_info.new_offset = new_size;
                new_size += box_info.box_size;
            }
        }

        // Now update chunk offsets in moov buffer
        let moov_buffer = if let Some(mut moov_buf) = moov_buffer {
            // Update chunk offsets using layout map
            let stco_co64_offsets = Self::find_stco_co64_offsets(&moov_buf);
            for (table_offset, is_co64) in stco_co64_offsets {
                if table_offset < 4 || table_offset + 4 > moov_buf.len() {
                    continue;
                }
                let entry_count = u32::from_be_bytes([
                    moov_buf[table_offset - 4],
                    moov_buf[table_offset - 3],
                    moov_buf[table_offset - 2],
                    moov_buf[table_offset - 1],
                ]) as usize;

                if is_co64 {
                    // Update co64 entries
                    for j in 0..entry_count {
                        let offset_pos = table_offset + j * 8;
                        if offset_pos + 8 > moov_buf.len() {
                            break;
                        }
                        let old_offset = u64::from_be_bytes([
                            moov_buf[offset_pos],
                            moov_buf[offset_pos + 1],
                            moov_buf[offset_pos + 2],
                            moov_buf[offset_pos + 3],
                            moov_buf[offset_pos + 4],
                            moov_buf[offset_pos + 5],
                            moov_buf[offset_pos + 6],
                            moov_buf[offset_pos + 7],
                        ]);
                        match Self::adjust_offset(old_offset, &boxes) {
                            Ok(new_offset) => {
                                moov_buf[offset_pos..offset_pos + 8]
                                    .copy_from_slice(&new_offset.to_be_bytes());
                            }
                            Err(e) => {
                                return Err(XmpError::BadValue(format!(
                                    "Failed to adjust co64 offset {}: {}",
                                    old_offset, e
                                )));
                            }
                        }
                    }
                } else {
                    // Update stco entries
                    for j in 0..entry_count {
                        let offset_pos = table_offset + j * 4;
                        if offset_pos + 4 > moov_buf.len() {
                            break;
                        }
                        let old_offset = u32::from_be_bytes([
                            moov_buf[offset_pos],
                            moov_buf[offset_pos + 1],
                            moov_buf[offset_pos + 2],
                            moov_buf[offset_pos + 3],
                        ]) as u64;
                        match Self::adjust_offset(old_offset, &boxes) {
                            Ok(new_offset) if new_offset <= u32::MAX as u64 => {
                                moov_buf[offset_pos..offset_pos + 4]
                                    .copy_from_slice(&(new_offset as u32).to_be_bytes());
                            }
                            Ok(new_offset) => {
                                return Err(XmpError::BadValue(format!(
                                    "stco offset {} exceeds u32::MAX, cannot update",
                                    new_offset
                                )));
                            }
                            Err(e) => {
                                return Err(XmpError::BadValue(format!(
                                    "Failed to adjust stco offset {}: {}",
                                    old_offset, e
                                )));
                            }
                        }
                    }
                }
            }
            Some(moov_buf)
        } else {
            None
        };

        // Write boxes in new order
        // 1. ftyp (already written)
        // 2. moov
        if let Some(moov_buf) = moov_buffer {
            if let Some(moov_idx) = moov_index {
                writer.seek(SeekFrom::Start(boxes[moov_idx].new_offset))?;
                writer.write_all(&moov_buf)?;
            }
        }

        // 3. XMP uuid box
        Self::write_xmp_uuid_box(&mut writer, xmp_bytes)?;

        // 4. Other non-mdat boxes (skip ftyp, moov, and mdat)
        for (i, box_info) in boxes.iter().enumerate() {
            if Some(i) == moov_index {
                continue;
            }
            if box_info.box_type == *b"mdat" {
                continue;
            }
            if box_info.box_type == *MP4_SIGNATURE {
                // Skip ftyp - already written at the beginning
                continue;
            }
            reader.seek(SeekFrom::Start(box_info.old_offset))?;
            let mut box_data = vec![0u8; box_info.box_size as usize];
            reader.read_exact(&mut box_data)?;
            writer.write_all(&box_data)?;
        }

        // 5. All mdat boxes
        for box_info in &boxes {
            if box_info.box_type == *b"mdat" {
                reader.seek(SeekFrom::Start(box_info.old_offset))?;
                let mut box_data = vec![0u8; box_info.box_size as usize];
                reader.read_exact(&mut box_data)?;
                writer.write_all(&box_data)?;
            }
        }

        Ok(())
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

        // For optimize-file-layout mode, use complete rewrite approach (matches Adobe C++ OptimizeFileLayout)
        #[cfg(feature = "optimize-file-layout")]
        if is_iso_base_media {
            return Self::write_xmp_optimized_layout(reader, writer, meta, ftyp_box.size);
        }

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

                // For ISO Base Media format, UUID box placement depends on feature flag:
                // - Default (optimize-file-layout enabled): insert UUID box after moov, before mdat
                //   (matches Adobe's kXMPFiles_OptimizeFileLayout behavior - optimized for streaming)
                // - Feature disabled: append UUID box to end of file (matches Adobe default behavior)
                #[cfg(feature = "optimize-file-layout")]
                let uuid_box_size = if is_iso_base_media && !xmp_written {
                    // Calculate UUID box size: header (8 or 16 if extended) + UUID (16) + XMP data
                    let uuid_box_data_size = 16 + xmp_bytes.len() as u64;
                    let uuid_box_total_size = 8 + uuid_box_data_size;
                    // Check if extended size is needed (size > u32::MAX)
                    if uuid_box_total_size > u32::MAX as u64 {
                        16 + uuid_box_data_size // extended size header (16 bytes)
                    } else {
                        uuid_box_total_size // normal header (8 bytes)
                    }
                } else {
                    0
                };

                #[cfg(not(feature = "optimize-file-layout"))]
                let uuid_box_size = 0u64; // UUID box will be appended at end, no offset change needed

                // Update chunk offsets if moov size changed OR if UUID box will be inserted (fast-start mode)
                let total_offset_delta = moov_size_delta + uuid_box_size as i64;
                if total_offset_delta != 0 {
                    Self::update_chunk_offsets_in_buffer(&mut moov_buffer, total_offset_delta)?;
                }

                // Write the updated moov box buffer to the final writer
                writer.write_all(&moov_buffer)?;

                // For ISO Base Media format, write UUID box based on feature flag
                #[cfg(feature = "optimize-file-layout")]
                if is_iso_base_media && !xmp_written {
                    // Optimize file layout: write UUID box immediately after moov (before any free boxes or mdat)
                    // This matches Adobe's kXMPFiles_OptimizeFileLayout behavior
                    Self::write_xmp_uuid_box(&mut writer, xmp_bytes)?;
                    xmp_written = true;
                }

                #[cfg(not(feature = "optimize-file-layout"))]
                // Append mode: UUID box will be written at end of file (handled below)
                // This matches Adobe's default behavior when OptimizeFileLayout is not set
                if is_iso_base_media && !xmp_written {
                    // Mark that we need to append UUID box later
                    // (xmp_written remains false)
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
                #[cfg(feature = "optimize-file-layout")]
                {
                    // In optimize-file-layout mode, skip free/skip/wide boxes (they will be removed)
                    // This matches Adobe's OptimizeFileLayout behavior
                    if box_info.box_type == *b"free"
                        || box_info.box_type == *b"skip"
                        || box_info.box_type == *b"wide"
                    {
                        // Skip these boxes - they will be removed in optimized layout
                        // Reader is already at box_start from above
                        reader.seek(SeekFrom::Start(box_start + box_info.size))?;
                    } else {
                        // Copy other boxes as-is
                        reader.seek(SeekFrom::Start(box_start))?;
                        let mut box_data = vec![0u8; box_info.size as usize];
                        reader.read_exact(&mut box_data)?;
                        writer.write_all(&box_data)?;
                    }
                }

                #[cfg(not(feature = "optimize-file-layout"))]
                {
                    // Copy other boxes as-is
                    // Reader is already at box_start from above
                    let mut box_data = vec![0u8; box_info.size as usize];
                    reader.read_exact(&mut box_data)?;
                    writer.write_all(&box_data)?;
                }
            }
        }

        // Write XMP box based on file format and feature flag
        if !xmp_written {
            if is_iso_base_media {
                #[cfg(feature = "optimize-file-layout")]
                {
                    // Optimize file layout mode: This should not happen if moov box was found
                    // But handle the case where moov box doesn't exist
                    if let Some(pos) = xmp_box_pos {
                        // Replace existing UUID box
                        writer.seek(SeekFrom::Start(pos))?;
                        Self::write_xmp_uuid_box(&mut writer, xmp_bytes)?;
                    } else {
                        // No moov box found - write UUID box at current position
                        Self::write_xmp_uuid_box(&mut writer, xmp_bytes)?;
                    }
                }

                #[cfg(not(feature = "optimize-file-layout"))]
                {
                    // Append mode: Write UUID box at end of file (matches Adobe default behavior)
                    // This doesn't require updating chunk offsets since mdat position doesn't change
                    // Matches Adobe behavior when kXMPFiles_OptimizeFileLayout is not set
                    if let Some(pos) = xmp_box_pos {
                        // Replace existing UUID box at its original position
                        writer.seek(SeekFrom::Start(pos))?;
                        Self::write_xmp_uuid_box(&mut writer, xmp_bytes)?;
                    } else {
                        // Append UUID box to end of file
                        Self::write_xmp_uuid_box(&mut writer, xmp_bytes)?;
                    }
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
                    // ISO Base Media format: just copy udta content as-is (XMP goes in top-level UUID box)
                    // Note: header already written above, so only copy content
                    let header_size = if has_extended_size { 16 } else { 8 };
                    let udta_content_start = box_start + header_size;
                    let udta_content_size = box_info.size - header_size;
                    reader.seek(SeekFrom::Start(udta_content_start))?;
                    let mut content_data = vec![0u8; udta_content_size as usize];
                    reader.read_exact(&mut content_data)?;
                    writer.write_all(&content_data)?;
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

    /// Find stco/co64 box offsets within moov buffer (for updating chunk offsets after rewrite)
    #[cfg(feature = "optimize-file-layout")]
    fn find_stco_co64_offsets(moov_buffer: &[u8]) -> Vec<(usize, bool)> {
        // Returns: Vec<(table_offset, is_co64)>
        let mut offsets = Vec::new();
        Self::find_stco_co64_recursive(moov_buffer, 0, moov_buffer.len(), &mut offsets);
        offsets
    }

    #[cfg(feature = "optimize-file-layout")]
    fn find_stco_co64_recursive(
        buffer: &[u8],
        start: usize,
        end: usize,
        offsets: &mut Vec<(usize, bool)>,
    ) {
        let mut pos = start;
        while pos + 8 <= end {
            let box_size = if pos + 4 <= end {
                u32::from_be_bytes([
                    buffer[pos],
                    buffer[pos + 1],
                    buffer[pos + 2],
                    buffer[pos + 3],
                ]) as usize
            } else {
                break;
            };

            if box_size < 8 || pos + box_size > end {
                break;
            }

            let box_type = &buffer[pos + 4..pos + 8];
            let (actual_size, header_size) = if box_size == 1 {
                if pos + 16 > end {
                    break;
                }
                let ext_size = u64::from_be_bytes([
                    buffer[pos + 8],
                    buffer[pos + 9],
                    buffer[pos + 10],
                    buffer[pos + 11],
                    buffer[pos + 12],
                    buffer[pos + 13],
                    buffer[pos + 14],
                    buffer[pos + 15],
                ]) as usize;
                (ext_size, 16)
            } else {
                (box_size, 8)
            };

            if pos + actual_size > end {
                break;
            }

            if box_type == b"stco" || box_type == b"co64" {
                // stco/co64 structure: header + version/flags(4) + entry_count(4) + offsets(4*N or 8*N)
                // table_start should point to first offset, which is header_size + 8 bytes after box start
                if pos + header_size + 8 <= pos + actual_size {
                    let table_start = pos + header_size + 8;
                    offsets.push((table_start, box_type == b"co64"));
                }
            } else if box_type == b"moov"
                || box_type == b"trak"
                || box_type == b"mdia"
                || box_type == b"minf"
                || box_type == b"stbl"
            {
                let content_start = pos + header_size;
                let content_end = pos + actual_size;
                if content_start < content_end {
                    Self::find_stco_co64_recursive(buffer, content_start, content_end, offsets);
                }
            }

            pos += actual_size;
        }
    }

    /// Update chunk offsets in stco/co64 boxes when moov box size changes
    ///
    /// When moov box size changes, all chunk offsets that point to data after moov need to be adjusted
    /// Uses recursive traversal to correctly find stco/co64 boxes within the moov box structure
    fn update_chunk_offsets_in_buffer(buffer: &mut [u8], offset_delta: i64) -> XmpResult<()> {
        if offset_delta == 0 {
            return Ok(());
        }
        Self::update_chunk_offsets_recursive(buffer, 0, buffer.len(), offset_delta)
    }

    /// Recursively traverse MP4 box structure to find and update stco/co64 boxes
    fn update_chunk_offsets_recursive(
        buffer: &mut [u8],
        start: usize,
        end: usize,
        offset_delta: i64,
    ) -> XmpResult<()> {
        let mut pos = start;
        while pos + 8 <= end {
            // Read box size
            let box_size = if pos + 4 <= end {
                u32::from_be_bytes([
                    buffer[pos],
                    buffer[pos + 1],
                    buffer[pos + 2],
                    buffer[pos + 3],
                ]) as usize
            } else {
                break;
            };

            if box_size < 8 || pos + box_size > end {
                break;
            }

            let box_type = &buffer[pos + 4..pos + 8];

            // Handle extended size
            let (actual_size, header_size) = if box_size == 1 {
                if pos + 16 > end {
                    break;
                }
                let ext_size = u64::from_be_bytes([
                    buffer[pos + 8],
                    buffer[pos + 9],
                    buffer[pos + 10],
                    buffer[pos + 11],
                    buffer[pos + 12],
                    buffer[pos + 13],
                    buffer[pos + 14],
                    buffer[pos + 15],
                ]) as usize;
                (ext_size, 16)
            } else {
                (box_size, 8)
            };

            if pos + actual_size > end {
                break;
            }

            // Update stco box
            if box_type == b"stco" {
                if pos + header_size + 12 <= pos + actual_size {
                    let entry_count = u32::from_be_bytes([
                        buffer[pos + header_size + 8],
                        buffer[pos + header_size + 9],
                        buffer[pos + header_size + 10],
                        buffer[pos + header_size + 11],
                    ]) as usize;

                    let table_start = pos + header_size + 12;
                    if table_start + entry_count * 4 <= pos + actual_size {
                        for i in 0..entry_count {
                            let offset_pos = table_start + i * 4;
                            let old_offset = u32::from_be_bytes([
                                buffer[offset_pos],
                                buffer[offset_pos + 1],
                                buffer[offset_pos + 2],
                                buffer[offset_pos + 3],
                            ]) as i64;

                            let new_offset = old_offset + offset_delta;
                            if new_offset >= 0 && new_offset <= u32::MAX as i64 {
                                buffer[offset_pos..offset_pos + 4]
                                    .copy_from_slice(&(new_offset as u32).to_be_bytes());
                            }
                        }
                    }
                }
            }
            // Update co64 box
            else if box_type == b"co64" {
                if pos + header_size + 12 <= pos + actual_size {
                    let entry_count = u32::from_be_bytes([
                        buffer[pos + header_size + 8],
                        buffer[pos + header_size + 9],
                        buffer[pos + header_size + 10],
                        buffer[pos + header_size + 11],
                    ]) as usize;

                    let table_start = pos + header_size + 12;
                    if table_start + entry_count * 8 <= pos + actual_size {
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

                            let new_offset = old_offset + offset_delta;
                            if new_offset >= 0 {
                                buffer[offset_pos..offset_pos + 8]
                                    .copy_from_slice(&new_offset.to_be_bytes());
                            }
                        }
                    }
                }
            }
            // Recursively process container boxes
            else if box_type == b"moov"
                || box_type == b"trak"
                || box_type == b"mdia"
                || box_type == b"minf"
                || box_type == b"stbl"
            {
                let content_start = pos + header_size;
                let content_end = pos + actual_size;
                if content_start < content_end {
                    Self::update_chunk_offsets_recursive(
                        buffer,
                        content_start,
                        content_end,
                        offset_delta,
                    )?;
                }
            }

            pos += actual_size;
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

    // Minimal valid MP4 file with ftyp box but no XMP
    fn create_minimal_mp4() -> Vec<u8> {
        let mut mp4 = Vec::new();
        // ftyp box: size (20), type ("ftyp"), major brand ("isom"), minor version (0), compatible brands ("isom")
        mp4.extend_from_slice(&20u32.to_be_bytes()); // box size
        mp4.extend_from_slice(MP4_SIGNATURE); // "ftyp"
        mp4.extend_from_slice(b"isom"); // major brand
        mp4.extend_from_slice(&0u32.to_be_bytes()); // minor version
        mp4.extend_from_slice(b"isom"); // compatible brand
        mp4
    }

    #[test]
    fn test_read_xmp_no_xmp() {
        let mp4_data = create_minimal_mp4();
        let reader = Cursor::new(mp4_data);
        let result = Mpeg4Handler::read_xmp(reader).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_invalid_mp4() {
        let invalid_data = vec![0x00, 0x01, 0x02, 0x03];
        let reader = Cursor::new(invalid_data);
        let result = Mpeg4Handler::read_xmp(reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_write_xmp() {
        // Create minimal MP4
        let mp4_data = create_minimal_mp4();
        let reader = Cursor::new(mp4_data);
        let mut writer = Cursor::new(Vec::new());

        // Create XMP metadata
        let mut meta = XmpMeta::new();
        meta.set_property(ns::DC, "title", XmpValue::String("Test Video".to_string()))
            .unwrap();

        // Write XMP
        Mpeg4Handler::write_xmp(reader, &mut writer, &meta).unwrap();

        // Read back XMP
        writer.set_position(0);
        let result = Mpeg4Handler::read_xmp(writer).unwrap();
        assert!(result.is_some());

        let read_meta = result.unwrap();
        let title_value = read_meta.get_property(ns::DC, "title");
        assert!(title_value.is_some());
        if let Some(XmpValue::String(title)) = title_value {
            assert_eq!(title, "Test Video");
        } else {
            panic!("Expected string value");
        }
    }
}
