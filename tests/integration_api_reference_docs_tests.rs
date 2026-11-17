//! Runs every request example of docs/api-reference.md against a fresh server and compares
//! the documented responses with the real ones, and checks the reference's parameter, error
//! code and rate limit tables against the code.
//!
//! # What an example is
//!
//! An example is a `####` section (up to the next heading of level 4 or higher) holding a
//! fenced `http` block. The block's first line is `METHOD /path[?query]`; later lines are
//! headers for the reader and are not sent (the test authenticates as the registry below says).
//! Inside the section:
//! - a fenced `json` block whose last non-blank line before it starts `**Request body:**` is
//!   the request body;
//! - exactly one line starts `**Response:**` and names the status in a code span
//!   (`` **Response:** `201 Created` ``); a fenced `json` block whose last non-blank line before
//!   it is that line is the response body.
//!
//! Every fenced `json` block of the reference must belong to an example in one of those two
//! roles, and every example must be in [`EXAMPLES`], so nothing in the reference goes
//! unchecked.
//!
//! # How an example is checked
//!
//! Each example runs on its own fresh server and database with two accounts, `alice` (a user)
//! and `admin` (an administrator), plus what its setup in [`setup`] creates. Self-registration
//! needs administrator approval, as on every production server. The setup also maps literal
//! text of the example (placeholder ids such as `{id}` or a documented UUID) to the real
//! values, applied to the request line and body before sending. Then:
//! - the response status must be the documented one;
//! - a documented response body must equal the real one: the same keys in every object, the
//!   same length for every array and the same values, except for [`VOLATILE`] keys (ids,
//!   tokens, timestamps), whose values need only have the same JSON type (so `null` versus a
//!   value still differs);
//! - every object in a documented request body must be described by a schema the setup names
//!   for its JSON pointer, and use only that schema's property names. (Serde ignores unknown
//!   fields, so running the request alone would not catch a documented field that does not
//!   exist.)
//!
//! The search examples are checked against their own document library by
//! tests/integration_api_reference_search_docs_tests.rs and are listed here as external.

use std::collections::{BTreeMap, BTreeSet};

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use regex::Regex;
use serde_json::{json, Value};
use tower::ServiceExt;
use utoipa::OpenApi;
use uuid::Uuid;

use readur::errors::{
    api_key::ApiKeyError, comment::CommentError, shared_link::SharedLinkError, AppError,
};

#[path = "support/api_reference_examples.rs"]
mod examples;
#[path = "support/markdown.rs"]
mod markdown;

use examples::{setup, Fixture, EXAMPLES};

fn api_reference() -> String {
    markdown::read("docs/api-reference.md")
}

/// Keys whose values change from run to run; only their JSON type is compared.
const VOLATILE: &[&str] = &[
    "id",
    "user_id",
    "document_id",
    "parent_id",
    "token",
    "plaintext",
    "key_prefix",
    "url",
    "file_path",
    "file_hash",
    "created_at",
    "updated_at",
    "expires_at",
    "last_used_at",
    "timestamp",
    "request_id",
];

/// One parsed example.
#[derive(Debug)]
struct Example {
    heading: String,
    method: String,
    path: String,
    request: Option<String>,
    status: u16,
    response: Option<Value>,
}

