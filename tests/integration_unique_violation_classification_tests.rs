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

fn ingest_request(
    user_id: Uuid,
    name: &str,
    data: &[u8],
    policy: DeduplicationPolicy,
) -> DocumentIngestionRequest {
    DocumentIngestionRequest {
        filename: name.to_string(),
        original_filename: name.to_string(),
        file_data: data.to_vec(),
        mime_type: "text/plain".to_string(),
        user_id,
        deduplication_policy: policy,
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

/// A policy-independent view of an ingestion outcome, so the race and non-race paths can be
/// compared for equality.
fn outcome_summary(result: &IngestionResult) -> (&'static str, Uuid, Option<String>) {
    match result {
        IngestionResult::Created(doc) => ("created", doc.id, None),
        IngestionResult::ExistingDocument(doc) => ("existing", doc.id, None),
        IngestionResult::Skipped {
            existing_document_id,
            reason,
        } => ("skipped", *existing_document_id, Some(reason.clone())),
        IngestionResult::TrackedAsDuplicate {
            existing_document_id,
        } => ("tracked", *existing_document_id, None),
    }
}

/// Insert a document with `data`'s hash for `user_id` in a transaction that is left open: the
/// row is invisible to other transactions, but its unique-index entry blocks a competing insert.
async fn uncommitted_winner(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    data: &[u8],
) -> Result<(Uuid, sqlx::Transaction<'static, sqlx::Postgres>)> {
    let hash = readur::utils::hash::to_hex(&<sha2::Sha256 as sha2::Digest>::digest(data));
    let winner_id = Uuid::new_v4();
    let mut winner = pool.begin().await?;
    sqlx::query(
        "INSERT INTO documents (id, filename, original_filename, file_path, file_size, mime_type, user_id, file_hash)
         VALUES ($1, 'winner.txt', 'winner.txt', '/elsewhere/winner.txt', $2, 'text/plain', $3, $4)",
    )
    .bind(winner_id)
    .bind(data.len() as i64)
    .bind(user_id)
    .bind(&hash)
    .execute(&mut *winner)
    .await?;
    Ok((winner_id, winner))
}

/// Commit `winner` as soon as an `INSERT INTO documents` is parked on its index entry; returns
/// whether that wait was observed (if not, the race was not exercised).
async fn commit_once_blocked(
    pool: sqlx::PgPool,
    winner: sqlx::Transaction<'static, sqlx::Postgres>,
) -> Result<bool> {
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
            return Ok(true);
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    winner.commit().await?;
    Ok(false)
}

fn stored_files(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    std::fs::read_dir(dir.join("documents"))
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.is_file())
                .collect()
        })
        .unwrap_or_default()
}

