use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use xmpkit::{XmpMeta, XmpValue};

fn create_populated_meta() -> XmpMeta {
    let mut meta = XmpMeta::new();
    meta.set_property(
        "http://ns.adobe.com/xap/1.0/",
        "CreatorTool",
        XmpValue::String("TestApp".to_string()),
    )
    .unwrap();
    meta.set_property(
        "http://ns.adobe.com/xap/1.0/",
        "CreateDate",
        XmpValue::String("2006-04-25T15:32:01+02:00".to_string()),
    )
    .unwrap();
    meta.set_property(
        "http://ns.adobe.com/exif/1.0/",
        "PixelXDimension",
        XmpValue::String("200".to_string()),
    )
    .unwrap();

    // Add array items
    for i in 0..10 {
        meta.append_array_item(
            "http://purl.org/dc/elements/1.1/",
            "subject",
            XmpValue::String(format!("item{}", i)),
        )
        .unwrap();
    }

    meta
}

fn bench_set_property(c: &mut Criterion) {
    c.bench_function("set_property", |b| {
        b.iter(|| {
            let mut meta = XmpMeta::new();
            meta.set_property(
                black_box("http://ns.adobe.com/xap/1.0/"),
                black_box("CreatorTool"),
                black_box(XmpValue::String("TestApp".to_string())),
            )
            .unwrap();
        });
    });
}

fn bench_get_property(c: &mut Criterion) {
    let meta = create_populated_meta();
    c.bench_function("get_property", |b| {
        b.iter(|| {
            let _value = black_box(&meta).get_property(
                black_box("http://ns.adobe.com/xap/1.0/"),
                black_box("CreatorTool"),
            );
        });
    });
}

fn bench_has_property(c: &mut Criterion) {
    let meta = create_populated_meta();
    c.bench_function("has_property", |b| {
        b.iter(|| {
            let _exists = black_box(&meta).has_property(
                black_box("http://ns.adobe.com/xap/1.0/"),
                black_box("CreatorTool"),
            );
        });
    });
}

fn bench_delete_property(c: &mut Criterion) {
    c.bench_function("delete_property", |b| {
        b.iter(|| {
            let mut meta = create_populated_meta();
            meta.delete_property(
                black_box("http://ns.adobe.com/xap/1.0/"),
                black_box("CreatorTool"),
            )
            .unwrap();
        });
    });
}

fn bench_append_array_item(c: &mut Criterion) {
    c.bench_function("append_array_item", |b| {
        b.iter(|| {
            let mut meta = XmpMeta::new();
            meta.append_array_item(
                black_box("http://purl.org/dc/elements/1.1/"),
                black_box("subject"),
                black_box(XmpValue::String("test".to_string())),
            )
            .unwrap();
        });
    });
}

fn bench_get_array_item(c: &mut Criterion) {
    let meta = create_populated_meta();
    c.bench_function("get_array_item", |b| {
        b.iter(|| {
            let _value = black_box(&meta).get_array_item(
                black_box("http://purl.org/dc/elements/1.1/"),
                black_box("subject"),
                black_box(0),
            );
        });
    });
}

fn bench_get_array_size(c: &mut Criterion) {
    let meta = create_populated_meta();
    c.bench_function("get_array_size", |b| {
        b.iter(|| {
            let _size = black_box(&meta).get_array_size(
                black_box("http://purl.org/dc/elements/1.1/"),
                black_box("subject"),
            );
        });
    });
}

fn bench_set_localized_text(c: &mut Criterion) {
    c.bench_function("set_localized_text", |b| {
        b.iter(|| {
            let mut meta = XmpMeta::new();
            meta.set_localized_text(
                black_box("http://purl.org/dc/elements/1.1/"),
                black_box("title"),
                black_box(""),
                black_box("x-default"),
                black_box("Test Title"),
            )
            .unwrap();
        });
    });
}

fn bench_get_localized_text(c: &mut Criterion) {
    let mut meta = XmpMeta::new();
    meta.set_localized_text(
        "http://purl.org/dc/elements/1.1/",
        "title",
        "",
        "x-default",
        "Test Title",
    )
    .unwrap();

    c.bench_function("get_localized_text", |b| {
        b.iter(|| {
            let _value = black_box(&meta).get_localized_text(
                black_box("http://purl.org/dc/elements/1.1/"),
                black_box("title"),
                black_box(""),
                black_box("x-default"),
            );
        });
    });
}

criterion_group!(
    benches,
    bench_set_property,
    bench_get_property,
    bench_has_property,
    bench_delete_property,
    bench_append_array_item,
    bench_get_array_item,
    bench_get_array_size,
    bench_set_localized_text,
    bench_get_localized_text
);
criterion_main!(benches);
