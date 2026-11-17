//! Keeps the test tooling ([`TOOLING`]: the E2E/test runner scripts, the test compose file,
//! the E2E workflow and .env.test) in step with what actually reads its settings. Every
//! environment variable these files hand to a process must be read by something:
//!
//! - the server or the Rust test harness: a `.rs` file under src/ or tests/ passes it as a
//!   string literal to `env::var`, `env::var_os`, `env_flag` or `env_list` (and `RUST_LOG`,
//!   read through `EnvFilter::try_from_default_env`);
//! - the frontend tooling: `process.env.NAME` in frontend/vite.config.ts,
//!   frontend/playwright.config.ts or a file under frontend/e2e/, or `import.meta.env.NAME`
//!   under frontend/src/;
//! - a script the setter runs: `$NAME` / `${NAME...}` in a tracked shell script, or `${NAME...}`
//!   interpolation in a tracked compose file, that the setter names (by path or file name, see
//!   [`scripts_run_by`]). A script reading its own variable proves nothing about its children,
//!   and neither does an unrelated script that happens to read a variable of the same name.
//!   .env.test is handed to the scripts that load it, and to what those run; for a workflow,
//!   `${{ env.NAME }}` in the same workflow also counts;
//! - third-party software, listed with the reason in [`THIRD_PARTY`].
//!
//! A variable is "handed to a process" when it is
//! - in a shell script: on an `export` line, or a `NAME=value` prefix of a command (followed by
//!   the command on the same line, or by a `\` continuation). A plain `NAME=value` statement is
//!   a shell variable and does not count;
//! - in a compose file or workflow: a key (or `- NAME=value` item) of an `environment:` or
//!   `env:` block;
//! - in .env.test: a `NAME=value` line (run-tests.sh exports the whole file).
//!
//! The other checks pin the specific drift the scripts once had: binaries that do not exist
//! (`cargo run --bin migrate`), health checks on the wrong path (`/health` instead of
//! `/api/health`), and the seeded admin password differing between run-tests.sh and the
//! compose file.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use regex::Regex;

#[path = "support/markdown.rs"]
mod markdown;

const SHELL_TOOLING: &[&str] = &["scripts/run-e2e-local.sh", "run-tests.sh"];
const YAML_TOOLING: &[&str] = &["docker-compose.test.yml", ".github/workflows/test-e2e.yml"];
const DOTENV_TOOLING: &[&str] = &[".env.test"];
const TOOLING: &[&[&str]] = &[SHELL_TOOLING, YAML_TOOLING, DOTENV_TOOLING];

/// Variables read by software outside this repository, with who reads them.
const THIRD_PARTY: &[(&str, &str)] = &[
    ("POSTGRES_USER", "the postgres image's entrypoint"),
    ("POSTGRES_PASSWORD", "the postgres image's entrypoint"),
    ("POSTGRES_DB", "the postgres image's entrypoint"),
    ("PGDATA", "the postgres image's entrypoint"),
    ("PGPASSWORD", "libpq (psql, createdb, dropdb)"),
    ("NODE_ENV", "node, npm and vite"),
    ("RUST_BACKTRACE", "the Rust standard library"),
    ("CARGO_TERM_COLOR", "cargo"),
];

use markdown::{read, repo_path};

fn relative(path: &Path) -> String {
    path.strip_prefix(repo_path(""))
        .unwrap()
        .display()
        .to_string()
}

fn add(read: &mut BTreeMap<String, BTreeSet<String>>, name: &str, file: &str) {
    read.entry(name.to_string())
        .or_default()
        .insert(file.to_string());
}

