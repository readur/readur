//! `OcrQueueService::enqueue_document` and its callers: a document that is
//! already queued is an `AlreadyQueued` outcome, never an error, and every
//! caller treats it as such.

use serde_json::{json, Value};
use uuid::Uuid;

use readur::models::Document;
use readur::ocr::queue::EnqueueOutcome;
use readur::services::ocr_retry_service::OcrRetryService;
use readur::test_utils::{document_helpers, TestContext};

async fn user(ctx: &TestContext) -> (Uuid, String) {
    let auth = ctx.auth_helper();
    let user = auth.create_test_user().await;
    let token = auth.login_user(&user.username, &user.password).await;
    (user.user_response.id, token)
}

async fn insert(ctx: &TestContext, mut document: Document) -> Document {
    // The helpers share a content hash; give each document its own so one
    // user can have several.
    document.file_hash = Some(Uuid::new_v4().simple().to_string());
    ctx.state()
        .db
        .create_document(document)
        .await
        .expect("failed to insert document")
}

/// `(status, attempts)` of every queue job for the document, oldest first.
async fn jobs(ctx: &TestContext, document_id: Uuid) -> Vec<(String, i32)> {
    sqlx::query_as(
        "SELECT status, attempts FROM ocr_queue WHERE document_id = $1 ORDER BY created_at",
    )
    .bind(document_id)
    .fetch_all(&ctx.state().db.pool)
    .await
    .unwrap()
}

async fn active_jobs(ctx: &TestContext, document_id: Uuid) -> usize {
    jobs(ctx, document_id)
        .await
        .iter()
        .filter(|(status, _)| status == "pending" || status == "processing")
        .count()
}

async fn ocr_status(ctx: &TestContext, document_id: Uuid) -> Option<String> {
    sqlx::query_scalar("SELECT ocr_status FROM documents WHERE id = $1")
        .bind(document_id)
        .fetch_one(&ctx.state().db.pool)
        .await
        .unwrap()
}

