//! MPEG-H HEIF(High Efficiency Image File Format) family support
//!
//! - Detect brands: `mif1`, `msf1`, `heic`, `heix`, `hevc`, `heis`, `avif`, `avis`
//! - XMP Storage: Typically in the top-level `meta` sub-box with `uuid`(XMP UUID) or `xml ` box.
//! - Write Strategy: Rewrite `meta` box, preserve version/flags, replace/append XMP box, copy other boxes as-is.

use crate::core::error::{XmpError, XmpResult};
use crate::core::metadata::XmpMeta;
use crate::files::formats::bmff::{
    copy_bytes, is_bmff, read_box, read_box_data, skip_box, FTYP_BOX, UUID_BOX, XMP_UUID,
};
use crate::files::handler::{FileHandler, XmpOptions};
use std::io::{Read, Seek, SeekFrom, Write};

/// MPEG-H file handler for XMP metadata
#[derive(Debug, Clone, Copy, Default)]
pub struct MpeghHandler;

/// HEIF / AVIF compatible brands
const HEIF_BRANDS: &[[u8; 4]] = &[
    *b"mif1", *b"msf1", *b"heic", *b"heix", *b"hevc", *b"heis", *b"avif", *b"avis",
];

// XMP_UUID is imported from bmff module

/// Box types used in HEIF metadata storage
const BOX_TYPE_XML: &[u8; 4] = b"xml ";
const BOX_TYPE_META: &[u8; 4] = b"meta";
// UUID_BOX is imported from bmff module

impl FileHandler for MpeghHandler {
    fn can_handle<R: Read + Seek>(&self, reader: &mut R) -> XmpResult<bool> {
        let pos = reader.stream_position()?;

        if !is_bmff(reader)? {
            reader.seek(SeekFrom::Start(pos))?;
            return Ok(false);
        }

        // Read primary brand (ftyp major brand)
        reader.seek(SeekFrom::Start(8))?;
        let mut brand = [0u8; 4];
        if reader.read_exact(&mut brand).is_err() {
            reader.seek(SeekFrom::Start(pos))?;
            return Ok(false);
        }
        reader.seek(SeekFrom::Start(pos))?;

        Ok(HEIF_BRANDS.contains(&brand))
    }

    fn read_xmp<R: Read + Seek>(
        &self,
        reader: &mut R,
        options: &XmpOptions,
    ) -> XmpResult<Option<XmpMeta>> {
        Self::read_xmp(reader, options)
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
        "HEIF"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["heic", "heif", "avif"]
    }
}

impl MpeghHandler {
    /// Read XMP from a HEIF file (search `meta` -> `uuid`(XMP UUID) or `xml `)
    pub fn read_xmp<R: Read + Seek>(
        mut reader: R,
        options: &XmpOptions,
    ) -> XmpResult<Option<XmpMeta>> {
        // ftyp
        let ftyp = read_box(&mut reader)?;
        if ftyp.box_type != *FTYP_BOX {
            return Err(XmpError::BadValue("Not a valid HEIF file".into()));
        }
        skip_box(&mut reader, &ftyp)?;

        // scan top-level boxes
        loop {
            let box_start = reader.stream_position()?;
            let box_info = match read_box(&mut reader) {
                Ok(b) => b,
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e.into()),
            };

            if box_info.box_type == *BOX_TYPE_META {
                // meta box content starts after header
                let meta_body = read_box_data(&mut reader, &box_info)?;
                let xmp_result = Self::extract_xmp_from_meta(&meta_body)?;
                if options.only_xmp {
                    return Ok(xmp_result);
                }

                let xmp_result_is_none = xmp_result.is_none();
                let mut meta = xmp_result.unwrap_or_else(XmpMeta::new);
                let mut reconciled = false;

                // Need to pass reader and file structure for Exif parsing
                // Store current position and meta box info for native metadata reading
                let meta_box_start = box_start;
                let meta_box_size = box_info.size;
                if let Some(native) = native_reconcile::read_native_metadata(
                    &meta_body,
                    &mut reader,
                    meta_box_start,
                    meta_box_size,
                )? {
                    native_reconcile::reconcile_to_xmp(&mut meta, &native);
                    reconciled = true;
                }

                if xmp_result_is_none && !reconciled {
                    return Ok(None);
                }
                return Ok(Some(meta));
            } else {
                // skip box payload
                skip_box(&mut reader, &box_info)?;
            }
        }

