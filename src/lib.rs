//! XMPKit - Pure Rust implementation of Adobe XMP Toolkit
//!
//! XMPKit is a pure Rust implementation of Adobe's XMP (Extensible Metadata Platform) Toolkit.
//! It provides APIs for reading, writing, and manipulating XMP metadata in various file formats
//! without c++ SDK bridge.
//!
//! ## Features
//!
//! - Pure Rust implementation
//! - Compatible with Adobe XMP standard
//! - Support for common file formats (JPEG, PNG, TIFF, MP3, GIF, MP4)
//! - Memory safe and high performance
//! - Cross-platform support (iOS, Android, HarmonyOS, macOS, Windows, Linux, Wasm)
//!
//! ## Quick Start
//!
//! ### Working with Files (Native Platforms)
//!
//! ```rust,no_run
//! use xmpkit::{XmpFile, XmpMeta, XmpValue};
//! use xmpkit::core::namespace::ns;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Open a file
//! let mut file = XmpFile::new();
//! file.open("image.jpg")?;
//!
//! // Read XMP metadata
//! if let Some(meta) = file.get_xmp() {
//!     if let Some(creator) = meta.get_property(ns::XMP, "CreatorTool") {
//!         println!("Creator: {:?}", creator);
//!     }
//! }
//!
//! // Modify metadata
//! if let Some(mut meta) = file.get_xmp().cloned() {
//!     meta.set_property(
//!         ns::XMP,
//!         "CreatorTool",
//!         XmpValue::String("MyApp".to_string()),
//!     )?;
//!     file.put_xmp(meta);
//! }
//!
//! // Save changes
//! file.save("output.jpg")?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Working with Bytes (Wasm and Memory-based Operations)
//!
//! ```rust,no_run
//! use xmpkit::{XmpFile, XmpMeta, XmpValue};
//! use xmpkit::core::namespace::ns;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let jpeg_data: &[u8] = &[]; // your JPEG file data
//!
//! // Load from bytes
//! let mut file = XmpFile::new();
//! file.from_bytes(jpeg_data)?;
//!
//! // Read and modify metadata
//! if let Some(mut meta) = file.get_xmp().cloned() {
//!     meta.set_property(
//!         ns::XMP,
//!         "CreatorTool",
//!         XmpValue::String("MyApp".to_string()),
//!     )?;
//!     file.put_xmp(meta);
//! }
//!
//! // Write back to bytes
//! let output_data = file.write_to_bytes()?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Parsing XMP from String
//!
//! ```rust
//! use xmpkit::{XmpMeta, XmpValue};
//! use xmpkit::core::namespace::ns;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let xmp_packet = r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
//! <x:xmpmeta xmlns:x="adobe:ns:meta/">
//!   <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
//!     <rdf:Description rdf:about="" xmlns:xmp="http://ns.adobe.com/xap/1.0/">
//!       <xmp:CreatorTool>MyApp</xmp:CreatorTool>
//!     </rdf:Description>
//!   </rdf:RDF>
//! </x:xmpmeta><?xpacket end="w"?>"#;
//!
//! let meta = XmpMeta::parse(xmp_packet)?;
//!
//! if let Some(creator) = meta.get_property(ns::XMP, "CreatorTool") {
//!     println!("Creator: {:?}", creator);
//! }
//!
//! // Modify and serialize
//! let mut meta = meta;
//! meta.set_property(
//!     ns::XMP,
//!     "ModifyDate",
//!     XmpValue::DateTime("2024-01-01T00:00:00Z".to_string()),
//! )?;
//!
//! let serialized = meta.serialize_packet()?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Modules
//!
//! - [`core`] - Core XMP functionality (parsing, serialization, metadata API)
//! - [`files`] - File format handlers for reading/writing XMP from files
//! - [`types`] - Common types and data structures (XmpValue, Qualifier)
//! - [`utils`] - Utility functions (date/time handling)
//!
//! ## Platform Support
//!
//! ### Native Platforms (iOS, Android, HarmonyOS, macOS, Windows, Linux)
//!
//! Native platforms support both file I/O and memory-based operations:
//!
//! - `XmpFile::open()` - Open files from file system
//! - `XmpFile::save()` - Save files to file system
//! - `XmpFile::from_bytes()` - Load from memory (also available)
//! - `XmpFile::write_to_bytes()` - Write to memory (also available)
//!
//! ### WebAssembly (Wasm)
//!
//! Wasm platforms support memory-based operations only:
//!
//! - `XmpFile::from_bytes()` - Load from memory
//! - `XmpFile::from_reader()` - Load from reader
//! - `XmpFile::write_to_bytes()` - Write to memory
//! - `XmpFile::write_to_writer()` - Write to writer
//!
//! To use xmpkit in JavaScript, enable the `wasm` feature:
//!
//! **Simple way (recommended):**
//!
//! 1. Create a new crate: `cargo new --lib my-wasm-app`
//! 2. Add to `Cargo.toml`:
//!    ```toml
//!    [lib]
//!    crate-type = ["cdylib"]
//!    [dependencies]
//!    xmpkit = { version = "0.1.0", features = ["wasm"] }
//!    ```
//! 3. Re-export bindings in `src/lib.rs`:
//!    ```rust,ignore
//!    pub use xmpkit::wasm::*;
//!    ```
//! 4. Build: `wasm-pack build --target web`
//!
//! Then use in JavaScript:
//!
//! ```javascript
//! import init, { read_xmp, write_xmp } from './pkg/my_wasm_app.js';
//! await init();
//! const result = read_xmp(new Uint8Array(/* file bytes */));
//! ```
//!
//! **Alternative**: Create a custom binding crate (see `docs/WEBASSEMBLY.md` for details)
//!
//! ### OpenHarmony/HarmonyOS (ArkTS)
//!
//! To use xmpkit in OpenHarmony/HarmonyOS applications, enable the `ohos` feature:
//!
//! 1. Add to `Cargo.toml`:
//!    ```toml
//!    [lib]
//!    crate-type = ["cdylib"]
//!    [dependencies]
//!    xmpkit = { version = "0.1.0", features = ["ohos"] }
//!    ```
//! 2. Re-export bindings in `src/lib.rs`:
//!    ```rust,ignore
//!    pub use xmpkit::ohos::*;
//!    ```
//! 3. Build: `ohrs build`
//!
//! Then use in ArkTS:
//!
//! ```typescript
//! import { XmpFile, XmpMeta } from 'libxmpkit.so';
//! const file = new XmpFile();
//! file.fromBytes(fileBytes);
//! const meta = file.getXmp();
//! ```
//!
//! ## Feature Flags
//!
//! - `core` - Core XMP functionality (enabled by default)
//! - `files` - File format support infrastructure (enabled by default)
//! - `jpeg`, `png`, `tiff`, `mp3`, `gif`, `mp4` - Individual file format handlers
//! - `full-formats` - Enable all file format handlers (enabled by default)
//! - `mutli-thread` - Multi-threaded runtime support (enabled by default)
//! - `wasm` - WebAssembly JavaScript bindings (optional, enables wasm-bindgen integration)
//! - `ohos` - OpenHarmony/HarmonyOS Node-API bindings (optional, enables napi-ohos integration)
//!
//! ## Supported File Formats
//!
//! | Format | Extensions | Read | Write |
//! |--------|-----------|------|-------|
//! | JPEG   | .jpg, .jpeg | Yes | Yes |
//! | PNG    | .png      | Yes | Yes |
//! | TIFF   | .tif, .tiff | Yes | Yes |
//! | MP3    | .mp3      | Yes | Yes |
//! | GIF    | .gif      | Yes | Yes |
//! | MP4    | .mp4      | Yes | Yes |

#[cfg(feature = "core")]
pub mod core;
#[cfg(feature = "files")]
pub mod files;
pub mod types;
pub mod utils;

#[cfg(feature = "wasm")]
pub mod wasm;

#[cfg(all(feature = "ohos", target_ohos))]
pub mod ohos;

// Re-export commonly used types
#[cfg(feature = "core")]
pub use core::error::{XmpError, XmpResult};
#[cfg(feature = "core")]
pub use core::metadata::XmpMeta;
#[cfg(feature = "core")]
pub use core::namespace::{
    get_all_registered_namespaces, get_builtin_namespace_uris, get_global_namespace_prefix,
    get_global_namespace_uri, is_namespace_registered, ns, register_namespace,
};
#[cfg(feature = "files")]
pub use files::{ReadOptions, XmpFile};
pub use types::qualifier::Qualifier;
pub use types::value::XmpValue;
pub use utils::datetime::XmpDateTime;
