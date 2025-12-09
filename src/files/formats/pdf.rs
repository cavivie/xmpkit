//! PDF file format handler
//!
//! This module provides functionality for reading and writing XMP metadata
//! in PDF files. The implementation is pure Rust and cross-platform compatible.
//!
//! PDF XMP Storage:
//! - XMP is stored in a Metadata stream object in the document catalog
//! - The XMP packet is embedded with standard markers:
//!   `<?xpacket begin="..." id="W5M0MpCehiHzreSzNTczkc9d"?>` ... `<?xpacket end="w"?>`
//!
//! Reference: Adobe XMP Specification Part 3 - Storage in Files

use crate::core::error::{XmpError, XmpResult};
use crate::core::metadata::XmpMeta;
use crate::files::handler::{FileHandler, XmpOptions};
use lopdf::{dictionary, Document, Object, Stream};
use std::io::{Read, Seek, Write};

/// PDF file signature
const PDF_SIGNATURE: &[u8] = b"%PDF-";

/// PDF file handler for XMP metadata
#[derive(Debug, Clone, Copy)]
pub struct PdfHandler;

impl FileHandler for PdfHandler {
    fn can_handle<R: Read + Seek>(&self, reader: &mut R) -> XmpResult<bool> {
        let mut header = [0u8; 5];
        if reader.read_exact(&mut header).is_err() {
            reader.rewind()?;
            return Ok(false);
        }
        reader.rewind()?;
        Ok(header == PDF_SIGNATURE)
    }

    fn read_xmp<R: Read + Seek>(
        &self,
        reader: &mut R,
        _options: &XmpOptions,
    ) -> XmpResult<Option<XmpMeta>> {
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
        "PDF"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["pdf"]
    }
}

impl PdfHandler {
    /// Read XMP metadata from a PDF file
    ///
    /// Uses lopdf to properly parse the PDF structure and extract XMP metadata
    /// from the document catalog's Metadata stream.
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
        // Load the PDF document
        let doc = Document::load_from(&mut reader).map_err(|e| {
            XmpError::IoError(std::io::Error::other(format!("Failed to load PDF: {}", e)))
        })?;

        // Get the catalog dictionary
        let catalog = doc.catalog().map_err(|e| {
            XmpError::IoError(std::io::Error::other(format!(
                "Failed to get PDF catalog: {}",
                e
            )))
        })?;

        // Look for Metadata reference in catalog
        let metadata_ref = match catalog.get(b"Metadata") {
            Ok(obj) => match obj.as_reference() {
                Ok(r) => r,
                Err(_) => return Ok(None), // Metadata exists but is not a reference
            },
            Err(_) => return Ok(None), // No Metadata in catalog
        };

        // Get the metadata stream object
        let metadata_obj = doc.get_object(metadata_ref).map_err(|e| {
            XmpError::IoError(std::io::Error::other(format!(
                "Failed to get metadata object: {}",
                e
            )))
        })?;

        // Extract the stream content
        let xmp_bytes = match metadata_obj {
            Object::Stream(ref stream) => {
                // Try to get decompressed content first, fallback to raw content
                // XMP streams are typically not compressed
                stream
                    .decompressed_content()
                    .unwrap_or_else(|_| stream.content.clone())
            }
            _ => return Ok(None), // Metadata is not a stream
        };

        // Convert to string and parse XMP
        let xmp_str = String::from_utf8(xmp_bytes)
            .map_err(|e| XmpError::ParseError(format!("Invalid UTF-8 in XMP: {}", e)))?;

        // Handle empty XMP
        if xmp_str.trim().is_empty() {
            return Ok(None);
        }

