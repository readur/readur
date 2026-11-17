//! Guards the dependency direction `routes -> services -> db`, with shared types in `models`:
//! nothing below the HTTP layer may reach back into `routes`. Business logic that both the REST
//! routes and the MCP tools need lives in `services` (or `db`/`models`), never in a route module.

use std::path::{Path, PathBuf};

/// Source trees that must not depend on `crate::routes`.
const LOWER_LAYERS: [&str; 4] = ["services", "db", "models", "mcp"];

/// `source` with comments, string literals and char literals blanked out, so only code is left.
/// Line structure is kept, so reported line numbers match the file.
fn code_only(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut out = String::with_capacity(source.len());
    let mut i = 0;
    while i < chars.len() {
        match non_code_len(&chars[i..]) {
            Some(len) => {
                let skipped = &chars[i..(i + len).min(chars.len())];
                out.extend(skipped.iter().filter(|c| **c == '\n'));
                out.push(' ');
                i += len.max(1);
            }
            None => {
                out.push(chars[i]);
                i += 1;
            }
        }
    }
    out
}

/// Length of the comment or literal `rest` starts with, or `None` when it starts with code.
fn non_code_len(rest: &[char]) -> Option<usize> {
    if rest.starts_with(&['/', '/']) {
        // Line comment (including doc comments): up to, not including, the newline.
        return Some(rest.iter().position(|c| *c == '\n').unwrap_or(rest.len()));
    }
    if rest.starts_with(&['/', '*']) {
        return Some(block_comment_len(rest));
    }
    if let Some(len) = raw_string_len(rest) {
        return Some(len);
    }
    match rest {
        ['"', ..] => Some(quoted_len(rest)),
        // Escaped char literal such as '\'' or '\u{22}'.
        ['\'', '\\', tail @ ..] => Some(
            2 + tail
                .iter()
                .position(|c| *c == '\'')
                .map_or(tail.len(), |p| p + 1),
        ),
        ['\'', _, '\'', ..] => Some(3),
        // Anything else, including a lone `'` (a lifetime), is code.
        _ => None,
    }
}

/// A block comment; Rust lets them nest.
fn block_comment_len(rest: &[char]) -> usize {
    let (mut depth, mut j) = (0usize, 0);
    while j < rest.len() {
        if rest[j..].starts_with(&['/', '*']) {
            depth += 1;
            j += 2;
        } else if rest[j..].starts_with(&['*', '/']) {
            depth -= 1;
            j += 2;
            if depth == 0 {
                return j;
            }
        } else {
            j += 1;
        }
    }
    rest.len()
}

/// A `"..."` string, honouring backslash escapes.
fn quoted_len(rest: &[char]) -> usize {
    let mut j = 1;
    while j < rest.len() && rest[j] != '"' {
        j += if rest[j] == '\\' { 2 } else { 1 };
    }
    j + 1
}

/// A raw string (`r"..."`, `r#"..."#`, `br"..."`, ...): ends at a quote followed by as many
/// `#`s as it opened with. `None` if `rest` does not start one.
fn raw_string_len(rest: &[char]) -> Option<usize> {
    let prefix = match rest {
        ['b', 'r', ..] => 2,
        ['r', ..] => 1,
        _ => return None,
    };
    let hashes = rest[prefix..].iter().take_while(|c| **c == '#').count();
    let open = prefix + hashes;
    if rest.get(open) != Some(&'"') {
        return None;
    }
    let closes_at =
        |j: usize| rest[j] == '"' && rest[j + 1..].iter().take(hashes).all(|c| *c == '#');
    let close = (open + 1..rest.len())
        .find(|&j| closes_at(j))
        .unwrap_or(rest.len());
    Some(close + 1 + hashes)
}

/// `(line number, line)` for every line of code in `source` that names the `routes` module.
fn route_references(source: &str) -> Vec<(usize, String)> {
    code_only(source)
        .lines()
        .enumerate()
        .filter(|(_, line)| names_routes_module(line))
        .map(|(n, line)| (n + 1, line.trim().to_string()))
        .collect()
}

/// Whether `line` contains `routes::` as a path segment (not as the tail of another
/// identifier such as `my_routes::`).
fn names_routes_module(line: &str) -> bool {
    line.match_indices("routes::").any(|(at, _)| {
        !line[..at]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphanumeric() || c == '_')
    })
}

fn rust_files(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("reading {}: {e}", dir.display()))
    {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, found);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            found.push(path);
        }
    }
}

#[test]
fn lower_layers_never_depend_on_routes() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    for layer in LOWER_LAYERS {
        let layer_dir = src.join(layer);
        let single_file = src.join(format!("{layer}.rs"));
        if layer_dir.is_dir() {
            rust_files(&layer_dir, &mut files);
        }
        if single_file.is_file() {
            files.push(single_file);
        }
    }
    assert!(
        files.len() > 50,
        "expected to scan the services/db/models/mcp trees, found only {} files",
        files.len()
    );

    let violations: Vec<String> = files
        .iter()
        .flat_map(|path| {
            let source = std::fs::read_to_string(path).unwrap();
            let shown = path.strip_prefix(&src).unwrap().display().to_string();
            route_references(&source)
                .into_iter()
                .map(move |(line, code)| format!("src/{shown}:{line}: {code}"))
        })
        .collect();
    assert!(
        violations.is_empty(),
        "services, db, models and mcp must not depend on routes (move the shared code down a \
         layer instead):\n{}",
        violations.join("\n")
    );
}

#[test]
fn code_only_keeps_code_and_drops_comments_and_strings() {
    let source = r##"
use crate::routes::labels::Label; // a real dependency
// use crate::routes::search; (commented out)
/* block: routes::x /* nested routes::y */ still comment routes::z */
/// Mirrors `routes::search`.
let s = "routes::in_a_string";
let r = r#"routes::in "a" raw string"#;
let c = '"'; let lifetime: &'static str = "";
let my_routes::x = 1;
crate::routes::api_router()
"##;
    let found = route_references(source);
    let lines: Vec<usize> = found.iter().map(|(n, _)| *n).collect();
    assert_eq!(lines, vec![2, 10], "{found:?}");
}
