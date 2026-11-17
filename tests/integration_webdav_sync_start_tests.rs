//! Starting the settings-level WebDAV sync is a single atomic claim: of any
//! number of concurrent starts exactly one runs, the rest get the "already
//! running" answer, and the claim is released on every exit path (success,
//! failure, panic, cancel) so the next start is accepted.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use readur::db::webdav::WEBDAV_SYNC_INTERRUPTED_MESSAGE;
use readur::models::source::UpdateWebDAVSyncState;
use readur::models::UpdateSettings;
use readur::scheduling::webdav_scheduler::WebDAVScheduler;
use readur::services::webdav_sync::{
    is_webdav_sync_live, run_claimed_webdav_sync, WebDavSyncStart,
};
use readur::test_utils::TestContext;
use serde_json::{json, Value};
use std::time::Duration;
use tower::ServiceExt;
use uuid::Uuid;
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

/// A host that never resolves (`.invalid` is reserved), so a sync against it
/// fails at once, in the connect-time address check.
const UNRESOLVABLE_URL: &str = "http://readur-sync-test.invalid";
const UNRESOLVABLE_ERROR: &str = "The server host could not be resolved";

struct Harness {
    ctx: TestContext,
    token: String,
    user_id: Uuid,
}

async fn harness() -> Harness {
    let ctx = TestContext::new().await;
    let helper = ctx.auth_helper();
    let mut user = helper.create_test_user().await;
    let token = user.login(&helper).await.unwrap().to_string();
    Harness {
        user_id: user.user_response.id,
        ctx,
        token,
    }
}

/// A WebDAV server whose listing takes `delay`, keeping a sync running.
async fn slow_server(delay: Duration) -> MockServer {
    let server = MockServer::start().await;
    let listing = r#"<?xml version="1.0"?><d:multistatus xmlns:d="DAV:"><d:response><d:href>/Documents/</d:href><d:propstat><d:prop><d:getetag>"root"</d:getetag><d:resourcetype><d:collection/></d:resourcetype></d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response></d:multistatus>"#;
    Mock::given(method("PROPFIND"))
        .respond_with(
            ResponseTemplate::new(207)
                .set_body_string(listing)
                .set_delay(delay),
        )
        .mount(&server)
        .await;
    server
}

impl Harness {
    async fn configure_webdav(&self, server_url: &str, auto_sync: bool) {
        let settings: UpdateSettings = serde_json::from_value(json!({
            "webdav_enabled": true,
            "webdav_server_url": server_url,
            "webdav_username": "u",
            "webdav_password": "p",
            "webdav_watch_folders": ["/Documents"],
            "webdav_file_extensions": ["pdf"],
            "webdav_auto_sync": auto_sync,
            "webdav_sync_interval_minutes": 60
        }))
        .unwrap();
        self.ctx
            .state()
            .db
            .create_or_update_settings(self.user_id, &settings)
            .await
            .unwrap();
    }

