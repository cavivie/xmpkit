//! PNG file format handler
//!
//! This module provides functionality for reading and writing XMP metadata
//! in PNG files. The implementation is pure Rust and cross-platform compatible.
//!
//! PNG XMP Storage:
//! - XMP Packet is stored in iTXt chunk with keyword "XML:com.adobe.xmp"
//! - iTXt chunk format: keyword (null-terminated) + compression flag + compression method + language tag + translated keyword + text
//! - For XMP, compression flag is 0 (uncompressed)

use crate::core::error::{XmpError, XmpResult};
use crate::core::metadata::XmpMeta;
use crate::files::handler::FileHandler;
use std::io::{Read, Seek, Write};

/// PNG file signature
const PNG_SIGNATURE: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

/// XMP keyword in iTXt chunk
const XMP_KEYWORD: &[u8] = b"XML:com.adobe.xmp\0";

/// PNG chunk type for iTXt
const CHUNK_TYPE_ITXT: &[u8] = b"iTXt";

/// PNG chunk type for IEND (end of file)
const CHUNK_TYPE_IEND: &[u8] = b"IEND";

/// PNG file handler for XMP metadata
#[derive(Debug, Clone, Copy)]
pub struct PngHandler;

impl FileHandler for PngHandler {
    fn can_handle<R: Read + Seek>(&self, reader: &mut R) -> XmpResult<bool> {
        let mut signature = [0u8; 8];
        reader.read_exact(&mut signature)?;
        reader.rewind()?;
        Ok(signature == PNG_SIGNATURE)
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
        Self::write_xmp(reader, writer, meta)
    }

    fn format_name(&self) -> &'static str {
        "PNG"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["png"]
    }
}

#[derive(Debug, Clone)]
struct PngChunk {
    length: u32,
    chunk_type: [u8; 4],
    data: Vec<u8>,
    crc: u32,
}

impl PngHandler {
    /// Read XMP metadata from a PNG file
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
        // Check PNG signature
        let mut signature = [0u8; 8];
        reader.read_exact(&mut signature)?;

        if signature != PNG_SIGNATURE {
            return Err(XmpError::BadValue("Not a valid PNG file".to_string()));
        }

        // Read chunks until we find iTXt with XMP keyword
        loop {
            let chunk = match Self::read_chunk(&mut reader) {
                Ok(chunk) => chunk,
                Err(e) if e.to_string().contains("failed to fill") => {
                    // End of file reached unexpectedly
                    break;
                }
                Err(e) => return Err(e),
            };

            if chunk.chunk_type == *CHUNK_TYPE_IEND {
                break;
            }

            if chunk.chunk_type == *CHUNK_TYPE_ITXT {
                if let Some(xmp_data) = Self::extract_xmp_from_itxt(&chunk.data)? {
                    let xmp_str = String::from_utf8(xmp_data).map_err(|e| {
                        XmpError::ParseError(format!("Invalid UTF-8 in XMP: {}", e))
                    })?;
                    return XmpMeta::parse(&xmp_str).map(Some);
                }
            }
        }

