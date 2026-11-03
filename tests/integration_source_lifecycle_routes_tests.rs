//! Source routes that must coordinate with a running sync: delete and
//! disable stop the sync first, and a deep scan goes through the same claim
//! as every other sync.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use readur::models::{CreateSource, SourceStatus, SourceType};
use readur::scheduling::source_scheduler::SourceScheduler;
use readur::test_utils::TestContext;
use readur::AppState;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceExt;
use uuid::Uuid;
use wiremock::{
    matchers::{method, path, path_regex},
    Mock, MockServer, ResponseTemplate,
};

const DEEP_SCAN_FILE_COUNT: usize = 30;

fn entry(href: &str, etag: &str, is_dir: bool) -> String {
    let resourcetype = if is_dir { "<d:resourcetype><d:collection/></d:resourcetype>" } else { "<d:resourcetype/>" };
    format!(
        r#"<d:response><d:href>{href}</d:href><d:propstat><d:prop>
            <d:getlastmodified>Tue, 29 Jul 2025 01:34:17 GMT</d:getlastmodified>
            <d:getetag>"{etag}"</d:getetag>{resourcetype}<d:getcontentlength>9</d:getcontentlength>
        </d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response>"#
    )
}

/// `/Documents` with many PDF files, each of which takes a second to download.
async fn mount_slow_documents(server: &MockServer) {
    let mut entries = vec![entry("/Documents/", "root-v1", true)];
    entries.extend((0..DEEP_SCAN_FILE_COUNT).map(|i| entry(&format!("/Documents/file-{i}.pdf"), &format!("f-{i}"), false)));
    let listing = format!(r#"<?xml version="1.0"?><d:multistatus xmlns:d="DAV:">{}</d:multistatus>"#, entries.join(""));
    Mock::given(method("PROPFIND"))
        .and(path("/Documents"))
        .respond_with(ResponseTemplate::new(207).set_body_string(listing))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/Documents/file-\d+\.pdf$"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/pdf")
                .set_body_bytes(b"%PDF-1.4\n".to_vec())
                .set_delay(Duration::from_secs(1)),
        )
        .mount(server)
        .await;
}

async fn downloads_received(server: &MockServer) -> usize {
    server
        .received_requests()
        .await
        .unwrap_or_default()
        .iter()
        .filter(|r| r.method == wiremock::http::Method::GET)
        .count()
}

struct Harness {
    ctx: TestContext,
    app: axum::Router,
    scheduler: Arc<SourceScheduler>,
    token: String,
    user_id: Uuid,
}

async fn harness() -> Harness {
    let ctx = TestContext::new().await;
    let helper = ctx.auth_helper();
    let mut admin = helper.create_admin_user().await;
    let token = admin.login(&helper).await.unwrap().to_string();
    let scheduler = Arc::new(SourceScheduler::new(ctx.state().clone()));
    let state = Arc::new(AppState { source_scheduler: Some(scheduler.clone()), ..(**ctx.state()).clone() });
    let app = readur::test_utils::create_test_app(state);
    Harness { user_id: admin.user_response.id, ctx, app, scheduler, token }
}

impl Harness {
    async fn send(&self, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
        let mut builder = Request::builder()
            .method(method)
            .uri(uri)
            .header("authorization", format!("Bearer {}", self.token));
        let body = match body {
            Some(v) => {
                builder = builder.header("content-type", "application/json");
                Body::from(v.to_string())
            }
            None => Body::empty(),
        };
        let response = self.app.clone().oneshot(builder.body(body).unwrap()).await.unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
    }

    async fn local_source(&self, dir: &std::path::Path) -> Uuid {
        let source = CreateSource {
            name: "lifecycle".into(),
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
        };
        self.ctx.state().db.create_source(self.user_id, &source).await.unwrap().id
    }

    async fn webdav_source(&self, server_url: &str) -> Uuid {
        let source = CreateSource {
            name: "webdav".into(),
            source_type: SourceType::WebDAV,
            enabled: Some(true),
            config: json!({
                "server_url": server_url,
                "username": "u",
                "password": "p",
                "watch_folders": ["/Documents"],
                "file_extensions": ["pdf"],
                "auto_sync": false,
                "sync_interval_minutes": 60,
                "server_type": "generic"
            }),
        };
        self.ctx.state().db.create_source(self.user_id, &source).await.unwrap().id
    }

    async fn source(&self, id: Uuid) -> Option<readur::models::Source> {
        self.ctx.state().db.get_source(self.user_id, id).await.unwrap()
    }

    /// Starts a claimed task that exits once its token is cancelled.
    async fn start_cancellable_run(&self, id: Uuid) {
        self.scheduler
            .run_claimed(id, |token, _progress| async move { token.cancelled().await })
            .await
            .unwrap();
        assert!(self.scheduler.is_running(id));
    }

