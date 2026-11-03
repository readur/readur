//! Normalised file-extension allow-list shared by every sync source.
//!
//! Every source type (WebDAV, S3, local folder) carries a user-supplied
//! `file_extensions` list. Users type these in many shapes -- `pdf`, `.PDF`,
//! `" Pdf "` -- and an empty list means "everything". Before this helper each
//! source compared the lower-cased on-disk extension against the raw config
//! strings, so `["PDF"]` or `[".pdf"]` silently matched nothing, and an empty
//! list meant "nothing" in some places and "everything" in others.

use std::path::Path;

/// A case-insensitive, dot-insensitive extension allow-list.
///
/// An empty filter allows every file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionFilter {
    allowed: Vec<String>,
}

impl ExtensionFilter {
    /// Builds a filter from user-supplied extensions, normalising each entry
    /// (trim whitespace, strip leading dots, lower-case) and dropping blanks.
    pub fn new<I, S>(extensions: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut allowed: Vec<String> = extensions
            .into_iter()
            .map(|ext| Self::normalise(ext.as_ref()))
            .filter(|ext| !ext.is_empty())
            .collect();
        allowed.sort();
        allowed.dedup();
        Self { allowed }
    }

    /// Normalises a single extension the same way the constructor does.
    pub fn normalise(extension: &str) -> String {
        extension.trim().trim_start_matches('.').trim().to_lowercase()
    }

    /// True when no extensions were configured, meaning every file is allowed.
    pub fn allows_all(&self) -> bool {
        self.allowed.is_empty()
    }

    /// The normalised allow-list (empty when [`Self::allows_all`]).
    pub fn allowed(&self) -> &[String] {
        &self.allowed
    }

    /// Whether a file name (or path) passes the filter, judged by its final
    /// extension. Files without an extension only pass when the filter allows
    /// everything.
    pub fn matches_name(&self, file_name: &str) -> bool {
        if self.allows_all() {
            return true;
        }
        let ext = Path::new(file_name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        self.matches_extension(ext)
    }

    /// Whether a bare extension (with or without a leading dot, any case)
    /// passes the filter.
    pub fn matches_extension(&self, extension: &str) -> bool {
        if self.allows_all() {
            return true;
        }
        let ext = Self::normalise(extension);
        if ext.is_empty() {
            return false;
        }
        self.allowed.binary_search(&ext).is_ok()
    }
}

impl Default for ExtensionFilter {
    fn default() -> Self {
        Self::new(std::iter::empty::<&str>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_filter_allows_everything() {
        let filter = ExtensionFilter::new(Vec::<String>::new());
        assert!(filter.allows_all());
        assert!(filter.matches_name("report.pdf"));
        assert!(filter.matches_name("README"));
        assert!(filter.matches_extension(""));
    }

    #[test]
    fn whitespace_only_entries_count_as_empty() {
        let filter = ExtensionFilter::new(["  ", ".", ""]);
        assert!(filter.allows_all());
    }

    #[test]
    fn normalises_case_dots_and_whitespace() {
        let filter = ExtensionFilter::new([".PDF", " Txt ", "png"]);
        assert_eq!(filter.allowed(), &["pdf", "png", "txt"]);
        assert!(filter.matches_name("scan.PDF"));
        assert!(filter.matches_name("/some/dir/notes.txt"));
        assert!(filter.matches_extension(".PNG"));
        assert!(filter.matches_extension("png"));
    }

    #[test]
    fn rejects_unlisted_and_missing_extensions() {
        let filter = ExtensionFilter::new(["pdf"]);
        assert!(!filter.matches_name("archive.zip"));
        assert!(!filter.matches_name("README"));
        assert!(!filter.matches_name(".hidden"));
        assert!(!filter.matches_extension(""));
    }

    #[test]
    fn uses_only_the_final_extension() {
        let filter = ExtensionFilter::new(["gz"]);
        assert!(filter.matches_name("backup.tar.gz"));
        let tar_only = ExtensionFilter::new(["tar"]);
        assert!(!tar_only.matches_name("backup.tar.gz"));
    }

    #[test]
    fn dedups_entries() {
        let filter = ExtensionFilter::new(["pdf", "PDF", ".pdf"]);
        assert_eq!(filter.allowed(), &["pdf"]);
    }
}
