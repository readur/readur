//! Source configuration access control, credential redaction, outbound URL
//! checks, and headers on served user content.

use axum::{
    body::Body,
    http::{HeaderMap, Request, StatusCode},
};
use readur::test_utils::{document_helpers, TestContext};
use serde_json::{json, Value};
use tower::ServiceExt;

async fn send(ctx: &TestContext, method: &str, uri: &str, token: &str, body: Option<Value>) -> (StatusCode, HeaderMap, Vec<u8>) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {}", token));
    let body = match body {
        Some(v) => {
            builder = builder.header("content-type", "application/json");
            Body::from(v.to_string())
        }
        None => Body::empty(),
    };
    let response = ctx.app().clone().oneshot(builder.body(body).unwrap()).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap().to_vec();
    (status, headers, bytes)
}

fn json_body(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).unwrap_or(Value::Null)
}

async fn tokens(ctx: &TestContext) -> (String, String) {
    let helper = ctx.auth_helper();
    let mut user = helper.create_test_user().await;
    let mut admin = helper.create_admin_user().await;
    let user_token = user.login(&helper).await.unwrap().to_string();
    let admin_token = admin.login(&helper).await.unwrap().to_string();
    (user_token, admin_token)
}

fn local_folder_source(path: &str) -> Value {
    json!({
        "name": format!("local-{}", uuid::Uuid::new_v4()),
        "source_type": "local_folder",
        "enabled": false,
        "config": {
            "watch_folders": [path],
            "file_extensions": ["pdf"],
            "auto_sync": false,
            "sync_interval_minutes": 60,
            "recursive": false,
            "follow_symlinks": false
        }
    })
}

#[tokio::test]
async fn local_folder_sources_are_admin_only_without_allowlist() {
    let ctx = TestContext::new().await;
    let (user_token, admin_token) = tokens(&ctx).await;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_str().unwrap();

    // Existing and missing paths get the same response for a regular user.
    for candidate in [path, "/definitely/not/a/real/path"] {
        let (status, _, _) = send(&ctx, "POST", "/api/sources", &user_token, Some(local_folder_source(candidate))).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "create {}", candidate);

        let (status, _, _) = send(
            &ctx,
            "POST",
            "/api/sources/test/connection",
            &user_token,
            Some(json!({"source_type": "local_folder", "config": local_folder_source(candidate)["config"]})),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "test connection {}", candidate);
    }

    let (status, _, body) = send(&ctx, "POST", "/api/sources", &admin_token, Some(local_folder_source(path))).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));

    let _ = ctx.cleanup_and_close().await;
}

