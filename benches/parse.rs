use criterion::{black_box, criterion_group, criterion_main, Criterion};
use xmpkit::XmpMeta;

// Simple XMP packet with minimal properties
const SIMPLE_XMP: &str = r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:xmp="http://ns.adobe.com/xap/1.0/">
  <rdf:Description rdf:about=""
                   xmp:CreatorTool="Adobe Photoshop CS2 Windows"/>
</rdf:RDF>
<?xpacket end="w"?>"#;

// Medium complexity XMP packet with multiple properties
const MEDIUM_XMP: &str = r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:xmp="http://ns.adobe.com/xap/1.0/"
         xmlns:dc="http://purl.org/dc/elements/1.1/"
         xmlns:exif="http://ns.adobe.com/exif/1.0/">
  <rdf:Description rdf:about=""
                   xmp:CreatorTool="Adobe Photoshop CS2 Windows"
                   xmp:CreateDate="2006-04-25T15:32:01+02:00"
                   xmp:ModifyDate="2006-04-27T15:38:36.655+02:00"
                   exif:PixelXDimension="200"
                   exif:PixelYDimension="200">
    <dc:subject>
      <rdf:Bag>
        <rdf:li>purple</rdf:li>
        <rdf:li>square</rdf:li>
        <rdf:li>test</rdf:li>
      </rdf:Bag>
    </dc:subject>
  </rdf:Description>
</rdf:RDF>
<?xpacket end="w"?>"#;

// Complex XMP packet with arrays, structures, and qualifiers
const COMPLEX_XMP: &str = r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:xmp="http://ns.adobe.com/xap/1.0/"
         xmlns:dc="http://purl.org/dc/elements/1.1/"
         xmlns:xmpMM="http://ns.adobe.com/xap/1.0/mm/"
         xmlns:tiff="http://ns.adobe.com/tiff/1.0/"
         xmlns:exif="http://ns.adobe.com/exif/1.0/"
         xmlns:photoshop="http://ns.adobe.com/photoshop/1.0/">
  <rdf:Description rdf:about=""
                   xmp:CreatorTool="Adobe Photoshop CS2 Windows"
                   xmp:CreateDate="2006-04-25T15:32:01+02:00"
                   xmp:ModifyDate="2006-04-27T15:38:36.655+02:00"
                   xmp:MetadataDate="2006-04-26T16:47:10+02:00"
                   xmpMM:DocumentID="uuid:FE607D9B5FD4DA118B7787757E22306B"
                   xmpMM:InstanceID="uuid:BF664E7B33D5DA119129F691B53239AD"
                   tiff:Orientation="1"
                   tiff:XResolution="720000/10000"
                   tiff:YResolution="720000/10000"
                   exif:PixelXDimension="200"
                   exif:PixelYDimension="200"
                   photoshop:ColorMode="3">
    <dc:description>
      <rdf:Alt>
        <rdf:li xml:lang="x-default">a test file (öäüßÖÄÜ€中文)</rdf:li>
      </rdf:Alt>
    </dc:description>
    <dc:title>
      <rdf:Alt>
        <rdf:li xml:lang="x-default">Purple Square</rdf:li>
      </rdf:Alt>
    </dc:title>
    <dc:creator>
      <rdf:Seq>
        <rdf:li>Llywelyn</rdf:li>
        <rdf:li>Stefan</rdf:li>
      </rdf:Seq>
    </dc:creator>
    <dc:subject>
      <rdf:Bag>
        <rdf:li>purple</rdf:li>
        <rdf:li>square</rdf:li>
        <rdf:li>Stefan</rdf:li>
        <rdf:li>XMP</rdf:li>
        <rdf:li>XMPFiles</rdf:li>
        <rdf:li>test</rdf:li>
      </rdf:Bag>
    </dc:subject>
  </rdf:Description>
</rdf:RDF>
<?xpacket end="w"?>"#;

fn bench_parse_simple(c: &mut Criterion) {
    c.bench_function("parse_simple", |b| {
        b.iter(|| {
            let _meta = XmpMeta::parse(black_box(SIMPLE_XMP)).unwrap();
        });
    });
}

fn bench_parse_medium(c: &mut Criterion) {
    c.bench_function("parse_medium", |b| {
        b.iter(|| {
            let _meta = XmpMeta::parse(black_box(MEDIUM_XMP)).unwrap();
        });
    });
}

fn bench_parse_complex(c: &mut Criterion) {
    c.bench_function("parse_complex", |b| {
        b.iter(|| {
            let _meta = XmpMeta::parse(black_box(COMPLEX_XMP)).unwrap();
        });
    });
}

fn bench_parse_from_str_trait(c: &mut Criterion) {
    c.bench_function("parse_from_str_trait", |b| {
        b.iter(|| {
            let _meta: XmpMeta = black_box(SIMPLE_XMP).parse().unwrap();
        });
    });
}

