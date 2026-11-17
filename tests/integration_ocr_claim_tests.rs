//! OCR job ownership: every dequeue writes a fresh claim, every worker write
//! is guarded by it, live workers heartbeat so stale recovery leaves them
//! alone, a cancel stops a running worker and frees its slot, and the
//! preprocessed image is removed on every way out of a job.

use std::sync::Arc;
use std::time::Duration;

use anyhow::anyhow;
use tempfile::TempDir;
use tokio::sync::Semaphore;
use uuid::Uuid;

use readur::db_guardrails_simple::{DocumentTransactionManager, OcrWrite, OcrWriteOutcome};
use readur::models::Document;
use readur::ocr::enhanced::{EnhancedOcrService, OcrResult};
use readur::ocr::queue::{ClaimLoss, EnqueueOutcome, JobRun, OcrQueueItem, OcrQueueService};
use readur::services::file_service::FileService;
use readur::storage::{factory::create_storage_backend, StorageConfig};
use readur::test_utils::{document_helpers, TestContext};

const TEXT: &str = "The quick brown fox jumps over the lazy dog near the river bank today";

async fn owner(ctx: &TestContext) -> Uuid {
    ctx.auth_helper().create_test_user().await.user_response.id
}

/// A pending text document whose file holds [`TEXT`].
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

fn queue(ctx: &TestContext) -> &OcrQueueService {
    &ctx.state().queue_service
}

async fn enqueue(ctx: &TestContext, doc: &Document) {
    let outcome = queue(ctx)
        .enqueue_document(doc.id, 5, doc.file_size)
        .await
        .unwrap();
    assert!(matches!(outcome, EnqueueOutcome::Queued(_)), "{outcome:?}");
}

async fn claim(ctx: &TestContext, doc: &Document) -> OcrQueueItem {
    let item = queue(ctx)
        .dequeue()
        .await
        .unwrap()
        .expect("a claimable job");
    assert_eq!(item.document_id, doc.id);
    assert!(item.claim_id.is_some(), "a dequeued job carries its claim");
    item
}

/// Moves the job's start and heartbeat `minutes` into the past.
async fn age(ctx: &TestContext, job: Uuid, started: i32, heartbeat: i32) {
    sqlx::query(
        "UPDATE ocr_queue SET started_at = NOW() - INTERVAL '1 minute' * $2, \
         heartbeat_at = NOW() - INTERVAL '1 minute' * $3 WHERE id = $1",
    )
    .bind(job)
    .bind(started)
    .bind(heartbeat)
    .execute(&ctx.state().db.pool)
    .await
    .unwrap();
}

/// `(status, claim_id, progress_current)` of a job, None when it is gone.
async fn job(ctx: &TestContext, id: Uuid) -> Option<(String, Option<Uuid>, Option<i32>)> {
    sqlx::query_as("SELECT status, claim_id, progress_current FROM ocr_queue WHERE id = $1")
        .bind(id)
        .fetch_optional(&ctx.state().db.pool)
        .await
        .unwrap()
}

/// `(ocr_status, ocr_text)` of a document.
async fn doc_state(ctx: &TestContext, id: Uuid) -> (Option<String>, Option<String>) {
    sqlx::query_as("SELECT ocr_status, ocr_text FROM documents WHERE id = $1")
        .bind(id)
        .fetch_one(&ctx.state().db.pool)
        .await
        .unwrap()
}

fn good_result(image: Option<String>) -> OcrResult {
    OcrResult {
        text: TEXT.to_string(),
        confidence: 95.0,
        processing_time_ms: 5,
        word_count: 14,
        preprocessing_applied: vec!["test".to_string()],
        processed_image_path: image,
    }
}