    async fn wait_until_not_running(&self, id: Uuid) {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
        while self.scheduler.is_running(id) {
            assert!(tokio::time::Instant::now() < deadline, "sync task did not exit in time");
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }
}

/// Deleting a source stops its running sync and waits for the task before
/// removing the row.
#[tokio::test]
async fn delete_stops_the_running_sync_first() {
    let h = harness().await;
    let dir = tempfile::tempdir().unwrap();
    let id = h.local_source(dir.path()).await;
    h.start_cancellable_run(id).await;

    let (status, _) = h.send("DELETE", &format!("/api/sources/{id}"), None).await;

    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(!h.scheduler.is_running(id));
    assert!(h.source(id).await.is_none());
    let _ = h.ctx.cleanup_and_close().await;
}

/// A sync task that does not exit after the stop request keeps the source:
/// the delete is refused rather than pulling the row out from under it.
#[tokio::test]
async fn delete_is_refused_while_the_sync_task_is_still_alive() {
    let h = harness().await;
    let dir = tempfile::tempdir().unwrap();
    let id = h.local_source(dir.path()).await;
    let (release, released) = tokio::sync::oneshot::channel::<()>();
    h.scheduler
        .run_claimed(id, |_token, _progress| async move {
            let _ = released.await;
        })
        .await
        .unwrap();

    let (status, _) = h.send("DELETE", &format!("/api/sources/{id}"), None).await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert!(h.source(id).await.is_some());
    release.send(()).unwrap();
    h.wait_until_not_running(id).await;
    let _ = h.ctx.cleanup_and_close().await;
}

/// Disabling a source stops its running sync.
#[tokio::test]
async fn disabling_stops_the_running_sync() {
    let h = harness().await;
    let dir = tempfile::tempdir().unwrap();
    let id = h.local_source(dir.path()).await;
    h.start_cancellable_run(id).await;

    let (status, body) = h.send("PUT", &format!("/api/sources/{id}"), Some(json!({"enabled": false}))).await;

    assert_eq!(status, StatusCode::OK, "{body}");
    h.wait_until_not_running(id).await;
    let source = h.source(id).await.unwrap();
    assert!(!source.enabled);
    assert_eq!(source.status, SourceStatus::Idle);
    let _ = h.ctx.cleanup_and_close().await;
}

/// A deep scan is refused while any sync task for the source is alive.
#[tokio::test]
async fn deep_scan_is_refused_while_a_sync_is_running() {
    let h = harness().await;
    let server = MockServer::start().await;
    let id = h.webdav_source(&server.uri()).await;
    h.start_cancellable_run(id).await;

    let (status, body) = h.send("POST", &format!("/api/sources/{id}/scan/deep"), None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], json!(false), "{body}");
    assert_eq!(body["error"], json!("source_already_syncing"), "{body}");
    h.scheduler.stop_sync(id).await.unwrap();
    h.wait_until_not_running(id).await;
    let _ = h.ctx.cleanup_and_close().await;
}

/// A deep scan claims the source, is visible as a running sync, and releases
/// the claim when it ends even when the server refuses every request.
#[tokio::test]
async fn deep_scan_claims_and_releases_the_source() {
    let h = harness().await;
    let server = MockServer::start().await;
    Mock::given(method("PROPFIND")).respond_with(ResponseTemplate::new(500)).mount(&server).await;
    let id = h.webdav_source(&server.uri()).await;

    let (status, body) = h.send("POST", &format!("/api/sources/{id}/scan/deep"), None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], json!(true), "{body}");
    let (second_status, second) = h.send("POST", &format!("/api/sources/{id}/sync"), None).await;
    assert!(
        second_status == StatusCode::CONFLICT || !h.scheduler.is_running(id),
        "a sync while the deep scan runs must be refused: {second_status} {second}"
    );
    h.wait_until_not_running(id).await;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    while h.source(id).await.unwrap().status == SourceStatus::Syncing {
        assert!(tokio::time::Instant::now() < deadline, "source stayed syncing after the deep scan");
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert!(!h.ctx.state().sync_progress_tracker.is_syncing(id));
    let _ = h.ctx.cleanup_and_close().await;
}

/// A stop request ends a deep scan that is in the middle of downloading:
/// the task exits well before it would have finished, the files not yet
/// started are not fetched, and the run is recorded as cancelled.
#[tokio::test]
async fn stop_ends_a_running_deep_scan() {
    let h = harness().await;
    let server = MockServer::start().await;
    mount_slow_documents(&server).await;
    let id = h.webdav_source(&server.uri()).await;

    let (status, body) = h.send("POST", &format!("/api/sources/{id}/scan/deep"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], json!(true), "{body}");

    // Wait until the scan is downloading, then stop it.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    while downloads_received(&server).await == 0 {
        assert!(tokio::time::Instant::now() < deadline, "deep scan never started downloading");
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    let (status, _) = h.send("POST", &format!("/api/sources/{id}/sync/stop"), None).await;
    assert_eq!(status, StatusCode::OK);

    // Left alone the scan would need 6 s (30 files, 5 at a time, 1 s each).
    let deadline = tokio::time::Instant::now() + Duration::from_secs(4);
    while h.scheduler.is_running(id) {
        assert!(tokio::time::Instant::now() < deadline, "deep scan kept running after the stop request");
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert!(downloads_received(&server).await < DEEP_SCAN_FILE_COUNT, "every file was still downloaded");
    let source = h.source(id).await.unwrap();
    assert_eq!(source.status, SourceStatus::Idle);
    assert_eq!(source.last_error.as_deref(), Some("Sync cancelled by user"));
    assert!(source.last_sync_at.is_some());
    assert!(!h.ctx.state().sync_progress_tracker.is_syncing(id));
    let _ = h.ctx.cleanup_and_close().await;
}

/// A source whose configuration cannot be used is a client error, not a
/// server one.
#[tokio::test]
async fn sync_of_a_misconfigured_source_is_a_bad_request() {
    let h = harness().await;
    let source = CreateSource {
        name: "no-credentials".into(),
        source_type: SourceType::WebDAV,
        enabled: Some(true),
        config: json!({
            "server_url": "http://127.0.0.1:1",
            "username": "",
            "password": "p",
            "watch_folders": ["/Documents"],
            "file_extensions": ["pdf"],
            "auto_sync": false,
            "sync_interval_minutes": 60,
            "server_type": "generic"
        }),
    };
    let id = h.ctx.state().db.create_source(h.user_id, &source).await.unwrap().id;

    let (status, _) = h.send("POST", &format!("/api/sources/{id}/sync"), None).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(!h.scheduler.is_running(id));
    let _ = h.ctx.cleanup_and_close().await;
}
