//! Per-file integrity in `SourceSyncService`: a download whose length does
//! not match the listing is a failure (retried next run), and a document
//! whose OCR enqueue fails is discarded so the next run retries it instead
//! of leaving it pending forever. If the row cannot be removed, the stored
//! file is kept so the surviving document stays openable.

use anyhow::anyhow;
use readur::models::{CreateSource, FileIngestionInfo, SourceType};
use readur::scheduling::source_sync::{SourceSyncService, SyncOutcome};
use readur::test_utils::TestContext;
use serde_json::json;
use readur::services::webdav::SyncProgress;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

fn listed(name: &str, size: i64) -> FileIngestionInfo {
    FileIngestionInfo {
        relative_path: format!("/{name}"),
        full_path: format!("/{name}"),
        #[allow(deprecated)]
        path: format!("/{name}"),
        name: name.to_string(),
        size,
        mime_type: "text/plain".into(),
        last_modified: None,
        etag: name.to_string(),
        is_directory: false,
        created_at: None,
        permissions: None,
        owner: None,
        group: None,
        metadata: None,
    }
}

async fn local_source(ctx: &TestContext, user_id: Uuid, dir: &std::path::Path) -> readur::models::Source {
    ctx.state()
        .db
        .create_source(
            user_id,
            &CreateSource {
                name: "integrity".into(),
                source_type: SourceType::LocalFolder,
                enabled: Some(true),
                config: json!({
                    "watch_folders": [dir.to_str().unwrap()],
                    "file_extensions": ["txt"],
                    "auto_sync": false,
                    "sync_interval_minutes": 60,
                    "recursive": true,
                    "follow_symlinks": false
                }),
            },
        )
        .await
        .unwrap()
}

/// Runs the pipeline over one listed file whose download returns `body`.
async fn sync_one(ctx: &TestContext, user_id: Uuid, source_id: Uuid, file: FileIngestionInfo, body: &'static [u8]) -> SyncOutcome {
    let folders = vec!["/".to_string()];
    SourceSyncService::new(ctx.state().clone())
        .run_pipeline(
            user_id,
            source_id,
            &folders,
            &["txt".to_string()],
            false,
            CancellationToken::new(),
            &Arc::new(SyncProgress::new()),
            move |_| {
                let file = file.clone();
                async move { Ok(vec![file].into()) }
            },
            |_| async move { Ok(body.to_vec()) },
        )
        .await
        .unwrap()
}

/// A body shorter or longer than the listed size is not ingested; the file
/// is counted as failed so the next run retries it.
#[tokio::test]
async fn download_length_must_match_the_listing() {
    let ctx = TestContext::new().await;
    let admin = ctx.auth_helper().create_admin_user().await;
    let dir = tempfile::tempdir().unwrap();
    let source = local_source(&ctx, admin.user_response.id, dir.path()).await;

    let outcome = sync_one(&ctx, admin.user_response.id, source.id, listed("short.txt", 100), b"only a few bytes").await;

    assert_eq!(outcome.failed, 1, "{outcome:?}");
    assert_eq!(outcome.processed, 0, "{outcome:?}");
    let first_error = outcome.first_error.clone().unwrap_or_default();
    assert!(first_error.contains("size mismatch"), "{first_error}");
    assert_eq!(ctx.state().db.count_documents_for_source(admin.user_response.id, source.id).await.unwrap().0, 0);

    let _ = ctx.cleanup_and_close().await;
}

/// A listing that does not know the size (0) cannot be checked, so the
/// download is accepted as it is.
#[tokio::test]
async fn unknown_listed_size_is_not_checked() {
    let ctx = TestContext::new().await;
    let admin = ctx.auth_helper().create_admin_user().await;
    let dir = tempfile::tempdir().unwrap();
    let source = local_source(&ctx, admin.user_response.id, dir.path()).await;

    let outcome = sync_one(&ctx, admin.user_response.id, source.id, listed("unsized.txt", 0), b"some content").await;

    assert_eq!(outcome.processed, 1, "{outcome:?}");
    assert_eq!(outcome.failed, 0, "{outcome:?}");

    let _ = ctx.cleanup_and_close().await;
}

