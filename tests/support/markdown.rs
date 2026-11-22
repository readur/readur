//! Markdown reading shared by the documentation tests.
//!
//! Included with `#[path = "support/markdown.rs"] mod markdown;`.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn repo_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

pub fn read(relative: &str) -> String {
    std::fs::read_to_string(repo_path(relative)).unwrap()
}

/// The level of a Markdown heading line (`## x` is 2), or 0 for any other line.
pub fn heading_level(line: &str) -> usize {
    let hashes = line.chars().take_while(|&c| c == '#').count();
    if hashes > 0 && line[hashes..].starts_with(' ') {
        hashes
    } else {
        0
    }
}

/// A heading and the text under it, up to the next heading of the same or a higher level.
#[derive(Debug, Clone)]
pub struct Section {
    pub heading: String,
    pub body: String,
}

/// Every section of `doc` whose heading has exactly `level` hashes, in order. Headings inside
/// fenced code blocks (shell comments) are not headings.
pub fn sections(doc: &str, level: usize) -> Vec<Section> {
    let mut out: Vec<Section> = Vec::new();
    let mut open: Option<Section> = None;
    let mut in_code = false;
    for line in doc.lines() {
        if line.trim_start().starts_with("```") {
            in_code = !in_code;
        }
        let found = if in_code { 0 } else { heading_level(line) };
        if found > 0 && found <= level {
            out.extend(open.take());
            if found == level {
                open = Some(Section {
                    heading: line.to_string(),
                    body: String::new(),
                });
            }
            continue;
        }
        if let Some(section) = &mut open {
            section.body.push_str(line);
            section.body.push('\n');
        }
    }
    out.extend(open);
    out
}

/// A fenced code block: its info string (`json`, `http`, ...) and contents. `preceded_by` is
/// the last non-blank line before the opening fence.
#[derive(Debug, Clone)]
pub struct CodeBlock {
    pub lang: String,
    pub text: String,
    pub preceded_by: String,
}

/// The fenced code blocks of `text`, in order.
pub fn code_blocks(text: &str) -> Vec<CodeBlock> {
    let mut blocks = Vec::new();
    let mut current: Option<CodeBlock> = None;
    let mut last_line = String::new();
    for line in text.lines() {
        let fence = line.trim_start().strip_prefix("```");
        match (&mut current, fence) {
            (None, Some(lang)) => {
                current = Some(CodeBlock {
                    lang: lang.trim().to_string(),
                    text: String::new(),
                    preceded_by: last_line.clone(),
                })
            }
            (Some(_), Some(_)) => blocks.extend(current.take()),
            (Some(block), None) => {
                block.text.push_str(line);
                block.text.push('\n');
            }
            (None, None) => {
                if !line.trim().is_empty() {
                    last_line = line.trim().to_string();
                }
            }
        }
    }
    blocks
}

/// Every file with `extension` under `dir`, recursively, sorted. Only files git tracks count, so
/// local scratch files (gitignored notes, editor backups) can never make a docs test pass or
/// fail; without a git checkout (e.g. a source tarball) every file on disk counts.
pub fn tracked_files(dir: &Path, extension: &str) -> Vec<PathBuf> {
    let repo = repo_path("");
    let listed = std::process::Command::new("git")
        .arg("-C")
        .arg(&repo)
        .args(["ls-files", "-z", "--"])
        .arg(dir)
        .output()
        .ok()
        .filter(|output| output.status.success());
    let mut out: Vec<PathBuf> = match listed {
        Some(output) => output
            .stdout
            .split(|&b| b == 0)
            .filter(|name| !name.is_empty())
            .map(|name| repo.join(String::from_utf8_lossy(name).as_ref()))
            .filter(|path| path.extension().is_some_and(|e| e == extension) && path.is_file())
            .collect(),
        None => {
            fn walk(dir: &Path, extension: &str, out: &mut Vec<PathBuf>) {
                for entry in std::fs::read_dir(dir).unwrap() {
                    let path = entry.unwrap().path();
                    if path.is_dir() {
                        walk(&path, extension, out);
                    } else if path.extension().is_some_and(|e| e == extension) {
                        out.push(path);
                    }
                }
            }
            let mut out = Vec::new();
            walk(dir, extension, &mut out);
            out
        }
    };
    out.sort();
    out
}

/// Every tracked `.md` file under `dir`, sorted (see [`tracked_files`]).
pub fn markdown_files(dir: &Path) -> Vec<PathBuf> {
    tracked_files(dir, "md")
}

/// README.md and every Markdown file under docs/.
pub fn all_doc_files() -> Vec<PathBuf> {
    let mut files = vec![repo_path("README.md")];
    files.extend(markdown_files(&repo_path("docs")));
    files
}

/// The cells of a Markdown table row (`| a | b |` gives `["a", "b"]`), or `None` for a line
/// that is not a row or is the header separator.
pub fn table_row(line: &str) -> Option<Vec<String>> {
    let inner = line.trim().strip_prefix('|')?.strip_suffix('|')?;
    let cells: Vec<String> = inner.split('|').map(|c| c.trim().to_string()).collect();
    if cells
        .iter()
        .all(|c| c.chars().all(|ch| ch == '-' || ch == ':'))
    {
        return None;
    }
    Some(cells)
}

/// The text of the first inline code span in `cell` (`` `x` `` gives `x`).
pub fn code_span(cell: &str) -> Option<&str> {
    let start = cell.find('`')? + 1;
    let len = cell[start..].find('`')?;
    Some(&cell[start..start + len])
}
