use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use xmpkit::{XmpMeta, XmpValue};

fn create_simple_meta() -> XmpMeta {
    let mut meta = XmpMeta::new();
    meta.set_property(
        "http://ns.adobe.com/xap/1.0/",
        "CreatorTool",
        XmpValue::String("TestApp".to_string()),
    )
    .unwrap();
    meta
}

fn create_medium_meta() -> XmpMeta {
    let mut meta = XmpMeta::new();
    meta.set_property(
        "http://ns.adobe.com/xap/1.0/",
        "CreatorTool",
        XmpValue::String("Adobe Photoshop CS2 Windows".to_string()),
    )
    .unwrap();
    meta.set_property(
        "http://ns.adobe.com/xap/1.0/",
        "CreateDate",
        XmpValue::String("2006-04-25T15:32:01+02:00".to_string()),
    )
    .unwrap();
    meta.set_property(
        "http://ns.adobe.com/xap/1.0/",
        "ModifyDate",
        XmpValue::String("2006-04-27T15:38:36.655+02:00".to_string()),
    )
    .unwrap();
    meta.set_property(
        "http://ns.adobe.com/exif/1.0/",
        "PixelXDimension",
        XmpValue::String("200".to_string()),
    )
    .unwrap();
    meta.set_property(
        "http://ns.adobe.com/exif/1.0/",
        "PixelYDimension",
        XmpValue::String("200".to_string()),
    )
    .unwrap();

    // Add array
    meta.append_array_item(
        "http://purl.org/dc/elements/1.1/",
        "subject",
        XmpValue::String("purple".to_string()),
    )
    .unwrap();
    meta.append_array_item(
        "http://purl.org/dc/elements/1.1/",
        "subject",
        XmpValue::String("square".to_string()),
    )
    .unwrap();
    meta.append_array_item(
        "http://purl.org/dc/elements/1.1/",
        "subject",
        XmpValue::String("test".to_string()),
    )
    .unwrap();

    meta
}

fn create_complex_meta() -> XmpMeta {
    let mut meta = XmpMeta::new();

    // Simple properties
    meta.set_property(
        "http://ns.adobe.com/xap/1.0/",
        "CreatorTool",
        XmpValue::String("Adobe Photoshop CS2 Windows".to_string()),
    )
    .unwrap();
    meta.set_property(
        "http://ns.adobe.com/xap/1.0/",
        "CreateDate",
        XmpValue::String("2006-04-25T15:32:01+02:00".to_string()),
    )
    .unwrap();
    meta.set_property(
        "http://ns.adobe.com/xap/1.0/",
        "ModifyDate",
        XmpValue::String("2006-04-27T15:38:36.655+02:00".to_string()),
    )
    .unwrap();
    meta.set_property(
        "http://ns.adobe.com/xap/1.0/mm/",
        "DocumentID",
        XmpValue::String("uuid:FE607D9B5FD4DA118B7787757E22306B".to_string()),
    )
    .unwrap();
    meta.set_property(
        "http://ns.adobe.com/tiff/1.0/",
        "Orientation",
        XmpValue::String("1".to_string()),
    )
    .unwrap();
    meta.set_property(
        "http://ns.adobe.com/exif/1.0/",
        "PixelXDimension",
        XmpValue::String("200".to_string()),
    )
    .unwrap();
    meta.set_property(
        "http://ns.adobe.com/exif/1.0/",
        "PixelYDimension",
        XmpValue::String("200".to_string()),
    )
    .unwrap();

    // Arrays
    meta.append_array_item(
        "http://purl.org/dc/elements/1.1/",
        "creator",
        XmpValue::String("Llywelyn".to_string()),
    )
    .unwrap();
    meta.append_array_item(
        "http://purl.org/dc/elements/1.1/",
        "creator",
        XmpValue::String("Stefan".to_string()),
    )
    .unwrap();

    meta.append_array_item(
        "http://purl.org/dc/elements/1.1/",
        "subject",
        XmpValue::String("purple".to_string()),
    )
    .unwrap();
    meta.append_array_item(
        "http://purl.org/dc/elements/1.1/",
        "subject",
        XmpValue::String("square".to_string()),
    )
    .unwrap();
    meta.append_array_item(
        "http://purl.org/dc/elements/1.1/",
        "subject",
        XmpValue::String("XMP".to_string()),
    )
    .unwrap();

    // Localized text (Alt array)
    meta.set_localized_text(
        "http://purl.org/dc/elements/1.1/",
        "title",
        "",
        "x-default",
        "Purple Square",
    )
    .unwrap();
    meta.set_localized_text(
        "http://purl.org/dc/elements/1.1/",
        "description",
        "",
        "x-default",
        "a test file (öäüßÖÄÜ€中文)",
    )
    .unwrap();

    meta
}

fn bench_serialize_simple(c: &mut Criterion) {
    let meta = create_simple_meta();
    c.bench_function("serialize_simple", |b| {
        b.iter(|| {
            let _result = black_box(&meta).serialize().unwrap();
        });
    });
}

fn bench_serialize_medium(c: &mut Criterion) {
    let meta = create_medium_meta();
    c.bench_function("serialize_medium", |b| {
        b.iter(|| {
            let _result = black_box(&meta).serialize().unwrap();
        });
    });
}

fn bench_serialize_complex(c: &mut Criterion) {
    let meta = create_complex_meta();
    c.bench_function("serialize_complex", |b| {
        b.iter(|| {
            let _result = black_box(&meta).serialize().unwrap();
        });
    });
}

fn bench_serialize_packet_simple(c: &mut Criterion) {
    let meta = create_simple_meta();
    c.bench_function("serialize_packet_simple", |b| {
        b.iter(|| {
            let _result = black_box(&meta).serialize_packet().unwrap();
        });
    });
}

fn bench_serialize_packet_complex(c: &mut Criterion) {
    let meta = create_complex_meta();
    c.bench_function("serialize_packet_complex", |b| {
        b.iter(|| {
            let _result = black_box(&meta).serialize_packet().unwrap();
        });
    });
}

criterion_group!(
    benches,
    bench_serialize_simple,
    bench_serialize_medium,
    bench_serialize_complex,
    bench_serialize_packet_simple,
    bench_serialize_packet_complex
);
criterion_main!(benches);
