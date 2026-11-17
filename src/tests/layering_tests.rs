//! Guards the dependency direction `routes -> services -> db`, with shared types in `models`:
//! nothing below the HTTP layer may reach back into `routes`. Business logic that both the REST
//! routes and the MCP tools (or the background schedulers) need lives in `services` (or
//! `db`/`models`), never in a route module.
//!
//! The guard is an allowlist, not a blocklist: every `.rs` file under `src/` is scanned except the
//! few that exist to wire routes up. A new top-level module is covered the day it is added.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Paths under `src/` (a file, or a directory and everything in it) that may name `routes`:
/// the route modules themselves, the binary that mounts the router, the OpenAPI document that
/// lists the handlers, the test harness that builds the router, and the in-crate tests of it.
const ROUTE_CONSUMERS: [&str; 5] = ["routes", "main.rs", "swagger.rs", "test_utils.rs", "tests"];

/// Layers that must be scanned. The walk covers everything outside [`ROUTE_CONSUMERS`] anyway;
/// this list pins that the important ones really were reached, so a broken walk cannot pass by
/// scanning nothing.
const REQUIRED_LAYERS: [&str; 9] = [
    "scheduling",
    "ingestion",
    "ocr",
    "storage",
    "services",
    "db",
    "models",
    "mcp",
    "monitoring",
];

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

/// `(line number, line)` for every line of code in `source` that names the `routes` module:
/// as a path segment (`crate::routes`, `routes::x`, `super::routes`), or anywhere inside a `use`
/// declaration (`use crate::{db, routes};`, which a later `routes::x` or an alias would hide).
fn route_references(source: &str) -> Vec<(usize, String)> {
    let code = code_only(source);
    let use_spans = use_declarations(&code);
    let lines: BTreeSet<usize> = identifier_positions(&code, "routes")
        .filter(|&at| {
            let in_path = code[..at].trim_end().ends_with("::")
                || code[at + "routes".len()..].trim_start().starts_with("::");
            in_path || use_spans.iter().any(|span| span.contains(&at))
        })
        .map(|at| code[..at].matches('\n').count())
        .collect();
    let code_lines: Vec<&str> = code.lines().collect();
    lines
        .into_iter()
        .map(|n| (n + 1, code_lines[n].trim().to_string()))
        .collect()
}

/// Byte offsets where `word` occurs as a whole identifier (not as part of `my_routes` or
/// `routes_table`).
fn identifier_positions<'a>(code: &'a str, word: &'a str) -> impl Iterator<Item = usize> + 'a {
    let is_ident = |c: char| c.is_alphanumeric() || c == '_';
    code.match_indices(word)
        .map(|(at, _)| at)
        .filter(move |&at| {
            !code[..at].chars().next_back().is_some_and(is_ident)
                && !code[at + word.len()..].chars().next().is_some_and(is_ident)
        })
}

/// Byte ranges of every `use ...;` declaration in comment- and literal-free `code`.
fn use_declarations(code: &str) -> Vec<std::ops::Range<usize>> {
    identifier_positions(code, "use")
        .map(|start| {
            let end = code[start..]
                .find(';')
                .map_or(code.len(), |p| start + p + 1);
            start..end
        })
        .collect()
}

/// Whether `relative` (a path under `src/`, `/`-separated) is, or is inside, a route consumer.
fn is_route_consumer(relative: &str) -> bool {
    ROUTE_CONSUMERS.iter().any(|allowed| {
        relative == *allowed
            || relative.starts_with(&format!("{allowed}/"))
            || relative == format!("{allowed}.rs")
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
    let mut all = Vec::new();
    rust_files(&src, &mut all);
    let relative = |path: &Path| {
        path.strip_prefix(&src)
            .unwrap()
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/")
    };
    let scanned: Vec<(String, &PathBuf)> = all
        .iter()
        .map(|path| (relative(path), path))
        .filter(|(shown, _)| !is_route_consumer(shown))
        .collect();

    for layer in REQUIRED_LAYERS {
        assert!(
            scanned
                .iter()
                .any(|(shown, _)| shown.starts_with(&format!("{layer}/"))
                    || *shown == format!("{layer}.rs")),
            "expected to scan src/{layer}, but no file of it was reached"
        );
    }
    assert!(
        scanned.len() > 150,
        "expected to scan most of src/, found only {} files",
        scanned.len()
    );

    let violations: Vec<String> = scanned
        .iter()
        .flat_map(|(shown, path)| {
            let source = std::fs::read_to_string(path).unwrap();
            route_references(&source)
                .into_iter()
                .map(move |(line, code)| format!("src/{shown}:{line}: {code}"))
        })
        .collect();
    assert!(
        violations.is_empty(),
        "only {ROUTE_CONSUMERS:?} may depend on routes (move the shared code down a layer \
         instead):\n{}",
        violations.join("\n")
    );
}

#[test]
fn only_the_router_wiring_is_allowed_to_name_routes() {
    for consumer in [
        "routes/labels.rs",
        "routes/sources/crud.rs",
        "routes.rs",
        "main.rs",
        "swagger.rs",
        "test_utils.rs",
        "tests/layering_tests.rs",
    ] {
        assert!(is_route_consumer(consumer), "{consumer}");
    }
    for lower in [
        "scheduling/webdav_scheduler.rs",
        "services/webdav_sync.rs",
        "lib.rs",
        "test_utils_mcp.rs",
        "routes_helpers.rs",
        "my_tests/x.rs",
        "testsuite.rs",
        "db/routes.rs",
    ] {
        assert!(!is_route_consumer(lower), "{lower}");
    }
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

#[test]
fn every_way_of_naming_the_routes_module_is_caught() {
    let source = r##"
use crate::routes;
use crate::routes as r;
use crate::{
    db,
    routes,
};
use super::super :: routes::x;
let a = readur::routes ::api_router();
let routes = vec![1];
fn routes() {}
let b = routes_table::x;
let c = config.routes;
pub mod routes;
use crate::db; let d = routes;
"##;
    let found = route_references(source);
    let lines: Vec<usize> = found.iter().map(|(n, _)| *n).collect();
    assert_eq!(lines, vec![2, 3, 6, 8, 9], "{found:?}");
}
