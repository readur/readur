//! Checks the verifiable claims the documentation makes, other than environment variables
//! (tests/integration_docs_env_vars_tests.rs) and API endpoints
//! (tests/integration_docs_endpoints_tests.rs). The pages are README.md and every tracked
//! `.md` file under docs/.
//!
//! - **Examples parse.** Every fenced ` ```yaml ` / ` ```yml ` block is valid YAML (several
//!   documents separated by `---` are fine) and every ` ```json ` block is valid JSON. A block
//!   indented inside a list item is dedented first.
//! - **Docker Compose files use the published image.** A YAML example with a top-level
//!   `services` map that names Readur's image names it as `ghcr.io/readur/readur:<tag>`, where
//!   the tag is `main`, `latest` or a version number (that is what .github/workflows/docker-build.yml
//!   publishes).
//! - **Commands exist.** In shell code blocks (` ```bash `, `sh`, `shell`, `console`, `zsh`;
//!   a line ending in `\` is joined with the next) and in inline code spans:
//!   - `readur <word>` run as a command (at the start of a command, after `--` or `sudo`, as
//!     `./readur` or `/app/readur`, or as `exec <container> readur`) names a subcommand of
//!     src/main.rs (or `--help`); inside a container (`docker exec`, `compose exec`,
//!     `kubectl exec`) it is run as `/app/readur` or `./readur`, since the image's /app is not
//!     on PATH;
//!   - `cargo run` (or `cargo watch -x run`) passes `--bin NAME` (Cargo.toml has several
//!     binaries and no `default-run`), and NAME is a binary (a `[[bin]]` of Cargo.toml or a
//!     file of src/bin/); `--features X` names features of Cargo.toml, and
//!     `cargo test --test NAME` names a test target under tests/;
//!   - options given to `migrate_to_s3` are options its clap definition declares;
//!   - `npm run NAME` names a script of frontend/package.json;
//!   - `helm ... --set KEY=VALUE` sets a key that exists in charts/readur/values.yaml, or an
//!     environment variable of the Readur container that docs/configuration-reference.md lists;
//!   - an inline code span that looks like a Helm value path (`auth.jwtSecret`) is a value of
//!     charts/readur/values.yaml.
//! - **Links resolve.** Every relative Markdown link points to an existing file, and a
//!   `#anchor` on a link to a Markdown page (or on its own) names one of that page's headings,
//!   slugged as MkDocs does. Every page in the mkdocs.yml navigation exists.
//! - **Hostnames are examples.** URLs use `localhost`, a loopback, private or documentation
//!   address, an example domain (`example.com`, `*.example.com`, `*.local`, ...), a single-label
//!   host (a Compose service name), a placeholder, or one of the public sites in
//!   [`PUBLIC_SITES`], never anyone's real server.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;
use serde::Deserialize;

#[path = "support/markdown.rs"]
mod markdown;

fn repo() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// Every documentation page: (path relative to the repository, text).
fn pages() -> Vec<(String, String)> {
    markdown::all_doc_files()
        .into_iter()
        .map(|path| {
            let rel = path.strip_prefix(repo()).unwrap().display().to_string();
            let text = std::fs::read_to_string(&path).unwrap();
            (rel, text)
        })
        .collect()
}

/// `text` with the indentation common to its non-blank lines removed.
fn dedent(text: &str) -> String {
    let indent = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);
    text.lines()
        .map(|l| l.get(indent..).unwrap_or("").trim_end())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Each block of `text` whose info string is one of `langs`, dedented.
fn blocks(text: &str, langs: &[&str]) -> Vec<String> {
    markdown::code_blocks(text)
        .into_iter()
        .filter(|b| langs.contains(&b.lang.split_whitespace().next().unwrap_or("")))
        .map(|b| dedent(&b.text))
        .collect()
}

/// The documents of a YAML stream.
fn yaml_documents(text: &str) -> Result<Vec<serde_yaml_ng::Value>, serde_yaml_ng::Error> {
    serde_yaml_ng::Deserializer::from_str(text)
        .map(serde_yaml_ng::Value::deserialize)
        .collect()
}

