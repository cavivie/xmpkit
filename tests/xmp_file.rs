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

/// Tests for streaming read functionality (Issue #31)
/// Verifies that XMP can be read without loading entire file into memory
mod streaming_read_tests {
    use super::*;
    use xmpkit::XmpOptions;

    #[test]
    fn read_only_mode_does_not_store_file_data() {
        if !fixture_exists("image2.jpg") {
            eprintln!("Skipping test: fixture image2.jpg not found");
            return;
        }

        // Read file content
        let file_data = std::fs::read(fixture_path("image2.jpg")).unwrap();

        // Open in read-only mode (default)
        let mut file = XmpFile::new();
        file.from_bytes(&file_data).unwrap();

        // Verify we can read XMP (if present) or at least open successfully
        // The file was opened successfully, so the read worked
        let _ = file.get_xmp();

        // Attempting to write should fail because file_data is not stored
        let result = file.write_to_bytes();
        assert!(
            result.is_err(),
            "Write should fail in read-only mode without file_data"
        );

        // Error message should indicate the issue
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("XmpOptions::for_update"),
            "Error should suggest using for_update()"
        );
    }

    #[test]
    fn for_update_mode_stores_file_data() {
        if !fixture_exists("image2.jpg") {
            eprintln!("Skipping test: fixture image2.jpg not found");
            return;
        }

        // Read file content
        let file_data = std::fs::read(fixture_path("image2.jpg")).unwrap();

        // Open with for_update option
        let mut file = XmpFile::new();
        file.from_bytes_with(&file_data, XmpOptions::default().for_update())
            .unwrap();

        // Verify we can read XMP
        let _ = file.get_xmp();

        // Writing should work because file_data is stored
        let result = file.write_to_bytes();
        // Note: This may still fail if there's no XMP metadata, but it shouldn't
        // fail due to missing file_data
        if let Err(e) = &result {
            let err_msg = e.to_string();
            // If it fails, it should NOT be because of missing file_data
            assert!(
                !err_msg.contains("file data not available"),
                "Should not fail due to missing file_data in for_update mode"
            );
        }
    }

    #[test]
    fn packet_scanning_mode_stores_file_data_when_for_update() {
        if !fixture_exists("image2.jpg") {
            eprintln!("Skipping test: fixture image2.jpg not found");
            return;
        }

        // Read file content
        let file_data = std::fs::read(fixture_path("image2.jpg")).unwrap();

        // Open with packet scanning and for_update
        let mut file = XmpFile::new();
        file.from_bytes_with(
            &file_data,
            XmpOptions::default().use_packet_scanning().for_update(),
        )
        .unwrap();

        // Writing should work because file_data is stored
        let result = file.write_to_bytes();
        if let Err(e) = &result {
            let err_msg = e.to_string();
            assert!(
                !err_msg.contains("file data not available"),
                "Should not fail due to missing file_data in packet_scanning + for_update mode"
            );
        }
    }

    #[test]
    fn packet_scanning_without_for_update_does_not_store_file_data() {
        if !fixture_exists("image2.jpg") {
            eprintln!("Skipping test: fixture image2.jpg not found");
            return;
        }

        // Read file content
        let file_data = std::fs::read(fixture_path("image2.jpg")).unwrap();

        // Open with packet scanning only (read-only)
        let mut file = XmpFile::new();
        file.from_bytes_with(&file_data, XmpOptions::default().use_packet_scanning())
            .unwrap();

        // Writing should fail because file_data is not stored
        let result = file.write_to_bytes();
        assert!(
            result.is_err(),
            "Write should fail in packet_scanning read-only mode"
        );
    }
}
