//! Keeps the environment variables the documentation names in step with the code: every
//! variable named in a checked file ([`checked_files`]: README.md, every tracked page under
//! docs/ and charts/readur/values.yaml) must be read somewhere, and every user-facing variable
//! the server reads must be in the configuration reference. The pages still listed in
//! [`UNAUDITED`] are not checked; each must still name a variable the code does not read, so
//! the list only shrinks.
//!
//! A variable is "named" by a doc when it appears as
//! - the first cell of a Markdown table row, as `` `NAME` ``;
//! - an inline code span holding just `NAME` or starting `NAME=`, where NAME has an underscore
//!   (so ordinary capitalised words in code spans do not count);
//! - in a fenced code block (or anywhere in the YAML file), at the start of a line, possibly
//!   after `export `, `- ` or a `# ` comment marker: `NAME=...`, `NAME: ...` or `- name: NAME`.
//!
//! NAME is `[A-Z][A-Z0-9_]*`. A variable is "read" when a `.rs` file under src/ passes it to
//! `env::var`, `env::var_os`, `env_flag` or `env_list`, as a string literal or through a
//! `const NAME_ENV: &str = "NAME";` in the same tree, when a frontend config (vite, Playwright)
//! reads it as `process.env.NAME`, or (for `RUST_LOG` only) when the server builds its log
//! filter with `EnvFilter::try_from_default_env`.
//!
//! Two kinds of name are allowed without being read by Readur's code: the API error codes the
//! server returns (each `=> "CODE"` in src/errors/), which the error documentation lists in
//! tables, and the few variables in [`READ_BY_OTHER_SOFTWARE`] that programs Readur runs or is
//! run with read (each with the program that reads it). Shell scripts in the docs name their own
//! variables in lowercase, so they are not taken for settings.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use regex::Regex;

#[path = "support/markdown.rs"]
mod markdown;

/// Files that name many variables, so finding few there means the extraction broke.
const NAMING_MANY: [&str; 3] = [
    "docs/configuration.md",
    "docs/configuration-reference.md",
    "charts/readur/values.yaml",
];

/// Pages that still name variables Readur does not read. Audit a page (make every variable it
/// names real, or reword it) and remove it from this list. docs/api-reference.md is waiting
/// only for its example script's `TOKEN` variable to be renamed to lowercase.
const UNAUDITED: &[&str] = &["docs/api-reference.md"];

/// Variables named in the docs that Readur's code does not read but software it runs, or is run
/// with, does: name -> who reads it.
const READ_BY_OTHER_SOFTWARE: &[(&str, &str)] = &[
    (
        "RUST_BACKTRACE",
        "the Rust runtime of every Readur binary, to print panic backtraces",
    ),
    (
        "PGDATA",
        "the postgres container image, for its data directory",
    ),
];

/// Variables the server reads that are not settings for users: test harness switches and
/// build metadata.
const INTERNAL: [&str; 3] = [
    "TEST_DATABASE_URL",
    "TEST_REQUIRES_LARGER_POOL",
    "RUST_VERSION",
];

fn repo() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// Test-support modules of src/, whose variables configure tests rather than the server.
fn is_test_support(path: &Path) -> bool {
    let name = path.file_name().unwrap().to_string_lossy();
    name.starts_with("test_") || name.ends_with("_tests.rs")
}