/// The examples of `doc`, and every json block that belongs to none (see the module docs).
fn parse_examples(doc: &str) -> (Vec<Example>, Vec<String>) {
    let status_re = Regex::new(r"^\*\*Response:\*\*.*?`(\d{3}) [^`]+`").unwrap();
    let mut examples = Vec::new();
    let mut stray = Vec::new();
    for section in markdown::sections(doc, 4) {
        let blocks = markdown::code_blocks(&section.body);
        let Some(http) = blocks.iter().find(|b| b.lang == "http") else {
            stray.extend(
                blocks
                    .iter()
                    .filter(|b| b.lang == "json")
                    .map(|_| section.heading.clone()),
            );
            continue;
        };
        let line = http.text.lines().next().unwrap_or("").trim();
        let (method, path) = line
            .split_once(' ')
            .unwrap_or_else(|| panic!("{}: bad request line {line:?}", section.heading));
        let response_lines: Vec<&str> = section
            .body
            .lines()
            .filter(|l| l.starts_with("**Response:**"))
            .collect();
        assert_eq!(
            response_lines.len(),
            1,
            "{}: needs one **Response:** line",
            section.heading
        );
        let status = status_re
            .captures(response_lines[0])
            .unwrap_or_else(|| panic!("{}: no status in {:?}", section.heading, response_lines[0]))
            [1]
        .parse()
        .unwrap();
        let mut request = None;
        let mut response = None;
        for block in blocks.iter().filter(|b| b.lang == "json") {
            if block.preceded_by.starts_with("**Request body:**") && request.is_none() {
                request = Some(block.text.clone());
            } else if block.preceded_by.starts_with("**Response:**") && response.is_none() {
                let value = serde_json::from_str(&block.text)
                    .unwrap_or_else(|e| panic!("{}: invalid response JSON: {e}", section.heading));
                response = Some(value);
            } else {
                stray.push(section.heading.clone());
            }
        }
        examples.push(Example {
            heading: section.heading.trim_start_matches("#### ").to_string(),
            method: method.to_string(),
            path: path.to_string(),
            request,
            status,
            response,
        });
    }
    (examples, stray)
}

#[test]
fn every_example_is_registered_and_every_json_block_is_an_example() {
    let (examples, stray) = parse_examples(&api_reference());
    assert!(
        stray.is_empty(),
        "json blocks outside a request or response role, under: {stray:?}"
    );
    let documented: BTreeSet<&str> = examples.iter().map(|e| e.heading.as_str()).collect();
    assert_eq!(
        documented.len(),
        examples.len(),
        "two examples share a heading"
    );
    let registered: BTreeSet<&str> = EXAMPLES.iter().map(|(h, _)| *h).collect();
    assert_eq!(documented, registered, "documented examples vs EXAMPLES");
    assert!(
        examples.len() > 25,
        "only {} examples found",
        examples.len()
    );
}

#[test]
fn example_parsing_follows_the_documented_rules() {
    let doc = "\
#### Make a Thing

```http
POST /api/things
Authorization: Bearer <token>
```

**Request body:**
```json
{\"name\": \"x\"}
```

**Response:** `201 Created`
```json
{\"id\": 1}
```

#### Prose

```json
{}
```
";
    let (examples, stray) = parse_examples(doc);
    assert_eq!(stray, ["#### Prose"]);
    let e = &examples[0];
    assert_eq!(
        (e.heading.as_str(), e.method.as_str(), e.path.as_str()),
        ("Make a Thing", "POST", "/api/things")
    );
    assert_eq!(e.request.as_deref(), Some("{\"name\": \"x\"}\n"));
    assert_eq!(
        (e.status, e.response.clone()),
        (201, Some(json!({"id": 1})))
    );
}

