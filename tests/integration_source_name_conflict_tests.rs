//! A duplicate source name, on create or on rename, is reported as `409 SOURCE_DUPLICATE_NAME`,
//! recognised from the unique-constraint violation Postgres reports (SQLSTATE 23505 on `sources_user_id_name_key`),
//! not from the wording of its error message.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use readur::db::unique_violation::{is_source_name_taken, SOURCE_NAME_UNIQUE_CONSTRAINT};
use readur::models::{CreateSource, SourceType};
use readur::test_utils::TestContext;
use serde_json::{json, Value};
use tower::ServiceExt;

async fn create_source(ctx: &TestContext, token: &str, body: &Value) -> (StatusCode, Value) {
    send(ctx, token, "POST", "/api/sources", body).await
}

async fn update_source(
    ctx: &TestContext,
    token: &str,
    id: &str,
    body: &Value,
) -> (StatusCode, Value) {
    send(ctx, token, "PUT", &format!("/api/sources/{id}"), body).await
}

async fn source_name(ctx: &TestContext, id: &str) -> String {
    sqlx::query_scalar("SELECT name FROM sources WHERE id = $1")
        .bind(id.parse::<uuid::Uuid>().unwrap())
        .fetch_one(ctx.state.db.get_pool())
        .await
        .unwrap()
}

async fn send(
    ctx: &TestContext,
    token: &str,
    method: &str,
    uri: &str,
    body: &Value,
) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let response = ctx.app().clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

fn webdav_source(name: &str) -> Value {
    json!({
        "name": name,
        "source_type": "webdav",
        "enabled": false,
        "config": {
            "server_url": "https://192.168.1.20/remote.php/dav",
            "username": "alice",
            "password": "secret",
            "watch_folders": ["/Documents"],
            "file_extensions": ["pdf"],
            "auto_sync": false,
            "sync_interval_minutes": 60,
            "server_type": "nextcloud"
        }
    })
}

#[tokio::test]
async fn a_duplicate_name_is_a_409_with_the_duplicate_name_code() {
    let ctx = TestContext::new().await;
    let helper = ctx.auth_helper();
    let mut alice = helper.create_test_user().await;
    let mut bob = helper.create_test_user().await;
    let alice_token = alice.login(&helper).await.unwrap().to_string();
    let bob_token = bob.login(&helper).await.unwrap().to_string();

    let (status, body) = create_source(&ctx, &alice_token, &webdav_source("Nextcloud")).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, body) = create_source(&ctx, &alice_token, &webdav_source("Nextcloud")).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["code"], "SOURCE_DUPLICATE_NAME");
    assert_eq!(body["error"], "A source with this name already exists");

    // Names are unique per user only.
    let (status, body) = create_source(&ctx, &bob_token, &webdav_source("Nextcloud")).await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

#[tokio::test]
async fn renaming_to_a_name_the_user_already_has_is_a_409_and_changes_nothing() {
    let ctx = TestContext::new().await;
    let helper = ctx.auth_helper();
    let mut alice = helper.create_test_user().await;
    let token = alice.login(&helper).await.unwrap().to_string();

    let (status, body) = create_source(&ctx, &token, &webdav_source("Nextcloud")).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) = create_source(&ctx, &token, &webdav_source("Backup")).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let backup = body["id"].as_str().unwrap().to_string();

    let rename = json!({ "name": "Nextcloud", "enabled": true });
    let (status, body) = update_source(&ctx, &token, &backup, &rename).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["code"], "SOURCE_DUPLICATE_NAME");
    assert_eq!(body["error"], "A source with this name already exists");

    // The whole update was refused, not just the name.
    assert_eq!(source_name(&ctx, &backup).await, "Backup");
    let enabled: bool = sqlx::query_scalar("SELECT enabled FROM sources WHERE id = $1")
        .bind(backup.parse::<uuid::Uuid>().unwrap())
        .fetch_one(ctx.state.db.get_pool())
        .await
        .unwrap();
    assert!(!enabled);

    // Keeping its own name is not a clash with itself.
    let (status, body) = update_source(&ctx, &token, &backup, &json!({ "name": "Backup" })).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["name"], "Backup");
}

#[tokio::test]
async fn renaming_to_another_users_source_name_is_allowed() {
    let ctx = TestContext::new().await;
    let helper = ctx.auth_helper();
    let mut alice = helper.create_test_user().await;
    let mut bob = helper.create_test_user().await;
    let alice_token = alice.login(&helper).await.unwrap().to_string();
    let bob_token = bob.login(&helper).await.unwrap().to_string();

    let (status, body) = create_source(&ctx, &alice_token, &webdav_source("Shared name")).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let alices = body["id"].as_str().unwrap().to_string();
    let (status, body) = create_source(&ctx, &bob_token, &webdav_source("Mine")).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let bobs = body["id"].as_str().unwrap().to_string();

    let (status, body) =
        update_source(&ctx, &bob_token, &bobs, &json!({ "name": "Shared name" })).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["name"], "Shared name");
    assert_eq!(source_name(&ctx, &bobs).await, "Shared name");
    assert_eq!(source_name(&ctx, &alices).await, "Shared name");

    // Bob still cannot rename Alice's source: it is not his to find.
    let (status, _) = update_source(&ctx, &bob_token, &alices, &json!({ "name": "Taken" })).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(source_name(&ctx, &alices).await, "Shared name");
}

#[tokio::test]
async fn the_database_reports_the_constraint_the_classifier_expects() {
    // Pins the constraint name against the real schema: if a migration renames or replaces the
    // unique key, this fails instead of duplicate names silently turning into 500s.
    let ctx = TestContext::new().await;
    let helper = ctx.auth_helper();
    let user = helper.create_test_user().await;
    let source = CreateSource {
        name: "Scans".to_string(),
        source_type: SourceType::WebDAV,
        enabled: Some(false),
        config: webdav_source("Scans")["config"].clone(),
    };
    ctx.state
        .db
        .create_source(user.user_response.id, &source)
        .await
        .unwrap();

    let error = ctx
        .state
        .db
        .create_source(user.user_response.id, &source)
        .await
        .expect_err("a second source with the same name must be refused");
    let db_error = error
        .chain()
        .find_map(|cause| cause.downcast_ref::<sqlx::Error>())
        .and_then(|e| e.as_database_error())
        .unwrap_or_else(|| panic!("expected a database error, got {error:#}"));
    assert_eq!(db_error.code().as_deref(), Some("23505"));
    assert_eq!(db_error.constraint(), Some(SOURCE_NAME_UNIQUE_CONSTRAINT));
    assert!(is_source_name_taken(&error));
}