// Large XMP packet with many properties and arrays
const LARGE_XMP: &str = r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:xmp="http://ns.adobe.com/xap/1.0/"
         xmlns:dc="http://purl.org/dc/elements/1.1/"
         xmlns:xmpMM="http://ns.adobe.com/xap/1.0/mm/"
         xmlns:tiff="http://ns.adobe.com/tiff/1.0/"
         xmlns:exif="http://ns.adobe.com/exif/1.0/"
         xmlns:photoshop="http://ns.adobe.com/photoshop/1.0/"
         xmlns:xmpRights="http://ns.adobe.com/xap/1.0/rights/"
         xmlns:Iptc4xmpCore="http://iptc.org/std/Iptc4xmpCore/1.0/xmlns/">
  <rdf:Description rdf:about=""
                   xmp:CreatorTool="Adobe Photoshop CS2 Windows"
                   xmp:CreateDate="2006-04-25T15:32:01+02:00"
                   xmp:ModifyDate="2006-04-27T15:38:36.655+02:00"
                   xmp:MetadataDate="2006-04-26T16:47:10+02:00"
                   xmpMM:DocumentID="uuid:FE607D9B5FD4DA118B7787757E22306B"
                   xmpMM:InstanceID="uuid:BF664E7B33D5DA119129F691B53239AD"
                   tiff:Orientation="1"
                   tiff:XResolution="720000/10000"
                   tiff:YResolution="720000/10000"
                   tiff:ResolutionUnit="2"
                   exif:PixelXDimension="200"
                   exif:PixelYDimension="200"
                   exif:ColorSpace="-1"
                   photoshop:ColorMode="3"
                   photoshop:ICCProfile="Dell 1905FP Color Profile"
                   photoshop:CaptionWriter="Stefan"
                   xmpRights:Marked="False">
    <dc:description>
      <rdf:Alt>
        <rdf:li xml:lang="x-default">a test file (öäüßÖÄÜ€中文)</rdf:li>
        <rdf:li xml:lang="en">A test file with special characters</rdf:li>
        <rdf:li xml:lang="zh">测试文件</rdf:li>
      </rdf:Alt>
    </dc:description>
    <dc:title>
      <rdf:Alt>
        <rdf:li xml:lang="x-default">Purple Square</rdf:li>
        <rdf:li xml:lang="en">Purple Square</rdf:li>
      </rdf:Alt>
    </dc:title>
    <dc:creator>
      <rdf:Seq>
        <rdf:li>Llywelyn</rdf:li>
        <rdf:li>Stefan</rdf:li>
        <rdf:li>John Doe</rdf:li>
        <rdf:li>Jane Smith</rdf:li>
      </rdf:Seq>
    </dc:creator>
    <dc:subject>
      <rdf:Bag>
        <rdf:li>purple</rdf:li>
        <rdf:li>square</rdf:li>
        <rdf:li>Stefan</rdf:li>
        <rdf:li>XMP</rdf:li>
        <rdf:li>XMPFiles</rdf:li>
        <rdf:li>test</rdf:li>
        <rdf:li>metadata</rdf:li>
        <rdf:li>RDF</rdf:li>
        <rdf:li>XML</rdf:li>
        <rdf:li>benchmark</rdf:li>
      </rdf:Bag>
    </dc:subject>
    <Iptc4xmpCore:CreatorContactInfo
        Iptc4xmpCore:CiAdrPcode="98110"
        Iptc4xmpCore:CiAdrCtry="US"
        Iptc4xmpCore:CiAdrCity="Seattle"
        Iptc4xmpCore:CiEmailWork="test@example.com"/>
  </rdf:Description>
</rdf:RDF>
<?xpacket end="w"?>"#;

fn bench_parse_large(c: &mut Criterion) {
    c.bench_function("parse_large", |b| {
        b.iter(|| {
            let _meta = XmpMeta::parse(black_box(LARGE_XMP)).unwrap();
        });
    });
}

// XMP without xpacket wrapper (just RDF)
const RDF_ONLY_XMP: &str = r#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:xmp="http://ns.adobe.com/xap/1.0/">
  <rdf:Description rdf:about=""
                   xmp:CreatorTool="Adobe Photoshop CS2 Windows"/>
</rdf:RDF>"#;

fn bench_parse_rdf_only(c: &mut Criterion) {
    c.bench_function("parse_rdf_only", |b| {
        b.iter(|| {
            let _meta = XmpMeta::parse(black_box(RDF_ONLY_XMP)).unwrap();
        });
    });
}

criterion_group!(
    benches,
    bench_parse_simple,
    bench_parse_medium,
    bench_parse_complex,
    bench_parse_large,
    bench_parse_rdf_only,
    bench_parse_from_str_trait
);
criterion_main!(benches);
