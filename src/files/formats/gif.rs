//! GIF file format handler
//!
//! This module provides functionality for reading and writing XMP metadata
//! in GIF files. The implementation is pure Rust and cross-platform compatible.
//!
//! GIF XMP Storage:
//! - XMP Packet is stored in an Application Extension Block
//! - Application Extension identifier: "XMP DataXMP\0"
//! - The XMP data follows the identifier in the extension data

use crate::core::error::{XmpError, XmpResult};
use crate::core::metadata::XmpMeta;
use crate::files::handler::FileHandler;
use std::io::{Read, Seek, SeekFrom, Write};

/// GIF file signature
const GIF_SIGNATURE_87A: &[u8] = b"GIF87a";
const GIF_SIGNATURE_89A: &[u8] = b"GIF89a";

/// Application Extension block type
const EXTENSION_INTRODUCER: u8 = 0x21;
const APPLICATION_EXTENSION_LABEL: u8 = 0xFF;

/// XMP Application Extension identifier (11 bytes, no null terminator)
/// Value: "XMP DataXMP"
const XMP_APP_IDENTIFIER: &[u8] = b"XMP DataXMP";

/// GIF file handler for XMP metadata
#[derive(Debug, Clone, Copy)]
pub struct GifHandler;

/// Result of handling an extension block
enum ExtensionResult {
    FoundXmp { offset: u64, length: u64 },
    Skipped,
}

impl FileHandler for GifHandler {
    fn can_handle<R: Read + Seek>(&self, reader: &mut R) -> XmpResult<bool> {
        let pos = reader.stream_position()?;
        let mut header = [0u8; 6];
        match reader.read_exact(&mut header) {
            Ok(_) => {
                reader.seek(SeekFrom::Start(pos))?;
                Ok(header == *GIF_SIGNATURE_87A || header == *GIF_SIGNATURE_89A)
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
        Self::write_xmp(reader, writer, meta)
    }

    fn format_name(&self) -> &'static str {
        "GIF"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["gif"]
    }
}

impl GifHandler {
    /// Read XMP metadata from a GIF file
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
        // Check GIF signature
        let mut signature = [0u8; 6];
        reader.read_exact(&mut signature)?;

        if signature != *GIF_SIGNATURE_87A && signature != *GIF_SIGNATURE_89A {
            return Err(XmpError::BadValue("Not a valid GIF file".to_string()));
        }

        // Read Logical Screen Descriptor (7 bytes after signature)
        // 2 bytes Screen Width + 2 bytes Screen Height = 4 bytes
        reader.seek(SeekFrom::Current(4))?;

        // 1 byte Packed Fields
        let mut packed_fields = [0u8; 1];
        reader.read_exact(&mut packed_fields)?;

        // 1 byte Background Color Index + 1 byte Pixel Aspect Ratio = 2 bytes
        reader.seek(SeekFrom::Current(2))?;

        // Check if Global Color Table exists (bit 7 of packed fields)
        if (packed_fields[0] & 0x80) != 0 {
            // Global Color Table exists, skip it
            let table_size = 2 << ((packed_fields[0] & 0x07) as usize);
            reader.seek(SeekFrom::Current((table_size * 3) as i64))?;
        }

        // Process blocks until we find XMP Application Extension
        loop {
            let mut block_type = [0u8; 1];
            match reader.read_exact(&mut block_type) {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    // End of file, no XMP found
                    return Ok(None);
                }
                Err(e) => return Err(e.into()),
            }

