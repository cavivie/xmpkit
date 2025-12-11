//! AVI file format handler
//!
//! AVI (Audio Video Interleave) uses RIFF container with form type "AVI ".
//! XMP is stored in a chunk with FourCC "_PMX" (same as WAV).
//!
//! AVI also contains native metadata in LIST/INFO chunks which can be
//! reconciled into XMP.
//!
//! Reference: https://docs.microsoft.com/en-us/windows/win32/directshow/avi-riff-file-reference

use super::{
    chunk_total_size, copy_chunk, info, read_all_chunks, validate_riff_header, write_chunk,
    write_riff_header, CHUNK_HEADER_SIZE, LIST_CHUNK_ID,
};
use crate::core::error::{XmpError, XmpResult};
use crate::core::metadata::XmpMeta;
use crate::files::handler::{FileHandler, XmpOptions};
use std::io::{Read, Seek, SeekFrom, Write};

// ============================================================================
// Constants
// ============================================================================

/// AVI format identifier (note the trailing space)
const AVI_SIGNATURE: &[u8; 4] = b"AVI ";

/// XMP chunk FourCC (same as WAV)
const XMP_CHUNK_ID: &[u8; 4] = b"_PMX";

// ============================================================================
// Handler
// ============================================================================

/// AVI file handler for XMP metadata
#[derive(Debug, Clone, Copy, Default)]
pub struct AviHandler;