#[tokio::test]
async fn source_credentials_are_redacted_and_preserved() {
    let ctx = TestContext::new().await;
    let (user_token, _) = tokens(&ctx).await;

    let create = json!({
        "name": "nextcloud",
        "source_type": "webdav",
        "enabled": false,
        "config": {
            "server_url": "https://192.168.1.20/remote.php/dav",
            "username": "alice",
            "password": "stored-secret",
            "watch_folders": ["/Documents"],
            "file_extensions": ["pdf"],
            "auto_sync": false,
            "sync_interval_minutes": 60,
            "server_type": "nextcloud"
        }
    });
    let (status, _, body) = send(&ctx, "POST", "/api/sources", &user_token, Some(create)).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert!(!String::from_utf8_lossy(&body).contains("stored-secret"));
    let created = json_body(&body);
    assert_eq!(created["config"]["has_password"], json!(true));
    assert!(created["config"].get("password").is_none());
    let id = created["id"].as_str().unwrap().to_string();

    // Update without a password (as the UI sends it back) keeps the stored one.
    let mut config = created["config"].clone();
    config["watch_folders"] = json!(["/Documents", "/Scans"]);
    config["password"] = json!("");
    let (status, _, body) = send(&ctx, "PUT", &format!("/api/sources/{}", id), &user_token, Some(json!({"config": config}))).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert_eq!(json_body(&body)["config"]["has_password"], json!(true));

    let stored_config = || async {
        let row: (Value,) = sqlx::query_as("SELECT config FROM sources WHERE id = $1")
            .bind(uuid::Uuid::parse_str(&id).unwrap())
            .fetch_one(&ctx.state().db.pool)
            .await
            .unwrap();
        row.0
    };
    let row = stored_config().await;
    assert_eq!(row["password"], json!("stored-secret"));
    assert_eq!(row["watch_folders"], json!(["/Documents", "/Scans"]));
    assert!(row.get("has_password").is_none());

    // Changing the account or server without re-entering the password is refused,
    // both for updates and for connection tests against the stored source.
    for (field, value) in [("username", json!("alice2")), ("server_url", json!("https://192.168.1.21/remote.php/dav"))] {
        let mut changed = config.clone();
        changed[field] = value;
        let (status, _, body) =
            send(&ctx, "PUT", &format!("/api/sources/{}", id), &user_token, Some(json!({"config": changed.clone()}))).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "update with changed {}", field);
        assert_eq!(json_body(&body)["error"], json!("Re-enter the password when changing the server or account"));

        let (status, _, body) = send(
            &ctx,
            "POST",
            "/api/sources/test/connection",
            &user_token,
            Some(json!({"source_type": "webdav", "source_id": id, "config": changed})),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "test connection with changed {}", field);
        assert_eq!(json_body(&body)["error"], json!("Re-enter the password when changing the server or account"));
    }
    assert_eq!(stored_config().await["username"], json!("alice"));

    // Re-entering the password allows the change.
    let mut changed = config.clone();
    changed["username"] = json!("alice2");
    changed["password"] = json!("new-secret");
    let (status, _, body) = send(&ctx, "PUT", &format!("/api/sources/{}", id), &user_token, Some(json!({"config": changed}))).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let row = stored_config().await;
    assert_eq!(row["username"], json!("alice2"));
    assert_eq!(row["password"], json!("new-secret"));

    let (status, _, body) = send(&ctx, "GET", &format!("/api/sources/{}", id), &user_token, None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!String::from_utf8_lossy(&body).contains("stored-secret"));

    let (status, _, body) = send(&ctx, "GET", "/api/sources", &user_token, None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!String::from_utf8_lossy(&body).contains("stored-secret"));
    assert!(!String::from_utf8_lossy(&body).contains("new-secret"));

    let _ = ctx.cleanup_and_close().await;
}

