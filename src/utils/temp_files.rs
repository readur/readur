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

    /// Guards a fresh, uniquely named file `<dir>/<stem>_<uuid>.<ext>`:
    /// concurrent jobs in one process never share (or delete) each other's
    /// scratch file.
    pub fn unique(dir: &str, stem: &str, ext: &str) -> Self {
        Self::new(format!("{}/{}_{}.{}", dir, stem, uuid::Uuid::new_v4(), ext))
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

/// The page images of one `pdftoppm` render, written into a directory of
/// their own: dropping the guard removes that directory with whatever pages
/// are in it, including pages written before they were listed, and never
/// scans the shared temp dir. Derefs to the listed page paths.
#[derive(Debug)]
pub struct TempPageImages {
    dir: PathBuf,
    paths: Vec<String>,
}

impl TempPageImages {
    /// Creates a fresh `<parent>/<stem>_<uuid>` directory for one render.
    /// The guard exists before the directory does, so no way out leaks it.
    pub async fn create(parent: &str, stem: &str) -> std::io::Result<Self> {
        let pages = Self {
            dir: Path::new(parent).join(format!("{}_{}", stem, uuid::Uuid::new_v4())),
            paths: Vec::new(),
        };
        tokio::fs::create_dir(&pages.dir).await?;
        Ok(pages)
    }

    /// Records the page images found for the render, in page order.
    pub fn set_paths(&mut self, paths: Vec<String>) {
        self.paths = paths;
    }

    /// The pdftoppm output root, inside the render's directory.
    pub fn prefix(&self) -> PathBuf {
        self.dir.join("page")
    }

    /// The render's directory.
    pub fn dir(&self) -> &Path {
        &self.dir
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
        // Bounded to this render's own pages.
        match std::fs::remove_dir_all(&self.dir) {
            Ok(()) => debug!("Cleaned up page images in {}", self.dir.display()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => warn!(
                "Failed to clean up page images in '{}': {}",
                self.dir.display(),
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
    fn unique_temp_paths_never_collide() {
        let a = TempPath::unique("/tmp/x", "quick_text", "txt");
        let b = TempPath::unique("/tmp/x", "quick_text", "txt");
        assert_ne!(a.as_str(), b.as_str());
        assert!(a.as_str().starts_with("/tmp/x/quick_text_"));
        assert!(a.as_str().ends_with(".txt"));
        // Never created: nothing to remove.
        let _ = (a.keep(), b.keep());
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

    #[tokio::test]
    async fn page_images_live_in_their_own_directory_and_leave_with_it() {
        let dir = tempfile::tempdir().unwrap();
        let parent = dir.path().to_str().unwrap();
        // Another job's file in the shared temp dir, with a page-like name
        let other = dir.path().join("page-1.png");
        touch(&other);

        let mut guard = TempPageImages::create(parent, "pdf_pages").await.unwrap();
        let second = TempPageImages::create(parent, "pdf_pages").await.unwrap();
        assert_ne!(
            guard.dir(),
            second.dir(),
            "each render gets its own directory"
        );
        assert_eq!(guard.prefix().parent(), Some(guard.dir()));
        assert_eq!(guard.dir().parent(), Some(dir.path()));
        // Pages written whether or not they were listed
        for page in ["page-01.png", "page-02.png"] {
            touch(&guard.dir().join(page));
        }
        guard.set_paths(vec!["page-01.png".into()]);
        assert_eq!(&*guard, ["page-01.png"]);
        let gone = guard.dir().to_path_buf();
        drop(guard);

        assert!(!gone.exists());
        assert!(second.dir().exists(), "another render's pages stay");
        assert!(other.exists(), "the shared temp dir is not swept");
        drop(second);
    }

    #[tokio::test]
    async fn page_images_guard_tolerates_a_vanished_directory() {
        let dir = tempfile::tempdir().unwrap();
        let guard = TempPageImages::create(dir.path().to_str().unwrap(), "pdf_pages")
            .await
            .unwrap();
        std::fs::remove_dir(guard.dir()).unwrap();
        drop(guard);
    }

    #[tokio::test]
    async fn page_images_creation_fails_without_a_parent() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("gone");
        assert!(
            TempPageImages::create(missing.to_str().unwrap(), "pdf_pages")
                .await
                .is_err()
        );
    }
}
