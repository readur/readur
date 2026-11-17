//! Keeps every API endpoint the documentation names on a real route, and keeps the endpoint
//! index of docs/api-reference.md complete and truthful about who may call each endpoint.
//!
//! # Which text counts as an endpoint mention
//!
//! README.md and every `.md` file under docs/ are scanned line by line (inside fenced code
//! blocks a line ending in `\` is joined with the next, so a multi-line `curl` command is one
//! line). Each occurrence of `/api/` starts a mention when the character before it is the start
//! of the line, whitespace or one of `` ` " ' ( [ = > ``, or when it directly follows a URL
//! authority (`http://host:port`, also `ws://` and `wss://`), a shell variable (`$BASE`, `${BASE}`) or a `}` (as in
//! `format!("{}/api/...")`). Anything else (`frontend/src/services/api/index.ts`,
//! `http://loki:3100/loki/api/v1/push`) is not a mention. Paths under `/api/v1/` are the
//! Prometheus and Loki APIs, not Readur's, and are skipped.
//!
//! The path runs over `A-Z a-z 0-9 _ - . / $ < > :` and balanced `{...}`, and stops at `?`,
//! `#` or any other character. Trailing `.`, `,`, `:` and `;` are dropped. The mention's method
//! is the HTTP method written right before it (`GET /api/x`, `POST https://host/api/x`); on a
//! `curl` line without one it is the `-X`/`--request` value, else POST when the command sends a
//! body (`-d`, `--data*`, `-F`, `--form`), else GET. A mention with no method only needs the
//! path to exist for some method; a path ending in `/` (as in an nginx `location /api/ {`)
//! only needs to be a prefix of a real route.
//!
//! # How a mention matches a route
//!
//! Path segments match one for one. A route segment `{param}` matches any placeholder the docs
//! write: `{anything}`, `<anything>`, `$VAR`, `${VAR}`, `:name`, an `UPPER_SNAKE` word, a UUID
//! or a number. A lowercase word never matches `{param}`, so `/api/documents/upload` does not
//! pass as `/api/documents/{id}`. The routes are tests/support/api_routes.rs: the OpenAPI
//! document plus the mounted routes without an annotation (the default-deny test probes them
//! all, so each is served).
//!
//! # The endpoint index
//!
//! docs/api-reference.md lists every route once, in tables whose rows read
//! `` | `METHOD /path` | access | description | ``, with access `public`, `user`, `admin` or
//! `admin or self`. The set of rows must equal the set of routes, and each row's access is
//! checked against the server: `public` routes answer without credentials, `user` routes
//! answer 401 without credentials and do not answer 401 or 403 to a regular user, `admin`
//! routes answer 403 to a regular user but not to an administrator, and `admin or self` routes
//! answer 403 to a regular user for another user's id but not for their own.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use regex::Regex;
use tower::ServiceExt;
use uuid::Uuid;

use readur::test_utils::TestContext;

#[path = "support/api_routes.rs"]
mod api_routes;
#[path = "support/markdown.rs"]
mod markdown;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Mention {
    method: Option<String>,
    path: String,
}

/// Lines of `text`, with `\`-continued lines inside code blocks joined.
fn logical_lines(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut in_code = false;
    let mut pending: Option<(usize, String)> = None;
    for (i, line) in text.lines().enumerate() {
        if line.trim_start().starts_with("```") {
            in_code = !in_code;
        }
        let (start, mut joined) = pending.take().unwrap_or((i + 1, String::new()));
        joined.push_str(line);
        if in_code && line.trim_end().ends_with('\\') {
            joined.pop();
            joined.push(' ');
            pending = Some((start, joined));
        } else {
            out.push((start, joined));
        }
    }
    out.extend(pending);
    out
}

/// The path starting at `rest` (which begins with `/api/`), per the module docs.
fn take_path(rest: &str) -> String {
    let mut path = String::new();
    let mut depth = 0;
    for c in rest.chars() {
        if depth > 0 {
            depth += usize::from(c == '{');
            depth -= usize::from(c == '}');
        } else if c == '{' {
            depth = 1;
        } else if !(c.is_ascii_alphanumeric() || "_-./$<>:".contains(c)) {
            break;
        }
        path.push(c);
    }
    path.trim_end_matches(['.', ',', ':', ';']).to_string()
}

