//! A stop request that interrupts a tracked WebDAV directory scan is not a
//! directory failure: nothing is recorded in the error tracker, so the next
//! run does not skip the directory for a cooldown it never earned.

use anyhow::Result;
use readur::{
    models::source_error::ListFailuresQuery,
    models::{CreateUser, UserRole},
    services::source_error_tracker::SourceErrorTracker,
    services::webdav::{RetryConfig, WebDAVConfig, WebDAVService},
    test_utils::TestContext,
    utils::sync_cancelled::is_cancelled,
};
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn a_scan_interrupted_by_a_stop_request_records_no_directory_failure() {
    let ctx = TestContext::new().await;
    let server = MockServer::start().await;
    let result: Result<()> = async {
        let user_id = ctx
            .state
            .db
            .create_user(CreateUser {
                username: format!("cancel_{}", uuid::Uuid::new_v4().simple()),
                email: format!("cancel_{}@example.com", uuid::Uuid::new_v4().simple()),
                password: "password123".to_string(),
                role: Some(UserRole::User),
            })
            .await?
            .id;
        // The server asks for a long wait; the stop request arrives during it.
        Mock::given(method("PROPFIND"))
            .respond_with(ResponseTemplate::new(503).insert_header("Retry-After", "60"))
            .mount(&server)
            .await;

        let config = WebDAVConfig {
            server_url: server.uri(),
            username: "testuser".to_string(),
            password: "testpass".to_string(),
            watch_folders: vec!["/Documents".to_string()],
            file_extensions: vec![],
            timeout_seconds: 5,
            server_type: Some("generic".to_string()),
        };
        let retry = RetryConfig { max_retries: 2, ..RetryConfig::default() };
        let token = CancellationToken::new();
        let service = WebDAVService::new_with_retry(config, retry)?.with_cancellation_token(token.clone());
        let tracker = SourceErrorTracker::new(ctx.state.db.clone());
        let canceller = token.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            canceller.cancel();
        });

        let started = Instant::now();
        let err = tokio::time::timeout(
            Duration::from_secs(5),
            service.discover_files_and_directories_with_error_tracking("/Documents", true, user_id, &tracker, None),
        )
        .await
        .expect("the scan must not wait out the Retry-After delay")
        .unwrap_err();
        assert!(is_cancelled(&err), "{err}");
        assert!(started.elapsed() < Duration::from_secs(1), "took {:?}", started.elapsed());

        let query = ListFailuresQuery { include_resolved: Some(true), include_excluded: Some(true), ..Default::default() };
        let failures = ctx.state.db.list_source_scan_failures(user_id, &query).await?;
        assert!(failures.is_empty(), "a stop request was recorded as a directory failure: {failures:?}");
        Ok(())
    }
    .await;

    if let Err(e) = ctx.cleanup_and_close().await {
        eprintln!("Warning: Test cleanup failed: {}", e);
    }
    result.unwrap();
}
