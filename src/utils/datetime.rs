//! XMP Date/Time utilities
//!
//! This module provides functionality for parsing and formatting XMP date/time values.
//! XMP uses a specific ISO 8601-like format that supports partial dates and time zones.

use crate::core::error::{XmpError, XmpResult};

/// XMP Date/Time structure
///
/// Represents a date/time value with optional components.
/// XMP supports partial dates (e.g., just year, or year-month).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmpDateTime {
    /// Year (can be negative for BCE dates)
    pub year: i32,
    /// Month (1-12, 0 means not set)
    pub month: u8,
    /// Day (1-31, 0 means not set)
    pub day: u8,
    /// Hour (0-23)
    pub hour: u8,
    /// Minute (0-59)
    pub minute: u8,
    /// Second (0-59)
    pub second: u8,
    /// Nanoseconds (0-999999999)
    pub nanosecond: u32,
    /// Whether date components are present
    pub has_date: bool,
    /// Whether time components are present
    pub has_time: bool,
    /// Whether timezone is present
    pub has_timezone: bool,
    /// Timezone sign: -1 (west), 0 (UTC), +1 (east)
    pub tz_sign: i8,
    /// Timezone hour offset (0-23)
    pub tz_hour: u8,
    /// Timezone minute offset (0-59)
    pub tz_minute: u8,
}

