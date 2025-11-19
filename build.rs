fn main() {
    #[cfg(feature = "ohos")]
    {
        // Only setup napi-ohos if we're building for an ohos target
        let target = std::env::var("TARGET").unwrap_or_default();
        if target.contains("ohos") {
            println!("cargo:rustc-cfg=target_ohos");
            napi_build_ohos::setup();
        }
    }
}
