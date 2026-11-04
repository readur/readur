//! Failure accounting for `SourceSyncService`: listing and file failures are
//! counted, surfaced on the source, and never reported as a clean sync.

use anyhow::anyhow;
use readur::models::{CreateSource, FileIngestionInfo, SourceStatus, SourceType};
use readur::scheduling::source_sync::{FolderDiscovery, SourceSyncService};
use readur::test_utils::TestContext;
use readur::utils::sync_cancelled::cancelled;
use serde_json::json;
use std::os::unix::fs::PermissionsExt;
use readur::services::webdav::SyncProgress;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

fn listing_of(names: &[&str]) -> Vec<FileIngestionInfo> {
    names
        .iter()
        .map(|name| FileIngestionInfo {
            relative_path: format!("/{name}"),
            full_path: format!("/{name}"),
            #[allow(deprecated)]
            path: format!("/{name}"),
            name: name.to_string(),
            size: 4,
            mime_type: "text/plain".into(),
            last_modified: None,
            etag: name.to_string(),
            is_directory: false,
            created_at: None,
            permissions: None,
            owner: None,
            group: None,
            metadata: None,
        })
        .collect()
}

/// When no watch folder can be listed the run fails outright instead of
/// reporting an empty, successful sync.
#[tokio::test]
async fn listing_failure_in_every_folder_fails_the_run() {
    let ctx = TestContext::new().await;
    let service = SourceSyncService::new(ctx.state().clone());
    let folders = vec!["/a".to_string(), "/b".to_string()];

    let err = service
        .run_pipeline(
            Uuid::new_v4(),
            Uuid::new_v4(),
            &folders,
            &["txt".to_string()],
            false,
            CancellationToken::new(),
            &Arc::new(SyncProgress::new()),
            |folder| async move { Err(anyhow!("listing {folder} refused")) },
            |_| async { Ok(Vec::new()) },
        )
        .await
        .unwrap_err();

    assert!(err.to_string().contains("listing /a refused"), "{err}");
    let _ = ctx.cleanup_and_close().await;
}

/// One unreadable folder is counted and the others still sync.
#[tokio::test]
async fn listing_failure_in_one_folder_is_counted() {
    let ctx = TestContext::new().await;
    let service = SourceSyncService::new(ctx.state().clone());
    let folders = vec!["/bad".to_string(), "/good".to_string()];

    let outcome = service
        .run_pipeline(
            Uuid::new_v4(),
            Uuid::new_v4(),
            &folders,
            &["txt".to_string()],
            false,
            CancellationToken::new(),
            &Arc::new(SyncProgress::new()),
            |folder| async move {
                if folder == "/bad" { Err(anyhow!("listing refused")) } else { Ok(FolderDiscovery::from(listing_of(&["one.txt"]))) }
            },
            |_| async { Err(anyhow!("download refused")) },
        )
        .await
        .unwrap();

    assert_eq!(outcome.folders_failed, 1, "{outcome:?}");
    assert_eq!(outcome.discovered, 1, "{outcome:?}");
    assert_eq!(outcome.failed, 1, "{outcome:?}");
    assert!(!outcome.is_clean());
    assert!(outcome.first_error.as_deref().unwrap().contains("listing refused"), "{outcome:?}");
    let summary = outcome.failure_summary().unwrap();
    assert!(summary.contains("1 of 1 files failed"), "{summary}");
    assert!(summary.contains("1 of 2 folders"), "{summary}");
    let _ = ctx.cleanup_and_close().await;
}

