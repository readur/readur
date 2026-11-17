//! A user that disappears while it is being updated is reported through a typed
//! signal (`Database::update_user` returns `Ok(None)`), not by matching error
//! text, and REST turns that into the same 404 as updating an id that never
//! existed.
//!
//! The "deleted mid-update" case is forced deterministically: another
//! transaction deletes the row but does not commit, so `update_user`'s lookup
//! still sees the user while its `UPDATE` blocks on the row lock. Once the
//! delete commits, the `UPDATE` re-checks the row, finds it gone and matches
//! nothing.

use anyhow::Result;
use axum::{body::Body, http::Request, http::StatusCode};
use serde_json::{json, Value};
use sqlx::{PgPool, Postgres, Transaction};
use std::time::Duration;
use tower::util::ServiceExt;
use uuid::Uuid;

use readur::{
    models::{CreateUser, UserRole},
    test_utils::TestContext,
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

/// Delete `id` in a transaction that is left open, so the row is locked but
/// still visible to other transactions.
async fn uncommitted_delete(pool: &PgPool, id: Uuid) -> Result<Transaction<'static, Postgres>> {
    let mut tx = pool.begin().await?;
    let deleted = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    assert_eq!(deleted.rows_affected(), 1);
    Ok(tx)
}

/// Commit `tx` as soon as a statement is waiting on a lock while running an
/// `UPDATE users`; returns whether that wait was observed.
async fn commit_once_update_blocks(
    pool: PgPool,
    tx: Transaction<'static, Postgres>,
) -> Result<bool> {
    for _ in 0..200 {
        let (waiting,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM pg_stat_activity
             WHERE datname = current_database() AND wait_event_type = 'Lock'
               AND query LIKE '%UPDATE users SET username%'",
        )
        .fetch_one(&pool)
        .await?;
        if waiting > 0 {
            tx.commit().await?;
            return Ok(true);
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    tx.commit().await?;
    Ok(false)
}

#[tokio::test]
async fn updating_a_missing_user_returns_none() -> Result<()> {
    let ctx = TestContext::new().await;
    let db = &ctx.state().db;

    let updated = db
        .update_user(Uuid::new_v4(), Some("ghost".into()), None, None, None)
        .await?;
    assert!(updated.is_none());

    // And a present user still comes back updated.
    let user = db.create_user(new_user("upd_present")).await?;
    let updated = db
        .update_user(user.id, None, None, None, Some(false))
        .await?
        .expect("user exists");
    assert_eq!(updated.id, user.id);
    assert!(!updated.is_active);
    Ok(())
}

#[tokio::test]
async fn user_deleted_mid_update_returns_none() -> Result<()> {
    let ctx = TestContext::new().await;
    let db = ctx.state().db.clone();
    let user = db.create_user(new_user("upd_race_db")).await?;

    let tx = uncommitted_delete(&db.pool, user.id).await?;
    let (updated, blocked) = tokio::join!(
        db.update_user(
            user.id,
            Some(format!("{}_renamed", user.username)),
            None,
            None,
            None
        ),
        commit_once_update_blocks(db.pool.clone(), tx)
    );
    assert!(
        blocked?,
        "update_user's UPDATE never waited on the uncommitted delete"
    );
    assert!(
        updated?.is_none(),
        "an UPDATE that matched no row is reported as Ok(None)"
    );
    assert!(db.get_user_by_id(user.id).await?.is_none());
    Ok(())
}

#[tokio::test]
async fn rest_update_of_user_deleted_mid_update_is_the_same_404_as_a_missing_user() -> Result<()> {
    let ctx = TestContext::new().await;
    let token = admin_token(&ctx).await;
    let db = ctx.state().db.clone();

    let (missing_status, missing_body) = send_json(
        &ctx,
        "PUT",
        &format!("/api/users/{}", Uuid::new_v4()),
        &token,
        json!({ "is_active": false }),
    )
    .await;
    assert_eq!(missing_status, StatusCode::NOT_FOUND);
    assert_eq!(missing_body["error"], "User not found");
    assert_eq!(missing_body["status"], 404);

    let user = db.create_user(new_user("upd_race_rest")).await?;
    let uri = format!("/api/users/{}", user.id);
    let tx = uncommitted_delete(&db.pool, user.id).await?;
    let ((status, body), blocked) = tokio::join!(
        send_json(&ctx, "PUT", &uri, &token, json!({ "is_active": false })),
        commit_once_update_blocks(db.pool.clone(), tx)
    );
    assert!(
        blocked?,
        "the handler's UPDATE never waited on the uncommitted delete"
    );
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    assert_eq!(
        body, missing_body,
        "same body whether the user vanished before or during the update"
    );
    Ok(())
}
