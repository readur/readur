//! Pure decisions the folder watcher makes about a file: whether it is
//! eligible for ingestion, where it sits relative to the watched roots, and
//! how it is queued for OCR. Kept free of I/O so each rule is unit-tested.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::config::Config;

/// Files larger than this (500 MB) are never ingested from a watch folder.
pub(crate) const MAX_FILE_SIZE: i64 = 500 * 1024 * 1024;

/// Hidden, temporary and OS bookkeeping files (`.foo`, `~$doc`, `*.tmp`,
/// `$RECYCLE.BIN`, `System Volume Information`) that are never ingested.
pub(crate) fn is_system_or_temp_file(filename: &str) -> bool {
    filename.starts_with('.')
        || filename.starts_with('~')
        || filename.ends_with(".tmp")
        || filename.ends_with(".temp")
        || filename.contains("$RECYCLE.BIN")
        || filename.contains("System Volume Information")
}

/// Where a file sits relative to the directories the watcher cares about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileLocation {
    /// Inside the upload directory, which WebDAV and manual uploads manage.
    UploadDir,
    /// Neither in the global watch folder nor in a per-user watch directory.
    OutsideWatchDirs,
    /// Inside a watched directory: eligible for ingestion.
    WatchDir,
}

/// The canonical roots a watched file is classified against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WatchRoots {
    pub upload: PathBuf,
    pub global_watch: PathBuf,
    /// Set when per-user watch directories are enabled.
    pub user_watch: Option<PathBuf>,
}

impl WatchRoots {
    /// The roots from `config`, canonicalised where they exist (as given
    /// otherwise).
    pub fn from_config(config: &Config) -> Self {
        Self {
            upload: canonical_or_given(&config.upload_path),
            global_watch: canonical_or_given(&config.watch_folder),
            user_watch: config
                .enable_per_user_watch
                .then(|| canonical_or_given(&config.user_watch_base_dir)),
        }
    }

    /// Classify a canonical file path. The upload directory wins over the
    /// watch directories, so a watch folder nested in it is still skipped.
    pub fn classify(&self, file_canonical: &Path) -> FileLocation {
        if file_canonical.starts_with(&self.upload) {
            return FileLocation::UploadDir;
        }
        let in_global_watch = file_canonical.starts_with(&self.global_watch);
        let in_user_watch = self
            .user_watch
            .as_ref()
            .is_some_and(|user_watch| file_canonical.starts_with(user_watch));
        if in_global_watch || in_user_watch {
            FileLocation::WatchDir
        } else {
            FileLocation::OutsideWatchDirs
        }
    }
}

fn canonical_or_given(path: &str) -> PathBuf {
    Path::new(path)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(path))
}

/// How long ago a file was created; zero for a creation time in the future.
pub(crate) fn age_since(created: SystemTime, now: SystemTime) -> Duration {
    now.duration_since(created).unwrap_or_default()
}

/// Whether a file of `age` is past the `max_age_hours` cut-off.
pub(crate) fn is_too_old(age: Duration, max_age_hours: u64) -> bool {
    age.as_secs() > max_age_hours * 3600
}

/// Why a read file's size rules it out, if it does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SizeSkip {
    TooLarge,
    Empty,
}

pub(crate) fn size_skip(file_size: i64) -> Option<SizeSkip> {
    if file_size > MAX_FILE_SIZE {
        Some(SizeSkip::TooLarge)
    } else if file_size == 0 {
        Some(SizeSkip::Empty)
    } else {
        None
    }
}

/// A file is stable when neither its size nor its modification time changed
/// between two samples taken a stability window apart.
pub(crate) fn is_unchanged(
    before: (u64, Option<SystemTime>),
    after: (u64, Option<SystemTime>),
) -> bool {
    before == after
}