#[tokio::test]
async fn a_slow_job_past_the_stale_window_is_not_reset_and_stores_its_result() {
    let ctx = TestContext::new().await;
    let dir = TempDir::new().unwrap();
    let doc = text_document(&ctx, &dir, owner(&ctx).await).await;
    enqueue(&ctx, &doc).await;
    let item = claim(&ctx, &doc).await;
    // Claimed long ago (waited for a slot, then a slow run), heartbeating now.
    age(&ctx, item.id, 30, 0).await;

    assert_eq!(queue(&ctx).recover_stale_items(10).await.unwrap(), 0);
    queue(&ctx)
        .process_item(item, &ocr_service(&dir).await)
        .await
        .unwrap();
    let state = doc_state(&ctx, doc.id).await;
    assert_eq!(state.0.as_deref(), Some("completed"), "{state:?}");
    assert_eq!(state.1.as_deref(), Some(TEXT));
}

#[tokio::test]
async fn a_running_job_keeps_heartbeating_while_its_work_runs() {
    let ctx = TestContext::new().await;
    let dir = TempDir::new().unwrap();
    let doc = text_document(&ctx, &dir, owner(&ctx).await).await;
    enqueue(&ctx, &doc).await;
    let item = claim(&ctx, &doc).await;
    let fast = queue(&ctx)
        .clone()
        .with_heartbeat_interval(Duration::from_millis(50));

    let work = async {
        // Silent for longer than the stale window, by the clock...
        age(&ctx, item.id, 30, 30).await;
        // ...until the timer heartbeats again.
        tokio::time::sleep(Duration::from_millis(400)).await;
        fast.recover_stale_items(10).await.unwrap()
    };
    match fast.run_claimed(item.claim().unwrap(), work).await.unwrap() {
        JobRun::Finished(recovered) => assert_eq!(recovered, 0, "a live job was reset"),
        JobRun::Stopped(loss) => panic!("stopped: {loss:?}"),
    }
}

#[tokio::test]
async fn a_dead_workers_job_is_recovered() {
    let ctx = TestContext::new().await;
    let dir = TempDir::new().unwrap();
    let doc = text_document(&ctx, &dir, owner(&ctx).await).await;
    enqueue(&ctx, &doc).await;
    let item = claim(&ctx, &doc).await;
    age(&ctx, item.id, 30, 11).await;

    assert_eq!(queue(&ctx).recover_stale_items(10).await.unwrap(), 1);
    let (status, claim_id, _) = job(&ctx, item.id).await.unwrap();
    assert_eq!(status, "pending");
    assert_eq!(claim_id, None, "the dead worker's claim is cleared");
    assert!(!queue(&ctx).heartbeat(&item.claim().unwrap()).await.unwrap());
}

#[tokio::test]
async fn a_worker_whose_job_was_reclaimed_writes_nothing() {
    let ctx = TestContext::new().await;
    let dir = TempDir::new().unwrap();
    let doc = text_document(&ctx, &dir, owner(&ctx).await).await;
    enqueue(&ctx, &doc).await;
    let a = claim(&ctx, &doc).await;
    age(&ctx, a.id, 30, 30).await;
    assert_eq!(queue(&ctx).recover_stale_items(10).await.unwrap(), 1);
    let b = claim(&ctx, &doc).await;
    assert_eq!(a.id, b.id, "the same job, claimed again");
    assert_ne!(a.claim_id, b.claim_id);
    let ocr = ocr_service(&dir).await;
    let b_row = || async { job(&ctx, b.id).await };
    let before = b_row().await;
    assert_eq!(before.as_ref().unwrap().1, b.claim_id);

    // A's success write.
    let writer = DocumentTransactionManager::new(ctx.state().db.pool.clone());
    let write = OcrWrite {
        document_id: doc.id,
        expected_filename: &doc.filename,
        ocr_text: TEXT,
        confidence: 99.0,
        word_count: 14,
        processing_time_ms: 5,
    };
    assert_eq!(
        writer
            .update_ocr_for_job(&a.claim().unwrap(), &write)
            .await
            .unwrap(),
        OcrWriteOutcome::ClaimLost
    );
    // A's whole run, success and failure.
    queue(&ctx).process_item(a.clone(), &ocr).await.unwrap();
    queue(&ctx)
        .finish_extraction(&a, &ocr, Err(anyhow!("tesseract crashed")))
        .await
        .unwrap();
    // A's progress.
    queue(&ctx)
        .update_progress(&a.claim().unwrap(), 9, 9)
        .await
        .unwrap();

    assert_eq!(b_row().await, before, "B's job row is untouched");
    let state = doc_state(&ctx, doc.id).await;
    assert_eq!(state.0.as_deref(), Some("processing"), "{state:?}");
    assert_eq!(state.1, None);

    queue(&ctx).process_item(b, &ocr).await.unwrap();
    assert_eq!(
        doc_state(&ctx, doc.id).await.0.as_deref(),
        Some("completed")
    );
}