const SHELL: &[&str] = &["bash", "sh", "shell", "console", "zsh"];

/// The command lines of the docs: each line of a shell block (continuations joined), and each
/// inline code span outside code blocks. (page, line).
fn command_lines() -> Vec<(String, String)> {
    let span = Regex::new(r"`([^`]+)`").unwrap();
    let mut out = Vec::new();
    for (page, text) in pages() {
        for block in blocks(&text, SHELL) {
            let mut joined = String::new();
            for line in block.lines() {
                match line.strip_suffix('\\') {
                    Some(start) => {
                        joined.push_str(start);
                        joined.push(' ');
                    }
                    None => {
                        joined.push_str(line);
                        out.push((page.clone(), std::mem::take(&mut joined)));
                    }
                }
            }
            if !joined.is_empty() {
                out.push((page.clone(), joined));
            }
        }
        let mut in_code = false;
        for line in text.lines() {
            if line.trim_start().starts_with("```") {
                in_code = !in_code;
                continue;
            }
            if !in_code {
                for caps in span.captures_iter(line) {
                    out.push((page.clone(), caps[1].to_string()));
                }
            }
        }
    }
    out
}

/// The variables docs/configuration-reference.md lists (first table cells).
fn reference_variables() -> BTreeSet<String> {
    markdown::read("docs/configuration-reference.md")
        .lines()
        .filter_map(markdown::table_row)
        .filter_map(|cells| {
            cells
                .first()
                .and_then(|c| markdown::code_span(c))
                .map(str::to_string)
        })
        .filter(|c| {
            c.chars()
                .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
        })
        .collect()
}

#[test]
fn yaml_and_json_examples_parse() {
    let mut bad = Vec::new();
    let mut yaml_count = 0;
    let mut json_count = 0;
    for (page, text) in pages() {
        for block in blocks(&text, &["yaml", "yml"]) {
            yaml_count += 1;
            if let Err(err) = yaml_documents(&block) {
                let first = block.lines().next().unwrap_or("");
                bad.push(format!("{page}: YAML block starting `{first}`: {err}"));
            }
        }
        for block in blocks(&text, &["json"]) {
            json_count += 1;
            if let Err(err) = serde_json::from_str::<serde_json::Value>(&block) {
                let first = block.lines().next().unwrap_or("");
                bad.push(format!("{page}: JSON block starting `{first}`: {err}"));
            }
        }
    }
    assert!(yaml_count > 20 && json_count > 20, "too few blocks found");
    assert!(
        bad.is_empty(),
        "examples that do not parse:\n{}",
        bad.join("\n")
    );
}

/// Whether `image` is Readur's published image with a published tag.
fn is_published_readur_image(image: &str) -> bool {
    static PUBLISHED: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^ghcr\.io/readur/readur:(main|latest|\d+(\.\d+){0,2})$").unwrap()
    });
    PUBLISHED.is_match(image)
}

#[test]
fn compose_examples_use_the_published_image() {
    let mut bad = Vec::new();
    let mut services_seen = 0;
    for (page, text) in pages() {
        for block in blocks(&text, &["yaml", "yml"]) {
            for doc in yaml_documents(&block).unwrap_or_default() {
                let Some(services) = doc.get("services").and_then(|s| s.as_mapping()) else {
                    continue;
                };
                for service in services.values() {
                    let Some(image) = service.get("image").and_then(|i| i.as_str()) else {
                        continue;
                    };
                    services_seen += 1;
                    if image.contains("readur/readur") && !is_published_readur_image(image) {
                        bad.push(format!("{page}: image `{image}`"));
                    }
                }
            }
        }
    }
    assert!(services_seen > 5, "too few Compose services found");
    assert!(
        bad.is_empty(),
        "Compose examples naming an image that is not published (use \
         ghcr.io/readur/readur:<main|latest|version>):\n{}",
        bad.join("\n")
    );
}

