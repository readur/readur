//! ETags the PROPFIND parser makes up for servers that omit `getetag`: a
//! stable one when `getlastmodified` is present, and one that never repeats
//! when there is no change information at all, so such entries are treated
//! as changed rather than as unchanged forever.

use readur::webdav_xml_parser::{parse_propfind_response, parse_propfind_response_with_directories};

fn multistatus(props: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
        <d:multistatus xmlns:d="DAV:">
            <d:response>
                <d:href>/webdav/Docs/</d:href>
                <d:propstat>
                    <d:prop>
                        <d:displayname>Docs</d:displayname>
                        <d:resourcetype><d:collection/></d:resourcetype>
                        {props}
                    </d:prop>
                    <d:status>HTTP/1.1 200 OK</d:status>
                </d:propstat>
            </d:response>
            <d:response>
                <d:href>/webdav/Docs/a.pdf</d:href>
                <d:propstat>
                    <d:prop>
                        <d:displayname>a.pdf</d:displayname>
                        <d:resourcetype/>
                        <d:getcontentlength>10</d:getcontentlength>
                        {props}
                    </d:prop>
                    <d:status>HTTP/1.1 200 OK</d:status>
                </d:propstat>
            </d:response>
        </d:multistatus>"#
    )
}

fn etags_of(xml: &str) -> Vec<String> {
    let mut etags: Vec<String> = parse_propfind_response_with_directories(xml).unwrap().into_iter().map(|e| e.etag).collect();
    etags.extend(parse_propfind_response(xml).unwrap().into_iter().map(|e| e.etag));
    assert_eq!(etags.len(), 3, "a directory and a file from the directory parser, the file from the file parser");
    etags
}

#[test]
fn missing_etag_with_last_modified_gives_a_stable_fallback() {
    let xml = multistatus("<d:getlastmodified>Tue, 29 Jul 2025 01:34:17 GMT</d:getlastmodified>");
    let first = etags_of(&xml);
    let second = etags_of(&xml);
    assert_eq!(first, second, "the same properties give the same ETag between listings");
    assert_eq!(first[0], "fallback-Tue, 29 Jul 2025 01:34:17 GMT-0");
    assert_eq!(first[1], "fallback-Tue, 29 Jul 2025 01:34:17 GMT-10");
}

#[test]
fn missing_etag_and_last_modified_never_compares_equal() {
    let xml = multistatus("");
    let first = etags_of(&xml);
    let second = etags_of(&xml);
    for (a, b) in first.iter().zip(&second) {
        assert!(a.starts_with("fallback-unknown-"), "{a}");
        assert_ne!(a, b, "no change information must not look unchanged");
    }
    assert_ne!(first[0], first[1], "entries in one listing do not share a value either");
}