        Ok(None)
    }

    /// Write XMP into HEIF by rewriting `meta` box and copying rest of file
    pub fn write_xmp<R: Read + Seek, W: Write + Seek>(
        mut reader: R,
        mut writer: W,
        meta: &XmpMeta,
    ) -> XmpResult<()> {
        let xmp_packet = meta.serialize_packet()?;
        let xmp_bytes = xmp_packet.as_bytes();

        // Read and copy ftyp
        let ftyp_box = read_box(&mut reader)?;
        if ftyp_box.box_type != *FTYP_BOX {
            return Err(XmpError::BadValue("Not a valid HEIF file".into()));
        }
        reader.seek(SeekFrom::Start(0))?;
        copy_bytes(&mut reader, &mut writer, ftyp_box.size)?;

        skip_box(&mut reader, &ftyp_box)?;

        let mut meta_written = false;

        // Process remaining top-level boxes
        loop {
            let box_start = reader.stream_position()?;
            let box_info = match read_box(&mut reader) {
                Ok(b) => b,
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e.into()),
            };

            if box_info.box_type == *BOX_TYPE_META {
                let meta_body = read_box_data(&mut reader, &box_info)?;
                let new_meta_body = Self::update_meta_with_xmp(&meta_body, xmp_bytes)?;
                Self::write_box(&mut writer, BOX_TYPE_META, &new_meta_body)?;
                meta_written = true;
            } else {
                // copy box as-is
                reader.seek(SeekFrom::Start(box_start))?;
                copy_bytes(&mut reader, &mut writer, box_info.size)?;
            }
        }

        if !meta_written {
            return Err(XmpError::BadValue(
                "HEIF meta box not found; cannot write XMP".into(),
            ));
        }

        Ok(())
    }

    /// Extract XMP from meta box body (after 4-byte version/flags)
    fn extract_xmp_from_meta(meta_body: &[u8]) -> XmpResult<Option<XmpMeta>> {
        if meta_body.len() < 4 {
            return Ok(None);
        }
        let mut cursor = 4usize; // skip version/flags
        while cursor + 8 <= meta_body.len() {
            let size = u32::from_be_bytes(meta_body[cursor..cursor + 4].try_into().unwrap()) as u64;
            let box_type = &meta_body[cursor + 4..cursor + 8];
            let (header, content_offset) = if size == 1 {
                if cursor + 16 > meta_body.len() {
                    break;
                }
                let ext =
                    u64::from_be_bytes(meta_body[cursor + 8..cursor + 16].try_into().unwrap());
                (16u64, ext.saturating_sub(16))
            } else {
                (8u64, size.saturating_sub(8))
            };
            let end = cursor + size as usize;
            if end > meta_body.len() {
                break;
            }
            let content_start = cursor + header as usize;

            if box_type == *UUID_BOX && content_offset >= 16 {
                let uuid = &meta_body[content_start..content_start + 16];
                if uuid == XMP_UUID {
                    let payload = &meta_body[content_start + 16..end];
                    let payload_str = std::str::from_utf8(payload).map_err(|e| {
                        XmpError::BadValue(format!("Invalid UTF-8 in HEIF XMP payload: {}", e))
                    })?;
                    let xmp = XmpMeta::parse(payload_str)?;
                    return Ok(Some(xmp));
                }
            } else if box_type == BOX_TYPE_XML {
                let payload = &meta_body[content_start..end];
                let payload_str = std::str::from_utf8(payload).map_err(|e| {
                    XmpError::BadValue(format!("Invalid UTF-8 in HEIF XMP payload: {}", e))
                })?;
                let xmp = XmpMeta::parse(payload_str)?;
                return Ok(Some(xmp));
            }

            let next = end;
            if next <= cursor {
                break;
            }
            cursor = next;
        }

        Ok(None)
    }

    /// Update meta body with new XMP packet; returns rebuilt meta body
    fn update_meta_with_xmp(meta_body: &[u8], xmp_bytes: &[u8]) -> XmpResult<Vec<u8>> {
        if meta_body.len() < 4 {
            return Err(XmpError::BadValue(
                "Invalid meta box (no version/flags)".into(),
            ));
        }

        let mut out = Vec::with_capacity(meta_body.len() + xmp_bytes.len() + 32);
        // preserve version/flags
        out.extend_from_slice(&meta_body[..4]);

        let mut cursor = 4usize;
        let mut replaced = false;

        while cursor + 8 <= meta_body.len() {
            let size = u32::from_be_bytes(meta_body[cursor..cursor + 4].try_into().unwrap()) as u64;
            let box_type = &meta_body[cursor + 4..cursor + 8];
            let (header, content_offset) = if size == 1 {
                if cursor + 16 > meta_body.len() {
                    break;
                }
                let ext =
                    u64::from_be_bytes(meta_body[cursor + 8..cursor + 16].try_into().unwrap());
                (16u64, ext.saturating_sub(16))
            } else {
                (8u64, size.saturating_sub(8))
            };
            let end = cursor + size as usize;
            if end > meta_body.len() {
                break;
            }
            let content_start = cursor + header as usize;

            // Replace uuid(XMP) or xml box with new payload
            if !replaced
                && ((box_type == *UUID_BOX
                    && content_offset >= 16
                    && &meta_body[content_start..content_start + 16] == XMP_UUID)
                    || box_type == *BOX_TYPE_XML)
            {
                let new_payload = if box_type == *UUID_BOX {
                    let mut buf = Vec::with_capacity(16 + xmp_bytes.len());
                    buf.extend_from_slice(XMP_UUID);
                    buf.extend_from_slice(xmp_bytes);
                    buf
                } else {
                    xmp_bytes.to_vec()
                };
                let box_tag: [u8; 4] = box_type.try_into().unwrap();
                Self::write_box(&mut out, &box_tag, &new_payload)?;
                replaced = true;
            } else {
                // copy original child box
                out.extend_from_slice(&meta_body[cursor..end]);
            }

            let next = end;
            if next <= cursor {
                break;
            }
            cursor = next;
        }

        if !replaced {
            // Append new uuid box with XMP
            let mut payload = Vec::with_capacity(16 + xmp_bytes.len());
            payload.extend_from_slice(XMP_UUID);
            payload.extend_from_slice(xmp_bytes);
            Self::write_box(&mut out, UUID_BOX, &payload)?;
        }

        Ok(out)
    }

    // read_box_data_exact is replaced by read_box_data from bmff module

    /// Helper: write a BMFF box (size + type + payload) to writer
    fn write_box<W: Write>(writer: &mut W, box_type: &[u8; 4], payload: &[u8]) -> XmpResult<()> {
        let size = 8u64 + payload.len() as u64;
        if size > u32::MAX as u64 {
            return Err(XmpError::BadValue("Box too large for 32-bit size".into()));
        }
        writer.write_all(&(size as u32).to_be_bytes())?;
        writer.write_all(box_type)?;
        writer.write_all(payload)?;
        Ok(())
    }
}