#[tokio::test]
async fn cancelling_a_running_job_frees_its_worker_slot_promptly() {
    let ctx = TestContext::new().await;
    let dir = TempDir::new().unwrap();
    let alice = owner(&ctx).await;
    let doc = text_document(&ctx, &dir, alice).await;
    let next = text_document(&ctx, &dir, alice).await;
    enqueue(&ctx, &doc).await;
    let slots = Arc::new(Semaphore::new(1));

    // The worker loop: a slot, then a claim, then a run holding the slot.
    let permit = slots.clone().acquire_owned().await.unwrap();
    let item = claim(&ctx, &doc).await;
    enqueue(&ctx, &next).await;
    let service = Arc::clone(&ctx.state().queue_service);
    let running = tokio::spawn(async move {
        let run = service
            .run_claimed(
                item.claim().unwrap(),
                tokio::time::sleep(Duration::from_secs(120)),
            )
            .await;
        drop(permit);
        run
    });
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(slots.available_permits(), 0);

    assert!(queue(&ctx)
        .cancel_document(doc.id)
        .await
        .unwrap()
        .cancelled());
    let run = tokio::time::timeout(Duration::from_secs(5), running)
        .await
        .expect("the cancelled job stops long before its work would finish")
        .unwrap()
        .unwrap();
    assert!(
        matches!(run, JobRun::Stopped(ClaimLoss::Cancelled)),
        "{run:?}"
    );
    let _slot = slots.try_acquire().expect("the worker slot was released");
    let claimed = queue(&ctx).dequeue().await.unwrap().expect("the next job");
    assert_eq!(claimed.document_id, next.id);
}

#[tokio::test]
async fn a_job_cancelled_by_another_process_stops_at_its_next_heartbeat() {
    let ctx = TestContext::new().await;
    let dir = TempDir::new().unwrap();
    let alice = owner(&ctx).await;
    let fast = queue(&ctx)
        .clone()
        .with_heartbeat_interval(Duration::from_millis(50));

    // Deleted (a cancel in another process: no token fires here) and
    // reclaimed (another claim written over this one).
    for (sql, expected) in [
        ("DELETE FROM ocr_queue WHERE id = $1", ClaimLoss::Cancelled),
        (
            "UPDATE ocr_queue SET claim_id = gen_random_uuid() WHERE id = $1",
            ClaimLoss::Reclaimed,
        ),
    ] {
        let doc = text_document(&ctx, &dir, alice).await;
        enqueue(&ctx, &doc).await;
        let item = claim(&ctx, &doc).await;
        let pool = ctx.state().db.pool.clone();
        let work = async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            sqlx::query(sql).bind(item.id).execute(&pool).await.unwrap();
            tokio::time::sleep(Duration::from_secs(120)).await;
        };
        let run = tokio::time::timeout(
            Duration::from_secs(5),
            fast.run_claimed(item.claim().unwrap(), work),
        )
        .await
        .expect("the run stops at the next heartbeat")
        .unwrap();
        assert!(
            matches!(run, JobRun::Stopped(loss) if loss == expected),
            "{run:?}"
        );
    }
}

#[tokio::test]
async fn progress_of_a_cancelled_job_never_reaches_its_retry() {
    let ctx = TestContext::new().await;
    let dir = TempDir::new().unwrap();
    let doc = text_document(&ctx, &dir, owner(&ctx).await).await;
    enqueue(&ctx, &doc).await;
    let cancelled = claim(&ctx, &doc).await;
    let _ = queue(&ctx).cancel_document(doc.id).await.unwrap();
    enqueue(&ctx, &doc).await;
    let retry = claim(&ctx, &doc).await;

    queue(&ctx)
        .update_progress(&cancelled.claim().unwrap(), 5, 10)
        .await
        .unwrap();
    assert_eq!(job(&ctx, retry.id).await.unwrap().2, Some(0));
    queue(&ctx)
        .update_progress(&retry.claim().unwrap(), 2, 10)
        .await
        .unwrap();
    assert_eq!(job(&ctx, retry.id).await.unwrap().2, Some(2));
}

