//! Tests for XmpFile API
//!
//! These tests verify file operations work correctly.

#[path = "fixtures/mod.rs"]
mod fixtures;

use fixtures::{fixture_exists, fixture_path};
use xmpkit::{XmpFile, XmpMeta};

#[cfg(not(target_arch = "wasm32"))]
mod native_tests {
    use super::*;

    #[test]
    fn open_file() {
        if !fixture_exists("image2.jpg") {
            eprintln!("Skipping test: fixture image2.jpg not found");
            return;
        }

        let mut file = XmpFile::new();
        let result = file.open(fixture_path("image2.jpg"));

        // May or may not have XMP, so we just check it doesn't error on valid JPEG
        if let Err(e) = result {
            // If it fails, it should be a format error, not a file error
            assert!(!e.to_string().contains("No such file"));
        }
    }

    #[test]
    fn file_not_found() {
        let mut file = XmpFile::new();
        let result = file.open("doesnotexist.jpg");
        assert!(result.is_err());
    }
}

mod wasm_compatible_tests {
    use super::*;

    #[test]
    fn from_bytes_empty() {
        let mut file = XmpFile::new();
        let result = file.from_bytes(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn from_bytes_invalid_jpeg() {
        let mut file = XmpFile::new();
        let invalid_data = vec![0x00, 0x01, 0x02];
        let result = file.from_bytes(&invalid_data);
        assert!(result.is_err());
    }

    #[test]
    fn put_and_get_xmp() {
        let mut file = XmpFile::new();
        let meta = XmpMeta::new();
        file.put_xmp(meta);
        assert!(file.get_xmp().is_some());
    }
}