async fn sql(ctx: &TestContext, query: &'static str, document_id: Uuid) {
    sqlx::query(query)
        .bind(document_id)
        .execute(&ctx.state().db.pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn enqueue_reports_already_queued_instead_of_failing() {
    let ctx = TestContext::new().await;
    let (alice, _) = user(&ctx).await;
    let doc = insert(
        &ctx,
        document_helpers::create_document_with_ocr_error(alice),
    )
    .await;
    let queue = &ctx.state().queue_service;

    let first = queue
        .enqueue_document(doc.id, 5, doc.file_size)
        .await
        .unwrap();
    assert!(matches!(first, EnqueueOutcome::Queued(_)), "{first:?}");
    let second = queue
        .enqueue_document(doc.id, 5, doc.file_size)
        .await
        .unwrap();
    assert_eq!(second, EnqueueOutcome::AlreadyQueued);
    assert_eq!(second.queue_id(), None);
    assert_eq!(active_jobs(&ctx, doc.id).await, 1);
    assert!(queue.has_active_job(doc.id).await.unwrap());

    assert!(
        queue.enqueue_document(Uuid::new_v4(), 5, 1).await.is_err(),
        "a job for a document that does not exist must be rejected"
    );

    // Once the job is done, the document can be queued again.
    sql(
        &ctx,
        "UPDATE ocr_queue SET status = 'completed' WHERE document_id = $1",
        doc.id,
    )
    .await;
    assert!(!queue.has_active_job(doc.id).await.unwrap());
    let again = queue
        .enqueue_document(doc.id, 5, doc.file_size)
        .await
        .unwrap();
    assert!(again.queue_id().is_some(), "{again:?}");
}

#[tokio::test]
async fn concurrent_enqueues_add_exactly_one_job() {
    let ctx = TestContext::new().await;
    let (alice, _) = user(&ctx).await;
    let doc = insert(
        &ctx,
        document_helpers::create_document_with_ocr_error(alice),
    )
    .await;
    let queue = &ctx.state().queue_service;

    let outcomes =
        futures::future::join_all((0..8).map(|_| queue.enqueue_document(doc.id, 5, doc.file_size)))
            .await;
    let queued = outcomes
        .into_iter()
        .map(|o| o.expect("a concurrent enqueue failed"))
        .filter(|o| o.queue_id().is_some())
        .count();
    assert_eq!(queued, 1);
    assert_eq!(active_jobs(&ctx, doc.id).await, 1);
}

#[tokio::test]
async fn enqueue_resets_a_stale_processing_status_only_when_it_adds_a_job() {
    let ctx = TestContext::new().await;
    let (alice, _) = user(&ctx).await;
    let queue = &ctx.state().queue_service;

    // 'processing' with no job: a worker died and its job is gone.
    let mut stuck = document_helpers::create_document_without_ocr(alice);
    stuck.ocr_status = Some("processing".to_string());
    let stuck = insert(&ctx, stuck).await;
    let outcome = queue
        .enqueue_document(stuck.id, 5, stuck.file_size)
        .await
        .unwrap();
    assert!(outcome.queue_id().is_some(), "{outcome:?}");
    assert_eq!(ocr_status(&ctx, stuck.id).await.as_deref(), Some("pending"));

    // 'processing' with a running job is genuine and must be left alone.
    sql(
        &ctx,
        "UPDATE ocr_queue SET status = 'processing', started_at = NOW() WHERE document_id = $1",
        stuck.id,
    )
    .await;
    sql(
        &ctx,
        "UPDATE documents SET ocr_status = 'processing' WHERE id = $1",
        stuck.id,
    )
    .await;
    let outcome = queue
        .enqueue_document(stuck.id, 5, stuck.file_size)
        .await
        .unwrap();
    assert_eq!(outcome, EnqueueOutcome::AlreadyQueued);
    assert_eq!(
        ocr_status(&ctx, stuck.id).await.as_deref(),
        Some("processing")
    );
}

#[tokio::test]
async fn enqueue_retires_a_pending_job_with_no_attempts_left() {
    // Such a job is never dequeued but would hold the unique index forever.
    let ctx = TestContext::new().await;
    let (alice, _) = user(&ctx).await;
    let doc = insert(
        &ctx,
        document_helpers::create_document_with_ocr_error(alice),
    )
    .await;
    sql(
        &ctx,
        "INSERT INTO ocr_queue (document_id, priority, file_size, status, attempts, max_attempts) \
         VALUES ($1, 5, 1, 'pending', 3, 3)",
        doc.id,
    )
    .await;

    let outcome = ctx
        .state()
        .queue_service
        .enqueue_document(doc.id, 5, 1)
        .await
        .unwrap();
    assert!(outcome.queue_id().is_some(), "{outcome:?}");
    assert_eq!(
        jobs(&ctx, doc.id).await,
        vec![("failed".to_string(), 3), ("pending".to_string(), 0)]
    );
}

#[tokio::test]
async fn stale_recovery_fails_jobs_with_no_attempts_left() {
    let ctx = TestContext::new().await;
    let (alice, _) = user(&ctx).await;
    let spent = insert(
        &ctx,
        document_helpers::create_document_with_ocr_error(alice),
    )
    .await;
    let retryable = insert(
        &ctx,
        document_helpers::create_document_with_ocr_error(alice),
    )
    .await;
    for (doc, attempts) in [(&spent, 3), (&retryable, 1)] {
        sqlx::query(
            "INSERT INTO ocr_queue (document_id, priority, file_size, status, attempts, max_attempts, started_at) \
             VALUES ($1, 5, 1, 'processing', $2, 3, NOW() - INTERVAL '1 hour')",
        )
        .bind(doc.id)
        .bind(attempts)
        .execute(&ctx.state().db.pool)
        .await
        .unwrap();
    }

    ctx.state()
        .queue_service
        .recover_stale_items(10)
        .await
        .unwrap();
    assert_eq!(jobs(&ctx, spent.id).await, vec![("failed".to_string(), 3)]);
    assert_eq!(
        jobs(&ctx, retryable.id).await,
        vec![("pending".to_string(), 1)]
    );
}

#[tokio::test]
async fn batch_enqueue_skips_documents_that_are_already_queued() {
    let ctx = TestContext::new().await;
    let (alice, _) = user(&ctx).await;
    let queued = insert(
        &ctx,
        document_helpers::create_document_with_ocr_error(alice),
    )
    .await;
    let fresh = insert(
        &ctx,
        document_helpers::create_document_with_ocr_error(alice),
    )
    .await;
    let queue = &ctx.state().queue_service;
    let _ = queue.enqueue_document(queued.id, 5, 1).await.unwrap();

    let ids = queue
        .enqueue_documents_batch(vec![(queued.id, 5, 1), (fresh.id, 5, 1)])
        .await
        .expect("an already queued document must not fail the batch");
    assert_eq!(ids.len(), 1);
    assert_eq!(active_jobs(&ctx, queued.id).await, 1);
    assert_eq!(active_jobs(&ctx, fresh.id).await, 1);
}

#[tokio::test]
async fn rest_bulk_retry_of_an_already_queued_document_succeeds_without_a_second_job() {
    let ctx = TestContext::new().await;
    let (alice, token) = user(&ctx).await;
    let doc = insert(
        &ctx,
        document_helpers::create_document_with_ocr_error(alice),
    )
    .await;
    let _ = ctx
        .state()
        .queue_service
        .enqueue_document(doc.id, 5, 1)
        .await
        .unwrap();

    let body: Value = serde_json::from_slice(
        &ctx.auth_helper()
            .make_authenticated_request(
                "POST",
                "/api/documents/ocr/retry/bulk",
                Some(json!({ "mode": "specific", "document_ids": [doc.id] })),
                &token,
            )
            .await,
    )
    .expect("bulk retry must return JSON");
    assert_eq!(body["success"], json!(true), "{body}");
    assert_eq!(body["matched_count"], json!(1), "{body}");
    assert_eq!(
        body["queued_count"],
        json!(0),
        "nothing new was queued: {body}"
    );
    assert_eq!(body["documents"][0]["queue_id"], Value::Null, "{body}");
    assert_eq!(active_jobs(&ctx, doc.id).await, 1);
}

#[tokio::test]
async fn retry_service_counts_only_documents_it_actually_queued() {
    let ctx = TestContext::new().await;
    let (alice, _) = user(&ctx).await;
    let queued = insert(
        &ctx,
        document_helpers::create_document_with_ocr_error(alice),
    )
    .await;
    let fresh = insert(
        &ctx,
        document_helpers::create_document_with_ocr_error(alice),
    )
    .await;
    let _ = ctx
        .state()
        .queue_service
        .enqueue_document(queued.id, 5, 1)
        .await
        .unwrap();

    let result = OcrRetryService::new(ctx.state().clone())
        .retry_specific_documents(alice, vec![queued.id, fresh.id], None)
        .await
        .unwrap();
    assert_eq!(result.matched_count, 2);
    assert_eq!(result.queued_count, 1);
    assert_eq!(active_jobs(&ctx, queued.id).await, 1);
    assert_eq!(active_jobs(&ctx, fresh.id).await, 1);
}

async fn insert_job(
    ctx: &TestContext,
    document_id: Uuid,
    status: &str,
    attempts: i32,
    max_attempts: i32,
) {
    sqlx::query(
        "INSERT INTO ocr_queue (document_id, priority, file_size, status, attempts, max_attempts) \
         VALUES ($1, 5, 1, $2, $3, $4)",
    )
    .bind(document_id)
    .bind(status)
    .bind(attempts)
    .bind(max_attempts)
    .execute(&ctx.state().db.pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn a_pending_job_with_no_attempts_left_is_not_active() {
    // No worker claims it, so reporting it as active would promise OCR
    // that never happens.
    let ctx = TestContext::new().await;
    let (alice, _) = user(&ctx).await;
    let doc = insert(
        &ctx,
        document_helpers::create_document_with_ocr_error(alice),
    )
    .await;
    insert_job(&ctx, doc.id, "pending", 3, 3).await;
    let queue = &ctx.state().queue_service;

    assert!(!queue.has_active_job(doc.id).await.unwrap());
    assert!(
        queue.dequeue().await.unwrap().is_none(),
        "the worker must not claim it either"
    );
}

#[tokio::test]
async fn claimable_and_running_jobs_are_active() {
    let ctx = TestContext::new().await;
    let (alice, _) = user(&ctx).await;
    let queue = &ctx.state().queue_service;
    for (status, attempts) in [("pending", 2), ("processing", 3)] {
        let doc = insert(
            &ctx,
            document_helpers::create_document_with_ocr_error(alice),
        )
        .await;
        insert_job(&ctx, doc.id, status, attempts, 3).await;
        assert!(
            queue.has_active_job(doc.id).await.unwrap(),
            "{status} with {attempts}/3 attempts"
        );
    }
}

#[tokio::test]
async fn has_active_job_agrees_with_what_the_worker_claims() {
    // `has_active_job` and `dequeue` share one predicate; this pins that a
    // job is active exactly when a worker will claim it or is running it.
    let ctx = TestContext::new().await;
    let (alice, _) = user(&ctx).await;
    let queue = &ctx.state().queue_service;
    let statuses = ["pending", "processing", "failed", "completed"];
    // (attempts, max_attempts); the table's check constraint keeps
    // attempts <= max_attempts.
    let attempts = [(0, 3), (2, 3), (3, 3), (0, 0), (0, 1), (1, 1)];
    for status in statuses {
        for (attempts, max_attempts) in attempts {
            let case = format!("{status} with {attempts}/{max_attempts} attempts");
            sqlx::query("DELETE FROM ocr_queue")
                .execute(&ctx.state().db.pool)
                .await
                .unwrap();
            let doc = insert(
                &ctx,
                document_helpers::create_document_with_ocr_error(alice),
            )
            .await;
            insert_job(&ctx, doc.id, status, attempts, max_attempts).await;

            let active = queue.has_active_job(doc.id).await.unwrap();
            let expected =
                status == "processing" || (status == "pending" && attempts < max_attempts);
            assert_eq!(active, expected, "{case}");

            let claimed = queue.dequeue().await.unwrap();
            if let Some(item) = &claimed {
                assert_eq!(item.document_id, doc.id, "{case}");
            }
            assert_eq!(
                active,
                claimed.is_some() || status == "processing",
                "{case}: active must mean claimable or running"
            );
            if claimed.is_some() {
                assert!(
                    queue.has_active_job(doc.id).await.unwrap(),
                    "{case}: a claimed job stays active while it runs"
                );
            }
        }
    }
}

#[tokio::test]
async fn mcp_ocr_text_for_a_job_with_no_attempts_left_points_to_retry() {
    // The job is still 'pending' but no worker will claim it: telling the
    // model to check back would have it poll a document that never changes.
    let ctx = TestContext::new().await;
    let (alice, token) = user(&ctx).await;
    let doc = insert(&ctx, document_helpers::create_document_without_ocr(alice)).await;
    insert_job(&ctx, doc.id, "pending", 3, 3).await;

    let ocr = ctx
        .mcp_call_tool(&token, "get_document_ocr_text", json!({ "id": doc.id }))
        .await;
    assert_eq!(ocr["isError"], json!(false), "{ocr}");
    let text = ocr["content"][0]["text"].as_str().unwrap_or_default();
    assert!(!text.to_lowercase().contains("check back"), "{ocr}");
    assert!(text.contains("retry_document_ocr"), "{ocr}");

    // The retry it points to retires the stuck job and queues one the
    // worker will run.
    let retry = ctx
        .mcp_call_tool(&token, "retry_document_ocr", json!({ "id": doc.id }))
        .await;
    assert_eq!(retry["isError"], json!(false), "{retry}");
    assert_eq!(retry["structuredContent"]["queued"], json!(true), "{retry}");
    assert!(ctx
        .state()
        .queue_service
        .has_active_job(doc.id)
        .await
        .unwrap());
    assert_eq!(
        jobs(&ctx, doc.id).await,
        vec![("failed".to_string(), 3), ("pending".to_string(), 0)]
    );
}

#[tokio::test]
async fn a_pending_job_with_null_attempt_limits_is_retired_not_left_blocking() {
    // The attempt columns are nullable. NULL makes the claimable predicate
    // NULL, so the worker never claims such a job; the enqueue must still
    // retire it rather than report the document as already queued.
    let ctx = TestContext::new().await;
    let (alice, _) = user(&ctx).await;
    let doc = insert(
        &ctx,
        document_helpers::create_document_with_ocr_error(alice),
    )
    .await;
    sql(
        &ctx,
        "INSERT INTO ocr_queue (document_id, priority, file_size, status, attempts, max_attempts) \
         VALUES ($1, 5, 1, 'pending', 0, NULL)",
        doc.id,
    )
    .await;
    let queue = &ctx.state().queue_service;
    assert!(!queue.has_active_job(doc.id).await.unwrap());

    let outcome = queue.enqueue_document(doc.id, 5, 1).await.unwrap();
    assert!(outcome.queue_id().is_some(), "{outcome:?}");
    assert!(queue.has_active_job(doc.id).await.unwrap());
    let statuses: Vec<String> = sqlx::query_scalar(
        "SELECT status FROM ocr_queue WHERE document_id = $1 ORDER BY created_at",
    )
    .bind(doc.id)
    .fetch_all(&ctx.state().db.pool)
    .await
    .unwrap();
    assert_eq!(statuses, vec!["failed".to_string(), "pending".to_string()]);
}
