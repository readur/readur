//! MCP status, source and resource tools: settings, queue stats, OCR
//! languages, sources (never exposing credentials) and the
//! `readur://documents/{id}` file resource.

use axum::http::StatusCode;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

use readur::{
    models::{CreateSource, SourceType, UpdateSettings},
    test_utils::{
        document_helpers, TestConfigBuilder, TestContext, TestUser, MCP_TEST_PROTOCOL_VERSION,
    },
};

async fn login(ctx: &TestContext, user: &TestUser) -> String {
    ctx.auth_helper()
        .login_user(&user.username, &user.password)
        .await
}

fn user_id(user: &TestUser) -> Uuid {
    user.user_response.id
}

fn text(result: &Value) -> &str {
    result["content"][0]["text"].as_str().unwrap_or_default()
}

/// `resources/read` needs an `Mcp-Name` header carrying the URI, which the
/// shared `mcp_post` helper only derives from `params.name`.
async fn read_resource(ctx: &TestContext, token: &str, uri: &str) -> (StatusCode, Value) {
    let message = json!({
        "jsonrpc": "2.0", "id": 1, "method": "resources/read",
        "params": {
            "uri": uri,
            "_meta": {
                "io.modelcontextprotocol/protocolVersion": MCP_TEST_PROTOCOL_VERSION,
                "io.modelcontextprotocol/clientInfo": { "name": "readur-tests", "version": "1.0" },
                "io.modelcontextprotocol/clientCapabilities": {}
            }
        }
    });
    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/api/mcp")
        .header("content-type", "application/json")
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", MCP_TEST_PROTOCOL_VERSION)
        .header("mcp-method", "resources/read")
        .header("mcp-name", uri)
        .header("authorization", format!("Bearer {token}"))
        .body(axum::body::Body::from(message.to_string()))
        .unwrap();
    let response = ctx.app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or_default())
}

/// Store `data` as a document owned by `owner` and return its id.
async fn create_file_document(
    ctx: &TestContext,
    owner: Uuid,
    filename: &str,
    mime_type: &str,
    data: &[u8],
) -> Uuid {
    let state = ctx.state();
    let mut document = document_helpers::create_test_document(owner);
    document.file_path = state
        .file_service
        .save_document_file(owner, document.id, filename, data)
        .await
        .expect("save test file");
    document.filename = filename.to_string();
    document.original_filename = filename.to_string();
    document.mime_type = mime_type.to_string();
    document.file_size = data.len() as i64;
    document.file_hash = Some(Uuid::new_v4().to_string());
    state.db.create_document(document).await.unwrap().id
}

async fn create_webdav_source(ctx: &TestContext, owner: Uuid, name: &str) -> Uuid {
    let source = CreateSource {
        name: name.to_string(),
        source_type: SourceType::WebDAV,
        enabled: Some(false),
        config: json!({
            "server_url": "https://dav.example.com",
            "username": "alice",
            "password": "super-secret-dav-password",
            "watch_folders": ["/Documents"],
            "file_extensions": ["pdf"],
            "auto_sync": false,
            "sync_interval_minutes": 60,
            "server_type": "generic"
        }),
    };
    ctx.state()
        .db
        .create_source(owner, &source)
        .await
        .unwrap()
        .id
}

#[tokio::test]
async fn get_settings_returns_callers_settings() {
    let ctx = TestContext::new().await;
    let user = ctx.auth_helper().create_test_user().await;
    let token = login(&ctx, &user).await;

    let result = ctx.mcp_call_tool(&token, "get_settings", json!({})).await;
    assert_eq!(result["isError"], json!(false), "{result}");
    assert_eq!(result["structuredContent"]["ocr_language"], json!("eng"));

    let update: UpdateSettings = serde_json::from_value(json!({ "ocr_language": "deu" })).unwrap();
    ctx.state()
        .db
        .create_or_update_settings(user_id(&user), &update)
        .await
        .unwrap();

    let result = ctx.mcp_call_tool(&token, "get_settings", json!({})).await;
    assert_eq!(result["structuredContent"]["ocr_language"], json!("deu"));
    assert!(text(&result).contains("deu"), "{}", text(&result));
    // Only whether a WebDAV password is set is ever reported.
    assert!(result["structuredContent"].get("webdav_password").is_none());
}

#[tokio::test]
async fn queue_stats_are_admin_only() {
    let ctx = TestContext::new().await;
    let auth = ctx.auth_helper();
    let user = auth.create_test_user().await;
    let admin = auth.create_admin_user().await;

    let result = ctx
        .mcp_call_tool(&login(&ctx, &user).await, "get_queue_stats", json!({}))
        .await;
    assert_eq!(result["isError"], json!(true), "{result}");

    let result = ctx
        .mcp_call_tool(&login(&ctx, &admin).await, "get_queue_stats", json!({}))
        .await;
    assert_eq!(result["isError"], json!(false), "{result}");
    assert!(
        result["structuredContent"]["pending"].is_number(),
        "{result}"
    );
}

#[tokio::test]
async fn ocr_languages_are_listed() {
    let ctx = TestContext::new().await;
    let user = ctx.auth_helper().create_test_user().await;
    let token = login(&ctx, &user).await;

    let result = ctx
        .mcp_call_tool(&token, "get_ocr_languages", json!({}))
        .await;
    assert_eq!(result["isError"], json!(false), "{result}");
    let languages = result["structuredContent"]["available_languages"]
        .as_array()
        .unwrap();
    assert!(!languages.is_empty());
    assert_eq!(
        result["structuredContent"]["current_user_language"],
        json!("eng")
    );
}

