//! Tests for XmpMeta API
//!
//! These tests are adapted from the original xmp-toolkit-rs tests
//! to work with our pure Rust implementation.

use xmpkit::XmpMeta;

#[test]
fn new_empty() {
    let m = XmpMeta::new();
    // Empty XmpMeta has no properties
    assert!(!m.has_property("http://ns.adobe.com/xap/1.0/", "CreatorTool"));
}

#[test]
fn default() {
    let m = XmpMeta::default();
    // Empty XmpMeta has no properties
    assert!(!m.has_property("http://ns.adobe.com/xap/1.0/", "CreatorTool"));
}

mod from_str {
    use xmpkit::{XmpMeta, XmpValue};

    const SIMPLE_XMP: &str = r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:xmp="http://ns.adobe.com/xap/1.0/">
  <rdf:Description rdf:about=""
                   xmp:CreatorTool="Adobe Photoshop CS2 Windows"/>
</rdf:RDF>
<?xpacket end="w"?>"#;

    #[test]
    fn happy_path() {
        let m = SIMPLE_XMP.parse::<XmpMeta>().unwrap();

        assert_eq!(
            m.get_property("http://ns.adobe.com/xap/1.0/", "CreatorTool"),
            Some(XmpValue::String("Adobe Photoshop CS2 Windows".to_string()))
        );
    }

    #[test]
    fn invalid_xml() {
        let result = "not valid xml".parse::<XmpMeta>();
        assert!(result.is_err());
    }
}

mod property_operations {
    use xmpkit::{XmpMeta, XmpValue};

    #[test]
    fn set_and_get_property() {
        let mut m = XmpMeta::new();

        m.set_property(
            "http://ns.adobe.com/xap/1.0/",
            "CreatorTool",
            XmpValue::String("TestApp".to_string()),
        )
        .unwrap();

        assert_eq!(
            m.get_property("http://ns.adobe.com/xap/1.0/", "CreatorTool"),
            Some(XmpValue::String("TestApp".to_string()))
        );
    }

    #[test]
    fn has_property() {
        let mut m = XmpMeta::new();

        assert!(!m.has_property("http://ns.adobe.com/xap/1.0/", "CreatorTool"));

        m.set_property(
            "http://ns.adobe.com/xap/1.0/",
            "CreatorTool",
            XmpValue::String("TestApp".to_string()),
        )
        .unwrap();

        assert!(m.has_property("http://ns.adobe.com/xap/1.0/", "CreatorTool"));
        assert!(!m.has_property("http://ns.adobe.com/xap/1.0/", "NonExistent"));
    }

    #[test]
    fn delete_property() {
        let mut m = XmpMeta::new();

        m.set_property(
            "http://ns.adobe.com/xap/1.0/",
            "CreatorTool",
            XmpValue::String("TestApp".to_string()),
        )
        .unwrap();

        assert!(m.has_property("http://ns.adobe.com/xap/1.0/", "CreatorTool"));

        m.delete_property("http://ns.adobe.com/xap/1.0/", "CreatorTool")
            .unwrap();

        assert!(!m.has_property("http://ns.adobe.com/xap/1.0/", "CreatorTool"));
    }
}

mod serialize {
    use xmpkit::{XmpMeta, XmpValue};

    #[test]
    fn serialize_rdf() {
        let mut m = XmpMeta::new();
        m.set_property(
            "http://ns.adobe.com/xap/1.0/",
            "CreatorTool",
            XmpValue::String("TestApp".to_string()),
        )
        .unwrap();

        let serialized = m.serialize().unwrap();
        assert!(serialized.contains("rdf:RDF"));
        assert!(serialized.contains("rdf:Description"));
        assert!(serialized.contains("CreatorTool"));
    }

    #[test]
    fn serialize_packet() {
        let mut m = XmpMeta::new();
        m.set_property(
            "http://ns.adobe.com/xap/1.0/",
            "CreatorTool",
            XmpValue::String("TestApp".to_string()),
        )
        .unwrap();

        let packet = m.serialize_packet().unwrap();
        assert!(packet.contains("<?xpacket"));
        assert!(packet.contains("rdf:RDF"));
    }

    #[test]
    fn round_trip() {
        let original_xmp = r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:xmp="http://ns.adobe.com/xap/1.0/">
  <rdf:Description rdf:about=""
                   xmp:CreatorTool="TestApp"/>
</rdf:RDF>
<?xpacket end="w"?>"#;

        let m1 = original_xmp.parse::<XmpMeta>().unwrap();
        let serialized = m1.serialize_packet().unwrap();
        let m2 = serialized.parse::<XmpMeta>().unwrap();

        assert_eq!(
            m1.get_property("http://ns.adobe.com/xap/1.0/", "CreatorTool"),
            m2.get_property("http://ns.adobe.com/xap/1.0/", "CreatorTool")
        );
    }
}
