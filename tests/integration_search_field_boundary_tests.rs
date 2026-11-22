//! A document's content and OCR text are searched together, but a phrase
//! must lie within one of them: the end of the content and the start of the
//! OCR text are not adjacent words. Words and the boolean operators over
//! them still apply to the whole document. Covered through REST
//! `GET /api/search/enhanced` (with its total and the timeline, which share
//! the list's condition) and the MCP `enhanced_search` tool.

use axum::{body::Body, http::Request, http::StatusCode};
use serde_json::{json, Value};
use tower::util::ServiceExt;
use uuid::Uuid;

use readur::test_utils::{document_helpers::create_test_document, TestContext};

/// A user, a token, and documents seeded for them.
struct Library {
    ctx: TestContext,
    token: String,
    user_id: Uuid,
}

impl Library {
    async fn new() -> Self {
        let ctx = TestContext::new().await;
        let auth = ctx.auth_helper();
        let user = auth.create_test_user().await;
        let token = auth.login_user(&user.username, &user.password).await;
        Self {
            ctx,
            token,
            user_id: user.user_response.id,
        }
    }

    /// Seeds a document with this content and OCR text; returns its id.
    async fn seed(&self, content: Option<&str>, ocr_text: Option<&str>) -> String {
        let mut doc = create_test_document(self.user_id);
        doc.content = content.map(String::from);
        doc.ocr_text = ocr_text.map(String::from);
        doc.tags = Vec::new();
        doc.file_hash = Some(format!("{:x}", Uuid::new_v4().as_u128()));
        let doc = self.ctx.state().db.create_document(doc).await.unwrap();
        doc.id.to_string()
    }

