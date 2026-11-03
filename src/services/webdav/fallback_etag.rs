//! Stand-in ETags for PROPFIND entries whose server omits `getetag`.
//!
//! Directory ETags are what smart sync compares between runs to decide
//! whether a folder needs scanning, so the stand-in must only compare equal
//! when the entry is known to be unchanged. With `getlastmodified` present
//! the value is derived from it (and the size) and is stable. Without it
//! there is no change information at all, and the value is unique per
//! parse so the entry is treated as changed and scanned every time rather
//! than skipped forever.

use uuid::Uuid;

/// Prefix of the stand-in produced when an entry carries no change information.
pub const UNKNOWN_FALLBACK_PREFIX: &str = "fallback-unknown-";

pub fn fallback_etag(last_modified: Option<&str>, content_length: Option<i64>) -> String {
    match last_modified {
        Some(modified) => format!("fallback-{}-{}", modified, content_length.unwrap_or(0)),
        None => format!("{}{}", UNKNOWN_FALLBACK_PREFIX, Uuid::new_v4()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_modified_gives_a_stable_value() {
        let modified = Some("Tue, 29 Jul 2025 01:34:17 GMT");
        assert_eq!(fallback_etag(modified, Some(10)), "fallback-Tue, 29 Jul 2025 01:34:17 GMT-10");
        assert_eq!(fallback_etag(modified, None), "fallback-Tue, 29 Jul 2025 01:34:17 GMT-0");
        assert_eq!(fallback_etag(modified, Some(10)), fallback_etag(modified, Some(10)));
    }

    #[test]
    fn no_change_information_never_repeats() {
        let first = fallback_etag(None, Some(10));
        let second = fallback_etag(None, Some(10));
        assert!(first.starts_with(UNKNOWN_FALLBACK_PREFIX), "{first}");
        assert_ne!(first, second);
    }
}