        Ok(None)
    }

    /// Write XMP metadata to a PNG file
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

        // Read and verify PNG signature
        let mut signature = [0u8; 8];
        reader.read_exact(&mut signature)?;
        writer.write_all(&signature)?;

        if signature != PNG_SIGNATURE {
            return Err(XmpError::BadValue("Not a valid PNG file".to_string()));
        }

        let mut xmp_written = false;
        let mut ihdr_written = false;

        // Process chunks
        loop {
            let chunk = Self::read_chunk(&mut reader)?;

            // Write IHDR first if we haven't written it yet
            if !ihdr_written && chunk.chunk_type == *b"IHDR" {
                writer.write_all(&chunk.length.to_be_bytes())?;
                writer.write_all(&chunk.chunk_type)?;
                writer.write_all(&chunk.data)?;
                writer.write_all(&chunk.crc.to_be_bytes())?;
                ihdr_written = true;
                continue;
            }

            // Skip old XMP iTXt chunks
            if chunk.chunk_type == *CHUNK_TYPE_ITXT && Self::is_xmp_itxt(&chunk.data) {
                // Write new XMP iTXt chunk
                if !xmp_written {
                    Self::write_xmp_itxt_chunk(&mut writer, xmp_bytes)?;
                    xmp_written = true;
                }
                continue;
            }

            // If we encounter IEND and haven't written XMP yet, write it before IEND
            if chunk.chunk_type == *CHUNK_TYPE_IEND && !xmp_written {
                Self::write_xmp_itxt_chunk(&mut writer, xmp_bytes)?;
                xmp_written = true;
            }

            // Write chunk
            writer.write_all(&chunk.length.to_be_bytes())?;
            writer.write_all(&chunk.chunk_type)?;
            writer.write_all(&chunk.data)?;
            writer.write_all(&chunk.crc.to_be_bytes())?;

            if chunk.chunk_type == *CHUNK_TYPE_IEND {
                break;
            }
        }

        Ok(())
    }

    /// Read a PNG chunk
    fn read_chunk<R: Read>(reader: &mut R) -> XmpResult<PngChunk> {
        // Read chunk length (4 bytes, big-endian)
        let mut length_bytes = [0u8; 4];
        reader.read_exact(&mut length_bytes)?;
        let length = u32::from_be_bytes(length_bytes);

        // Read chunk type (4 bytes)
        let mut chunk_type = [0u8; 4];
        reader.read_exact(&mut chunk_type)?;

        // Read chunk data
        let mut data = vec![0u8; length as usize];
        reader.read_exact(&mut data)?;

        // Read CRC (4 bytes, big-endian)
        let mut crc_bytes = [0u8; 4];
        reader.read_exact(&mut crc_bytes)?;
        let crc = u32::from_be_bytes(crc_bytes);

        Ok(PngChunk {
            length,
            chunk_type,
            data,
            crc,
        })
    }

    /// Check if an iTXt chunk contains XMP data
    fn is_xmp_itxt(data: &[u8]) -> bool {
        data.len() >= XMP_KEYWORD.len() && data[..XMP_KEYWORD.len()] == *XMP_KEYWORD
    }

    /// Extract XMP data from an iTXt chunk
    fn extract_xmp_from_itxt(data: &[u8]) -> XmpResult<Option<Vec<u8>>> {
        if !Self::is_xmp_itxt(data) {
            return Ok(None);
        }

        // iTXt format: keyword (null-terminated) + compression flag (1 byte) + compression method (1 byte) + language tag (null-terminated) + translated keyword (null-terminated) + text
        let keyword_len = XMP_KEYWORD.len();
        if data.len() < keyword_len + 2 {
            return Ok(None);
        }

        let compression_flag = data[keyword_len];
        let _compression_method = data[keyword_len + 1];

        // XMP should be uncompressed
        if compression_flag != 0 {
            return Err(XmpError::NotSupported(
                "Compressed XMP in PNG not yet supported".to_string(),
            ));
        }

        // Find the start of text data (after keyword, compression flag, compression method, language tag, translated keyword)
        let mut text_start = keyword_len + 2;

        // Skip language tag (null-terminated)
        while text_start < data.len() && data[text_start] != 0 {
            text_start += 1;
        }
        if text_start >= data.len() {
            return Ok(None);
        }
        text_start += 1; // Skip null terminator

        // Skip translated keyword (null-terminated)
        while text_start < data.len() && data[text_start] != 0 {
            text_start += 1;
        }
        if text_start >= data.len() {
            return Ok(None);
        }
        text_start += 1; // Skip null terminator

        // Extract text data
        Ok(Some(data[text_start..].to_vec()))
    }

    /// Write an XMP iTXt chunk
    fn write_xmp_itxt_chunk<W: Write>(writer: &mut W, xmp_data: &[u8]) -> XmpResult<()> {
        // Build iTXt chunk data
        let mut chunk_data = Vec::new();
        chunk_data.extend_from_slice(XMP_KEYWORD); // keyword
        chunk_data.push(0); // compression flag (0 = uncompressed)
        chunk_data.push(0); // compression method (0 = deflate/inflate, but we're uncompressed)
        chunk_data.push(0); // language tag (empty, null-terminated)
        chunk_data.push(0); // translated keyword (empty, null-terminated)
        chunk_data.extend_from_slice(xmp_data); // XMP text

        // Calculate CRC
        let mut crc_data = Vec::new();
        crc_data.extend_from_slice(CHUNK_TYPE_ITXT);
        crc_data.extend_from_slice(&chunk_data);
        let crc = Self::calculate_crc(&crc_data);

        // Write chunk length
        writer.write_all(&(chunk_data.len() as u32).to_be_bytes())?;

        // Write chunk type
        writer.write_all(CHUNK_TYPE_ITXT)?;

        // Write chunk data
        writer.write_all(&chunk_data)?;

        // Write CRC
        writer.write_all(&crc.to_be_bytes())?;

        Ok(())
    }

    /// Calculate PNG CRC-32
    ///
    /// PNG uses CRC-32 with polynomial 0xEDB88320
    fn calculate_crc(data: &[u8]) -> u32 {
        let mut crc = 0xFFFFFFFFu32;
        let table = Self::crc_table();

        for &byte in data {
            let index = ((crc ^ (byte as u32)) & 0xFF) as usize;
            crc = (crc >> 8) ^ table[index];
        }

        crc ^ 0xFFFFFFFF
    }

    /// Generate CRC-32 lookup table
    fn crc_table() -> [u32; 256] {
        let mut table = [0u32; 256];
        let polynomial = 0xEDB88320u32;

        for (i, item) in table.iter_mut().enumerate() {
            let mut crc = i as u32;
            for _ in 0..8 {
                if crc & 1 != 0 {
                    crc = (crc >> 1) ^ polynomial;
                } else {
                    crc >>= 1;
                }
            }
            *item = crc;
        }

        table
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_xmp_itxt() {
        let mut data = XMP_KEYWORD.to_vec();
        data.extend_from_slice(b"XMP data");
        assert!(PngHandler::is_xmp_itxt(&data));

        let other_data = b"Other keyword\0";
        assert!(!PngHandler::is_xmp_itxt(other_data));
    }

    #[test]
    fn test_extract_xmp_from_itxt() {
        let mut data = XMP_KEYWORD.to_vec();
        data.push(0); // compression flag
        data.push(0); // compression method
        data.push(0); // language tag (empty)
        data.push(0); // translated keyword (empty)
        data.extend_from_slice(b"<rdf:RDF>test</rdf:RDF>");

        let extracted = PngHandler::extract_xmp_from_itxt(&data).unwrap();
        assert_eq!(extracted, Some(b"<rdf:RDF>test</rdf:RDF>".to_vec()));
    }

    #[test]
    fn test_crc_calculation() {
        let data = b"IHDR";
        let crc = PngHandler::calculate_crc(data);
        // Just verify it doesn't panic and returns a value
        assert!(crc != 0 || data.is_empty());
    }
}
