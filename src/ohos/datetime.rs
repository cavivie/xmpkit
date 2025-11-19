//! OpenHarmony bindings for XMP date/time utilities

use crate::ohos::error::xmp_error_to_ohos_error;
use crate::utils::datetime::XmpDateTime as RustXmpDateTime;
use napi_derive_ohos::napi;
use napi_ohos::bindgen_prelude::*;

#[napi]
#[derive(Clone, Default)]
pub struct XmpDateTime {
    pub(crate) inner: RustXmpDateTime,
}

#[napi]
impl XmpDateTime {
    #[napi(constructor)]
    pub fn new() -> XmpDateTime {
        XmpDateTime::default()
    }

    #[napi]
    pub fn parse(s: String) -> Result<XmpDateTime> {
        RustXmpDateTime::parse(&s)
            .map(|dt| XmpDateTime { inner: dt })
            .map_err(|e| Error::from_reason(format!("{}", xmp_error_to_ohos_error(e))))
    }

    #[napi(getter)]
    pub fn year(&self) -> i32 {
        self.inner.year
    }
}