/// Native metadata reconciliation (HEIF)
mod native_reconcile {
    use super::*;

    /// HEIF native metadata item
    #[derive(Debug, Clone)]
    pub enum NativeMetadataItem {
        Exif(ExifFields),
        #[allow(dead_code)] // Reserved for future text metadata support
        Text {
            box_type: [u8; 4],
            value: String,
        },
    }

    /// Exif fields extracted from HEIF
    #[derive(Debug, Clone, Default)]
    pub struct ExifFields {
        pub datetime_original: Option<String>,
        pub make: Option<String>,
        pub model: Option<String>,
        pub artist: Option<String>,
        pub copyright: Option<String>,
        pub software: Option<String>,
    }

    /// Read native metadata from HEIF meta box body
    /// Scans for non-XMP metadata boxes (mainly Exif) that can be reconciled into XMP
    pub fn read_native_metadata<R: Read + Seek>(
        meta_body: &[u8],
        reader: &mut R,
        _meta_box_start: u64,
        _meta_box_size: u64,
    ) -> XmpResult<Option<Vec<NativeMetadataItem>>> {
        if meta_body.len() < 4 {
            return Ok(None);
        }

        let mut items = Vec::new();
        let mut cursor = 4usize; // skip version/flags

        while cursor + 8 <= meta_body.len() {
            let size = u32::from_be_bytes(
                meta_body[cursor..cursor + 4]
                    .try_into()
                    .map_err(|_| XmpError::BadValue("Invalid box size".into()))?,
            ) as u64;

            if size < 8 {
                break;
            }

            let box_type: [u8; 4] = meta_body[cursor + 4..cursor + 8]
                .try_into()
                .map_err(|_| XmpError::BadValue("Invalid box type".into()))?;

            let (header_size, content_offset) = if size == 1 {
                if cursor + 16 > meta_body.len() {
                    break;
                }
                let ext = u64::from_be_bytes(
                    meta_body[cursor + 8..cursor + 16]
                        .try_into()
                        .map_err(|_| XmpError::BadValue("Invalid extended size".into()))?,
                );
                (16u64, ext.saturating_sub(16))
            } else {
                (8u64, size.saturating_sub(8))
            };

            let end = cursor + size as usize;
            if end > meta_body.len() {
                break;
            }

            let content_start = cursor + header_size as usize;

            // Skip XMP boxes (uuid with XMP UUID or xml box)
            let is_xmp = if box_type == *UUID_BOX && content_offset >= 16 {
                content_start + 16 <= meta_body.len()
                    && &meta_body[content_start..content_start + 16] == XMP_UUID
            } else {
                box_type == *BOX_TYPE_XML
            };

            if !is_xmp {
                // Parse HEIF item structure boxes (iinf, iloc, iref) to find Exif
                match &box_type {
                    b"iinf" => {
                        // Item Information Box - find Exif item
                        if let Some(exif_item_id) =
                            parse_iinf_for_exif(&meta_body[content_start..end])?
                        {
                            // Try to find Exif location in iloc (will be parsed later)
                            // For now, store Exif item ID for later processing
                            if let Some(exif_data) =
                                find_and_read_exif(reader, meta_body, exif_item_id)?
                            {
                                if let Some(exif_fields) = parse_exif_tiff(&exif_data)? {
                                    items.push(NativeMetadataItem::Exif(exif_fields));
                                }
                            }
                        }
                    }
                    _ => {
                        // Try to extract text from other metadata boxes
                        if let Some(value) =
                            extract_text_from_box(&box_type, &meta_body[content_start..end])?
                        {
                            items.push(NativeMetadataItem::Text { box_type, value });
                        }
                    }
                }
            }

            let next = end;
            if next <= cursor {
                break;
            }
            cursor = next;
        }

        if items.is_empty() {
            Ok(None)
        } else {
            Ok(Some(items))
        }
    }

