//! Locating the page images `pdftoppm` writes.
//!
//! pdftoppm names each page `<prefix>-<N>.<ext>` and zero-pads `N` to the digit
//! count of the document's *total* page count, not of the requested `-f`/`-l`
//! range: page 1 of a 10-page PDF is `-01.png`, of a 100-page PDF `-001.png`.
//! Guessing `-1.png` silently loses pages (#980), so callers list what pdftoppm
//! actually produced instead of predicting the names.

use std::path::{Path, PathBuf};

/// The page number encoded in a pdftoppm output file name, or `None` when
/// `file_name` is not `<prefix_name>-<digits>.<ext>`.
pub fn page_number(file_name: &str, prefix_name: &str, ext: &str) -> Option<u32> {
    let digits = file_name
        .strip_prefix(prefix_name)?
        .strip_prefix('-')?
        .strip_suffix(ext)?
        .strip_suffix('.')?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// The page images pdftoppm wrote for `prefix` (the output root it was given,
/// without extension), ordered by page number.
pub async fn list_page_images(prefix: &str, ext: &str) -> std::io::Result<Vec<PathBuf>> {
    let prefix = Path::new(prefix);
    let dir = match prefix.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let prefix_name = prefix.file_name().and_then(|n| n.to_str()).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "pdftoppm prefix has no file name",
        )
    })?;

    let mut pages = Vec::new();
    let mut entries = tokio::fs::read_dir(dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name();
        if let Some(page) = name.to_str().and_then(|n| page_number(n, prefix_name, ext)) {
            pages.push((page, entry.path()));
        }
    }
    pages.sort_by_key(|(page, _)| *page);
    Ok(pages.into_iter().map(|(_, path)| path).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_unpadded_and_zero_padded_page_numbers() {
        assert_eq!(page_number("doc-1.png", "doc", "png"), Some(1));
        assert_eq!(page_number("doc-01.png", "doc", "png"), Some(1));
        assert_eq!(page_number("doc-001.png", "doc", "png"), Some(1));
        assert_eq!(page_number("doc-042.png", "doc", "png"), Some(42));
        assert_eq!(page_number("doc-468.png", "doc", "png"), Some(468));
    }

    #[test]
    fn rejects_names_that_are_not_this_prefixes_pages() {
        // A sibling run whose prefix merely starts with ours.
        assert_eq!(page_number("doc_2-1.png", "doc", "png"), None);
        assert_eq!(page_number("doc-1.jpg", "doc", "png"), None);
        assert_eq!(page_number("doc.png", "doc", "png"), None);
        assert_eq!(page_number("doc-.png", "doc", "png"), None);
        assert_eq!(page_number("doc-1-2.png", "doc", "png"), None);
        assert_eq!(page_number("doc-1png", "doc", "png"), None);
    }

    #[tokio::test]
    async fn lists_padded_pages_in_page_order_and_ignores_strangers() {
        let dir = tempfile::tempdir().unwrap();
        for name in [
            "run-10.png",
            "run-02.png",
            "run-01.png",
            "run_other-01.png",
            "unrelated.txt",
        ] {
            std::fs::write(dir.path().join(name), b"").unwrap();
        }
        let prefix = dir.path().join("run");

        let pages = list_page_images(prefix.to_str().unwrap(), "png")
            .await
            .unwrap();

        let names: Vec<_> = pages
            .iter()
            .map(|p| p.file_name().unwrap().to_str().unwrap())
            .collect();
        assert_eq!(names, ["run-01.png", "run-02.png", "run-10.png"]);
    }

    /// Runs the real binary against a 10+ page fixture so a change in
    /// pdftoppm's naming scheme fails here rather than in production.
    #[tokio::test]
    async fn finds_the_first_page_pdftoppm_writes_for_a_long_pdf() {
        let fixture = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_files/porters-handbook_en.pdf"
        );
        let probe = std::process::Command::new("pdftoppm").arg("-v").output();
        if probe.is_err() || !Path::new(fixture).exists() {
            eprintln!("skipping: pdftoppm or {fixture} unavailable");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let prefix = dir.path().join("long");

        let status = std::process::Command::new("pdftoppm")
            .args(["-f", "1", "-l", "2", "-scale-to", "50", "-png", fixture])
            .arg(&prefix)
            .status()
            .unwrap();
        assert!(status.success());

        let pages = list_page_images(prefix.to_str().unwrap(), "png")
            .await
            .unwrap();
        assert_eq!(pages.len(), 2, "expected pages 1 and 2, got {pages:?}");
        assert_eq!(
            page_number(
                pages[0].file_name().unwrap().to_str().unwrap(),
                "long",
                "png"
            ),
            Some(1)
        );
    }
}
