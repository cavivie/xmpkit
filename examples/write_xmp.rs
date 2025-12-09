//! Write XMP metadata to a file
//!
//! This example demonstrates how to write XMP metadata to various file formats.
//! It writes properties to the Dublin Core, XMP Basic, and custom namespaces.

use std::env;

use xmpkit::{core::namespace::ns, register_namespace, ReadOptions, XmpFile, XmpMeta};

fn write_xmp_to_file() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command-line arguments.
    // Expected: input_file output_file
    let args: Vec<String> = env::args().collect();

    let (input_path, output_path) = match args.len() {
        3 => Ok((&args[1], &args[2])),
        n => Err(format!(
            "expected 2 arguments (input_file output_file), got {} arguments",
            n - 1
        )),
    }?;

    // Open the input file with for_update option (required for writing)
    let mut xmp_file = XmpFile::new();
    xmp_file.open_with(input_path, ReadOptions::default().for_update())?;

    // Get existing XMP or create new metadata
    let mut xmp = xmp_file.get_xmp().cloned().unwrap_or_else(XmpMeta::new);

    // =========================================
    // Set simple properties
    // =========================================

    // Set the creator tool (XMP Basic namespace)
    xmp.set_property(ns::XMP, "CreatorTool", "XMPKit Example v1.0".into())?;

    // Set the title (Dublin Core namespace) with localized text
    xmp.set_localized_text(ns::DC, "title", "en", "en-US", "My Document Title")?;
    xmp.set_localized_text(ns::DC, "title", "zh", "zh-CN", "我的文档标题")?;

    // Set the description
    xmp.set_localized_text(
        ns::DC,
        "description",
        "en",
        "en-US",
        "This is a sample document with XMP metadata.",
    )?;

    // Set rights/copyright information
    xmp.set_localized_text(ns::DC, "rights", "en", "en-US", "© 2024 Example Corp.")?;

    // =========================================
    // Set array properties
    // =========================================

    // Add creators (authors)
    xmp.append_array_item(ns::DC, "creator", "John Doe".into())?;
    xmp.append_array_item(ns::DC, "creator", "Jane Smith".into())?;

    // Add keywords/subjects
    xmp.append_array_item(ns::DC, "subject", "XMP".into())?;
    xmp.append_array_item(ns::DC, "subject", "Metadata".into())?;
    xmp.append_array_item(ns::DC, "subject", "Rust".into())?;
    xmp.append_array_item(ns::DC, "subject", "Example".into())?;

    // =========================================
    // Set date properties
    // =========================================

    // Set creation and modification dates
    // You can parse ISO 8601 date strings
    let create_date = xmpkit::XmpDateTime::parse("2024-01-15T10:30:00+08:00")?;
    let modify_date = xmpkit::XmpDateTime::parse("2024-12-09T14:00:00+08:00")?;
    xmp.set_date_time(ns::XMP, "CreateDate", &create_date)?;
    xmp.set_date_time(ns::XMP, "ModifyDate", &modify_date)?;
    xmp.set_date_time(ns::XMP, "MetadataDate", &modify_date)?;

    // =========================================
    // Set struct properties
    // =========================================

    // Set a structured property (e.g., IPTC Creator Contact Info)
    // First register the IPTC namespace if not already registered
    let iptc_ext = "http://iptc.org/std/Iptc4xmpExt/2008-02-29/";
    register_namespace(iptc_ext, "Iptc4xmpExt")?;

    // =========================================
    // Custom namespace example
    // =========================================

    // Register and use a custom namespace
    let custom_ns = "http://example.com/myapp/1.0/";
    register_namespace(custom_ns, "myapp")?;

    xmp.set_property(custom_ns, "AppVersion", "2.0.1".into())?;
    xmp.set_property(custom_ns, "DocumentType", "report".into())?;
    xmp.set_property(custom_ns, "ProcessedBy", "XMPKit".into())?;

    // =========================================
    // Update the file
    // =========================================

    // Put the modified XMP back into the file
    xmp_file.put_xmp(xmp);

    // Save to the output file
    xmp_file.save(output_path)?;

    println!("Successfully wrote XMP metadata to: {}", output_path);
    println!();
    println!("Properties written:");
    println!("  - xmp:CreatorTool");
    println!("  - dc:title (en-US, zh-CN)");
    println!("  - dc:description (en-US)");
    println!("  - dc:rights (en-US)");
    println!("  - dc:creator (array)");
    println!("  - dc:subject (array)");
    println!("  - xmp:CreateDate, xmp:ModifyDate, xmp:MetadataDate");
    println!("  - myapp:AppVersion, myapp:DocumentType, myapp:ProcessedBy");

    Ok(())
}

fn main() {
    if let Err(err) = write_xmp_to_file() {
        eprintln!("Error: {:?}", err);
        std::process::exit(1);
    }
}
