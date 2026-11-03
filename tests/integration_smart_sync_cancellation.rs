//! A cancelled token stops smart sync before it talks to the server.

use anyhow::Result;
use readur::{
    models::{CreateUser, UserRole},
    services::webdav::{RetryConfig, SmartSyncService, WebDAVConfig, WebDAVService},
    test_utils::TestContext,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn cancelled_smart_sync_returns_error_without_requests() {
    let ctx = TestContext::new().await;
    let server = MockServer::start().await;
    let result: Result<()> = async {
        let user_id = ctx
            .state
            .db
            .create_user(CreateUser {
                username: format!("cancel_{}", Uuid::new_v4().simple()),
                email: format!("cancel_{}@example.com", Uuid::new_v4().simple()),
                password: "password123".to_string(),
                role: Some(UserRole::User),
            })
            .await?
            .id;
        Mock::given(method("PROPFIND")).respond_with(ResponseTemplate::new(207)).expect(0).mount(&server).await;

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
        let token = CancellationToken::new();
        token.cancel();
        let service = WebDAVService::new_with_retry(config, retry)?.with_cancellation_token(token.clone());
        let smart_sync = SmartSyncService::new(ctx.state.clone()).with_cancellation_token(token);

        let err = smart_sync
            .evaluate_and_sync_detailed(user_id, None, &service, "/Documents", None)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("cancelled"), "{err}");
        Ok(())
    }
    .await;

    if let Err(e) = ctx.cleanup_and_close().await {
        eprintln!("Warning: Test cleanup failed: {}", e);
    }
    result.unwrap();
}
