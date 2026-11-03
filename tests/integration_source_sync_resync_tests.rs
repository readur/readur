//! Re-running a sync must not download files that are already synced: a
//! document with the same source path, size and modification time is
//! skipped before download.

use chrono::{DateTime, Duration, Utc};
use readur::models::{CreateSource, FileIngestionInfo, SourceType};
use readur::scheduling::source_sync::{SourceSyncService, SyncOutcome};
use readur::test_utils::TestContext;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

fn listed(name: &str, size: i64, modified: Option<DateTime<Utc>>) -> FileIngestionInfo {
    FileIngestionInfo {
        relative_path: format!("/docs/{name}"),
        full_path: format!("/remote/docs/{name}"),
        #[allow(deprecated)]
        path: format!("/docs/{name}"),
        name: name.to_string(),
        size,
        mime_type: "text/plain".into(),
        last_modified: modified,
        etag: name.to_string(),
        is_directory: false,
        created_at: None,
        permissions: None,
        owner: None,
        group: None,
        metadata: None,
    }
}

struct Harness {
    ctx: TestContext,
    user_id: Uuid,
    source_id: Uuid,
    downloads: Arc<AtomicUsize>,
}

async fn harness(dir: &std::path::Path) -> Harness {
    let ctx = TestContext::new().await;
    let admin = ctx.auth_helper().create_admin_user().await;
    let user_id = admin.user_response.id;
    let source = ctx
        .state()
        .db
        .create_source(
            user_id,
            &CreateSource {
                name: "resync".into(),
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
        .unwrap();
    Harness { ctx, user_id, source_id: source.id, downloads: Arc::new(AtomicUsize::new(0)) }
}

impl Harness {
    /// Runs the pipeline over `files`, every download returning `body` and
    /// bumping the download counter.
    async fn run(&self, files: Vec<FileIngestionInfo>, body: &'static [u8]) -> SyncOutcome {
        let folders = vec!["/docs".to_string()];
        let downloads = self.downloads.clone();
        SourceSyncService::new(self.ctx.state().clone())
            .run_pipeline(
                self.user_id,
                self.source_id,
                &folders,
                &["txt".to_string()],
                false,
                CancellationToken::new(),
                move |_| {
                    let files = files.clone();
                    async move { Ok(files) }
                },
                move |_| {
                    let downloads = downloads.clone();
                    async move {
                        downloads.fetch_add(1, Ordering::SeqCst);
                        Ok(body.to_vec())
                    }
                },
            )
            .await
            .unwrap()
    }

    async fn document_count(&self) -> i64 {
        self.ctx.state().db.count_documents_for_source(self.user_id, self.source_id).await.unwrap().0
    }
}

/// Same path, size and modification time: the second run downloads nothing.
#[tokio::test]
async fn unchanged_files_are_not_downloaded_again() {
    let dir = tempfile::tempdir().unwrap();
    let h = harness(dir.path()).await;
    let modified = Some(Utc::now() - Duration::hours(1));
    let body: &[u8] = b"hello world";
    let files = vec![listed("a.txt", body.len() as i64, modified), listed("b.txt", body.len() as i64, modified)];

    let first = h.run(files.clone(), body).await;
    assert_eq!(first.processed, 1, "{first:?}"); // b.txt is a content duplicate of a.txt
    assert_eq!(first.skipped, 1, "{first:?}");
    assert_eq!(h.downloads.load(Ordering::SeqCst), 2);
    assert_eq!(h.document_count().await, 1);

    let second = h.run(files, body).await;
    assert_eq!(second.processed, 0, "{second:?}");
    assert_eq!(second.unchanged, 1, "{second:?}");
    assert_eq!(second.failed, 0, "{second:?}");
    assert_eq!(h.downloads.load(Ordering::SeqCst), 3, "only the duplicate (never stored) is downloaded again");
    assert_eq!(h.document_count().await, 1);

    let _ = h.ctx.cleanup_and_close().await;
}

/// A changed size or modification time, or a missing one, downloads again.
#[tokio::test]
async fn changed_or_unknown_metadata_downloads_again() {
    let dir = tempfile::tempdir().unwrap();
    let h = harness(dir.path()).await;
    let modified = Some(Utc::now() - Duration::hours(1));
    let body: &[u8] = b"hello world";

    let first = h.run(vec![listed("a.txt", body.len() as i64, modified)], body).await;
    assert_eq!(first.processed, 1, "{first:?}");
    assert_eq!(h.downloads.load(Ordering::SeqCst), 1);

    // Newer modification time: downloaded (then skipped by content hash).
    let newer = Some(Utc::now());
    let run = h.run(vec![listed("a.txt", body.len() as i64, newer)], body).await;
    assert_eq!(run.unchanged, 0, "{run:?}");
    assert_eq!(h.downloads.load(Ordering::SeqCst), 2);

    // Different size: downloaded.
    let run = h.run(vec![listed("a.txt", 1, modified)], body).await;
    assert_eq!(run.unchanged, 0, "{run:?}");
    assert_eq!(h.downloads.load(Ordering::SeqCst), 3);

    // No modification time from the source: downloaded.
    let run = h.run(vec![listed("a.txt", body.len() as i64, None)], body).await;
    assert_eq!(run.unchanged, 0, "{run:?}");
    assert_eq!(h.downloads.load(Ordering::SeqCst), 4);

    let _ = h.ctx.cleanup_and_close().await;
}

/// The real LocalFolder path: a second sync of an unchanged directory ingests
/// nothing and keeps the document count.
#[tokio::test]
async fn second_local_folder_sync_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("one.txt"), b"one").unwrap();
    std::fs::create_dir_all(dir.path().join("nested")).unwrap();
    std::fs::write(dir.path().join("nested/two.txt"), b"two").unwrap();
    let h = harness(dir.path()).await;
    let service = SourceSyncService::new(h.ctx.state().clone());
    let source = h.ctx.state().db.get_source(h.user_id, h.source_id).await.unwrap().unwrap();

    let first = service.sync_source(&source, false).await.unwrap();
    assert_eq!(first.processed, 2, "{first:?}");
    assert_eq!(h.document_count().await, 2);

    let second = service.sync_source(&source, false).await.unwrap();
    assert_eq!(second.processed, 0, "{second:?}");
    assert_eq!(second.unchanged, 2, "{second:?}");
    assert_eq!(second.failed, 0, "{second:?}");
    assert_eq!(h.document_count().await, 2);

    let _ = h.ctx.cleanup_and_close().await;
}