#[tokio::test]
async fn list_sources_shows_own_sources_without_secrets() {
    let ctx = TestContext::new().await;
    let user = ctx.auth_helper().create_test_user().await;
    let token = login(&ctx, &user).await;

    let result = ctx.mcp_call_tool(&token, "list_sources", json!({})).await;
    assert_eq!(result["isError"], json!(false), "{result}");
    assert_eq!(result["structuredContent"]["sources"], json!([]));

    let source_id = create_webdav_source(&ctx, user_id(&user), "Office NAS").await;
    let result = ctx.mcp_call_tool(&token, "list_sources", json!({})).await;
    let sources = result["structuredContent"]["sources"].as_array().unwrap();
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0]["id"], json!(source_id.to_string()));
    assert_eq!(sources[0]["config"]["has_password"], json!(true));
    assert!(sources[0]["config"].get("password").is_none());
    assert!(text(&result).contains("Office NAS"));
    assert!(!result.to_string().contains("super-secret-dav-password"));
}

#[tokio::test]
async fn source_sync_status_is_scoped_to_owner() {
    let ctx = TestContext::new().await;
    let auth = ctx.auth_helper();
    let owner = auth.create_test_user().await;
    let other = auth.create_test_user().await;
    let source_id = create_webdav_source(&ctx, user_id(&owner), "Owner source").await;

    let result = ctx
        .mcp_call_tool(
            &login(&ctx, &owner).await,
            "get_source_sync_status",
            json!({ "source_id": source_id }),
        )
        .await;
    assert_eq!(result["isError"], json!(false), "{result}");
    assert_eq!(
        result["structuredContent"]["sync_in_progress"],
        json!(false)
    );
    assert!(!result.to_string().contains("super-secret-dav-password"));

    let result = ctx
        .mcp_call_tool(
            &login(&ctx, &other).await,
            "get_source_sync_status",
            json!({ "source_id": source_id }),
        )
        .await;
    assert_eq!(result["isError"], json!(true), "{result}");
}

#[tokio::test]
async fn resource_templates_list_document_template() {
    let ctx = TestContext::new().await;
    let user = ctx.auth_helper().create_test_user().await;
    let token = login(&ctx, &user).await;

    let result = ctx
        .mcp_request(&token, "resources/templates/list", json!({}))
        .await;
    let templates = result["resourceTemplates"].as_array().unwrap();
    assert!(
        templates
            .iter()
            .any(|t| t["uriTemplate"] == json!("readur://documents/{id}")),
        "{result}"
    );
}

#[tokio::test]
async fn resource_read_returns_own_files_only() {
    let upload_dir = tempfile::tempdir().expect("temp upload dir");
    let ctx = TestContext::with_config(
        TestConfigBuilder::default().with_upload_path(upload_dir.path().to_str().unwrap()),
    )
    .await;
    let auth = ctx.auth_helper();
    let owner = auth.create_test_user().await;
    let other = auth.create_test_user().await;

    let text_id = create_file_document(
        &ctx,
        user_id(&owner),
        "note.txt",
        "text/plain",
        b"hello from a resource",
    )
    .await;
    let binary = [0u8, 159, 146, 150, 1, 2, 3];
    let binary_id =
        create_file_document(&ctx, user_id(&owner), "blob.png", "image/png", &binary).await;
    let owner_token = login(&ctx, &owner).await;

    let uri = format!("readur://documents/{text_id}");
    let (status, body) = read_resource(&ctx, &owner_token, &uri).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let contents = &body["result"]["contents"][0];
    assert_eq!(contents["text"], json!("hello from a resource"), "{body}");
    assert_eq!(contents["mimeType"], json!("text/plain"));
    assert_eq!(contents["uri"], json!(uri));

    let uri = format!("readur://documents/{binary_id}");
    let (_, body) = read_resource(&ctx, &owner_token, &uri).await;
    let contents = &body["result"]["contents"][0];
    use base64ct::{Base64, Encoding};
    assert_eq!(
        contents["blob"],
        json!(Base64::encode_string(&binary)),
        "{body}"
    );
    assert_eq!(contents["mimeType"], json!("image/png"));

    // Another user gets an error, not the file.
    let uri = format!("readur://documents/{text_id}");
    let (_, body) = read_resource(&ctx, &login(&ctx, &other).await, &uri).await;
    assert!(body["error"].is_object(), "{body}");
    assert!(!body.to_string().contains("hello from a resource"));

    // Malformed URIs are rejected.
    let (_, body) = read_resource(&ctx, &owner_token, "readur://documents/not-a-uuid").await;
    assert!(body["error"].is_object(), "{body}");
}

#[tokio::test]
async fn sync_status_rejects_non_uuid_source_id() {
    let ctx = TestContext::new().await;
    let user = ctx.auth_helper().create_test_user().await;
    let token = login(&ctx, &user).await;

    let message = json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {
            "name": "get_source_sync_status",
            "arguments": { "source_id": "not-a-uuid" },
            "_meta": {
                "io.modelcontextprotocol/protocolVersion": MCP_TEST_PROTOCOL_VERSION,
                "io.modelcontextprotocol/clientInfo": { "name": "readur-tests", "version": "1.0" },
                "io.modelcontextprotocol/clientCapabilities": {}
            }
        }
    });
    let (_, body) = ctx.mcp_post(Some(&token), message).await;
    // Either a JSON-RPC invalid-params error or a tool error, never a result.
    let rejected = body["error"].is_object() || body["result"]["isError"] == json!(true);
    assert!(rejected, "{body}");
}
