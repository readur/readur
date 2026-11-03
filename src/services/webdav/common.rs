/// Common utilities and shared functions for WebDAV services

use anyhow::{anyhow, Result};

/// Build a standardized User-Agent string for all WebDAV requests
/// This ensures consistent identification across all WebDAV operations
pub fn build_user_agent() -> String {
    format!("Readur/{} (WebDAV-Sync; +https://github.com/readur)",
            env!("CARGO_PKG_VERSION"))
}

/// Checks that a downloaded body is the complete file and not an HTML page
/// (for example a login page served by a proxy in front of the WebDAV server).
///
/// `content_type` is the media type from the response without parameters, and
/// `content_length` the declared length when the server sent one.
pub fn verify_download(
    path: &str,
    file_name: &str,
    body: &[u8],
    content_length: Option<u64>,
    content_type: Option<&str>,
) -> Result<()> {
    if let Some(expected) = content_length {
        if body.len() as u64 != expected {
            return Err(anyhow!(
                "Incomplete download for '{}': received {} of {} bytes",
                path, body.len(), expected
            ));
        }
    }

    let extension = file_name.rsplit_once('.').map(|(_, ext)| ext.to_ascii_lowercase());
    let expects_html = matches!(extension.as_deref(), Some("html") | Some("htm") | Some("xhtml"));
    if expects_html {
        return Ok(());
    }

    let declared_html = content_type.is_some_and(|ct| ct == "text/html" || ct == "application/xhtml+xml");
    let unspecified = content_type.map_or(true, |ct| ct == "application/octet-stream");
    if declared_html || (unspecified && looks_like_html(body)) {
        return Err(anyhow!(
            "Download for '{}' returned an HTML page instead of file content (Content-Type: {}); \
             the server may be answering with a login or error page",
            path, content_type.unwrap_or("none")
        ));
    }
    Ok(())
}

/// True when the body starts (after whitespace or a BOM) with an HTML document marker.
fn looks_like_html(body: &[u8]) -> bool {
    let head = &body[..body.len().min(512)];
    let text = String::from_utf8_lossy(head);
    let trimmed = text.trim_start_matches('\u{feff}').trim_start().to_ascii_lowercase();
    trimmed.starts_with("<!doctype html") || trimmed.starts_with("<html")
}

/// Parses a `Retry-After` header value (delta-seconds or HTTP-date) into a wait
/// duration, capped at `max`. Dates in the past yield zero. Returns `None` for
/// values that are neither.
pub fn retry_after_delay(value: &str, now: chrono::DateTime<chrono::Utc>, max: std::time::Duration) -> Option<std::time::Duration> {
    let value = value.trim();
    let seconds = if let Ok(secs) = value.parse::<u64>() {
        secs
    } else {
        let at = chrono::DateTime::parse_from_rfc2822(value).ok()?.with_timezone(&chrono::Utc);
        (at - now).num_seconds().max(0) as u64
    };
    Some(std::time::Duration::from_secs(seconds).min(max))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn complete_body_with_matching_length_is_accepted() {
        assert!(verify_download("/a/report.pdf", "report.pdf", b"%PDF-1.4 ...", Some(12), Some("application/pdf")).is_ok());
        assert!(verify_download("/a/report.pdf", "report.pdf", b"%PDF-1.4 ...", None, None).is_ok());
    }

    #[test]
    fn short_body_is_rejected() {
        let err = verify_download("/a/report.pdf", "report.pdf", b"%PDF", Some(1000), Some("application/pdf")).unwrap_err();
        assert!(err.to_string().contains("received 4 of 1000 bytes"), "{err}");
    }

    #[test]
    fn html_content_type_for_non_html_file_is_rejected() {
        let err = verify_download("/a/report.pdf", "report.pdf", b"<html><body>login</body></html>", None, Some("text/html")).unwrap_err();
        assert!(err.to_string().contains("HTML page"), "{err}");
    }

    #[test]
    fn html_body_without_content_type_is_rejected() {
        assert!(verify_download("/a/scan.jpg", "scan.jpg", b"\n  <!DOCTYPE HTML><html>", None, None).is_err());
        assert!(verify_download("/a/scan.jpg", "scan.jpg", b"\xEF\xBB\xBF<html>", None, Some("application/octet-stream")).is_err());
    }

    #[test]
    fn html_files_and_typed_text_are_accepted() {
        assert!(verify_download("/a/page.html", "page.html", b"<html></html>", None, Some("text/html")).is_ok());
        assert!(verify_download("/a/page.HTM", "page.HTM", b"<html></html>", None, None).is_ok());
        // A text file that happens to contain markup, served with its real type.
        assert!(verify_download("/a/notes.txt", "notes.txt", b"<html>draft</html>", None, Some("text/plain")).is_ok());
    }

    #[test]
    fn retry_after_seconds_and_dates() {
        let now = chrono::DateTime::parse_from_rfc2822("Wed, 21 Oct 2015 07:28:00 GMT").unwrap().with_timezone(&chrono::Utc);
        let max = Duration::from_secs(120);
        assert_eq!(retry_after_delay("5", now, max), Some(Duration::from_secs(5)));
        assert_eq!(retry_after_delay(" 7 ", now, max), Some(Duration::from_secs(7)));
        assert_eq!(retry_after_delay("500", now, max), Some(max));
        assert_eq!(retry_after_delay("Wed, 21 Oct 2015 07:28:30 GMT", now, max), Some(Duration::from_secs(30)));
        assert_eq!(retry_after_delay("Wed, 21 Oct 2015 07:00:00 GMT", now, max), Some(Duration::ZERO));
        assert_eq!(retry_after_delay("soon", now, max), None);
        assert_eq!(retry_after_delay("", now, max), None);
    }
}
