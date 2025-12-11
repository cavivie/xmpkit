//! PSD (Photoshop) file format handler
//!
//! This module provides XMP metadata reading and writing support for Adobe Photoshop files.
//!
//! ## PSD File Structure
//!
//! PSD files have the following structure:
//! 1. **Header** (26 bytes):
//!    - Signature: "8BPS" (4 bytes)
//!    - Version: 1 (PSD) or 2 (PSB) (2 bytes)
//!    - Reserved: 6 bytes (zeros)
//!    - Channels: 2 bytes (big-endian)
//!    - Height: 4 bytes (big-endian)
//!    - Width: 4 bytes (big-endian)
//!    - Depth: 2 bytes (big-endian)
//!    - Color mode: 2 bytes (big-endian)
//!
//! 2. **Color Mode Data Section**:
//!    - Length: 4 bytes (big-endian)
//!    - Data: variable
//!
//! 3. **Image Resources Section**:
//!    - Length: 4 bytes (big-endian)
//!    - Resources: variable (XMP is stored here as resource ID 1060)
//!
//! 4. **Layer and Mask Information Section**
//! 5. **Image Data Section**
//!
//! ## XMP Storage
//!
//! XMP metadata is stored in the Image Resources section as a Photoshop Image Resource (PSIR)
//! with ID 1060 (0x0424). The image resource format is:
//! - Type: "8BIM" (4 bytes)
//! - ID: 2 bytes (big-endian)
//! - Name: Pascal string (length byte + chars, padded to even)
//! - Data length: 4 bytes (big-endian)
//! - Data: variable (padded to even)

use std::io::{Read, Seek, SeekFrom, Write};

use crate::core::XmpMeta;
use crate::files::handler::FileHandler;
use crate::files::handler::XmpOptions;
use crate::XmpResult;

// PSD signature
const PSD_SIGNATURE: &[u8; 4] = b"8BPS";

// Image resource signature
const PSIR_SIGNATURE: &[u8; 4] = b"8BIM";

// Image resource IDs
const PSIR_XMP: u16 = 1060;

// Header size
const PSD_HEADER_SIZE: u64 = 26;

// Minimum file size: header (26) + color mode length (4) + image resources length (4) = 34
const MIN_PSD_SIZE: u64 = 34;

// Minimum image resource size: type(4) + id(2) + name(2) + data_len(4) = 12
const MIN_PSIR_SIZE: usize = 12;

/// PSD file format handler
#[derive(Debug, Default, Clone)]
pub struct PsdHandler;

impl PsdHandler {
    /// Create a new PSD handler
    pub fn new() -> Self {
        Self
    }
}

impl FileHandler for PsdHandler {
    /// Check if this is a valid PSD file:
    /// 1. File length >= 34 bytes (header + two section lengths)
    /// 2. Check "8BPS" signature at offset 0
    /// 3. Check version is 1 (PSD) or 2 (PSB)
    fn can_handle<R: Read + Seek>(&self, reader: &mut R) -> XmpResult<bool> {
        let pos = reader.stream_position()?;

        // Check file length
        let file_len = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(pos))?;

        if file_len < MIN_PSD_SIZE {
            return Ok(false);
        }

