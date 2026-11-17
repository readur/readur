//! MCP document tools: list, get, OCR text, upload, delete and OCR retry.

use axum::http::StatusCode;
use base64ct::{Base64, Encoding};
use serde_json::{json, Value};
use uuid::Uuid;

use readur::models::Document;
use readur::test_utils::{document_helpers, TestConfigBuilder, TestContext};

/// A logged-in user: (id, token).
async fn login(ctx: &TestContext) -> (Uuid, String) {
    let auth = ctx.auth_helper();
    let user = auth.create_test_user().await;
    let token = auth.login_user(&user.username, &user.password).await;
    (user.user_response.id, token)
}

async fn insert(ctx: &TestContext, document: Document) -> Document {
    ctx.state()
        .db
        .create_document(document)
        .await
        .expect("failed to insert document")
}

fn text(result: &Value) -> &str {
    result["content"][0]["text"].as_str().unwrap_or_default()
}

fn assert_ok(result: &Value) {
    assert_eq!(result["isError"], json!(false), "tool failed: {result}");
}

fn assert_tool_error(result: &Value) {
    assert_eq!(
        result["isError"],
        json!(true),
        "expected a tool error: {result}"
    );
}

async fn queued(ctx: &TestContext, document_id: Uuid) -> bool {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ocr_queue WHERE document_id = $1")
        .bind(document_id)
        .fetch_one(&ctx.state().db.pool)
        .await
        .unwrap();
    count > 0
}

#[tokio::test]
async fn list_documents_returns_only_own_documents() {
    let ctx = TestContext::new().await;
    let (alice, alice_token) = login(&ctx).await;
    let (bob, _) = login(&ctx).await;
    let mine = insert(&ctx, document_helpers::create_test_document(alice)).await;
    let theirs = insert(&ctx, document_helpers::create_test_document(bob)).await;

    let result = ctx
        .mcp_call_tool(&alice_token, "list_documents", json!({}))
        .await;
    assert_ok(&result);
    let ids: Vec<&str> = result["structuredContent"]["documents"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&mine.id.to_string().as_str()), "{result}");
    assert!(!ids.contains(&theirs.id.to_string().as_str()), "{result}");
    assert_eq!(result["structuredContent"]["pagination"]["total"], json!(1));
    assert!(text(&result).contains(&mine.id.to_string()));
}

#[tokio::test]
async fn list_documents_filters_by_ocr_status() {
    let ctx = TestContext::new().await;
    let (alice, token) = login(&ctx).await;
    insert(&ctx, document_helpers::create_test_document(alice)).await;
    let pending = insert(&ctx, document_helpers::create_document_without_ocr(alice)).await;

    let result = ctx
        .mcp_call_tool(&token, "list_documents", json!({ "ocr_status": "pending" }))
        .await;
    assert_ok(&result);
    let docs = result["structuredContent"]["documents"].as_array().unwrap();
    assert_eq!(docs.len(), 1, "{result}");
    assert_eq!(docs[0]["id"], json!(pending.id));

    let bad = ctx
        .mcp_call_tool(&token, "list_documents", json!({ "ocr_status": "bogus" }))
        .await;
    assert_tool_error(&bad);
}

#[tokio::test]
async fn get_document_returns_metadata() {
    let ctx = TestContext::new().await;
    let (alice, token) = login(&ctx).await;
    let doc = insert(&ctx, document_helpers::create_test_document(alice)).await;

    let result = ctx
        .mcp_call_tool(&token, "get_document", json!({ "id": doc.id }))
        .await;
    assert_ok(&result);
    assert_eq!(result["structuredContent"]["id"], json!(doc.id));
    assert_eq!(
        result["structuredContent"]["original_filename"],
        json!(doc.original_filename)
    );
    assert!(text(&result).contains(&doc.original_filename));
    assert!(text(&result).contains("completed"));
}

