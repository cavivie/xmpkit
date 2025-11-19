//! WebAssembly JavaScript bindings for xmpkit
//!
//! This module provides wasm-bindgen bindings for using xmpkit in JavaScript/TypeScript.
//! Enable the `wasm` feature to use these bindings.
//!
//! # Usage
//!
//! Add to your `Cargo.toml`:
//! ```toml
//! [lib]
//! crate-type = ["cdylib"]
//!
//! [dependencies]
//! xmpkit = { version = "0.1.0", features = ["wasm"] }
//! ```
//!
//! Then build with:
//! ```bash
//! wasm-pack build --target web --out-dir pkg
//! ```
//!
//! Use in JavaScript:
//! ```javascript
//! import init, { read_xmp, write_xmp, WasmXmpFile, WasmXmpMeta } from './pkg/xmpkit.js';
//! await init();
//! const result = read_xmp(new Uint8Array(/* file bytes */));
//! ```
//!
//! This module provides WebAssembly bindings that mirror the Rust API.
//! Use `XmpFile` and `XmpMeta` classes in JavaScript just like in Rust.

mod datetime;
mod error;
mod file;
mod meta;
mod namespace;
mod qualifier;
mod value;

pub use datetime::XmpDateTime;
pub use error::{XmpError, XmpErrorKind};
pub use file::{ReadOptions, XmpFile};
pub use meta::XmpMeta;
pub use namespace::{
    get_all_registered_namespaces, get_builtin_namespace_uris, get_namespace_prefix,
    get_namespace_uri, is_namespace_registered, namespace_prefix, namespace_uri,
    register_namespace, Namespace,
};
pub use qualifier::Qualifier;
pub use value::{XmpValue, XmpValueKind};
