//! WebAssembly bindings for XMP date/time utilities

use crate::utils::datetime::XmpDateTime as RustXmpDateTime;
use crate::wasm::error::{xmp_error_to_wasm_error, XmpError};
use wasm_bindgen::prelude::*;

/// XMP Date/Time structure
///
/// Represents a date/time value with optional components.
/// XMP supports partial dates (e.g., just year, or year-month).
#[wasm_bindgen]
#[derive(Clone, Default)]
pub struct XmpDateTime {
    pub(crate) inner: RustXmpDateTime,
}

#[wasm_bindgen]
impl XmpDateTime {
    /// Create a new empty XMP date/time
    #[wasm_bindgen(constructor)]
    pub fn new() -> XmpDateTime {
        XmpDateTime::default()
    }

    /// Parse an XMP date/time string
    ///
    /// XMP date/time format:
    /// - `YYYY` - year only
    /// - `YYYY-MM` - year and month
    /// - `YYYY-MM-DD` - date only
    /// - `YYYY-MM-DDThh:mm:ss` - date and time
    /// - `YYYY-MM-DDThh:mm:ss.sss` - with fractional seconds
    /// - `YYYY-MM-DDThh:mm:ssZ` - UTC timezone
    /// - `YYYY-MM-DDThh:mm:ss+hh:mm` - timezone offset
    ///
    /// # Example
    ///
    /// ```javascript
    /// import { XmpDateTime } from './pkg/xmpkit.js';
    /// const dt = XmpDateTime.parse("2023-12-25T10:30:00Z");
    /// console.log(dt.year); // 2023
    /// ```
    pub fn parse(s: &str) -> Result<XmpDateTime, XmpError> {
        RustXmpDateTime::parse(s)
            .map(|dt| XmpDateTime { inner: dt })
            .map_err(xmp_error_to_wasm_error)
    }

    /// Format as XMP date/time string
    pub fn format(&self) -> String {
        self.inner.format()
    }

    /// Get the year
    #[wasm_bindgen(getter)]
    pub fn year(&self) -> i32 {
        self.inner.year
    }

    /// Get the month (1-12, 0 means not set)
    #[wasm_bindgen(getter)]
    pub fn month(&self) -> u8 {
        self.inner.month
    }

    /// Get the day (1-31, 0 means not set)
    #[wasm_bindgen(getter)]
    pub fn day(&self) -> u8 {
        self.inner.day
    }

    /// Get the hour (0-23)
    #[wasm_bindgen(getter)]
    pub fn hour(&self) -> u8 {
        self.inner.hour
    }

    /// Get the minute (0-59)
    #[wasm_bindgen(getter)]
    pub fn minute(&self) -> u8 {
        self.inner.minute
    }

    /// Get the second (0-59)
    #[wasm_bindgen(getter)]
    pub fn second(&self) -> u8 {
        self.inner.second
    }

    /// Get the nanoseconds (0-999999999)
    #[wasm_bindgen(getter)]
    pub fn nanosecond(&self) -> u32 {
        self.inner.nanosecond
    }

    /// Whether date components are present
    #[wasm_bindgen(getter)]
    pub fn has_date(&self) -> bool {
        self.inner.has_date
    }

    /// Whether time components are present
    #[wasm_bindgen(getter)]
    pub fn has_time(&self) -> bool {
        self.inner.has_time
    }

    /// Whether timezone is present
    #[wasm_bindgen(getter)]
    pub fn has_timezone(&self) -> bool {
        self.inner.has_timezone
    }

    /// Timezone sign: -1 (west), 0 (UTC), +1 (east)
    #[wasm_bindgen(getter)]
    pub fn tz_sign(&self) -> i8 {
        self.inner.tz_sign
    }

    /// Timezone hour offset (0-23)
    #[wasm_bindgen(getter)]
    pub fn tz_hour(&self) -> u8 {
        self.inner.tz_hour
    }

    /// Timezone minute offset (0-59)
    #[wasm_bindgen(getter)]
    pub fn tz_minute(&self) -> u8 {
        self.inner.tz_minute
    }
}
