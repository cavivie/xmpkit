//! WAV file format handler
//!
//! WAV (Waveform Audio File Format) uses RIFF container with form type "WAVE".
//! XMP is stored in a chunk with FourCC "_PMX" (reverse of "XMP_").
//!
//! WAV also contains native metadata in LIST/INFO chunks which can be
//! reconciled into XMP.
//!
//! Reference: http://www-mmsp.ece.mcgill.ca/Documents/AudioFormats/WAVE/WAVE.html

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

/// WAVE format identifier
const WAVE_SIGNATURE: &[u8; 4] = b"WAVE";

/// XMP chunk FourCC (note: reversed from WebP's "XMP ")
const XMP_CHUNK_ID: &[u8; 4] = b"_PMX";

// ============================================================================
// Handler
// ============================================================================

/// WAV file handler for XMP metadata
#[derive(Debug, Clone, Copy, Default)]
pub struct WavHandler;

impl FileHandler for WavHandler {
    fn can_handle<R: Read + Seek>(&self, reader: &mut R) -> XmpResult<bool> {
        let pos = reader.stream_position()?;

        // Check minimum file length (RIFF header + fmt chunk header)
        let file_len = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(pos))?;
        if file_len < 20 {
            return Ok(false);
        }

        // Validate RIFF header and check form type
        match validate_riff_header(reader) {
            Ok(form_type) => {
                reader.seek(SeekFrom::Start(pos))?;
                Ok(&form_type == WAVE_SIGNATURE)
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
        // Validate WAV header
        let form_type = validate_riff_header(reader)?;
        if &form_type != WAVE_SIGNATURE {
            return Err(XmpError::BadValue("Not a valid WAV file".to_string()));
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
        // Validate WAV header
        let form_type = validate_riff_header(reader)?;
        if &form_type != WAVE_SIGNATURE {
            return Err(XmpError::BadValue("Not a valid WAV file".to_string()));
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
        write_riff_header(writer, new_file_size, WAVE_SIGNATURE)?;

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
        "WAV"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["wav"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::namespace::ns;
    use crate::types::value::XmpValue;
    use std::io::Cursor;

    /// Format chunk (required in WAV)
    const FMT_CHUNK_ID: &[u8; 4] = b"fmt ";

    /// Data chunk
    const DATA_CHUNK_ID: &[u8; 4] = b"data";

    /// Create a minimal valid WAV file
    fn create_minimal_wav() -> Vec<u8> {
        let mut wav = Vec::new();

        // RIFF header
        wav.extend_from_slice(b"RIFF");

        // fmt chunk (minimal: 16 bytes for PCM)
        let fmt_data: Vec<u8> = vec![
            0x01, 0x00, // Audio format: PCM
            0x01, 0x00, // Channels: 1 (mono)
            0x44, 0xAC, 0x00, 0x00, // Sample rate: 44100
            0x88, 0x58, 0x01, 0x00, // Byte rate: 88200
            0x02, 0x00, // Block align: 2
            0x10, 0x00, // Bits per sample: 16
        ];

        // data chunk (empty for testing)
        let data_chunk: Vec<u8> = vec![];

        // Calculate file size
        let file_size = 4 + 8 + fmt_data.len() + 8 + data_chunk.len();
        wav.extend_from_slice(&(file_size as u32).to_le_bytes());

        // WAVE signature
        wav.extend_from_slice(WAVE_SIGNATURE);

        // fmt chunk
        wav.extend_from_slice(FMT_CHUNK_ID);
        wav.extend_from_slice(&(fmt_data.len() as u32).to_le_bytes());
        wav.extend_from_slice(&fmt_data);

        // data chunk
        wav.extend_from_slice(DATA_CHUNK_ID);
        wav.extend_from_slice(&(data_chunk.len() as u32).to_le_bytes());

        wav
    }

    /// Create a WAV file with INFO metadata
    fn create_wav_with_info() -> Vec<u8> {
        let mut wav = Vec::new();

        wav.extend_from_slice(b"RIFF");

        // fmt chunk
        let fmt_data: Vec<u8> = vec![
            0x01, 0x00, 0x01, 0x00, 0x44, 0xAC, 0x00, 0x00, 0x88, 0x58, 0x01, 0x00, 0x02, 0x00,
            0x10, 0x00,
        ];

        // LIST/INFO chunk
        let mut info_data = Vec::new();
        info_data.extend_from_slice(b"INFO");
        // INAM (title)
        info_data.extend_from_slice(b"INAM");
        let title = b"Test Title\0";
        info_data.extend_from_slice(&(title.len() as u32).to_le_bytes());
        info_data.extend_from_slice(title);
        if title.len() % 2 == 1 {
            info_data.push(0);
        }
        // IART (artist)
        info_data.extend_from_slice(b"IART");
        let artist = b"Test Artist\0";
        info_data.extend_from_slice(&(artist.len() as u32).to_le_bytes());
        info_data.extend_from_slice(artist);

        // data chunk
        let data_chunk: Vec<u8> = vec![];

        // Calculate file size
        let file_size = 4 + 8 + fmt_data.len() + 8 + info_data.len() + 8 + data_chunk.len();
        wav.extend_from_slice(&(file_size as u32).to_le_bytes());

        wav.extend_from_slice(WAVE_SIGNATURE);

        // fmt chunk
        wav.extend_from_slice(FMT_CHUNK_ID);
        wav.extend_from_slice(&(fmt_data.len() as u32).to_le_bytes());
        wav.extend_from_slice(&fmt_data);

        // LIST chunk
        wav.extend_from_slice(LIST_CHUNK_ID);
        wav.extend_from_slice(&(info_data.len() as u32).to_le_bytes());
        wav.extend_from_slice(&info_data);

        // data chunk
        wav.extend_from_slice(DATA_CHUNK_ID);
        wav.extend_from_slice(&(data_chunk.len() as u32).to_le_bytes());

        wav
    }

    #[test]
    fn test_can_handle_wav() {
        let handler = WavHandler;
        let wav_data = create_minimal_wav();
        let mut reader = Cursor::new(wav_data);
        assert!(handler.can_handle(&mut reader).unwrap());
    }

    #[test]
    fn test_can_handle_non_wav() {
        let handler = WavHandler;
        let non_wav_data = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
        let mut reader = Cursor::new(non_wav_data);
        assert!(!handler.can_handle(&mut reader).unwrap());
    }

    #[test]
    fn test_read_xmp_no_xmp() {
        let handler = WavHandler;
        let wav_data = create_minimal_wav();
        let mut reader = Cursor::new(wav_data);
        let result = handler
            .read_xmp(&mut reader, &XmpOptions::default())
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_read_info_reconcile() {
        let handler = WavHandler;
        let wav_data = create_wav_with_info();
        let mut reader = Cursor::new(wav_data);
        let result = handler
            .read_xmp(&mut reader, &XmpOptions::default())
            .unwrap();

        // INFO reconciliation should work
        assert!(result.is_some(), "Should have XMP from INFO reconciliation");
    }

    #[test]
    fn test_write_and_read_xmp() {
        let handler = WavHandler;
        let wav_data = create_minimal_wav();
        let mut reader = Cursor::new(wav_data);
        let mut writer = Cursor::new(Vec::new());

        let mut meta = XmpMeta::new();
        meta.set_property(ns::DC, "title", XmpValue::String("Test WAV".to_string()))
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
        let handler = WavHandler;
        assert_eq!(handler.format_name(), "WAV");
        assert_eq!(handler.extensions(), &["wav"]);
    }
}