pub(crate) fn is_text_extractable_file(mime_type: &str) -> bool {
    // Check mime types that support text extraction (OCR + Office documents + plain text)
    matches!(
        mime_type,
        // OCR-able files
        "application/pdf" |
        "image/png" | "image/jpeg" | "image/jpg" |
        "image/tiff" | "image/bmp" | "image/gif" |
        // Plain text
        "text/plain" |
        // Office document formats
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" | // DOCX
        "application/msword" |                                                      // DOC
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" |      // XLSX
        "application/vnd.ms-excel" |                                                // XLS
        "application/vnd.openxmlformats-officedocument.presentationml.presentation" // PPTX (for future)
    )
}

/// Calculate priority based on file size and type (smaller files and images get higher priority)
pub(crate) fn calculate_priority(file_size: i64, mime_type: &str) -> i32 {
    let base_priority = crate::ocr::priority::priority_for_file_size(file_size);

    // Boost priority for images (usually faster to OCR)
    let type_boost = if mime_type.starts_with("image/") {
        2
    } else if mime_type == "text/plain" {
        1
    } else {
        0
    };

    (base_priority + type_boost).min(10)
}

/// Check if the given bytes represent a valid PDF file
/// Handles PDFs with leading null bytes or whitespace
pub(crate) fn is_valid_pdf(data: &[u8]) -> bool {
    if data.len() < 5 {
        return false;
    }

    // Find the first occurrence of "%PDF-" in the first 1KB of the file
    // Some PDFs have leading null bytes or other metadata
    let search_limit = data.len().min(1024);
    let search_data = &data[0..search_limit];

    for i in 0..=search_limit.saturating_sub(5) {
        if &search_data[i..i + 5] == b"%PDF-" {
            return true;
        }
    }

    false
}

