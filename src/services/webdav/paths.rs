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

/// Canonical form of a configured watch folder: percent-escapes decoded once,
/// exactly one leading `/` and no trailing one (the server root is `/`).
///
/// Watch folders are compared with the decoded href paths that come back from
/// PROPFIND (`/Documents`), so a folder configured as `Documents` or
/// `/Documents/` must name the same path, or the folder's own entry is never
/// recognised and the root is scanned twice. Decoding is needed because
/// paths are now percent-encoded when placed in a request URL: a folder that
/// was configured as `/My%20Docs` to reach a space before that change would
/// otherwise be requested as `/My%2520Docs`. The cost is that a folder whose
/// name literally contains a valid escape sequence must be configured
/// encoded (`%2520` for a literal `%20`). The function is idempotent on its
/// own output except for that case.
pub fn normalize_watch_folder(folder: &str) -> String {
    let decoded = decode_href(folder.trim());
    let trimmed = decoded.trim_matches('/');
    if trimmed.is_empty() { "/".to_string() } else { format!("/{trimmed}") }
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
    fn watch_folders_get_one_leading_slash_and_no_trailing_slash() {
        assert_eq!(normalize_watch_folder("Documents"), "/Documents");
        assert_eq!(normalize_watch_folder("/Documents/"), "/Documents");
        assert_eq!(normalize_watch_folder("//Documents/Sub//"), "/Documents/Sub");
        assert_eq!(normalize_watch_folder(" /Documents "), "/Documents");
        for root in ["", "/", "//", "  "] {
            assert_eq!(normalize_watch_folder(root), "/", "{root:?}");
        }
    }

    #[test]
    fn watch_folders_are_percent_decoded_once() {
        assert_eq!(normalize_watch_folder("/My%20Docs"), "/My Docs");
        assert_eq!(normalize_watch_folder("/My Docs"), "/My Docs");
        assert_eq!(normalize_watch_folder("/caf%C3%A9"), "/café");
        // Invalid escapes are not an error and are kept as typed.
        assert_eq!(normalize_watch_folder("/100%/done"), "/100%/done");
        // Already-normalised output is stable.
        let once = normalize_watch_folder("/My%20Docs/");
        assert_eq!(normalize_watch_folder(&once), once);
    }

    #[test]
    fn decode_then_encode_round_trips() {
        for href in ["/My%20Docs/", "/caf%C3%A9.pdf", "/a%23b%3Fc%2Bd%25e.txt", "/Docs/plain.txt"] {
            assert_eq!(encode_path(&decode_href(href)), href, "round trip of {href}");
        }
    }
}
