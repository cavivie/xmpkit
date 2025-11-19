#[cfg(feature = "mutli-thread")]
#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::thread;
    use xmpkit::XmpMeta;

    #[test]
    fn test_concurrent_reads() {
        let xml = r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:xmp="http://ns.adobe.com/xap/1.0/">
  <rdf:Description rdf:about=""
                   xmp:CreatorTool="TestApp"/>
</rdf:RDF>
<?xpacket end="w"?>"#;

        let meta = Arc::new(XmpMeta::parse(xml).unwrap());

        let mut handles = vec![];

        // Spawn 10 threads that all read concurrently
        for _ in 0..10 {
            let meta_clone = meta.clone();
            let handle = thread::spawn(move || {
                let value = meta_clone.get_property("http://ns.adobe.com/xap/1.0/", "CreatorTool");
                assert_eq!(value, Some(xmpkit::XmpValue::String("TestApp".to_string())));
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn test_concurrent_reads_multiple_properties() {
        let xml = r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:xmp="http://ns.adobe.com/xap/1.0/"
         xmlns:dc="http://purl.org/dc/elements/1.1/">
  <rdf:Description rdf:about=""
                   xmp:CreatorTool="TestApp"
                   dc:title="Test Title"/>
</rdf:RDF>
<?xpacket end="w"?>"#;

        let meta = Arc::new(XmpMeta::parse(xml).unwrap());

        let mut handles = vec![];

        // Spawn threads that read different properties concurrently
        for i in 0..5 {
            let meta_clone = meta.clone();
            let handle = thread::spawn(move || {
                if i % 2 == 0 {
                    let value =
                        meta_clone.get_property("http://ns.adobe.com/xap/1.0/", "CreatorTool");
                    assert_eq!(value, Some(xmpkit::XmpValue::String("TestApp".to_string())));
                } else {
                    let value =
                        meta_clone.get_property("http://purl.org/dc/elements/1.1/", "title");
                    assert_eq!(
                        value,
                        Some(xmpkit::XmpValue::String("Test Title".to_string()))
                    );
                }
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }
    }
}
