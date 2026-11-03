//! The scheduler's claim and completion protocol: one run per source at a
//! time, counters written once, every run (including a panicking one)
//! recorded, and no new run while a stopped task is still winding down.

use readur::models::{CreateSource, SourceStatus, SourceType};
use readur::scheduling::source_scheduler::SourceScheduler;
use readur::scheduling::source_sync::SourceSyncService;
use readur::test_utils::TestContext;
use serde_json::json;
use std::time::Duration;
use uuid::Uuid;

struct Fixture {
    ctx: TestContext,
    user_id: Uuid,
    source_id: Uuid,
    dir: tempfile::TempDir,
}

async fn fixture(file_count: usize) -> Fixture {
    let ctx = TestContext::new().await;
    let admin = ctx.auth_helper().create_admin_user().await;
    let user_id = admin.user_response.id;
    let dir = tempfile::tempdir().unwrap();
    for i in 0..file_count {
        std::fs::write(dir.path().join(format!("file-{i}.txt")), format!("content {i}")).unwrap();
    }
    let source = ctx
        .state()
        .db
        .create_source(
            user_id,
            &CreateSource {
                name: "claim".into(),
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
    Fixture { ctx, user_id, source_id: source.id, dir }
}

impl Fixture {
    async fn source(&self) -> readur::models::Source {
        self.ctx.state().db.get_source(self.user_id, self.source_id).await.unwrap().unwrap()
    }

    async fn document_count(&self) -> i64 {
        self.ctx.state().db.count_documents_for_source(self.user_id, self.source_id).await.unwrap().0
    }
}

async fn wait_until_not_running(scheduler: &SourceScheduler, source_id: Uuid) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    while scheduler.is_running(source_id) {
        assert!(tokio::time::Instant::now() < deadline, "sync task did not exit in time");
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

/// Two simultaneous triggers claim the source once: one run, counters equal
/// to the number of files, not double.
#[tokio::test]
async fn concurrent_triggers_start_exactly_one_run() {
    let f = fixture(3).await;
    let scheduler = SourceScheduler::new(f.ctx.state().clone());

    let (a, b) = tokio::join!(scheduler.trigger_sync(f.source_id), scheduler.trigger_sync(f.source_id));
    assert_eq!(a.is_ok() as u8 + b.is_ok() as u8, 1, "exactly one trigger must win: {a:?} / {b:?}");
    wait_until_not_running(&scheduler, f.source_id).await;

    let source = f.source().await;
    assert_eq!(source.status, SourceStatus::Idle, "{:?}", source.last_error);
    assert_eq!(source.last_error, None);
    assert_eq!(source.total_files_synced, 3);
    assert_eq!(source.total_files_pending, 0);
    assert!(source.last_sync_at.is_some());
    assert_eq!(f.document_count().await, 3);
    assert!(!f.ctx.state().sync_progress_tracker.is_syncing(f.source_id));

    let _ = f.ctx.cleanup_and_close().await;
}

/// `total_files_synced` accumulates across runs and is written once per run.
#[tokio::test]
async fn counters_are_cumulative_and_written_once() {
    let f = fixture(2).await;
    let service = SourceSyncService::new(f.ctx.state().clone());

    service.sync_source(&f.source().await, false).await.unwrap();
    let after_first = f.source().await;
    assert_eq!(after_first.total_files_synced, 2);
    assert_eq!(after_first.total_files_pending, 0);
    assert_eq!(after_first.status, SourceStatus::Idle);

    std::fs::write(f.dir.path().join("file-new.txt"), b"a third file").unwrap();
    service.sync_source(&f.source().await, false).await.unwrap();
    let after_second = f.source().await;
    assert_eq!(after_second.total_files_synced, 3);
    assert_eq!(after_second.total_files_pending, 0);
    assert!(after_second.last_sync_at >= after_first.last_sync_at);

    let _ = f.ctx.cleanup_and_close().await;
}

/// A run that fails before discovering anything still moves `last_sync_at`,
/// so the scheduler does not retry it on every tick.
#[tokio::test]
async fn failed_run_records_last_sync_at() {
    let f = fixture(1).await;
    let source = f.source().await;
    assert_eq!(source.last_sync_at, None);
    std::fs::remove_dir_all(f.dir.path()).unwrap();

    let err = SourceSyncService::new(f.ctx.state().clone()).sync_source(&source, false).await.unwrap_err();
    assert!(err.to_string().contains("does not exist"), "{err}");

    let source = f.source().await;
    assert_eq!(source.status, SourceStatus::Error);
    assert!(source.last_sync_at.is_some());
    assert!(source.last_error.unwrap().contains("does not exist"));

    let _ = f.ctx.cleanup_and_close().await;
}

/// A sync task that panics is recorded as a failed run and leaves no
/// registry or progress entry behind.
#[tokio::test]
async fn panicking_sync_task_is_recorded_and_released() {
    let f = fixture(0).await;
    let scheduler = SourceScheduler::new(f.ctx.state().clone());

    scheduler
        .run_claimed(f.source_id, |_token, _progress| async { panic!("sync exploded") })
        .await
        .unwrap();
    wait_until_not_running(&scheduler, f.source_id).await;
    // The supervisor writes the status after the task has exited.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    while f.source().await.status == SourceStatus::Syncing {
        assert!(tokio::time::Instant::now() < deadline, "status still syncing after the panic");
        tokio::time::sleep(Duration::from_millis(25)).await;
    }

    let source = f.source().await;
    assert_eq!(source.status, SourceStatus::Error);
    assert!(source.last_error.clone().unwrap_or_default().contains("panicked"), "{:?}", source.last_error);
    assert!(source.last_sync_at.is_some());
    assert!(!f.ctx.state().sync_progress_tracker.is_syncing(f.source_id));
    assert!(scheduler.trigger_sync(f.source_id).await.is_ok(), "a new run can start after the panic");
    wait_until_not_running(&scheduler, f.source_id).await;

    let _ = f.ctx.cleanup_and_close().await;
}

/// After a stop, a new sync is refused until the stopped task has exited;
/// then it starts normally.
#[tokio::test]
async fn stop_then_trigger_is_refused_until_the_task_exits() {
    let f = fixture(1).await;
    let scheduler = SourceScheduler::new(f.ctx.state().clone());
    let (release, released) = tokio::sync::oneshot::channel::<()>();

    scheduler
        .run_claimed(f.source_id, |token, _progress| async move {
            token.cancelled().await;
            let _ = released.await;
        })
        .await
        .unwrap();
    assert!(scheduler.is_running(f.source_id));
    assert!(f.ctx.state().sync_progress_tracker.is_syncing(f.source_id));

    scheduler.stop_sync(f.source_id).await.unwrap();
    let source = f.source().await;
    assert_eq!(source.status, SourceStatus::Idle);
    assert_eq!(source.last_error.as_deref(), Some("Sync cancelled by user"));
    assert!(!f.ctx.state().sync_progress_tracker.is_syncing(f.source_id));

    let err = scheduler.trigger_sync(f.source_id).await.unwrap_err();
    assert!(err.to_string().contains("stopping"), "{err}");
    assert!(scheduler.is_running(f.source_id), "the stopped task still holds its slot");

    release.send(()).unwrap();
    wait_until_not_running(&scheduler, f.source_id).await;
    scheduler.trigger_sync(f.source_id).await.unwrap();
    wait_until_not_running(&scheduler, f.source_id).await;
    assert_eq!(f.source().await.status, SourceStatus::Idle);
    assert_eq!(f.document_count().await, 1);

    let _ = f.ctx.cleanup_and_close().await;
}
