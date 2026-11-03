//! A server that reports neither `getetag` nor `getlastmodified` gives smart
//! sync nothing to compare, so its folders are scanned on every evaluation
//! instead of being skipped as unchanged.

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

/// An entry with no `getetag` and no `getlastmodified`.
fn entry(href: &str, is_dir: bool) -> String {
    let resourcetype = if is_dir { "<d:resourcetype><d:collection/></d:resourcetype>" } else { "<d:resourcetype/>" };
    format!(
        r#"<d:response><d:href>{href}</d:href><d:propstat><d:prop>
            {resourcetype}<d:getcontentlength>10</d:getcontentlength>
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
            username: format!("nochangeinfo_{}", Uuid::new_v4().simple()),
            email: format!("nochangeinfo_{}@example.com", Uuid::new_v4().simple()),
            password: "password123".to_string(),
            role: Some(UserRole::User),
        })
        .await
        .expect("create user")
        .id
}

async fn mount_listings(server: &MockServer) {
    Mock::given(method("PROPFIND"))
        .and(path("/Documents"))
        .respond_with(ResponseTemplate::new(207).set_body_string(multistatus(&[
            entry("/Documents/", true),
            entry("/Documents/Sub/", true),
            entry("/Documents/root.pdf", false),
        ])))
        .mount(server)
        .await;
    Mock::given(method("PROPFIND"))
        .and(path("/Documents/Sub/"))
        .respond_with(ResponseTemplate::new(207).set_body_string(multistatus(&[
            entry("/Documents/Sub/", true),
            entry("/Documents/Sub/nested.pdf", false),
        ])))
        .mount(server)
        .await;
}

#[tokio::test]
async fn folders_without_change_information_are_scanned_every_time() {
    let ctx = TestContext::new().await;
    let server = MockServer::start().await;
    let result: Result<()> = async {
        let user_id = create_user(&ctx).await;
        mount_listings(&server).await;
        let service = service_for(&server);
        let smart_sync = SmartSyncService::new(ctx.state.clone());

        let first = smart_sync.evaluate_and_sync_detailed(user_id, None, &service, "/Documents", None).await?;
        assert!(matches!(first.strategy, Some(SmartSyncStrategy::FullDeepScan)), "{:?}", first.strategy);
        let mut names: Vec<&str> = first.files.iter().map(|f| f.name.as_str()).collect();
        names.sort();
        assert_eq!(names, vec!["nested.pdf", "root.pdf"]);

        // The rows written by the first run carry no usable change information.
        let stored = ctx.state.db.list_webdav_directories(user_id).await?;
        assert!(!stored.is_empty());
        for row in &stored {
            assert!(row.directory_etag.starts_with("fallback-unknown-"), "{row:?}");
        }

        for _ in 0..2 {
            let again = smart_sync.evaluate_and_sync_detailed(user_id, None, &service, "/Documents", None).await?;
            assert!(again.strategy.is_some(), "a folder without change information is never skipped");
            let mut names: Vec<&str> = again.files.iter().map(|f| f.name.as_str()).collect();
            names.sort();
            assert_eq!(names, vec!["nested.pdf", "root.pdf"]);
        }
        Ok(())
    }
    .await;

    if let Err(e) = ctx.cleanup_and_close().await {
        eprintln!("Warning: Test cleanup failed: {}", e);
    }
    result.unwrap();
}