/// A file that cannot be read is counted as failed; the source ends in the
/// error status with a summary and the failed file stays pending.
#[tokio::test]
async fn unreadable_file_marks_the_source_as_failed_with_counts() {
    let ctx = TestContext::new().await;
    let admin = ctx.auth_helper().create_admin_user().await;
    let user_id = admin.user_response.id;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("ok.txt"), b"readable").unwrap();
    let locked = dir.path().join("locked.txt");
    std::fs::write(&locked, b"unreadable").unwrap();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();

    let source = ctx
        .state()
        .db
        .create_source(
            user_id,
            &CreateSource {
                name: "failing".into(),
                source_type: SourceType::LocalFolder,
                enabled: Some(true),
                config: json!({
                    "watch_folders": [dir.path().to_str().unwrap()],
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

    let outcome = SourceSyncService::new(ctx.state().clone()).sync_source(&source, false).await.unwrap();

    assert_eq!(outcome.discovered, 2, "{outcome:?}");
    assert_eq!(outcome.processed, 1, "{outcome:?}");
    assert_eq!(outcome.failed, 1, "{outcome:?}");
    assert_eq!(ctx.state().db.count_documents_for_source(user_id, source.id).await.unwrap().0, 1);

    let source = ctx.state().db.get_source(user_id, source.id).await.unwrap().unwrap();
    assert_eq!(source.status, SourceStatus::Error);
    let last_error = source.last_error.clone().unwrap_or_default();
    assert!(last_error.contains("1 of 2 files failed"), "{last_error}");
    assert!(last_error.contains("locked.txt"), "{last_error}");
    assert_eq!(source.total_files_pending, 1);

    let _ = ctx.cleanup_and_close().await;
}

#[tokio::test]
async fn unlisted_subdirectory_marks_the_run_not_clean_but_syncs_listed_files() {
    let ctx = TestContext::new().await;
    let service = SourceSyncService::new(ctx.state().clone());
    let folders = vec!["/docs".to_string()];

    let outcome = service
        .run_pipeline(
            Uuid::new_v4(),
            Uuid::new_v4(),
            &folders,
            &["txt".to_string()],
            false,
            CancellationToken::new(),
            &Arc::new(SyncProgress::new()),
            |_| async move {
                Ok(FolderDiscovery {
                    files: listing_of(&["one.txt"]),
                    failed_directories: vec!["/docs/broken".to_string(), "/docs/broken/deeper".to_string()],
                    directory_commit: None,
                })
            },
            |_| async { Err(anyhow!("download refused")) },
        )
        .await
        .unwrap();

    assert_eq!(outcome.directories_failed, 2, "{outcome:?}");
    assert_eq!(outcome.folders_failed, 0, "{outcome:?}");
    assert_eq!(outcome.discovered, 1, "{outcome:?}");
    assert!(!outcome.is_clean());
    let summary = outcome.failure_summary().unwrap();
    assert!(summary.contains("2 subdirectories could not be listed"), "{summary}");
    assert!(summary.contains("directory /docs/broken: could not be listed"), "{summary}");
}

/// A download that fails for a real reason while the stop request arrives is
/// still a failed file; only the cancellation error itself is not counted.
#[tokio::test]
async fn a_real_failure_during_cancellation_is_still_counted() {
    let ctx = TestContext::new().await;
    let service = SourceSyncService::new(ctx.state().clone());
    let folders = vec!["/docs".to_string()];
    let token = CancellationToken::new();
    let cancel = token.clone();

    let outcome = service
        .run_pipeline(
            Uuid::new_v4(),
            Uuid::new_v4(),
            &folders,
            &["txt".to_string()],
            false,
            token,
            &Arc::new(SyncProgress::new()),
            |_| async move { Ok(FolderDiscovery::from(listing_of(&["one.txt"]))) },
            move |_| {
                let cancel = cancel.clone();
                async move {
                    cancel.cancel();
                    Err(anyhow!("download refused"))
                }
            },
        )
        .await
        .unwrap();

    assert!(outcome.cancelled, "{outcome:?}");
    assert_eq!(outcome.failed, 1, "{outcome:?}");
    assert!(outcome.first_error.as_deref().unwrap().contains("download refused"), "{outcome:?}");
    let _ = ctx.cleanup_and_close().await;
}

/// The cancellation error a download returns once the token fires is not a
/// file failure.
#[tokio::test]
async fn the_cancellation_error_itself_is_not_a_failure() {
    let ctx = TestContext::new().await;
    let service = SourceSyncService::new(ctx.state().clone());
    let folders = vec!["/docs".to_string()];
    let token = CancellationToken::new();
    let cancel = token.clone();

    let outcome = service
        .run_pipeline(
            Uuid::new_v4(),
            Uuid::new_v4(),
            &folders,
            &["txt".to_string()],
            false,
            token,
            &Arc::new(SyncProgress::new()),
            |_| async move { Ok(FolderDiscovery::from(listing_of(&["one.txt"]))) },
            move |_| {
                let cancel = cancel.clone();
                async move {
                    cancel.cancel();
                    Err(cancelled("WebDAV operation"))
                }
            },
        )
        .await
        .unwrap();

    assert!(outcome.cancelled, "{outcome:?}");
    assert_eq!(outcome.failed, 0, "{outcome:?}");
    assert!(outcome.first_error.is_none(), "{outcome:?}");
    let _ = ctx.cleanup_and_close().await;
}

/// A stop request that arrives while a folder is being listed ends the run
/// as cancelled, not as a run in which every folder failed.
#[tokio::test]
async fn a_stop_request_during_listing_is_a_cancelled_run_not_a_failed_one() {
    let ctx = TestContext::new().await;
    let service = SourceSyncService::new(ctx.state().clone());
    let folders = vec!["/docs".to_string()];
    let token = CancellationToken::new();
    let cancel = token.clone();

    let outcome = service
        .run_pipeline(
            Uuid::new_v4(),
            Uuid::new_v4(),
            &folders,
            &["txt".to_string()],
            false,
            token,
            &Arc::new(SyncProgress::new()),
            move |folder| {
                let cancel = cancel.clone();
                async move {
                    cancel.cancel();
                    Err(anyhow!("listing {folder} aborted"))
                }
            },
            |_| async { Ok(Vec::new()) },
        )
        .await
        .expect("a stopped run is not a failed run");

    assert!(outcome.cancelled, "{outcome:?}");
    assert_eq!(outcome.folders_failed, 0, "{outcome:?}");
    assert!(outcome.first_error.is_none(), "{outcome:?}");
    let _ = ctx.cleanup_and_close().await;
}