    /// Parse iinf (Item Information Box) to find Exif item ID
    fn parse_iinf_for_exif(iinf_data: &[u8]) -> XmpResult<Option<u32>> {
        if iinf_data.len() < 4 {
            return Ok(None);
        }

        let mut cursor = 0usize;
        // iinf starts with version (1 byte) + flags (3 bytes) + entry_count (variable)
        let version = iinf_data[cursor];
        cursor += 4;

        // Read entry_count (can be 1 or 4 bytes depending on version)
        let entry_count = if version == 0 {
            if cursor + 2 > iinf_data.len() {
                return Ok(None);
            }
            u16::from_be_bytes([iinf_data[cursor], iinf_data[cursor + 1]]) as u32
        } else {
            if cursor + 4 > iinf_data.len() {
                return Ok(None);
            }
            u32::from_be_bytes([
                iinf_data[cursor],
                iinf_data[cursor + 1],
                iinf_data[cursor + 2],
                iinf_data[cursor + 3],
            ])
        };
        cursor += if version == 0 { 2 } else { 4 };

        // Parse each infe (Item Information Entry)
        for _ in 0..entry_count {
            if cursor + 4 > iinf_data.len() {
                break;
            }

            // Read infe box header
            let infe_size = u32::from_be_bytes([
                iinf_data[cursor],
                iinf_data[cursor + 1],
                iinf_data[cursor + 2],
                iinf_data[cursor + 3],
            ]) as usize;

            if infe_size < 8 || cursor + infe_size > iinf_data.len() {
                break;
            }

            let infe_type = &iinf_data[cursor + 4..cursor + 8];
            if infe_type == b"infe" {
                // Parse infe content to find item_type "Exif"
                let infe_content_start = cursor + 8;
                if infe_content_start + 4 <= iinf_data.len() {
                    // Check for "Exif" item type (simplified - actual structure is more complex)
                    // Infe structure: version(1) + flags(3) + item_ID + item_type + ...
                    let item_type_start = infe_content_start + 4; // Skip version/flags and item_ID
                    if item_type_start + 4 <= iinf_data.len() {
                        let item_type = &iinf_data[item_type_start..item_type_start + 4];
                        if item_type == b"Exif" {
                            // Found Exif item - extract item ID
                            if infe_content_start + 4 <= iinf_data.len() {
                                let item_id = u32::from_be_bytes([
                                    iinf_data[infe_content_start],
                                    iinf_data[infe_content_start + 1],
                                    iinf_data[infe_content_start + 2],
                                    iinf_data[infe_content_start + 3],
                                ]);
                                return Ok(Some(item_id));
                            }
                        }
                    }
                }
            }

            cursor += infe_size;
        }

        Ok(None)
    }

    /// Find Exif item location in iloc and read Exif data from mdat
    fn find_and_read_exif<R: Read + Seek>(
        reader: &mut R,
        meta_body: &[u8],
        exif_item_id: u32,
    ) -> XmpResult<Option<Vec<u8>>> {
        // First, find iloc box in meta_body
        let mut cursor = 4usize; // skip version/flags
        let mut iloc_data: Option<&[u8]> = None;

        while cursor + 8 <= meta_body.len() {
            let size = u32::from_be_bytes(
                meta_body[cursor..cursor + 4]
                    .try_into()
                    .map_err(|_| XmpError::BadValue("Invalid box size".into()))?,
            ) as usize;

            if size < 8 || cursor + size > meta_body.len() {
                break;
            }

            let box_type = &meta_body[cursor + 4..cursor + 8];
            if box_type == b"iloc" {
                let content_start = cursor + 8;
                iloc_data = Some(&meta_body[content_start..cursor + size]);
                break;
            }

            cursor += size;
        }

        let iloc_data = match iloc_data {
            Some(d) => d,
            None => return Ok(None),
        };

        // Parse iloc to find Exif item location
        let exif_location = parse_iloc_for_item(iloc_data, exif_item_id)?;
        let exif_location = match exif_location {
            Some(loc) => loc,
            None => return Ok(None),
        };

        // Find mdat box and read Exif data
        let saved_pos = reader.stream_position()?;
        reader.seek(SeekFrom::Start(0))?;

        // Skip ftyp
        let ftyp = read_box(reader)?;
        skip_box(reader, &ftyp)?;

        // Find mdat box
        while let Ok(box_info) = read_box(reader) {
            if box_info.box_type == *b"mdat" {
                // Found mdat - read Exif data
                let mdat_data_start = box_info.data_offset;
                let exif_offset = mdat_data_start + exif_location.offset;
                reader.seek(SeekFrom::Start(exif_offset))?;

                let mut exif_data = vec![0u8; exif_location.length as usize];
                reader.read_exact(&mut exif_data)?;

                reader.seek(SeekFrom::Start(saved_pos))?;
                return Ok(Some(exif_data));
            } else {
                skip_box(reader, &box_info)?;
            }
        }

        reader.seek(SeekFrom::Start(saved_pos))?;
        Ok(None)
    }

