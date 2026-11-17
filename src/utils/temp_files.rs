//! RAII guards for the scratch files the OCR extractors write under TEMP_DIR.
//!
//! An extraction future can be dropped at any `.await` (job cancellation,
//! `with_ocr_timeout`, a lost claim), so cleanup cannot live in code that runs
//! after the work finishes. Each guard removes its file(s) when it goes out of
//! scope, on success, on error and on drop alike. Drop is synchronous, so it
//! uses `std::fs`; a file that is already gone is not an error.

use std::path::{Path, PathBuf};

use tracing::{debug, warn};

/// Removes `path`, ignoring a file that is already gone and logging any other
/// failure.
pub fn remove_quietly(path: &Path) {
    match std::fs::remove_file(path) {
        Ok(()) => debug!("Cleaned up temporary file: {}", path.display()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => warn!(
            "Failed to clean up temporary file '{}': {}",
            path.display(),
            e
        ),
    }
}

/// A single temporary file, removed when the guard is dropped unless
/// [`TempPath::keep`] hands it over first.
#[derive(Debug)]
pub struct TempPath(Option<String>);

impl TempPath {
    /// Guards `path`; the file need not exist yet.
    pub fn new(path: impl Into<String>) -> Self {
        Self(Some(path.into()))
    }

    /// The guarded path.
    pub fn as_str(&self) -> &str {
        self.0.as_deref().unwrap_or_default()
    }

    /// Disarms the guard and returns the path; the caller now owns the file.
    pub fn keep(mut self) -> String {
        self.0.take().unwrap_or_default()
    }
}

impl AsRef<str> for TempPath {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Drop for TempPath {
    fn drop(&mut self) {
        if let Some(path) = self.0.take() {
            remove_quietly(Path::new(&path));
        }
    }
}

/// Every page image `pdftoppm` writes for `prefix` (`<prefix>-<N>.<ext>`),
/// including pages written before they were listed: dropping the guard removes
/// whatever matches at that moment. Derefs to the listed page paths.
#[derive(Debug)]
pub struct TempPageImages {
    prefix: PathBuf,
    ext: &'static str,
    paths: Vec<String>,
}

impl TempPageImages {
    /// Guards the page images of the pdftoppm output root `prefix`.
    pub fn new(prefix: impl Into<PathBuf>, ext: &'static str) -> Self {
        Self {
            prefix: prefix.into(),
            ext,
            paths: Vec::new(),
        }
    }

    /// Records the page images found for the prefix, in page order.
    pub fn set_paths(&mut self, paths: Vec<String>) {
        self.paths = paths;
    }

    /// The pdftoppm output root.
    pub fn prefix(&self) -> &Path {
        &self.prefix
    }
}

impl std::ops::Deref for TempPageImages {
    type Target = [String];

    fn deref(&self) -> &[String] {
        &self.paths
    }
}

impl Drop for TempPageImages {
    fn drop(&mut self) {
        match crate::utils::pdftoppm::list_page_images_blocking(&self.prefix, self.ext) {
            Ok(pages) => pages.iter().for_each(|p| remove_quietly(p)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => warn!(
                "Failed to list page images for '{}': {}",
                self.prefix.display(),
                e
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(path: &Path) {
        std::fs::write(path, b"x").unwrap();
    }

    #[test]
    fn temp_path_removes_its_file_on_drop() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.txt");
        touch(&file);
        drop(TempPath::new(file.to_string_lossy()));
        assert!(!file.exists());
    }

    #[test]
    fn temp_path_ignores_a_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        // Never created: dropping must neither panic nor create anything
        drop(TempPath::new(
            dir.path().join("never.txt").to_string_lossy(),
        ));
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn kept_temp_path_survives_the_guard() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("kept.png");
        touch(&file);
        let guard = TempPath::new(file.to_string_lossy());
        assert_eq!(guard.as_str(), file.to_string_lossy());
        assert_eq!(guard.keep(), file.to_string_lossy());
        assert!(file.exists());
    }

    #[test]
    fn page_images_guard_removes_only_its_own_pages() {
        let dir = tempfile::tempdir().unwrap();
        let prefix = dir.path().join("pdf_page_abc");
        for name in [
            "pdf_page_abc-1.png",
            "pdf_page_abc-02.png",
            "pdf_page_abc-003.png",
        ] {
            touch(&dir.path().join(name));
        }
        // Another job's pages and an unrelated file stay
        let other = dir.path().join("pdf_page_abd-1.png");
        let unrelated = dir.path().join("pdf_page_abc-1.txt");
        touch(&other);
        touch(&unrelated);

        let mut guard = TempPageImages::new(&prefix, "png");
        assert_eq!(guard.prefix(), prefix);
        guard.set_paths(vec!["pdf_page_abc-1.png".into()]);
        assert_eq!(&*guard, ["pdf_page_abc-1.png"]);
        drop(guard);

        let mut left: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        left.sort();
        assert_eq!(left, ["pdf_page_abc-1.txt", "pdf_page_abd-1.png"]);
    }

    #[test]
    fn page_images_guard_tolerates_a_missing_directory() {
        let dir = tempfile::tempdir().unwrap();
        drop(TempPageImages::new(
            dir.path().join("gone/pdf_page_x"),
            "png",
        ));
    }
}
