//! MCP search tools: `search_documents`, `enhanced_search` and
//! `get_search_facets`.

use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

use readur::models::Document;
use readur::test_utils::{
    document_helpers::create_test_document, TestContext, MCP_TEST_PROTOCOL_VERSION,
};

/// A user and a session token for them.
async fn user_with_token(ctx: &TestContext) -> (Uuid, String) {
    let auth = ctx.auth_helper();
    let user = auth.create_test_user().await;
    let token = auth.login_user(&user.username, &user.password).await;
    (user.user_response.id, token)
}

/// Inserts a completed document whose OCR text contains `ocr_text`.
async fn seed_document(
    ctx: &TestContext,
    user_id: Uuid,
    filename: &str,
    mime_type: &str,
    ocr_text: &str,
) -> Document {
    let mut doc = create_test_document(user_id);
    doc.filename = filename.to_string();
    doc.original_filename = filename.to_string();
    doc.mime_type = mime_type.to_string();
    doc.content = None;
    doc.ocr_text = Some(ocr_text.to_string());
    doc.tags = Vec::new();
    doc.file_hash = Some(format!("{:x}", Uuid::new_v4().as_u128()));
    ctx.state().db.create_document(doc).await.unwrap()
}

fn text(result: &Value) -> &str {
    result["content"][0]["text"].as_str().unwrap()
}