    /// Parse iloc (Item Location Box) to find item location
    struct ItemLocation {
        offset: u64,
        length: u64,
    }

    fn parse_iloc_for_item(iloc_data: &[u8], item_id: u32) -> XmpResult<Option<ItemLocation>> {
        if iloc_data.len() < 8 {
            return Ok(None);
        }

        let mut cursor = 0usize;
        // iloc: version(1) + flags(3) + offset_size(4 bits) + length_size(4 bits) + base_offset_size(4 bits) + index_size(4 bits)
        let version = iloc_data[cursor];
        cursor += 4;

        let size_flags = iloc_data[cursor];
        cursor += 1;
        let offset_size = ((size_flags >> 4) & 0x0F) as usize;
        let length_size = (size_flags & 0x0F) as usize;
        let base_offset_size = ((iloc_data[cursor] >> 4) & 0x0F) as usize;
        cursor += 1;
        let index_size = if version < 2 {
            0
        } else {
            (iloc_data[cursor] & 0x0F) as usize
        };
        cursor += if version < 2 { 0 } else { 1 };

        // Read item_count
        let item_count = if version < 2 {
            if cursor + 2 > iloc_data.len() {
                return Ok(None);
            }
            u16::from_be_bytes([iloc_data[cursor], iloc_data[cursor + 1]]) as u32
        } else {
            if cursor + 4 > iloc_data.len() {
                return Ok(None);
            }
            u32::from_be_bytes([
                iloc_data[cursor],
                iloc_data[cursor + 1],
                iloc_data[cursor + 2],
                iloc_data[cursor + 3],
            ])
        };
        cursor += if version < 2 { 2 } else { 4 };

        // Parse each item
        for _ in 0..item_count {
            // Read item_ID
            if cursor + 2 > iloc_data.len() {
                break;
            }
            let current_item_id = if version < 2 {
                u16::from_be_bytes([iloc_data[cursor], iloc_data[cursor + 1]]) as u32
            } else {
                if cursor + 4 > iloc_data.len() {
                    break;
                }
                u32::from_be_bytes([
                    iloc_data[cursor],
                    iloc_data[cursor + 1],
                    iloc_data[cursor + 2],
                    iloc_data[cursor + 3],
                ])
            };
            cursor += if version < 2 { 2 } else { 4 };

            // Skip construction_method (2 bits) if version >= 1
            if version >= 1 {
                cursor += 1; // Skip reserved + construction_method
            }

            // Read data_reference_index
            if cursor + 2 > iloc_data.len() {
                break;
            }
            cursor += 2;

            // Read base_offset
            if cursor + base_offset_size > iloc_data.len() {
                break;
            }
            let base_offset =
                read_variable_size_int(&iloc_data[cursor..cursor + base_offset_size])?;
            cursor += base_offset_size;

            // Read extent_count
            if cursor + 2 > iloc_data.len() {
                break;
            }
            let extent_count =
                u16::from_be_bytes([iloc_data[cursor], iloc_data[cursor + 1]]) as usize;
            cursor += 2;

            if current_item_id == item_id && extent_count > 0 {
                // Read first extent
                if cursor + index_size + offset_size + length_size > iloc_data.len() {
                    break;
                }
                cursor += index_size; // Skip extent_index if present
                let extent_offset =
                    read_variable_size_int(&iloc_data[cursor..cursor + offset_size])?;
                cursor += offset_size;
                let extent_length =
                    read_variable_size_int(&iloc_data[cursor..cursor + length_size])?;

                return Ok(Some(ItemLocation {
                    offset: base_offset + extent_offset,
                    length: extent_length,
                }));
            }

            // Skip remaining extents for this item
            for _ in 0..extent_count {
                if cursor + index_size + offset_size + length_size > iloc_data.len() {
                    break;
                }
                cursor += index_size + offset_size + length_size;
            }
        }

        Ok(None)
    }

    /// Read variable-size integer (1, 2, 4, or 8 bytes)
    fn read_variable_size_int(data: &[u8]) -> XmpResult<u64> {
        match data.len() {
            1 => Ok(data[0] as u64),
            2 => Ok(u16::from_be_bytes([data[0], data[1]]) as u64),
            4 => Ok(u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as u64),
            8 => Ok(u64::from_be_bytes([
                data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
            ])),
            _ => Err(XmpError::BadValue("Invalid variable-size integer".into())),
        }
    }