#[test]
fn image_check_follows_the_documented_rules() {
    for ok in [
        "ghcr.io/readur/readur:main",
        "ghcr.io/readur/readur:latest",
        "ghcr.io/readur/readur:2.6",
        "ghcr.io/readur/readur:2.6.0",
    ] {
        assert!(is_published_readur_image(ok), "{ok}");
    }
    for bad in [
        "readur/readur:latest",
        "ghcr.io/readur/readur",
        "ghcr.io/readur/readur:v2.6.0",
        "ghcr.io/readur/readur:stable",
    ] {
        assert!(!is_published_readur_image(bad), "{bad}");
    }
}

/// The subcommands of the `readur` binary, from the clap `Commands` enum of src/main.rs.
fn readur_subcommands() -> BTreeSet<String> {
    let main = markdown::read("src/main.rs");
    let start = main.find("enum Commands {").expect("Commands enum");
    let body = &main[start..start + main[start..].find('}').unwrap()];
    let variant = Regex::new(r"(?m)^\s*([A-Z][A-Za-z]*),?\s*$").unwrap();
    variant
        .captures_iter(body)
        .map(|caps| {
            let mut kebab = String::new();
            for (i, ch) in caps[1].chars().enumerate() {
                if ch.is_ascii_uppercase() && i > 0 {
                    kebab.push('-');
                }
                kebab.push(ch.to_ascii_lowercase());
            }
            kebab
        })
        .collect()
}

/// The binaries `cargo run --bin` accepts.
fn binaries() -> BTreeSet<String> {
    let cargo = markdown::read("Cargo.toml");
    let name = Regex::new(r#"(?m)^\[\[bin\]\]\s*\nname\s*=\s*"([^"]+)""#).unwrap();
    let mut out: BTreeSet<String> = name
        .captures_iter(&cargo)
        .map(|c| c[1].to_string())
        .collect();
    for path in markdown::tracked_files(&repo().join("src/bin"), "rs") {
        out.insert(path.file_stem().unwrap().to_string_lossy().into_owned());
    }
    out
}

/// Whether `cargo run` needs `--bin`: Cargo.toml declares several binaries and no `default-run`.
fn needs_bin() -> bool {
    binaries().len() > 1 && !markdown::read("Cargo.toml").contains("default-run")
}

/// The features of Cargo.toml.
fn features() -> BTreeSet<String> {
    let cargo = markdown::read("Cargo.toml");
    let start = cargo.find("[features]").expect("[features]");
    let section = &cargo[start + "[features]".len()..];
    let section = &section[..section.find("\n[").unwrap_or(section.len())];
    let key = Regex::new(r"(?m)^([a-z][a-z0-9_-]*)\s*=").unwrap();
    key.captures_iter(section)
        .map(|c| c[1].to_string())
        .collect()
}

/// The test targets `cargo test --test` accepts: the files of tests/ and `[[test]]` names.
fn test_targets() -> BTreeSet<String> {
    let cargo = markdown::read("Cargo.toml");
    let name = Regex::new(r#"(?m)^\[\[test\]\]\s*\nname\s*=\s*"([^"]+)""#).unwrap();
    let mut out: BTreeSet<String> = name
        .captures_iter(&cargo)
        .map(|c| c[1].to_string())
        .collect();
    for entry in std::fs::read_dir(repo().join("tests")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "rs") {
            out.insert(path.file_stem().unwrap().to_string_lossy().into_owned());
        }
    }
    out
}

/// The options of the migrate_to_s3 binary, from its clap `#[arg(...)]` fields.
fn migrate_to_s3_options() -> BTreeSet<String> {
    let source = markdown::read("src/bin/migrate_to_s3.rs");
    let field = Regex::new(r"#\[arg\(([^)]*)\)\]\s*\n\s*([a-z_]+):").unwrap();
    let mut out: BTreeSet<String> = ["--help", "-h"].map(String::from).into();
    for caps in field.captures_iter(&source) {
        let (attrs, name) = (&caps[1], &caps[2]);
        if attrs.contains("long") {
            out.insert(format!("--{}", name.replace('_', "-")));
        }
        if attrs.contains("short") {
            out.insert(format!("-{}", &name[..1]));
        }
    }
    out
}

/// The scripts of frontend/package.json.
fn npm_scripts() -> BTreeSet<String> {
    let package: serde_json::Value =
        serde_json::from_str(&markdown::read("frontend/package.json")).unwrap();
    package["scripts"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect()
}

/// What is wrong with the commands in `line`, if anything.
fn command_problems(line: &str) -> Vec<String> {
    let mut problems = readur_problems(line);
    problems.extend(cargo_problems(line));
    problems.extend(migrate_to_s3_problems(line));
    problems.extend(npm_problems(line));
    problems.extend(helm_problems(line));
    problems
}

/// `readur` run with an unknown subcommand, or by bare name inside a container.
fn readur_problems(line: &str) -> Vec<String> {
    static READUR: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?:^|[;&|(]\s*|\$\s+|sudo\s+|--\s+|exec\s+(?:-\S+\s+)*[^-\s]\S*\s+)(?:\./|/app/)?readur(?:\s+([a-z-][\w-]*))?(?:\s|$)",
        )
        .unwrap()
    });
    static BARE_READUR: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?:^|\s)readur\s+(serve|reset-admin-password|rotate-jwt-secret)\b").unwrap()
    });
    let subcommands = readur_subcommands();
    let mut problems: Vec<String> = READUR
        .captures_iter(line)
        .filter_map(|caps| caps.get(1).map(|m| m.as_str()))
        .filter(|sub| !subcommands.contains(*sub) && !["--help", "-h"].contains(sub))
        .map(|sub| format!("`readur {sub}`: no such subcommand"))
        .collect();
    // The image's WORKDIR is /app, which is not on PATH, so inside a container the binary is
    // /app/readur (or ./readur).
    let in_container = ["docker exec", "compose exec", "kubectl exec", "podman exec"]
        .iter()
        .any(|c| line.contains(c));
    if in_container {
        problems.extend(BARE_READUR.captures_iter(line).map(|caps| {
            format!(
                "`readur {}` in a container: /app is not on PATH, run /app/readur",
                &caps[1]
            )
        }));
    }
    problems
}

