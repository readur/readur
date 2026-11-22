//! The "Search Endpoints" section of docs/api-reference.md, checked against the code: the
//! documented parameters are exactly `SearchRequest`'s (as serde and the OpenAPI document see
//! them), every documented request runs, and every documented response is what the server
//! returns for the documented library, value for value. A docs-wide scan also keeps every
//! `/api/search` URL in docs/ on a real route with real parameters.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use axum::{body::Body, http::Request, http::StatusCode};
use serde_json::{json, Value};
use tower::util::ServiceExt;
use utoipa::OpenApi;
use uuid::Uuid;

#[path = "support/markdown.rs"]
mod markdown;

use readur::models::SearchRequest;
use readur::test_utils::{document_helpers::create_test_document, TestContext};

fn repo_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn api_reference() -> String {
    std::fs::read_to_string(repo_path("docs/api-reference.md")).unwrap()
}

/// The text under `heading` up to the next heading of the same or a higher level.
fn section<'a>(doc: &'a str, heading: &str) -> &'a str {
    let level = heading.chars().take_while(|&c| c == '#').count();
    let start = doc
        .find(&format!("{heading}\n"))
        .unwrap_or_else(|| panic!("no {heading:?} heading"))
        + heading.len()
        + 1;
    let rest = &doc[start..];
    let end = rest
        .lines()
        .scan(0, |offset, line| {
            let at = *offset;
            *offset += line.len() + 1;
            Some((at, line))
        })
        .find(|(_, line)| {
            let hashes = line.chars().take_while(|&c| c == '#').count();
            hashes > 0 && hashes <= level && line[hashes..].starts_with(' ')
        })
        .map_or(rest.len(), |(at, _)| at);
    &rest[..end]
}

/// The contents of the fenced code blocks in `text` tagged `lang`.
fn code_blocks(text: &str, lang: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current: Option<String> = None;
    for line in text.lines() {
        match (&mut current, line.trim_start().strip_prefix("```")) {
            (None, Some(tag)) if tag.trim() == lang => current = Some(String::new()),
            (Some(block), Some(_)) => {
                blocks.push(std::mem::take(block));
                current = None;
            }
            (Some(block), None) => {
                block.push_str(line);
                block.push('\n');
            }
            (None, _) => {}
        }
    }
    blocks
}

/// The parameter names of the "Search Parameters" table, in order.
fn documented_parameters(doc: &str) -> Vec<String> {
    section(doc, "#### Search Parameters")
        .lines()
        .filter_map(|line| line.strip_prefix("| `"))
        .map(|rest| rest.split('`').next().unwrap().to_string())
        .collect()
}

fn search_section(doc: &str) -> &str {
    section(doc, "### Search Endpoints")
}

/// The request line of the `http` block under each `#### <heading>`.
fn documented_request(doc: &str, heading: &str) -> String {
    let blocks = code_blocks(section(search_section(doc), heading), "http");
    let line = blocks
        .first()
        .unwrap_or_else(|| panic!("no http block under {heading}"))
        .trim();
    line.strip_prefix("GET ")
        .unwrap_or_else(|| panic!("{heading}: not a GET: {line}"))
        .to_string()
}

/// The JSON response example under `#### <heading>`.
fn documented_response(doc: &str, heading: &str) -> Value {
    let blocks = code_blocks(section(search_section(doc), heading), "json");
    let block = blocks
        .first()
        .unwrap_or_else(|| panic!("no json block under {heading}"));
    serde_json::from_str(block).unwrap_or_else(|e| panic!("{heading}: invalid JSON: {e}"))
}

fn sorted(names: impl IntoIterator<Item = String>) -> BTreeSet<String> {
    names.into_iter().collect()
}

#[test]
fn documented_parameters_are_the_request_fields() {
    let documented = documented_parameters(&api_reference());
    assert_eq!(
        documented.len(),
        sorted(documented.clone()).len(),
        "a parameter is documented twice: {documented:?}"
    );
    let documented = sorted(documented);

    // Every field serde reads (none is renamed or skipped, so serialising lists them all).
    let fields = serde_json::to_value(SearchRequest::default()).unwrap();
    let fields = sorted(fields.as_object().unwrap().keys().cloned());
    assert_eq!(documented, fields, "documented parameters vs SearchRequest");

    // And every query parameter the OpenAPI document gives the three search routes.
    let api = readur::swagger::ApiDoc::openapi();
    for path in [
        "/api/search",
        "/api/search/enhanced",
        "/api/search/timeline",
    ] {
        let get = api.paths.paths[path].get.as_ref().unwrap();
        let params = sorted(get.parameters.iter().flatten().map(|p| match p {
            utoipa::openapi::RefOr::T(p) => p.name.clone(),
            utoipa::openapi::RefOr::Ref(r) => panic!("{path}: unresolved {}", r.ref_location),
        }));
        assert_eq!(
            documented, params,
            "documented parameters vs OpenAPI {path}"
        );
    }
    let facets = api.paths.paths["/api/search/facets"].get.as_ref().unwrap();
    assert!(facets.parameters.as_ref().is_none_or(Vec::is_empty));
}