/// Variable name -> the files (or third parties) reading it, excluding readers that only
/// count for some setters (other shell scripts, same-workflow references), see
/// [`script_readers`].
fn variables_read() -> BTreeMap<String, BTreeSet<String>> {
    let mut out = BTreeMap::new();
    let call =
        Regex::new(r#"(?:env::var(?:_os)?|env_flag|env_list)\(\s*"([A-Z][A-Z0-9_]*)""#).unwrap();
    for dir in ["src", "tests"] {
        for path in markdown::tracked_files(&repo_path(dir), "rs") {
            let text = std::fs::read_to_string(&path).unwrap();
            let file = relative(&path);
            for caps in call.captures_iter(&text) {
                add(&mut out, &caps[1], &file);
            }
            if text.contains("EnvFilter::try_from_default_env") {
                add(&mut out, "RUST_LOG", &file);
            }
        }
    }

    let process_env = Regex::new(r"process\.env\.([A-Z][A-Z0-9_]*)").unwrap();
    let mut frontend: Vec<String> = vec![
        "frontend/vite.config.ts".into(),
        "frontend/playwright.config.ts".into(),
    ];
    frontend.extend(
        markdown::tracked_files(&repo_path("frontend/e2e"), "ts")
            .iter()
            .map(|p| relative(p)),
    );
    for file in &frontend {
        for caps in process_env.captures_iter(&read(file)) {
            add(&mut out, &caps[1], file);
        }
    }
    let meta_env = Regex::new(r"import\.meta\.env\.([A-Z][A-Z0-9_]*)").unwrap();
    for ext in ["ts", "tsx"] {
        for path in markdown::tracked_files(&repo_path("frontend/src"), ext) {
            let text = std::fs::read_to_string(&path).unwrap();
            for caps in meta_env.captures_iter(&text) {
                add(&mut out, &caps[1], &relative(&path));
            }
        }
    }

    for (name, who) in THIRD_PARTY {
        add(&mut out, name, who);
    }
    out
}

/// Every tracked shell script and compose file, with the variables it reads by expansion.
fn script_readers() -> BTreeMap<String, BTreeSet<String>> {
    let expansion = Regex::new(r"\$\{?([A-Z][A-Z0-9_]*)").unwrap();
    let mut files: Vec<String> = markdown::tracked_files(&repo_path(""), "sh")
        .iter()
        .map(|p| relative(p))
        .collect();
    files.extend(
        markdown::tracked_files(&repo_path(""), "yml")
            .iter()
            .map(|p| relative(p))
            .filter(|f| {
                Path::new(f)
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("docker-compose"))
            }),
    );
    files
        .into_iter()
        .map(|file| {
            let text = read(&file);
            let names = expansion
                .captures_iter(&text)
                .map(|caps| caps[1].to_string())
                .collect();
            (file, names)
        })
        .collect()
}

/// The end of the shell word starting at `start` (quotes, `$(...)` and `${...}` included).
fn shell_word_end(line: &[u8], start: usize) -> usize {
    let mut i = start;
    while i < line.len() {
        match line[i] {
            b' ' | b'\t' | b';' | b'|' | b'&' | b')' => return i,
            _ => i = shell_token_end(line, i),
        }
    }
    i
}

/// The end of the quoted string, substitution or single character at `i`.
fn shell_token_end(line: &[u8], i: usize) -> usize {
    match line[i] {
        b'\\' => (i + 2).min(line.len()),
        b'\'' => line[i + 1..]
            .iter()
            .position(|&c| c == b'\'')
            .map_or(line.len(), |p| i + p + 2),
        b'"' => {
            let mut j = i + 1;
            while j < line.len() && line[j] != b'"' {
                j = if line[j] == b'$' && line.get(j + 1).is_some_and(|c| *c == b'(' || *c == b'{')
                {
                    shell_token_end(line, j)
                } else if line[j] == b'\\' {
                    j + 2
                } else {
                    j + 1
                };
            }
            (j + 1).min(line.len())
        }
        b'$' if matches!(line.get(i + 1), Some(b'(') | Some(b'{')) => {
            let close = if line[i + 1] == b'(' { b')' } else { b'}' };
            let mut j = i + 2;
            while j < line.len() && line[j] != close {
                j = shell_token_end(line, j);
            }
            (j + 1).min(line.len())
        }
        _ => i + 1,
    }
}

/// Variables a shell script hands to processes (see the module docs).
fn shell_exports(text: &str) -> BTreeSet<String> {
    let export = Regex::new(r"^\s*export\s+([A-Z][A-Z0-9_]*)\b").unwrap();
    // A command's environment prefix starts a line or a `$(`.
    let assignment = Regex::new(r"(?:^\s*|\$\(\s*)([A-Z][A-Z0-9_]*)=").unwrap();
    let mut names = BTreeSet::new();
    for line in text.lines() {
        if let Some(caps) = export.captures(line) {
            names.insert(caps[1].to_string());
            continue;
        }
        for caps in assignment.captures_iter(line) {
            let value_start = caps.get(0).unwrap().end();
            let end = shell_word_end(line.as_bytes(), value_start);
            let rest = line[end..].trim_start();
            if !(rest.is_empty() || rest.starts_with('#') || rest.starts_with(';')) {
                names.insert(caps[1].to_string());
            }
        }
    }
    names
}

