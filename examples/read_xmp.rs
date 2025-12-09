//! Read XMP metadata from a file
//!
//! This example demonstrates how to read XMP metadata from various file formats.
//! It reads properties from the XMP Basic, Dublin Core, and Exif schemas.

use std::env;

use xmpkit::{core::namespace::ns, XmpFile, XmpValue};

fn read_xmp_from_file() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command-line arguments. There should be only one
    // argument: a path to a file to be read.
    let args: Vec<String> = env::args().collect();

    let path = match args.len() {
        // args[0] = path to executable
        2 => Ok(&args[1]),
        n => Err(format!(
            "expected 1 argument (file name), got {} arguments",
            n - 1
        )),
    }?;

    let mut xmp_file = XmpFile::new();
    xmp_file.open(path)?;

    // Retrieve the XMP from the file.
    let xmp = xmp_file
        .get_xmp()
        .ok_or_else(|| format!("unable to process XMP in file {}", path))?;

    // Display the simple property "CreatorTool" by providing
    // the namespace URI and the name of the property.
    if let Some(creator_tool) = xmp.get_property(ns::XMP, "CreatorTool") {
        if let XmpValue::String(value) = creator_tool {
            println!("CreatorTool = {}", value);
        }
    }

    // Display the first element of the `creator` array.
    if let Some(size) = xmp.get_array_size(ns::DC, "creator") {
        if size > 0 {
            if let Some(first_creator) = xmp.get_array_item(ns::DC, "creator", 0) {
                if let XmpValue::String(value) = first_creator {
                    println!("dc:creator = {}", value);
                }
            }
        } else {
            println!("No creator found");
        }
    } else {
        println!("No creator found");
    }

    // Display all elements in the `subject` property (which is an array).
    // Note that the C++ XMP Toolkit's indices are 1-based. This example's output
    // instead follows Rust's convention of being 0-based.
    if let Some(size) = xmp.get_array_size(ns::DC, "subject") {
        for index in 0..size {
            if let Some(subject) = xmp.get_array_item(ns::DC, "subject", index) {
                if let XmpValue::String(value) = subject {
                    println!("dc:subject[{}] = {}", index, value);
                }
            }
        }
    }

    // Get a localized text item; display the `title` property in English.
    if let Some((value, _actual_lang)) = xmp.get_localized_text(ns::DC, "title", "en", "en-US") {
        println!("dc:title in English = {}", value);
    }

    // Get a localized text item; display the `title` property in French.
    if let Some((value, _actual_lang)) = xmp.get_localized_text(ns::DC, "title", "fr", "fr-FR") {
        println!("dc:title in French = {}", value);
    }

    // Get a date property; read the `MetadataDate` property if it exists. If so,
    // convert the `XmpDateTime` into a string and display it.
    if let Some(dt) = xmp.get_date_time(ns::XMP, "MetadataDate") {
        println!("xmp:MetadataDate = {}", dt.format());
    }

    // Discover if the Exif Flash structure is available. If so, display the
    // flash status at the time the photograph was taken.
    if let Some(value) = xmp.get_struct_field(ns::EXIF, "Flash", "Fired") {
        if let XmpValue::String(s) = value {
            println!("Flash Used = {}", s);
        }
    }

    Ok(())
}

fn main() {
    if let Err(err) = read_xmp_from_file() {
        eprintln!("Error: {:?}", err);
        std::process::exit(1);
    }
}