/// `cargo run` without `--bin` or with an unknown binary, unknown features, unknown test
/// targets.
fn cargo_problems(line: &str) -> Vec<String> {
    static CARGO_RUN: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"cargo\s+(?:\+\S+\s+)?run\b([^|;&]*)|cargo\s+watch\b.*?-x\s+'?run\b([^|;&']*)")
            .unwrap()
    });
    static BIN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"--bin[= ]([\w-]+)").unwrap());
    static FEATURES: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"--features[= ]"?([\w, -]+?)"?(?:\s|$)"#).unwrap());
    static CARGO_TEST: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"cargo\s+(?:\+\S+\s+)?test\b[^|;&]*?--test[= ]([\w-]+)").unwrap()
    });
    let mut problems = Vec::new();
    for caps in CARGO_RUN.captures_iter(line) {
        let args = caps.get(1).or(caps.get(2)).map_or("", |m| m.as_str());
        if !BIN.is_match(args) && needs_bin() {
            problems.push(
                "`cargo run` without `--bin`: Cargo.toml has several binaries and no default-run"
                    .to_string(),
            );
        }
        problems.extend(
            BIN.captures_iter(args)
                .filter(|bin| !binaries().contains(&bin[1]))
                .map(|bin| format!("`--bin {}`: no such binary", &bin[1])),
        );
    }
    if line.contains("cargo ") {
        problems.extend(
            FEATURES
                .captures_iter(line)
                .flat_map(|feats| {
                    feats[1]
                        .split([',', ' '])
                        .filter(|f| !f.is_empty())
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                })
                .filter(|feat| !features().contains(feat))
                .map(|feat| format!("`--features {feat}`: no such feature")),
        );
    }
    problems.extend(
        CARGO_TEST
            .captures_iter(line)
            .filter(|caps| !test_targets().contains(&caps[1]))
            .map(|caps| format!("`--test {}`: no such test target", &caps[1])),
    );
    problems
}