    async fn get(&self, uri: &str) -> (StatusCode, Value) {
        let request = Request::builder()
            .uri(uri)
            .header("Authorization", format!("Bearer {}", self.token))
            .body(Body::empty())
            .unwrap();
        let response = self.ctx.app().clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    /// REST enhanced search: the hits, after checking the total and the
    /// timeline agree with them.
    async fn rest_ids(&self, query: &str, mode: &str) -> Vec<String> {
        let params = format!(
            "query={}&search_mode={mode}&limit=100",
            urlencoding::encode(query)
        );
        let (status, body) = self.get(&format!("/api/search/enhanced?{params}")).await;
        assert_eq!(status, StatusCode::OK, "{mode} {query:?}: {body}");
        let hits = ids(&body["documents"]);
        assert_eq!(body["total"], json!(hits.len()), "{mode} {query:?}: {body}");

        let (status, months) = self.get(&format!("/api/search/timeline?{params}")).await;
        assert_eq!(status, StatusCode::OK, "{mode} {query:?}: {months}");
        let timeline: i64 = months
            .as_array()
            .unwrap()
            .iter()
            .map(|month| month["count"].as_i64().unwrap())
            .sum();
        assert_eq!(timeline, hits.len() as i64, "{mode} {query:?}: {months}");
        hits
    }

    /// MCP `enhanced_search`: the structured result.
    async fn mcp(&self, query: &str, mode: &str) -> Value {
        let result = self
            .ctx
            .mcp_call_tool(
                &self.token,
                "enhanced_search",
                json!({ "query": query, "search_mode": mode, "limit": 100 }),
            )
            .await;
        assert_eq!(
            result["isError"],
            json!(false),
            "{mode} {query:?}: {result}"
        );
        result["structuredContent"].clone()
    }

    /// The hits of `query` in `mode`, the same through REST and MCP.
    async fn hits(&self, query: &str, mode: &str) -> Vec<String> {
        let mut rest = self.rest_ids(query, mode).await;
        let mut mcp = ids(&self.mcp(query, mode).await["documents"]);
        rest.sort();
        mcp.sort();
        assert_eq!(rest, mcp, "{mode} {query:?}: REST and MCP differ");
        rest
    }
}

fn ids(documents: &Value) -> Vec<String> {
    documents
        .as_array()
        .unwrap_or_else(|| panic!("no documents array: {documents}"))
        .iter()
        .map(|doc| doc["id"].as_str().unwrap().to_string())
        .collect()
}

/// Documents with "total" and "due" adjacent in one field, or only across
/// the join of the two fields.
struct Corpus {
    across: String,
    in_content: String,
    in_ocr: String,
    in_ocr_draft_in_content: String,
}

async fn corpus(lib: &Library) -> Corpus {
    Corpus {
        // Content ends with "total", OCR text starts with "due".
        across: lib
            .seed(Some("statement grand total"), Some("due 2024 payable"))
            .await,
        in_content: lib
            .seed(Some("the total due is 500"), Some("unrelated scan"))
            .await,
        in_ocr: lib.seed(None, Some("amount total due 300")).await,
        in_ocr_draft_in_content: lib
            .seed(Some("draft notes"), Some("final total due 100"))
            .await,
    }
}

fn assert_hits(query: &str, hits: &[String], expected: &[&String], excluded: &[&String]) {
    for id in expected {
        assert!(hits.contains(id), "{query:?} missed {id}: {hits:?}");
    }
    for id in excluded {
        assert!(!hits.contains(id), "{query:?} matched {id}: {hits:?}");
    }
}

#[tokio::test]
async fn phrases_never_match_across_content_and_ocr_text() {
    let lib = Library::new().await;
    let c = corpus(&lib).await;
    let within = [&c.in_content, &c.in_ocr, &c.in_ocr_draft_in_content];
    for (query, mode) in [
        ("total due", "phrase"),
        ("\"total due\"", "phrase"),
        ("total <-> due", "boolean"),
        ("\"total due\"", "boolean"),
        ("total <1> due", "boolean"),
    ] {
        let hits = lib.hits(query, mode).await;
        assert_hits(query, &hits, &within, &[&c.across]);
    }
}

#[tokio::test]
async fn words_still_match_across_the_two_fields() {
    let lib = Library::new().await;
    let c = corpus(&lib).await;
    // Simple mode and plain AND need no adjacency, so the split document
    // matches like the others.
    let all = [
        &c.across,
        &c.in_content,
        &c.in_ocr,
        &c.in_ocr_draft_in_content,
    ];
    for (query, mode) in [("total due", "simple"), ("total & due", "boolean")] {
        let hits = lib.hits(query, mode).await;
        assert_hits(query, &hits, &all, &[]);
    }
    // A distance spanning the join is no more allowed than adjacency.
    let hits = lib.hits("grand <2> due", "boolean").await;
    assert!(hits.is_empty(), "{hits:?}");
}

#[tokio::test]
async fn negation_still_applies_to_the_whole_document() {
    let lib = Library::new().await;
    let c = corpus(&lib).await;
    let draft_in_ocr = lib
        .seed(Some("invoice total due 50"), Some("draft copy"))
        .await;

    let hits = lib.hits("\"total due\" -draft", "boolean").await;
    assert_hits(
        "\"total due\" -draft",
        &hits,
        &[&c.in_content, &c.in_ocr],
        &[&c.across, &c.in_ocr_draft_in_content, &draft_in_ocr],
    );
    let hits = lib.hits("total NOT draft", "boolean").await;
    assert_hits(
        "total NOT draft",
        &hits,
        &[&c.across, &c.in_content, &c.in_ocr],
        &[&c.in_ocr_draft_in_content, &draft_in_ocr],
    );
    // A negated phrase excludes only documents holding it within a field.
    let hits = lib.hits("total -\"total due\"", "boolean").await;
    assert_hits(
        "total -\"total due\"",
        &hits,
        &[&c.across],
        &[&c.in_content, &c.in_ocr, &draft_in_ocr],
    );
}

#[tokio::test]
async fn stop_words_are_ignored_as_to_tsquery_ignores_them() {
    let lib = Library::new().await;
    let report = lib.seed(None, Some("report on regional sales")).await;
    let other = lib.seed(None, Some("weather report")).await;

    let hits = lib.hits("report of sales", "boolean").await;
    assert_hits("report of sales", &hits, &[&report], &[&other]);
    let hits = lib.hits("report -the", "boolean").await;
    assert_hits("report -the", &hits, &[&report, &other], &[]);
    let hits = lib.hits("sales | the", "boolean").await;
    assert_hits("sales | the", &hits, &[&report], &[&other]);
    // Nothing but stop words matches nothing, negated or not.
    for query in ["the", "-the", "NOT (the | a)", "\"of the\""] {
        assert!(lib.hits(query, "boolean").await.is_empty(), "{query:?}");
    }
}

#[tokio::test]
async fn words_postgres_splits_match_within_one_field() {
    let lib = Library::new().await;
    let joined = lib.seed(None, Some("send an e-mail today")).await;
    let split = lib.seed(Some("letter e"), Some("mail room")).await;
    let hits = lib.hits("e-mail", "boolean").await;
    assert_hits("e-mail", &hits, &[&joined], &[&split]);
}

/// The highlighted text of every snippet of every result.
fn highlights(structured: &Value) -> Vec<String> {
    let mut out = Vec::new();
    for doc in structured["documents"].as_array().unwrap() {
        for snippet in doc["snippets"].as_array().unwrap() {
            let text = snippet["text"].as_str().unwrap();
            for range in snippet["highlight_ranges"].as_array().unwrap() {
                let start = range["start"].as_u64().unwrap() as usize;
                let end = range["end"].as_u64().unwrap() as usize;
                out.push(text[start..end].to_string());
            }
        }
    }
    out
}

#[tokio::test]
async fn phrase_mode_highlights_the_phrase_as_one_range() {
    let lib = Library::new().await;
    lib.seed(
        None,
        Some("Statement: the TOTAL\n   due is 500; total alone here"),
    )
    .await;

    let structured = lib.mcp("\"total   due\"", "phrase").await;
    assert_eq!(highlights(&structured), ["TOTAL\n   due"], "{structured}");

    let (status, body) = lib
        .get("/api/search/enhanced?query=total%20due&search_mode=phrase")
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(highlights(&body), ["TOTAL\n   due"], "{body}");
}

#[tokio::test]
async fn phrase_mode_falls_back_to_word_highlights_after_stemming() {
    let lib = Library::new().await;
    // phraseto_tsquery matches "invoices due" against "invoice due" by stem,
    // so the phrase as typed occurs nowhere; its words still get snippets.
    lib.seed(None, Some("the invoice due today")).await;
    let structured = lib.mcp("invoices due", "phrase").await;
    assert_eq!(structured["total"], json!(1), "{structured}");
    assert_eq!(highlights(&structured), ["due"], "{structured}");
}