fn curl_method(line: &str) -> String {
    let explicit = Regex::new(r"(?:-X\s*|--request\s+)([A-Z]+)").unwrap();
    if let Some(caps) = explicit.captures(line) {
        return caps[1].to_string();
    }
    let body = Regex::new(r"\s(?:-d|--data[a-z-]*|-F|--form)\s").unwrap();
    if body.is_match(line) {
        "POST".to_string()
    } else {
        "GET".to_string()
    }
}

/// Every endpoint mention in `text` (see the module docs).
fn mentions(text: &str) -> Vec<(usize, Mention)> {
    let authority =
        Regex::new(r"(?:(?:https?|wss?)://[A-Za-z0-9.\-]+(?::\d+)?|\$\{?[A-Za-z_]+\}?|\})$")
            .unwrap();
    let method_before = Regex::new(r"(?:^|[^A-Za-z])(GET|POST|PUT|DELETE|PATCH)\s+$").unwrap();
    let mut out = Vec::new();
    for (line_no, line) in logical_lines(text) {
        for (at, _) in line.match_indices("/api/") {
            let before = &line[..at];
            let url_start = authority.find(before).map(|m| m.start());
            let plain = before
                .chars()
                .last()
                .is_none_or(|c| c.is_whitespace() || "`\"'([=>".contains(c));
            if url_start.is_none() && !plain {
                continue;
            }
            let path = take_path(&line[at..]);
            if path.starts_with("/api/v1/") {
                continue;
            }
            let lead = &before[..url_start.unwrap_or(before.len())];
            let method = match method_before.captures(lead) {
                Some(caps) => Some(caps[1].to_string()),
                None if line.contains("curl ") => Some(curl_method(&line)),
                None => None,
            };
            out.push((line_no, Mention { method, path }));
        }
    }
    out
}

fn is_placeholder(segment: &str) -> bool {
    static UPPER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[A-Z][A-Z0-9_]*$").unwrap());
    static ID: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^(?:[0-9]+|[0-9a-fA-F]{8}-[0-9a-fA-F-]{27})$").unwrap());
    segment.starts_with(['{', '<', '$', ':']) || UPPER.is_match(segment) || ID.is_match(segment)
}

/// True when `path` names `template` (or, for a path ending in `/`, a prefix of it).
fn path_matches(path: &str, template: &str) -> bool {
    let prefix = path.ends_with('/');
    let doc: Vec<&str> = path.trim_end_matches('/').split('/').collect();
    let route: Vec<&str> = template.split('/').collect();
    if doc.len() > route.len() || (!prefix && doc.len() != route.len()) {
        return false;
    }
    doc.iter()
        .zip(&route)
        .all(|(d, r)| d == r || (r.starts_with('{') && r.ends_with('}') && is_placeholder(d)))
}

fn mention_exists(mention: &Mention, routes: &[(String, String)]) -> bool {
    routes.iter().any(|(method, template)| {
        mention.method.as_deref().is_none_or(|m| m == method)
            && path_matches(&mention.path, template)
    })
}

#[test]
fn every_endpoint_the_docs_mention_exists() {
    let routes = api_routes::all_routes();
    let mut missing = Vec::new();
    let mut checked = 0;
    for file in markdown::all_doc_files() {
        let text = std::fs::read_to_string(&file).unwrap();
        let name = file
            .strip_prefix(markdown::repo_path(""))
            .unwrap()
            .display()
            .to_string();
        for (line, mention) in mentions(&text) {
            checked += 1;
            if !mention_exists(&mention, &routes) {
                let method = mention.method.as_deref().unwrap_or("(any method)");
                missing.push(format!("{name}:{line}: {method} {}", mention.path));
            }
        }
    }
    assert!(
        checked > 200,
        "only {checked} mentions found; is the scan broken?"
    );
    assert!(
        missing.is_empty(),
        "documented endpoints that do not exist:\n{}",
        missing.join("\n")
    );
}

