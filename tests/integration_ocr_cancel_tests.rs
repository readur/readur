//! Cancelling a document's OCR (`OcrQueueService::cancel_document`,
//! `POST /api/documents/{id}/ocr/stop`, the `cancel_document_ocr` MCP tool):
//! a queued job never runs, a running job's result is never written, a
//! cancel racing a finishing worker has exactly one winner, and a cancelled
//! document can be retried.

use axum::{body::Body, http::Request, http::StatusCode};
use serde_json::{json, Value};
use tempfile::TempDir;
use tower::ServiceExt;
use uuid::Uuid;

use readur::db_guardrails_simple::{DocumentTransactionManager, OcrWrite, OcrWriteOutcome};
use readur::models::Document;
use readur::ocr::enhanced::EnhancedOcrService;
use readur::ocr::queue::{
    CancelOutcome, CancelledDocument, EnqueueOutcome, CANCELLED_FAILURE_REASON,
};
use readur::services::file_service::FileService;
use readur::storage::{factory::create_storage_backend, StorageConfig};
use readur::test_utils::{document_helpers, TestContext};

const TEXT: &str = "The quick brown fox jumps over the lazy dog near the river bank today";

async fn user(ctx: &TestContext) -> (Uuid, String) {
    let auth = ctx.auth_helper();
    let user = auth.create_test_user().await;
    let token = auth.login_user(&user.username, &user.password).await;
    (user.user_response.id, token)
}

/// A pending text document whose file holds [`TEXT`], so a worker run on it
/// really extracts text.
async fn text_document(ctx: &TestContext, dir: &TempDir, owner: Uuid) -> Document {
    let path = dir.path().join(format!("{}.txt", Uuid::new_v4().simple()));
    std::fs::write(&path, TEXT).unwrap();
    let mut doc = document_helpers::create_test_document(owner);
    doc.filename = "note.txt".to_string();
    doc.original_filename = "note.txt".to_string();
    doc.file_path = path.to_string_lossy().into_owned();
    doc.file_size = TEXT.len() as i64;
    doc.mime_type = "text/plain".to_string();
    doc.ocr_text = None;
    doc.ocr_confidence = None;
    doc.ocr_word_count = None;
    doc.ocr_processing_time_ms = None;
    doc.ocr_completed_at = None;
    doc.ocr_status = Some("pending".to_string());
    doc.file_hash = Some(Uuid::new_v4().simple().to_string());
    ctx.state().db.create_document(doc).await.unwrap()
}

async fn ocr_service(dir: &TempDir) -> EnhancedOcrService {
    let path = dir.path().to_string_lossy().into_owned();
    let storage = create_storage_backend(StorageConfig::Local {
        upload_path: path.clone(),
    })
    .await
    .unwrap();
    EnhancedOcrService::new(
        path.clone(),
        FileService::with_storage(path, storage),
        100,
        100,
        60,
    )
}

/// `(ocr_status, ocr_failure_reason, ocr_text)` of the document.
async fn doc_state(
    ctx: &TestContext,
    id: Uuid,
) -> (Option<String>, Option<String>, Option<String>) {
    sqlx::query_as("SELECT ocr_status, ocr_failure_reason, ocr_text FROM documents WHERE id = $1")
        .bind(id)
        .fetch_one(&ctx.state().db.pool)
        .await
        .unwrap()
}

async fn job_count(ctx: &TestContext, id: Uuid) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM ocr_queue WHERE document_id = $1")
        .bind(id)
        .fetch_one(&ctx.state().db.pool)
        .await
        .unwrap()
}

async fn failed_document_records(ctx: &TestContext, filename: &str, owner: Uuid) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM failed_documents WHERE user_id = $1 AND filename = $2")
        .bind(owner)
        .bind(filename)
        .fetch_one(&ctx.state().db.pool)
        .await
        .unwrap()
}

async fn enqueue(ctx: &TestContext, doc: &Document) {
    let outcome = ctx
        .state()
        .queue_service
        .enqueue_document(doc.id, 5, doc.file_size)
        .await
        .unwrap();
    assert!(matches!(outcome, EnqueueOutcome::Queued(_)), "{outcome:?}");
}

/// Claims the next job, which must be `doc`'s.
async fn claim(ctx: &TestContext, doc: &Document) -> readur::ocr::queue::OcrQueueItem {
    let item = ctx
        .state()
        .queue_service
        .dequeue()
        .await
        .unwrap()
        .expect("a claimable job");
    assert_eq!(item.document_id, doc.id);
    item
}

