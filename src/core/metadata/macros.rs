//! Macros for root node access
//!
//! These macros provide convenient access to the root node with proper
//! error handling for both single-threaded and multi-threaded modes.

// Internal macros for lock handling
// These macros are only used within the metadata module
#[cfg(not(feature = "mutli-thread"))]
#[doc(hidden)]
macro_rules! root_read {
    ($root:expr) => {
        $crate::core::metadata::node::root_read(&$root)
    };
}

#[cfg(feature = "mutli-thread")]
#[doc(hidden)]
macro_rules! root_read {
    ($root:expr) => {
        $crate::core::metadata::node::root_read(&$root).map_err(|_| {
            $crate::core::error::XmpError::InternalError("Lock poisoned".to_string())
        })?
    };
}

#[cfg(not(feature = "mutli-thread"))]
#[doc(hidden)]
macro_rules! root_write {
    ($root:expr) => {
        $crate::core::metadata::node::root_write(&$root)
    };
}

#[cfg(feature = "mutli-thread")]
#[doc(hidden)]
macro_rules! root_write {
    ($root:expr) => {
        $crate::core::metadata::node::root_write(&$root).map_err(|_| {
            $crate::core::error::XmpError::InternalError("Lock poisoned".to_string())
        })?
    };
}

#[cfg(not(feature = "mutli-thread"))]
#[doc(hidden)]
macro_rules! root_read_opt {
    ($root:expr) => {
        $crate::core::metadata::node::root_read(&$root)
    };
}

#[cfg(feature = "mutli-thread")]
#[doc(hidden)]
macro_rules! root_read_opt {
    ($root:expr) => {
        $crate::core::metadata::node::root_read(&$root).ok()?
    };
}