        // Read signature and version
        let mut header = [0u8; 6];
        match reader.read_exact(&mut header) {
            Ok(_) => {
                reader.seek(SeekFrom::Start(pos))?;

                // Check signature
                if &header[0..4] != PSD_SIGNATURE {
                    return Ok(false);
                }

                // Check version (1 = PSD, 2 = PSB)
                let version = u16::from_be_bytes([header[4], header[5]]);
                Ok(version == 1 || version == 2)
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
        _options: &XmpOptions,
    ) -> XmpResult<Option<XmpMeta>> {
        reader.rewind()?;

        // Skip header
        reader.seek(SeekFrom::Start(PSD_HEADER_SIZE))?;

        // Skip color mode data section
        let color_mode_len = read_u32_be(reader)?;
        reader.seek(SeekFrom::Current(color_mode_len as i64))?;

        // Read image resources section
        let psir_len = read_u32_be(reader)?;
        if psir_len == 0 {
            return Ok(None);
        }

        let psir_start = reader.stream_position()?;
        let psir_end = psir_start + psir_len as u64;

        // Parse image resources looking for XMP (ID 1060)
        while reader.stream_position()? + MIN_PSIR_SIZE as u64 <= psir_end {
            // Read resource header
            let mut rsrc_type = [0u8; 4];
            if reader.read_exact(&mut rsrc_type).is_err() {
                break;
            }

            // Check for 8BIM signature
            if &rsrc_type != PSIR_SIGNATURE {
                // Unknown resource type, skip to next
                break;
            }

            // Read resource ID
            let rsrc_id = read_u16_be(reader)?;

            // Read Pascal string name (length byte + chars, padded to even)
            let name_len = read_u8(reader)? as u64;
            // Name is padded to make total (length byte + chars) even
            // So we skip: name_len bytes + padding to make (1 + name_len) even
            let name_padded_len = if (1 + name_len) % 2 == 0 {
                name_len
            } else {
                name_len + 1
            };
            reader.seek(SeekFrom::Current(name_padded_len as i64))?;

            // Read data length
            let data_len = read_u32_be(reader)?;
            let data_start = reader.stream_position()?;

            // Check if this is the XMP resource
            if rsrc_id == PSIR_XMP && data_len > 0 {
                // Read XMP data
                let mut xmp_data = vec![0u8; data_len as usize];
                reader.read_exact(&mut xmp_data)?;

                // Parse XMP
                let xmp_str = String::from_utf8_lossy(&xmp_data);
                match XmpMeta::parse(&xmp_str) {
                    Ok(meta) => return Ok(Some(meta)),
                    Err(_) => return Ok(None),
                }
            }

            // Skip to next resource (data is padded to even)
            let data_padded_len = if data_len % 2 == 0 {
                data_len
            } else {
                data_len + 1
            };
            let next_pos = data_start + data_padded_len as u64;
            if next_pos > psir_end {
                break;
            }
            reader.seek(SeekFrom::Start(next_pos))?;
        }

        Ok(None)
    }

    fn write_xmp<R: Read + Seek, W: Write + Seek>(
        &self,
        reader: &mut R,
        writer: &mut W,
        meta: &XmpMeta,
    ) -> XmpResult<()> {
        reader.rewind()?;

        // Serialize XMP
        let xmp_packet = meta.serialize_packet()?;
        let xmp_bytes = xmp_packet.as_bytes();

        // Read header
        let mut header = [0u8; PSD_HEADER_SIZE as usize];
        reader.read_exact(&mut header)?;
        writer.write_all(&header)?;

        // Copy color mode data section
        let color_mode_len = read_u32_be(reader)?;
        writer.write_all(&color_mode_len.to_be_bytes())?;
        if color_mode_len > 0 {
            copy_bytes(reader, writer, color_mode_len as u64)?;
        }

        // Read and process image resources section
        let psir_len = read_u32_be(reader)?;
        let psir_start = reader.stream_position()?;

        // Build new image resources
        let mut new_resources: Vec<u8> = Vec::new();
        let mut found_xmp = false;

        if psir_len > 0 {
            let psir_end = psir_start + psir_len as u64;

            // Parse existing resources
            while reader.stream_position()? + MIN_PSIR_SIZE as u64 <= psir_end {
                let rsrc_start = reader.stream_position()?;

                // Read resource header
                let mut rsrc_type = [0u8; 4];
                if reader.read_exact(&mut rsrc_type).is_err() {
                    break;
                }

                // Check for 8BIM signature
                if &rsrc_type != PSIR_SIGNATURE {
                    // Copy remaining bytes as-is
                    reader.seek(SeekFrom::Start(rsrc_start))?;
                    let remaining = psir_end - rsrc_start;
                    copy_bytes(reader, &mut new_resources, remaining)?;
                    break;
                }

                // Read resource ID
                let rsrc_id = read_u16_be(reader)?;

                // Read Pascal string name
                let name_len = read_u8(reader)?;
                let name_padded_len = if (1 + name_len as u64) % 2 == 0 {
                    name_len as u64
                } else {
                    name_len as u64 + 1
                };

                // Read name bytes
                let mut name_bytes = vec![0u8; name_padded_len as usize];
                if name_padded_len > 0 {
                    reader.read_exact(&mut name_bytes)?;
                }

                // Read data length
                let data_len = read_u32_be(reader)?;
                let data_padded_len = if data_len % 2 == 0 {
                    data_len
                } else {
                    data_len + 1
                };

                if rsrc_id == PSIR_XMP {
                    // Replace XMP resource with new data
                    found_xmp = true;
                    write_xmp_resource(&mut new_resources, xmp_bytes)?;

                    // Skip old XMP data
                    reader.seek(SeekFrom::Current(data_padded_len as i64))?;
                } else {
                    // Copy resource as-is
                    new_resources.extend_from_slice(&rsrc_type);
                    new_resources.extend_from_slice(&rsrc_id.to_be_bytes());
                    new_resources.push(name_len);
                    new_resources.extend_from_slice(&name_bytes);
                    new_resources.extend_from_slice(&data_len.to_be_bytes());

                    // Copy data
                    let mut data = vec![0u8; data_padded_len as usize];
                    reader.read_exact(&mut data)?;
                    new_resources.extend_from_slice(&data);
                }

                // Check bounds
                if reader.stream_position()? > psir_end {
                    break;
                }
            }
        }

        // Add XMP resource if not found
        if !found_xmp {
            write_xmp_resource(&mut new_resources, xmp_bytes)?;
        }

        // Write new image resources section
        writer.write_all(&(new_resources.len() as u32).to_be_bytes())?;
        writer.write_all(&new_resources)?;

        // Skip old image resources in reader
        reader.seek(SeekFrom::Start(psir_start + psir_len as u64))?;

        // Copy rest of file (layer info, image data)
        copy_to_end(reader, writer)?;

        Ok(())
    }

    fn format_name(&self) -> &'static str {
        "PSD"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["psd", "psb"]
    }
}

