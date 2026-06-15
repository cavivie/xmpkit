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

    const GOOGLE_PANO_XMP: &str = r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
    <x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="XMP Core 5.5.0">
        <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
            <rdf:Description
                rdf:about=""
                xmlns:GPano="http://ns.google.com/photos/1.0/panorama/"
                xmlns:GCamera="http://ns.google.com/photos/1.0/camera/"
                GPano:UsePanoramaViewer="True"
                GPano:IsPhotosphere="True"
                GPano:ProjectionType="equirectangular"
                GPano:CroppedAreaImageHeightPixels="4000"
                GPano:CroppedAreaImageWidthPixels="8000"
                GPano:FullPanoHeightPixels="4000"
                GPano:FullPanoWidthPixels="8000"
                GPano:CroppedAreaTopPixels="0"
                GPano:CroppedAreaLeftPixels="0"
                GPano:FirstPhotoDate="2022-05-17T10:29:50+00:00"
                GPano:LastPhotoDate="2022-05-17T10:32:42+00:00"
                GPano:SourcePhotosCount="38"
                GPano:PoseHeadingDegrees="276"
                GPano:LargestValidInteriorRectLeft="0"
                GPano:LargestValidInteriorRectTop="0"
                GPano:LargestValidInteriorRectWidth="8704"
                GPano:LargestValidInteriorRectHeight="4352"
            >
                <GCamera:SpecialTypeID>
                    <rdf:Bag>
                        <rdf:li>com.google.android.apps.camera.gallery.specialtype.SpecialType-PHOTOSPHERE</rdf:li>
                    </rdf:Bag>
                </GCamera:SpecialTypeID>
            </rdf:Description>
        </rdf:RDF>
    </x:xmpmeta>
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
    fn parse_google_pano_xmp() {
        let m = GOOGLE_PANO_XMP.parse::<XmpMeta>().unwrap();

        let gp_ns = "http://ns.google.com/photos/1.0/panorama/";
        assert_eq!(
            m.get_property(gp_ns, "UsePanoramaViewer"),
            Some(XmpValue::String("True".to_string()))
        );
        assert_eq!(
            m.get_property(gp_ns, "IsPhotosphere"),
            Some(XmpValue::String("True".to_string()))
        );
        assert_eq!(
            m.get_property(gp_ns, "ProjectionType"),
            Some(XmpValue::String("equirectangular".to_string()))
        );
        assert_eq!(
            m.get_property(gp_ns, "CroppedAreaImageHeightPixels"),
            Some(XmpValue::String("4000".to_string()))
        );
        assert_eq!(
            m.get_property(gp_ns, "CroppedAreaImageWidthPixels"),
            Some(XmpValue::String("8000".to_string()))
        );
        assert_eq!(
            m.get_property(gp_ns, "FullPanoHeightPixels"),
            Some(XmpValue::String("4000".to_string()))
        );
        assert_eq!(
            m.get_property(gp_ns, "FullPanoWidthPixels"),
            Some(XmpValue::String("8000".to_string()))
        );
        assert_eq!(
            m.get_property(gp_ns, "CroppedAreaTopPixels"),
            Some(XmpValue::String("0".to_string()))
        );
        assert_eq!(
            m.get_property(gp_ns, "CroppedAreaLeftPixels"),
            Some(XmpValue::String("0".to_string()))
        );
        assert_eq!(
            m.get_property(gp_ns, "FirstPhotoDate"),
            Some(XmpValue::String("2022-05-17T10:29:50+00:00".to_string()))
        );
        assert_eq!(
            m.get_property(gp_ns, "LastPhotoDate"),
            Some(XmpValue::String("2022-05-17T10:32:42+00:00".to_string()))
        );
        assert_eq!(
            m.get_property(gp_ns, "SourcePhotosCount"),
            Some(XmpValue::String("38".to_string()))
        );
        assert_eq!(
            m.get_property(gp_ns, "PoseHeadingDegrees"),
            Some(XmpValue::String("276".to_string()))
        );
        assert_eq!(
            m.get_property(gp_ns, "LargestValidInteriorRectLeft"),
            Some(XmpValue::String("0".to_string()))
        );
        assert_eq!(
            m.get_property(gp_ns, "LargestValidInteriorRectTop"),
            Some(XmpValue::String("0".to_string()))
        );
        assert_eq!(
            m.get_property(gp_ns, "LargestValidInteriorRectWidth"),
            Some(XmpValue::String("8704".to_string()))
        );
        assert_eq!(
            m.get_property(gp_ns, "LargestValidInteriorRectHeight"),
            Some(XmpValue::String("4352".to_string()))
        );

        let gc_ns = "http://ns.google.com/photos/1.0/camera/";
        assert_eq!(m.get_array_size(gc_ns, "SpecialTypeID"), Some(1));
        assert_eq!(
            m.get_array_item(gc_ns, "SpecialTypeID", 0),
            Some(XmpValue::String(
                "com.google.android.apps.camera.gallery.specialtype.SpecialType-PHOTOSPHERE"
                    .to_string()
            ))
        );
        assert_eq!(
            m.get_property(gc_ns, "SpecialTypeID[1]"),
            Some(XmpValue::String(
                "com.google.android.apps.camera.gallery.specialtype.SpecialType-PHOTOSPHERE"
                    .to_string()
            ))
        );
    }

    #[test]
    fn parse_mwg_regions_with_description() {
        let xmp_str = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about=""
    xmlns:mwg-rs="http://www.metadataworkinggroup.com/schemas/regions/">
   <mwg-rs:Regions>
    <mwg-rs:RegionList>
     <rdf:Seq>
      <rdf:li>
       <mwg-rs:Description>This is a region description</mwg-rs:Description>
      </rdf:li>
     </rdf:Seq>
    </mwg-rs:RegionList>
   </mwg-rs:Regions>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>"#;

        let meta = xmp_str.parse::<XmpMeta>().unwrap();
        let path = "Regions/mwg-rs:RegionList[1]/mwg-rs:Description";
        let ns = "http://www.metadataworkinggroup.com/schemas/regions/";

        assert_eq!(
            meta.get_property(ns, path),
            Some(XmpValue::String("This is a region description".to_string()))
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