fn assert_cancelled_state(state: (Option<String>, Option<String>, Option<String>)) {
    assert_eq!(state.0.as_deref(), Some("failed"), "{state:?}");
    assert_eq!(
        state.1.as_deref(),
        Some(CANCELLED_FAILURE_REASON),
        "{state:?}"
    );
    assert_eq!(state.2, None, "a cancelled job's text must not be written");
}

#[tokio::test]
async fn cancelling_a_pending_job_removes_it_before_any_worker_claims_it() {
    let ctx = TestContext::new().await;
    let dir = TempDir::new().unwrap();
    let (alice, _) = user(&ctx).await;
    let doc = text_document(&ctx, &dir, alice).await;
    enqueue(&ctx, &doc).await;
    let queue = &ctx.state().queue_service;

    let outcome = queue.cancel_document(doc.id).await.unwrap();
    assert_eq!(
        outcome,
        CancelOutcome::Cancelled {
            was_running: false,
            document: CancelledDocument::Failed
        }
    );
    assert_eq!(job_count(&ctx, doc.id).await, 0);
    assert!(!queue.has_active_job(doc.id).await.unwrap());
    assert!(
        queue.dequeue().await.unwrap().is_none(),
        "nothing left to claim"
    );
    assert_cancelled_state(doc_state(&ctx, doc.id).await);

    assert_eq!(
        queue.cancel_document(doc.id).await.unwrap(),
        CancelOutcome::NothingToCancel,
        "a second cancel finds nothing"
    );
}

#[tokio::test]
async fn a_running_job_cancelled_mid_flight_never_writes_its_result() {
    let ctx = TestContext::new().await;
    let dir = TempDir::new().unwrap();
    let (alice, _) = user(&ctx).await;
    let doc = text_document(&ctx, &dir, alice).await;
    enqueue(&ctx, &doc).await;
    let item = claim(&ctx, &doc).await;
    let queue = &ctx.state().queue_service;

    let outcome = queue.cancel_document(doc.id).await.unwrap();
    assert_eq!(
        outcome,
        CancelOutcome::Cancelled {
            was_running: true,
            document: CancelledDocument::Failed
        }
    );

    // The worker finishes the extraction it had started.
    queue
        .process_item(item, &ocr_service(&dir).await)
        .await
        .unwrap();
    assert_cancelled_state(doc_state(&ctx, doc.id).await);
    assert_eq!(job_count(&ctx, doc.id).await, 0);
}

#[tokio::test]
async fn a_cancelled_running_job_that_then_fails_writes_no_failure() {
    let ctx = TestContext::new().await;
    let dir = TempDir::new().unwrap();
    let (alice, _) = user(&ctx).await;
    let doc = text_document(&ctx, &dir, alice).await;
    enqueue(&ctx, &doc).await;
    let item = claim(&ctx, &doc).await;
    // The extraction will fail: its file is gone.
    std::fs::remove_file(&doc.file_path).unwrap();
    let queue = &ctx.state().queue_service;

    let _ = queue.cancel_document(doc.id).await.unwrap();
    queue
        .process_item(item, &ocr_service(&dir).await)
        .await
        .unwrap();
    assert_cancelled_state(doc_state(&ctx, doc.id).await);
    assert_eq!(job_count(&ctx, doc.id).await, 0, "no job put back to retry");
    assert_eq!(
        failed_document_records(&ctx, &doc.filename, alice).await,
        0,
        "a cancelled job records no failure"
    );
}

#[tokio::test]
async fn cancel_racing_a_finishing_worker_has_exactly_one_winner() {
    let ctx = TestContext::new().await;
    let dir = TempDir::new().unwrap();
    let (alice, _) = user(&ctx).await;
    let writer = DocumentTransactionManager::new(ctx.state().db.pool.clone());
    let queue = &ctx.state().queue_service;
    let (mut cancels_won, mut workers_won) = (0, 0);

    for _ in 0..20 {
        let doc = text_document(&ctx, &dir, alice).await;
        enqueue(&ctx, &doc).await;
        let claim = claim(&ctx, &doc).await.claim().unwrap();
        let write = OcrWrite {
            document_id: doc.id,
            expected_filename: &doc.filename,
            ocr_text: TEXT,
            confidence: 99.0,
            word_count: 14,
            processing_time_ms: 5,
        };
        let (cancel, written) = tokio::join!(
            queue.cancel_document(doc.id),
            writer.update_ocr_for_job(&claim, &write)
        );
        let (cancel, written) = (cancel.unwrap(), written.unwrap());
        let state = doc_state(&ctx, doc.id).await;
        match (cancel, written) {
            (
                CancelOutcome::Cancelled {
                    was_running: true,
                    document: CancelledDocument::Failed,
                },
                OcrWriteOutcome::JobCancelled,
            ) => {
                cancels_won += 1;
                assert_cancelled_state(state);
            }
            (CancelOutcome::NothingToCancel, OcrWriteOutcome::Written) => {
                workers_won += 1;
                assert_eq!(state.0.as_deref(), Some("completed"), "{state:?}");
                assert_eq!(state.2.as_deref(), Some(TEXT));
            }
            other => panic!("both or neither won: {other:?}, document {state:?}"),
        }
        assert_eq!(job_count(&ctx, doc.id).await, 0);
    }
    assert_eq!(cancels_won + workers_won, 20);
}

