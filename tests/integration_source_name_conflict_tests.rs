//! A duplicate source name is reported as `409 SOURCE_DUPLICATE_NAME`, recognised from the
//! unique-constraint violation Postgres reports (SQLSTATE 23505 on `sources_user_id_name_key`),
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
    let request = Request::builder()
        .method("POST")
        .uri("/api/sources")
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