    async fn post(&self, uri: &str) -> (StatusCode, Value) {
        let request = Request::builder()
            .method("POST")
            .uri(uri)
            .header("authorization", format!("Bearer {}", self.token))
            .body(Body::empty())
            .unwrap();
        let response = self.ctx.app().clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    async fn start(&self) -> (StatusCode, Value) {
        self.post("/api/webdav/sync/start").await
    }

    async fn sync_state(&self) -> Option<readur::models::source::WebDAVSyncState> {
        self.ctx
            .state()
            .db
            .get_webdav_sync_state(self.user_id)
            .await
            .unwrap()
    }

    /// Waits until the user's sync task has exited and its end is recorded.
    async fn wait_until_finished(&self) {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
        while is_webdav_sync_live(self.user_id)
            || self.sync_state().await.is_some_and(|s| s.is_running)
        {
            assert!(
                tokio::time::Instant::now() < deadline,
                "WebDAV sync did not finish in time"
            );
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }
}

fn already_running() -> Value {
    json!({
        "success": false,
        "error": "sync_already_running",
        "message": "A WebDAV sync is already in progress. Please wait for it to complete before starting a new sync."
    })
}

fn started() -> Value {
    json!({"success": true, "message": "WebDAV sync started successfully"})
}

/// N simultaneous starts: exactly one starts a sync, every other gets the
/// response a start made while a sync runs gets; once the sync has
/// finished, a new start is accepted.
#[tokio::test]
async fn concurrent_starts_start_exactly_one_sync() {
    let h = harness().await;
    let server = slow_server(Duration::from_secs(2)).await;
    h.configure_webdav(&server.uri(), false).await;

    let responses = futures::future::join_all((0..8).map(|_| h.start())).await;

    let mut winners = 0;
    for (status, body) in &responses {
        assert_eq!(*status, StatusCode::OK, "{body}");
        if *body == started() {
            winners += 1;
        } else {
            assert_eq!(*body, already_running());
        }
    }
    assert_eq!(winners, 1, "exactly one start must win: {responses:?}");
    assert!(is_webdav_sync_live(h.user_id));
    assert!(h.sync_state().await.unwrap().is_running);
    // A start made while the sync is plainly running gets the same answer.
    assert_eq!(h.start().await, (StatusCode::OK, already_running()));

    h.wait_until_finished().await;
    assert_eq!(h.start().await, (StatusCode::OK, started()));
    h.wait_until_finished().await;
    let _ = h.ctx.cleanup_and_close().await;
}

/// A sync that fails releases the claim and records why.
#[tokio::test]
async fn a_failed_sync_releases_the_claim() {
    let h = harness().await;
    h.configure_webdav(UNRESOLVABLE_URL, false).await;

    assert_eq!(h.start().await, (StatusCode::OK, started()));
    h.wait_until_finished().await;
    let state = h.sync_state().await.unwrap();
    assert!(!state.is_running);
    assert_eq!(state.errors, vec![UNRESOLVABLE_ERROR.to_string()]);

    assert_eq!(
        h.start().await,
        (StatusCode::OK, started()),
        "a new start is accepted after a failure"
    );
    h.wait_until_finished().await;
    let _ = h.ctx.cleanup_and_close().await;
}

/// A sync task that panics is recorded as failed and releases the claim.
#[tokio::test]
async fn a_panicking_sync_releases_the_claim() {
    let h = harness().await;
    let server = slow_server(Duration::from_millis(10)).await;
    h.configure_webdav(&server.uri(), false).await;
    let (finished_tx, finished_rx) = tokio::sync::oneshot::channel();

    let start = run_claimed_webdav_sync(
        h.ctx.state().clone(),
        h.user_id,
        async { panic!("sync exploded") },
        move |result| async move {
            let _ = finished_tx.send(result.map_err(|e| e.to_string()));
        },
    )
    .await
    .unwrap();
    assert_eq!(start, WebDavSyncStart::Started);

    let result = tokio::time::timeout(Duration::from_secs(10), finished_rx)
        .await
        .unwrap()
        .unwrap();
    assert!(result.unwrap_err().contains("panicked"));
    assert!(
        !is_webdav_sync_live(h.user_id),
        "the slot is released before on_finished runs"
    );
    let state = h.sync_state().await.unwrap();
    assert!(!state.is_running, "the panic is recorded, not left running");
    assert!(
        state.errors[0].contains("sync task panicked"),
        "{:?}",
        state.errors
    );

    assert_eq!(h.start().await, (StatusCode::OK, started()));
    h.wait_until_finished().await;
    let _ = h.ctx.cleanup_and_close().await;
}

/// A row already marked running (by another server instance, say) is
/// respected: nothing starts in this process.
#[tokio::test]
async fn a_sync_marked_running_in_the_database_is_respected() {
    let h = harness().await;
    let server = slow_server(Duration::from_millis(10)).await;
    h.configure_webdav(&server.uri(), false).await;
    let running = UpdateWebDAVSyncState {
        last_sync_at: None,
        sync_cursor: None,
        is_running: true,
        files_processed: 0,
        files_remaining: 0,
        current_folder: None,
        errors: Vec::new(),
    };
    h.ctx
        .state()
        .db
        .update_webdav_sync_state(h.user_id, &running)
        .await
        .unwrap();

    assert_eq!(h.start().await, (StatusCode::OK, already_running()));
    assert!(
        !is_webdav_sync_live(h.user_id),
        "a refused claim leaves no slot behind"
    );
    let _ = h.ctx.cleanup_and_close().await;
}

/// After a cancel, a new start is refused while the cancelled task winds
/// down, then accepted once it has exited.
#[tokio::test]
async fn a_cancelled_sync_keeps_the_claim_until_its_task_exits() {
    let h = harness().await;
    let server = slow_server(Duration::from_secs(2)).await;
    h.configure_webdav(&server.uri(), false).await;
    assert_eq!(h.start().await, (StatusCode::OK, started()));

    let (status, body) = h.post("/api/webdav/sync/cancel").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], json!(true), "{body}");
    assert!(
        !h.sync_state().await.unwrap().is_running,
        "the cancel marks the row stopped at once"
    );
    assert!(is_webdav_sync_live(h.user_id), "the task is still listing");
    assert_eq!(h.start().await, (StatusCode::OK, already_running()));

    h.wait_until_finished().await;
    let state = h.sync_state().await.unwrap();
    assert_eq!(
        state.errors,
        vec!["Sync failed: Sync cancelled by user".to_string()]
    );
    assert_eq!(h.start().await, (StatusCode::OK, started()));
    h.wait_until_finished().await;
    let _ = h.ctx.cleanup_and_close().await;
}