/// Write XMP image resource to buffer
fn write_xmp_resource(buffer: &mut Vec<u8>, xmp_data: &[u8]) -> XmpResult<()> {
    // Write 8BIM signature
    buffer.extend_from_slice(PSIR_SIGNATURE);

    // Write resource ID (1060 = XMP)
    buffer.extend_from_slice(&PSIR_XMP.to_be_bytes());

    // Write empty Pascal string name (1 byte length = 0, 1 byte padding)
    buffer.push(0); // name length
    buffer.push(0); // padding to make even

    // Write data length
    buffer.extend_from_slice(&(xmp_data.len() as u32).to_be_bytes());

    // Write XMP data
    buffer.extend_from_slice(xmp_data);

    // Pad to even if needed
    if xmp_data.len() % 2 != 0 {
        buffer.push(0);
    }

    Ok(())
}

/// Read a big-endian u32
fn read_u32_be<R: Read>(reader: &mut R) -> XmpResult<u32> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(u32::from_be_bytes(buf))
}

/// Read a big-endian u16
fn read_u16_be<R: Read>(reader: &mut R) -> XmpResult<u16> {
    let mut buf = [0u8; 2];
    reader.read_exact(&mut buf)?;
    Ok(u16::from_be_bytes(buf))
}

/// Read a single byte
fn read_u8<R: Read>(reader: &mut R) -> XmpResult<u8> {
    let mut buf = [0u8; 1];
    reader.read_exact(&mut buf)?;
    Ok(buf[0])
}

/// Copy bytes from reader to writer
fn copy_bytes<R: Read, W: Write>(reader: &mut R, writer: &mut W, len: u64) -> XmpResult<()> {
    const BUFFER_SIZE: usize = 8192;
    let mut remaining = len;
    let mut buffer = [0u8; BUFFER_SIZE];

    while remaining > 0 {
        let to_read = std::cmp::min(remaining as usize, BUFFER_SIZE);
        reader.read_exact(&mut buffer[..to_read])?;
        writer.write_all(&buffer[..to_read])?;
        remaining -= to_read as u64;
    }

    Ok(())
}