/// `(path, query parameter names)` of every `/api/search` URL in `text`.
fn search_urls(text: &str) -> Vec<(String, Vec<String>)> {
    let mut urls = Vec::new();
    for (at, _) in text.match_indices("/api/search") {
        let url: String = text[at..]
            .chars()
            .take_while(|c| !c.is_whitespace() && !"`\"')|".contains(*c))
            .collect();
        let (path, query) = url.split_once('?').unwrap_or((&url, ""));
        let params = query
            .split('&')
            .filter(|p| !p.is_empty())
            .map(|p| p.split('=').next().unwrap().to_string())
            .collect();
        urls.push((path.trim_end_matches(['.', ',', ':']).to_string(), params));
    }
    urls
}

#[test]
fn every_search_url_in_the_docs_is_a_real_route_with_real_parameters() {
    let parameters = sorted(documented_parameters(&api_reference()));
    let routes = [
        "/api/search",
        "/api/search/enhanced",
        "/api/search/timeline",
        "/api/search/facets",
    ];
    let files = markdown::all_doc_files();
    let mut checked = 0;
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        for (path, params) in search_urls(&text) {
            let file = file.display();
            assert!(routes.contains(&path.as_str()), "{file}: no route {path}");
            for param in params {
                assert!(
                    parameters.contains(&param),
                    "{file}: {path} has no parameter {param:?}"
                );
            }
            checked += 1;
        }
    }
    assert!(
        checked > 20,
        "only {checked} search URLs found; is the scan broken?"
    );
}

#[test]
fn url_scan_reads_paths_and_parameter_names() {
    assert_eq!(
        search_urls("see `GET /api/search/enhanced?query=a%20b&tags=x,y` and /api/search."),
        [
            (
                "/api/search/enhanced".to_string(),
                vec!["query".to_string(), "tags".to_string()]
            ),
            ("/api/search".to_string(), vec![]),
        ]
    );
    assert_eq!(
        search_urls("GET /api/search/enhanced?label_ids=<label-id>")[0].1,
        ["label_ids"]
    );
}

/// The documented library: a labelled PDF and two PNGs.
struct Library {
    ctx: TestContext,
    token: String,
}

impl Library {
    async fn new() -> Self {
        let ctx = TestContext::new().await;
        let auth = ctx.auth_helper();
        let user = auth.create_test_user().await;
        let token = auth.login_user(&user.username, &user.password).await;
        let user_id: Uuid = user.user_response.id;
        let pool = &ctx.state().db.pool;
        let (label_id,): (Uuid,) = sqlx::query_as(
            "INSERT INTO labels (user_id, name, color) VALUES ($1, 'Finance', '#0969da') RETURNING id",
        )
        .bind(user_id)
        .fetch_one(pool)
        .await
        .unwrap();

        let documents = [
            (
                "march_invoice.pdf",
                "application/pdf",
                "invoice for march",
                "2024-03-10T09:00:00Z",
                true,
            ),
            (
                "scan.png",
                "image/png",
                "scanned invoice",
                "2024-03-20T09:00:00Z",
                false,
            ),
            (
                "copy.png",
                "image/png",
                "invoice copy",
                "2024-06-01T09:00:00Z",
                false,
            ),
        ];
        for (filename, mime, text, created, finance) in documents {
            let mut doc = create_test_document(user_id);
            doc.filename = filename.to_string();
            doc.original_filename = filename.to_string();
            doc.file_size = 2048;
            doc.mime_type = mime.to_string();
            doc.content = None;
            doc.ocr_text = Some(text.to_string());
            doc.ocr_word_count = Some(3);
            doc.created_at = created.parse().unwrap();
            doc.updated_at = doc.created_at;
            doc.tags = Vec::new();
            doc.file_hash = Some(format!("{:x}", Uuid::new_v4().as_u128()));
            let id = ctx.state().db.create_document(doc).await.unwrap().id;
            if finance {
                sqlx::query("INSERT INTO document_labels (document_id, label_id) VALUES ($1, $2)")
                    .bind(id)
                    .bind(label_id)
                    .execute(pool)
                    .await
                    .unwrap();
            }
        }
        Self { ctx, token }
    }