impl FileHandler for AviHandler {
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
                Ok(&form_type == AVI_SIGNATURE)
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
        options: &XmpOptions,
    ) -> XmpResult<Option<XmpMeta>> {
        // Validate AVI header
        let form_type = validate_riff_header(reader)?;
        if &form_type != AVI_SIGNATURE {
            return Err(XmpError::BadValue("Not a valid AVI file".to_string()));
        }

        // Read all chunks
        let chunks = read_all_chunks(reader)?;

        // Find and read XMP chunk
        let mut meta = None;
        if let Some(xmp_chunk) = chunks.iter().find(|c| c.id == *XMP_CHUNK_ID) {
            reader.seek(SeekFrom::Start(xmp_chunk.offset + CHUNK_HEADER_SIZE))?;
            let mut xmp_data = vec![0u8; xmp_chunk.size as usize];
            reader.read_exact(&mut xmp_data)?;

            let xmp_str = String::from_utf8(xmp_data)
                .map_err(|e| XmpError::ParseError(format!("Invalid UTF-8 in XMP: {}", e)))?;

            meta = Some(XmpMeta::parse(&xmp_str)?);
        }

        // If only_xmp is set, skip reconciliation
        if options.only_xmp {
            return Ok(meta);
        }

        // Read and reconcile INFO metadata
        let had_xmp = meta.is_some();
        let mut xmp_meta = meta.unwrap_or_else(XmpMeta::new);
        let mut reconciled = false;

        // Find LIST/INFO chunk
        for chunk in &chunks {
            if chunk.id == *LIST_CHUNK_ID {
                let info_items = info::read_info_list(reader, chunk)?;
                if !info_items.is_empty() {
                    info::reconcile_to_xmp(&mut xmp_meta, &info_items);
                    reconciled = true;
                }
            }
        }

        if !had_xmp && !reconciled {
            Ok(None)
        } else {
            Ok(Some(xmp_meta))
        }
    }

    fn write_xmp<R: Read + Seek, W: Write + Seek>(
        &self,
        reader: &mut R,
        writer: &mut W,
        meta: &XmpMeta,
    ) -> XmpResult<()> {
        // Validate AVI header
        let form_type = validate_riff_header(reader)?;
        if &form_type != AVI_SIGNATURE {
            return Err(XmpError::BadValue("Not a valid AVI file".to_string()));
        }

        // Serialize XMP metadata
        let xmp_packet = meta.serialize_packet()?;
        let xmp_bytes = xmp_packet.as_bytes();

        // Read all chunks
        let chunks = read_all_chunks(reader)?;

        // Find existing XMP chunk
        let xmp_chunk = chunks.iter().find(|c| c.id == *XMP_CHUNK_ID);

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
            old_file_size + new_xmp_size as u32
        };

        // Write new RIFF header
        write_riff_header(writer, new_file_size, AVI_SIGNATURE)?;

        // Copy chunks, replacing or appending XMP
        let mut xmp_written = false;

        for chunk in &chunks {
            if chunk.id == *XMP_CHUNK_ID {
                // Replace with new XMP
                write_chunk(writer, XMP_CHUNK_ID, xmp_bytes)?;
                xmp_written = true;
                continue;
            }

            // Copy chunk as-is
            copy_chunk(reader, writer, chunk)?;
        }

        // Append XMP if not already written
        if !xmp_written {
            write_chunk(writer, XMP_CHUNK_ID, xmp_bytes)?;
        }

        Ok(())
    }

    fn format_name(&self) -> &'static str {
        "AVI"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["avi"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::namespace::ns;
    use crate::types::value::XmpValue;
    use std::io::Cursor;

    /// Create a minimal valid AVI file
    fn create_minimal_avi() -> Vec<u8> {
        let mut avi = Vec::new();

        // RIFF header
        avi.extend_from_slice(b"RIFF");

        // hdrl list (minimal AVI header)
        let mut hdrl_data = Vec::new();
        hdrl_data.extend_from_slice(b"hdrl");
        // avih chunk (main AVI header, 56 bytes)
        hdrl_data.extend_from_slice(b"avih");
        let avih_data = [0u8; 56];
        hdrl_data.extend_from_slice(&(avih_data.len() as u32).to_le_bytes());
        hdrl_data.extend_from_slice(&avih_data);

        // movi list (movie data, empty for testing)
        let mut movi_data = Vec::new();
        movi_data.extend_from_slice(b"movi");

        // Calculate file size
        let file_size = 4 + 8 + hdrl_data.len() + 8 + movi_data.len();
        avi.extend_from_slice(&(file_size as u32).to_le_bytes());

        // AVI signature
        avi.extend_from_slice(AVI_SIGNATURE);

        // hdrl LIST
        avi.extend_from_slice(LIST_CHUNK_ID);
        avi.extend_from_slice(&(hdrl_data.len() as u32).to_le_bytes());
        avi.extend_from_slice(&hdrl_data);

        // movi LIST
        avi.extend_from_slice(LIST_CHUNK_ID);
        avi.extend_from_slice(&(movi_data.len() as u32).to_le_bytes());
        avi.extend_from_slice(&movi_data);

        avi
    }

    #[test]
    fn test_can_handle_avi() {
        let handler = AviHandler;
        let avi_data = create_minimal_avi();
        let mut reader = Cursor::new(avi_data);
        assert!(handler.can_handle(&mut reader).unwrap());
    }

    #[test]
    fn test_can_handle_non_avi() {
        let handler = AviHandler;
        let non_avi_data = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
        let mut reader = Cursor::new(non_avi_data);
        assert!(!handler.can_handle(&mut reader).unwrap());
    }

    #[test]
    fn test_read_xmp_no_xmp() {
        let handler = AviHandler;
        let avi_data = create_minimal_avi();
        let mut reader = Cursor::new(avi_data);
        let result = handler
            .read_xmp(&mut reader, &XmpOptions::default())
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_write_and_read_xmp() {
        let handler = AviHandler;
        let avi_data = create_minimal_avi();
        let mut reader = Cursor::new(avi_data);
        let mut writer = Cursor::new(Vec::new());

        let mut meta = XmpMeta::new();
        meta.set_property(ns::DC, "title", XmpValue::String("Test AVI".to_string()))
            .unwrap();

        handler.write_xmp(&mut reader, &mut writer, &meta).unwrap();

        writer.set_position(0);
        let result = handler
            .read_xmp(&mut writer, &XmpOptions::default().only_xmp())
            .unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn test_format_info() {
        let handler = AviHandler;
        assert_eq!(handler.format_name(), "AVI");
        assert_eq!(handler.extensions(), &["avi"]);
    }
}