fn hit_ids(result: &Value) -> Vec<String> {
    result["structuredContent"]["documents"]
        .as_array()
        .expect("structuredContent has no documents array")
        .iter()
        .map(|doc| doc["id"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn search_documents_finds_seeded_document_by_ocr_text() {
    let ctx = TestContext::new().await;
    let (user_id, token) = user_with_token(&ctx).await;
    let doc = seed_document(
        &ctx,
        user_id,
        "invoice-march.pdf",
        "application/pdf",
        "Quarterly zephyrquartz invoice for consulting services",
    )
    .await;
    seed_document(
        &ctx,
        user_id,
        "other.pdf",
        "application/pdf",
        "Nothing relevant here",
    )
    .await;

    let result = ctx
        .mcp_call_tool(
            &token,
            "search_documents",
            json!({ "query": "zephyrquartz" }),
        )
        .await;

    assert_eq!(result["isError"], json!(false), "{result}");
    assert_eq!(hit_ids(&result), vec![doc.id.to_string()]);
    assert_eq!(result["structuredContent"]["total"], json!(1));
    let text = text(&result);
    assert!(text.contains("invoice-march.pdf"), "{text}");
    assert!(text.contains(&doc.id.to_string()), "{text}");
}

#[tokio::test]
async fn search_documents_filters_by_mime_type() {
    let ctx = TestContext::new().await;
    let (user_id, token) = user_with_token(&ctx).await;
    let pdf = seed_document(
        &ctx,
        user_id,
        "a.pdf",
        "application/pdf",
        "zephyrquartz pdf",
    )
    .await;
    seed_document(&ctx, user_id, "b.txt", "text/plain", "zephyrquartz text").await;

    let result = ctx
        .mcp_call_tool(
            &token,
            "search_documents",
            json!({ "query": "zephyrquartz", "mime_types": ["application/pdf"] }),
        )
        .await;

    assert_eq!(result["isError"], json!(false), "{result}");
    assert_eq!(hit_ids(&result), vec![pdf.id.to_string()]);
}

#[tokio::test]
async fn search_does_not_return_other_users_documents() {
    let ctx = TestContext::new().await;
    let (owner_id, _) = user_with_token(&ctx).await;
    let (_, other_token) = user_with_token(&ctx).await;
    seed_document(
        &ctx,
        owner_id,
        "secret.pdf",
        "application/pdf",
        "zephyrquartz secret",
    )
    .await;

    for tool in ["search_documents", "enhanced_search"] {
        let result = ctx
            .mcp_call_tool(&other_token, tool, json!({ "query": "zephyrquartz" }))
            .await;
        assert_eq!(result["isError"], json!(false), "{tool}: {result}");
        assert!(hit_ids(&result).is_empty(), "{tool} leaked: {result}");
        assert_eq!(result["structuredContent"]["total"], json!(0), "{tool}");
    }
}

#[tokio::test]
async fn enhanced_search_returns_snippets() {
    let ctx = TestContext::new().await;
    let (user_id, token) = user_with_token(&ctx).await;
    let doc = seed_document(
        &ctx,
        user_id,
        "report.txt",
        "text/plain",
        "The annual report mentions zephyrquartz revenue growing steadily.",
    )
    .await;

    let result = ctx
        .mcp_call_tool(
            &token,
            "enhanced_search",
            json!({ "query": "zephyrquartz" }),
        )
        .await;

    assert_eq!(result["isError"], json!(false), "{result}");
    assert_eq!(hit_ids(&result), vec![doc.id.to_string()]);
    let snippets = result["structuredContent"]["documents"][0]["snippets"]
        .as_array()
        .unwrap();
    assert!(!snippets.is_empty(), "no snippets: {result}");
    assert!(snippets[0]["text"]
        .as_str()
        .unwrap()
        .contains("zephyrquartz"));
    let text = text(&result);
    assert!(text.contains("zephyrquartz revenue"), "{text}");

    let without = ctx
        .mcp_call_tool(
            &token,
            "enhanced_search",
            json!({ "query": "zephyrquartz", "include_snippets": false }),
        )
        .await;
    assert_eq!(
        without["structuredContent"]["documents"][0]["snippets"],
        json!([])
    );
}

#[tokio::test]
async fn enhanced_search_supports_search_modes() {
    let ctx = TestContext::new().await;
    let (user_id, token) = user_with_token(&ctx).await;
    let doc = seed_document(
        &ctx,
        user_id,
        "a.txt",
        "text/plain",
        "alpha zephyrquartz beta",
    )
    .await;

    let result = ctx
        .mcp_call_tool(
            &token,
            "enhanced_search",
            json!({ "query": "alpha zephyrquartz", "search_mode": "phrase" }),
        )
        .await;
    assert_eq!(result["isError"], json!(false), "{result}");
    assert_eq!(hit_ids(&result), vec![doc.id.to_string()]);
}

#[tokio::test]
async fn get_search_facets_counts_mime_types() {
    let ctx = TestContext::new().await;
    let (user_id, token) = user_with_token(&ctx).await;
    let (other_id, _) = user_with_token(&ctx).await;
    seed_document(&ctx, user_id, "a.pdf", "application/pdf", "one").await;
    seed_document(&ctx, user_id, "b.pdf", "application/pdf", "two").await;
    seed_document(&ctx, user_id, "c.txt", "text/plain", "three").await;
    seed_document(&ctx, other_id, "d.png", "image/png", "four").await;

    let result = ctx
        .mcp_call_tool(&token, "get_search_facets", json!({}))
        .await;

    assert_eq!(result["isError"], json!(false), "{result}");
    let mime_types = &result["structuredContent"]["mime_types"];
    assert_eq!(
        mime_types,
        &json!([
            { "value": "application/pdf", "count": 2 },
            { "value": "text/plain", "count": 1 }
        ])
    );
    let text = text(&result);
    assert!(text.contains("application/pdf (2 documents)"), "{text}");
    assert!(!text.contains("image/png"), "{text}");
}

#[tokio::test]
async fn empty_query_is_a_tool_error() {
    let ctx = TestContext::new().await;
    let (_, token) = user_with_token(&ctx).await;

    for tool in ["search_documents", "enhanced_search"] {
        let result = ctx
            .mcp_call_tool(&token, tool, json!({ "query": "" }))
            .await;
        assert_eq!(result["isError"], json!(true), "{tool}: {result}");
        assert!(
            text(&result).contains("at least 2 characters"),
            "{tool}: {result}"
        );
    }
}

#[tokio::test]
async fn invalid_ocr_status_filter_is_a_tool_error() {
    let ctx = TestContext::new().await;
    let (_, token) = user_with_token(&ctx).await;

    let result = ctx
        .mcp_call_tool(
            &token,
            "search_documents",
            json!({ "query": "zephyrquartz", "ocr_status": "bogus" }),
        )
        .await;
    assert_eq!(result["isError"], json!(true), "{result}");
    assert!(text(&result).contains("ocr_status"), "{result}");
}

#[tokio::test]
async fn non_uuid_label_ids_are_rejected() {
    let ctx = TestContext::new().await;
    let (_, token) = user_with_token(&ctx).await;

    let message = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "search_documents",
            "arguments": { "query": "zephyrquartz", "label_ids": ["not-a-uuid"] },
            "_meta": {
                "io.modelcontextprotocol/protocolVersion": MCP_TEST_PROTOCOL_VERSION,
                "io.modelcontextprotocol/clientInfo": { "name": "readur-tests", "version": "1.0" },
                "io.modelcontextprotocol/clientCapabilities": {}
            }
        }
    });
    let (_, body) = ctx.mcp_post(Some(&token), message).await;
    // rmcp reports undeserializable arguments as a failed tool call.
    assert_eq!(body["result"]["isError"], json!(true), "{body}");
    assert!(
        body["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("UUID"),
        "{body}"
    );
}

#[tokio::test]
async fn search_tools_are_listed() {
    let ctx = TestContext::new().await;
    let (_, token) = user_with_token(&ctx).await;
    let names = ctx.mcp_tool_names(&token).await;
    for tool in ["search_documents", "enhanced_search", "get_search_facets"] {
        assert!(names.iter().any(|n| n == tool), "{tool} missing: {names:?}");
    }
}

#[tokio::test]
async fn get_search_facets_separates_unused_labels() {
    let ctx = TestContext::new().await;
    let (user_id, token) = user_with_token(&ctx).await;
    let doc = seed_document(&ctx, user_id, "a.pdf", "application/pdf", "one").await;
    let pool = &ctx.state().db.pool;
    for name in ["Invoices", "Never used"] {
        sqlx::query("INSERT INTO labels (user_id, name, color) VALUES ($1, $2, '#123456')")
            .bind(user_id)
            .bind(name)
            .execute(pool)
            .await
            .unwrap();
    }
    sqlx::query(
        "INSERT INTO document_labels (document_id, label_id, assigned_by) \
         SELECT $1, id, $2 FROM labels WHERE user_id = $2 AND name = 'Invoices'",
    )
    .bind(doc.id)
    .bind(user_id)
    .execute(pool)
    .await
    .unwrap();

    let result = ctx
        .mcp_call_tool(&token, "get_search_facets", json!({}))
        .await;
    assert_eq!(result["isError"], json!(false), "{result}");
    let text = result["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("  - Invoices (1 document)"), "{text}");
    // New accounts also start with unused default labels.
    let unused = text
        .lines()
        .find(|line| line.starts_with("Labels not on any document yet:"))
        .expect("unused labels line");
    assert!(unused.contains("Never used"), "{text}");
    assert!(!unused.contains("Invoices"), "{text}");
    assert!(!text.contains("Never used (0"), "{text}");
}

/// Boolean-mode ids for `query` via MCP `enhanced_search`; asserts success.
async fn boolean_hits(ctx: &TestContext, token: &str, query: &str) -> Vec<String> {
    let result = ctx
        .mcp_call_tool(
            token,
            "enhanced_search",
            json!({ "query": query, "search_mode": "boolean" }),
        )
        .await;
    assert_eq!(
        result["isError"],
        json!(false),
        "boolean query {query:?} failed: {result}"
    );
    hit_ids(&result)
}

#[tokio::test]
async fn boolean_search_understands_documented_operators() {
    // The tool documents AND/OR/NOT and the web UI documents &, |, !. Raw
    // input used to go to to_tsquery, where "invoice AND 2024" is a syntax
    // error surfaced as an opaque server fault.
    let ctx = TestContext::new().await;
    let (user_id, token) = user_with_token(&ctx).await;
    let paid = seed_document(
        &ctx,
        user_id,
        "paid.txt",
        "text/plain",
        "invoice paid march",
    )
    .await
    .id
    .to_string();
    let draft = seed_document(
        &ctx,
        user_id,
        "draft.txt",
        "text/plain",
        "invoice draft march",
    )
    .await
    .id
    .to_string();
    let receipt = seed_document(
        &ctx,
        user_id,
        "receipt.txt",
        "text/plain",
        "receipt paid april",
    )
    .await
    .id
    .to_string();

    let both = boolean_hits(&ctx, &token, "invoice AND march").await;
    assert!(
        both.contains(&paid) && both.contains(&draft) && !both.contains(&receipt),
        "{both:?}"
    );

    let either = boolean_hits(&ctx, &token, "invoice OR receipt").await;
    assert!(
        either.contains(&paid) && either.contains(&draft) && either.contains(&receipt),
        "{either:?}"
    );

    for query in ["invoice NOT draft", "invoice & !draft", "invoice -draft"] {
        let hits = boolean_hits(&ctx, &token, query).await;
        assert!(
            hits.contains(&paid) && !hits.contains(&draft),
            "{query}: {hits:?}"
        );
    }

    let phrase = boolean_hits(&ctx, &token, "\"invoice paid\"").await;
    assert_eq!(phrase, vec![paid.clone()]);
}

#[tokio::test]
async fn malformed_boolean_queries_never_fail() {
    let ctx = TestContext::new().await;
    let (user_id, token) = user_with_token(&ctx).await;
    seed_document(&ctx, user_id, "a.txt", "text/plain", "invoice paid").await;
    for query in [
        "((invoice",
        "invoice &",
        "| | invoice",
        "!!",
        "invoice & | paid",
        "\"unterminated",
        "NOT",
    ] {
        boolean_hits(&ctx, &token, query).await;
    }
}

#[tokio::test]
async fn rest_boolean_search_accepts_web_ui_syntax() {
    let ctx = TestContext::new().await;
    let (user_id, token) = user_with_token(&ctx).await;
    let paid = seed_document(&ctx, user_id, "paid.txt", "text/plain", "invoice paid")
        .await
        .id
        .to_string();
    let auth = ctx.auth_helper();
    for query in ["invoice%20%26%20paid", "invoice%20AND%20paid", "((invoice"] {
        let body: Value = serde_json::from_slice(
            &auth
                .make_authenticated_request(
                    "GET",
                    &format!("/api/search/enhanced?query={query}&search_mode=boolean"),
                    None,
                    &token,
                )
                .await,
        )
        .unwrap_or_else(|e| panic!("{query}: not JSON ({e}), the search failed"));
        if query != "((invoice" {
            let ids: Vec<&str> = body["documents"]
                .as_array()
                .unwrap()
                .iter()
                .map(|d| d["id"].as_str().unwrap())
                .collect();
            assert!(ids.contains(&paid.as_str()), "{query}: {body}");
        }
    }
}

#[tokio::test]
async fn enhanced_search_rejects_unbounded_requests() {
    // enhanced search used to skip the length and pagination checks basic
    // search applies, sending them straight to Postgres.
    let ctx = TestContext::new().await;
    let (_, token) = user_with_token(&ctx).await;
    let long = "x".repeat(1001);
    let result = ctx
        .mcp_call_tool(&token, "enhanced_search", json!({ "query": long }))
        .await;
    assert_eq!(result["isError"], json!(true), "{result}");
    assert!(text(&result).contains("1000 characters"), "{result}");

    for params in [
        format!("query={}", "x".repeat(1001)),
        "query=invoice&limit=-1".into(),
        "query=invoice&limit=100000".into(),
        "query=invoice&snippet_length=-1".into(),
    ] {
        let response = ctx
            .app()
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri(format!("/api/search/enhanced?{params}"))
                    .header("authorization", format!("Bearer {token}"))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            axum::http::StatusCode::BAD_REQUEST,
            "{params}"
        );
    }
}
