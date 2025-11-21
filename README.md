<div align="center">

# XMPKit

<div>
  <img src="assets/logo-icon.svg" alt="XMPKit Logo" width="120" height="120">
</div>

**Pure Rust implementation of Adobe XMP Toolkit**

[![Crates.io][crates-badge]][crates-url]
[![MIT licensed][mit-badge]][mit-url]
[![Apache licensed, Version 2.0][apache-badge]][apache-url]
[![Build Status][actions-badge]][actions-url]

[crates-badge]: https://img.shields.io/crates/v/xmpkit.svg
[crates-url]: https://crates.io/crates/xmpkit
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/cavivie/xmpkit/blob/master/LICENSE-MIT
[apache-badge]: https://img.shields.io/badge/license-APACHE2.0-blue.svg
[apache-url]: https://github.com/cavivie/xmpkit/blob/master/LICENSE-APACHE
[actions-badge]: https://github.com/cavivie/xmpkit/workflows/CI/badge.svg
[actions-url]: https://github.com/cavivie/xmpkit/actions?query=workflow%3ACI+branch%3Amain

</div>

## Overview

XMPKit is a pure Rust implementation of Adobe's XMP (Extensible Metadata Platform) Toolkit. It provides APIs for reading, writing, and manipulating XMP metadata in various file formats without any C++ dependencies.

## Features

- Pure Rust implementation (no C++ dependencies)
- Compatible with Adobe XMP standard
- Support for common file formats (JPEG, PNG, TIFF, etc.)
- Memory safe and high performance
- Zero-cost abstractions
- Cross-platform support (iOS, Android, HarmonyOS, macOS, Windows, Linux, Wasm)

## Quick Start

```rust
use xmpkit::{XmpFile, XmpMeta, register_namespace};

// Open an image file
let mut file = XmpFile::new();
file.open("photo.jpg")?;

// Read XMP metadata
if let Some(meta) = file.get_xmp() {
    // Get image title (Dublin Core namespace - built-in, no registration needed)
    if let Some(title) = meta.get_property("http://purl.org/dc/elements/1.1/", "title") {
        println!("Title: {}", title);
    }

    // Get creator tool (XMP namespace - built-in, no registration needed)
    if let Some(creator) = meta.get_property("http://ns.adobe.com/xap/1.0/", "CreatorTool") {
        println!("Created with: {}", creator);
    }
}

// Modify metadata
if let Some(mut meta) = file.get_xmp().cloned() {
    // Set image title (built-in namespace, no registration needed)
    meta.set_property("http://purl.org/dc/elements/1.1/", "title", "My Photo")?;

    // Set creator tool (built-in namespace, no registration needed)
    meta.set_property("http://ns.adobe.com/xap/1.0/", "CreatorTool", "MyApp v1.0")?;

    // For custom namespaces, register first before setting properties
    register_namespace("http://example.com/myapp/1.0/", "myapp")?;
    meta.set_property("http://example.com/myapp/1.0/", "CustomProperty", "Custom Value")?;

    // Update metadata in file
    file.put_xmp(meta);
}

// Save the modified image
file.save("photo_updated.jpg")?;
```

## Documentation

Full API documentation is available at [docs.rs/xmpkit](https://docs.rs/xmpkit).

For WebAssembly/JavaScript integration, see [WEBASSEMBLY.md](docs/WEBASSEMBLY.md), [here is a online demo](https://cavivie.github.io/xmpkit).

## Project Status

### File Format Support

| Format | Extensions | Read XMP | Write XMP | Status |
|--------|-----------|----------|-----------|--------|
| JPEG | .jpg, .jpeg | Yes | Yes | Fully supported |
| PNG | .png | Yes | Yes | Fully supported |
| TIFF | .tif, .tiff | Yes | Yes | Fully supported |
| MP3 | .mp3 | Yes | Yes | Fully supported |
| GIF | .gif | Yes | Yes | Fully supported |
| MP4 | .mp4 | Yes | Yes | Fully supported |
| PDF | .pdf | No | No | Planned |
| WebP | .webp | No | No | Planned |

### Platform Support

| Platform | Architecture | File I/O | Memory I/O | Status |
|----------|-------------|----------|------------|--------|
| **Native Platforms** |
| macOS | x86_64, arm64 | Yes | Yes | Fully supported |
| Linux | x86_64, arm64 | Yes | Yes | Fully supported |
| Windows | x86_64, arm64 | Yes | Yes | Fully supported |
| iOS | arm64 | Yes | Yes | Fully supported |
| Android | arm64, armv7, x86_64 | Yes | Yes | Fully supported |
| HarmonyOS | arm64, armv7, x86_64 | Yes | Yes | Fully supported (use `ohos` feature for Node-API bindings) |
| **Web Platforms** |
| WebAssembly | wasm32 | No | Yes | Partial (use `from_bytes()` / `from_reader()`, see [WEBASSEMBLY](docs/WEBASSEMBLY.md)) |

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](docs/CONTRIBUTING.md) for guidelines.

## License

This project is licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## References

- [Adobe XMP Toolkit SDK - Rust](https://github.com/adobe/xmp-toolkit-rs)
- [Adobe XMP Toolkit SDK - C++](https://github.com/adobe/XMP-Toolkit-SDK)
- [XMP Specification](https://www.adobe.com/devnet/xmp.html)
