use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::fs;
use std::io::{Cursor, Seek};
use xmpkit::files::FileHandler;
use xmpkit::{files, XmpFile, XmpMeta, XmpValue};

fn bench_read_jpeg_from_bytes(c: &mut Criterion) {
    // Load test JPEG file
    let jpeg_data = fs::read("tests/fixtures/data/image2.jpg").unwrap();
    let jpeg_data_clone = jpeg_data.clone();

    c.bench_function("read_jpeg_from_bytes", |b| {
        b.iter(|| {
            let mut file = XmpFile::new();
            let _result = file.from_bytes(black_box(&jpeg_data_clone));
        });
    });
}

fn bench_read_jpeg_from_reader(c: &mut Criterion) {
    // Load test JPEG file
    let jpeg_data = fs::read("tests/fixtures/data/image2.jpg").unwrap();

    c.bench_function("read_jpeg_from_reader", |b| {
        b.iter(|| {
            let cursor = Cursor::new(black_box(&jpeg_data));
            let mut file = XmpFile::new();
            let _result = file.from_reader(cursor);
        });
    });
}

fn bench_write_jpeg_to_bytes(c: &mut Criterion) {
    // Load test JPEG file
    let jpeg_data = fs::read("tests/fixtures/data/image2.jpg").unwrap();

    c.bench_function("write_jpeg_to_bytes", |b| {
        b.iter(|| {
            // Create XMP metadata for each iteration
            let mut meta = XmpMeta::new();
            meta.set_property(
                "http://ns.adobe.com/xap/1.0/",
                "CreatorTool",
                XmpValue::String("Benchmark Test".to_string()),
            )
            .unwrap();

            // Use FileHandler to write XMP
            let mut reader = Cursor::new(black_box(&jpeg_data));
            let mut writer = Cursor::new(Vec::new());

            let registry = files::default_registry();
            let handler = registry.find_by_detection(&mut reader).unwrap().unwrap();

            reader.rewind().unwrap();
            handler.write_xmp(&mut reader, &mut writer, &meta).unwrap();
            let _result = writer.into_inner();
        });
    });
}

fn bench_detect_format(c: &mut Criterion) {
    // Load test JPEG file
    let jpeg_data = fs::read("tests/fixtures/data/image2.jpg").unwrap();

    c.bench_function("detect_format", |b| {
        b.iter(|| {
            let mut cursor = Cursor::new(black_box(&jpeg_data));
            let registry = files::default_registry();
            let _handler = registry.find_by_detection(&mut cursor);
        });
    });
}

criterion_group!(
    benches,
    bench_read_jpeg_from_bytes,
    bench_read_jpeg_from_reader,
    bench_write_jpeg_to_bytes,
    bench_detect_format
);
criterion_main!(benches);