fn json_type(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// Differences between a documented and a real response (see the module docs).
fn compare(doc: &Value, real: &Value, at: &str, out: &mut Vec<String>) {
    match (doc, real) {
        (Value::Object(d), Value::Object(r)) => {
            let dk: BTreeSet<&String> = d.keys().collect();
            let rk: BTreeSet<&String> = r.keys().collect();
            if dk != rk {
                out.push(format!("{at}: documented keys {dk:?}, real keys {rk:?}"));
                return;
            }
            for (key, d) in d {
                let here = format!("{at}/{key}");
                if VOLATILE.contains(&key.as_str()) {
                    if json_type(d) != json_type(&r[key]) {
                        out.push(format!("{here}: documented {d}, real {}", r[key]));
                    }
                } else {
                    compare(d, &r[key], &here, out);
                }
            }
        }
        (Value::Array(d), Value::Array(r)) => {
            if d.len() != r.len() {
                out.push(format!(
                    "{at}: documented {} items, real {}: {real}",
                    d.len(),
                    r.len()
                ));
                return;
            }
            for (i, (d, r)) in d.iter().zip(r).enumerate() {
                compare(d, r, &format!("{at}/{i}"), out);
            }
        }
        (Value::Number(d), Value::Number(r)) => {
            if (d.as_f64().unwrap() - r.as_f64().unwrap()).abs() > 1e-6 {
                out.push(format!("{at}: documented {d}, real {r}"));
            }
        }
        _ if doc != real => out.push(format!("{at}: documented {doc}, real {real}")),
        _ => {}
    }
}

#[test]
fn comparison_follows_the_documented_rules() {
    let diff = |doc: Value, real: Value| {
        let mut out = Vec::new();
        compare(&doc, &real, "", &mut out);
        out
    };
    assert!(diff(json!({"id": "a", "n": 1.5}), json!({"id": "b", "n": 1.5})).is_empty());
    assert_eq!(diff(json!({"id": "a"}), json!({"id": null})).len(), 1);
    assert_eq!(diff(json!({"a": 1}), json!({"a": 1, "b": 2})).len(), 1);
    assert_eq!(diff(json!([1]), json!([1, 2])).len(), 1);
    assert_eq!(diff(json!({"name": "x"}), json!({"name": "y"})).len(), 1);
}

/// The property names of a serialized schema.
fn properties(schema: &Value) -> BTreeSet<String> {
    schema["properties"]
        .as_object()
        .map(|p| p.keys().cloned().collect())
        .unwrap_or_default()
}

/// Request-body objects that no schema covers, or that use a name the schema lacks.
fn check_request(body: &Value, at: &str, schemas: &BTreeMap<&str, Value>, out: &mut Vec<String>) {
    match body {
        Value::Object(map) => {
            let Some(schema) = schemas.get(at) else {
                out.push(format!(
                    "request {at:?}: no schema to check this object against"
                ));
                return;
            };
            let known = properties(schema);
            for (key, value) in map {
                if !known.contains(key) {
                    out.push(format!(
                        "request {at}/{key}: not a field (fields: {known:?})"
                    ));
                }
                check_request(value, &format!("{at}/{key}"), schemas, out);
            }
        }
        Value::Array(items) => {
            for item in items {
                check_request(item, &format!("{at}/0"), schemas, out);
            }
        }
        _ => {}
    }
}

async fn run_example(example: &Example) -> Vec<String> {
    let fx = Fixture::new(&example.heading).await;
    let setup = setup(&example.heading, &fx).await;
    let apply = |text: &str| {
        setup
            .subst
            .iter()
            .fold(text.to_string(), |t, (from, to)| t.replace(from, to))
    };
    let mut request = Request::builder()
        .method(Method::from_bytes(example.method.as_bytes()).unwrap())
        .uri(apply(&example.path));
    if let Some(token) = fx.token(setup.who) {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let body = if let Some(text) = &example.request {
        request = request.header("content-type", "application/json");
        Body::from(apply(text))
    } else if let Some((filename, mime, bytes)) = &setup.multipart {
        let boundary = "readur-docs-boundary";
        request = request.header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        );
        let mut data = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: {mime}\r\n\r\n"
        )
        .into_bytes();
        data.extend_from_slice(bytes);
        data.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        Body::from(data)
    } else {
        Body::empty()
    };
    let response = fx
        .ctx
        .app()
        .clone()
        .oneshot(request.body(body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let real: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);

    let mut out = Vec::new();
    if status != StatusCode::from_u16(example.status).unwrap() {
        out.push(format!(
            "status: documented {}, real {status}: {real}",
            example.status
        ));
    }
    if let Some(documented) = &example.response {
        compare(documented, &real, "", &mut out);
    }
    if let Some(text) = &example.request {
        let body: Value = serde_json::from_str(text).expect("request body is not JSON");
        let schemas: BTreeMap<&str, Value> = setup.request_schemas.into_iter().collect();
        check_request(&body, "", &schemas, &mut out);
    }
    let _ = fx.ctx.cleanup_and_close().await;
    out
}

#[tokio::test]
async fn every_example_returns_the_documented_response() {
    let (examples, _) = parse_examples(&api_reference());
    let mut failures = Vec::new();
    let mut ran = 0;
    for example in &examples {
        let external = EXAMPLES
            .iter()
            .any(|(h, ext)| *h == example.heading && ext.is_some());
        if external {
            continue;
        }
        ran += 1;
        for problem in run_example(example).await {
            failures.push(format!(
                "{} ({} {}): {problem}",
                example.heading, example.method, example.path
            ));
        }
    }
    assert!(ran > 20, "only {ran} examples ran");
    assert!(
        failures.is_empty(),
        "examples that do not match the server:\n{}",
        failures.join("\n")
    );
}

/// The parameter names in the first column of the table under `heading`.
fn table_names(doc: &str, heading: &str) -> Vec<String> {
    let section = markdown::sections(doc, heading.chars().take_while(|&c| c == '#').count())
        .into_iter()
        .find(|s| s.heading == heading)
        .unwrap_or_else(|| panic!("no {heading:?} section"));
    section
        .body
        .lines()
        .filter_map(markdown::table_row)
        .skip(1)
        .filter_map(|cells| markdown::code_span(&cells[0]).map(str::to_string))
        .collect()
}

fn openapi_query_params(path: &str) -> BTreeSet<String> {
    let api = readur::swagger::ApiDoc::openapi();
    let get = api.paths.paths[path].get.as_ref().unwrap();
    get.parameters
        .iter()
        .flatten()
        .map(|p| match p {
            utoipa::openapi::RefOr::T(p) => p,
            utoipa::openapi::RefOr::Ref(r) => panic!("{path}: unresolved {}", r.ref_location),
        })
        .filter(|p| matches!(p.parameter_in, utoipa::openapi::path::ParameterIn::Query))
        .map(|p| p.name.clone())
        .collect()
}

#[test]
fn documented_query_parameters_are_the_real_ones() {
    let doc = api_reference();
    for (heading, path) in [
        ("#### Document List Parameters", "/api/documents"),
        ("#### Search Parameters", "/api/search/enhanced"),
    ] {
        let names = table_names(&doc, heading);
        let set: BTreeSet<String> = names.iter().cloned().collect();
        assert_eq!(
            set.len(),
            names.len(),
            "{heading}: a parameter is listed twice"
        );
        assert_eq!(
            set,
            openapi_query_params(path),
            "{heading} vs OpenAPI {path}"
        );
    }
}

/// `(code, status)` of every error the three tabled error types can return.
fn error_codes() -> Vec<(&'static str, Vec<(String, u16)>)> {
    fn pairs<E: AppError>(errors: Vec<E>) -> Vec<(String, u16)> {
        errors
            .iter()
            .map(|e| (e.error_code().to_string(), e.status_code().as_u16()))
            .collect()
    }
    let s = String::new;
    vec![
        (
            "Shared Links",
            pairs(vec![
                SharedLinkError::NotFound,
                SharedLinkError::Expired,
                SharedLinkError::Revoked,
                SharedLinkError::PasswordRequired,
                SharedLinkError::InvalidPassword,
                SharedLinkError::MaxViewsReached,
                SharedLinkError::PermissionDenied { reason: s() },
                SharedLinkError::DocumentNotFound,
                SharedLinkError::RateLimited {
                    retry_after_secs: 1,
                },
                SharedLinkError::InternalError { message: s() },
            ]),
        ),
        (
            "API Keys",
            pairs(vec![
                ApiKeyError::NotFound,
                ApiKeyError::InvalidRequest { reason: s() },
                ApiKeyError::MaxKeysReached { limit: 20 },
                ApiKeyError::PermissionDenied { reason: s() },
                ApiKeyError::RateLimited {
                    retry_after_secs: 1,
                },
                ApiKeyError::InternalError { message: s() },
            ]),
        ),
        (
            "Comments",
            pairs(vec![
                CommentError::NotFound,
                CommentError::DocumentNotFound,
                CommentError::PermissionDenied { reason: s() },
                CommentError::ContentEmpty,
                CommentError::ContentTooLong {
                    length: 0,
                    max_length: 0,
                },
                CommentError::ParentNotFound,
                CommentError::NestingTooDeep,
                CommentError::RateLimited {
                    retry_after_secs: 1,
                },
                CommentError::InternalError { message: s() },
            ]),
        ),
    ]
}

#[test]
fn error_code_tables_list_every_code_with_its_status() {
    let doc = api_reference();
    for (area, real) in error_codes() {
        let heading = format!("#### {area} Error Codes");
        let section = markdown::sections(&doc, 4)
            .into_iter()
            .find(|s| s.heading == heading)
            .unwrap_or_else(|| panic!("no {heading:?} section"));
        let documented: BTreeSet<(String, u16)> = section
            .body
            .lines()
            .filter_map(markdown::table_row)
            .skip(1)
            .map(|cells| {
                let code = markdown::code_span(&cells[0]).unwrap().to_string();
                (code, cells[1].parse().unwrap())
            })
            .collect();
        let real: BTreeSet<(String, u16)> = real.into_iter().collect();
        assert_eq!(documented, real, "{heading} vs the error type");
    }
}

/// Checks `limiter` with one key until it refuses: the number of requests it allowed, and the
/// wait it then asked for.
async fn exhaust<K: std::hash::Hash + Eq + Clone>(
    limiter: &readur::rate_limit::RateLimiter<K>,
    key: K,
) -> (u32, u64) {
    for allowed in 0..10_000 {
        if let Err(retry_after) = limiter.check(&key).await {
            return (allowed, retry_after);
        }
    }
    panic!("the limiter never refused");
}

/// `(requests allowed, window in seconds)` for the limiter a rate limit table row describes.
async fn measured_limit(row: &str) -> (u32, u64) {
    let limiters = readur::rate_limit::RateLimiters::new();
    let ip: std::net::IpAddr = "192.0.2.1".parse().unwrap();
    let user = Uuid::new_v4();
    let name = || "alice".to_string();
    match row {
        "Failed logins for one username from one address" => {
            exhaust(&limiters.login_failures_by_account_ip, (name(), ip)).await
        }
        "Failed logins for one username" => {
            exhaust(&limiters.login_failures_by_username, name()).await
        }
        "Failed logins from one address" => exhaust(&limiters.login_failures_by_ip, ip).await,
        "Self-registrations from one address" => exhaust(&limiters.registration_by_ip, ip).await,
        "OIDC login starts and code exchanges from one address" => {
            exhaust(&limiters.auth_misc_by_ip, ip).await
        }
        "Shared link password checks from one address" => {
            exhaust(&limiters.shared_link_password, ip).await
        }
        "Shared link access from one address" => exhaust(&limiters.shared_link_public, ip).await,
        "Comments created by one user" => exhaust(&limiters.comment_creation, user).await,
        "Shared links created by one user" => exhaust(&limiters.shared_link_creation, user).await,
        "API keys created by one user" => exhaust(&limiters.api_key_creation, user).await,
        _ => panic!("rate limit row {row:?} is not mapped to a limiter"),
    }
}

/// Every limiter of `RateLimiters`, which the table must describe one row each.
const LIMITERS: usize = 10;

#[tokio::test]
async fn rate_limit_table_matches_the_limiters() {
    let doc = api_reference();
    let section = markdown::sections(&doc, 2)
        .into_iter()
        .find(|s| s.heading == "## Rate Limiting")
        .expect("no Rate Limiting section");
    let limit = Regex::new(r"^(\d+) per (minute|hour|15 minutes)$").unwrap();
    let rows: Vec<Vec<String>> = section
        .body
        .lines()
        .filter_map(markdown::table_row)
        .skip(1)
        .collect();
    assert_eq!(rows.len(), LIMITERS, "one row per limiter");
    for cells in rows {
        let caps = limit.captures(&cells[1]).unwrap_or_else(|| {
            panic!(
                "{:?}: limit {:?} is not `N per minute|hour|15 minutes`",
                cells[0], cells[1]
            )
        });
        let count: u32 = caps[1].parse().unwrap();
        let window: u64 = match &caps[2] {
            "minute" => 60,
            "hour" => 3600,
            _ => 900,
        };
        let (allowed, retry_after) = measured_limit(&cells[0]).await;
        assert_eq!(allowed, count, "{:?}: requests allowed", cells[0]);
        assert!(
            retry_after <= window && retry_after + 5 > window,
            "{:?}: retry after {retry_after}s, documented window {window}s",
            cells[0]
        );
    }
}