    async fn get(&self, uri: &str) -> (StatusCode, Value) {
        let response = self
            .ctx
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .header("Authorization", format!("Bearer {}", self.token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    async fn get_ok(&self, uri: &str) -> Value {
        let (status, body) = self.get(uri).await;
        assert_eq!(status, StatusCode::OK, "{uri}: {body}");
        body
    }
}

/// Copies the values that differ from run to run (ids, timestamps the server sets, timing)
/// from `documented` into `real`, after checking they are present there.
fn copy_volatile(real: &mut Value, documented: &Value) {
    fn copy(real: &mut Value, documented: &Value, key: &str) {
        assert!(real.get(key).is_some(), "real response lacks {key}: {real}");
        real[key] = documented[key].clone();
    }
    copy(real, documented, "query_time_ms");
    let doc = &documented["documents"][0];
    let real_doc = &mut real["documents"][0];
    copy(real_doc, doc, "id");
    let label = &doc["labels"][0];
    let real_label = &mut real_doc["labels"][0];
    for key in ["id", "user_id", "created_at", "updated_at"] {
        copy(real_label, label, key);
    }
}

#[tokio::test]
async fn documented_enhanced_search_returns_the_documented_response() {
    let lib = Library::new().await;
    let doc = api_reference();
    let request = documented_request(&doc, "#### Enhanced Search");
    let documented = documented_response(&doc, "#### Enhanced Search");
    let mut real = lib.get_ok(&request).await;
    copy_volatile(&mut real, &documented);
    assert_eq!(real, documented, "{request}");
}

#[tokio::test]
async fn documented_basic_search_returns_the_documented_shape() {
    let lib = Library::new().await;
    let doc = api_reference();
    let request = documented_request(&doc, "#### Search Documents");
    let documented = documented_response(&doc, "#### Enhanced Search");
    let real = lib.get_ok(&request).await;

    let keys = |v: &Value| sorted(v.as_object().unwrap().keys().cloned());
    assert_eq!(keys(&real), keys(&documented));
    assert_eq!(real["total"], 3);
    let docs = real["documents"].as_array().unwrap();
    assert_eq!(docs.len(), 3);
    for found in docs {
        assert_eq!(keys(found), keys(&documented["documents"][0]));
        // As documented: no ranking and no snippets.
        assert_eq!(found["search_rank"], Value::Null);
        assert_eq!(found["snippets"], json!([]));
    }
    assert_eq!(real["suggestions"], json!([]));
    assert_eq!(real["query_time_ms"], 0);
    // Newest first by default.
    assert_eq!(docs[0]["filename"], "copy.png");
}

#[tokio::test]
async fn documented_timeline_and_facets_return_the_documented_responses() {
    let lib = Library::new().await;
    let doc = api_reference();
    for heading in ["#### Search Timeline", "#### Search Facets"] {
        let request = documented_request(&doc, heading);
        assert_eq!(
            lib.get_ok(&request).await,
            documented_response(&doc, heading),
            "{request}"
        );
    }
}

#[tokio::test]
async fn documented_errors_are_returned() {
    let lib = Library::new().await;
    let (status, body) = lib.get("/api/search?query=a").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "SEARCH_QUERY_TOO_SHORT", "{body}");
    assert_eq!(body["status"], 400);
    assert!(body["error"].is_string(), "{body}");
    for uri in [
        "/api/search/enhanced?query=a",
        "/api/search/timeline?query=a",
        "/api/search/enhanced?query=invoice&limit=1001",
        "/api/search/enhanced?query=invoice&snippet_length=2001",
        "/api/search/enhanced?query=invoice&search_mode=regex",
        "/api/search/enhanced?query=invoice&label_ids=nope",
        "/api/search/enhanced?query=invoice&created_from=2025-01-01T00:00:00Z&created_to=2024-01-01T00:00:00Z",
    ] {
        assert_eq!(lib.get(uri).await.0, StatusCode::BAD_REQUEST, "{uri}");
    }
    // The documented bounds themselves are accepted, and a filter alone needs no query.
    for uri in [
        "/api/search/enhanced?query=invoice&limit=1000&snippet_length=2000",
        "/api/search/enhanced?mime_types=image/png",
    ] {
        assert_eq!(lib.get(uri).await.0, StatusCode::OK, "{uri}");
    }
}