/// Options given to migrate_to_s3 that it does not declare.
fn migrate_to_s3_problems(line: &str) -> Vec<String> {
    let Some(at) = line.rfind("migrate_to_s3") else {
        return Vec::new();
    };
    let rest = &line[at + "migrate_to_s3".len()..];
    let rest = rest.split(['|', ';', '>', '&']).next().unwrap_or("");
    // After `cargo run ... --bin migrate_to_s3`, the tool's options follow ` -- `.
    let rest = match rest.find(" -- ") {
        Some(sep) => &rest[sep + 4..],
        None if line[..at].contains("cargo ") => "",
        None => rest,
    };
    let options = migrate_to_s3_options();
    rest.split_whitespace()
        .filter(|t| t.starts_with('-'))
        .map(|token| token.split('=').next().unwrap())
        .filter(|option| *option != "--" && !options.contains(*option))
        .map(|option| format!("`migrate_to_s3 {option}`: no such option"))
        .collect()
}

/// `npm run` with a script frontend/package.json does not define.
fn npm_problems(line: &str) -> Vec<String> {
    static NPM_RUN: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"npm\s+run\s+([\w:.-]+)").unwrap());
    NPM_RUN
        .captures_iter(line)
        .filter(|caps| !npm_scripts().contains(&caps[1]))
        .map(|caps| format!("`npm run {}`: no such script", &caps[1]))
        .collect()
}

/// `helm --set` of a key the chart does not have.
fn helm_problems(line: &str) -> Vec<String> {
    static HELM_SET: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"--set(?:-string)?[= ](\S+)").unwrap());
    if !line.contains("helm ") {
        return Vec::new();
    }
    HELM_SET
        .captures_iter(line)
        .flat_map(|caps| {
            caps[1]
                .trim_matches(['"', '\''])
                .split(',')
                .map(|assignment| assignment.split('=').next().unwrap().to_string())
                .collect::<Vec<_>>()
        })
        .filter(|key| !helm_key_exists(key))
        .map(|key| format!("`--set {key}`: not a value of charts/readur"))
        .collect()
}

static HELM_VALUES: LazyLock<serde_yaml_ng::Value> = LazyLock::new(|| {
    serde_yaml_ng::from_str(&markdown::read("charts/readur/values.yaml")).unwrap()
});

/// Whether `key` (`a.b[0].c`, dots escaped as `\.`) is a value of charts/readur/values.yaml, or
/// an environment variable of the Readur container that the configuration reference lists.
fn helm_key_exists(key: &str) -> bool {
    const ENV: &str = "controllers.main.containers.readur.env.";
    if let Some(var) = key.strip_prefix(ENV) {
        return reference_variables().contains(var);
    }
    let unescaped = key.replace("\\.", "\u{0}");
    let mut node = &*HELM_VALUES;
    for part in unescaped.split('.') {
        let part = part.replace('\u{0}', ".");
        let (name, index) = match part.split_once('[') {
            Some((name, rest)) => (
                name.to_string(),
                rest.trim_end_matches(']').parse::<usize>().ok(),
            ),
            None => (part, None),
        };
        let Some(next) = node.get(name.as_str()) else {
            return false;
        };
        node = match index {
            Some(i) => match next.get(i) {
                Some(item) => item,
                None => return false,
            },
            None => next,
        };
    }
    true
}

#[test]
fn commands_in_the_docs_exist() {
    // A value path (`auth.jwtSecret`, `ingress.main.hosts[0].host`), not a file name such as
    // `auth.spec.ts`.
    let helm_path = Regex::new(
        r"^(?:auth|controllers|persistence|service|ingress)(?:\.[A-Za-z_]\w*(?:\[\d+\])?)+$",
    )
    .unwrap();
    let file_name = Regex::new(r"\.(?:ts|tsx|js|rs|md|ya?ml|json|toml|html)$").unwrap();
    let mut problems = Vec::new();
    let lines = command_lines();
    assert!(lines.len() > 500, "too few command lines found");
    for (page, line) in &lines {
        for problem in command_problems(line) {
            problems.push(format!("{page}: {problem}   (in `{}`)", line.trim()));
        }
        let span = line.trim();
        if helm_path.is_match(span) && !file_name.is_match(span) && !helm_key_exists(span) {
            problems.push(format!(
                "{page}: `{}` is not a value of charts/readur",
                line.trim()
            ));
        }
    }
    assert!(
        problems.is_empty(),
        "commands the code does not provide:\n{}",
        problems.join("\n")
    );
}

