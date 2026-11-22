//! The built-in MCP endpoint at `/api/mcp`: transport, authentication and
//! configuration. Tool behaviour is covered per area in
//! `integration_mcp_*_tests.rs`.

use axum::http::StatusCode;
use serde_json::json;

use readur::test_utils::{TestConfigBuilder, TestContext};

async fn user_token(ctx: &TestContext) -> String {
    let auth = ctx.auth_helper();
    let user = auth.create_test_user().await;
    auth.login_user(&user.username, &user.password).await
}

fn tools_list() -> serde_json::Value {
    json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {} })
}

#[tokio::test]
async fn rejects_requests_without_credentials() {
    let ctx = TestContext::new().await;
    let (status, _) = ctx.mcp_post(None, tools_list()).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn rejects_invalid_bearer_tokens() {
    let ctx = TestContext::new().await;
    for token in ["not-a-jwt", "readur_pat_doesnotexist0000000000000000"] {
        let (status, _) = ctx.mcp_post(Some(token), tools_list()).await;
        assert_eq!(
            status,
            StatusCode::UNAUTHORIZED,
            "token {token} was accepted"
        );
    }
}

#[tokio::test]
async fn whoami_works_with_session_jwt() {
    let ctx = TestContext::new().await;
    let auth = ctx.auth_helper();
    let user = auth.create_test_user().await;
    let token = auth.login_user(&user.username, &user.password).await;

    let result = ctx.mcp_call_tool(&token, "whoami", json!({})).await;
    assert_eq!(result["isError"], json!(false));
    assert_eq!(
        result["structuredContent"]["username"],
        json!(user.username)
    );
    assert!(result["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains(&user.username));
}

#[tokio::test]
async fn whoami_works_with_api_key() {
    let ctx = TestContext::new().await;
    let auth = ctx.auth_helper();
    let user = auth.create_test_user().await;
    let jwt = auth.login_user(&user.username, &user.password).await;
    let api_key = ctx.create_api_key(&jwt).await;
    assert!(api_key.starts_with("readur_pat_"));

    let result = ctx.mcp_call_tool(&api_key, "whoami", json!({})).await;
    assert_eq!(
        result["structuredContent"]["username"],
        json!(user.username)
    );
}

#[tokio::test]
async fn legacy_clients_can_initialize_then_list_tools() {
    // Clients on pre-2026-07-28 revisions (e.g. 2025-06-18) still send
    // `initialize` first; the stateless server must answer both requests.
    let ctx = TestContext::new().await;
    let token = user_token(&ctx).await;

    let init = json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": { "name": "legacy-client", "version": "1.0" }
        }
    });
    let (status, body) = ctx.mcp_post(Some(&token), init).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["result"]["serverInfo"]["name"], json!("readur"));
    assert!(body["result"]["capabilities"]["tools"].is_object());

    let list = json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": { "protocolVersion": "2025-06-18" } });
    let (status, body) = ctx.mcp_post(Some(&token), list).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let names: Vec<&str> = body["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"whoami"), "{names:?}");
}

#[tokio::test]
async fn delete_tool_hidden_unless_enabled() {
    let ctx = TestContext::new().await;
    let token = user_token(&ctx).await;
    let names = ctx.mcp_tool_names(&token).await;
    assert!(names.contains(&"whoami".to_string()));
    assert!(!names.contains(&"delete_document".to_string()));
}

#[tokio::test]
async fn endpoint_is_not_found_when_disabled() {
    let ctx = TestContext::with_config(TestConfigBuilder::default().with_mcp_enabled(false)).await;
    let token = user_token(&ctx).await;
    let (status, _) = ctx.mcp_post(Some(&token), tools_list()).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

async fn auth_config_mcp_enabled(ctx: &TestContext) -> serde_json::Value {
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    let response = ctx
        .app()
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/auth/config")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let config: serde_json::Value = serde_json::from_slice(&body).unwrap();
    config["mcp_enabled"].clone()
}

#[tokio::test]
async fn public_auth_config_reports_mcp_availability() {
    let ctx = TestContext::new().await;
    assert_eq!(auth_config_mcp_enabled(&ctx).await, json!(true));

    let ctx = TestContext::with_config(TestConfigBuilder::default().with_mcp_enabled(false)).await;
    assert_eq!(auth_config_mcp_enabled(&ctx).await, json!(false));
}