#[tokio::test]
async fn other_users_documents_are_not_found() {
    let ctx = TestContext::new().await;
    let (alice, _) = login(&ctx).await;
    let (_, bob_token) = login(&ctx).await;
    let doc = insert(&ctx, document_helpers::create_test_document(alice)).await;

    for tool in [
        "get_document",
        "get_document_ocr_text",
        "retry_document_ocr",
    ] {
        let result = ctx
            .mcp_call_tool(&bob_token, tool, json!({ "id": doc.id }))
            .await;
        assert_tool_error(&result);
        assert!(
            !text(&result).contains("extracted OCR text"),
            "{tool} leaked content: {result}"
        );
    }
}

#[tokio::test]
async fn non_uuid_ids_are_rejected() {
    let ctx = TestContext::new().await;
    let (_, token) = login(&ctx).await;
    let message = json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {
            "name": "get_document",
            "arguments": { "id": "not-a-uuid" },
            "_meta": {
                "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                "io.modelcontextprotocol/clientInfo": { "name": "t", "version": "1" },
                "io.modelcontextprotocol/clientCapabilities": {}
            }
        }
    });
    let (status, body) = ctx.mcp_post(Some(&token), message).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    // rmcp reports argument deserialization failures as a tool error.
    assert_eq!(body["result"]["isError"], json!(true), "{body}");
    assert!(text(&body["result"]).contains("UUID"), "{body}");
}

#[tokio::test]
async fn ocr_text_is_paged() {
    let ctx = TestContext::new().await;
    let (alice, token) = login(&ctx).await;
    let mut doc = document_helpers::create_test_document(alice);
    let full: String = ('a'..='z').cycle().take(50).collect();
    doc.ocr_text = Some(full.clone());
    let doc = insert(&ctx, doc).await;

    let first = ctx
        .mcp_call_tool(
            &token,
            "get_document_ocr_text",
            json!({ "id": doc.id, "max_chars": 20 }),
        )
        .await;
    assert_ok(&first);
    let data = &first["structuredContent"];
    assert_eq!(data["text"], json!(&full[..20]));
    assert_eq!(data["total_chars"], json!(50));
    assert_eq!(data["next_offset"], json!(20));
    assert!(text(&first).contains(&full[..20]));
    assert!(text(&first).contains("offset"));

    let last = ctx
        .mcp_call_tool(
            &token,
            "get_document_ocr_text",
            json!({ "id": doc.id, "offset": 40, "max_chars": 20 }),
        )
        .await;
    assert_ok(&last);
    assert_eq!(last["structuredContent"]["text"], json!(&full[40..]));
    assert_eq!(last["structuredContent"]["next_offset"], Value::Null);

    let past_end = ctx
        .mcp_call_tool(
            &token,
            "get_document_ocr_text",
            json!({ "id": doc.id, "offset": 51 }),
        )
        .await;
    assert_tool_error(&past_end);
}

#[tokio::test]
async fn ocr_text_reports_pending_status() {
    let ctx = TestContext::new().await;
    let (alice, token) = login(&ctx).await;
    let doc = insert(&ctx, document_helpers::create_document_without_ocr(alice)).await;

    let result = ctx
        .mcp_call_tool(&token, "get_document_ocr_text", json!({ "id": doc.id }))
        .await;
    assert_ok(&result);
    assert!(text(&result).contains("pending"), "{result}");
    assert_eq!(result["structuredContent"]["has_ocr_text"], json!(false));
    assert_eq!(result["structuredContent"]["ocr_status"], json!("pending"));
}

#[tokio::test]
async fn ocr_text_explains_completed_document_without_text() {
    // `mark_documents_ocr_completed` can leave a completed document with no
    // stored text; it must not be described as waiting in the queue.
    let ctx = TestContext::new().await;
    let (alice, token) = login(&ctx).await;
    let mut doc = document_helpers::create_document_without_ocr(alice);
    doc.ocr_status = Some("completed".to_string());
    let doc = insert(&ctx, doc).await;

    let result = ctx
        .mcp_call_tool(
            &token,
            "get_document_ocr_text",
            json!({ "id": doc.id, "offset": 10 }),
        )
        .await;
    assert_ok(&result);
    assert!(text(&result).contains("retry_document_ocr"), "{result}");
    assert!(
        !text(&result).contains("waiting in the OCR queue"),
        "{result}"
    );
}