            match block_type[0] {
                EXTENSION_INTRODUCER => {
                    match Self::handle_extension_block(&mut reader)? {
                        ExtensionResult::FoundXmp { offset, .. } => {
                            // XMP found - seek back to packet data start and parse
                            reader.seek(SeekFrom::Start(offset))?;
                            return Ok(Some(XmpMeta::parse(&Self::read_xmp_from_extension(
                                &mut reader,
                            )?)?));
                        }
                        ExtensionResult::Skipped => {
                            // Extension was skipped, continue to next block
                        }
                    }
                }
                0x2C => {
                    // Image Separator - skip image data
                    Self::skip_image_data(&mut reader)?;
                }
                0x3B => {
                    // Trailer - end of file
                    return Ok(None);
                }
                _ => {
                    // Unknown block type, try to skip
                    Self::skip_unknown_block(&mut reader)?;
                }
            }
        }
    }

    /// Skip image data
    fn skip_image_data<R: Read + Seek>(reader: &mut R) -> XmpResult<()> {
        // Skip Image Descriptor dimensions (8 bytes)
        reader.seek(SeekFrom::Current(8))?;

        // Read Packed Fields (1 byte)
        let mut packed = [0u8; 1];
        reader.read_exact(&mut packed)?;

        // Skip Local Color Table if present
        if (packed[0] & 0x80) != 0 {
            let table_size = 2 << ((packed[0] & 0x07) as usize);
            reader.seek(SeekFrom::Current((table_size * 3) as i64))?;
        }

        // Skip LZW Minimum code size (1 byte)
        reader.seek(SeekFrom::Current(1))?;

        // Skip image data sub-blocks until terminator
        loop {
            let mut sub_block_size = [0u8; 1];
            reader.read_exact(&mut sub_block_size)?;

            if sub_block_size[0] == 0 {
                break;
            }

            reader.seek(SeekFrom::Current(sub_block_size[0] as i64))?;
        }
        Ok(())
    }

    /// Skip unknown block
    fn skip_unknown_block<R: Read + Seek>(reader: &mut R) -> XmpResult<()> {
        // Try to skip by reading until we find a known marker
        let mut buffer = [0u8; 1];
        while reader.read_exact(&mut buffer).is_ok() {
            if buffer[0] == EXTENSION_INTRODUCER || buffer[0] == 0x2C || buffer[0] == 0x3B {
                // Found a known marker, seek back one byte
                reader.seek(SeekFrom::Current(-1))?;
                break;
            }
        }
        Ok(())
    }

    /// Read XMP packet from Application Extension
    ///
    /// Implementation logic:
    /// 1. Record offset after APP_ID (XMPPacketOffset)
    /// 2. Skip all sub-blocks to calculate total length
    /// 3. Calculate packet length = current_offset - XMPPacketOffset - MAGIC_TRAILER_LEN
    /// 4. Read packet_length bytes from XMPPacketOffset
    ///    - If first byte is '<' (0x3c): direct format, read as pure XML
    ///    - Otherwise: sub-block format (original files), parse sub-blocks to extract XML
    fn read_xmp_from_extension<R: Read + Seek>(reader: &mut R) -> XmpResult<String> {
        // Record offset after APP_ID (XMPPacketOffset)
        let xmp_packet_offset = reader.stream_position()?;

        // Peek at first byte to determine format
        let mut first_byte = [0u8; 1];
        reader.read_exact(&mut first_byte)?;
        reader.seek(SeekFrom::Start(xmp_packet_offset))?; // Reset

        // Skip all sub-blocks to calculate total length (like C++ does)
        loop {
            let mut sub_block_size = [0u8; 1];
            reader.read_exact(&mut sub_block_size)?;

            if sub_block_size[0] == 0 {
                break;
            }

            // Skip sub-block data
            reader.seek(SeekFrom::Current(sub_block_size[0] as i64))?;
        }

        // Calculate packet length (excluding magic trailer)
        // C++: XMPPacketLength = fileRef->Offset() - XMPPacketOffset - MAGIC_TRAILER_LEN
        const MAGIC_TRAILER_LEN: u64 = 258;
        let current_offset = reader.stream_position()?;
        let packet_length = current_offset.saturating_sub(xmp_packet_offset + MAGIC_TRAILER_LEN);

        if packet_length == 0 {
            return Err(XmpError::BadValue(
                "Corrupt GIF file: packet length is zero".to_string(),
            ));
        }

        // Seek back to XMPPacketOffset and read packet_length bytes
        reader.seek(SeekFrom::Start(xmp_packet_offset))?;
        let mut raw_data = vec![0u8; packet_length as usize];
        reader.read_exact(&mut raw_data)?;

        // Check format: if first byte is '<' (0x3c), it's direct format (C++ written)
        // Otherwise, it's sub-block format (original files)
        let packet_data = if raw_data[0] == 0x3c {
            // Direct format: data is pure XML (C++ writes this way)
            raw_data
        } else {
            // Sub-block format: parse sub-blocks to extract pure XML
            let mut packet_data = Vec::new();
            let mut offset = 0;
            while offset < raw_data.len() {
                if raw_data[offset] == 0 {
                    break;
                }
                let block_size = raw_data[offset] as usize;
                offset += 1; // Skip size byte

                if offset + block_size > raw_data.len() {
                    break;
                }

                packet_data.extend_from_slice(&raw_data[offset..offset + block_size]);
                offset += block_size;
            }
            packet_data
        };

        // Convert to UTF-8 string
        String::from_utf8(packet_data)
            .map_err(|e| XmpError::ParseError(format!("Invalid UTF-8 in XMP packet: {}", e)))
    }

    /// Write XMP metadata to a GIF file
    ///
    /// Two cases:
    /// - If XMP exists: Copy file up to XMP packet start, write new XMP packet data,
    ///   skip old XMP packet, copy rest of file
    /// - If no XMP: Copy file up to trailer, write complete XMP Application Extension, copy rest
    pub fn write_xmp<R: Read + Seek, W: Write + Seek>(
        mut reader: R,
        mut writer: W,
        meta: &XmpMeta,
    ) -> XmpResult<()> {
        let xmp_packet = meta.serialize_packet()?;
        let xmp_bytes = xmp_packet.as_bytes();

        // Find XMP packet offset/length or trailer offset
        let (xmp_packet_offset, xmp_packet_length, trailer_offset) =
            Self::find_xmp_or_trailer_offset(&mut reader)?;

        reader.rewind()?;

        if let Some(xmp_offset) = xmp_packet_offset {
            // Case 1: XMP already exists - replace it
            // Copy file up to XMP packet data start (after APP_ID)
            Self::copy_bytes(&mut reader, &mut writer, xmp_offset)?;

            // Write new XMP packet data + magic trailer
            // Note: xmp_offset points to packet data start (after APP_ID),
            // so we only write packet data + magic trailer, not the extension header
            Self::write_xmp_packet_data(&mut writer, xmp_bytes)?;

            // Skip old XMP packet (data + magic trailer)
            if let Some(old_length) = xmp_packet_length {
                const MAGIC_TRAILER_LEN: u64 = 258;
                reader.seek(SeekFrom::Current((old_length + MAGIC_TRAILER_LEN) as i64))?;
            }

            // Copy rest of file
            let current_pos = reader.stream_position()?;
            let file_end = reader.seek(SeekFrom::End(0))?;
            reader.seek(SeekFrom::Start(current_pos))?;
            Self::copy_bytes(&mut reader, &mut writer, file_end - current_pos)?;
        } else if let Some(trailer_pos) = trailer_offset {
            // Case 2: No XMP exists - insert before trailer
            // Copy file up to trailer position
            Self::copy_bytes(&mut reader, &mut writer, trailer_pos)?;

            // Write complete XMP Application Extension
            Self::write_xmp_application_extension(&mut writer, xmp_bytes)?;

            // Copy rest of file (trailer and beyond)
            let current_pos = reader.stream_position()?;
            let file_end = reader.seek(SeekFrom::End(0))?;
            reader.seek(SeekFrom::Start(current_pos))?;
            Self::copy_bytes(&mut reader, &mut writer, file_end - current_pos)?;
        } else {
            return Err(XmpError::BadValue(
                "Not able to write XMP packet in GIF file".to_string(),
            ));
        }

        Ok(())
    }

    /// Find XMP packet offset/length or trailer offset
    ///
    /// Returns: (xmp_packet_offset, xmp_packet_length, trailer_offset)
    /// - xmp_packet_offset: Position after APP_ID where XMP packet data starts
    /// - xmp_packet_length: Length of XMP packet data excluding magic trailer
    /// - trailer_offset: Position of trailer byte
    fn find_xmp_or_trailer_offset<R: Read + Seek>(
        reader: &mut R,
    ) -> XmpResult<(Option<u64>, Option<u64>, Option<u64>)> {
        reader.rewind()?;

        // Validate GIF signature and skip header
        Self::skip_gif_header(reader)?;

        let mut xmp_packet_offset = None;
        let mut xmp_packet_length = None;
        let mut trailer_offset = None;

        // Parse GIF blocks to find XMP or trailer
        loop {
            if Self::is_at_end(reader)? {
                break;
            }

            let block_type = Self::read_block_type(reader)?;
            match block_type {
                Some(0x2C) => {
                    // Image Separator - skip image data
                    Self::skip_image_data(reader)?;
                }
                Some(EXTENSION_INTRODUCER) => {
                    match Self::handle_extension_block(reader)? {
                        ExtensionResult::FoundXmp { offset, length } => {
                            xmp_packet_offset = Some(offset);
                            xmp_packet_length = Some(length);
                        }
                        ExtensionResult::Skipped => {
                            // Extension was skipped, continue to next block
                        }
                    }
                }
                Some(0x3B) => {
                    // Trailer - record offset
                    trailer_offset = Some(reader.stream_position()? - 1);
                    break;
                }
                Some(bt) => {
                    return Err(XmpError::BadValue(format!(
                        "Invalid GIF block type: 0x{:02X}",
                        bt
                    )));
                }
                None => break, // EOF
            }
        }

        Ok((xmp_packet_offset, xmp_packet_length, trailer_offset))
    }

    /// Skip GIF header (signature + Logical Screen Descriptor + Global Color Table if present)
    fn skip_gif_header<R: Read + Seek>(reader: &mut R) -> XmpResult<()> {
        // Check GIF signature
        let mut signature = [0u8; 6];
        reader.read_exact(&mut signature)?;
        if signature != *GIF_SIGNATURE_87A && signature != *GIF_SIGNATURE_89A {
            return Err(XmpError::BadValue("Not a valid GIF file".to_string()));
        }

        // Read Logical Screen Descriptor
        let mut lsd = [0u8; 7];
        reader.read_exact(&mut lsd)?;

        // Skip Global Color Table if present
        let packed = lsd[4];
        if (packed & 0x80) != 0 {
            let table_size = 2 << ((packed & 0x07) as usize);
            reader.seek(SeekFrom::Current((table_size * 3) as i64))?;
        }

        Ok(())
    }

    /// Check if reader is at end of file
    fn is_at_end<R: Read + Seek>(reader: &mut R) -> XmpResult<bool> {
        let current_pos = reader.stream_position()?;
        let file_end = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(current_pos))?;
        Ok(current_pos >= file_end)
    }

    /// Read block type byte
    fn read_block_type<R: Read + Seek>(reader: &mut R) -> XmpResult<Option<u8>> {
        let mut block_type = [0u8; 1];
        match reader.read_exact(&mut block_type) {
            Ok(_) => Ok(Some(block_type[0])),
            Err(_) => Ok(None), // EOF
        }
    }

    /// Handle an extension block
    /// Returns FoundXmp if XMP was found, Skipped otherwise
    fn handle_extension_block<R: Read + Seek>(reader: &mut R) -> XmpResult<ExtensionResult> {
        let label = Self::read_byte(reader)?;
        let block_size = Self::read_byte(reader)?;

        // Check if it's Application Extension
        if label == APPLICATION_EXTENSION_LABEL && block_size == 11 {
            return Self::handle_application_extension(reader);
        }

        // Other extension types - skip sub-blocks
        Self::skip_extension_sub_blocks(reader, block_size)?;
        Ok(ExtensionResult::Skipped)
    }

    /// Read a single byte
    fn read_byte<R: Read>(reader: &mut R) -> XmpResult<u8> {
        let mut buf = [0u8; 1];
        reader.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    /// Handle Application Extension block
    fn handle_application_extension<R: Read + Seek>(reader: &mut R) -> XmpResult<ExtensionResult> {
        // Read APP_ID (11 bytes)
        let mut app_id = vec![0u8; 11];
        reader.read_exact(&mut app_id)?;

        if app_id == XMP_APP_IDENTIFIER {
            // Found XMP Application Extension
            let xmp_packet_offset = reader.stream_position()?;
            let xmp_end = Self::skip_sub_blocks(reader)?;

            // Calculate packet length: total length minus magic trailer
            const MAGIC_TRAILER_LEN: u64 = 258;
            let packet_length = if xmp_end >= xmp_packet_offset + MAGIC_TRAILER_LEN {
                xmp_end - xmp_packet_offset - MAGIC_TRAILER_LEN
            } else {
                0
            };

            return Ok(ExtensionResult::FoundXmp {
                offset: xmp_packet_offset,
                length: packet_length,
            });
        }

        // Other Application Extension - skip sub-blocks
        Self::skip_sub_blocks(reader)?;
        Ok(ExtensionResult::Skipped)
    }

    /// Skip extension sub-blocks
    fn skip_extension_sub_blocks<R: Read + Seek>(
        reader: &mut R,
        initial_size: u8,
    ) -> XmpResult<()> {
        let mut current_block_size = initial_size;
        while current_block_size != 0x00 {
            reader.seek(SeekFrom::Current(current_block_size as i64))?;
            current_block_size = Self::read_byte(reader).unwrap_or(0);
        }
        Ok(())
    }

    /// Skip all sub-blocks and return end position
    fn skip_sub_blocks<R: Read + Seek>(reader: &mut R) -> XmpResult<u64> {
        loop {
            let mut sub_block_size = [0u8; 1];
            if reader.read_exact(&mut sub_block_size).is_err() {
                break;
            }

            if sub_block_size[0] == 0 {
                break;
            }

            reader.seek(SeekFrom::Current(sub_block_size[0] as i64))?;
        }
        Ok(reader.stream_position()?)
    }

    /// Copy bytes from reader to writer
    fn copy_bytes<R: Read, W: Write>(reader: &mut R, writer: &mut W, count: u64) -> XmpResult<()> {
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

    /// Write XMP packet data + magic trailer (for replacing existing XMP)
    ///
    /// Writes data directly (not in sub-block format).
    /// Note: This doesn't strictly follow GIF spec (should be sub-blocks), but matches common behavior
    fn write_xmp_packet_data<W: Write>(writer: &mut W, xmp_bytes: &[u8]) -> XmpResult<()> {
        // Write XMP packet data directly
        writer.write_all(xmp_bytes)?;

        // Write magic trailer directly (258 bytes: 0x01 + 0xFF..0x00 + 0x00)
        // Format: 0x01, then 0xFF down to 0x00, then 0x00 (sub-block terminator)
        writer.write_all(&[0x01])?;
        for byte in (0x00..=0xFF).rev() {
            writer.write_all(&[byte])?;
        }

        // End of extension data (sub-block terminator)
        writer.write_all(&[0x00])?;

        Ok(())
    }

    /// Write XMP Application Extension (for inserting new XMP)
    ///
    /// Writes: Extension Introducer (0x21), Label (0xFF), APP_ID Length (11), APP_ID,
    /// XMP packet data (as sub-blocks), Magic trailer (as sub-blocks), Terminator (0x00)
    fn write_xmp_application_extension<W: Write>(
        writer: &mut W,
        xmp_bytes: &[u8],
    ) -> XmpResult<()> {
        // Extension Introducer
        writer.write_all(&[EXTENSION_INTRODUCER])?;
        // Application Extension Label
        writer.write_all(&[APPLICATION_EXTENSION_LABEL])?;
        // Application Identifier Length
        writer.write_all(&[11])?;
        // Application Identifier
        writer.write_all(XMP_APP_IDENTIFIER)?;

        // Write packet data + magic trailer
        Self::write_xmp_packet_data(writer, xmp_bytes)
    }
}
