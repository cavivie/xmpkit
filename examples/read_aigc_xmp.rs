// This application will accept a file path to a resource, open
// the file as read-only, then read the AIGC XMP data from the file.
// Once the XMP packet is available, it will access several
// properties and print those values to stdout.

use std::env;

// Note: In examples, we need to use the crate name directly
// The crate name is "xmpkit" as defined in Cargo.toml
use xmpkit::{ReadOptions, XmpError, XmpFile, XmpResult};

const AIGC_NS: &str = "http://www.tc260.org.cn/ns/AIGC/1.0/";

fn read_xmp_from_file() -> XmpResult<()> {
    // Parse command-line arguments. There should be only one
    // argument: a path to a file to be read.
    let args: Vec<String> = env::args().collect();

    let path = match args.len() {
        // args[0] = path to executable
        2 => &args[1],
        n => {
            eprintln!("expected 1 argument (file name), got {} arguments", n - 1);
            std::process::exit(1);
        }
    };

    // Open the file for read-only access and request to use a format-specific
    // handler.
    let mut f = XmpFile::new();

    f.open_with(path, ReadOptions::default().only_xmp().use_smart_handler())
        .or_else(|_err| {
            // There might not be an appropriate handler available.
            // Retry using packet scanning, providing a different set of
            // open-file options.
            eprintln!(
                "No smart handler available for file {}. Trying packet scanning.",
                path
            );
            f.open_with(path, ReadOptions::default().use_packet_scanning())
        })
        .map_err(|e| XmpError::BadValue(format!("could not find XMP in file {}: {}", path, e)))?;

    // Retrieve the XMP from the file.
    let xmp = f
        .get_xmp()
        .ok_or_else(|| XmpError::BadValue(format!("unable to process XMP in file {}", path)))?;

    // Read custom namespace property (e.g., TC260:AIGC)
    // Try to read from the TC260 namespace
    if let Some(aigc_value) = xmp.get_property(AIGC_NS, "AIGC") {
        println!("TC260:AIGC = {}", aigc_value);
    } else {
        println!("TC260:AIGC property not found");
    }

    Ok(())
}

fn main() {
    if let Err(err) = read_xmp_from_file() {
        eprintln!("Error: {:?}", err);
        std::process::exit(1);
    }
}