/// The concurrent-upload race, forced deterministically for every deduplication policy: another
/// transaction holds an uncommitted row with the same user and hash, so ingestion's duplicate
/// lookup misses it and its insert blocks on the unique index. Once the other transaction
/// commits, the insert fails with a unique violation on `idx_documents_user_file_hash`. Ingestion
/// must report the committed document exactly as the up-front duplicate check would have for the
/// requested policy (and remove the file it had already stored), not an error and not a fixed
/// `ExistingDocument`.
#[tokio::test]
async fn concurrent_duplicate_upload_honours_the_policy_like_the_non_race_path() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let ctx = TestContext::with_config(
        TestConfigBuilder::default().with_upload_path(dir.path().to_str().unwrap()),
    )
    .await;
    let state = ctx.state();
    let service = DocumentIngestionService::new(state.db.clone(), (*state.file_service).clone());
    let data = b"content uploaded twice at the same moment";

    for (policy, expected_kind) in [
        (DeduplicationPolicy::Skip, "skipped"),
        (DeduplicationPolicy::ReturnExisting, "existing"),
        (DeduplicationPolicy::TrackAsDuplicate, "tracked"),
    ] {
        let user = state.db.create_user(new_user("uv_race")).await?;
        let (winner_id, winner) = uncommitted_winner(&state.db.pool, user.id, data).await?;

        let (outcome, blocked) = tokio::join!(
            service.ingest_document(ingest_request(user.id, "loser.txt", data, policy.clone())),
            commit_once_blocked(state.db.pool.clone(), winner)
        );
        assert!(
            blocked?,
            "{policy:?}: ingestion's insert never waited on the uncommitted duplicate"
        );
        let raced = outcome.map_err(|e| anyhow::anyhow!("{policy:?}: {e}"))?;
        let raced = outcome_summary(&raced);
        assert_eq!(raced.0, expected_kind, "{policy:?}: {raced:?}");
        assert_eq!(
            raced.1, winner_id,
            "{policy:?}: reports the committed document"
        );

        // The same request without a race goes through the up-front check: identical outcome.
        let direct = service
            .ingest_document(ingest_request(user.id, "loser.txt", data, policy.clone()))
            .await
            .map_err(|e| anyhow::anyhow!("{policy:?}: {e}"))?;
        assert_eq!(
            outcome_summary(&direct),
            raced,
            "{policy:?}: race and non-race paths report the same outcome"
        );

        let (rows,): (i64,) = sqlx::query_as("SELECT count(*) FROM documents WHERE user_id = $1")
            .bind(user.id)
            .fetch_one(&state.db.pool)
            .await?;
        assert_eq!(rows, 1, "{policy:?}: only the winner's row exists");
        let (failed,): (i64,) =
            sqlx::query_as("SELECT count(*) FROM failed_documents WHERE user_id = $1")
                .bind(user.id)
                .fetch_one(&state.db.pool)
                .await?;
        assert_eq!(
            failed, 0,
            "{policy:?}: a lost race is not recorded as a failed ingestion"
        );
        let stored = stored_files(dir.path());
        assert!(
            stored.is_empty(),
            "{policy:?}: the loser's stored file is removed: {stored:?}"
        );
    }
    Ok(())
}

/// The REST upload pipeline (`services::documents::upload_document`, `Skip`) answers a duplicate
/// with the same 409 whether the duplicate was found up front or by losing the insert race.
#[tokio::test]
async fn rest_upload_conflict_is_identical_with_and_without_the_race() -> Result<()> {
    use axum::response::IntoResponse;
    use readur::errors::document::DocumentError;
    use readur::services::documents::{upload_document, UploadRequest};

    let dir = tempfile::tempdir()?;
    let ctx = TestContext::with_config(
        TestConfigBuilder::default().with_upload_path(dir.path().to_str().unwrap()),
    )
    .await;
    let state = ctx.state();
    let user = state.db.create_user(new_user("uv_race_rest")).await?;
    let data = b"uploaded through REST while another upload commits";
    let upload = || UploadRequest {
        filename: "loser.txt".to_string(),
        mime_type: "text/plain".to_string(),
        data: data.to_vec(),
        ocr_languages: Vec::new(),
        ocr_language: None,
        label_ids: Vec::new(),
    };

    let (winner_id, winner) = uncommitted_winner(&state.db.pool, user.id, data).await?;
    let (raced, blocked) = tokio::join!(
        upload_document(state, &user, upload()),
        commit_once_blocked(state.db.pool.clone(), winner)
    );
    assert!(
        blocked?,
        "the upload's insert never waited on the uncommitted duplicate"
    );
    let raced = raced.err().expect("a raced duplicate upload is rejected");
    let direct = upload_document(state, &user, upload())
        .await
        .err()
        .expect("a duplicate upload is rejected");

    let (DocumentError::Conflict(raced_msg), DocumentError::Conflict(direct_msg)) =
        (&raced, &direct)
    else {
        panic!("expected two conflicts, got {raced:?} and {direct:?}");
    };
    assert_eq!(
        raced_msg, direct_msg,
        "same message whether or not the race happened"
    );
    assert!(raced_msg.contains(&winner_id.to_string()), "{raced_msg}");
    assert_eq!(raced.into_response().status(), StatusCode::CONFLICT);
    assert_eq!(direct.into_response().status(), StatusCode::CONFLICT);
    assert!(
        stored_files(dir.path()).is_empty(),
        "no file outlives the rejected uploads"
    );
    Ok(())
}
