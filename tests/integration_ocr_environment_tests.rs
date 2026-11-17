//! Environment settings of the OCR pipeline: temporary files go to
//! `TEMP_DIR` (or the platform temp directory) through
//! `readur::config::temp_dir`, never to a hardcoded `/tmp`; and
//! `OCR_LOW_MEMORY_MODE`, which no OCR path ever honoured, is gone for good.

use std::path::Path;

/// Server source files (test support excluded) and their text before the
/// first `#[cfg(test)]`, which is where unit tests start.
fn server_sources() -> Vec<(String, String)> {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"));
    let listed = std::process::Command::new("git")
        .args(["ls-files", "src"])
        .current_dir(repo)
        .output()
        .expect("git ls-files");
    String::from_utf8(listed.stdout)
        .unwrap()
        .lines()
        .filter(|path| path.ends_with(".rs") && !path.starts_with("src/tests/"))
        .filter(|path| {
            let name = Path::new(path)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned();
            !(name.starts_with("test_") || name.ends_with("_tests.rs") || name == "tests.rs")
        })
        .filter_map(|path| {
            let text = std::fs::read_to_string(repo.join(path)).ok()?;
            let server_part = text
                .split("#[cfg(test)]")
                .next()
                .unwrap_or_default()
                .to_string();
            Some((path.to_string(), server_part))
        })
        .collect()
}

#[test]
fn no_server_code_hardcodes_tmp() {
    let sources = server_sources();
    assert!(
        sources.len() > 200,
        "found only {} source files",
        sources.len()
    );
    let offenders: Vec<String> = sources
        .iter()
        .flat_map(|(path, text)| {
            text.lines()
                .enumerate()
                .filter(|(_, line)| line.contains("\"/tmp") && !line.trim_start().starts_with("//"))
                .map(move |(n, line)| format!("{path}:{}: {}", n + 1, line.trim()))
        })
        .collect();
    assert!(
        offenders.is_empty(),
        "temporary files must go through readur::config::temp_dir():\n{}",
        offenders.join("\n")
    );
}

#[test]
fn temp_dir_follows_the_setting() {
    assert_eq!(
        readur::config::resolve_temp_dir(Some("/srv/scratch")),
        Path::new("/srv/scratch")
    );
    assert_eq!(readur::config::resolve_temp_dir(None), std::env::temp_dir());
}

/// `OCR_LOW_MEMORY_MODE` was read only by a code path document processing
/// never runs; it was removed rather than left as a switch that does nothing.
/// Memory use is bounded by `CONCURRENT_OCR_JOBS` and the size limits.
#[test]
fn ocr_low_memory_mode_is_neither_read_nor_documented() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"));
    let listed = std::process::Command::new("git")
        .args(["ls-files", "src", "docs", "README.md"])
        .current_dir(repo)
        .output()
        .expect("git ls-files");
    let mentions: Vec<String> = String::from_utf8(listed.stdout)
        .unwrap()
        .lines()
        .filter(|path| path.ends_with(".rs") || path.ends_with(".md"))
        .filter(|path| {
            std::fs::read_to_string(repo.join(path))
                .map(|text| text.contains("OCR_LOW_MEMORY_MODE"))
                .unwrap_or(false)
        })
        .map(str::to_string)
        .collect();
    assert!(
        mentions.is_empty(),
        "OCR_LOW_MEMORY_MODE is mentioned in {mentions:?}"
    );
}
