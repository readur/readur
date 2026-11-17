//! `POST /api/sources/{id}/sync/stop` decides on the scheduler's typed stop
//! result, and the scheduler's tick decides on a source's configuration
//! itself, never on the text of an earlier error.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use readur::models::{CreateSource, SourceStatus, SourceType};
use readur::scheduling::source_scheduler::{SourceScheduler, StopSyncError};
use readur::test_utils::{TestConfigBuilder, TestContext};
use readur::AppState;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceExt;
use uuid::Uuid;

struct Harness {
    ctx: TestContext,
    app: axum::Router,
    scheduler: Arc<SourceScheduler>,
    token: String,
    user_id: Uuid,
    dir: tempfile::TempDir,
    _uploads: tempfile::TempDir,
}

async fn harness() -> Harness {
    let uploads = tempfile::tempdir().unwrap();
    let config = TestConfigBuilder::default().with_upload_path(uploads.path().to_str().unwrap());
    let ctx = TestContext::with_config(config).await;
    let helper = ctx.auth_helper();
    // An admin may sync any local folder.
    let mut user = helper.create_admin_user().await;
    let token = user.login(&helper).await.unwrap().to_string();
    let scheduler = Arc::new(SourceScheduler::new(ctx.state().clone()));
    let state = Arc::new(AppState {
        source_scheduler: Some(scheduler.clone()),
        ..(**ctx.state()).clone()
    });
    let app = readur::test_utils::create_test_app(state);
    let dir = tempfile::tempdir().unwrap();
    Harness {
        user_id: user.user_response.id,
        ctx,
        app,
        scheduler,
        token,
        dir,
        _uploads: uploads,
    }
}

impl Harness {
    async fn stop(&self, id: Uuid) -> StatusCode {
        let request = Request::builder()
            .method("POST")
            .uri(format!("/api/sources/{id}/sync/stop"))
            .header("authorization", format!("Bearer {}", self.token))
            .body(Body::empty())
            .unwrap();
        self.app.clone().oneshot(request).await.unwrap().status()
    }

    async fn local_source(&self, config: serde_json::Value) -> Uuid {
        let source = CreateSource {
            name: format!("stop-{}", Uuid::new_v4()),
            source_type: SourceType::LocalFolder,
            enabled: Some(true),
            config,
        };
        self.ctx
            .state()
            .db
            .create_source(self.user_id, &source)
            .await
            .unwrap()
            .id
    }

    fn valid_config(&self, auto_sync: bool) -> serde_json::Value {
        json!({
            "watch_folders": [self.dir.path().to_str().unwrap()],
            "file_extensions": ["txt"],
            "auto_sync": auto_sync,
            "sync_interval_minutes": 60,
            "recursive": true,
            "follow_symlinks": false
        })
    }

    async fn source(&self, id: Uuid) -> readur::models::Source {
        self.ctx
            .state()
            .db
            .get_source(self.user_id, id)
            .await
            .unwrap()
            .unwrap()
    }

    async fn wait_until_not_running(&self, id: Uuid) {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
        while self.scheduler.is_running(id) {
            assert!(
                tokio::time::Instant::now() < deadline,
                "sync task did not exit in time"
            );
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    async fn set_status(&self, id: Uuid, status: &str, last_error: &str, hours_ago: i32) {
        sqlx::query(
            "UPDATE sources SET status = $1, last_error = $2, \
             last_error_at = NOW() - make_interval(hours => $3) WHERE id = $4",
        )
        .bind(status)
        .bind(last_error)
        .bind(hours_ago)
        .bind(id)
        .execute(self.ctx.state().db.get_pool())
        .await
        .unwrap();
    }
}

/// Stopping a running sync: 200, the task is cancelled and the source is
/// marked idle with the cancelled message.
#[tokio::test]
async fn stopping_a_running_sync_cancels_it() {
    let h = harness().await;
    let id = h.local_source(h.valid_config(false)).await;
    h.scheduler
        .run_claimed(
            id,
            |token, _progress| async move { token.cancelled().await },
        )
        .await
        .unwrap();

    assert_eq!(h.stop(id).await, StatusCode::OK);

    h.wait_until_not_running(id).await;
    let source = h.source(id).await;
    assert_eq!(source.status, SourceStatus::Idle);
    assert_eq!(source.last_error.as_deref(), Some("Sync cancelled by user"));
    let _ = h.ctx.cleanup_and_close().await;
}

/// Stopping when nothing runs is a success too: 200, and a source left
/// marked syncing is put back to idle.
#[tokio::test]
async fn stopping_when_nothing_runs_succeeds_and_idles_the_source() {
    let h = harness().await;
    let id = h.local_source(h.valid_config(false)).await;
    h.set_status(id, "syncing", "", 0).await;
    assert_eq!(
        h.scheduler.stop_sync(id).await,
        Err(StopSyncError::NotRunning)
    );

    assert_eq!(h.stop(id).await, StatusCode::OK);

    let source = h.source(id).await;
    assert_eq!(source.status, SourceStatus::Idle);
    assert_eq!(source.last_error.as_deref(), Some("No sync was running"));
    let _ = h.ctx.cleanup_and_close().await;
}

/// Another user's source, or none at all, is not found.
#[tokio::test]
async fn stopping_an_unknown_source_is_not_found() {
    let h = harness().await;
    assert_eq!(h.stop(Uuid::new_v4()).await, StatusCode::NOT_FOUND);
    let _ = h.ctx.cleanup_and_close().await;
}

/// A source in the error state whose stored error still says
/// "Configuration error" but whose configuration has since become valid is
/// synced: the tick validates the configuration instead of reading the text.
#[tokio::test]
async fn a_fixed_configuration_is_synced_despite_an_old_configuration_error() {
    let h = harness().await;
    std::fs::write(h.dir.path().join("a.txt"), b"hello").unwrap();
    let id = h.local_source(h.valid_config(true)).await;
    h.set_status(id, "error", "Configuration error: watch folder missing", 0)
        .await;

    h.scheduler.check_and_sync_sources().await.unwrap();

    h.wait_until_not_running(id).await;
    let source = h.source(id).await;
    assert_eq!(source.status, SourceStatus::Idle, "{:?}", source.last_error);
    assert_eq!(source.total_files_synced, 1);
    let _ = h.ctx.cleanup_and_close().await;
}

/// An invalid configuration is recorded as a configuration error and not
/// synced; a later tick within the hour leaves the record alone.
#[tokio::test]
async fn an_invalid_configuration_is_recorded_once_and_never_synced() {
    let h = harness().await;
    let id = h.local_source(json!({"watch_folders": "not a list"})).await;

    h.scheduler.check_and_sync_sources().await.unwrap();

    assert!(!h.scheduler.is_running(id));
    let first = h.source(id).await;
    assert_eq!(first.status, SourceStatus::Error);
    let error = first.last_error.clone().unwrap();
    assert!(error.starts_with("Configuration error: "), "{error}");

    h.scheduler.check_and_sync_sources().await.unwrap();
    let second = h.source(id).await;
    assert_eq!(
        second.last_error_at, first.last_error_at,
        "not rewritten within the hour"
    );
    assert!(!h.scheduler.is_running(id));

    // An error from another cause that is over an hour old is replaced.
    h.set_status(id, "error", "an unrelated failure", 2).await;
    h.scheduler.check_and_sync_sources().await.unwrap();
    assert_eq!(h.source(id).await.last_error, Some(error));
    let _ = h.ctx.cleanup_and_close().await;
}
