//! Unique-constraint clashes are classified by SQLSTATE 23505 plus the constraint name, not by
//! error text. These tests pin the constraint names against the migrated schema, check real
//! Postgres errors carry them, and check the REST and ingestion outcomes that depend on them.

use anyhow::Result;
use axum::{body::Body, http::Request, http::StatusCode};
use serde_json::{json, Value};
use std::time::Duration;
use tower::util::ServiceExt;
use uuid::Uuid;

use readur::{
    db::unique_violation::{
        violated_unique_constraint_in, DOCUMENTS_USER_FILE_HASH_UNIQUE, USERS_EMAIL_UNIQUE,
        USERS_USERNAME_UNIQUE,
    },
    ingestion::document_ingestion::{
        DeduplicationPolicy, DocumentIngestionRequest, DocumentIngestionService, IngestionResult,
    },
    models::{CreateUser, UserRole},
    test_utils::{TestConfigBuilder, TestContext},
};

fn new_user(prefix: &str) -> CreateUser {
    let suffix = Uuid::new_v4().simple();
    CreateUser {
        username: format!("{prefix}_{suffix}"),
        email: format!("{prefix}_{suffix}@example.com"),
        password: "test_password_123".to_string(),
        role: Some(UserRole::User),
    }
}

async fn send_json(
    ctx: &TestContext,
    method: &str,
    uri: &str,
    token: &str,
    body: Value,
) -> (StatusCode, Value) {
    let response = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn admin_token(ctx: &TestContext) -> String {
    let helper = ctx.auth_helper();
    let admin = helper.create_admin_user().await;
    helper.login_user(&admin.username, &admin.password).await
}

/// The names the classifiers look for are the ones the migrations actually create: a renamed
/// constraint fails here instead of silently turning clashes into 500s.
#[tokio::test]
async fn classified_constraint_names_exist_in_the_schema() -> Result<()> {
    let ctx = TestContext::new().await;
    let pool = &ctx.state().db.pool;

    for (name, column) in [
        (USERS_USERNAME_UNIQUE, "username"),
        (USERS_EMAIL_UNIQUE, "email"),
    ] {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT pg_get_constraintdef(c.oid) FROM pg_constraint c
             WHERE c.conname = $1 AND c.conrelid = 'users'::regclass AND c.contype = 'u'",
        )
        .bind(name)
        .fetch_optional(pool)
        .await?;
        let def = row
            .unwrap_or_else(|| panic!("unique constraint {name} missing on users"))
            .0;
        assert_eq!(def, format!("UNIQUE ({column})"));
    }

    let (is_unique, def): (bool, String) = sqlx::query_as(
        "SELECT i.indisunique, pg_get_indexdef(i.indexrelid) FROM pg_index i
         WHERE i.indexrelid = $1::regclass",
    )
    .bind(DOCUMENTS_USER_FILE_HASH_UNIQUE)
    .fetch_one(pool)
    .await?;
    assert!(
        is_unique,
        "{DOCUMENTS_USER_FILE_HASH_UNIQUE} must be a unique index"
    );
    assert!(
        def.contains("(user_id, file_hash)"),
        "unexpected index definition: {def}"
    );
    Ok(())
}

/// Real Postgres errors from `Database::create_user` carry the constraint the classifier keys on.
#[tokio::test]
async fn duplicate_user_inserts_report_the_violated_constraint() -> Result<()> {
    let ctx = TestContext::new().await;
    let db = &ctx.state().db;
    let first = db.create_user(new_user("uv_db")).await?;

    let mut same_name = new_user("uv_db_other");
    same_name.username = first.username.clone();
    let err = db
        .create_user(same_name)
        .await
        .expect_err("duplicate username must fail");
    assert_eq!(
        violated_unique_constraint_in(&err),
        Some(USERS_USERNAME_UNIQUE)
    );

    let mut same_email = new_user("uv_db_other");
    same_email.email = first.email.clone();
    let err = db
        .create_user(same_email)
        .await
        .expect_err("duplicate email must fail");
    assert_eq!(
        violated_unique_constraint_in(&err),
        Some(USERS_EMAIL_UNIQUE)
    );
    Ok(())
}