    /// Parse Exif/TIFF data and extract key fields
    fn parse_exif_tiff(exif_data: &[u8]) -> XmpResult<Option<ExifFields>> {
        if exif_data.len() < 8 {
            return Ok(None);
        }

        // Check TIFF header (II for little-endian or MM for big-endian)
        let is_le = exif_data[0] == 0x49
            && exif_data[1] == 0x49
            && exif_data[2] == 0x2A
            && exif_data[3] == 0x00;
        let is_be = exif_data[0] == 0x4D
            && exif_data[1] == 0x4D
            && exif_data[2] == 0x00
            && exif_data[3] == 0x2A;

        if !is_le && !is_be {
            return Ok(None);
        }

        let mut fields = ExifFields::default();

        // Read first IFD offset (bytes 4-7)
        let first_ifd_offset = if is_le {
            u32::from_le_bytes([exif_data[4], exif_data[5], exif_data[6], exif_data[7]]) as usize
        } else {
            u32::from_be_bytes([exif_data[4], exif_data[5], exif_data[6], exif_data[7]]) as usize
        };

        if first_ifd_offset >= exif_data.len() {
            return Ok(None);
        }

        // Parse IFD entries
        parse_ifd_entries(
            &exif_data[first_ifd_offset..],
            exif_data,
            is_le,
            &mut fields,
        )?;

        if fields.datetime_original.is_some()
            || fields.make.is_some()
            || fields.model.is_some()
            || fields.artist.is_some()
            || fields.copyright.is_some()
            || fields.software.is_some()
        {
            Ok(Some(fields))
        } else {
            Ok(None)
        }
    }

    /// Parse IFD entries and extract Exif fields
    fn parse_ifd_entries(
        ifd_data: &[u8],
        full_data: &[u8],
        is_le: bool,
        fields: &mut ExifFields,
    ) -> XmpResult<()> {
        if ifd_data.len() < 2 {
            return Ok(());
        }

        // Read entry count
        let entry_count = if is_le {
            u16::from_le_bytes([ifd_data[0], ifd_data[1]]) as usize
        } else {
            u16::from_be_bytes([ifd_data[0], ifd_data[1]]) as usize
        };

        let mut cursor = 2;
        for _ in 0..entry_count {
            if cursor + 12 > ifd_data.len() {
                break;
            }

            // Read IFD entry (12 bytes: tag(2) + type(2) + count(4) + value/offset(4))
            let tag = if is_le {
                u16::from_le_bytes([ifd_data[cursor], ifd_data[cursor + 1]])
            } else {
                u16::from_be_bytes([ifd_data[cursor], ifd_data[cursor + 1]])
            };
            let type_ = if is_le {
                u16::from_le_bytes([ifd_data[cursor + 2], ifd_data[cursor + 3]])
            } else {
                u16::from_be_bytes([ifd_data[cursor + 2], ifd_data[cursor + 3]])
            };
            let count = if is_le {
                u32::from_le_bytes([
                    ifd_data[cursor + 4],
                    ifd_data[cursor + 5],
                    ifd_data[cursor + 6],
                    ifd_data[cursor + 7],
                ])
            } else {
                u32::from_be_bytes([
                    ifd_data[cursor + 4],
                    ifd_data[cursor + 5],
                    ifd_data[cursor + 6],
                    ifd_data[cursor + 7],
                ])
            };
            let value_or_offset = if is_le {
                u32::from_le_bytes([
                    ifd_data[cursor + 8],
                    ifd_data[cursor + 9],
                    ifd_data[cursor + 10],
                    ifd_data[cursor + 11],
                ])
            } else {
                u32::from_be_bytes([
                    ifd_data[cursor + 8],
                    ifd_data[cursor + 9],
                    ifd_data[cursor + 10],
                    ifd_data[cursor + 11],
                ])
            };

            // Extract key Exif fields
            match tag {
                0x0132 => {
                    // DateTime
                    if let Some(val) =
                        read_exif_string(full_data, type_, count, value_or_offset, is_le)?
                    {
                        fields.datetime_original = Some(val);
                    }
                }
                0x9003 => {
                    // DateTimeOriginal
                    if let Some(val) =
                        read_exif_string(full_data, type_, count, value_or_offset, is_le)?
                    {
                        fields.datetime_original = Some(val);
                    }
                }
                0x010F => {
                    // Make
                    if let Some(val) =
                        read_exif_string(full_data, type_, count, value_or_offset, is_le)?
                    {
                        fields.make = Some(val);
                    }
                }
                0x0110 => {
                    // Model
                    if let Some(val) =
                        read_exif_string(full_data, type_, count, value_or_offset, is_le)?
                    {
                        fields.model = Some(val);
                    }
                }
                0x013B => {
                    // Artist
                    if let Some(val) =
                        read_exif_string(full_data, type_, count, value_or_offset, is_le)?
                    {
                        fields.artist = Some(val);
                    }
                }
                0x8298 => {
                    // Copyright
                    if let Some(val) =
                        read_exif_string(full_data, type_, count, value_or_offset, is_le)?
                    {
                        fields.copyright = Some(val);
                    }
                }
                0x0131 => {
                    // Software
                    if let Some(val) =
                        read_exif_string(full_data, type_, count, value_or_offset, is_le)?
                    {
                        fields.software = Some(val);
                    }
                }
                _ => {}
            }

            cursor += 12;
        }

        Ok(())
    }