impl XmpDateTime {
    /// Create a new empty XMP date/time
    pub fn new() -> Self {
        Self {
            year: 0,
            month: 0,
            day: 0,
            hour: 0,
            minute: 0,
            second: 0,
            nanosecond: 0,
            has_date: false,
            has_time: false,
            has_timezone: false,
            tz_sign: 0,
            tz_hour: 0,
            tz_minute: 0,
        }
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
    /// - `YYYY-MM-DDThh:mm:ss-hh:mm` - negative timezone offset
    ///
    /// # Example
    ///
    /// ```rust
    /// use xmpkit::utils::datetime::XmpDateTime;
    ///
    /// let dt = XmpDateTime::parse("2023-12-25T10:30:00Z").unwrap();
    /// assert_eq!(dt.year, 2023);
    /// assert_eq!(dt.month, 12);
    /// assert_eq!(dt.day, 25);
    /// ```
    pub fn parse(s: &str) -> XmpResult<Self> {
        if s.is_empty() {
            return Err(XmpError::BadValue("Empty date/time string".to_string()));
        }

        let mut dt = Self::new();
        let mut pos = 0;
        let bytes = s.as_bytes();

        // Check if this is a time-only value (starts with 'T' or has ':' early)
        let time_only = bytes[0] == b'T'
            || (bytes.len() >= 2 && bytes[1] == b':')
            || (bytes.len() >= 3 && bytes[2] == b':');

        if !time_only {
            dt.has_date = true;

            // Parse year (may be negative)
            let year_start = pos;
            if bytes[pos] == b'-' {
                pos += 1;
            }
            while pos < bytes.len() && bytes[pos].is_ascii_digit() {
                pos += 1;
            }
            if pos == year_start || (year_start > 0 && pos == year_start + 1) {
                return Err(XmpError::BadValue(
                    "Invalid year in date string".to_string(),
                ));
            }
            let year_str = std::str::from_utf8(&bytes[year_start..pos])
                .map_err(|_| XmpError::BadValue("Invalid UTF-8 in date string".to_string()))?;
            dt.year = year_str
                .parse()
                .map_err(|_| XmpError::BadValue("Invalid year value".to_string()))?;

            if pos >= bytes.len() {
                return Ok(dt);
            }

            // Parse month
            if bytes[pos] != b'-' {
                return Err(XmpError::BadValue(
                    "Invalid date string, expected '-' after year".to_string(),
                ));
            }
            pos += 1;

            let month_start = pos;
            while pos < bytes.len() && bytes[pos].is_ascii_digit() {
                pos += 1;
            }
            if pos == month_start {
                return Err(XmpError::BadValue(
                    "Invalid month in date string".to_string(),
                ));
            }
            let month_str = std::str::from_utf8(&bytes[month_start..pos])
                .map_err(|_| XmpError::BadValue("Invalid UTF-8 in date string".to_string()))?;
            let month_val: u8 = month_str
                .parse()
                .map_err(|_| XmpError::BadValue("Invalid month value".to_string()))?;
            dt.month = if month_val == 0 {
                1
            } else if month_val > 12 {
                12
            } else {
                month_val
            };

            if pos >= bytes.len() {
                return Ok(dt);
            }

            // Parse day
            if bytes[pos] != b'-' {
                return Err(XmpError::BadValue(
                    "Invalid date string, expected '-' after month".to_string(),
                ));
            }
            pos += 1;

            let day_start = pos;
            while pos < bytes.len() && bytes[pos] != b'T' && bytes[pos].is_ascii_digit() {
                pos += 1;
            }
            if pos == day_start {
                return Err(XmpError::BadValue("Invalid day in date string".to_string()));
            }
            let day_str = std::str::from_utf8(&bytes[day_start..pos])
                .map_err(|_| XmpError::BadValue("Invalid UTF-8 in date string".to_string()))?;
            let day_val: u8 = day_str
                .parse()
                .map_err(|_| XmpError::BadValue("Invalid day value".to_string()))?;
            dt.day = if day_val == 0 {
                1
            } else if day_val > 31 {
                31
            } else {
                day_val
            };
        }

        // Parse time
        if pos < bytes.len() && bytes[pos] == b'T' {
            pos += 1;
        } else if !time_only {
            return Ok(dt);
        }

        dt.has_time = true;

        // Parse hour
        let hour_start = pos;
        while pos < bytes.len() && bytes[pos].is_ascii_digit() {
            pos += 1;
        }
        if pos == hour_start {
            return Err(XmpError::BadValue(
                "Invalid hour in date string".to_string(),
            ));
        }
        let hour_str = std::str::from_utf8(&bytes[hour_start..pos])
            .map_err(|_| XmpError::BadValue("Invalid UTF-8 in date string".to_string()))?;
        let hour_val: u8 = hour_str
            .parse()
            .map_err(|_| XmpError::BadValue("Invalid hour value".to_string()))?;
        dt.hour = if hour_val > 23 { 23 } else { hour_val };

        if pos >= bytes.len() || bytes[pos] != b':' {
            return Err(XmpError::BadValue(
                "Invalid date string, expected ':' after hour".to_string(),
            ));
        }
        pos += 1;

        // Parse minute
        let minute_start = pos;
        while pos < bytes.len()
            && bytes[pos] != b':'
            && bytes[pos] != b'Z'
            && bytes[pos] != b'+'
            && bytes[pos] != b'-'
            && bytes[pos].is_ascii_digit()
        {
            pos += 1;
        }
        if pos == minute_start {
            return Err(XmpError::BadValue(
                "Invalid minute in date string".to_string(),
            ));
        }
        let minute_str = std::str::from_utf8(&bytes[minute_start..pos])
            .map_err(|_| XmpError::BadValue("Invalid UTF-8 in date string".to_string()))?;
        let minute_val: u8 = minute_str
            .parse()
            .map_err(|_| XmpError::BadValue("Invalid minute value".to_string()))?;
        dt.minute = if minute_val > 59 { 59 } else { minute_val };

        if pos >= bytes.len() {
            return Ok(dt);
        }

        // Parse second (optional)
        if bytes[pos] == b':' {
            pos += 1;

            let second_start = pos;
            while pos < bytes.len()
                && bytes[pos] != b'.'
                && bytes[pos] != b'Z'
                && bytes[pos] != b'+'
                && bytes[pos] != b'-'
                && bytes[pos].is_ascii_digit()
            {
                pos += 1;
            }
            if pos == second_start {
                return Err(XmpError::BadValue(
                    "Invalid second in date string".to_string(),
                ));
            }
            let second_str = std::str::from_utf8(&bytes[second_start..pos])
                .map_err(|_| XmpError::BadValue("Invalid UTF-8 in date string".to_string()))?;
            let second_val: u8 = second_str
                .parse()
                .map_err(|_| XmpError::BadValue("Invalid second value".to_string()))?;
            dt.second = if second_val > 59 { 59 } else { second_val };

            // Parse fractional seconds (optional)
            if pos < bytes.len() && bytes[pos] == b'.' {
                pos += 1;
                let frac_start = pos;
                while pos < bytes.len()
                    && bytes[pos] != b'Z'
                    && bytes[pos] != b'+'
                    && bytes[pos] != b'-'
                    && bytes[pos].is_ascii_digit()
                {
                    pos += 1;
                }
                if pos > frac_start {
                    let frac_str = std::str::from_utf8(&bytes[frac_start..pos]).map_err(|_| {
                        XmpError::BadValue("Invalid UTF-8 in date string".to_string())
                    })?;
                    let mut frac_val: u32 = frac_str.parse().map_err(|_| {
                        XmpError::BadValue("Invalid fractional second value".to_string())
                    })?;
                    // Normalize to nanoseconds (max 9 digits)
                    let digits = pos - frac_start;
                    if digits > 9 {
                        for _ in 9..digits {
                            frac_val /= 10;
                        }
                    } else {
                        for _ in digits..9 {
                            frac_val *= 10;
                        }
                    }
                    if frac_val >= 1_000_000_000 {
                        return Err(XmpError::BadValue(
                            "Fractional second is out of range".to_string(),
                        ));
                    }
                    dt.nanosecond = frac_val;
                }
            }
        }

        if pos >= bytes.len() {
            return Ok(dt);
        }

        // Parse timezone
        dt.has_timezone = true;

        if bytes[pos] == b'Z' {
            dt.tz_sign = 0;
            pos += 1;
        } else if bytes[pos] == b'+' || bytes[pos] == b'-' {
            dt.tz_sign = if bytes[pos] == b'+' { 1 } else { -1 };
            pos += 1;

            // Parse timezone hour
            let tz_hour_start = pos;
            while pos < bytes.len() && bytes[pos].is_ascii_digit() {
                pos += 1;
            }
            if pos == tz_hour_start {
                return Err(XmpError::BadValue(
                    "Invalid timezone hour in date string".to_string(),
                ));
            }
            let tz_hour_str = std::str::from_utf8(&bytes[tz_hour_start..pos])
                .map_err(|_| XmpError::BadValue("Invalid UTF-8 in date string".to_string()))?;
            let tz_hour_val: u8 = tz_hour_str
                .parse()
                .map_err(|_| XmpError::BadValue("Invalid timezone hour value".to_string()))?;
            if tz_hour_val > 23 {
                return Err(XmpError::BadValue(
                    "Timezone hour is out of range".to_string(),
                ));
            }
            dt.tz_hour = tz_hour_val;

            if pos >= bytes.len() || bytes[pos] != b':' {
                return Err(XmpError::BadValue(
                    "Invalid date string, expected ':' after timezone hour".to_string(),
                ));
            }
            pos += 1;

            // Parse timezone minute
            let tz_minute_start = pos;
            while pos < bytes.len() && bytes[pos].is_ascii_digit() {
                pos += 1;
            }
            if pos == tz_minute_start {
                return Err(XmpError::BadValue(
                    "Invalid timezone minute in date string".to_string(),
                ));
            }
            let tz_minute_str = std::str::from_utf8(&bytes[tz_minute_start..pos])
                .map_err(|_| XmpError::BadValue("Invalid UTF-8 in date string".to_string()))?;
            let tz_minute_val: u8 = tz_minute_str
                .parse()
                .map_err(|_| XmpError::BadValue("Invalid timezone minute value".to_string()))?;
            if tz_minute_val > 59 {
                return Err(XmpError::BadValue(
                    "Timezone minute is out of range".to_string(),
                ));
            }
            dt.tz_minute = tz_minute_val;
        }

        if pos < bytes.len() {
            return Err(XmpError::BadValue(
                "Invalid date string, extra characters at end".to_string(),
            ));
        }

        Ok(dt)
    }

    /// Format an XMP date/time to string
    ///
    /// Formats the date/time according to XMP specification:
    /// - Year only: `YYYY`
    /// - Year and month: `YYYY-MM`
    /// - Date only: `YYYY-MM-DD`
    /// - Date and time: `YYYY-MM-DDThh:mm:ss`
    /// - With fractional seconds: `YYYY-MM-DDThh:mm:ss.sss`
    /// - With timezone: `YYYY-MM-DDThh:mm:ssZ` or `YYYY-MM-DDThh:mm:ss+hh:mm`
    pub fn format(&self) -> String {
        let mut result = String::new();

        // Format date portion
        if self.has_date {
            if self.month == 0 {
                // Year only
                result.push_str(&format!("{:04}", self.year));
            } else if self.day == 0 {
                // Year and month
                result.push_str(&format!("{:04}-{:02}", self.year, self.month));
            } else {
                // Full date
                result.push_str(&format!(
                    "{:04}-{:02}-{:02}",
                    self.year, self.month, self.day
                ));
            }
        }

        // Format time portion
        if self.has_time {
            if self.has_date {
                result.push('T');
            }
            if self.nanosecond == 0 {
                result.push_str(&format!(
                    "{:02}:{:02}:{:02}",
                    self.hour, self.minute, self.second
                ));
            } else {
                // Format nanoseconds, removing trailing zeros
                let mut ns_str = format!("{:09}", self.nanosecond);
                while ns_str.ends_with('0') {
                    ns_str.pop();
                }
                result.push_str(&format!(
                    "{:02}:{:02}:{:02}.{}",
                    self.hour, self.minute, self.second, ns_str
                ));
            }
        }

        // Format timezone
        if self.has_timezone {
            if self.tz_sign == 0 {
                result.push('Z');
            } else {
                let sign = if self.tz_sign < 0 { '-' } else { '+' };
                result.push_str(&format!(
                    "{}{:02}:{:02}",
                    sign, self.tz_hour, self.tz_minute
                ));
            }
        }

        result
    }

    /// Validate the date/time values
    ///
    /// Checks that all values are within valid ranges.
    pub fn validate(&self) -> XmpResult<()> {
        if self.has_date {
            if self.month != 0 && (self.month < 1 || self.month > 12) {
                return Err(XmpError::BadValue("Month is out of range".to_string()));
            }
            if self.day != 0 && (self.day < 1 || self.day > 31) {
                return Err(XmpError::BadValue("Day is out of range".to_string()));
            }
        }

        if self.has_time {
            if self.hour > 23 {
                return Err(XmpError::BadValue("Hour is out of range".to_string()));
            }
            if self.minute > 59 {
                return Err(XmpError::BadValue("Minute is out of range".to_string()));
            }
            if self.second > 59 {
                return Err(XmpError::BadValue("Second is out of range".to_string()));
            }
            if self.nanosecond >= 1_000_000_000 {
                return Err(XmpError::BadValue("Nanosecond is out of range".to_string()));
            }
        }

        if self.has_timezone {
            if self.tz_hour > 23 {
                return Err(XmpError::BadValue(
                    "Timezone hour is out of range".to_string(),
                ));
            }
            if self.tz_minute > 59 {
                return Err(XmpError::BadValue(
                    "Timezone minute is out of range".to_string(),
                ));
            }
            if self.tz_sign == 0 && (self.tz_hour != 0 || self.tz_minute != 0) {
                return Err(XmpError::BadValue(
                    "UTC timezone must have zero hour and minute".to_string(),
                ));
            }
        }

        Ok(())
    }
}

impl Default for XmpDateTime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_year_only() {
        let dt = XmpDateTime::parse("2023").unwrap();
        assert_eq!(dt.year, 2023);
        assert_eq!(dt.month, 0);
        assert_eq!(dt.has_date, true);
        assert_eq!(dt.has_time, false);
    }

