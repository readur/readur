//! OCR queue ordering through the real enqueue paths and `dequeue`: work a
//! user asked for (an upload, a single-document OCR retry) is claimed before
//! background ingestion of a similar size that was queued earlier, and
//! smaller files still go first.

use readur::models::{Document, User};
use readur::ocr::priority::priority_for_file_size;
use readur::services::documents::{retry_document_ocr, upload_document, UploadRequest};
use readur::test_utils::{document_helpers, TestConfigBuilder, TestContext};
use uuid::Uuid;

const MB: i64 = 1024 * 1024;

async fn user(ctx: &TestContext) -> User {
    let auth = ctx.auth_helper();
    let created = auth.create_test_user().await;
    ctx.state()
        .db
        .get_user_by_id(created.user_response.id)
        .await
        .unwrap()
        .unwrap()
}

/// A stored document of `size` bytes (as recorded; no file is needed to
/// queue it).
async fn document(ctx: &TestContext, owner: Uuid, size: i64) -> Document {
    let mut document = document_helpers::create_document_with_ocr_error(owner);
    document.file_hash = Some(Uuid::new_v4().simple().to_string());
    document.file_size = size;
    ctx.state().db.create_document(document).await.unwrap()
}

/// Queues `document` the way background ingestion (source sync, watch
/// folder, batch import) does.
async fn queue_as_background(ctx: &TestContext, document: &Document) {
    let _queued = ctx
        .state()
        .queue_service
        .enqueue_document(
            document.id,
            priority_for_file_size(document.file_size),
            document.file_size,
        )
        .await
        .unwrap();
}

async fn next_claimed(ctx: &TestContext) -> Uuid {
    ctx.state()
        .queue_service
        .dequeue()
        .await
        .unwrap()
        .expect("a job to claim")
        .document_id
}

#[tokio::test]
async fn single_retry_runs_ahead_of_earlier_background_work_of_the_same_size() {
    let ctx = TestContext::new().await;
    let owner = user(&ctx).await;
    for size in [100, 3 * MB, 8 * MB, 20 * MB, 80 * MB] {
        let background = document(&ctx, owner.id, size).await;
        let retried = document(&ctx, owner.id, size).await;
        queue_as_background(&ctx, &background).await;
        retry_document_ocr(ctx.state(), &owner, retried.id, None, None)
            .await
            .unwrap();

        assert_eq!(next_claimed(&ctx).await, retried.id, "{size} bytes");
        assert_eq!(next_claimed(&ctx).await, background.id, "{size} bytes");
    }
    let _ = ctx.cleanup_and_close().await;
}

#[tokio::test]
async fn upload_runs_ahead_of_earlier_background_work_of_the_same_size() {
    let uploads = tempfile::tempdir().unwrap();
    let ctx = TestContext::with_config(
        TestConfigBuilder::default().with_upload_path(uploads.path().to_str().unwrap()),
    )
    .await;
    let owner = user(&ctx).await;
    let data = b"an upload someone is waiting for".to_vec();
    let background = document(&ctx, owner.id, data.len() as i64).await;
    queue_as_background(&ctx, &background).await;

    let uploaded = upload_document(
        ctx.state(),
        &owner,
        UploadRequest {
            filename: "waiting.txt".into(),
            mime_type: "text/plain".into(),
            data,
            ..Default::default()
        },
    )
    .await
    .unwrap();

    assert_eq!(next_claimed(&ctx).await, uploaded.id);
    assert_eq!(next_claimed(&ctx).await, background.id);
    let _ = ctx.cleanup_and_close().await;
}

/// Size still dominates: a small background file runs before a user's
/// large one.
#[tokio::test]
async fn small_background_work_runs_ahead_of_a_large_retry() {
    let ctx = TestContext::new().await;
    let owner = user(&ctx).await;
    let large = document(&ctx, owner.id, 80 * MB).await;
    let small = document(&ctx, owner.id, 100).await;
    retry_document_ocr(ctx.state(), &owner, large.id, None, None)
        .await
        .unwrap();
    queue_as_background(&ctx, &small).await;

    assert_eq!(next_claimed(&ctx).await, small.id);
    assert_eq!(next_claimed(&ctx).await, large.id);
    let _ = ctx.cleanup_and_close().await;
}