#[test]
fn mention_extraction_follows_the_documented_rules() {
    let text = "\
See `GET /api/documents/{id}` and POST https://readur.example.com/api/labels.
Prose naming /api/health, `/api/auth/me` and (/api/settings).
frontend/src/services/api/index.ts and http://loki:3100/loki/api/v1/push are not mentions.
```bash
curl -X DELETE http://localhost:8000/api/labels/$LABEL_ID \\
  -H \"Authorization: Bearer $TOKEN\"
curl -s http://localhost:8000/api/search?query=x
curl http://localhost:8000/api/documents \\
  -F \"file=@a.pdf\"
curl http://localhost:9090/api/v1/targets
reqwest::get(format!(\"{}/api/queue/stats\", base));
```
location /api/ {
";
    let found: Vec<(usize, Mention)> = mentions(text);
    let m = |method: Option<&str>, path: &str| Mention {
        method: method.map(str::to_string),
        path: path.to_string(),
    };
    assert_eq!(
        found,
        [
            (1, m(Some("GET"), "/api/documents/{id}")),
            (1, m(Some("POST"), "/api/labels")),
            (2, m(None, "/api/health")),
            (2, m(None, "/api/auth/me")),
            (2, m(None, "/api/settings")),
            (5, m(Some("DELETE"), "/api/labels/$LABEL_ID")),
            (7, m(Some("GET"), "/api/search")),
            (8, m(Some("POST"), "/api/documents")),
            (11, m(None, "/api/queue/stats")),
            (13, m(None, "/api/")),
        ]
    );
}

#[test]
fn matching_follows_the_documented_rules() {
    let routes = vec![
        ("GET".to_string(), "/api/documents/{id}".to_string()),
        ("POST".to_string(), "/api/documents".to_string()),
    ];
    let exists = |method: Option<&str>, path: &str| {
        mention_exists(
            &Mention {
                method: method.map(str::to_string),
                path: path.to_string(),
            },
            &routes,
        )
    };
    for path in [
        "/api/documents/{document_id}",
        "/api/documents/<id>",
        "/api/documents/$DOC_ID",
        "/api/documents/${DOC_ID}",
        "/api/documents/DOCUMENT_ID",
        "/api/documents/550e8400-e29b-41d4-a716-446655440000",
    ] {
        assert!(exists(Some("GET"), path), "{path}");
    }
    assert!(!exists(Some("GET"), "/api/documents/upload"));
    assert!(!exists(Some("POST"), "/api/documents/upload"));
    assert!(!exists(Some("DELETE"), "/api/documents"));
    assert!(exists(None, "/api/documents"));
    assert!(exists(None, "/api/"));
    assert!(exists(None, "/api/documents/"));
    assert!(!exists(None, "/api/docs/"));
    assert!(!exists(None, "/api/documents/{id}/status"));
}

/// The access level an index row claims.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Access {
    Public,
    User,
    Admin,
    AdminOrSelf,
}

/// `(METHOD, template) -> access` for every index row of docs/api-reference.md.
fn index_rows() -> BTreeMap<(String, String), Access> {
    let doc = markdown::read("docs/api-reference.md");
    let endpoint = Regex::new(r"^(GET|POST|PUT|DELETE|PATCH) (/\S*)$").unwrap();
    let mut rows = BTreeMap::new();
    for line in doc.lines() {
        let Some(cells) = markdown::table_row(line) else {
            continue;
        };
        let Some(caps) = cells
            .first()
            .and_then(|c| markdown::code_span(c))
            .and_then(|s| endpoint.captures(s))
        else {
            continue;
        };
        let access = match cells.get(1).map(String::as_str) {
            Some("public") => Access::Public,
            Some("user") => Access::User,
            Some("admin") => Access::Admin,
            Some("admin or self") => Access::AdminOrSelf,
            _ => continue,
        };
        let key = (caps[1].to_string(), caps[2].to_string());
        assert!(
            rows.insert(key.clone(), access).is_none(),
            "{key:?} is indexed twice"
        );
    }
    rows
}

#[test]
fn the_index_lists_every_route_once() {
    let indexed: BTreeSet<(String, String)> = index_rows().into_keys().collect();
    let routes: BTreeSet<(String, String)> = api_routes::all_routes().into_iter().collect();
    let unlisted: Vec<_> = routes.difference(&indexed).collect();
    let unknown: Vec<_> = indexed.difference(&routes).collect();
    assert!(
        unlisted.is_empty(),
        "routes missing from the api-reference index: {unlisted:#?}"
    );
    assert!(
        unknown.is_empty(),
        "index rows that are not routes: {unknown:#?}"
    );
}

/// The WebSocket upgrade extractor rejects an in-process request (which cannot be upgraded)
/// with 400 before the handler authenticates, so this route is only checked to be routed.
const WEBSOCKET_ROUTE: &str = "/api/sources/{id}/sync/progress/ws";

fn concrete(template: &str, own_id: Uuid) -> String {
    template
        .split('/')
        .map(|segment| match segment {
            "{source_type}" => "webdav".to_string(),
            "{token}" => "not-a-real-token".to_string(),
            "{own_id}" => own_id.to_string(),
            s if s.starts_with('{') => Uuid::new_v4().to_string(),
            s => s.to_string(),
        })
        .collect::<Vec<_>>()
        .join("/")
}