/// The startup reset marks running syncs with the exact interrupted marker,
/// and the scheduler resumes exactly the users carrying it: an error that
/// merely mentions a server restart is not taken for the marker.
#[tokio::test]
async fn only_syncs_carrying_the_interrupted_marker_are_resumed() {
    let h = harness().await;
    let db = &h.ctx.state().db;
    h.configure_webdav(UNRESOLVABLE_URL, true).await;
    let running = UpdateWebDAVSyncState {
        last_sync_at: None,
        sync_cursor: None,
        is_running: true,
        files_processed: 4,
        files_remaining: 0,
        current_folder: Some("/Documents".into()),
        errors: vec!["earlier".into()],
    };
    db.update_webdav_sync_state(h.user_id, &running)
        .await
        .unwrap();
    assert!(db.reset_running_webdav_syncs().await.unwrap() >= 1);
    let reset = h.sync_state().await.unwrap();
    assert!(!reset.is_running);
    assert_eq!(
        reset.errors,
        vec![
            "earlier".to_string(),
            WEBDAV_SYNC_INTERRUPTED_MESSAGE.to_string()
        ]
    );

    let other = h
        .ctx
        .auth_helper()
        .create_test_user()
        .await
        .user_response
        .id;
    let settings: UpdateSettings = serde_json::from_value(json!({
        "webdav_enabled": true, "webdav_server_url": UNRESOLVABLE_URL, "webdav_username": "u",
        "webdav_password": "p", "webdav_watch_folders": ["/Documents"], "webdav_auto_sync": true
    }))
    .unwrap();
    db.create_or_update_settings(other, &settings)
        .await
        .unwrap();
    let lookalike = vec![format!(
        "{} (but not the marker)",
        WEBDAV_SYNC_INTERRUPTED_MESSAGE
    )];
    let other_state = UpdateWebDAVSyncState {
        is_running: false,
        errors: lookalike.clone(),
        current_folder: None,
        ..running
    };
    db.update_webdav_sync_state(other, &other_state)
        .await
        .unwrap();

    WebDAVScheduler::new(h.ctx.state().clone())
        .resume_interrupted_syncs()
        .await
        .unwrap();

    h.wait_until_finished().await;
    // The resumed run ran (and failed against the unresolvable host).
    assert_eq!(
        h.sync_state().await.unwrap().errors,
        vec![UNRESOLVABLE_ERROR.to_string()]
    );
    assert!(!is_webdav_sync_live(other));
    assert_eq!(
        db.get_webdav_sync_state(other)
            .await
            .unwrap()
            .unwrap()
            .errors,
        lookalike
    );
    let _ = h.ctx.cleanup_and_close().await;
}

/// The database claim alone (what a second server instance relies on):
/// of many concurrent claims exactly one wins, and a released row can be
/// claimed again.
#[tokio::test]
async fn the_database_claim_admits_exactly_one_of_many() {
    let h = harness().await;
    let db = &h.ctx.state().db;

    let claims = futures::future::join_all((0..10).map(|_| db.claim_webdav_sync(h.user_id))).await;
    let won = claims
        .into_iter()
        .map(Result::unwrap)
        .filter(|&claimed| claimed)
        .count();
    assert_eq!(won, 1);
    assert!(
        !db.claim_webdav_sync(h.user_id).await.unwrap(),
        "held claims are refused"
    );

    let released = UpdateWebDAVSyncState {
        last_sync_at: None,
        sync_cursor: None,
        is_running: false,
        files_processed: 0,
        files_remaining: 0,
        current_folder: None,
        errors: Vec::new(),
    };
    db.update_webdav_sync_state(h.user_id, &released)
        .await
        .unwrap();
    assert!(
        db.claim_webdav_sync(h.user_id).await.unwrap(),
        "a released row is claimable"
    );
    let _ = h.ctx.cleanup_and_close().await;
}

/// A progress write only touches a row still marked running, so it can
/// never undo a cancel that landed before it.
#[tokio::test]
async fn a_progress_write_never_undoes_a_cancel() {
    let h = harness().await;
    let db = &h.ctx.state().db;
    let progress = UpdateWebDAVSyncState {
        last_sync_at: None,
        sync_cursor: None,
        is_running: true,
        files_processed: 7,
        files_remaining: 3,
        current_folder: Some("/Documents".into()),
        errors: Vec::new(),
    };
    assert!(db.claim_webdav_sync(h.user_id).await.unwrap());
    assert!(db
        .update_running_webdav_sync_progress(h.user_id, &progress)
        .await
        .unwrap());
    assert_eq!(h.sync_state().await.unwrap().files_processed, 7);

    let cancelled = UpdateWebDAVSyncState {
        is_running: false,
        files_processed: 7,
        ..progress
    };
    db.update_webdav_sync_state(h.user_id, &cancelled)
        .await
        .unwrap();
    let late = UpdateWebDAVSyncState {
        is_running: true,
        files_processed: 9,
        current_folder: None,
        errors: Vec::new(),
        ..cancelled
    };
    assert!(!db
        .update_running_webdav_sync_progress(h.user_id, &late)
        .await
        .unwrap());
    let state = h.sync_state().await.unwrap();
    assert!(!state.is_running, "the cancel stands");
    assert_eq!(state.files_processed, 7);
    let _ = h.ctx.cleanup_and_close().await;
}