#[tokio::test]
async fn metadata_service_addresses_are_refused() {
    let ctx = TestContext::new().await;
    let (user_token, _) = tokens(&ctx).await;

    let webdav = json!({
        "name": "metadata",
        "source_type": "webdav",
        "enabled": false,
        "config": {
            "server_url": "http://169.254.169.254/latest",
            "username": "u",
            "password": "p",
            "watch_folders": ["/"],
            "file_extensions": ["pdf"],
            "auto_sync": false,
            "sync_interval_minutes": 60,
            "server_type": "generic"
        }
    });
    let (status, _, _) = send(&ctx, "POST", "/api/sources", &user_token, Some(webdav.clone())).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, _, body) = send(
        &ctx,
        "POST",
        "/api/sources/test/connection",
        &user_token,
        Some(json!({"source_type": "webdav", "config": webdav["config"]})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json_body(&body)["success"], json!(false));

    let s3 = json!({
        "name": "s3-metadata",
        "source_type": "s3",
        "enabled": false,
        "config": {
            "bucket_name": "b",
            "region": "us-east-1",
            "access_key_id": "a",
            "secret_access_key": "s",
            "endpoint_url": "http://[fe80::1]:9000",
            "watch_folders": [],
            "file_extensions": ["pdf"],
            "auto_sync": false,
            "sync_interval_minutes": 60
        }
    });
    let (status, _, _) = send(&ctx, "POST", "/api/sources", &user_token, Some(s3)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, _, body) = send(
        &ctx,
        "POST",
        "/api/webdav/test/connection",
        &user_token,
        Some(json!({"server_url": "http://169.254.169.254", "username": "u", "password": "p", "server_type": "generic"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json_body(&body)["success"], json!(false));

    let _ = ctx.cleanup_and_close().await;
}

#[tokio::test]
async fn settings_never_return_the_webdav_password() {
    let ctx = TestContext::new().await;
    let (user_token, _) = tokens(&ctx).await;

    let (status, _, body) = send(&ctx, "PUT", "/api/settings", &user_token, Some(json!({"webdav_password": "dav-secret"}))).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let settings = json_body(&body);
    assert_eq!(settings["has_webdav_password"], json!(true));
    assert!(settings.get("webdav_password").is_none());

    // Empty keeps the stored value.
    let (status, _, body) = send(&ctx, "PUT", "/api/settings", &user_token, Some(json!({"webdav_password": ""}))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json_body(&body)["has_webdav_password"], json!(true));

    // ...but not when the server changes at the same time.
    let (status, _, body) = send(
        &ctx,
        "PUT",
        "/api/settings",
        &user_token,
        Some(json!({"webdav_password": "", "webdav_server_url": "https://192.168.1.30/remote.php/dav"})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(json_body(&body)["error"], json!("Re-enter the password when changing the server or account"));

    let (status, _, body) = send(&ctx, "GET", "/api/settings", &user_token, None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!String::from_utf8_lossy(&body).contains("dav-secret"));

    let _ = ctx.cleanup_and_close().await;
}

#[tokio::test]
async fn user_content_is_served_with_restrictive_headers_and_bounded_share_views() {
    let ctx = TestContext::new().await;
    let helper = ctx.auth_helper();
    let mut user = helper.create_test_user().await;
    let token = user.login(&helper).await.unwrap().to_string();
    let user_id = uuid::Uuid::parse_str(&user.id()).unwrap();

    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("page.html");
    std::fs::write(&file_path, "<script>alert(1)</script>").unwrap();

    let mut document = document_helpers::create_test_document(user_id);
    document.file_path = file_path.to_string_lossy().into_owned();
    document.mime_type = "text/html".to_string();
    document.original_filename = "page\".html".to_string();
    let document = ctx.state().db.create_document(document).await.unwrap();

    let (status, headers, _) = send(&ctx, "GET", &format!("/api/documents/{}/view", document.id), &token, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers["x-content-type-options"], "nosniff");
    assert!(headers["content-disposition"].to_str().unwrap().starts_with("attachment"));
    assert!(headers["content-security-policy"].to_str().unwrap().contains("sandbox"));

    let (status, headers, _) = send(&ctx, "GET", &format!("/api/documents/{}/download", document.id), &token, None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(headers["content-disposition"].to_str().unwrap().starts_with("attachment"));

    let (status, _, body) = send(
        &ctx,
        "POST",
        "/api/shared/links",
        &token,
        Some(json!({"document_id": document.id, "max_views": 2})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let share_token = json_body(&body)["token"].as_str().unwrap().to_string();

    let view = |ctx: &TestContext, share_token: String| {
        let app = ctx.app().clone();
        async move {
            let request = Request::builder()
                .method("POST")
                .uri(format!("/api/public/shared/{}/view", share_token))
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap();
            let response = app.oneshot(request).await.unwrap();
            (response.status(), response.headers().clone())
        }
    };

    let results = futures::future::join_all((0..6).map(|_| view(&ctx, share_token.clone()))).await;
    let served: Vec<_> = results.iter().filter(|(s, _)| *s == StatusCode::OK).collect();
    assert_eq!(served.len(), 2, "max_views must hold under concurrent requests: {:?}", results.iter().map(|r| r.0).collect::<Vec<_>>());
    for (_, headers) in served {
        assert_eq!(headers["x-content-type-options"], "nosniff");
        assert!(headers["content-disposition"].to_str().unwrap().starts_with("attachment"));
    }

    let _ = ctx.cleanup_and_close().await;
}
