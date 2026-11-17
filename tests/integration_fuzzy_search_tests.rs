//! Fuzzy search matches word by word (pg_trgm `word_similarity` of the query against the closest
//! stretch of a document's content + OCR text), ranks by that score, and takes its cutoff from
//! the user's `fuzzy_search_threshold` setting. Covered through REST `GET /api/search/enhanced`,
//! the MCP `enhanced_search` tool and `GET /api/search/timeline`, which all share one query
//! builder.

use std::collections::{BTreeSet, HashMap};

use axum::{body::Body, http::Request, http::StatusCode};
use serde_json::{json, Value};
use tower::util::ServiceExt;
use uuid::Uuid;

use readur::models::FuzzyThreshold;
use readur::test_utils::{document_helpers::create_test_document, TestContext};

/// Everyday words, none spelled like "invoice", repeated into long documents.
const FILLER: &str = "the quarterly report describes weather patterns across northern regions \
    including rainfall temperature humidity wind speed and seasonal variation measured by local \
    stations over several decades while researchers compared historical records with satellite \
    observations to identify long term trends affecting agriculture forestry fisheries tourism \
    transport energy demand public health infrastructure planning coastal erosion river flooding \
    drought resilience community preparedness emergency response budgets staffing training \
    equipment maintenance schedules";

/// About `words` words of `FILLER`, with `insert` (if any) placed in the middle.
fn long_text(words: usize, insert: Option<&str>) -> String {
    let filler: Vec<&str> = FILLER.split_whitespace().collect();
    let mut text: Vec<&str> = filler.iter().cycle().take(words).copied().collect();
    if let Some(insert) = insert {
        text.insert(words / 2, insert);
    }
    text.join(" ")
}

/// A user with a token and a small library, keyed by name.
struct Library {
    ctx: TestContext,
    token: String,
    user_id: Uuid,
    key_by_id: HashMap<String, &'static str>,
}

impl Library {
    async fn new(documents: &[(&'static str, String)]) -> Self {
        let ctx = TestContext::new().await;
        let auth = ctx.auth_helper();
        let user = auth.create_test_user().await;
        let token = auth.login_user(&user.username, &user.password).await;
        let user_id = user.user_response.id;
        let mut key_by_id = HashMap::new();
        for (key, text) in documents {
            let mut doc = create_test_document(user_id);
            doc.content = None;
            doc.ocr_text = Some(text.clone());
            doc.tags = Vec::new();
            doc.file_hash = Some(format!("{:x}", Uuid::new_v4().as_u128()));
            let id = ctx.state().db.create_document(doc).await.unwrap().id;
            key_by_id.insert(id.to_string(), *key);
        }
        Self {
            ctx,
            token,
            user_id,
            key_by_id,
        }
    }

    async fn request(&self, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
        let mut builder = Request::builder()
            .method(method)
            .uri(uri)
            .header("Authorization", format!("Bearer {}", self.token));
        let body = match body {
            Some(body) => {
                builder = builder.header("content-type", "application/json");
                Body::from(body.to_string())
            }
            None => Body::empty(),
        };
        let response = self
            .ctx
            .app
            .clone()
            .oneshot(builder.body(body).unwrap())
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

    async fn set_threshold(&self, threshold: f32) {
        let (status, body) = self
            .request(
                "PUT",
                "/api/settings",
                Some(json!({ "fuzzy_search_threshold": threshold })),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "saving the threshold: {body}");
    }

    fn keys(&self, documents: &Value) -> Vec<&'static str> {
        documents
            .as_array()
            .unwrap_or_else(|| panic!("no documents array: {documents}"))
            .iter()
            .map(|doc| self.key_by_id[doc["id"].as_str().unwrap()])
            .collect()
    }

    /// REST fuzzy search: the documents found, in rank order, after checking that the total
    /// counts exactly them.
    async fn rest_fuzzy(&self, query: &str) -> Vec<&'static str> {
        let uri = format!("/api/search/enhanced?query={query}&search_mode=fuzzy&limit=100");
        let (status, body) = self.request("GET", &uri, None).await;
        assert_eq!(status, StatusCode::OK, "{uri}: {body}");
        let found = self.keys(&body["documents"]);
        assert_eq!(
            body["total"],
            json!(found.len()),
            "{uri}: count and list differ"
        );
        found
    }

    /// MCP `enhanced_search` in fuzzy mode, checked the same way.
    async fn mcp_fuzzy(&self, query: &str) -> Vec<&'static str> {
        let result = self
            .ctx
            .mcp_call_tool(
                &self.token,
                "enhanced_search",
                json!({ "query": query, "search_mode": "fuzzy", "limit": 100 }),
            )
            .await;
        assert_eq!(result["isError"], json!(false), "{query:?}: {result}");
        let content = &result["structuredContent"];
        let found = self.keys(&content["documents"]);
        assert_eq!(
            content["total"],
            json!(found.len()),
            "{query:?}: count and list differ"
        );
        found
    }