#[test]
fn command_checks_follow_the_documented_rules() {
    let subcommands = readur_subcommands();
    assert_eq!(
        subcommands,
        ["reset-admin-password", "rotate-jwt-secret", "serve"]
            .map(String::from)
            .into()
    );
    assert!(binaries().contains("migrate_to_s3") && binaries().contains("readur"));
    assert!(features().contains("s3") && features().contains("ocr"));
    assert!(migrate_to_s3_options().contains("--dry-run"));
    assert!(migrate_to_s3_options().contains("-d"));
    assert!(npm_scripts().contains("test:e2e"));

    for ok in [
        "readur",
        "readur serve",
        "./readur rotate-jwt-secret",
        "docker compose exec readur /app/readur reset-admin-password",
        "kubectl exec deploy/readur -- /app/readur reset-admin-password",
        "docker exec -e ADMIN_PASSWORD=x readur /app/readur reset-admin-password",
        "docker exec -it readur psql -U readur",
        "docker compose logs readur",
        "cargo run --release --bin migrate_to_s3 -- --dry-run --limit 10",
        "cargo run --bin migrate_to_s3 --features s3 -- --enable-rollback",
        "./target/release/migrate_to_s3 --delete-local",
        "cargo test --test integration_docs_claims_tests",
        "RUST_LOG=debug cargo run --bin readur",
        "cargo watch -x 'run --bin readur'",
        "npm run test:e2e",
        "helm install readur ./charts/readur --set auth.jwtSecret=x,ingress.main.enabled=true",
        "helm upgrade r . --set controllers.main.containers.readur.env.PUBLIC_URL=https://x",
        "helm upgrade r . --set ingress.main.hosts[0].host=readur.example.com",
    ] {
        assert_eq!(command_problems(ok), Vec::<String>::new(), "{ok}");
    }
    for bad in [
        "readur migrate",
        "docker compose exec readur /app/readur create-user",
        "docker compose exec readur readur reset-admin-password",
        "kubectl exec deploy/readur -- env ADMIN_PASSWORD=x readur reset-admin-password",
        "cargo run --bin readur-cli",
        "cargo run",
        "RUST_LOG=debug cargo run --release",
        "cargo watch -x run",
        "cargo build --features redis",
        "cargo test --test no_such_tests",
        "cargo run --bin migrate_to_s3 -- --bucket x",
        "npm run lint:fix",
        "helm install r . --set redis.enabled=true",
        "helm install r . --set controllers.main.containers.readur.env.REDIS_URL=x",
    ] {
        assert!(!command_problems(bad).is_empty(), "{bad}");
    }
}

/// The anchor MkDocs gives a heading (Python-Markdown's `toc` slugify): Markdown markup and
/// non-ASCII characters dropped, then characters other than word characters, spaces and
/// hyphens, lowercased, runs of spaces and hyphens turned into one hyphen.
fn slug(heading: &str) -> String {
    static LINK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[([^\]]*)\]\([^)]*\)").unwrap());
    let text = heading.trim_start_matches('#').trim();
    let text = text.split(" {#").next().unwrap();
    let text = LINK.replace_all(text, "$1");
    let kept: String = text
        .chars()
        .filter(|c| {
            c.is_ascii()
                && (c.is_ascii_alphanumeric() || *c == '_' || *c == '-' || c.is_ascii_whitespace())
        })
        .collect::<String>()
        .trim()
        .to_lowercase();
    let mut out = String::new();
    for ch in kept.chars() {
        if ch == '-' || ch.is_whitespace() {
            if !out.ends_with('-') {
                out.push('-');
            }
        } else {
            out.push(ch);
        }
    }
    out
}