    /// Read Exif string value
    fn read_exif_string(
        full_data: &[u8],
        type_: u16,
        count: u32,
        value_or_offset: u32,
        is_le: bool,
    ) -> XmpResult<Option<String>> {
        if type_ != 2 {
            // ASCII type
            return Ok(None);
        }

        let data = if count <= 4 {
            // Value is inline - copy to Vec to avoid lifetime issues
            let bytes: [u8; 4] = if is_le {
                value_or_offset.to_le_bytes()
            } else {
                value_or_offset.to_be_bytes()
            };
            bytes[..count as usize].to_vec()
        } else {
            // Value is at offset
            let offset = value_or_offset as usize;
            if offset + count as usize > full_data.len() {
                return Ok(None);
            }
            full_data[offset..offset + count as usize].to_vec()
        };

        // Exif strings are null-terminated
        let null_pos = data.iter().position(|&b| b == 0).unwrap_or(data.len());
        let text = String::from_utf8_lossy(&data[..null_pos])
            .trim()
            .to_string();
        if text.is_empty() {
            Ok(None)
        } else {
            Ok(Some(text))
        }
    }

    /// Extract text value from a metadata box
    fn extract_text_from_box(_box_type: &[u8; 4], content: &[u8]) -> XmpResult<Option<String>> {
        // For now, we only handle simple text boxes
        // HEIF doesn't have as many standard metadata boxes as QuickTime,
        // but we can try to extract UTF-8 text from unknown boxes
        if content.is_empty() {
            return Ok(None);
        }

        // Try to decode as UTF-8
        if let Ok(text) = std::str::from_utf8(content) {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                return Ok(Some(trimmed.to_string()));
            }
        }

        // Try to decode as UTF-16 BE (common in some metadata formats)
        if content.len().is_multiple_of(2) && content.len() >= 2 {
            let mut u16s = Vec::with_capacity(content.len() / 2);
            for chunk in content.chunks(2) {
                if chunk.len() == 2 {
                    u16s.push(u16::from_be_bytes([chunk[0], chunk[1]]));
                }
            }
            let text = String::from_utf16_lossy(&u16s);
            let trimmed = text.trim();
            if !trimmed.is_empty()
                && trimmed.chars().all(|c| {
                    c.is_control()
                        || c.is_alphanumeric()
                        || c.is_whitespace()
                        || c.is_ascii_punctuation()
                })
            {
                return Ok(Some(trimmed.to_string()));
            }
        }