/// The first 50 bytes of a file as printable ASCII, other bytes shown as
/// `.`, for logging why a file was rejected. Files shorter than 50 bytes
/// show an empty header (their whole length is logged alongside).
pub(crate) fn header_preview(data: &[u8]) -> String {
    data.get(0..50)
        .unwrap_or(&[])
        .iter()
        .map(|&b| {
            if (32..=126).contains(&b) {
                b as char
            } else {
                '.'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_and_temp_files_are_recognised() {
        for name in [
            ".hidden.pdf",
            "~$report.docx",
            "scan.tmp",
            "scan.temp",
            "$RECYCLE.BIN",
            "System Volume Information",
        ] {
            assert!(is_system_or_temp_file(name), "{name}");
        }
        for name in ["report.pdf", "a.tmp.pdf", "temp.png", "my~file.pdf", ""] {
            assert!(!is_system_or_temp_file(name), "{name}");
        }
    }

    fn roots(user_watch: Option<&str>) -> WatchRoots {
        WatchRoots {
            upload: PathBuf::from("/data/uploads"),
            global_watch: PathBuf::from("/data/watch"),
            user_watch: user_watch.map(PathBuf::from),
        }
    }

    #[test]
    fn files_in_the_upload_dir_are_classified_first() {
        let mut nested = roots(None);
        nested.global_watch = PathBuf::from("/data/uploads/watch");
        assert_eq!(
            nested.classify(Path::new("/data/uploads/watch/a.pdf")),
            FileLocation::UploadDir
        );
        assert_eq!(
            roots(None).classify(Path::new("/data/uploads/a.pdf")),
            FileLocation::UploadDir
        );
    }

    #[test]
    fn files_in_global_or_user_watch_dirs_are_eligible() {
        let r = roots(Some("/data/user_watch"));
        assert_eq!(
            r.classify(Path::new("/data/watch/sub/a.pdf")),
            FileLocation::WatchDir
        );
        assert_eq!(
            r.classify(Path::new("/data/user_watch/alice/a.pdf")),
            FileLocation::WatchDir
        );
    }

    #[test]
    fn user_watch_dirs_only_count_when_enabled() {
        assert_eq!(
            roots(None).classify(Path::new("/data/user_watch/alice/a.pdf")),
            FileLocation::OutsideWatchDirs
        );
        assert_eq!(
            roots(Some("/data/user_watch")).classify(Path::new("/etc/a.pdf")),
            FileLocation::OutsideWatchDirs
        );
    }

    #[test]
    fn classification_matches_whole_path_components() {
        assert_eq!(
            roots(None).classify(Path::new("/data/watchers/a.pdf")),
            FileLocation::OutsideWatchDirs
        );
    }

    #[test]
    fn age_is_zero_for_future_creation_times() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(10_000);
        assert_eq!(
            age_since(now - Duration::from_secs(60), now),
            Duration::from_secs(60)
        );
        assert_eq!(
            age_since(now + Duration::from_secs(60), now),
            Duration::ZERO
        );
    }

    #[test]
    fn too_old_is_strictly_past_the_cutoff() {
        assert!(!is_too_old(Duration::from_secs(3600), 1));
        assert!(is_too_old(Duration::from_secs(3601), 1));
        assert!(is_too_old(Duration::from_secs(1), 0));
        assert!(!is_too_old(Duration::ZERO, 0));
    }

    #[test]
    fn size_rules_out_empty_and_oversized_files() {
        assert_eq!(size_skip(0), Some(SizeSkip::Empty));
        assert_eq!(size_skip(1), None);
        assert_eq!(size_skip(MAX_FILE_SIZE), None);
        assert_eq!(size_skip(MAX_FILE_SIZE + 1), Some(SizeSkip::TooLarge));
    }

    #[test]
    fn stability_needs_both_size_and_mtime_unchanged() {
        let t = Some(SystemTime::UNIX_EPOCH + Duration::from_secs(5));
        let later = Some(SystemTime::UNIX_EPOCH + Duration::from_secs(6));
        assert!(is_unchanged((10, t), (10, t)));
        assert!(is_unchanged((10, None), (10, None)));
        assert!(!is_unchanged((10, t), (11, t)));
        assert!(!is_unchanged((10, t), (10, later)));
        assert!(!is_unchanged((10, t), (10, None)));
    }

    #[test]
    fn text_extractable_types_cover_ocr_text_and_office() {
        for mime in [
            "application/pdf",
            "image/png",
            "image/jpeg",
            "image/tiff",
            "text/plain",
            "application/msword",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "application/vnd.ms-excel",
        ] {
            assert!(is_text_extractable_file(mime), "{mime}");
        }
        for mime in ["application/octet-stream", "video/mp4", "image/webp"] {
            assert!(!is_text_extractable_file(mime), "{mime}");
        }
    }

    #[test]
    fn priority_boosts_images_and_text_capped_at_ten() {
        const MB: i64 = 1024 * 1024;
        assert_eq!(calculate_priority(100 * MB, "image/png"), 4);
        assert_eq!(calculate_priority(100 * MB, "text/plain"), 3);
        assert_eq!(calculate_priority(1, "image/png"), 10);
    }

    #[test]
    fn valid_pdf_header_may_follow_leading_bytes_within_1kb() {
        assert!(is_valid_pdf(b"%PDF-1.7\n"));
        let mut padded = vec![0u8; 1000];
        padded.extend_from_slice(b"%PDF-1.4");
        assert!(is_valid_pdf(&padded));
        let mut too_late = vec![0u8; 1024];
        too_late.extend_from_slice(b"%PDF-1.4");
        assert!(!is_valid_pdf(&too_late));
        assert!(!is_valid_pdf(b"%PDF"));
        assert!(!is_valid_pdf(b"not a pdf at all"));
    }

    #[test]
    fn header_preview_masks_unprintable_bytes_and_stops_at_50() {
        let mut data = b"%PDF\x00\n~".to_vec();
        data.resize(80, b'a');
        assert_eq!(header_preview(&data), format!("%PDF..~{}", "a".repeat(43)));
        assert_eq!(header_preview(&[b'a'; 49]), "");
        assert_eq!(header_preview(b""), "");
    }
}