/// Keys of the `environment:` / `env:` blocks of a compose file or workflow.
fn yaml_env_keys(text: &str) -> BTreeSet<String> {
    let block = Regex::new(r"^(\s*)(?:environment|env):\s*$").unwrap();
    let entry = Regex::new(r"^(\s*)(?:-\s+)?([A-Za-z_][A-Za-z0-9_]*)(?:=|:\s|:$)").unwrap();
    let mut names = BTreeSet::new();
    let mut block_indent: Option<usize> = None;
    let mut child_indent: Option<usize> = None;
    for line in text.lines() {
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        if let Some(parent) = block_indent {
            if indent > parent {
                let child = *child_indent.get_or_insert(indent);
                if indent == child {
                    if let Some(caps) = entry.captures(line) {
                        names.insert(caps[2].to_string());
                    }
                }
                continue;
            }
            block_indent = None;
            child_indent = None;
        }
        if let Some(caps) = block.captures(line) {
            block_indent = Some(caps[1].len());
        }
    }
    names
}

fn dotenv_keys(text: &str) -> BTreeSet<String> {
    let line_re = Regex::new(r"^([A-Z][A-Z0-9_]*)=").unwrap();
    text.lines()
        .filter_map(|l| line_re.captures(l).map(|c| c[1].to_string()))
        .collect()
}

/// Whether `text` names `script`: its repository path or its file name, as a whole word (so
/// `run.sh` does not match `prerun.sh`).
fn names_script(text: &str, script: &str) -> bool {
    let name = Path::new(script)
        .file_name()
        .map_or(script.to_string(), |n| n.to_string_lossy().into_owned());
    let word = Regex::new(&format!(
        r"(?:^|[^A-Za-z0-9_.-])(?:{}|{})\b",
        regex::escape(script),
        regex::escape(&name)
    ))
    .unwrap();
    word.is_match(text)
}

/// `text` followed by the Dockerfiles a compose file builds its services from (`dockerfile:`),
/// whose CMD runs the service's script.
fn with_built_dockerfiles(text: &str) -> String {
    let dockerfile = Regex::new(r"(?m)^\s*dockerfile:\s*(\S+)\s*$").unwrap();
    let mut all = text.to_string();
    for caps in dockerfile.captures_iter(text) {
        all.push('\n');
        all.push_str(&read(&caps[1]));
    }
    all
}

/// The scripts (of `scripts`) that `file`, with contents `text`, hands its environment to.
/// A shell script, compose file or workflow runs the scripts it names (a compose service also
/// those its Dockerfile names); .env.test reaches the scripts that name (and load) it, and
/// whatever those run.
fn scripts_run_by<'a>(
    file: &str,
    text: &str,
    scripts: &'a BTreeMap<String, BTreeSet<String>>,
) -> BTreeSet<&'a str> {
    let named_in = |text: &str, setter: &str| -> BTreeSet<&'a str> {
        let text = with_built_dockerfiles(text);
        scripts
            .keys()
            .filter(|script| script.as_str() != setter && names_script(&text, script))
            .map(String::as_str)
            .collect()
    };
    if !DOTENV_TOOLING.contains(&file) {
        return named_in(text, file);
    }
    let loaders: BTreeSet<&'a str> = scripts
        .keys()
        .filter(|script| names_script(&read(script), file))
        .map(String::as_str)
        .collect();
    let mut run = loaders.clone();
    for loader in loaders {
        run.extend(named_in(&read(loader), loader));
    }
    run
}

/// Whether one of the scripts `file` runs reads `name` (see [`scripts_run_by`]).
fn read_by_a_script_it_runs(
    file: &str,
    text: &str,
    name: &str,
    scripts: &BTreeMap<String, BTreeSet<String>>,
) -> bool {
    scripts_run_by(file, text, scripts)
        .into_iter()
        .any(|script| scripts[script].contains(name))
}

fn handed_to_processes(file: &str) -> BTreeSet<String> {
    let text = read(file);
    if SHELL_TOOLING.contains(&file) {
        shell_exports(&text)
    } else if YAML_TOOLING.contains(&file) {
        yaml_env_keys(&text)
    } else {
        dotenv_keys(&text)
    }
}