/// Variable name -> the files reading it.
fn variables_read() -> BTreeMap<String, BTreeSet<String>> {
    let call =
        Regex::new(r#"(?:env::var(?:_os)?|env_flag|env_list)\(\s*"([A-Z][A-Z0-9_]*)""#).unwrap();
    let call_const =
        Regex::new(r"(?:env::var(?:_os)?|env_flag|env_list)\(\s*([A-Z][A-Z0-9_]*)\s*\)").unwrap();
    let const_def = Regex::new(r#"const ([A-Z][A-Z0-9_]*): &str = "([A-Z][A-Z0-9_]*)";"#).unwrap();
    let mut read: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let sources: Vec<(String, String)> = markdown::tracked_files(&repo().join("src"), "rs")
        .iter()
        .map(|path| {
            let file = path.strip_prefix(repo()).unwrap().display().to_string();
            (file, std::fs::read_to_string(path).unwrap())
        })
        .collect();
    let constants: BTreeMap<String, String> = sources
        .iter()
        .flat_map(|(_, text)| const_def.captures_iter(text))
        .map(|caps| (caps[1].to_string(), caps[2].to_string()))
        .collect();
    for (file, text) in &sources {
        let literal = call.captures_iter(text).map(|caps| caps[1].to_string());
        let through_const = call_const
            .captures_iter(text)
            .filter_map(|caps| constants.get(&caps[1]).cloned());
        for name in literal.chain(through_const) {
            read.entry(name).or_default().insert(file.clone());
        }
        if text.contains("EnvFilter::try_from_default_env") {
            read.entry("RUST_LOG".into())
                .or_default()
                .insert(file.clone());
        }
    }
    let process_env = Regex::new(r"process\.env\.([A-Z][A-Z0-9_]*)").unwrap();
    for config in ["frontend/vite.config.ts", "frontend/playwright.config.ts"] {
        let text = std::fs::read_to_string(repo().join(config)).unwrap();
        for caps in process_env.captures_iter(&text) {
            read.entry(caps[1].to_string())
                .or_default()
                .insert(config.to_string());
        }
    }
    read
}

/// The error codes the API returns: every `=> "CODE"` in src/errors/.
fn error_codes() -> BTreeSet<String> {
    let code = Regex::new(r#"=>\s*"([A-Z][A-Z0-9_]*)""#).unwrap();
    markdown::tracked_files(&repo().join("src/errors"), "rs")
        .iter()
        .flat_map(|path| {
            let text = std::fs::read_to_string(path).unwrap();
            code.captures_iter(&text)
                .map(|caps| caps[1].to_string())
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Whether a name a doc uses is accounted for: read by Readur, an error code, or read by other
/// software.
fn accounted_for(
    name: &str,
    read: &BTreeMap<String, BTreeSet<String>>,
    codes: &BTreeSet<String>,
) -> bool {
    read.contains_key(name)
        || codes.contains(name)
        || READ_BY_OTHER_SOFTWARE.iter().any(|(n, _)| *n == name)
}

/// The variables `text` names (see the module docs). `yaml` treats every line as code.
fn named_variables(text: &str, yaml: bool) -> BTreeSet<String> {
    let table_cell = Regex::new(r"^\|\s*`([A-Z][A-Z0-9_]*)`\s*\|").unwrap();
    let code_span = Regex::new(r"`([A-Z][A-Z0-9]*_[A-Z0-9_]*)(?:=[^`]*)?`").unwrap();
    let code_line = Regex::new(
        r"^\s*(?:#\s*)?(?:export\s+|-\s+)?(?:name:\s*([A-Z][A-Z0-9_]*)\s*$|([A-Z][A-Z0-9_]*)(?:=|:\s))",
    )
    .unwrap();
    let mut names = BTreeSet::new();
    let mut in_code = yaml;
    for line in text.lines() {
        if !yaml && line.trim_start().starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            if let Some(caps) = code_line.captures(line) {
                names.insert(caps.get(1).or(caps.get(2)).unwrap().as_str().to_string());
            }
            continue;
        }
        if let Some(caps) = table_cell.captures(line) {
            names.insert(caps[1].to_string());
        }
        for caps in code_span.captures_iter(line) {
            names.insert(caps[1].to_string());
        }
    }
    names
}

fn documented(file: &str) -> BTreeSet<String> {
    let text = std::fs::read_to_string(repo().join(file)).unwrap();
    named_variables(&text, file.ends_with(".yaml"))
}

/// The checked files: README.md, every tracked page under docs/ except the
/// [`UNAUDITED`] ones, and the Helm chart's values.yaml.
fn checked_files() -> Vec<String> {
    markdown::all_doc_files()
        .into_iter()
        .map(|path| path.strip_prefix(repo()).unwrap().display().to_string())
        .chain(["charts/readur/values.yaml".to_string()])
        .filter(|file| !UNAUDITED.contains(&file.as_str()))
        .collect()
}

#[test]
fn every_documented_variable_is_read_by_the_code() {
    let read = variables_read();
    let codes = error_codes();
    let checked = checked_files();
    for file in NAMING_MANY {
        assert!(checked.iter().any(|f| f == file), "{file} is not checked");
        let names = documented(file);
        assert!(
            names.len() > 10,
            "{file}: only {names:?} found; is the extraction broken?"
        );
    }
    let mut missing = Vec::new();
    for file in &checked {
        missing.extend(
            documented(file)
                .into_iter()
                .filter(|name| !accounted_for(name, &read, &codes))
                .map(|name| format!("{file}: {name}")),
        );
    }
    assert!(
        missing.is_empty(),
        "documented but never read (remove them from the docs, or read them; name shell script \
         variables in lowercase):\n{}",
        missing.join("\n")
    );
}

#[test]
fn every_unaudited_page_still_names_a_variable_nobody_reads() {
    let read = variables_read();
    let codes = error_codes();
    for file in UNAUDITED {
        assert!(repo().join(file).is_file(), "{file} does not exist");
        let fake: Vec<String> = documented(file)
            .into_iter()
            .filter(|n| !accounted_for(n, &read, &codes))
            .collect();
        assert!(
            !fake.is_empty(),
            "{file} now names only real variables: remove it from UNAUDITED"
        );
    }
}

#[test]
fn the_exceptions_are_needed_and_not_readur_settings() {
    let read = variables_read();
    let named: BTreeSet<String> = checked_files()
        .iter()
        .flat_map(|file| documented(file))
        .collect();
    for (name, reader) in READ_BY_OTHER_SOFTWARE {
        assert!(
            !read.contains_key(*name),
            "{name} is read by Readur itself; drop it from READ_BY_OTHER_SOFTWARE"
        );
        assert!(
            named.contains(*name),
            "no checked page names {name} (read by {reader}); drop it from READ_BY_OTHER_SOFTWARE"
        );
    }
    let codes = error_codes();
    for code in [
        "SHARED_LINK_NOT_FOUND",
        "USER_NOT_FOUND",
        "SEARCH_QUERY_TOO_SHORT",
    ] {
        assert!(codes.contains(code), "error code {code} not detected");
    }
    // Error codes are detected from the code, not by their shape.
    assert!(!codes.contains("DOCUMENT_NOT_FOUND"));
    assert!(!codes.contains("JWT_SECRET"));
}

#[test]
fn every_variable_the_server_reads_is_in_the_reference() {
    let reference = documented("docs/configuration-reference.md");
    let missing: Vec<String> = variables_read()
        .into_iter()
        .filter(|(name, files)| {
            !INTERNAL.contains(&name.as_str())
                && !reference.contains(name)
                && files
                    .iter()
                    .any(|f| f.ends_with(".rs") && !is_test_support(Path::new(f)))
        })
        .map(|(name, files)| format!("{name} (read in {files:?})"))
        .collect();
    assert!(
        missing.is_empty(),
        "read by the server but missing from docs/configuration-reference.md:\n{}",
        missing.join("\n")
    );
}

#[test]
fn reading_detection_sees_every_reader_form() {
    let read = variables_read();
    // env::var, a multi-line env_flag call, env_list, the implicit RUST_LOG and the frontend.
    // A const holding the name, and the Playwright config.
    for name in [
        "DATABASE_URL",
        "OIDC_LINK_EXISTING_BY_EMAIL",
        "CORS_ALLOWED_ORIGINS",
        "RUST_LOG",
        "CLIENT_PORT",
        "READUR_TEST_POSTGRES_URL",
        "PLAYWRIGHT_BASE_URL",
    ] {
        assert!(read.contains_key(name), "{name} not detected");
    }
    // The removed variable this test was written to catch stays undetected.
    assert!(!read.contains_key("FUZZY_SEARCH_THRESHOLD"));
}

#[test]
fn extraction_follows_the_documented_rules() {
    let markdown = "\
| Variable | Default |
|----------|---------|
| `TABLE_VAR` | `x` |
| `auth.existingSecret` | `\"\"` |
Prose with `SPAN_VAR`, `SPAN_ASSIGN=1`, `ASC` and `lower_case`.

```bash
export EXPORTED_VAR=1
PLAIN_VAR=2   # comment
JWT_SECRET=$(openssl rand -hex 32)
echo $NOT_ASSIGNED
```

```yaml
environment:
  - LIST_VAR=1
  MAP_VAR: \"1\"
env:
  - name: K8S_VAR
    value: \"1\"
```
";
    let names: Vec<String> = named_variables(markdown, false).into_iter().collect();
    assert_eq!(
        names,
        [
            "EXPORTED_VAR",
            "JWT_SECRET",
            "K8S_VAR",
            "LIST_VAR",
            "MAP_VAR",
            "PLAIN_VAR",
            "SPAN_ASSIGN",
            "SPAN_VAR",
            "TABLE_VAR",
        ]
    );

    let yaml = "env:\n  SERVER_PORT: \"8000\"\n  # PUBLIC_URL: \"https://x\"\n  repository: readur/readur\n";
    let names: Vec<String> = named_variables(yaml, true).into_iter().collect();
    assert_eq!(names, ["PUBLIC_URL", "SERVER_PORT"]);
}
