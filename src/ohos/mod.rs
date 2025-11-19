//! OpenHarmony/HarmonyOS Node-API bindings for xmpkit
//!
//! This module provides napi-ohos bindings for using xmpkit in OpenHarmony/HarmonyOS applications.
//! Enable the `ohos` feature to use these bindings.
//!
//! # Usage
//!
//! Add to your `Cargo.toml`:
//! ```toml
//! [lib]
//! crate-type = ["cdylib"]
//!
//! [dependencies]
//! xmpkit = { version = "0.1.0", features = ["ohos"] }
//! ```
//!
//! Then build with:
//! ```bash
//! ohrs build
//! ```
//!
//! Use in ArkTS:
//! ```typescript
//! import { XmpFile, XmpMeta } from 'libxmpkit.so';
//! const file = new XmpFile();
//! file.fromBytes(fileBytes);
//! const meta = file.getXmp();
//! ```
//!
//! This module provides Node-API bindings that mirror the Rust API.
//! Use `XmpFile` and `XmpMeta` classes in ArkTS just like in Rust.

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
    get_namespace_uri, is_namespace_registered, namespace_uri, register_namespace, Namespace,
};
pub use qualifier::Qualifier;
pub use value::{XmpValue, XmpValueKind};

// Module registration is done automatically by napi-ohos runtime
// No need to explicitly register the module