#[test]
fn every_variable_the_tooling_sets_is_read() {
    let read_by = variables_read();
    let scripts = script_readers();
    let workflow_ref = Regex::new(r"\$\{\{\s*env\.([A-Z][A-Z0-9_]*)\s*\}\}").unwrap();
    let mut unread = Vec::new();
    for file in TOOLING.iter().flat_map(|group| group.iter()) {
        let names = handed_to_processes(file);
        assert!(
            names.len() >= 3,
            "{file}: only {names:?} found; is the extraction broken?"
        );
        let same_workflow: BTreeSet<String> = if file.starts_with(".github/workflows/") {
            workflow_ref
                .captures_iter(&read(file))
                .map(|c| c[1].to_string())
                .collect()
        } else {
            BTreeSet::new()
        };
        let text = read(file);
        for name in names {
            if !read_by.contains_key(&name)
                && !read_by_a_script_it_runs(file, &text, &name, &scripts)
                && !same_workflow.contains(&name)
            {
                unread.push(format!("{file}: {name}"));
            }
        }
    }
    assert!(
        unread.is_empty(),
        "set by the test tooling but read by nothing (remove them, or use the variable the \
         consumer reads; e.g. the server listens on SERVER_ADDRESS or SERVER_HOST/SERVER_PORT, \
         and the Vite dev server proxies /api to VITE_API_PROXY_TARGET):\n{}",
        unread.join("\n")
    );
}

