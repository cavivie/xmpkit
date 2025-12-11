//! SVG (Scalable Vector Graphics) file format handler
//!
//! This module provides XMP metadata reading and writing support for SVG files.
//!
//! ## SVG XMP Storage
//!
//! XMP metadata in SVG files is stored within the `<metadata>` element of the root `<svg>` element:
//!
//! ```xml
//! <svg xmlns="http://www.w3.org/2000/svg" ...>
//!   <metadata>
//!     <x:xmpmeta xmlns:x="adobe:ns:meta/">
//!       <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
//!         <!-- XMP content -->
//!       </rdf:RDF>
//!     </x:xmpmeta>
//!   </metadata>
//!   <!-- SVG content -->
//! </svg>
//! ```
//!
//! According to SVG 1.1 specification, the `<metadata>` element should contain
//! metadata about the SVG content. XMP is typically wrapped in `<x:xmpmeta>` or
//! can be directly as `<rdf:RDF>`.

use std::io::{Read, Seek, SeekFrom, Write};

use quick_xml::escape::unescape;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};

use crate::core::XmpMeta;
use crate::files::handler::FileHandler;
use crate::files::handler::XmpOptions;
use crate::XmpResult;

// SVG namespace
const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";

// XMP namespace for xmpmeta wrapper
const XMP_META_NAMESPACE: &str = "adobe:ns:meta/";

/// SVG file format handler
#[derive(Debug, Default, Clone)]
pub struct SvgHandler;

