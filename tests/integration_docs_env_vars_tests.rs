//! Keeps the environment variables the documentation names in step with the code: every
//! variable named in a checked file ([`DOCUMENTED_IN`]: the configuration docs, the OIDC guide,
//! charts/readur/values.yaml and every other page whose variables are all real) must be read
//! somewhere, and every user-facing variable the server reads must be in the configuration
//! reference. README.md and every other page under docs/ that names a variable must be either
//! checked or listed in [`UNAUDITED`], whose entries must still name a variable the code does
//! not read, so the list only shrinks.
//!
//! A variable is "named" by a doc when it appears as
//! - the first cell of a Markdown table row, as `` `NAME` ``;
//! - an inline code span holding just `NAME` or starting `NAME=`, where NAME has an underscore
//!   (so ordinary capitalised words in code spans do not count);
//! - in a fenced code block (or anywhere in the YAML file), at the start of a line, possibly
//!   after `export `, `- ` or a `# ` comment marker: `NAME=...`, `NAME: ...` or `- name: NAME`.
//!
//! NAME is `[A-Z][A-Z0-9_]*`. A variable is "read" when a `.rs` file under src/ passes it as a
//! string literal to `env::var`, `env::var_os`, `env_flag` or `env_list`, when the frontend
//! development config reads it as `process.env.NAME`, or (for `RUST_LOG` only) when the server
//! builds its log filter with `EnvFilter::try_from_default_env`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use regex::Regex;

#[path = "support/markdown.rs"]
mod markdown;

const DOCUMENTED_IN: &[&str] = &[
    "docs/configuration.md",
    "docs/configuration-reference.md",
    "charts/readur/values.yaml",
    "docs/oidc-setup.md",
    "README.md",
    "docs/REVERSE_PROXY.md",
    "docs/WATCH_FOLDER.md",
    "docs/administration/cli-tools.md",
    "docs/administration/storage-migration.md",
    "docs/architecture.md",
    "docs/dev/LABEL_TESTING.md",
    "docs/dev/development.md",
    "docs/index.md",
    "docs/installation.md",
    "docs/mcp-guide.md",
    "docs/per-user-watch-directories.md",
    "docs/sources-guide.md",
    "docs/user-guide.md",
    "docs/user-management-guide.md",
];

/// Pages that still name variables Readur does not read: settings of other software in
/// examples, shell variables of scripts, and settings that do not exist. Audit a page (make
/// every variable it names real, or reword it) and move it to [`DOCUMENTED_IN`].
const UNAUDITED: &[&str] = &[
    "docs/administration/migration-troubleshooting.md",
    "docs/api-reference.md",
    "docs/backup-recovery.md",
    "docs/deployment.md",
    "docs/dev/DEPLOYMENT_SUMMARY.md",
    "docs/dev/ERROR_SYSTEM.md",
    "docs/dev/QUEUE_IMPROVEMENTS.md",
    "docs/dev/README-E2E.md",
    "docs/dev/TESTING.md",
    "docs/dev/architecture.md",
    "docs/dev/test-infrastructure.md",
    "docs/getting-started/configuration.md",
    "docs/getting-started/installation.md",
    "docs/getting-started/quickstart.md",
    "docs/health-monitoring-guide.md",
    "docs/integrations.md",
    "docs/migration-guide.md",
    "docs/notifications-guide.md",
    "docs/performance-tuning.md",
    "docs/quickstart/docker.md",
    "docs/quickstart/self-hosting.md",
    "docs/redesign/BRIEF.md",
    "docs/s3-storage-guide.md",
    "docs/s3-troubleshooting.md",
    "docs/security-guide.md",
    "docs/self-hosting/authentication.md",
    "docs/self-hosting/backup.md",
    "docs/self-hosting/index.md",
    "docs/self-hosting/monitoring.md",
    "docs/self-hosting/performance.md",
    "docs/self-hosting/storage.md",
    "docs/swagger-ui-guide.md",
    "docs/synology.md",
    "docs/troubleshooting.md",
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
    let mut read: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let sources = markdown::tracked_files(&repo().join("src"), "rs");
    for path in &sources {
        let text = std::fs::read_to_string(path).unwrap();
        let file = path.strip_prefix(repo()).unwrap().display().to_string();
        for caps in call.captures_iter(&text) {
            read.entry(caps[1].to_string())
                .or_default()
                .insert(file.clone());
        }
        if text.contains("EnvFilter::try_from_default_env") {
            read.entry("RUST_LOG".into()).or_default().insert(file);
        }
    }
    let process_env = Regex::new(r"process\.env\.([A-Z][A-Z0-9_]*)").unwrap();
    for config in ["frontend/vite.config.ts"] {
        let text = std::fs::read_to_string(repo().join(config)).unwrap();
        for caps in process_env.captures_iter(&text) {
            read.entry(caps[1].to_string())
                .or_default()
                .insert(config.to_string());
        }
    }
    read
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

#[test]
fn every_documented_variable_is_read_by_the_code() {
    let read = variables_read();
    let mut missing = Vec::new();
    for file in DOCUMENTED_IN {
        let names = documented(file);
        assert!(
            !names.is_empty(),
            "{file}: no variables found; is the extraction broken?"
        );
        if file.starts_with("docs/configuration") || file.ends_with(".yaml") {
            assert!(
                names.len() > 10,
                "{file}: only {names:?} found; is the extraction broken?"
            );
        }
        missing.extend(
            names
                .into_iter()
                .filter(|name| !read.contains_key(name))
                .map(|name| format!("{file}: {name}")),
        );
    }
    assert!(
        missing.is_empty(),
        "documented but never read (remove them from the docs, or read them):\n{}",
        missing.join("\n")
    );
}

/// Every Markdown page outside [`DOCUMENTED_IN`] that names a variable.
fn unchecked_pages_naming_variables() -> BTreeMap<String, BTreeSet<String>> {
    let pages = markdown::all_doc_files();
    pages
        .into_iter()
        .map(|path| path.strip_prefix(repo()).unwrap().display().to_string())
        .filter(|file| !DOCUMENTED_IN.contains(&file.as_str()))
        .map(|file| {
            let names = documented(&file);
            (file, names)
        })
        .filter(|(_, names)| !names.is_empty())
        .collect()
}

#[test]
fn every_page_naming_variables_is_checked_or_listed_as_unaudited() {
    let read = variables_read();
    let pages = unchecked_pages_naming_variables();
    let unlisted: Vec<&String> = pages
        .keys()
        .filter(|file| !UNAUDITED.contains(&file.as_str()))
        .collect();
    assert!(
        unlisted.is_empty(),
        "pages naming variables that are neither checked nor listed as unaudited (add them to \
         DOCUMENTED_IN once every variable they name is read by the code): {unlisted:?}"
    );
    for file in UNAUDITED {
        assert!(
            !DOCUMENTED_IN.contains(file),
            "{file} is both checked and unaudited"
        );
        let fake = pages.get(*file).map_or(0, |names| {
            names.iter().filter(|n| !read.contains_key(*n)).count()
        });
        assert!(
            fake > 0,
            "{file} now names only real variables: move it from UNAUDITED to DOCUMENTED_IN"
        );
    }
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
    for name in [
        "DATABASE_URL",
        "OIDC_LINK_EXISTING_BY_EMAIL",
        "CORS_ALLOWED_ORIGINS",
        "RUST_LOG",
        "CLIENT_PORT",
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
