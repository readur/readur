//! A subdirectory whose listing fails must not be recorded as scanned: its
//! stored state is kept, healthy siblings are still returned, and the failure
//! is reported to the caller. A failing root listing is an error.

use anyhow::Result;
use readur::{
    models::{CreateUser, CreateWebDAVDirectory, UserRole},
    services::webdav::{RetryConfig, SmartSyncService, SmartSyncStrategy, WebDAVConfig, WebDAVService},
    test_utils::TestContext,
};
use uuid::Uuid;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

fn service_for(server: &MockServer) -> WebDAVService {
    let config = WebDAVConfig {
        server_url: server.uri(),
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        watch_folders: vec!["/Documents".to_string()],
        file_extensions: vec![],
        timeout_seconds: 5,
        server_type: Some("generic".to_string()),
    };
    let retry = RetryConfig { max_retries: 0, initial_delay_ms: 1, max_delay_ms: 1, rate_limit_backoff_ms: 1, ..RetryConfig::default() };
    WebDAVService::new_with_retry(config, retry).expect("service")
}

fn entry(href: &str, etag: &str, is_dir: bool) -> String {
    let resourcetype = if is_dir { "<d:resourcetype><d:collection/></d:resourcetype>" } else { "<d:resourcetype/>" };
    format!(
        r#"<d:response><d:href>{href}</d:href><d:propstat><d:prop>
            <d:getlastmodified>Tue, 29 Jul 2025 01:34:17 GMT</d:getlastmodified>
            <d:getetag>"{etag}"</d:getetag>{resourcetype}<d:getcontentlength>10</d:getcontentlength>
        </d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response>"#
    )
}

fn multistatus(entries: &[String]) -> String {
    format!(r#"<?xml version="1.0"?><d:multistatus xmlns:d="DAV:">{}</d:multistatus>"#, entries.join(""))
}

async fn propfind(server: &MockServer, at: &str, response: ResponseTemplate) {
    Mock::given(method("PROPFIND")).and(path(at)).respond_with(response).mount(server).await;
}

async fn create_user(ctx: &TestContext) -> Uuid {
    ctx.state
        .db
        .create_user(CreateUser {
            username: format!("partial_{}", Uuid::new_v4().simple()),
            email: format!("partial_{}@example.com", Uuid::new_v4().simple()),
            password: "password123".to_string(),
            role: Some(UserRole::User),
        })
        .await
        .expect("create user")
        .id
}

fn dir(user_id: Uuid, path: &str, etag: &str) -> CreateWebDAVDirectory {
    CreateWebDAVDirectory { user_id, directory_path: path.to_string(), directory_etag: etag.to_string(), file_count: 0, total_size_bytes: 0 }
}

#[tokio::test]
async fn failed_subdirectory_is_reported_and_not_recorded_as_scanned() {
    let ctx = TestContext::new().await;
    let server = MockServer::start().await;
    let result: Result<()> = async {
        let user_id = create_user(&ctx).await;
        let db = &ctx.state.db;

        // Stale rows that a complete scan would prune; an incomplete one must keep them.
        db.bulk_create_or_update_webdav_directories(&[
            dir(user_id, "/Documents/Sub2/old/", "stale-old"),
            dir(user_id, "/Documents/Gone/", "stale-gone"),
        ])
        .await?;

        propfind(&server, "/Documents", ResponseTemplate::new(207).set_body_string(multistatus(&[
            entry("/Documents/", "root-v1", true),
            entry("/Documents/Sub1/", "sub1-v1", true),
            entry("/Documents/Sub2/", "sub2-v1", true),
            entry("/Documents/root.txt", "f-root", false),
        ]))).await;
        propfind(&server, "/Documents/Sub1/", ResponseTemplate::new(207).set_body_string(multistatus(&[
            entry("/Documents/Sub1/", "sub1-v1", true),
            entry("/Documents/Sub1/a.pdf", "f-a", false),
        ]))).await;
        propfind(&server, "/Documents/Sub2/", ResponseTemplate::new(500)).await;

        let service = service_for(&server);
        let smart_sync = SmartSyncService::new(ctx.state.clone());
        let outcome = smart_sync
            .evaluate_and_sync_detailed(user_id, None, &service, "/Documents", None)
            .await?;

        assert!(matches!(outcome.strategy, Some(SmartSyncStrategy::FullDeepScan)), "{:?}", outcome.strategy);
        assert_eq!(outcome.failed_directories, vec!["/Documents/Sub2/".to_string()]);
        let mut names: Vec<&str> = outcome.files.iter().map(|f| f.name.as_str()).collect();
        names.sort();
        assert_eq!(names, vec!["a.pdf", "root.txt"]);

        let stored: std::collections::HashMap<String, String> = db
            .list_webdav_directories(user_id)
            .await?
            .into_iter()
            .map(|d| (d.directory_path, d.directory_etag))
            .collect();
        assert_eq!(stored.get("/Documents/Sub1/").map(String::as_str), Some("sub1-v1"));
        assert!(!stored.contains_key("/Documents/Sub2/"), "failed directory must not be recorded: {stored:?}");
        assert_eq!(stored.get("/Documents/Sub2/old/").map(String::as_str), Some("stale-old"));
        assert_eq!(stored.get("/Documents/Gone/").map(String::as_str), Some("stale-gone"));
        Ok(())
    }
    .await;

    if let Err(e) = ctx.cleanup_and_close().await {
        eprintln!("Warning: Test cleanup failed: {}", e);
    }
    result.unwrap();
}

#[tokio::test]
async fn failed_root_listing_is_an_error_not_an_empty_sync() {
    let ctx = TestContext::new().await;
    let server = MockServer::start().await;
    let result: Result<()> = async {
        let user_id = create_user(&ctx).await;
        propfind(&server, "/Documents", ResponseTemplate::new(500)).await;

        let service = service_for(&server);
        let smart_sync = SmartSyncService::new(ctx.state.clone());
        let outcome = smart_sync
            .evaluate_and_sync_detailed(user_id, None, &service, "/Documents", None)
            .await;
        assert!(outcome.is_err(), "root failure must surface as an error, got {:?}", outcome.map(|o| o.files.len()));

        // The thin wrapper must agree.
        let legacy = smart_sync.evaluate_and_sync(user_id, None, &service, "/Documents", None).await;
        assert!(legacy.is_err());
        Ok(())
    }
    .await;

    if let Err(e) = ctx.cleanup_and_close().await {
        eprintln!("Warning: Test cleanup failed: {}", e);
    }
    result.unwrap();
}