    #[test]
    fn test_parse_year_month() {
        let dt = XmpDateTime::parse("2023-12").unwrap();
        assert_eq!(dt.year, 2023);
        assert_eq!(dt.month, 12);
        assert_eq!(dt.day, 0);
    }

    #[test]
    fn test_parse_full_date() {
        let dt = XmpDateTime::parse("2023-12-25").unwrap();
        assert_eq!(dt.year, 2023);
        assert_eq!(dt.month, 12);
        assert_eq!(dt.day, 25);
        assert_eq!(dt.has_time, false);
    }

    #[test]
    fn test_parse_date_time() {
        let dt = XmpDateTime::parse("2023-12-25T10:30:00").unwrap();
        assert_eq!(dt.year, 2023);
        assert_eq!(dt.month, 12);
        assert_eq!(dt.day, 25);
        assert_eq!(dt.hour, 10);
        assert_eq!(dt.minute, 30);
        assert_eq!(dt.second, 0);
        assert_eq!(dt.has_time, true);
    }

    #[test]
    fn test_parse_with_timezone_utc() {
        let dt = XmpDateTime::parse("2023-12-25T10:30:00Z").unwrap();
        assert_eq!(dt.has_timezone, true);
        assert_eq!(dt.tz_sign, 0);
    }

    #[test]
    fn test_parse_with_timezone_offset() {
        let dt = XmpDateTime::parse("2023-12-25T10:30:00+08:00").unwrap();
        assert_eq!(dt.has_timezone, true);
        assert_eq!(dt.tz_sign, 1);
        assert_eq!(dt.tz_hour, 8);
        assert_eq!(dt.tz_minute, 0);
    }