#[tokio::test]
async fn rest_create_with_taken_username_or_email_is_409_with_the_same_body() -> Result<()> {
    let ctx = TestContext::new().await;
    let token = admin_token(&ctx).await;
    let existing = ctx.state().db.create_user(new_user("uv_rest")).await?;
    let fresh = new_user("uv_rest_new");

    let (status, body) = send_json(
        &ctx,
        "POST",
        "/api/users",
        &token,
        json!({
            "username": existing.username, "email": fresh.email, "password": "Password123!"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(
        body,
        json!({"error": "Username already exists", "code": "USER_DUPLICATE_USERNAME", "status": 409})
    );

    let (status, body) = send_json(
        &ctx,
        "POST",
        "/api/users",
        &token,
        json!({
            "username": fresh.username, "email": existing.email, "password": "Password123!"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(
        body,
        json!({"error": "Email already exists", "code": "USER_DUPLICATE_EMAIL", "status": 409})
    );
    Ok(())
}

#[tokio::test]
async fn rest_update_to_taken_username_or_email_is_409_with_the_same_body() -> Result<()> {
    let ctx = TestContext::new().await;
    let token = admin_token(&ctx).await;
    let taken = ctx.state().db.create_user(new_user("uv_upd_taken")).await?;
    let target = ctx
        .state()
        .db
        .create_user(new_user("uv_upd_target"))
        .await?;
    let uri = format!("/api/users/{}", target.id);

    let (status, body) = send_json(
        &ctx,
        "PUT",
        &uri,
        &token,
        json!({ "username": taken.username }),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(
        body,
        json!({"error": "Username already exists", "code": "USER_DUPLICATE_USERNAME", "status": 409})
    );

    let (status, body) =
        send_json(&ctx, "PUT", &uri, &token, json!({ "email": taken.email })).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(
        body,
        json!({"error": "Email already exists", "code": "USER_DUPLICATE_EMAIL", "status": 409})
    );

    // Neither failed update changed the account.
    let unchanged = ctx
        .state()
        .db
        .get_user_by_id(target.id)
        .await?
        .expect("target still exists");
    assert_eq!(
        (unchanged.username, unchanged.email),
        (target.username, target.email)
    );
    Ok(())
}

fn ingest_request(user_id: Uuid, name: &str, data: &[u8]) -> DocumentIngestionRequest {
    DocumentIngestionRequest {
        filename: name.to_string(),
        original_filename: name.to_string(),
        file_data: data.to_vec(),
        mime_type: "text/plain".to_string(),
        user_id,
        deduplication_policy: DeduplicationPolicy::ReturnExisting,
        source_type: Some("test".to_string()),
        source_id: None,
        original_created_at: None,
        original_modified_at: None,
        source_path: None,
        file_permissions: None,
        file_owner: None,
        file_group: None,
        source_metadata: None,
    }
}

/// The concurrent-upload race, forced deterministically: another transaction holds an
/// uncommitted row with the same user and hash, so ingestion's duplicate lookup misses it and its
/// insert blocks on the unique index. Once the other transaction commits, the insert fails with a
/// unique violation on `idx_documents_user_file_hash`, which ingestion must turn into the
/// committed document (and remove the file it had already stored), not an error.
#[tokio::test]
async fn concurrent_duplicate_upload_returns_the_winning_document() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let ctx = TestContext::with_config(
        TestConfigBuilder::default().with_upload_path(dir.path().to_str().unwrap()),
    )
    .await;
    let state = ctx.state();
    let user = state.db.create_user(new_user("uv_race")).await?;
    let data = b"content uploaded twice at the same moment";
    let hash = readur::utils::hash::to_hex(&<sha2::Sha256 as sha2::Digest>::digest(data));

    let winner_id = Uuid::new_v4();
    let mut winner = state.db.pool.begin().await?;
    sqlx::query(
        "INSERT INTO documents (id, filename, original_filename, file_path, file_size, mime_type, user_id, file_hash)
         VALUES ($1, 'winner.txt', 'winner.txt', '/elsewhere/winner.txt', $2, 'text/plain', $3, $4)",
    )
    .bind(winner_id)
    .bind(data.len() as i64)
    .bind(user.id)
    .bind(&hash)
    .execute(&mut *winner)
    .await?;

    let service = DocumentIngestionService::new(state.db.clone(), (*state.file_service).clone());
    let pool = state.db.pool.clone();
    let commit_once_blocked = async move {
        // Wait until ingestion's INSERT is parked on the winner's index entry, then commit.
        for _ in 0..200 {
            let (waiting,): (i64,) = sqlx::query_as(
                "SELECT count(*) FROM pg_stat_activity
                 WHERE datname = current_database() AND wait_event_type = 'Lock'
                   AND query LIKE '%INSERT INTO documents%'",
            )
            .fetch_one(&pool)
            .await?;
            if waiting > 0 {
                winner.commit().await?;
                return Ok::<bool, anyhow::Error>(true);
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        winner.commit().await?;
        Ok(false)
    };

    let (outcome, blocked) = tokio::join!(
        service.ingest_document(ingest_request(user.id, "loser.txt", data)),
        commit_once_blocked
    );
    assert!(
        blocked?,
        "ingestion's insert never waited on the uncommitted duplicate"
    );

    match outcome.map_err(|e| anyhow::anyhow!("{e}"))? {
        IngestionResult::ExistingDocument(doc) => assert_eq!(doc.id, winner_id),
        other => panic!("expected ExistingDocument(winner), got {other:?}"),
    }

    let (rows,): (i64,) = sqlx::query_as("SELECT count(*) FROM documents WHERE user_id = $1")
        .bind(user.id)
        .fetch_one(&state.db.pool)
        .await?;
    assert_eq!(rows, 1, "only the winner's row exists");
    let (failed,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM failed_documents WHERE user_id = $1")
            .bind(user.id)
            .fetch_one(&state.db.pool)
            .await?;
    assert_eq!(
        failed, 0,
        "a lost race is not recorded as a failed ingestion"
    );
    let stored: Vec<_> = std::fs::read_dir(dir.path().join("documents"))
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_file())
                .collect()
        })
        .unwrap_or_default();
    assert!(
        stored.is_empty(),
        "the loser's stored file is removed: {stored:?}"
    );
    Ok(())
}