#[tokio::test]
async fn upload_still_queues_ocr_when_labels_fail() {
    // Labelling runs after the document is stored. If it fails, the upload
    // must still succeed and be queued for OCR: a retry would only hit
    // deduplication and leave the document unprocessed for good.
    let ctx = TestContext::new().await;
    let (alice, token) = login(&ctx).await;
    let pool = &ctx.state().db.pool;
    let label: Uuid = sqlx::query_scalar(
        "INSERT INTO labels (user_id, name, color) VALUES ($1, 'doomed', '#123456') RETURNING id",
    )
    .bind(alice)
    .fetch_one(pool)
    .await
    .unwrap();
    // This test's own database: make every label assignment fail.
    sqlx::raw_sql(
        "CREATE FUNCTION fail_label_insert() RETURNS trigger AS $$ \
         BEGIN RAISE EXCEPTION 'simulated label failure'; END $$ LANGUAGE plpgsql; \
         CREATE TRIGGER fail_label_insert BEFORE INSERT ON document_labels \
         FOR EACH ROW EXECUTE FUNCTION fail_label_insert();",
    )
    .execute(pool)
    .await
    .unwrap();

    let result = ctx
        .mcp_call_tool(
            &token,
            "upload_document",
            json!({
                "filename": "labels-fail.txt",
                "content_base64": Base64::encode_string(b"labels will fail"),
                "label_ids": [label]
            }),
        )
        .await;
    assert_ok(&result);
    let id: Uuid = result["structuredContent"]["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!(queued(&ctx, id).await, "document was not queued for OCR");
    assert_eq!(
        result["structuredContent"]["warnings"]
            .as_array()
            .unwrap()
            .len(),
        1,
        "{result}"
    );
    assert!(text(&result).contains("add_document_label"), "{result}");
}

#[tokio::test]
async fn upload_text_file_is_listed_and_queued() {
    let ctx = TestContext::new().await;
    let (_, token) = login(&ctx).await;
    let content = Base64::encode_string(b"Hello from the MCP upload test.\n");

    let result = ctx
        .mcp_call_tool(
            &token,
            "upload_document",
            json!({ "filename": "mcp-note.txt", "content_base64": content }),
        )
        .await;
    assert_ok(&result);
    let id: Uuid = result["structuredContent"]["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(
        result["structuredContent"]["mime_type"],
        json!("text/plain")
    );
    assert!(queued(&ctx, id).await, "document was not queued for OCR");
    assert_eq!(result["structuredContent"]["warnings"], json!([]));

    let list = ctx.mcp_call_tool(&token, "list_documents", json!({})).await;
    assert!(text(&list).contains(&id.to_string()), "{list}");
}

#[tokio::test]
async fn upload_applies_own_labels_and_rejects_foreign_ones() {
    let ctx = TestContext::new().await;
    let (alice, token) = login(&ctx).await;
    let (bob, _) = login(&ctx).await;
    let pool = &ctx.state().db.pool;
    let mut label_ids = Vec::new();
    for (owner, name) in [(alice, "mcp-mine"), (bob, "mcp-theirs")] {
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO labels (user_id, name, color) VALUES ($1, $2, '#123456') RETURNING id",
        )
        .bind(owner)
        .bind(name)
        .fetch_one(pool)
        .await
        .unwrap();
        label_ids.push(id);
    }

    let foreign = ctx
        .mcp_call_tool(
            &token,
            "upload_document",
            json!({
                "filename": "foreign.txt",
                "content_base64": Base64::encode_string(b"foreign label"),
                "label_ids": [label_ids[1]]
            }),
        )
        .await;
    assert_tool_error(&foreign);

    let result = ctx
        .mcp_call_tool(
            &token,
            "upload_document",
            json!({
                "filename": "labelled.txt",
                "content_base64": Base64::encode_string(b"labelled upload"),
                "label_ids": [label_ids[0]]
            }),
        )
        .await;
    assert_ok(&result);
    let doc_id: Uuid = result["structuredContent"]["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let labels = ctx.state().db.get_document_labels(doc_id).await.unwrap();
    assert_eq!(labels.len(), 1);
    assert_eq!(labels[0].id, label_ids[0]);
}

#[tokio::test]
async fn upload_rejects_invalid_base64() {
    let ctx = TestContext::new().await;
    let (_, token) = login(&ctx).await;
    let result = ctx
        .mcp_call_tool(
            &token,
            "upload_document",
            json!({ "filename": "bad.txt", "content_base64": "this is not base64!!" }),
        )
        .await;
    assert_tool_error(&result);
    assert!(text(&result).to_lowercase().contains("base64"), "{result}");
}

#[tokio::test]
async fn upload_rejects_files_over_the_size_limit() {
    let ctx = TestContext::with_config(TestConfigBuilder::default().with_max_file_size_mb(1)).await;
    let (_, token) = login(&ctx).await;
    let content = Base64::encode_string(&vec![b'x'; 1024 * 1024 + 1]);
    let result = ctx
        .mcp_call_tool(
            &token,
            "upload_document",
            json!({ "filename": "big.txt", "content_base64": content }),
        )
        .await;
    assert_tool_error(&result);
    assert!(text(&result).contains("exceeds"), "{result}");
}

#[tokio::test]
async fn delete_document_is_hidden_by_default() {
    let ctx = TestContext::new().await;
    let (_, token) = login(&ctx).await;
    let names = ctx.mcp_tool_names(&token).await;
    for tool in [
        "list_documents",
        "get_document",
        "get_document_ocr_text",
        "upload_document",
        "retry_document_ocr",
    ] {
        assert!(
            names.contains(&tool.to_string()),
            "{tool} missing: {names:?}"
        );
    }
    assert!(!names.contains(&"delete_document".to_string()));
}

#[tokio::test]
async fn delete_document_works_when_enabled() {
    let ctx =
        TestContext::with_config(TestConfigBuilder::default().with_mcp_allow_delete(true)).await;
    let (alice, token) = login(&ctx).await;
    let (_, bob_token) = login(&ctx).await;
    assert!(ctx
        .mcp_tool_names(&token)
        .await
        .contains(&"delete_document".to_string()));
    let doc = insert(&ctx, document_helpers::create_test_document(alice)).await;

    let foreign = ctx
        .mcp_call_tool(&bob_token, "delete_document", json!({ "id": doc.id }))
        .await;
    assert_tool_error(&foreign);

    let result = ctx
        .mcp_call_tool(&token, "delete_document", json!({ "id": doc.id }))
        .await;
    assert_ok(&result);
    let gone = ctx
        .state()
        .db
        .get_document_by_id(doc.id, alice, readur::models::UserRole::User)
        .await
        .unwrap();
    assert!(gone.is_none());

    let again = ctx
        .mcp_call_tool(&token, "delete_document", json!({ "id": doc.id }))
        .await;
    assert_tool_error(&again);
}

#[tokio::test]
async fn retry_ocr_queues_own_document() {
    let ctx = TestContext::new().await;
    let (alice, token) = login(&ctx).await;
    let doc = insert(
        &ctx,
        document_helpers::create_document_with_ocr_error(alice),
    )
    .await;

    let result = ctx
        .mcp_call_tool(&token, "retry_document_ocr", json!({ "id": doc.id }))
        .await;
    assert_ok(&result);
    assert_eq!(result["structuredContent"]["queued"], json!(true));
    assert!(queued(&ctx, doc.id).await);
}

#[tokio::test]
async fn retry_ocr_twice_reports_already_queued_instead_of_failing() {
    // The queue allows one pending/processing job per document, and a
    // retried 'failed' document keeps that status until the worker starts.
    // The second retry must be a no-op, not a unique-index violation.
    let ctx = TestContext::new().await;
    let (alice, token) = login(&ctx).await;
    let doc = insert(
        &ctx,
        document_helpers::create_document_with_ocr_error(alice),
    )
    .await;

    let first = ctx
        .mcp_call_tool(&token, "retry_document_ocr", json!({ "id": doc.id }))
        .await;
    assert_ok(&first);
    assert_eq!(first["structuredContent"]["queued"], json!(true));

    let second = ctx
        .mcp_call_tool(&token, "retry_document_ocr", json!({ "id": doc.id }))
        .await;
    assert_ok(&second);
    assert_eq!(
        second["structuredContent"]["queued"],
        json!(false),
        "{second}"
    );
    assert!(text(&second).contains("already queued"), "{second}");
}

#[tokio::test]
async fn retry_ocr_on_a_freshly_uploaded_document_is_a_no_op() {
    let ctx = TestContext::new().await;
    let (_, token) = login(&ctx).await;
    let upload = ctx
        .mcp_call_tool(
            &token,
            "upload_document",
            json!({ "filename": "fresh.txt", "content_base64": Base64::encode_string(b"fresh upload") }),
        )
        .await;
    assert_ok(&upload);
    let id = upload["structuredContent"]["id"].clone();

    let retry = ctx
        .mcp_call_tool(&token, "retry_document_ocr", json!({ "id": id }))
        .await;
    assert_ok(&retry);
    assert_eq!(
        retry["structuredContent"]["queued"],
        json!(false),
        "{retry}"
    );
}

#[tokio::test]
async fn rest_retry_ocr_twice_returns_200_not_500() {
    let ctx = TestContext::new().await;
    let (alice, token) = login(&ctx).await;
    let doc = insert(
        &ctx,
        document_helpers::create_document_with_ocr_error(alice),
    )
    .await;
    let auth = ctx.auth_helper();
    let uri = format!("/api/documents/{}/ocr/retry", doc.id);

    let first: Value = serde_json::from_slice(
        &auth
            .make_authenticated_request("POST", &uri, Some(json!({})), &token)
            .await,
    )
    .unwrap();
    assert_eq!(first["success"], json!(true), "{first}");
    let second: Value = serde_json::from_slice(
        &auth
            .make_authenticated_request("POST", &uri, Some(json!({})), &token)
            .await,
    )
    .expect("second retry must return JSON, not a 500");
    assert_eq!(second["success"], json!(false), "{second}");
}

#[tokio::test]
async fn ocr_text_offset_at_the_end_is_rejected() {
    let ctx = TestContext::new().await;
    let (alice, token) = login(&ctx).await;
    let mut doc = document_helpers::create_test_document(alice);
    doc.ocr_text = Some("0123456789".to_string());
    doc.ocr_status = Some("completed".to_string());
    let doc = insert(&ctx, doc).await;

    let last = ctx
        .mcp_call_tool(
            &token,
            "get_document_ocr_text",
            json!({ "id": doc.id, "offset": 9 }),
        )
        .await;
    assert_ok(&last);
    assert_eq!(last["structuredContent"]["text"], json!("9"));

    let past = ctx
        .mcp_call_tool(
            &token,
            "get_document_ocr_text",
            json!({ "id": doc.id, "offset": 10 }),
        )
        .await;
    assert_tool_error(&past);
    assert!(text(&past).contains("past the end"), "{past}");
}

#[tokio::test]
async fn ocr_text_flags_stale_text_while_a_new_run_is_queued() {
    let ctx = TestContext::new().await;
    let (alice, token) = login(&ctx).await;
    let mut doc = document_helpers::create_test_document(alice);
    doc.ocr_text = Some("old blurry text".to_string());
    doc.ocr_status = Some("completed".to_string());
    let doc = insert(&ctx, doc).await;

    let before = ctx
        .mcp_call_tool(&token, "get_document_ocr_text", json!({ "id": doc.id }))
        .await;
    assert_eq!(
        before["structuredContent"]["reprocessing"],
        json!(false),
        "{before}"
    );
    assert!(!text(&before).contains("previous OCR run"), "{before}");

    let retry = ctx
        .mcp_call_tool(&token, "retry_document_ocr", json!({ "id": doc.id }))
        .await;
    assert_eq!(retry["structuredContent"]["queued"], json!(true), "{retry}");

    let during = ctx
        .mcp_call_tool(&token, "get_document_ocr_text", json!({ "id": doc.id }))
        .await;
    assert_ok(&during);
    assert_eq!(
        during["structuredContent"]["reprocessing"],
        json!(true),
        "{during}"
    );
    assert!(text(&during).contains("previous OCR run"), "{during}");
    assert!(text(&during).contains("old blurry text"), "{during}");
}

#[tokio::test]
async fn retry_ocr_with_languages_stores_them_as_the_users_preference() {
    // The retry validates the languages once and then saves them; saving
    // must not be skipped or depend on a second validation pass.
    let ctx = TestContext::new().await;
    let (alice, token) = login(&ctx).await;
    let doc = insert(
        &ctx,
        document_helpers::create_document_with_ocr_error(alice),
    )
    .await;
    let auth = ctx.auth_helper();

    let body: Value = serde_json::from_slice(
        &auth
            .make_authenticated_request(
                "POST",
                &format!("/api/documents/{}/ocr/retry", doc.id),
                Some(json!({ "languages": ["eng"] })),
                &token,
            )
            .await,
    )
    .unwrap();
    assert_eq!(body["success"], json!(true), "{body}");

    let settings = ctx
        .state()
        .db
        .get_user_settings(alice)
        .await
        .unwrap()
        .expect("settings saved");
    assert_eq!(settings.preferred_languages, vec!["eng".to_string()]);
    assert_eq!(settings.primary_language, "eng");
}

#[tokio::test]
async fn concurrent_ocr_retries_queue_exactly_one_job() {
    // A double-click or a client retry sends several retries at once. With a
    // check-then-insert they could all pass the check and all but one hit the
    // queue's unique index (500). The insert itself must arbitrate.
    let ctx = TestContext::with_config(TestConfigBuilder::default()).await;
    let (alice, _) = login(&ctx).await;
    let doc = insert(
        &ctx,
        document_helpers::create_document_with_ocr_error(alice),
    )
    .await;
    let user = ctx.state().db.get_user_by_id(alice).await.unwrap().unwrap();

    let attempts = (0..8).map(|_| {
        readur::services::documents::retry_document_ocr(ctx.state(), &user, doc.id, None, None)
    });
    let outcomes = futures::future::join_all(attempts).await;

    let mut queued = 0;
    for outcome in outcomes {
        let outcome = outcome.unwrap_or_else(|e| panic!("a concurrent retry failed: {e:?}"));
        if outcome.queued {
            queued += 1;
        }
    }
    assert_eq!(queued, 1, "exactly one retry should add a job");
    let jobs: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ocr_queue WHERE document_id = $1")
        .bind(doc.id)
        .fetch_one(&ctx.state().db.pool)
        .await
        .unwrap();
    assert_eq!(jobs, 1);
}

#[tokio::test]
async fn wrapped_base64_upload_near_the_size_limit_is_accepted() {
    // Clients often send MIME-wrapped base64 (76 characters + CRLF). The
    // JSON-RPC body limit must leave room for that, or rmcp rejects a file
    // under MAX_FILE_SIZE_MB with a transport error before the tool runs.
    let uploads = tempfile::tempdir().unwrap();
    let ctx = TestContext::with_config(
        TestConfigBuilder::default().with_upload_path(uploads.path().to_str().unwrap()),
    )
    .await;
    let (_, token) = login(&ctx).await;
    let max = ctx.state().config.max_file_size_mb as usize * 1024 * 1024;
    let content: Vec<u8> = (0..max - 1024).map(|i| (i % 251) as u8).collect();
    let encoded = Base64::encode_string(&content);
    let wrapped = encoded
        .as_bytes()
        .chunks(76)
        .map(|line| std::str::from_utf8(line).unwrap())
        .collect::<Vec<_>>()
        .join("\r\n");

    let result = ctx
        .mcp_call_tool(
            &token,
            "upload_document",
            json!({ "filename": "near-limit.bin", "content_base64": wrapped }),
        )
        .await;
    assert_ok(&result);
    assert_eq!(
        result["structuredContent"]["file_size"],
        json!(content.len())
    );
}

#[tokio::test]
async fn ocr_queue_index_still_rejects_duplicate_active_and_orphan_jobs() {
    // Migration 20261116000001 moved duplicate prevention from the guardrail
    // trigger to the partial unique index. The enqueue API tolerates
    // duplicates (tests/integration_ocr_enqueue_tests.rs); the table itself
    // must still refuse them for any raw insert.
    let ctx = TestContext::new().await;
    let (alice, _) = login(&ctx).await;
    let doc = insert(
        &ctx,
        document_helpers::create_document_with_ocr_error(alice),
    )
    .await;
    let pool = &ctx.state().db.pool;
    let raw_insert = |document_id: Uuid| {
        sqlx::query("INSERT INTO ocr_queue (document_id, priority, file_size) VALUES ($1, 5, 1)")
            .bind(document_id)
            .execute(pool)
    };

    raw_insert(doc.id).await.expect("first job is accepted");
    assert!(
        raw_insert(doc.id).await.is_err(),
        "a second pending job for the same document must be rejected"
    );
    assert!(
        raw_insert(Uuid::new_v4()).await.is_err(),
        "a job for a document that does not exist must be rejected"
    );
    sqlx::query("UPDATE ocr_queue SET status = 'completed' WHERE document_id = $1")
        .bind(doc.id)
        .execute(pool)
        .await
        .unwrap();
    raw_insert(doc.id).await.expect("re-queue after completion");
}

async fn ocr_status(ctx: &TestContext, document_id: Uuid) -> Option<String> {
    sqlx::query_scalar("SELECT ocr_status FROM documents WHERE id = $1")
        .bind(document_id)
        .fetch_one(&ctx.state().db.pool)
        .await
        .unwrap()
}

/// A document left at 'processing' with no queue job, as after a worker
/// crash whose job was cleaned up.
async fn stuck_processing_document(ctx: &TestContext, owner: Uuid) -> Document {
    let mut doc = document_helpers::create_document_without_ocr(owner);
    doc.ocr_status = Some("processing".to_string());
    insert(ctx, doc).await
}

#[tokio::test]
async fn stuck_processing_document_is_reported_and_can_be_retried_via_mcp() {
    let ctx = TestContext::new().await;
    let (alice, token) = login(&ctx).await;
    let doc = stuck_processing_document(&ctx, alice).await;

    let before = ctx
        .mcp_call_tool(&token, "get_document_ocr_text", json!({ "id": doc.id }))
        .await;
    assert_ok(&before);
    assert!(text(&before).contains("retry_document_ocr"), "{before}");
    assert!(!text(&before).contains("Check back"), "{before}");

    let retry = ctx
        .mcp_call_tool(&token, "retry_document_ocr", json!({ "id": doc.id }))
        .await;
    assert_ok(&retry);
    assert_eq!(retry["structuredContent"]["queued"], json!(true), "{retry}");
    assert!(queued(&ctx, doc.id).await);
    // The stale status is reset; the worker sets 'processing' when it starts.
    assert_eq!(ocr_status(&ctx, doc.id).await.as_deref(), Some("pending"));

    let after = ctx
        .mcp_call_tool(&token, "get_document_ocr_text", json!({ "id": doc.id }))
        .await;
    assert!(text(&after).contains("waiting in the OCR queue"), "{after}");
}

#[tokio::test]
async fn stuck_processing_document_can_be_retried_via_rest() {
    let ctx = TestContext::new().await;
    let (alice, token) = login(&ctx).await;
    let doc = stuck_processing_document(&ctx, alice).await;

    let body: Value = serde_json::from_slice(
        &ctx.auth_helper()
            .make_authenticated_request(
                "POST",
                &format!("/api/documents/{}/ocr/retry", doc.id),
                Some(json!({})),
                &token,
            )
            .await,
    )
    .unwrap();
    assert_eq!(body["success"], json!(true), "{body}");
    assert!(queued(&ctx, doc.id).await);
    assert_eq!(ocr_status(&ctx, doc.id).await.as_deref(), Some("pending"));
}

#[tokio::test]
async fn processing_document_with_a_running_job_is_not_queued_again() {
    let ctx = TestContext::new().await;
    let (alice, token) = login(&ctx).await;
    let doc = stuck_processing_document(&ctx, alice).await;
    sqlx::query(
        "INSERT INTO ocr_queue (document_id, priority, file_size, status, started_at) \
         VALUES ($1, 5, 1, 'processing', NOW())",
    )
    .bind(doc.id)
    .execute(&ctx.state().db.pool)
    .await
    .unwrap();

    let ocr = ctx
        .mcp_call_tool(&token, "get_document_ocr_text", json!({ "id": doc.id }))
        .await;
    assert!(text(&ocr).contains("Check back shortly"), "{ocr}");

    let retry = ctx
        .mcp_call_tool(&token, "retry_document_ocr", json!({ "id": doc.id }))
        .await;
    assert_ok(&retry);
    assert_eq!(
        retry["structuredContent"]["queued"],
        json!(false),
        "{retry}"
    );
    assert!(text(&retry).contains("already queued"), "{retry}");
    assert_eq!(
        ocr_status(&ctx, doc.id).await.as_deref(),
        Some("processing")
    );
}

#[tokio::test]
async fn identical_mcp_upload_returns_the_existing_document_with_labels() {
    let uploads = tempfile::tempdir().unwrap();
    let ctx = TestContext::with_config(
        TestConfigBuilder::default().with_upload_path(uploads.path().to_str().unwrap()),
    )
    .await;
    let (alice, token) = login(&ctx).await;
    let label: Uuid = sqlx::query_scalar(
        "INSERT INTO labels (user_id, name, color) VALUES ($1, 'reupload', '#123456') RETURNING id",
    )
    .bind(alice)
    .fetch_one(&ctx.state().db.pool)
    .await
    .unwrap();
    let content = Base64::encode_string(b"the same bytes twice");

    let first = ctx
        .mcp_call_tool(
            &token,
            "upload_document",
            json!({ "filename": "once.txt", "content_base64": content }),
        )
        .await;
    assert_ok(&first);
    let second = ctx
        .mcp_call_tool(
            &token,
            "upload_document",
            json!({ "filename": "twice.txt", "content_base64": content, "label_ids": [label] }),
        )
        .await;
    assert_ok(&second);
    assert_eq!(
        second["structuredContent"]["status"],
        json!("duplicate"),
        "{second}"
    );
    assert_eq!(
        second["structuredContent"]["id"], first["structuredContent"]["id"],
        "{second}"
    );
    assert!(text(&second).contains("already exists"), "{second}");

    let id: Uuid = first["structuredContent"]["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let labels = ctx.state().db.get_document_labels(id).await.unwrap();
    assert_eq!(labels.iter().map(|l| l.id).collect::<Vec<_>>(), vec![label]);
}

#[tokio::test]
async fn identical_rest_upload_is_still_a_conflict() {
    // The web UI treats any 2xx upload as a new document and shows the 409
    // message as the reason a file was not added; keep that contract.
    let uploads = tempfile::tempdir().unwrap();
    let ctx = TestContext::with_config(
        TestConfigBuilder::default().with_upload_path(uploads.path().to_str().unwrap()),
    )
    .await;
    let (alice, token) = login(&ctx).await;
    let upload = || {
        let boundary = "readur-reupload-boundary";
        let body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"same.txt\"\r\n\
             Content-Type: text/plain\r\n\r\nidentical REST upload\r\n--{boundary}--\r\n"
        );
        let request = axum::http::Request::builder()
            .method("POST")
            .uri("/api/documents")
            .header("Authorization", format!("Bearer {token}"))
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={boundary}"),
            )
            .body(axum::body::Body::from(body))
            .unwrap();
        tower::ServiceExt::oneshot(ctx.app().clone(), request)
    };

    let first = upload().await.unwrap();
    assert_eq!(first.status(), StatusCode::OK);
    let second = upload().await.unwrap();
    assert_eq!(second.status(), StatusCode::CONFLICT);
    let body: Value = serde_json::from_slice(
        &axum::body::to_bytes(second.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert!(body.to_string().contains("already exists"), "{body}");

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM documents WHERE user_id = $1")
        .bind(alice)
        .fetch_one(&ctx.state().db.pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}
