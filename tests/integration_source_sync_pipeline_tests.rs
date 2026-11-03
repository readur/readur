//! End-to-end tests for `SourceSyncService`: a LocalFolder source is synced
//! against a temporary directory and the documents it produces are checked in
//! the database. These are the only tests that ingest files through the
//! generic sync pipeline.

use readur::models::{CreateSource, SourceStatus, SourceType};
use readur::scheduling::source_sync::{SourceSyncService, SyncOutcome};
use readur::test_utils::TestContext;
use serde_json::json;
use std::path::Path;
use uuid::Uuid;

struct Fixture {
    ctx: TestContext,
    user_id: Uuid,
    source_id: Uuid,
    _dir: tempfile::TempDir,
}

async fn fixture(extensions: &[&str], files: &[(&str, &[u8])]) -> Fixture {
    let ctx = TestContext::new().await;
    let admin = ctx.auth_helper().create_admin_user().await;
    let user_id = admin.user_response.id;
    let dir = tempfile::tempdir().unwrap();
    for (relative, content) in files {
        write_file(dir.path(), relative, content);
    }
    let source = ctx
        .state()
        .db
        .create_source(
            user_id,
            &CreateSource {
                name: "pipeline".into(),
                source_type: SourceType::LocalFolder,
                enabled: Some(true),
                config: json!({
                    "watch_folders": [dir.path().to_str().unwrap()],
                    "file_extensions": extensions,
                    "auto_sync": false,
                    "sync_interval_minutes": 60,
                    "recursive": true,
                    "follow_symlinks": false
                }),
            },
        )
        .await
        .unwrap();
    Fixture { ctx, user_id, source_id: source.id, _dir: dir }
}

fn write_file(root: &Path, relative: &str, content: &[u8]) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
}

impl Fixture {
    async fn sync(&self) -> anyhow::Result<SyncOutcome> {
        let source = self.ctx.state().db.get_source(self.user_id, self.source_id).await.unwrap().unwrap();
        SourceSyncService::new(self.ctx.state().clone()).sync_source(&source, false).await
    }

    async fn document_count(&self) -> i64 {
        self.ctx.state().db.count_documents_for_source(self.user_id, self.source_id).await.unwrap().0
    }

    async fn source(&self) -> readur::models::Source {
        self.ctx.state().db.get_source(self.user_id, self.source_id).await.unwrap().unwrap()
    }
}

/// Files in nested sub-directories are discovered once and ingested; files
/// outside the extension allow-list and directories are not.
#[tokio::test]
async fn nested_local_folder_files_are_ingested() {
    let f = fixture(
        &["txt", "pdf"],
        &[
            ("root.txt", b"root file"),
            ("a/first.txt", b"first nested file"),
            ("a/b/second.pdf", b"%PDF-1.4 second nested file"),
            ("a/b/ignored.bin", b"not an allowed extension"),
        ],
    )
    .await;

    let outcome = f.sync().await.unwrap();

    assert_eq!(outcome.discovered, 3, "{outcome:?}");
    assert_eq!(outcome.processed, 3, "{outcome:?}");
    assert_eq!(outcome.failed, 0, "{outcome:?}");
    assert_eq!(outcome.folders_failed, 0, "{outcome:?}");
    assert_eq!(f.document_count().await, 3);

    let source = f.source().await;
    assert_eq!(source.status, SourceStatus::Idle);
    assert_eq!(source.last_error, None);
    assert_eq!(source.total_files_pending, 0);

    let _ = f.ctx.cleanup_and_close().await;
}

/// A file larger than `max_file_size_mb` is skipped before download and
/// counted as skipped rather than failed.
#[tokio::test]
async fn oversized_files_are_skipped_not_downloaded() {
    let f = fixture(&["txt"], &[("small.txt", b"small")]).await;
    let max_bytes = f.ctx.state().config.max_file_size_mb as usize * 1024 * 1024;
    write_file(f._dir.path(), "huge.txt", &vec![b'x'; max_bytes + 1]);

    let outcome = f.sync().await.unwrap();

    assert_eq!(outcome.discovered, 1, "{outcome:?}");
    assert_eq!(outcome.processed, 1, "{outcome:?}");
    assert_eq!(outcome.skipped, 1, "{outcome:?}");
    assert_eq!(outcome.failed, 0, "{outcome:?}");
    assert_eq!(f.document_count().await, 1);
    assert_eq!(f.source().await.status, SourceStatus::Idle);

    let _ = f.ctx.cleanup_and_close().await;
}
