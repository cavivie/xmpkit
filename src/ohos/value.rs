//! OpenHarmony bindings for XMP value types

use napi_derive_ohos::napi;

#[napi]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum XmpValueKind {
    String,
    Integer,
    Boolean,
    DateTime,
}

#[napi]
#[derive(Clone, Debug)]
pub struct XmpValue {
    kind: XmpValueKind,
    #[allow(dead_code)]
    string_value: Option<String>,
    #[allow(dead_code)]
    integer_value: Option<i64>,
    #[allow(dead_code)]
    boolean_value: Option<bool>,
}

#[napi]
impl XmpValue {
    #[napi(constructor)]
    pub fn string(s: String) -> XmpValue {
        XmpValue {
            kind: XmpValueKind::String,
            string_value: Some(s),
            integer_value: None,
            boolean_value: None,
        }
    }

    #[napi]
    pub fn integer(i: i64) -> XmpValue {
        XmpValue {
            kind: XmpValueKind::Integer,
            string_value: None,
            integer_value: Some(i),
            boolean_value: None,
        }
    }

    #[napi]
    pub fn boolean(b: bool) -> XmpValue {
        XmpValue {
            kind: XmpValueKind::Boolean,
            string_value: None,
            integer_value: None,
            boolean_value: Some(b),
        }
    }

    #[napi(getter)]
    pub fn kind(&self) -> XmpValueKind {
        self.kind
    }
}
