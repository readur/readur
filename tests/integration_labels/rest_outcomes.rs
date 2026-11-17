//! REST label endpoints end to end: duplicate names map to 409 through the structured
//! unique-violation check, and each add/remove outcome keeps its status code.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use readur::db::labels::{is_label_name_taken, LabelAssignOutcome};
use readur::test_utils::{document_helpers, TestContext, TestUser};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::util::ServiceExt;
use uuid::Uuid;

async fn user_with_token(ctx: &TestContext) -> (TestUser, String) {
    let auth = ctx.auth_helper();
    let user = auth.create_test_user().await;
    let token = auth.login_user(&user.username, &user.password).await;
    (user, token)
}

async fn request(
    ctx: &TestContext,
    method: &str,
    uri: &str,
    token: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {token}"));
    let body = match body {
        Some(body) => {
            builder = builder.header("content-type", "application/json");
            Body::from(body.to_string())
        }
        None => Body::empty(),
    };
    let response = ctx
        .app
        .clone()
        .oneshot(builder.body(body).unwrap())
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

async fn create_label(ctx: &TestContext, token: &str, name: &str) -> Uuid {
    let (status, body) = request(
        ctx,
        "POST",
        "/api/labels",
        token,
        Some(json!({ "name": name, "color": "#0969da" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["id"].as_str().unwrap().parse().unwrap()
}

async fn create_document(ctx: &TestContext, user: &TestUser) -> Uuid {
    let document = document_helpers::create_test_document(user.user_response.id);
    ctx.state.db.create_document(document).await.unwrap().id
}

fn assignment_uri(document_id: Uuid, label_id: Uuid) -> String {
    format!("/api/labels/documents/{document_id}/labels/{label_id}")
}

#[tokio::test]
async fn duplicate_label_name_is_a_409_on_create_and_rename() {
    let ctx = TestContext::new().await;
    let (_, token) = user_with_token(&ctx).await;
    create_label(&ctx, &token, "Taken").await;

    let (status, body) = request(
        &ctx,
        "POST",
        "/api/labels",
        &token,
        Some(json!({ "name": "Taken", "color": "#ff0000" })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert!(body.to_string().contains("already exists"), "{body}");

    let other = create_label(&ctx, &token, "Free").await;
    let (status, body) = request(
        &ctx,
        "PUT",
        &format!("/api/labels/{other}"),
        &token,
        Some(json!({ "name": "Taken" })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert!(body.to_string().contains("already exists"), "{body}");

    // Another user may use the same name: uniqueness is per user.
    let (_, other_token) = user_with_token(&ctx).await;
    create_label(&ctx, &other_token, "Taken").await;
}

/// Against real Postgres errors: the label-name constraint is a name clash, every other
/// unique violation (labels' primary key, a repeated document assignment) is not.
#[tokio::test]
async fn only_the_label_name_constraint_counts_as_a_name_clash() {
    let ctx = TestContext::new().await;
    let (user, _) = user_with_token(&ctx).await;
    let user_id = user.user_response.id;
    let pool = &ctx.state.db.pool;

    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO labels (user_id, name, color) VALUES ($1, 'Orig', '#000000') RETURNING id",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap();

    let same_name =
        sqlx::query("INSERT INTO labels (user_id, name, color) VALUES ($1, 'Orig', '#000000')")
            .bind(user_id)
            .execute(pool)
            .await
            .unwrap_err();
    assert!(is_label_name_taken(&same_name), "{same_name}");

    let same_id = sqlx::query(
        "INSERT INTO labels (id, user_id, name, color) VALUES ($1, $2, 'Other', '#000000')",
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await
    .unwrap_err();
    assert!(
        same_id.as_database_error().unwrap().is_unique_violation(),
        "expected a unique violation: {same_id}"
    );
    assert!(!is_label_name_taken(&same_id), "{same_id}");

    let document_id = create_document(&ctx, &user).await;
    let assign = "INSERT INTO document_labels (document_id, label_id) VALUES ($1, $2)";
    sqlx::query(assign)
        .bind(document_id)
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
    let repeated = sqlx::query(assign)
        .bind(document_id)
        .bind(id)
        .execute(pool)
        .await
        .unwrap_err();
    assert!(repeated.as_database_error().unwrap().is_unique_violation());
    assert!(!is_label_name_taken(&repeated), "{repeated}");
}

#[tokio::test]
async fn add_document_label_statuses() {
    let ctx = TestContext::new().await;
    let (user, token) = user_with_token(&ctx).await;
    let label_id = create_label(&ctx, &token, "Mine").await;
    let document_id = create_document(&ctx, &user).await;

    let (owner, owner_token) = user_with_token(&ctx).await;
    let foreign_label = create_label(&ctx, &owner_token, "Theirs").await;
    let foreign_document = create_document(&ctx, &owner).await;

    let uri = assignment_uri(document_id, label_id);
    assert_eq!(
        request(&ctx, "POST", &uri, &token, None).await.0,
        StatusCode::CREATED
    );
    assert_eq!(
        request(&ctx, "POST", &uri, &token, None).await.0,
        StatusCode::OK
    );

    for (doc, label) in [
        (Uuid::new_v4(), label_id),
        (foreign_document, label_id),
        (document_id, Uuid::new_v4()),
        (document_id, foreign_label),
    ] {
        let (status, _) = request(&ctx, "POST", &assignment_uri(doc, label), &token, None).await;
        assert_eq!(
            status,
            StatusCode::NOT_FOUND,
            "document {doc}, label {label}"
        );
    }

    // None of the refused calls wrote anything.
    let labels = ctx.state.db.get_document_labels(document_id).await.unwrap();
    assert_eq!(
        labels.iter().map(|l| l.id).collect::<Vec<_>>(),
        vec![label_id]
    );
    assert!(ctx
        .state
        .db
        .get_document_labels(foreign_document)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn remove_document_label_statuses() {
    let ctx = TestContext::new().await;
    let (user, token) = user_with_token(&ctx).await;
    let label_id = create_label(&ctx, &token, "Mine").await;
    let document_id = create_document(&ctx, &user).await;
    let uri = assignment_uri(document_id, label_id);
    assert_eq!(
        request(&ctx, "POST", &uri, &token, None).await.0,
        StatusCode::CREATED
    );

    // Someone else cannot take it off.
    let (_, intruder_token) = user_with_token(&ctx).await;
    assert_eq!(
        request(&ctx, "DELETE", &uri, &intruder_token, None).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        ctx.state
            .db
            .get_document_labels(document_id)
            .await
            .unwrap()
            .len(),
        1
    );

    let missing = assignment_uri(Uuid::new_v4(), label_id);
    assert_eq!(
        request(&ctx, "DELETE", &missing, &token, None).await.0,
        StatusCode::NOT_FOUND
    );

    assert_eq!(
        request(&ctx, "DELETE", &uri, &token, None).await.0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        request(&ctx, "DELETE", &uri, &token, None).await.0,
        StatusCode::NOT_FOUND
    );
    assert!(ctx
        .state
        .db
        .get_document_labels(document_id)
        .await
        .unwrap()
        .is_empty());
}

/// Many simultaneous assignments of the same pair: the unique constraint decides, exactly
/// one call adds the label and every other one sees it already there. None errors.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_assignments_add_exactly_once() {
    const CALLS: usize = 32;
    let ctx = TestContext::new().await;
    let (user, token) = user_with_token(&ctx).await;
    let user_id = user.user_response.id;
    let label_id = create_label(&ctx, &token, "Raced").await;
    let document_id = create_document(&ctx, &user).await;

    let barrier = Arc::new(tokio::sync::Barrier::new(CALLS));
    let tasks: Vec<_> = (0..CALLS)
        .map(|_| {
            let db = ctx.state.db.clone();
            let barrier = barrier.clone();
            tokio::spawn(async move {
                barrier.wait().await;
                db.add_document_label_for_user(document_id, label_id, user_id)
                    .await
            })
        })
        .collect();
    let outcomes: Vec<LabelAssignOutcome> = futures::future::join_all(tasks)
        .await
        .into_iter()
        .map(|joined| joined.unwrap().expect("assignment errored"))
        .collect();

    let added = outcomes
        .iter()
        .filter(|o| **o == LabelAssignOutcome::Added)
        .count();
    let already = outcomes
        .iter()
        .filter(|o| **o == LabelAssignOutcome::AlreadyAssigned)
        .count();
    assert_eq!(added, 1, "{outcomes:?}");
    assert_eq!(already, CALLS - 1, "{outcomes:?}");
    assert_eq!(
        ctx.state
            .db
            .get_document_labels(document_id)
            .await
            .unwrap()
            .len(),
        1
    );
}
