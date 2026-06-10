use std::collections::HashMap;
use xmpkit::{register_namespace, XmpMeta, XmpValue};

#[test]
fn test_parsing_nested_structs() {
    let xml = r#"
    <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
             xmlns:exif="http://ns.adobe.com/exif/1.0/">
      <rdf:Description rdf:about="">
        <exif:Flash>
          <exif:Fired>True</exif:Fired>
        </exif:Flash>
      </rdf:Description>
    </rdf:RDF>"#;

    let meta = XmpMeta::parse(xml).unwrap();

    // Verify that "Flash" is correctly parsed as a Structure containing the "Fired" field,
    // rather than being incorrectly flattened to root.
    let val = meta.get_struct_field("http://ns.adobe.com/exif/1.0/", "Flash", "Fired");

    assert!(
        val.is_some(),
        "exif:Flash/exif:Fired not found! Flash structure was not created (flattened to root: {})",
        meta.get_property("http://ns.adobe.com/exif/1.0/", "Fired")
            .is_some()
    );
    assert_eq!(val.unwrap().to_string(), "True");
}

#[test]
fn test_serialization_of_nested_struct_field() {
    let mut meta = XmpMeta::new();
    register_namespace("http://ns.google.com/photos/1.0/container/", "Container").unwrap();

    // Set a nested field
    meta.set_struct_field(
        "http://ns.google.com/photos/1.0/container/",
        "Directory[1]/Container:Item",
        "some_field",
        XmpValue::String("value".to_string()),
    )
    .unwrap();

    // Verify that the serializer can serialize nested structure fields successfully
    // without returning BadXPath errors.
    let serialize_result = meta.serialize();

    assert!(
        serialize_result.is_ok(),
        "Serialization failed! Error: {:?}",
        serialize_result.err()
    );
}

#[test]
fn test_value_to_node_complex_types() {
    let mut meta = XmpMeta::new();
    let mut struct_val = HashMap::new();
    struct_val.insert("field".to_string(), XmpValue::String("value".to_string()));

    let val = XmpValue::Structure(struct_val);

    // Verify that complex types like XmpValue::Structure can be successfully
    // converted to Node and set via set_property.
    let result = meta.set_property("http://ns.adobe.com/xap/1.0/", "TestStruct", val);

    assert!(
        result.is_ok(),
        "Failed to set structure property! Error: {:?}",
        result.err()
    );
}

#[test]
fn test_nested_struct_does_not_early_exit() {
    let xml = r#"
    <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
             xmlns:exif="http://ns.adobe.com/exif/1.0/">
      <rdf:Description rdf:about="">
        <exif:Flash>
          <exif:Fired>True</exif:Fired>
        </exif:Flash>
        <exif:ExposureMode>0</exif:ExposureMode>
      </rdf:Description>
    </rdf:RDF>"#;

    let meta = XmpMeta::parse(xml).unwrap();

    // Verify nested struct was parsed
    let flash_fired = meta.get_struct_field("http://ns.adobe.com/exif/1.0/", "Flash", "Fired");
    assert!(flash_fired.is_some());
    assert_eq!(flash_fired.unwrap().to_string(), "True");

    // Verify property after nested struct was ALSO parsed (no early exit)
    let exp_mode = meta.get_property("http://ns.adobe.com/exif/1.0/", "ExposureMode");
    assert!(
        exp_mode.is_some(),
        "exif:ExposureMode not found! Parser exited early after exif:Flash ended"
    );
    assert_eq!(exp_mode.unwrap().to_string(), "0");
}

#[test]
fn test_nested_namespace_serialization() {
    let mut meta = XmpMeta::new();
    // Register some unique namespaces
    register_namespace("http://ns.google.com/photos/1.0/container/", "Container").unwrap();
    register_namespace("http://ns.google.com/photos/1.0/nested/", "Nested").unwrap();

    // Set a nested field that uses "Nested" namespace
    // Directory is in "Container" namespace
    meta.set_struct_field(
        "http://ns.google.com/photos/1.0/container/",
        "Directory[1]/Nested:Struct",
        "field",
        XmpValue::String("value".to_string()),
    )
    .unwrap();

    let serialize_result = meta.serialize();
    assert!(serialize_result.is_ok());
    let xml = serialize_result.unwrap();

    // Verify that the nested namespace was declared in the header
    assert!(
        xml.contains("xmlns:Nested=\"http://ns.google.com/photos/1.0/nested/\""),
        "Serialized XML is missing namespace declaration for nested namespace 'Nested'. XML:\n{}",
        xml
    );
}