#[tokio::test]
async fn nothing_to_cancel_leaves_the_document_alone() {
    let ctx = TestContext::new().await;
    let (alice, _) = user(&ctx).await;
    let mut completed = document_helpers::create_test_document(alice);
    completed.file_hash = Some(Uuid::new_v4().simple().to_string());
    let completed = ctx.state().db.create_document(completed).await.unwrap();
    let queue = &ctx.state().queue_service;

    assert_eq!(
        queue.cancel_document(completed.id).await.unwrap(),
        CancelOutcome::NothingToCancel
    );
    assert_eq!(
        queue.cancel_document(Uuid::new_v4()).await.unwrap(),
        CancelOutcome::NothingToCancel
    );
    let state = doc_state(&ctx, completed.id).await;
    assert_eq!(state.0.as_deref(), Some("completed"));
    assert_eq!(state.2.as_deref(), Some("This is extracted OCR text"));
}

#[tokio::test]
async fn a_cancelled_retry_of_a_completed_document_keeps_its_text() {
    let ctx = TestContext::new().await;
    let (alice, _) = user(&ctx).await;
    let mut completed = document_helpers::create_test_document(alice);
    completed.file_hash = Some(Uuid::new_v4().simple().to_string());
    let completed = ctx.state().db.create_document(completed).await.unwrap();
    enqueue(&ctx, &completed).await;

    let outcome = ctx
        .state()
        .queue_service
        .cancel_document(completed.id)
        .await
        .unwrap();
    assert!(outcome.cancelled());
    let state = doc_state(&ctx, completed.id).await;
    assert_eq!(state.0.as_deref(), Some("completed"), "{state:?}");
    assert_eq!(state.2.as_deref(), Some("This is extracted OCR text"));
}

#[tokio::test]
async fn a_cancelled_document_can_be_retried_to_completion() {
    let ctx = TestContext::new().await;
    let dir = TempDir::new().unwrap();
    let (alice, _) = user(&ctx).await;
    let doc = text_document(&ctx, &dir, alice).await;
    enqueue(&ctx, &doc).await;
    let item = claim(&ctx, &doc).await;
    let queue = &ctx.state().queue_service;
    let _ = queue.cancel_document(doc.id).await.unwrap();
    let ocr = ocr_service(&dir).await;
    queue.process_item(item, &ocr).await.unwrap();

    enqueue(&ctx, &doc).await;
    let retried = claim(&ctx, &doc).await;
    queue.process_item(retried, &ocr).await.unwrap();
    let state = doc_state(&ctx, doc.id).await;
    assert_eq!(state.0.as_deref(), Some("completed"), "{state:?}");
    assert_eq!(state.2.as_deref(), Some(TEXT));
}

async fn post_stop(ctx: &TestContext, token: &str, id: Uuid) -> (StatusCode, Value) {
    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/documents/{id}/ocr/stop"))
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = ctx.app().clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or_default())
}

#[tokio::test]
async fn http_stop_cancels_queued_ocr_and_reports_what_it_did() {
    let ctx = TestContext::new().await;
    let dir = TempDir::new().unwrap();
    let (alice, token) = user(&ctx).await;
    let doc = text_document(&ctx, &dir, alice).await;
    enqueue(&ctx, &doc).await;

    let (status, body) = post_stop(&ctx, &token, doc.id).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["success"], json!(true), "{body}");
    assert_eq!(body["cancelled"], json!(true), "{body}");
    assert_eq!(body["was_running"], json!(false), "{body}");
    assert_eq!(body["document_status"], json!("failed"), "{body}");
    assert!(
        body["message"].as_str().unwrap().contains("'failed'"),
        "{body}"
    );
    assert_eq!(job_count(&ctx, doc.id).await, 0);
    assert_cancelled_state(doc_state(&ctx, doc.id).await);

    let (status, again) = post_stop(&ctx, &token, doc.id).await;
    assert_eq!(status, StatusCode::OK, "{again}");
    assert_eq!(again["success"], json!(true), "{again}");
    assert_eq!(again["cancelled"], json!(false), "{again}");
    assert_eq!(again["document_status"], json!("failed"), "{again}");
}