async fn status(ctx: &TestContext, method: &str, uri: &str, token: Option<&str>) -> StatusCode {
    let mut request = Request::builder()
        .method(Method::from_bytes(method.as_bytes()).unwrap())
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let request = request.body(Body::from("{}")).unwrap();
    ctx.app().clone().oneshot(request).await.unwrap().status()
}

#[tokio::test]
async fn index_access_levels_match_the_server() {
    let ctx = TestContext::new().await;
    let auth = ctx.auth_helper();
    let user = auth.create_test_user().await;
    let user_token = auth.login_user(&user.username, &user.password).await;
    let admin = auth.create_admin_user().await;
    let admin_token = auth.login_user(&admin.username, &admin.password).await;
    let own_id = user.user_response.id;

    // Logout revokes the caller's sessions, so it goes last.
    let mut rows: Vec<_> = index_rows().into_iter().collect();
    rows.sort_by_key(|((_, path), _)| path.ends_with("/logout"));
    let mut wrong = Vec::new();
    for ((method, template), access) in rows {
        let uri = concrete(&template, own_id);
        let anonymous = status(&ctx, &method, &uri, None).await;
        if template == WEBSOCKET_ROUTE {
            if anonymous != StatusCode::BAD_REQUEST {
                wrong.push(format!(
                    "{method} {template}: {anonymous} without credentials"
                ));
            }
            continue;
        }
        let as_user = status(&ctx, &method, &uri, Some(&user_token)).await;
        let denied = |s: StatusCode| s == StatusCode::UNAUTHORIZED || s == StatusCode::FORBIDDEN;
        let ok = match access {
            Access::Public => anonymous != StatusCode::UNAUTHORIZED,
            Access::User => anonymous == StatusCode::UNAUTHORIZED && !denied(as_user),
            Access::Admin => {
                let as_admin = status(&ctx, &method, &uri, Some(&admin_token)).await;
                anonymous == StatusCode::UNAUTHORIZED
                    && as_user == StatusCode::FORBIDDEN
                    && !denied(as_admin)
            }
            Access::AdminOrSelf => {
                let own = concrete(&template.replace("{id}", "{own_id}"), own_id);
                let as_self = status(&ctx, &method, &own, Some(&user_token)).await;
                anonymous == StatusCode::UNAUTHORIZED
                    && as_user == StatusCode::FORBIDDEN
                    && !denied(as_self)
            }
        };
        if !ok {
            wrong.push(format!(
                "{method} {template} documented as {access:?}: {anonymous} without credentials, {as_user} as a user"
            ));
        }
    }
    let _ = ctx.cleanup_and_close().await;
    assert!(
        wrong.is_empty(),
        "index access levels that do not match:\n{}",
        wrong.join("\n")
    );
}

/// The anchor a Markdown heading gets: lowercase, without characters other than letters,
/// digits, spaces and hyphens, with spaces turned into hyphens.
fn slug(heading: &str) -> String {
    heading
        .trim_start_matches('#')
        .trim()
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-')
        .map(|c| if c == ' ' { '-' } else { c })
        .collect()
}

/// Links into docs/api-reference.md (`api-reference.md#x` from any page, `#x` within it) must
/// name one of its headings, so moving a section cannot silently break them.
#[test]
fn links_into_the_api_reference_name_real_headings() {
    let doc = markdown::read("docs/api-reference.md");
    let anchors: BTreeSet<String> = doc
        .lines()
        .filter(|l| markdown::heading_level(l) > 0)
        .map(slug)
        .collect();
    assert!(anchors.contains("set-a-documents-labels"));
    let link = Regex::new(r"\]\((?:[./]*api-reference\.md)?#([a-z0-9-]+)\)").unwrap();
    let mut broken = Vec::new();
    for file in markdown::all_doc_files() {
        let text = std::fs::read_to_string(&file).unwrap();
        let same_page = file.ends_with("docs/api-reference.md");
        for caps in link.captures_iter(&text) {
            let whole = &caps[0];
            if (same_page || whole.contains("api-reference.md")) && !anchors.contains(&caps[1]) {
                broken.push(format!("{}: {whole}", file.display()));
            }
        }
    }
    assert!(
        broken.is_empty(),
        "links to missing api-reference headings:\n{}",
        broken.join("\n")
    );
}
