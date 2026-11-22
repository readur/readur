//! Where Readur writes temporary files (`TEMP_DIR`).

use std::path::PathBuf;

/// The directory every temporary file goes to: `TEMP_DIR` when it is set to
/// a non-empty value, otherwise the platform's temporary directory
/// (`std::env::temp_dir()`, which itself honours `TMPDIR` on Unix).
///
/// Read on every call rather than once at startup so the OCR workers, the
/// thumbnail generator and the health check always agree.
pub fn temp_dir() -> PathBuf {
    resolve_temp_dir(std::env::var("TEMP_DIR").ok().as_deref())
}

/// `temp_dir` for a given `TEMP_DIR` value.
pub fn resolve_temp_dir(configured: Option<&str>) -> PathBuf {
    match configured.map(str::trim) {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => std::env::temp_dir(),
    }
}

/// `temp_dir` as a string, for the APIs that build paths with `format!`.
pub fn temp_dir_string() -> String {
    temp_dir().to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configured_directory_wins() {
        assert_eq!(
            resolve_temp_dir(Some("/srv/readur-tmp")),
            PathBuf::from("/srv/readur-tmp")
        );
    }

    #[test]
    fn unset_or_blank_falls_back_to_the_platform_directory() {
        assert_eq!(resolve_temp_dir(None), std::env::temp_dir());
        assert_eq!(resolve_temp_dir(Some("")), std::env::temp_dir());
        assert_eq!(resolve_temp_dir(Some("  ")), std::env::temp_dir());
    }
}