        Ok(None)
    }

    /// Reconcile native metadata into XMP
    /// Only adds properties that don't already exist in XMP
    pub fn reconcile_to_xmp(xmp: &mut XmpMeta, native: &Vec<NativeMetadataItem>) {
        use crate::core::namespace::ns;

        for item in native {
            match item {
                NativeMetadataItem::Exif(exif_fields) => {
                    // Map Exif fields to XMP properties
                    if let Some(datetime) = &exif_fields.datetime_original {
                        if xmp.get_property(ns::XMP, "CreateDate").is_none() {
                            let _ =
                                xmp.set_property(ns::XMP, "CreateDate", datetime.clone().into());
                        }
                    }

                    if let Some(make) = &exif_fields.make {
                        if xmp.get_property(ns::TIFF, "Make").is_none() {
                            let _ = xmp.set_property(ns::TIFF, "Make", make.clone().into());
                        }
                    }

                    if let Some(model) = &exif_fields.model {
                        if xmp.get_property(ns::TIFF, "Model").is_none() {
                            let _ = xmp.set_property(ns::TIFF, "Model", model.clone().into());
                        }
                    }

                    if let Some(artist) = &exif_fields.artist {
                        if xmp.get_array_size(ns::DC, "creator").unwrap_or(0) == 0 {
                            let _ = xmp.append_array_item(ns::DC, "creator", artist.clone().into());
                        }
                    }

                    if let Some(copyright) = &exif_fields.copyright {
                        if xmp
                            .get_localized_text(ns::DC, "rights", "x-default", "x-default")
                            .is_none()
                        {
                            let _ = xmp.set_localized_text(
                                ns::DC,
                                "rights",
                                "x-default",
                                "x-default",
                                copyright,
                            );
                        }
                    }

                    if let Some(software) = &exif_fields.software {
                        if xmp.get_property(ns::XMP, "CreatorTool").is_none() {
                            let _ =
                                xmp.set_property(ns::XMP, "CreatorTool", software.clone().into());
                        }
                    }
                }
                NativeMetadataItem::Text { .. } => {
                    // Text metadata boxes are not commonly used in HEIF
                    // Most metadata is in Exif format
                }
            }
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

    fn make_ftyp_heic() -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&20u32.to_be_bytes()); // size
        buf.extend_from_slice(FTYP_BOX); // type
        buf.extend_from_slice(b"heic"); // major brand
        buf.extend_from_slice(&0u32.to_be_bytes()); // minor version
        buf.extend_from_slice(b"heic"); // compatible brand
        buf
    }

    /// Create a minimal HEIF file with ftyp and empty meta box
    fn create_minimal_heif() -> Vec<u8> {
        let mut buf = make_ftyp_heic();

        // Add meta box with version/flags only (no children)
        let meta_body_size = 4u32; // version/flags only
        let meta_box_size = 8 + meta_body_size; // header + body
        buf.extend_from_slice(&meta_box_size.to_be_bytes()); // box size
        buf.extend_from_slice(BOX_TYPE_META); // box type
        buf.extend_from_slice(&0u32.to_be_bytes()); // version/flags

        buf
    }

    /// Create a minimal HEIF file with ftyp and meta box containing XMP UUID box
    fn create_minimal_heif_with_xmp(xmp_data: &[u8]) -> Vec<u8> {
        let mut buf = make_ftyp_heic();

        // Build XMP UUID child box
        let mut xmp_child = Vec::new();
        xmp_child.extend_from_slice(XMP_UUID);
        xmp_child.extend_from_slice(xmp_data);

        // Build meta box body: version/flags + XMP UUID child box
        let mut meta_body = Vec::new();
        meta_body.extend_from_slice(&0u32.to_be_bytes()); // version/flags

        let xmp_child_size = (8 + xmp_child.len()) as u32;
        meta_body.extend_from_slice(&xmp_child_size.to_be_bytes()); // child box size
        meta_body.extend_from_slice(UUID_BOX); // child box type
        meta_body.extend_from_slice(&xmp_child); // child box payload

        // Build meta box
        let meta_box_size = (8 + meta_body.len()) as u32;
        buf.extend_from_slice(&meta_box_size.to_be_bytes()); // box size
        buf.extend_from_slice(BOX_TYPE_META); // box type
        buf.extend_from_slice(&meta_body); // box body

        buf
    }

    #[test]
    fn test_can_handle_heic() {
        let data = make_ftyp_heic();
        let mut cursor = Cursor::new(data);
        let handler = MpeghHandler;
        assert!(handler.can_handle(&mut cursor).unwrap());
    }

    #[test]
    fn test_read_xmp_no_xmp() {
        let heif_data = create_minimal_heif();
        let reader = Cursor::new(heif_data);
        let result = MpeghHandler::read_xmp(reader, &XmpOptions::default()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_invalid_heif() {
        let invalid_data = vec![0x00, 0x01, 0x02, 0x03];
        let reader = Cursor::new(invalid_data);
        let result = MpeghHandler::read_xmp(reader, &XmpOptions::default());
        assert!(result.is_err());
    }

    #[test]
    fn test_read_xmp_with_xmp() {
        // Create XMP packet
        let mut meta = XmpMeta::new();
        meta.set_property(ns::DC, "title", XmpValue::String("Test Image".to_string()))
            .unwrap();
        let xmp_packet = meta.serialize_packet().unwrap();
        let xmp_bytes = xmp_packet.as_bytes();

        // Create HEIF with XMP
        let heif_data = create_minimal_heif_with_xmp(xmp_bytes);
        let reader = Cursor::new(heif_data);
        let result = MpeghHandler::read_xmp(reader, &XmpOptions::default()).unwrap();
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
    fn test_write_xmp() {
        // Create minimal HEIF
        let heif_data = create_minimal_heif();
        let reader = Cursor::new(heif_data);
        let mut writer = Cursor::new(Vec::new());

        // Create XMP metadata
        let mut meta = XmpMeta::new();
        meta.set_property(ns::DC, "title", XmpValue::String("Test Image".to_string()))
            .unwrap();

        // Write XMP
        MpeghHandler::write_xmp(reader, &mut writer, &meta).unwrap();

        // Read back XMP
        writer.set_position(0);
        let result = MpeghHandler::read_xmp(writer, &XmpOptions::default()).unwrap();
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
    fn test_update_meta_replaces_uuid() {
        // meta body: version/flags + uuid(XMP) box with payload "old"
        let mut meta = Vec::new();
        meta.extend_from_slice(&0u32.to_be_bytes()); // version/flags

        let mut child = Vec::new();
        child.extend_from_slice(XMP_UUID);
        child.extend_from_slice(b"old");

        let size = (8 + child.len()) as u32;
        meta.extend_from_slice(&size.to_be_bytes());
        meta.extend_from_slice(UUID_BOX);
        meta.extend_from_slice(&child);

        let updated = MpeghHandler::update_meta_with_xmp(&meta, b"new").unwrap();
        assert!(updated.windows(b"new".len()).any(|w| w == b"new"));
        // ensure XMP UUID still present
        assert!(updated.windows(XMP_UUID.len()).any(|w| w == XMP_UUID));
    }
}