#[tokio::test]
async fn cancelling_a_completed_documents_re_ocr_reports_that_it_kept_its_text() {
    let ctx = TestContext::new().await;
    let (alice, token) = user(&ctx).await;
    let mut completed = document_helpers::create_test_document(alice);
    completed.file_hash = Some(Uuid::new_v4().simple().to_string());
    let completed = ctx.state().db.create_document(completed).await.unwrap();

    // Queued re-OCR, through the service.
    enqueue(&ctx, &completed).await;
    assert_eq!(
        ctx.state()
            .queue_service
            .cancel_document(completed.id)
            .await
            .unwrap(),
        CancelOutcome::Cancelled {
            was_running: false,
            document: CancelledDocument::KeptCompleted
        }
    );

    // Running re-OCR, over HTTP.
    enqueue(&ctx, &completed).await;
    let _running = claim(&ctx, &completed).await;
    let (status, body) = post_stop(&ctx, &token, completed.id).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["cancelled"], json!(true), "{body}");
    assert_eq!(body["was_running"], json!(true), "{body}");
    assert_eq!(body["document_status"], json!("completed"), "{body}");
    let message = body["message"].as_str().unwrap();
    assert!(message.contains("keeps its existing OCR text"), "{body}");
    assert!(!message.contains("'failed'"), "{body}");
    let state = doc_state(&ctx, completed.id).await;
    assert_eq!(state.0.as_deref(), Some("completed"), "{state:?}");
    assert_eq!(state.2.as_deref(), Some("This is extracted OCR text"));
}

#[tokio::test]
async fn http_stop_of_another_users_or_a_missing_document_is_not_found() {
    let ctx = TestContext::new().await;
    let dir = TempDir::new().unwrap();
    let (alice, _) = user(&ctx).await;
    let (_, bob_token) = user(&ctx).await;
    let doc = text_document(&ctx, &dir, alice).await;
    enqueue(&ctx, &doc).await;

    let (status, body) = post_stop(&ctx, &bob_token, doc.id).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    assert_eq!(job_count(&ctx, doc.id).await, 1, "alice's job is untouched");
    assert_eq!(doc_state(&ctx, doc.id).await.0.as_deref(), Some("pending"));

    let (status, body) = post_stop(&ctx, &bob_token, Uuid::new_v4()).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
}

#[tokio::test]
async fn mcp_cancel_document_ocr_tool() {
    let ctx = TestContext::new().await;
    let dir = TempDir::new().unwrap();
    let (alice, token) = user(&ctx).await;
    let (_, bob_token) = user(&ctx).await;
    let doc = text_document(&ctx, &dir, alice).await;
    enqueue(&ctx, &doc).await;

    let tools = ctx.mcp_request(&token, "tools/list", json!({})).await;
    let tool = tools["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "cancel_document_ocr")
        .expect("cancel_document_ocr is listed")
        .clone();
    assert_eq!(tool["annotations"]["idempotentHint"], json!(true), "{tool}");
    assert_eq!(
        tool["annotations"]["destructiveHint"],
        json!(false),
        "{tool}"
    );
    assert_eq!(tool["annotations"]["readOnlyHint"], json!(false), "{tool}");

    let foreign = ctx
        .mcp_call_tool(&bob_token, "cancel_document_ocr", json!({ "id": doc.id }))
        .await;
    assert_eq!(foreign["isError"], json!(true), "{foreign}");
    assert_eq!(job_count(&ctx, doc.id).await, 1);

    let result = ctx
        .mcp_call_tool(&token, "cancel_document_ocr", json!({ "id": doc.id }))
        .await;
    assert_eq!(result["isError"], json!(false), "{result}");
    assert_eq!(
        result["structuredContent"]["cancelled"],
        json!(true),
        "{result}"
    );
    assert_cancelled_state(doc_state(&ctx, doc.id).await);

    let again = ctx
        .mcp_call_tool(&token, "cancel_document_ocr", json!({ "id": doc.id }))
        .await;
    assert_eq!(again["isError"], json!(false), "{again}");
    assert_eq!(
        again["structuredContent"]["cancelled"],
        json!(false),
        "{again}"
    );
}