/// When the OCR enqueue fails after the document was created, the document
/// is removed again and the file counted as failed; once the queue accepts
/// inserts again the next run ingests it.
#[tokio::test]
async fn failed_ocr_enqueue_discards_the_document_for_retry() {
    let ctx = TestContext::new().await;
    let admin = ctx.auth_helper().create_admin_user().await;
    let user_id = admin.user_response.id;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("scan.txt"), b"needs ocr").unwrap();
    let source = local_source(&ctx, user_id, dir.path()).await;
    let pool = ctx.state().db.get_pool();

    for statement in [
        "CREATE FUNCTION refuse_ocr_queue_insert() RETURNS trigger AS $$
         BEGIN RAISE EXCEPTION 'ocr_queue unavailable'; END $$ LANGUAGE plpgsql",
        "CREATE TRIGGER refuse_ocr_queue_insert BEFORE INSERT ON ocr_queue
         FOR EACH ROW EXECUTE FUNCTION refuse_ocr_queue_insert()",
    ] {
        sqlx::query(statement).execute(pool).await.unwrap();
    }

    let service = SourceSyncService::new(ctx.state().clone());
    let outcome = service.sync_source(&source, true).await.unwrap();
    assert_eq!(outcome.failed, 1, "{outcome:?}");
    assert_eq!(outcome.processed, 0, "{outcome:?}");
    assert!(outcome.first_error.clone().unwrap_or_default().contains("OCR"), "{outcome:?}");
    assert_eq!(ctx.state().db.count_documents_for_source(user_id, source.id).await.unwrap().0, 0);

    for statement in ["DROP TRIGGER refuse_ocr_queue_insert ON ocr_queue", "DROP FUNCTION refuse_ocr_queue_insert()"] {
        sqlx::query(statement).execute(pool).await.unwrap();
    }

    let outcome = service.sync_source(&source, true).await.unwrap();
    assert_eq!(outcome.processed, 1, "{outcome:?}");
    assert_eq!(outcome.failed, 0, "{outcome:?}");
    assert_eq!(ctx.state().db.count_documents_for_source(user_id, source.id).await.unwrap().0, 1);
    let queued: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ocr_queue").fetch_one(pool).await.unwrap();
    assert_eq!(queued, 1);

    let _ = ctx.cleanup_and_close().await;
}

/// When the document row cannot be deleted after a failed OCR enqueue, the
/// stored file is left alone: the document is still openable and nothing is
/// removed that the surviving row points at.
#[tokio::test]
async fn failed_row_delete_keeps_the_stored_file() {
    let ctx = TestContext::new().await;
    let admin = ctx.auth_helper().create_admin_user().await;
    let user_id = admin.user_response.id;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("scan.txt"), b"needs ocr").unwrap();
    let source = local_source(&ctx, user_id, dir.path()).await;
    let pool = ctx.state().db.get_pool();

    for statement in [
        "CREATE FUNCTION refuse_ocr_queue_insert() RETURNS trigger AS $$
         BEGIN RAISE EXCEPTION 'ocr_queue unavailable'; END $$ LANGUAGE plpgsql",
        "CREATE TRIGGER refuse_ocr_queue_insert BEFORE INSERT ON ocr_queue
         FOR EACH ROW EXECUTE FUNCTION refuse_ocr_queue_insert()",
        "CREATE FUNCTION refuse_document_delete() RETURNS trigger AS $$
         BEGIN RAISE EXCEPTION 'documents unavailable'; END $$ LANGUAGE plpgsql",
        "CREATE TRIGGER refuse_document_delete BEFORE DELETE ON documents
         FOR EACH ROW EXECUTE FUNCTION refuse_document_delete()",
    ] {
        sqlx::query(statement).execute(pool).await.unwrap();
    }

    let outcome = SourceSyncService::new(ctx.state().clone()).sync_source(&source, true).await.unwrap();
    assert_eq!(outcome.failed, 1, "{outcome:?}");

    let file_path: String = sqlx::query_scalar("SELECT file_path FROM documents WHERE source_id = $1")
        .bind(source.id)
        .fetch_one(pool)
        .await
        .expect("the document row survives the failed delete");
    assert!(
        std::path::Path::new(&file_path).exists(),
        "stored file {file_path} must not be deleted while its row exists"
    );

    for statement in [
        "DROP TRIGGER refuse_document_delete ON documents",
        "DROP FUNCTION refuse_document_delete()",
        "DROP TRIGGER refuse_ocr_queue_insert ON ocr_queue",
        "DROP FUNCTION refuse_ocr_queue_insert()",
    ] {
        sqlx::query(statement).execute(pool).await.unwrap();
    }

    let _ = ctx.cleanup_and_close().await;
}
