//! Files that live directly inside a watch folder are detected through the
//! folder's own ETag, not only through its subdirectories' ETags. Smart sync
//! records nothing itself: each test commits the returned directory rows,
//! as the sync pipeline does once the folder's files are processed.

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
    service_watching(server, "/Documents")
}

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
            username: format!("rootfiles_{}", Uuid::new_v4().simple()),
            email: format!("rootfiles_{}@example.com", Uuid::new_v4().simple()),
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

/// Root listing: the folder itself with `root_etag`, one unchanged subdirectory and one file.
async fn mount_root_listing(server: &MockServer, root_etag: &str) {
    Mock::given(method("PROPFIND"))
        .and(path("/Documents"))
        .respond_with(ResponseTemplate::new(207).set_body_string(multistatus(&[
            entry("/Documents/", root_etag, true),
            entry("/Documents/Sub/", "sub-v1", true),
            entry("/Documents/new.pdf", "f-new", false),
        ])))
        .mount(server)
        .await;
    // The unchanged subdirectory must not be rescanned.
    Mock::given(method("PROPFIND"))
        .and(path("/Documents/Sub/"))
        .respond_with(ResponseTemplate::new(207).set_body_string(multistatus(&[entry("/Documents/Sub/", "sub-v1", true)])))
        .expect(0)
        .mount(server)
        .await;
}

#[tokio::test]
async fn changed_folder_etag_returns_files_directly_inside_it() {
    let ctx = TestContext::new().await;
    let server = MockServer::start().await;
    let result: Result<()> = async {
        let user_id = create_user(&ctx).await;
        let db = &ctx.state.db;
        db.bulk_create_or_update_webdav_directories(&[
            dir(user_id, "/Documents", "root-v1"),
            dir(user_id, "/Documents/Sub/", "sub-v1"),
        ])
        .await?;
        mount_root_listing(&server, "root-v2").await;

        let service = service_for(&server);
        let smart_sync = SmartSyncService::new(ctx.state.clone());
        let outcome = smart_sync.evaluate_and_sync_detailed(user_id, None, &service, "/Documents", None).await?;

        assert!(matches!(outcome.strategy, Some(SmartSyncStrategy::TargetedScan(_))), "{:?}", outcome.strategy);
        let names: Vec<&str> = outcome.files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["new.pdf"]);
        assert!(outcome.failed_directories.is_empty());

        // Once committed, the folder's new ETag means the next evaluation has nothing to do.
        outcome.directory_commit.commit(db).await?;
        let stored = db.list_webdav_directories(user_id).await?;
        let root = stored.iter().find(|d| d.directory_path == "/Documents").expect("root row");
        assert_eq!(root.directory_etag, "root-v2");

        let again = smart_sync.evaluate_and_sync_detailed(user_id, None, &service, "/Documents", None).await?;
        assert!(again.strategy.is_none(), "{:?}", again.strategy);
        assert!(again.files.is_empty());
        Ok(())
    }
    .await;

    if let Err(e) = ctx.cleanup_and_close().await {
        eprintln!("Warning: Test cleanup failed: {}", e);
    }
    result.unwrap();
}

#[tokio::test]
async fn unknown_folder_etag_lists_root_files_once() {
    // Databases from before folder ETags were stored have only subdirectory rows.
    let ctx = TestContext::new().await;
    let server = MockServer::start().await;
    let result: Result<()> = async {
        let user_id = create_user(&ctx).await;
        ctx.state
            .db
            .bulk_create_or_update_webdav_directories(&[dir(user_id, "/Documents/Sub/", "sub-v1")])
            .await?;
        mount_root_listing(&server, "root-v1").await;

        let service = service_for(&server);
        let smart_sync = SmartSyncService::new(ctx.state.clone());
        let outcome = smart_sync.evaluate_and_sync_detailed(user_id, None, &service, "/Documents", None).await?;
        assert_eq!(outcome.files.len(), 1);
        outcome.directory_commit.commit(&ctx.state.db).await?;

        let again = smart_sync.evaluate_and_sync_detailed(user_id, None, &service, "/Documents", None).await?;
        assert!(again.strategy.is_none(), "{:?}", again.strategy);
        Ok(())
    }
    .await;

    if let Err(e) = ctx.cleanup_and_close().await {
        eprintln!("Warning: Test cleanup failed: {}", e);
    }
    result.unwrap();
}

#[tokio::test]
async fn unchanged_folder_and_subdirectories_skip_sync() {
    let ctx = TestContext::new().await;
    let server = MockServer::start().await;
    let result: Result<()> = async {
        let user_id = create_user(&ctx).await;
        ctx.state
            .db
            .bulk_create_or_update_webdav_directories(&[
                dir(user_id, "/Documents", "root-v1"),
                dir(user_id, "/Documents/Sub/", "sub-v1"),
            ])
            .await?;
        mount_root_listing(&server, "root-v1").await;

        let service = service_for(&server);
        let smart_sync = SmartSyncService::new(ctx.state.clone());
        let outcome = smart_sync.evaluate_and_sync_detailed(user_id, None, &service, "/Documents", None).await?;
        assert!(outcome.strategy.is_none(), "{:?}", outcome.strategy);
        assert!(outcome.files.is_empty());
        Ok(())
    }
    .await;

    if let Err(e) = ctx.cleanup_and_close().await {
        eprintln!("Warning: Test cleanup failed: {}", e);
    }
    result.unwrap();
}