        XmpMeta::parse(&xmp_str).map(Some)
    }

    /// Write XMP metadata to a PDF file
    ///
    /// Uses lopdf to properly modify the PDF structure:
    /// - If the PDF has existing XMP metadata, it updates the metadata stream
    /// - If the PDF has no XMP metadata, it creates a new metadata stream
    ///   and adds a reference to it in the document catalog
    ///
    /// # Arguments
    ///
    /// * `reader` - A reader for the source PDF
    /// * `writer` - A writer for the output PDF
    /// * `meta` - The XMP metadata to write
    ///
    /// # Returns
    ///
    /// * `Ok(())` on success
    /// * `Err(XmpError)` if an error occurs
    pub fn write_xmp<R: Read + Seek, W: Write + Seek>(
        mut reader: R,
        mut writer: W,
        meta: &XmpMeta,
    ) -> XmpResult<()> {
        // Load the PDF document
        let mut doc = Document::load_from(&mut reader).map_err(|e| {
            XmpError::IoError(std::io::Error::other(format!("Failed to load PDF: {}", e)))
        })?;

        // Serialize XMP to packet format
        let xmp_packet = meta.serialize_packet()?;
        let xmp_bytes = xmp_packet.into_bytes();

        // Create the metadata stream
        let metadata_stream = Stream::new(
            dictionary! {
                "Type" => "Metadata",
                "Subtype" => "XML",
            },
            xmp_bytes,
        );

        // Get catalog object ID
        let catalog_id = doc.catalog().map_err(|e| {
            XmpError::IoError(std::io::Error::other(format!(
                "Failed to get PDF catalog: {}",
                e
            )))
        })?;

        // Check if there's an existing Metadata reference
        let existing_metadata_ref = catalog_id
            .get(b"Metadata")
            .ok()
            .and_then(|obj| obj.as_reference().ok());

        let metadata_id = if let Some(ref_id) = existing_metadata_ref {
            // Update existing metadata object
            doc.objects.insert(ref_id, Object::Stream(metadata_stream));
            ref_id
        } else {
            // Add new metadata object
            doc.add_object(Object::Stream(metadata_stream))
        };

        // Update catalog to reference metadata
        let catalog_obj_id = doc
            .trailer
            .get(b"Root")
            .map_err(|e| {
                XmpError::IoError(std::io::Error::other(format!("Failed to get Root: {}", e)))
            })?
            .as_reference()
            .map_err(|e| {
                XmpError::IoError(std::io::Error::other(format!(
                    "Root is not a reference: {}",
                    e
                )))
            })?;

        if let Some(Object::Dictionary(ref mut catalog_dict)) = doc.objects.get_mut(&catalog_obj_id)
        {
            catalog_dict.set("Metadata", Object::Reference(metadata_id));
        }

        // Save the modified document
        doc.save_to(&mut writer).map_err(|e| {
            XmpError::IoError(std::io::Error::other(format!("Failed to save PDF: {}", e)))
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// Create a minimal valid PDF using lopdf
    fn create_minimal_pdf() -> Vec<u8> {
        let mut doc = Document::with_version("1.4");

        // Create a minimal page
        let pages_id = doc.new_object_id();
        let page_id = doc.new_object_id();

        let page = dictionary! {
            "Type" => "Page",
            "Parent" => Object::Reference(pages_id),
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        };
        doc.objects.insert(page_id, Object::Dictionary(page));

        let pages = dictionary! {
            "Type" => "Pages",
            "Kids" => vec![Object::Reference(page_id)],
            "Count" => 1,
        };
        doc.objects.insert(pages_id, Object::Dictionary(pages));

        // Create catalog
        let catalog_id = doc.new_object_id();
        let catalog = dictionary! {
            "Type" => "Catalog",
            "Pages" => Object::Reference(pages_id),
        };
        doc.objects.insert(catalog_id, Object::Dictionary(catalog));

        // Set trailer
        doc.trailer.set("Root", Object::Reference(catalog_id));

        let mut buffer = Vec::new();
        doc.save_to(&mut buffer).unwrap();
        buffer
    }

    /// Create a PDF with XMP metadata using lopdf
    fn create_pdf_with_xmp(xmp: &str) -> Vec<u8> {
        let mut doc = Document::with_version("1.4");

        // Create a minimal page
        let pages_id = doc.new_object_id();
        let page_id = doc.new_object_id();

        let page = dictionary! {
            "Type" => "Page",
            "Parent" => Object::Reference(pages_id),
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        };
        doc.objects.insert(page_id, Object::Dictionary(page));

        let pages = dictionary! {
            "Type" => "Pages",
            "Kids" => vec![Object::Reference(page_id)],
            "Count" => 1,
        };
        doc.objects.insert(pages_id, Object::Dictionary(pages));

        // Create metadata stream
        let metadata_stream = Stream::new(
            dictionary! {
                "Type" => "Metadata",
                "Subtype" => "XML",
            },
            xmp.as_bytes().to_vec(),
        );
        let metadata_id = doc.add_object(Object::Stream(metadata_stream));

        // Create catalog with metadata reference
        let catalog_id = doc.new_object_id();
        let catalog = dictionary! {
            "Type" => "Catalog",
            "Pages" => Object::Reference(pages_id),
            "Metadata" => Object::Reference(metadata_id),
        };
        doc.objects.insert(catalog_id, Object::Dictionary(catalog));

        // Set trailer
        doc.trailer.set("Root", Object::Reference(catalog_id));

        let mut buffer = Vec::new();
        doc.save_to(&mut buffer).unwrap();
        buffer
    }

    fn create_minimal_xmp_packet() -> String {
        r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about=""
      xmlns:dc="http://purl.org/dc/elements/1.1/">
      <dc:title>Test PDF</dc:title>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#
            .to_string()
    }

    #[test]
    fn test_can_handle_pdf() {
        let pdf_data = create_minimal_pdf();
        let mut reader = Cursor::new(pdf_data);
        let handler = PdfHandler;
        assert!(handler.can_handle(&mut reader).unwrap());
    }

    #[test]
    fn test_can_handle_non_pdf() {
        let handler = PdfHandler;

        // JPEG file (need at least 5 bytes to compare with PDF signature)
        let jpeg_data = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
        let mut reader = Cursor::new(jpeg_data);
        assert!(!handler.can_handle(&mut reader).unwrap());

        // PNG file
        let png_data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        let mut reader = Cursor::new(png_data);
        assert!(!handler.can_handle(&mut reader).unwrap());

        // Empty file
        let empty_data: Vec<u8> = vec![];
        let mut reader = Cursor::new(empty_data);
        assert!(!handler.can_handle(&mut reader).unwrap());

        // Short file
        let short_data = vec![0x25, 0x50]; // "%P"
        let mut reader = Cursor::new(short_data);
        assert!(!handler.can_handle(&mut reader).unwrap());
    }

    #[test]
    fn test_read_xmp_from_pdf() {
        let xmp_packet = create_minimal_xmp_packet();
        let pdf_data = create_pdf_with_xmp(&xmp_packet);
        let reader = Cursor::new(pdf_data);

        let result = PdfHandler::read_xmp(reader).unwrap();
        assert!(result.is_some());

        let meta = result.unwrap();
        let title = meta.get_property(crate::core::namespace::ns::DC, "title");
        assert!(title.is_some());
    }

    #[test]
    fn test_read_xmp_no_xmp() {
        // PDF without XMP
        let pdf_data = create_minimal_pdf();
        let reader = Cursor::new(pdf_data);

        let result = PdfHandler::read_xmp(reader).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_write_xmp_to_pdf_with_existing_xmp() {
        // Create PDF with XMP
        let original_xmp = create_minimal_xmp_packet();
        let pdf_data = create_pdf_with_xmp(&original_xmp);

        // Create new metadata
        let mut new_meta = XmpMeta::new();
        let _ = new_meta.set_property(
            crate::core::namespace::ns::DC,
            "title",
            "Updated Title".into(),
        );
        let _ = new_meta.set_property(
            crate::core::namespace::ns::DC,
            "creator",
            "Test Author".into(),
        );

        // Write XMP
        let reader = Cursor::new(pdf_data);
        let mut output = Cursor::new(Vec::new());
        PdfHandler::write_xmp(reader, &mut output, &new_meta).unwrap();

        // Read back and verify
        output.set_position(0);
        let result = PdfHandler::read_xmp(&mut output).unwrap();
        assert!(result.is_some());

        let meta = result.unwrap();
        let title = meta.get_property(crate::core::namespace::ns::DC, "title");
        assert_eq!(
            title.and_then(|v| v.as_str().map(|s| s.to_string())),
            Some("Updated Title".to_string())
        );
    }

    #[test]
    fn test_write_xmp_to_pdf_without_existing_xmp() {
        // Create PDF without XMP
        let pdf_data = create_minimal_pdf();

        // Create new metadata
        let mut new_meta = XmpMeta::new();
        let _ = new_meta.set_property(crate::core::namespace::ns::DC, "title", "New Title".into());

        // Write XMP
        let reader = Cursor::new(pdf_data);
        let mut output = Cursor::new(Vec::new());
        PdfHandler::write_xmp(reader, &mut output, &new_meta).unwrap();

        // Read back and verify
        output.set_position(0);
        let result = PdfHandler::read_xmp(&mut output).unwrap();
        assert!(result.is_some());

        let meta = result.unwrap();
        let title = meta.get_property(crate::core::namespace::ns::DC, "title");
        assert_eq!(
            title.and_then(|v| v.as_str().map(|s| s.to_string())),
            Some("New Title".to_string())
        );
    }

    #[test]
    fn test_invalid_pdf() {
        let invalid_data = vec![0x00, 0x01, 0x02, 0x03, 0x04];
        let reader = Cursor::new(invalid_data);
        let result = PdfHandler::read_xmp(reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_format_info() {
        let handler = PdfHandler;
        assert_eq!(handler.format_name(), "PDF");
        assert_eq!(handler.extensions(), &["pdf"]);
    }
}