/// The anchors of a Markdown page: its headings' slugs (repeated headings get `_1`, `_2`, ...
/// as in MkDocs) and explicit `{#id}` attributes.
fn anchors(text: &str) -> BTreeSet<String> {
    let explicit = Regex::new(r"\{#([\w-]+)\}").unwrap();
    let mut out = BTreeSet::new();
    let mut in_code = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code || markdown::heading_level(line) == 0 {
            continue;
        }
        if let Some(caps) = explicit.captures(line) {
            out.insert(caps[1].to_string());
            continue;
        }
        let base = slug(line);
        let mut candidate = base.clone();
        let mut n = 0;
        while out.contains(&candidate) {
            n += 1;
            candidate = format!("{base}_{n}");
        }
        out.insert(candidate);
    }
    out
}

/// The relative link targets of `text` outside code blocks and code spans.
fn relative_links(text: &str) -> Vec<String> {
    let link = Regex::new(r"\]\(([^)\s]+)(?:\s+[^)]*)?\)").unwrap();
    let span = Regex::new(r"`[^`]*`").unwrap();
    let mut out = Vec::new();
    let mut in_code = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            continue;
        }
        let line = span.replace_all(line, "");
        for caps in link.captures_iter(&line) {
            let target = caps[1].trim_matches(['<', '>']);
            if !target.contains("://") && !target.starts_with("mailto:") {
                out.push(target.to_string());
            }
        }
    }
    out
}

#[test]
fn links_resolve_to_files_and_headings() {
    let mut broken = Vec::new();
    let mut checked = 0;
    for (page, text) in pages() {
        let dir = Path::new(&page).parent().unwrap().to_path_buf();
        for target in relative_links(&text) {
            checked += 1;
            let (path, anchor) = match target.split_once('#') {
                Some((path, anchor)) => (path, Some(anchor)),
                None => (target.as_str(), None),
            };
            let file: PathBuf = if path.is_empty() {
                PathBuf::from(&page)
            } else {
                normalize(&dir.join(path))
            };
            let full = repo().join(&file);
            if !full.exists() {
                broken.push(format!(
                    "{page}: ({target}): {} does not exist",
                    file.display()
                ));
                continue;
            }
            if let Some(anchor) = anchor {
                if full.extension().is_some_and(|e| e == "md") {
                    let target_text = std::fs::read_to_string(&full).unwrap();
                    if !anchors(&target_text).contains(anchor) {
                        broken.push(format!("{page}: ({target}): no heading #{anchor}"));
                    }
                }
            }
        }
    }
    assert!(checked > 200, "too few links found");
    assert!(broken.is_empty(), "broken links:\n{}", broken.join("\n"));
}

/// `path` with `.` and `..` components resolved lexically.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

#[test]
fn slugs_follow_mkdocs() {
    assert_eq!(slug("## Quick Start"), "quick-start");
    assert_eq!(
        slug("### `POST /api/auth/login` - Log in"),
        "post-apiauthlogin-log-in"
    );
    assert_eq!(slug("## S3 & WebDAV: Setup"), "s3-webdav-setup");
    assert_eq!(slug("## Use [Docker](docker.md) 🐳"), "use-docker");
    assert_eq!(slug("## READ_ME_first"), "read_me_first");
    let page = "# A\n## Same\n```\n## not a heading\n```\n## Same\n## Custom {#mine}\n";
    let found: Vec<String> = anchors(page).into_iter().collect();
    assert_eq!(found, ["a", "mine", "same", "same_1"]);
}

#[test]
fn every_page_in_the_navigation_exists() {
    let config: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&markdown::read("mkdocs.yml")).expect("mkdocs.yml parses");
    fn collect(node: &serde_yaml_ng::Value, out: &mut Vec<String>) {
        match node {
            serde_yaml_ng::Value::String(s) if s.ends_with(".md") => out.push(s.clone()),
            serde_yaml_ng::Value::Sequence(items) => items.iter().for_each(|i| collect(i, out)),
            serde_yaml_ng::Value::Mapping(map) => map.values().for_each(|v| collect(v, out)),
            _ => {}
        }
    }
    let mut pages = Vec::new();
    collect(&config["nav"], &mut pages);
    assert!(pages.len() > 30, "too few navigation entries: {pages:?}");
    let missing: Vec<&String> = pages
        .iter()
        .filter(|p| !repo().join("docs").join(p).is_file())
        .collect();
    assert!(
        missing.is_empty(),
        "navigation entries without a page: {missing:?}"
    );
}