#[tokio::test]
async fn deep_scan_records_the_folder_etag() {
    let ctx = TestContext::new().await;
    let server = MockServer::start().await;
    let result: Result<()> = async {
        let user_id = create_user(&ctx).await;
        Mock::given(method("PROPFIND"))
            .and(path("/Documents"))
            .respond_with(ResponseTemplate::new(207).set_body_string(multistatus(&[
                entry("/Documents/", "root-v1", true),
                entry("/Documents/Sub/", "sub-v1", true),
            ])))
            .mount(&server)
            .await;
        Mock::given(method("PROPFIND"))
            .and(path("/Documents/Sub/"))
            .respond_with(ResponseTemplate::new(207).set_body_string(multistatus(&[entry("/Documents/Sub/", "sub-v1", true)])))
            .mount(&server)
            .await;

        let service = service_for(&server);
        let smart_sync = SmartSyncService::new(ctx.state.clone());
        let first = smart_sync.evaluate_and_sync_detailed(user_id, None, &service, "/Documents", None).await?;
        assert!(matches!(first.strategy, Some(SmartSyncStrategy::FullDeepScan)));
        first.directory_commit.commit(&ctx.state.db).await?;

        let stored = ctx.state.db.list_webdav_directories(user_id).await?;
        let root = stored.iter().find(|d| d.directory_path == "/Documents").expect("root row after deep scan");
        assert_eq!(root.directory_etag, "root-v1");

        let second = smart_sync.evaluate_and_sync_detailed(user_id, None, &service, "/Documents", None).await?;
        assert!(second.strategy.is_none(), "{:?}", second.strategy);
        Ok(())
    }
    .await;

    if let Err(e) = ctx.cleanup_and_close().await {
        eprintln!("Warning: Test cleanup failed: {}", e);
    }
    result.unwrap();
}

/// Listing of `folder` with its own entry and one file, and no subdirectories.
async fn mount_flat_listing(server: &MockServer, request_path: &str, href_dir: &str, file: &str) {
    Mock::given(method("PROPFIND"))
        .and(path(request_path))
        .respond_with(ResponseTemplate::new(207).set_body_string(multistatus(&[
            entry(&format!("{href_dir}/"), "root-v1", true),
            entry(&format!("{href_dir}/{file}"), "f-1", false),
        ])))
        .expect(1)
        .mount(server)
        .await;
}

/// A watch folder configured without a leading slash still matches the
/// folder's own href entry: its ETag is stored under `/Documents` and the
/// root is listed once, not again as a pseudo-subdirectory.
#[tokio::test]
async fn watch_folder_without_leading_slash_matches_its_own_listing() {
    let ctx = TestContext::new().await;
    let server = MockServer::start().await;
    let result: Result<()> = async {
        let user_id = create_user(&ctx).await;
        mount_flat_listing(&server, "/Documents", "/Documents", "new.pdf").await;

        let service = service_for(&server);
        let smart_sync = SmartSyncService::new(ctx.state.clone());
        let outcome = smart_sync.evaluate_and_sync_detailed(user_id, None, &service, "Documents", None).await?;
        outcome.directory_commit.commit(&ctx.state.db).await?;

        let names: Vec<&str> = outcome.files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["new.pdf"], "the root must be listed exactly once");
        let stored = ctx.state.db.list_webdav_directories(user_id).await?;
        let root = stored.iter().find(|d| d.directory_path == "/Documents").expect("root row under /Documents");
        assert_eq!(root.directory_etag, "root-v1");
        assert!(!stored.iter().any(|d| d.directory_path == "Documents"), "{stored:?}");
        Ok(())
    }
    .await;

    server.verify().await;
    if let Err(e) = ctx.cleanup_and_close().await {
        eprintln!("Warning: Test cleanup failed: {}", e);
    }
    result.unwrap();
}

/// A watch folder that was configured percent-encoded keeps working: the
/// service decodes it once when it is built, it is requested encoded once,
/// and stored decoded.
#[tokio::test]
async fn percent_encoded_watch_folder_is_decoded_once() {
    let ctx = TestContext::new().await;
    let server = MockServer::start().await;
    let result: Result<()> = async {
        let user_id = create_user(&ctx).await;
        mount_flat_listing(&server, "/My%20Docs", "/My%20Docs", "a.pdf").await;

        let service = service_watching(&server, "/My%20Docs");
        let folder = service.get_config().watch_folders[0].clone();
        assert_eq!(folder, "/My Docs");
        let smart_sync = SmartSyncService::new(ctx.state.clone());
        let outcome = smart_sync.evaluate_and_sync_detailed(user_id, None, &service, &folder, None).await?;
        outcome.directory_commit.commit(&ctx.state.db).await?;

        let names: Vec<&str> = outcome.files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["a.pdf"]);
        assert_eq!(outcome.files[0].relative_path, "/My Docs/a.pdf");
        let stored = ctx.state.db.list_webdav_directories(user_id).await?;
        assert!(stored.iter().any(|d| d.directory_path == "/My Docs"), "{stored:?}");
        Ok(())
    }
    .await;

    server.verify().await;
    if let Err(e) = ctx.cleanup_and_close().await {
        eprintln!("Warning: Test cleanup failed: {}", e);
    }
    result.unwrap();
}