    /// Total matches the fuzzy timeline reports, over all months.
    async fn timeline_total(&self, query: &str) -> i64 {
        let uri = format!("/api/search/timeline?query={query}&search_mode=fuzzy");
        let (status, body) = self.request("GET", &uri, None).await;
        assert_eq!(status, StatusCode::OK, "{uri}: {body}");
        body.as_array()
            .unwrap()
            .iter()
            .map(|month| month["count"].as_i64().unwrap())
            .sum()
    }
}

fn set(keys: &[&'static str]) -> BTreeSet<&'static str> {
    keys.iter().copied().collect()
}

/// The library most tests use: a 5000-word document with "invoice" in the middle, one without
/// it, and short documents spelled like the query to different degrees.
async fn invoice_library() -> Library {
    Library::new(&[
        ("long_invoice", long_text(5000, Some("invoice"))),
        ("long_unrelated", long_text(5000, None)),
        ("short_invoice", "the invoice total".to_string()),
        ("short_typo", "the invoce total".to_string()),
    ])
    .await
}

#[tokio::test]
async fn misspelled_word_finds_long_document_containing_it() {
    let lib = invoice_library().await;
    for found in [
        lib.rest_fuzzy("invoce").await,
        lib.mcp_fuzzy("invoce").await,
    ] {
        let found = set(&found);
        assert!(found.contains("long_invoice"), "{found:?}");
        assert!(!found.contains("long_unrelated"), "{found:?}");
        assert_eq!(found, set(&["long_invoice", "short_invoice", "short_typo"]));
    }
}

#[tokio::test]
async fn closer_spelling_ranks_first_and_rank_is_the_word_similarity() {
    let lib = invoice_library().await;
    let rest = lib.rest_fuzzy("invoce").await;
    let mcp = lib.mcp_fuzzy("invoce").await;
    // The exact spelling scores 1.0; "invoice" scores the same in a short or a long document,
    // because only the closest stretch of the text counts.
    assert_eq!(rest[0], "short_typo", "{rest:?}");
    assert_eq!(mcp[0], "short_typo", "{mcp:?}");

    let (_, body) = lib
        .request(
            "GET",
            "/api/search/enhanced?query=invoce&search_mode=fuzzy",
            None,
        )
        .await;
    let ranks: HashMap<&str, f64> = body["documents"]
        .as_array()
        .unwrap()
        .iter()
        .map(|doc| {
            (
                lib.key_by_id[doc["id"].as_str().unwrap()],
                doc["search_rank"].as_f64().unwrap(),
            )
        })
        .collect();
    assert!((ranks["short_typo"] - 1.0).abs() < 1e-6, "{ranks:?}");
    assert!(ranks["short_invoice"] < 1.0, "{ranks:?}");
    assert!(
        (ranks["short_invoice"] - ranks["long_invoice"]).abs() < 1e-6,
        "{ranks:?}"
    );
}

#[tokio::test]
async fn user_threshold_decides_what_matches() {
    let lib = invoice_library().await;

    // Strict: only the query's own spelling.
    lib.set_threshold(0.9).await;
    assert_eq!(set(&lib.rest_fuzzy("invoce").await), set(&["short_typo"]));
    assert_eq!(set(&lib.mcp_fuzzy("invoce").await), set(&["short_typo"]));
    assert_eq!(lib.timeline_total("invoce").await, 1);

    // Lenient: chance letter overlap in the unrelated long document is now enough.
    lib.set_threshold(0.25).await;
    let all = set(&[
        "long_invoice",
        "long_unrelated",
        "short_invoice",
        "short_typo",
    ]);
    assert_eq!(set(&lib.rest_fuzzy("invoce").await), all);
    assert_eq!(set(&lib.mcp_fuzzy("invoce").await), all);
    assert_eq!(lib.timeline_total("invoce").await, 4);

    // 1.0 still accepts an exact spelling (the cutoff is inclusive).
    lib.set_threshold(1.0).await;
    assert_eq!(set(&lib.rest_fuzzy("invoce").await), set(&["short_typo"]));
}

#[tokio::test]
async fn default_threshold_applies_without_settings() {
    let lib = invoice_library().await;
    let stored = lib
        .ctx
        .state()
        .db
        .get_fuzzy_search_threshold(lib.user_id)
        .await
        .unwrap();
    assert_eq!(stored, None, "a new user has no settings row");
    assert_eq!(
        set(&lib.rest_fuzzy("invoce").await),
        set(&["long_invoice", "short_invoice", "short_typo"])
    );
    assert_eq!(lib.timeline_total("invoce").await, 3);
    // The settings API reports the same default the search applies.
    let (_, settings) = lib.request("GET", "/api/settings", None).await;
    let reported = settings["fuzzy_search_threshold"].as_f64().unwrap();
    assert!(
        (reported - f64::from(FuzzyThreshold::DEFAULT)).abs() < 1e-6,
        "{settings}"
    );
}

#[tokio::test]
async fn out_of_range_threshold_is_rejected_when_saved() {
    let lib = invoice_library().await;
    for bad in [-0.1, 1.5] {
        let (status, body) = lib
            .request(
                "PUT",
                "/api/settings",
                Some(json!({ "fuzzy_search_threshold": bad })),
            )
            .await;
        assert!(status.is_client_error(), "{bad}: {status} {body}");
    }
}

#[tokio::test]
async fn threshold_does_not_leak_into_other_modes_or_users() {
    let lib = invoice_library().await;
    lib.set_threshold(0.9).await;
    // Simple mode is unaffected by the fuzzy setting.
    let (status, body) = lib
        .request("GET", "/api/search/enhanced?query=invoice&limit=100", None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        set(&lib.keys(&body["documents"])),
        set(&["long_invoice", "short_invoice"])
    );

    // Another user of the same server keeps the default cutoff.
    let auth = lib.ctx.auth_helper();
    let other = auth.create_test_user().await;
    let other_token = auth.login_user(&other.username, &other.password).await;
    let mut doc = create_test_document(other.user_response.id);
    doc.content = None;
    doc.ocr_text = Some("the invoice total".to_string());
    doc.file_hash = Some(format!("{:x}", Uuid::new_v4().as_u128()));
    lib.ctx.state().db.create_document(doc).await.unwrap();
    let result = lib
        .ctx
        .mcp_call_tool(
            &other_token,
            "enhanced_search",
            json!({ "query": "invoce", "search_mode": "fuzzy" }),
        )
        .await;
    assert_eq!(result["structuredContent"]["total"], json!(1), "{result}");
}

/// The highlighted words of each found document, by key: every highlight range of every snippet,
/// as text. Ranges are byte offsets into the snippet text.
fn highlights(lib: &Library, documents: &Value) -> HashMap<&'static str, Vec<String>> {
    documents
        .as_array()
        .unwrap_or_else(|| panic!("no documents array: {documents}"))
        .iter()
        .map(|doc| {
            let words = doc["snippets"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|snippet| {
                    let text = snippet["text"].as_str().unwrap().to_string();
                    snippet["highlight_ranges"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(move |range| {
                            let start = range["start"].as_u64().unwrap() as usize;
                            let end = range["end"].as_u64().unwrap() as usize;
                            text[start..end].to_string()
                        })
                        .collect::<Vec<_>>()
                })
                .collect();
            (lib.key_by_id[doc["id"].as_str().unwrap()], words)
        })
        .collect()
}

impl Library {
    /// Highlights of a REST enhanced search and of the MCP `enhanced_search` tool for the same
    /// query and mode, after checking they agree.
    async fn rest_and_mcp_highlights(
        &self,
        query: &str,
        mode: &str,
    ) -> HashMap<&'static str, Vec<String>> {
        let uri = format!(
            "/api/search/enhanced?query={}&search_mode={mode}&limit=100",
            query.replace(' ', "%20")
        );
        let (status, body) = self.request("GET", &uri, None).await;
        assert_eq!(status, StatusCode::OK, "{uri}: {body}");
        let rest = highlights(self, &body["documents"]);
        let result = self
            .ctx
            .mcp_call_tool(
                &self.token,
                "enhanced_search",
                json!({ "query": query, "search_mode": mode, "limit": 100 }),
            )
            .await;
        assert_eq!(result["isError"], json!(false), "{query:?}: {result}");
        let mcp = highlights(self, &result["structuredContent"]["documents"]);
        assert_eq!(
            rest, mcp,
            "{query:?} ({mode}): REST and MCP highlight differently"
        );
        rest
    }
}

fn words(list: &[&str]) -> Vec<String> {
    list.iter().map(|w| w.to_string()).collect()
}

#[tokio::test]
async fn fuzzy_snippets_highlight_the_words_that_matched() {
    let lib = invoice_library().await;
    // Every document a misspelt query finds comes back with a snippet showing why: the word the
    // trigram cutoff matched, which is spelled differently from the query.
    let found = lib.rest_and_mcp_highlights("invoce", "fuzzy").await;
    assert_eq!(found.len(), 3, "{found:?}");
    assert_eq!(found["long_invoice"], words(&["invoice"]));
    assert_eq!(found["short_invoice"], words(&["invoice"]));
    assert_eq!(found["short_typo"], words(&["invoce"]));

    // A multi-word query highlights the near-misses of each of its words.
    let found = lib.rest_and_mcp_highlights("invoce totl", "fuzzy").await;
    assert_eq!(found["short_invoice"], words(&["invoice", "total"]));
}

#[tokio::test]
async fn fuzzy_snippets_use_the_users_threshold() {
    let lib = invoice_library().await;
    // word_similarity('invoce', 'invoice') is 0.571: below a 0.6 cutoff "invoice" is neither a
    // match nor highlighted, so the remaining hit highlights only the exact spelling.
    lib.set_threshold(0.6).await;
    let found = lib.rest_and_mcp_highlights("invoce", "fuzzy").await;
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found["short_typo"], words(&["invoce"]));

    // At 0.57 the same word both matches and is highlighted.
    lib.set_threshold(0.57).await;
    let found = lib.rest_and_mcp_highlights("invoce", "fuzzy").await;
    assert_eq!(found["short_invoice"], words(&["invoice"]));
}