/// Real public sites the docs may link to or download from.
const PUBLIC_SITES: &[&str] = &[
    "github.com",
    "raw.githubusercontent.com",
    "ghcr.io",
    "readur.app",
    "docs.rs",
    "crates.io",
    "rustup.rs",
    "docs.docker.com",
    "hub.docker.com",
    "nodejs.org",
    "www.postgresql.org",
    "postgresql.org",
    "tesseract-ocr.github.io",
    "helm.sh",
    "kubernetes.io",
    "prometheus.io",
    "grafana.com",
    "playwright.dev",
    "modelcontextprotocol.io",
    "letsencrypt.org",
    "nginx.org",
    "caddyserver.com",
    "doc.traefik.io",
    "aws.amazon.com",
    "docs.aws.amazon.com",
    "min.io",
    "unpkg.com",
    "swagger.io",
    "cert-manager.io",
    "openid.net",
    "datatracker.ietf.org",
    // Identity providers and their consoles (issuer URLs in the OIDC guide).
    "accounts.google.com",
    "console.cloud.google.com",
    "login.microsoftonline.com",
    "portal.azure.com",
    "auth0.com",
    "okta.com",
    "goauthentik.io",
    // S3-compatible storage providers.
    "amazonaws.com",
    "digitaloceanspaces.com",
    "backblazeb2.com",
    // Reference documentation.
    "doc.rust-lang.org",
    "react.dev",
    "vitest.dev",
    "testing-library.com",
    "mui.com",
    "www.npmjs.com",
    "squidfunk.github.io",
    "www.mkdocs.org",
    "www.markdownguide.org",
    "owasp.org",
    "www.cisecurity.org",
    "www.nist.gov",
    "securityheaders.com",
    "ssl-config.mozilla.org",
];

/// Whether `host` is an example, local or placeholder host, or a public site.
fn host_allowed(host: &str) -> bool {
    static PRIVATE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(127\.|10\.|192\.168\.|172\.(1[6-9]|2\d|3[01])\.|192\.0\.2\.|198\.51\.100\.|203\.0\.113\.|0\.0\.0\.0$|\[::1?\]$)")
            .unwrap()
    });
    let host = host.to_ascii_lowercase();
    if host.contains(['<', '{', '$', '*']) || !host.contains('.') && !host.starts_with('[') {
        return true;
    }
    if PRIVATE.is_match(&host) {
        return true;
    }
    // Example domains (RFC 2606/6761 names, Kubernetes and mDNS suffixes): the domain itself
    // or any subdomain.
    let example_domains = [
        "example.com",
        "example.org",
        "example.net",
        "example",
        "local",
        "localhost",
        "internal",
        "test",
        "svc",
    ];
    let under = |domain: &str| host == domain || host.ends_with(&format!(".{domain}"));
    example_domains.iter().any(|d| under(d)) || PUBLIC_SITES.iter().any(|site| under(site))
}

#[test]
fn hostnames_are_examples_or_public_sites() {
    let url = Regex::new(r#"(?:https?|wss?)://([^/\s:"'`)<>\],]+)"#).unwrap();
    let mut bad = BTreeSet::new();
    for (page, text) in pages() {
        for caps in url.captures_iter(&text) {
            if !host_allowed(&caps[1]) {
                bad.insert(format!("{page}: {}", &caps[1]));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "hostnames that are not examples (use readur.example.com) or listed public sites:\n{}",
        bad.into_iter().collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn host_check_follows_the_documented_rules() {
    for ok in [
        "localhost",
        "127.0.0.1",
        "192.168.1.10",
        "readur.example.com",
        "example.com",
        "auth.example.org",
        "readur.local",
        "postgres",
        "<your-host>",
        "${DOMAIN}",
        "github.com",
        "docs.github.com",
    ] {
        assert!(host_allowed(ok), "{ok}");
    }
    for bad in [
        "readur.mycompany.com",
        "8.8.8.8",
        "notexample.com",
        "nas.home.arpa",
    ] {
        assert!(!host_allowed(bad), "{bad}");
    }
}