/// Binaries this crate builds: [[bin]] targets plus src/bin/*.rs (cargo's autobins).
fn binaries() -> BTreeSet<String> {
    let manifest_bin = Regex::new(r#"(?m)^\[\[bin\]\]\s*\nname\s*=\s*"([^"]+)""#).unwrap();
    let mut bins: BTreeSet<String> = manifest_bin
        .captures_iter(&read("Cargo.toml"))
        .map(|c| c[1].to_string())
        .collect();
    for path in markdown::tracked_files(&repo_path("src/bin"), "rs") {
        bins.insert(path.file_stem().unwrap().to_string_lossy().into_owned());
    }
    bins
}

#[test]
fn the_tooling_runs_only_binaries_that_exist() {
    let bins = binaries();
    assert!(
        bins.contains("readur") && bins.contains("migrate_to_s3"),
        "{bins:?}: is the extraction broken?"
    );
    let bin_arg = Regex::new(r"--bin[ =]([A-Za-z0-9_-]+)").unwrap();
    let release_path = Regex::new(r"target/(?:release|debug)/([A-Za-z0-9_-]+)").unwrap();
    let mut missing = Vec::new();
    for file in TOOLING.iter().flat_map(|group| group.iter()) {
        let text = read(file);
        for caps in bin_arg
            .captures_iter(&text)
            .chain(release_path.captures_iter(&text))
        {
            if !bins.contains(&caps[1]) {
                missing.push(format!("{file}: {}", &caps[1]));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "binaries that do not exist (the server applies its migrations on startup):\n{}",
        missing.join("\n")
    );
}

#[test]
fn health_checks_use_the_api_health_route() {
    let routes = read("src/routes/mod.rs");
    assert!(
        routes.contains(".route(\"/api/health\""),
        "src/routes/mod.rs no longer routes /api/health; update this test and the tooling"
    );
    let health_url = Regex::new(r"https?://[^\s/\x22']+(/[^\s\x22']*health)").unwrap();
    let mut wrong = Vec::new();
    let mut checked = 0;
    for file in TOOLING.iter().flat_map(|group| group.iter()) {
        for caps in health_url.captures_iter(&read(file)) {
            checked += 1;
            if &caps[1] != "/api/health" {
                wrong.push(format!("{file}: {}", &caps[0]));
            }
        }
    }
    assert!(
        checked >= 3,
        "only {checked} health URLs found; is the extraction broken?"
    );
    assert!(
        wrong.is_empty(),
        "health checks on a path the server does not serve:\n{}",
        wrong.join("\n")
    );
}

/// The value of `NAME="value"` in a shell script.
fn shell_constant(text: &str, name: &str) -> String {
    let re = Regex::new(&format!(r#"(?m)^{name}="([^"$]*)"$"#)).unwrap();
    re.captures(text)
        .unwrap_or_else(|| panic!("run-tests.sh no longer sets {name}"))[1]
        .to_string()
}

#[test]
fn run_tests_and_the_compose_file_agree_on_the_seeded_admin() {
    let script = read("run-tests.sh");
    let compose = read("docker-compose.test.yml");

    let default_password = shell_constant(&script, "DEFAULT_ADMIN_PASSWORD");
    let compose_password = Regex::new(r"(?m)^\s+ADMIN_PASSWORD: \$\{ADMIN_PASSWORD:-([^}]+)\}$")
        .unwrap()
        .captures(&compose)
        .expect("readur_test's ADMIN_PASSWORD must be ${ADMIN_PASSWORD:-<default>}")[1]
        .to_string();
    assert_eq!(default_password, compose_password);
    assert!(
        script.contains(r#"export ADMIN_PASSWORD="${ADMIN_PASSWORD:-$DEFAULT_ADMIN_PASSWORD}""#),
        "run-tests.sh must export the password compose interpolates"
    );
    assert!(
        script.contains(r#"E2E_ADMIN_PASSWORD="$ADMIN_PASSWORD""#),
        "run-tests.sh must hand the E2E suite the password the server was seeded with"
    );

    let username = shell_constant(&script, "ADMIN_USERNAME_IN_COMPOSE");
    assert!(
        compose.contains(&format!("      ADMIN_USERNAME: {username}\n")),
        "readur_test's ADMIN_USERNAME must be {username}"
    );
    assert!(script.contains(r#"E2E_ADMIN_USERNAME="$ADMIN_USERNAME_IN_COMPOSE""#));
}

#[test]
fn the_containerised_dev_server_proxies_to_the_test_backend() {
    let compose = read("docker-compose.test.yml");
    let port = Regex::new(r"(?m)^\s+SERVER_PORT: (\d+)")
        .unwrap()
        .captures(&compose)
        .expect("readur_test sets SERVER_PORT")[1]
        .to_string();
    assert!(compose.contains(&format!(
        "VITE_API_PROXY_TARGET: http://readur_test:{port}\n"
    )));
    assert!(compose.contains(&format!("http://localhost:{port}/api/health")));
    assert!(read("run-tests.sh").contains(&format!("API_URL=\"http://localhost:{port}\"")));
}

/// The local and containerised E2E runs go through the Vite dev server, so its /api proxy must
/// carry the sync-progress WebSocket too (the CI run is served by the backend directly).
#[test]
fn the_dev_server_proxies_api_websockets() {
    let sync_progress = read("frontend/src/services/api/syncProgress.ts");
    assert!(
        sync_progress.contains("/api/sources/"),
        "the sync-progress WebSocket moved out of /api; revisit the dev server proxy"
    );
    let vite = read("frontend/vite.config.ts");
    let block = vite
        .split_once("'/api': {")
        .and_then(|(_, rest)| rest.split_once("\n      },"))
        .expect("vite.config.ts proxies '/api'")
        .0
        .to_string();
    assert!(block.contains("target: PROXY_TARGET"), "{block}");
    assert!(
        block.contains("ws: true"),
        "the /api proxy must forward WebSockets: {block}"
    );
    // Name-based and HTTPS upstreams need their own Host.
    assert!(
        block.contains("changeOrigin: true"),
        "the /api proxy must send the target's Host: {block}"
    );
    // With Host rewritten, the server's WebSocket Origin check needs the host the browser
    // addressed, which it reads from X-Forwarded-Host (and the scheme from X-Forwarded-Proto).
    for header in ["X-Forwarded-Host", "X-Forwarded-Proto"] {
        assert!(
            block.contains("'proxyReqWs'") && block.contains(&format!("setHeader('{header}'")),
            "the /api proxy must set {header} on WebSocket handshakes: {block}"
        );
    }
    let server = read("src/routes/sources/progress_ws.rs");
    assert!(
        server.contains("fn websocket_origin_allowed")
            && server.contains("first_entry(\"x-forwarded-host\")")
            && server.contains("via_trusted_proxy && matches_forwarded_host"),
        "the WebSocket origin check no longer honours X-Forwarded-Host from trusted proxies; \
         revisit the proxy in vite.config.ts"
    );
    // The local runner's backend believes the dev server's forwarding headers whatever
    // TRUSTED_PROXIES the calling shell has.
    assert!(
        read("scripts/run-e2e-local.sh").contains("TRUSTED_PROXIES=\"127.0.0.1,::1\""),
        "scripts/run-e2e-local.sh must trust the loopback dev server"
    );
}

#[test]
fn the_e2e_workflow_targets_the_port_the_server_listens_on() {
    let workflow = read(".github/workflows/test-e2e.yml");
    let port = Regex::new(r"(?m)^\s+SERVER_PORT: (\d+)")
        .unwrap()
        .captures(&workflow)
        .expect("the workflow sets SERVER_PORT for the server")[1]
        .to_string();
    let urls: Vec<String> = Regex::new(r"http://localhost:(\d+)")
        .unwrap()
        .captures_iter(&workflow)
        .map(|c| c[1].to_string())
        .collect();
    assert!(urls.len() >= 2, "{urls:?}: is the extraction broken?");
    assert!(
        urls.iter().all(|u| *u == port),
        "URLs {urls:?} must use SERVER_PORT {port}"
    );
}

#[test]
fn extraction_follows_the_documented_rules() {
    let script = r#"
PLAIN="${E2E_X:-1}"
QUOTED="$(cd "$(dirname "$0")/.." && pwd)"   # comment
STAMP=$(date +"%Y %m")
export EXPORTED="x"
export BARE
PREFIX=1 cmd arg
CONT="a b" \
    NEXT="$(echo "c d")" \
    cmd
out=$(INNER="x" \
    cargo test 2>&1)
lower=1 cmd
local LOCAL_VAR=1
"#;
    let names: Vec<String> = shell_exports(script).into_iter().collect();
    assert_eq!(
        names,
        ["BARE", "CONT", "EXPORTED", "INNER", "NEXT", "PREFIX"]
    );

    let yaml = "\
env:
  TOP: 1
jobs:
  job:
    steps:
      - name: step
        run: |
          NOT_ENV=1 cmd
        env:
          STEP_VAR: ${{ env.TOP }}
          # COMMENTED: 1
          NESTED:
            DEEPER: 1
services:
  db:
    environment:
      - LIST_VAR=1
    healthcheck:
      test: [x]
";
    let names: Vec<String> = yaml_env_keys(yaml).into_iter().collect();
    assert_eq!(names, ["LIST_VAR", "NESTED", "STEP_VAR", "TOP"]);

    let read_by = variables_read();
    for name in [
        "DATABASE_URL",
        "RUST_LOG",
        "VITE_API_PROXY_TARGET",
        "PLAYWRIGHT_BASE_URL",
        "API_URL",
    ] {
        assert!(read_by.contains_key(name), "{name} not detected");
    }
    // The variables the scripts once set for nothing stay undetected.
    for name in [
        "ROCKET_PORT",
        "VITE_API_BASE_URL",
        "VITE_API_URL",
        "FRONTEND_PATH",
        "WATCH_PATH",
    ] {
        assert!(!read_by.contains_key(name), "{name} is read now?");
    }
}

/// A variable counts as read by another script only when the setter runs that script: an
/// unrelated script reading a variable of the same name proves nothing.
#[test]
fn only_scripts_the_setter_runs_count_as_readers() {
    let scripts: BTreeMap<String, BTreeSet<String>> = [
        ("scripts/child.sh", &["CHILD_ONLY"][..]),
        ("scripts/unrelated.sh", &["UNRELATED_ONLY"][..]),
        ("scripts/prechild.sh", &["PRECHILD_ONLY"][..]),
        ("docker-compose.test.yml", &["COMPOSE_ONLY"][..]),
    ]
    .into_iter()
    .map(|(file, names)| {
        let names = names.iter().map(|n| n.to_string()).collect();
        (file.to_string(), names)
    })
    .collect();
    let setter = r#"
export CHILD_ONLY=1 UNRELATED_ONLY=1 PRECHILD_ONLY=1 COMPOSE_ONLY=1
"$REPO_ROOT/scripts/child.sh"
COMPOSE_FILE="docker-compose.test.yml"
"#;
    let reads = |name: &str| read_by_a_script_it_runs("run-tests.sh", setter, name, &scripts);
    assert!(reads("CHILD_ONLY"), "the child it runs reads it");
    assert!(reads("COMPOSE_ONLY"), "the compose file it names reads it");
    // The old rule accepted these: another script mentions them, but this one never runs it.
    assert!(!reads("UNRELATED_ONLY"));
    // `child.sh` must not be taken for `prechild.sh`.
    assert!(!reads("PRECHILD_ONLY"));

    // A script reading its own variable proves nothing about its children.
    assert!(!read_by_a_script_it_runs(
        "scripts/child.sh",
        "export CHILD_ONLY=1\n./scripts/child.sh",
        "CHILD_ONLY",
        &scripts
    ));
}
