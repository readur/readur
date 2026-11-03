//! Translation between WebDAV hrefs (percent-encoded, as sent on the wire) and
//! the decoded paths used everywhere else: watch-folder comparisons, directory
//! tracking rows and `FileIngestionInfo::relative_path`.
//!
//! Decode exactly once, when an href leaves the PROPFIND parser; encode exactly
//! once, when a path is placed in a request URL.

use std::borrow::Cow;

/// Percent-decodes an href from a PROPFIND response. Sequences that are not
/// valid percent-escapes are kept as they are, so a literal `%` survives.
pub fn decode_href(href: &str) -> String {
    match urlencoding::decode(href) {
        Ok(decoded) => decoded.into_owned(),
        Err(_) => href.to_string(),
    }
}

/// Percent-encodes each segment of a decoded path for use in a URL, keeping
/// the `/` separators (and any trailing slash) intact.
pub fn encode_path(path: &str) -> String {
    path.split('/')
        .map(|segment| encode_segment(segment))
        .collect::<Vec<_>>()
        .join("/")
}

fn encode_segment(segment: &str) -> Cow<'_, str> {
    urlencoding::encode(segment)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_spaces_unicode_and_reserved_characters() {
        assert_eq!(decode_href("/My%20Docs/caf%C3%A9%20%23notes%2B1%3F.pdf"), "/My Docs/café #notes+1?.pdf");
        assert_eq!(decode_href("/remote.php/dav/files/j%C3%B6rg/Docs/"), "/remote.php/dav/files/jörg/Docs/");
        assert_eq!(decode_href("/100%25%20done.txt"), "/100% done.txt");
    }

    #[test]
    fn invalid_escapes_are_preserved() {
        assert_eq!(decode_href("/100%.pdf"), "/100%.pdf");
        assert_eq!(decode_href("/a%2.txt"), "/a%2.txt");
        assert_eq!(decode_href("/plain/path.txt"), "/plain/path.txt");
    }

    #[test]
    fn encodes_each_segment_and_keeps_separators() {
        assert_eq!(encode_path("My Docs/café #notes+1?.pdf"), "My%20Docs/caf%C3%A9%20%23notes%2B1%3F.pdf");
        assert_eq!(encode_path("/Docs/Sub/"), "/Docs/Sub/");
        assert_eq!(encode_path("100% done.txt"), "100%25%20done.txt");
        assert_eq!(encode_path(""), "");
    }

    #[test]
    fn decode_then_encode_round_trips() {
        for href in ["/My%20Docs/", "/caf%C3%A9.pdf", "/a%23b%3Fc%2Bd%25e.txt", "/Docs/plain.txt"] {
            assert_eq!(encode_path(&decode_href(href)), href, "round trip of {href}");
        }
    }
}
