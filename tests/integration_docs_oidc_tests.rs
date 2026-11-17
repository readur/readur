//! docs/oidc-setup.md checked against the code: its "Environment Variables" table lists every
//! `OIDC_*` variable the server reads, and its Required column is what the server does.
//!
//! The table's rows are `` | `NAME` | Yes or No | description | example | ``. "Required" means
//! OIDC sign-in needs the variable: with every `Yes` variable set (and nothing else), the
//! configuration loads and the OIDC client initializes against a provider; leaving out any one
//! of them either turns OIDC off (`OIDC_ENABLED`) or makes the client fail to initialize, while
//! the configuration itself still loads, as the guide says ("the server still starts").
//!
//! The provider is a local mock serving a discovery document. Each check loads the
//! configuration with `Config::from_env` while holding a lock, with every variable the check
//! depends on cleared first and restored afterwards.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::sync::Mutex;

use regex::Regex;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use readur::config::Config;
use readur::oidc::OidcClient;

#[path = "support/markdown.rs"]
mod markdown;

static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Variables cleared around every load.
const ISOLATED: &[&str] = &[
    "OIDC_ENABLED",
    "OIDC_CLIENT_ID",
    "OIDC_CLIENT_SECRET",
    "OIDC_ISSUER_URL",
    "OIDC_REDIRECT_URI",
    "OIDC_AUTO_REGISTER",
    "OIDC_LINK_EXISTING_BY_EMAIL",
    "ALLOW_LOCAL_AUTH",
    "PUBLIC_URL",
    "DATABASE_URL",
    "JWT_SECRET",
    "READUR_INSECURE_DEV_MODE",
];

/// `Config::from_env` with exactly `vars` set among [`ISOLATED`] (plus a database URL).
fn load(vars: &[(&str, String)]) -> anyhow::Result<Config> {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let saved: Vec<(&str, Option<String>)> =
        ISOLATED.iter().map(|k| (*k, env::var(k).ok())).collect();
    for key in ISOLATED {
        env::remove_var(key);
    }
    env::set_var("DATABASE_URL", "postgresql://test:test@localhost/test");
    for (key, value) in vars {
        env::set_var(key, value);
    }
    let result = Config::from_env();
    for (key, value) in saved {
        match value {
            Some(value) => env::set_var(key, value),
            None => env::remove_var(key),
        }
    }
    result
}

/// `NAME -> required` for every row of the guide's environment variable table.
fn documented_requirements() -> BTreeMap<String, bool> {
    let doc = markdown::read("docs/oidc-setup.md");
    let section = markdown::sections(&doc, 3)
        .into_iter()
        .find(|s| s.heading == "### Environment Variables")
        .expect("no Environment Variables section");
    section
        .body
        .lines()
        .filter_map(markdown::table_row)
        .skip(1)
        .map(|cells| {
            let name = markdown::code_span(&cells[0])
                .expect("variable in a code span")
                .to_string();
            let required = match cells[1].as_str() {
                "Yes" => true,
                "No" => false,
                other => panic!("{name}: Required is {other:?}, not Yes or No"),
            };
            (name, required)
        })
        .collect()
}

/// A provider whose discovery document names it as the issuer.
async fn provider() -> MockServer {
    let server = MockServer::start().await;
    let base = server.uri();
    Mock::given(method("GET"))
        .and(path("/.well-known/openid-configuration"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "issuer": base,
            "authorization_endpoint": format!("{base}/authorize"),
            "token_endpoint": format!("{base}/token"),
            "jwks_uri": format!("{base}/jwks"),
        })))
        .mount(&server)
        .await;
    server
}

/// A working value for a documented variable.
fn sample(name: &str, issuer: &str) -> String {
    match name {
        "OIDC_ENABLED" => "true".into(),
        "OIDC_CLIENT_ID" => "readur".into(),
        "OIDC_CLIENT_SECRET" => "client-secret".into(),
        "OIDC_ISSUER_URL" => issuer.into(),
        "OIDC_REDIRECT_URI" => "https://readur.example.com/api/auth/oidc/callback".into(),
        _ => panic!("no sample value for documented required variable {name}"),
    }
}

/// Whether OIDC sign-in works with exactly `vars` set: the configuration must load either way.
async fn oidc_works(vars: &[(&str, String)]) -> bool {
    let config = load(vars).expect("the configuration must load whether or not OIDC works");
    config.oidc_enabled && OidcClient::new(&config).await.is_ok()
}

#[tokio::test]
async fn documented_required_variables_are_exactly_what_oidc_needs() {
    let server = provider().await;
    let documented = documented_requirements();
    let required: Vec<&str> = documented
        .iter()
        .filter(|(_, r)| **r)
        .map(|(n, _)| n.as_str())
        .collect();
    assert!(
        required.contains(&"OIDC_ENABLED"),
        "OIDC_ENABLED must be required: {required:?}"
    );
    let all: Vec<(&str, String)> = required
        .iter()
        .map(|n| (*n, sample(n, &server.uri())))
        .collect();

    // Every required variable and nothing else: OIDC works, so the rest are optional.
    assert!(
        oidc_works(&all).await,
        "OIDC must work with only the required variables {required:?}"
    );

    // Without any one of them it does not, but the server still starts.
    for missing in &required {
        let vars: Vec<(&str, String)> = all.iter().filter(|(n, _)| n != missing).cloned().collect();
        assert!(
            !oidc_works(&vars).await,
            "{missing} is documented as required, but OIDC works without it"
        );
    }
}

#[test]
fn only_disabling_both_sign_in_methods_stops_the_server() {
    let off = || ("ALLOW_LOCAL_AUTH", "false".to_string());
    assert!(
        load(&[off()]).is_err(),
        "no OIDC and no local login must stop the server"
    );
    assert!(load(&[off(), ("OIDC_ENABLED", "true".to_string())]).is_ok());
    assert!(
        load(&[("OIDC_ENABLED", "true".to_string())]).is_ok(),
        "an incomplete OIDC setup must not stop the server"
    );
}

#[test]
fn the_table_lists_every_oidc_variable_the_code_reads() {
    let reader = Regex::new(r#"(?:env::var|env_flag)\(\s*"(OIDC_[A-Z0-9_]+)""#).unwrap();
    let mut read = BTreeSet::new();
    for file in ["src/config.rs", "src/config/security.rs"] {
        let text = markdown::read(file);
        read.extend(reader.captures_iter(&text).map(|c| c[1].to_string()));
    }
    assert!(read.len() >= 6, "only {read:?} found; is the scan broken?");
    let documented: BTreeSet<String> = documented_requirements().into_keys().collect();
    let missing: Vec<&String> = read.difference(&documented).collect();
    assert!(
        missing.is_empty(),
        "OIDC variables missing from docs/oidc-setup.md: {missing:?}"
    );
}