impl FileHandler for SvgHandler {
    /// Check if this is a valid SVG file using quick-xml
    fn can_handle<R: Read + Seek>(&self, reader: &mut R) -> XmpResult<bool> {
        let pos = reader.stream_position()?;

        // Read first 4KB to check for SVG
        let mut buffer = vec![0u8; 4096];
        let bytes_read = match reader.read(&mut buffer) {
            Ok(n) => n,
            Err(_) => {
                reader.seek(SeekFrom::Start(pos))?;
                return Ok(false);
            }
        };

        reader.seek(SeekFrom::Start(pos))?;

        if bytes_read < 10 {
            return Ok(false);
        }

        // Convert to string
        let content = match std::str::from_utf8(&buffer[..bytes_read]) {
            Ok(s) => s,
            Err(_) => return Ok(false),
        };

        // Use quick-xml to check for SVG element
        let mut xml_reader = Reader::from_str(content);
        xml_reader.config_mut().trim_text(true);

        loop {
            match xml_reader.read_event() {
                Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                    let name = e.local_name();
                    let name_str = std::str::from_utf8(name.as_ref()).unwrap_or("");

                    // Check if it's an SVG element
                    if name_str.eq_ignore_ascii_case("svg") {
                        return Ok(true);
                    }

                    // Check for SVG namespace in attributes
                    for attr in e.attributes().flatten() {
                        if let Ok(value) = attr.unescape_value() {
                            if value.as_ref() == SVG_NAMESPACE {
                                return Ok(true);
                            }
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }

        Ok(false)
    }

    fn read_xmp<R: Read + Seek>(
        &self,
        reader: &mut R,
        _options: &XmpOptions,
    ) -> XmpResult<Option<XmpMeta>> {
        reader.rewind()?;

        // Read entire file
        let mut content = String::new();
        reader.read_to_string(&mut content)?;

        // Parse with quick-xml to find metadata
        let mut xml_reader = Reader::from_str(&content);
        xml_reader.config_mut().trim_text(true);

        let mut in_metadata = false;
        let mut metadata_depth = 0;
        let mut xmp_content = String::new();
        let mut capture_xmp = false;
        let mut xmp_depth = 0;

        loop {
            match xml_reader.read_event() {
                Ok(Event::Start(e)) => {
                    let name = e.local_name();
                    let name_str = std::str::from_utf8(name.as_ref()).unwrap_or("");

                    if name_str == "metadata" && !in_metadata {
                        in_metadata = true;
                        metadata_depth = 1;
                    } else if in_metadata {
                        metadata_depth += 1;

                        // Check for xmpmeta or RDF
                        if name_str == "xmpmeta" || name_str == "RDF" {
                            capture_xmp = true;
                            xmp_depth = 1;
                            // Include the opening tag
                            xmp_content.push('<');
                            xmp_content.push_str(&reconstruct_element(&e));
                            xmp_content.push('>');
                        } else if capture_xmp {
                            xmp_depth += 1;
                            xmp_content.push('<');
                            xmp_content.push_str(&reconstruct_element(&e));
                            xmp_content.push('>');
                        }
                    }
                }
                Ok(Event::End(e)) => {
                    let name = e.local_name();
                    let name_str = std::str::from_utf8(name.as_ref()).unwrap_or("");

                    if capture_xmp {
                        xmp_content.push_str("</");
                        xmp_content
                            .push_str(std::str::from_utf8(e.name().as_ref()).unwrap_or(name_str));
                        xmp_content.push('>');
                        xmp_depth -= 1;

                        if xmp_depth == 0 {
                            capture_xmp = false;
                        }
                    }

                    if in_metadata {
                        metadata_depth -= 1;
                        if metadata_depth == 0 {
                            // We found the metadata, stop parsing
                            break;
                        }
                    }
                }
                Ok(Event::Empty(e)) => {
                    if capture_xmp {
                        xmp_content.push('<');
                        xmp_content.push_str(&reconstruct_element(&e));
                        xmp_content.push_str("/>");
                    }
                }
                Ok(Event::Text(e)) => {
                    if capture_xmp {
                        let raw_text = String::from_utf8_lossy(e.as_ref());
                        if let Ok(text) = unescape(&raw_text) {
                            xmp_content.push_str(&text);
                        } else {
                            xmp_content.push_str(&raw_text);
                        }
                    }
                }
                Ok(Event::CData(e)) => {
                    if capture_xmp {
                        xmp_content.push_str("<![CDATA[");
                        xmp_content.push_str(std::str::from_utf8(e.as_ref()).unwrap_or(""));
                        xmp_content.push_str("]]>");
                    }
                }
                Ok(Event::PI(e)) => {
                    if capture_xmp {
                        xmp_content.push_str("<?");
                        xmp_content.push_str(std::str::from_utf8(e.as_ref()).unwrap_or(""));
                        xmp_content.push_str("?>");
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }

        if xmp_content.is_empty() {
            return Ok(None);
        }

        // Wrap in xpacket if it's just RDF
        let xmp_to_parse = if xmp_content.contains("<?xpacket") {
            xmp_content
        } else {
            format!(
                r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
{}
<?xpacket end="w"?>"#,
                xmp_content
            )
        };

        match XmpMeta::parse(&xmp_to_parse) {
            Ok(meta) => Ok(Some(meta)),
            Err(_) => Ok(None),
        }
    }

    fn write_xmp<R: Read + Seek, W: Write + Seek>(
        &self,
        reader: &mut R,
        writer: &mut W,
        meta: &XmpMeta,
    ) -> XmpResult<()> {
        reader.rewind()?;

        // Read entire file
        let mut content = String::new();
        reader.read_to_string(&mut content)?;

        // Serialize XMP with xmpmeta wrapper
        let xmp_packet = meta.serialize_packet()?;
        let new_metadata_content = format!(
            r#"<x:xmpmeta xmlns:x="{}">
{}
</x:xmpmeta>"#,
            XMP_META_NAMESPACE, xmp_packet
        );

        // Parse and rewrite using quick-xml
        let mut xml_reader = Reader::from_str(&content);
        xml_reader.config_mut().trim_text(false); // Preserve whitespace for output

        let mut output = Vec::new();
        let mut xml_writer = Writer::new(&mut output);

        let mut in_metadata = false;
        let mut metadata_depth = 0;
        let mut wrote_metadata = false;

        loop {
            match xml_reader.read_event() {
                Ok(Event::Start(ref e)) => {
                    let name = e.local_name();
                    let name_str = std::str::from_utf8(name.as_ref()).unwrap_or("");

                    if name_str == "metadata" && !in_metadata {
                        in_metadata = true;
                        metadata_depth = 1;
                        // Write new metadata element
                        write_event(&mut xml_writer, Event::Start(BytesStart::new("metadata")))?;
                        // Write XMP content as raw text
                        write_event(
                            &mut xml_writer,
                            Event::Text(BytesText::from_escaped(&new_metadata_content)),
                        )?;
                        wrote_metadata = true;
                    } else if in_metadata {
                        metadata_depth += 1;
                        // Skip content inside old metadata
                    } else {
                        write_event(&mut xml_writer, Event::Start(e.clone()))?;
                    }
                }
                Ok(Event::End(ref e)) => {
                    let name = e.local_name();
                    let name_str = std::str::from_utf8(name.as_ref()).unwrap_or("");

                    if in_metadata {
                        metadata_depth -= 1;
                        if metadata_depth == 0 {
                            in_metadata = false;
                            // Write closing metadata tag
                            write_event(&mut xml_writer, Event::End(BytesEnd::new("metadata")))?;
                        }
                    } else {
                        // Insert metadata before </svg> if we haven't written it yet
                        if name_str.eq_ignore_ascii_case("svg") && !wrote_metadata {
                            // Write new metadata element
                            write_event(
                                &mut xml_writer,
                                Event::Text(BytesText::from_escaped("\n")),
                            )?;
                            write_event(
                                &mut xml_writer,
                                Event::Start(BytesStart::new("metadata")),
                            )?;
                            write_event(
                                &mut xml_writer,
                                Event::Text(BytesText::from_escaped(&new_metadata_content)),
                            )?;
                            write_event(&mut xml_writer, Event::End(BytesEnd::new("metadata")))?;
                            write_event(
                                &mut xml_writer,
                                Event::Text(BytesText::from_escaped("\n")),
                            )?;
                            wrote_metadata = true;
                        }
                        write_event(&mut xml_writer, Event::End(e.clone()))?;
                    }
                }
                Ok(Event::Empty(ref e)) => {
                    if !in_metadata {
                        write_event(&mut xml_writer, Event::Empty(e.clone()))?;
                    }
                }
                Ok(Event::Text(ref e)) => {
                    if !in_metadata {
                        write_event(&mut xml_writer, Event::Text(e.clone()))?;
                    }
                }
                Ok(Event::Decl(ref e)) => {
                    write_event(&mut xml_writer, Event::Decl(e.clone()))?;
                }
                Ok(Event::PI(ref e)) => {
                    if !in_metadata {
                        write_event(&mut xml_writer, Event::PI(e.clone()))?;
                    }
                }
                Ok(Event::Comment(ref e)) => {
                    if !in_metadata {
                        write_event(&mut xml_writer, Event::Comment(e.clone()))?;
                    }
                }
                Ok(Event::CData(ref e)) => {
                    if !in_metadata {
                        write_event(&mut xml_writer, Event::CData(e.clone()))?;
                    }
                }
                Ok(Event::DocType(ref e)) => {
                    write_event(&mut xml_writer, Event::DocType(e.clone()))?;
                }
                Ok(Event::Eof) => break,
                // Handle any other events (e.g., GeneralRef) - skip them
                Ok(_) => {}
                Err(e) => {
                    return Err(crate::XmpError::ParseError(format!(
                        "XML parse error: {}",
                        e
                    )));
                }
            }
        }

        writer.write_all(&output)?;

        Ok(())
    }

    fn format_name(&self) -> &'static str {
        "SVG"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["svg"]
    }
}

/// Helper to write an XML event
fn write_event<W: std::io::Write>(writer: &mut Writer<W>, event: Event) -> XmpResult<()> {
    writer
        .write_event(event)
        .map_err(|e| crate::XmpError::SerializationError(e.to_string()))
}

/// Reconstruct an XML element with its attributes
fn reconstruct_element(e: &BytesStart) -> String {
    let mut result = String::new();

    // Add element name
    result.push_str(std::str::from_utf8(e.name().as_ref()).unwrap_or(""));

    // Add attributes
    for attr in e.attributes().flatten() {
        result.push(' ');
        result.push_str(std::str::from_utf8(attr.key.as_ref()).unwrap_or(""));
        result.push_str("=\"");
        if let Ok(value) = attr.unescape_value() {
            // Escape the value for XML
            for c in value.chars() {
                match c {
                    '&' => result.push_str("&amp;"),
                    '<' => result.push_str("&lt;"),
                    '>' => result.push_str("&gt;"),
                    '"' => result.push_str("&quot;"),
                    _ => result.push(c),
                }
            }
        }
        result.push('"');
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn create_test_svg() -> String {
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
  <rect x="10" y="10" width="80" height="80" fill="blue"/>
</svg>"#
            .to_string()
    }

    fn create_test_svg_with_xmp() -> String {
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
<metadata>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:dc="http://purl.org/dc/elements/1.1/">
  <rdf:Description rdf:about="">
    <dc:title>Test SVG</dc:title>
  </rdf:Description>
</rdf:RDF>
<?xpacket end="w"?>
</x:xmpmeta>
</metadata>
  <rect x="10" y="10" width="80" height="80" fill="blue"/>
</svg>"#
            .to_string()
    }

    #[test]
    fn test_can_handle_valid_svg() {
        let handler = SvgHandler::default();
        let svg = create_test_svg();
        let mut cursor = Cursor::new(svg.as_bytes());

        assert!(handler.can_handle(&mut cursor).unwrap());
    }

    #[test]
    fn test_can_handle_svg_with_xmp() {
        let handler = SvgHandler::default();
        let svg = create_test_svg_with_xmp();
        let mut cursor = Cursor::new(svg.as_bytes());

        assert!(handler.can_handle(&mut cursor).unwrap());
    }

    #[test]
    fn test_can_handle_invalid() {
        let handler = SvgHandler::default();

        // Not SVG - HTML
        let mut cursor = Cursor::new(b"<html><body>Hello</body></html>");
        assert!(!handler.can_handle(&mut cursor).unwrap());

        // Binary data
        let mut cursor = Cursor::new(vec![0x89, 0x50, 0x4E, 0x47]);
        assert!(!handler.can_handle(&mut cursor).unwrap());

        // Too short
        let mut cursor = Cursor::new(b"<svg");
        assert!(!handler.can_handle(&mut cursor).unwrap());
    }

    #[test]
    fn test_read_xmp_no_xmp() {
        let handler = SvgHandler::default();
        let svg = create_test_svg();
        let mut cursor = Cursor::new(svg.as_bytes());

        let result = handler.read_xmp(&mut cursor, &XmpOptions::default());
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_read_xmp_with_xmp() {
        let handler = SvgHandler::default();
        let svg = create_test_svg_with_xmp();
        let mut cursor = Cursor::new(svg.as_bytes());

        let result = handler.read_xmp(&mut cursor, &XmpOptions::default());
        assert!(result.is_ok());
        let meta = result.unwrap();
        assert!(meta.is_some());
    }

    #[test]
    fn test_write_xmp_new() {
        let handler = SvgHandler::default();
        let svg = create_test_svg();
        let mut reader = Cursor::new(svg.as_bytes());
        let mut writer = Cursor::new(Vec::new());

        // Create XMP metadata
        let mut meta = XmpMeta::new();
        meta.set_property("http://purl.org/dc/elements/1.1/", "title", "My SVG".into())
            .unwrap();

        // Write XMP
        let result = handler.write_xmp(&mut reader, &mut writer, &meta);
        assert!(result.is_ok());

        // Verify output contains metadata
        let output = String::from_utf8(writer.into_inner()).unwrap();
        assert!(output.contains("<metadata>"));
        assert!(output.contains("x:xmpmeta"));
    }

    #[test]
    fn test_write_xmp_replace() {
        let handler = SvgHandler::default();
        let svg = create_test_svg_with_xmp();
        let mut reader = Cursor::new(svg.as_bytes());
        let mut writer = Cursor::new(Vec::new());

        // Create new XMP metadata
        let mut meta = XmpMeta::new();
        meta.set_property(
            "http://purl.org/dc/elements/1.1/",
            "title",
            "Updated SVG".into(),
        )
        .unwrap();

        // Write XMP
        let result = handler.write_xmp(&mut reader, &mut writer, &meta);
        assert!(result.is_ok());

        // Read back and verify
        let written_data = writer.into_inner();
        let mut read_cursor = Cursor::new(&written_data);
        let read_result = handler.read_xmp(&mut read_cursor, &XmpOptions::default());
        assert!(read_result.is_ok());
        assert!(read_result.unwrap().is_some());
    }

    #[test]
    fn test_format_info() {
        let handler = SvgHandler::default();
        assert_eq!(handler.format_name(), "SVG");
        assert!(handler.extensions().contains(&"svg"));
    }

    #[test]
    fn test_handles_comments() {
        let handler = SvgHandler::default();
        let svg = r#"<?xml version="1.0"?>
<!-- This is a comment -->
<svg xmlns="http://www.w3.org/2000/svg">
  <!-- Another comment with <metadata> fake tag -->
  <rect/>
</svg>"#;
        let mut cursor = Cursor::new(svg.as_bytes());

        // Should handle and not be confused by comments
        assert!(handler.can_handle(&mut cursor).unwrap());

        cursor.rewind().unwrap();
        let result = handler.read_xmp(&mut cursor, &XmpOptions::default());
        assert!(result.is_ok());
        assert!(result.unwrap().is_none()); // No real metadata
    }

    #[test]
    fn test_handles_cdata() {
        let handler = SvgHandler::default();
        let svg = r#"<?xml version="1.0"?>
<svg xmlns="http://www.w3.org/2000/svg">
  <script><![CDATA[
    // Some JavaScript with <metadata> text
  ]]></script>
</svg>"#;
        let mut cursor = Cursor::new(svg.as_bytes());

        assert!(handler.can_handle(&mut cursor).unwrap());

        cursor.rewind().unwrap();
        let result = handler.read_xmp(&mut cursor, &XmpOptions::default());
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}