#[tokio::test]
async fn non_fuzzy_snippets_highlight_the_query_as_typed() {
    let lib = invoice_library().await;
    for mode in ["simple", "phrase", "boolean"] {
        let found = lib.rest_and_mcp_highlights("invoice", mode).await;
        assert_eq!(found.len(), 2, "{mode}: {found:?}");
        assert_eq!(found["long_invoice"], words(&["invoice"]), "{mode}");
        assert_eq!(found["short_invoice"], words(&["invoice"]), "{mode}");
    }
    // Simple mode highlights substrings as typed, not near-misses, even with a fuzzy setting.
    lib.set_threshold(0.1).await;
    let found = lib.rest_and_mcp_highlights("total", "simple").await;
    assert_eq!(found["short_typo"], words(&["total"]));
    assert_eq!(found["short_invoice"], words(&["total"]));
}

/// The fuzzy condition can be answered from `idx_documents_content_trgm`: the index exists over
/// exactly the expression the search uses, and the planner picks it for `<%`.
#[tokio::test]
async fn fuzzy_condition_is_served_by_the_trigram_index() {
    let lib = invoice_library().await;
    let pool = &lib.ctx.state().db.pool;
    let indexdef: String = sqlx::query_scalar(
        "SELECT indexdef FROM pg_indexes WHERE indexname = 'idx_documents_content_trgm'",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    assert!(indexdef.contains("gin_trgm_ops"), "{indexdef}");
    assert!(
        indexdef
            .contains("COALESCE(content, ''::text) || ' '::text) || COALESCE(ocr_text, ''::text)"),
        "{indexdef}"
    );

    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SET LOCAL enable_seqscan = off")
        .execute(&mut *tx)
        .await
        .unwrap();
    let plan: Vec<String> = sqlx::query_scalar(
        "EXPLAIN SELECT id FROM documents \
         WHERE $1 <% (COALESCE(content, '') || ' ' || COALESCE(ocr_text, ''))",
    )
    .bind("invoce")
    .fetch_all(&mut *tx)
    .await
    .unwrap();
    let plan = plan.join("\n");
    assert!(plan.contains("idx_documents_content_trgm"), "{plan}");
}