/// Copy remaining bytes from reader to writer
fn copy_to_end<R: Read, W: Write>(reader: &mut R, writer: &mut W) -> XmpResult<()> {
    const BUFFER_SIZE: usize = 8192;
    let mut buffer = [0u8; BUFFER_SIZE];

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        writer.write_all(&buffer[..bytes_read])?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// Create a minimal valid PSD file for testing
    fn create_test_psd() -> Vec<u8> {
        let mut data = Vec::new();

        // Header (26 bytes)
        data.extend_from_slice(b"8BPS"); // Signature
        data.extend_from_slice(&1u16.to_be_bytes()); // Version 1
        data.extend_from_slice(&[0u8; 6]); // Reserved
        data.extend_from_slice(&3u16.to_be_bytes()); // Channels
        data.extend_from_slice(&100u32.to_be_bytes()); // Height
        data.extend_from_slice(&100u32.to_be_bytes()); // Width
        data.extend_from_slice(&8u16.to_be_bytes()); // Depth
        data.extend_from_slice(&3u16.to_be_bytes()); // Color mode (RGB)

        // Color mode data section (empty)
        data.extend_from_slice(&0u32.to_be_bytes());

        // Image resources section (empty)
        data.extend_from_slice(&0u32.to_be_bytes());

        // Layer and mask info section (empty)
        data.extend_from_slice(&0u32.to_be_bytes());

        // Image data section (minimal)
        data.extend_from_slice(&0u16.to_be_bytes()); // Compression = raw

        data
    }

    /// Create a PSD file with XMP
    fn create_test_psd_with_xmp(xmp: &str) -> Vec<u8> {
        let mut data = Vec::new();

        // Header (26 bytes)
        data.extend_from_slice(b"8BPS"); // Signature
        data.extend_from_slice(&1u16.to_be_bytes()); // Version 1
        data.extend_from_slice(&[0u8; 6]); // Reserved
        data.extend_from_slice(&3u16.to_be_bytes()); // Channels
        data.extend_from_slice(&100u32.to_be_bytes()); // Height
        data.extend_from_slice(&100u32.to_be_bytes()); // Width
        data.extend_from_slice(&8u16.to_be_bytes()); // Depth
        data.extend_from_slice(&3u16.to_be_bytes()); // Color mode (RGB)

        // Color mode data section (empty)
        data.extend_from_slice(&0u32.to_be_bytes());

        // Build XMP image resource
        let xmp_bytes = xmp.as_bytes();
        let xmp_padded_len = if xmp_bytes.len() % 2 == 0 {
            xmp_bytes.len()
        } else {
            xmp_bytes.len() + 1
        };

        // Image resource: 8BIM(4) + ID(2) + name(2) + len(4) + data
        let rsrc_len = 4 + 2 + 2 + 4 + xmp_padded_len;
        data.extend_from_slice(&(rsrc_len as u32).to_be_bytes());

        // XMP resource
        data.extend_from_slice(b"8BIM");
        data.extend_from_slice(&PSIR_XMP.to_be_bytes());
        data.push(0); // name length
        data.push(0); // padding
        data.extend_from_slice(&(xmp_bytes.len() as u32).to_be_bytes());
        data.extend_from_slice(xmp_bytes);
        if xmp_bytes.len() % 2 != 0 {
            data.push(0);
        }

        // Layer and mask info section (empty)
        data.extend_from_slice(&0u32.to_be_bytes());

        // Image data section (minimal)
        data.extend_from_slice(&0u16.to_be_bytes());

        data
    }

    #[test]
    fn test_can_handle_valid_psd() {
        let handler = PsdHandler::new();
        let data = create_test_psd();
        let mut cursor = Cursor::new(data);

        assert!(handler.can_handle(&mut cursor).unwrap());
    }

    #[test]
    fn test_can_handle_psb() {
        let handler = PsdHandler::new();
        let mut data = create_test_psd();
        // Change version to 2 (PSB)
        data[4] = 0;
        data[5] = 2;
        let mut cursor = Cursor::new(data);

        assert!(handler.can_handle(&mut cursor).unwrap());
    }

    #[test]
    fn test_can_handle_invalid() {
        let handler = PsdHandler::new();

        // Too short
        let mut cursor = Cursor::new(vec![0u8; 10]);
        assert!(!handler.can_handle(&mut cursor).unwrap());

        // Wrong signature
        let mut cursor = Cursor::new(b"NOTPSD".to_vec());
        assert!(!handler.can_handle(&mut cursor).unwrap());

        // Wrong version
        let mut data = create_test_psd();
        data[4] = 0;
        data[5] = 3; // Invalid version
        let mut cursor = Cursor::new(data);
        assert!(!handler.can_handle(&mut cursor).unwrap());
    }

    #[test]
    fn test_read_xmp_no_xmp() {
        let handler = PsdHandler::new();
        let data = create_test_psd();
        let mut cursor = Cursor::new(data);

        let result = handler.read_xmp(&mut cursor, &XmpOptions::default());
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_read_xmp_with_xmp() {
        let handler = PsdHandler::new();
        let xmp = r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:dc="http://purl.org/dc/elements/1.1/">
  <rdf:Description rdf:about="">
    <dc:creator><rdf:Seq><rdf:li>Test Author</rdf:li></rdf:Seq></dc:creator>
  </rdf:Description>
</rdf:RDF>
<?xpacket end="w"?>"#;

        let data = create_test_psd_with_xmp(xmp);
        let mut cursor = Cursor::new(data);

        let result = handler.read_xmp(&mut cursor, &XmpOptions::default());
        assert!(result.is_ok());
        let meta = result.unwrap();
        assert!(meta.is_some());
    }

    #[test]
    fn test_write_xmp() {
        let handler = PsdHandler::new();
        let data = create_test_psd();
        let mut reader = Cursor::new(data);
        let mut writer = Cursor::new(Vec::new());

        // Create XMP metadata
        let mut meta = XmpMeta::new();
        meta.set_property(
            "http://purl.org/dc/elements/1.1/",
            "creator",
            "Test Author".into(),
        )
        .unwrap();

        // Write XMP
        let result = handler.write_xmp(&mut reader, &mut writer, &meta);
        assert!(result.is_ok());

        // Verify we can read it back
        let written_data = writer.into_inner();
        let mut read_cursor = Cursor::new(written_data);
        let read_result = handler.read_xmp(&mut read_cursor, &XmpOptions::default());
        assert!(read_result.is_ok());
        assert!(read_result.unwrap().is_some());
    }

    #[test]
    fn test_format_info() {
        let handler = PsdHandler::new();
        assert_eq!(handler.format_name(), "PSD");
        assert!(handler.extensions().contains(&"psd"));
        assert!(handler.extensions().contains(&"psb"));
    }
}