/// Claims a fresh job for a new document and returns it with a temporary
/// "preprocessed image" file.
async fn job_with_image(
    ctx: &TestContext,
    dir: &TempDir,
    owner: Uuid,
) -> (Document, OcrQueueItem, std::path::PathBuf) {
    // Earlier cases put their failed attempts back in the queue.
    sqlx::query("DELETE FROM ocr_queue WHERE status = 'pending'")
        .execute(&ctx.state().db.pool)
        .await
        .unwrap();
    let doc = text_document(ctx, dir, owner).await;
    enqueue(ctx, &doc).await;
    let item = claim(ctx, &doc).await;
    let image = dir
        .path()
        .join(format!("processed_{}.png", Uuid::new_v4().simple()));
    std::fs::write(&image, b"not really a png").unwrap();
    (doc, item, image)
}

#[tokio::test]
async fn the_preprocessed_image_is_removed_on_every_way_out() {
    let ctx = TestContext::new().await;
    let dir = TempDir::new().unwrap();
    let alice = owner(&ctx).await;
    let ocr = ocr_service(&dir).await;
    let path_of = |image: &std::path::Path| Some(image.to_string_lossy().into_owned());

    // Quality failure.
    let (doc, item, image) = job_with_image(&ctx, &dir, alice).await;
    let mut garbage = good_result(path_of(&image));
    garbage.confidence = 1.0;
    queue(&ctx)
        .finish_extraction(&item, &ocr, Ok(garbage))
        .await
        .unwrap();
    assert!(!image.exists(), "quality failure leaked the image");
    assert_eq!(doc_state(&ctx, doc.id).await.0.as_deref(), Some("failed"));

    // Rejected: the document completed meanwhile.
    let (doc, item, image) = job_with_image(&ctx, &dir, alice).await;
    sqlx::query("UPDATE documents SET ocr_status = 'completed' WHERE id = $1")
        .bind(doc.id)
        .execute(&ctx.state().db.pool)
        .await
        .unwrap();
    queue(&ctx)
        .finish_extraction(&item, &ocr, Ok(good_result(path_of(&image))))
        .await
        .unwrap();
    assert!(!image.exists(), "a rejected write leaked the image");

    // Cancelled while it ran.
    let (doc, item, image) = job_with_image(&ctx, &dir, alice).await;
    let _ = queue(&ctx).cancel_document(doc.id).await.unwrap();
    queue(&ctx)
        .finish_extraction(&item, &ocr, Ok(good_result(path_of(&image))))
        .await
        .unwrap();
    assert!(!image.exists(), "a cancelled job leaked the image");

    // Reclaimed by another worker, failing.
    let (doc, item, image) = job_with_image(&ctx, &dir, alice).await;
    let mut empty = good_result(path_of(&image));
    empty.text.clear();
    empty.word_count = 0;
    sqlx::query("UPDATE ocr_queue SET claim_id = gen_random_uuid() WHERE id = $1")
        .bind(item.id)
        .execute(&ctx.state().db.pool)
        .await
        .unwrap();
    queue(&ctx)
        .finish_extraction(&item, &ocr, Ok(empty))
        .await
        .unwrap();
    assert!(!image.exists(), "a lost claim leaked the image");
    assert_eq!(
        doc_state(&ctx, doc.id).await.0.as_deref(),
        Some("processing")
    );

    // Success.
    let (doc, item, image) = job_with_image(&ctx, &dir, alice).await;
    queue(&ctx)
        .finish_extraction(&item, &ocr, Ok(good_result(path_of(&image))))
        .await
        .unwrap();
    assert!(!image.exists(), "success leaked the image");
    assert_eq!(
        doc_state(&ctx, doc.id).await.0.as_deref(),
        Some("completed")
    );
}
