//! A configured watch folder is percent-decoded exactly once, when the
//! service is built, and the decoded path is what every later step uses: a
//! folder literally named `a%20b` is configured as `/a%2520b`, requested as
//! `/a%2520b` on the wire, and never turns into `/a b` through repeated
//! normalisation in evaluation, scanning or discovery.

use anyhow::Result;
use readur::{
    models::{CreateUser, UserRole},
    services::webdav::{RetryConfig, SmartSyncService, SmartSyncStrategy, WebDAVConfig, WebDAVService},
    test_utils::TestContext,
};
use uuid::Uuid;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

fn service_watching(server: &MockServer, watch_folder: &str) -> WebDAVService {
    let config = WebDAVConfig {
        server_url: server.uri(),
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        watch_folders: vec![watch_folder.to_string()],
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

async fn create_user(ctx: &TestContext) -> Uuid {
    ctx.state
        .db
        .create_user(CreateUser {
            username: format!("decodeonce_{}", Uuid::new_v4().simple()),
            email: format!("decodeonce_{}@example.com", Uuid::new_v4().simple()),
            password: "password123".to_string(),
            role: Some(UserRole::User),
        })
        .await
        .expect("create user")
        .id
}

/// The folder `a%20b` (which the server addresses as `/a%2520b`) with one
/// subdirectory and one file; the paths a double decode would produce are
/// mounted with an expectation of zero requests.
async fn mount_listings(server: &MockServer) {
    Mock::given(method("PROPFIND"))
        .and(path("/a%2520b"))
        .respond_with(ResponseTemplate::new(207).set_body_string(multistatus(&[
            entry("/a%2520b/", "root-v1", true),
            entry("/a%2520b/Sub/", "sub-v1", true),
            entry("/a%2520b/x.pdf", "f-1", false),
        ])))
        .expect(1..)
        .mount(server)
        .await;
    Mock::given(method("PROPFIND"))
        .and(path("/a%2520b/Sub/"))
        .respond_with(ResponseTemplate::new(207).set_body_string(multistatus(&[
            entry("/a%2520b/Sub/", "sub-v1", true),
            entry("/a%2520b/Sub/y.pdf", "f-2", false),
        ])))
        .expect(1..)
        .mount(server)
        .await;
    for wrong in ["/a%20b", "/a%20b/Sub/", "/a b", "/a b/Sub/"] {
        Mock::given(method("PROPFIND"))
            .and(path(wrong))
            .respond_with(ResponseTemplate::new(207).set_body_string(multistatus(&[])))
            .expect(0)
            .mount(server)
            .await;
    }
}

#[tokio::test]
async fn encoded_watch_folder_is_requested_as_configured_on_every_run() {
    let ctx = TestContext::new().await;
    let server = MockServer::start().await;
    let result: Result<()> = async {
        let user_id = create_user(&ctx).await;
        mount_listings(&server).await;

        let service = service_watching(&server, "/a%2520b/");
        let folder = service.get_config().watch_folders[0].clone();
        assert_eq!(folder, "/a%20b", "decoded exactly once when the service is built");

        let smart_sync = SmartSyncService::new(ctx.state.clone());
        // First run: nothing known, full scan of the folder and its subdirectory.
        let first = smart_sync.evaluate_and_sync_detailed(user_id, None, &service, &folder, None).await?;
        assert!(matches!(first.strategy, Some(SmartSyncStrategy::FullDeepScan)), "{:?}", first.strategy);
        let mut paths: Vec<&str> = first.files.iter().map(|f| f.relative_path.as_str()).collect();
        paths.sort();
        assert_eq!(paths, vec!["/a%20b/Sub/y.pdf", "/a%20b/x.pdf"]);
        first.directory_commit.commit(&ctx.state.db).await?;

        // Later runs go through evaluation and discovery again with the same
        // folder string and must still request the same wire path.
        let again = smart_sync.evaluate_and_sync_detailed(user_id, None, &service, &folder, None).await?;
        assert!(again.failed_directories.is_empty(), "{:?}", again.failed_directories);

        // The same holds for a forced deep scan, which the deep-scan route uses.
        let forced = smart_sync
            .perform_smart_sync(user_id, None, &service, &folder, SmartSyncStrategy::FullDeepScan, None)
            .await?;
        assert_eq!(forced.files.len(), 2);
        Ok(())
    }
    .await;

    server.verify().await;
    if let Err(e) = ctx.cleanup_and_close().await {
        eprintln!("Warning: Test cleanup failed: {}", e);
    }
    result.unwrap();
}