    #[test]
    fn test_parse_with_fractional_seconds() {
        let dt = XmpDateTime::parse("2023-12-25T10:30:00.123Z").unwrap();
        assert_eq!(dt.second, 0);
        assert_eq!(dt.nanosecond, 123_000_000);
    }

    #[test]
    fn test_format_year_only() {
        let mut dt = XmpDateTime::new();
        dt.has_date = true;
        dt.year = 2023;
        assert_eq!(dt.format(), "2023");
    }

    #[test]
    fn test_format_year_month() {
        let mut dt = XmpDateTime::new();
        dt.has_date = true;
        dt.year = 2023;
        dt.month = 12;
        assert_eq!(dt.format(), "2023-12");
    }

    #[test]
    fn test_format_full_date_time() {
        let mut dt = XmpDateTime::new();
        dt.has_date = true;
        dt.has_time = true;
        dt.year = 2023;
        dt.month = 12;
        dt.day = 25;
        dt.hour = 10;
        dt.minute = 30;
        dt.second = 0;
        assert_eq!(dt.format(), "2023-12-25T10:30:00");
    }

    #[test]
    fn test_format_with_timezone() {
        let mut dt = XmpDateTime::new();
        dt.has_date = true;
        dt.has_time = true;
        dt.has_timezone = true;
        dt.year = 2023;
        dt.month = 12;
        dt.day = 25;
        dt.hour = 10;
        dt.minute = 30;
        dt.second = 0;
        dt.tz_sign = 0;
        assert_eq!(dt.format(), "2023-12-25T10:30:00Z");
    }

    #[test]
    fn test_round_trip() {
        let test_cases = vec![
            "2023",
            "2023-12",
            "2023-12-25",
            "2023-12-25T10:30:00",
            "2023-12-25T10:30:00Z",
            "2023-12-25T10:30:00+08:00",
            "2023-12-25T10:30:00.123Z",
        ];

        for test_case in test_cases {
            let dt = XmpDateTime::parse(test_case).unwrap();
            let formatted = dt.format();
            // Note: Round-trip may not be exact due to normalization (e.g., "2023-12-25T10:30:00" vs "2023-12-25T10:30:00")
            // But parsing the formatted result should work
            let dt2 = XmpDateTime::parse(&formatted).unwrap();
            assert_eq!(dt.year, dt2.year);
            assert_eq!(dt.month, dt2.month);
            assert_eq!(dt.day, dt2.day);
            assert_eq!(dt.hour, dt2.hour);
            assert_eq!(dt.minute, dt2.minute);
            assert_eq!(dt.second, dt2.second);
        }
    }
}
