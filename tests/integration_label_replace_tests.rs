//! `PUT /api/labels/documents/{id}` replaces a document's labels in one transaction, with
//! ownership enforced inside the statements: a foreign document or label changes nothing, and
//! concurrent replacements of the same document never mix their label sets.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use readur::test_utils::{document_helpers::create_test_document, TestContext};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

async fn put_labels(
    ctx: &TestContext,
    token: &str,
    document_id: Uuid,
    label_ids: &[Uuid],
) -> (StatusCode, Value) {
    let request = Request::builder()
        .method("PUT")
        .uri(format!("/api/labels/documents/{document_id}"))
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(json!({ "label_ids": label_ids }).to_string()))
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

/// The label ids on `document_id`, sorted.
async fn labels_on(pool: &PgPool, document_id: Uuid) -> Vec<Uuid> {
    let mut ids: Vec<Uuid> =
        sqlx::query_scalar("SELECT label_id FROM document_labels WHERE document_id = $1")
            .bind(document_id)
            .fetch_all(pool)
            .await
            .unwrap();
    ids.sort();
    ids
}

/// The user's five default labels, sorted by id.
async fn default_labels(pool: &PgPool, user_id: Uuid) -> Vec<Uuid> {
    sqlx::query_scalar("SELECT id FROM labels WHERE user_id = $1 ORDER BY id")
        .bind(user_id)
        .fetch_all(pool)
        .await
        .unwrap()
}

fn sorted(ids: &[Uuid]) -> Vec<Uuid> {
    let mut ids = ids.to_vec();
    ids.sort();
    ids
}

fn ids_in(body: &Value) -> Vec<Uuid> {
    let mut ids: Vec<Uuid> = body
        .as_array()
        .unwrap_or_else(|| panic!("expected a label list, got {body}"))
        .iter()
        .map(|label| label["id"].as_str().unwrap().parse().unwrap())
        .collect();
    ids.sort();
    ids
}

struct Fixture {
    ctx: TestContext,
    token: String,
    labels: Vec<Uuid>,
    document_id: Uuid,
}

async fn fixture() -> Fixture {
    let ctx = TestContext::new().await;
    let helper = ctx.auth_helper();
    let mut user = helper.create_test_user().await;
    let token = user.login(&helper).await.unwrap().to_string();
    let user_id = user.user_response.id;
    let labels = default_labels(ctx.state.db.get_pool(), user_id).await;
    let document_id = ctx
        .state
        .db
        .create_document(create_test_document(user_id))
        .await
        .unwrap()
        .id;
    Fixture {
        ctx,
        token,
        labels,
        document_id,
    }
}

#[tokio::test]
async fn replacing_returns_exactly_the_committed_set() {
    let f = fixture().await;
    let pool = f.ctx.state.db.get_pool();

    let (status, body) = put_labels(&f.ctx, &f.token, f.document_id, &f.labels[..2]).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(ids_in(&body), sorted(&f.labels[..2]));
    assert_eq!(labels_on(pool, f.document_id).await, sorted(&f.labels[..2]));

    let (status, body) = put_labels(&f.ctx, &f.token, f.document_id, &f.labels[2..4]).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(ids_in(&body), sorted(&f.labels[2..4]));
    assert_eq!(
        labels_on(pool, f.document_id).await,
        sorted(&f.labels[2..4])
    );

    let (status, body) = put_labels(&f.ctx, &f.token, f.document_id, &[]).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body, json!([]));
    assert!(labels_on(pool, f.document_id).await.is_empty());
}

#[tokio::test]
async fn a_foreign_or_repeated_label_changes_nothing() {
    let f = fixture().await;
    let pool = f.ctx.state.db.get_pool();
    let helper = f.ctx.auth_helper();
    let other = helper.create_test_user().await;
    let foreign_label = default_labels(pool, other.user_response.id).await[0];

    let (status, _) = put_labels(&f.ctx, &f.token, f.document_id, &f.labels[..1]).await;
    assert_eq!(status, StatusCode::OK);

    // One of the requested labels is someone else's: 400, and the old labels stay.
    let (status, _) = put_labels(
        &f.ctx,
        &f.token,
        f.document_id,
        &[f.labels[1], foreign_label],
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(labels_on(pool, f.document_id).await, vec![f.labels[0]]);

    // So is an id that is not a label at all, and a label listed twice.
    for ids in [vec![Uuid::new_v4()], vec![f.labels[2], f.labels[2]]] {
        let (status, _) = put_labels(&f.ctx, &f.token, f.document_id, &ids).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{ids:?}");
        assert_eq!(labels_on(pool, f.document_id).await, vec![f.labels[0]]);
    }
}

#[tokio::test]
async fn a_foreign_document_is_not_found_and_left_alone() {
    let f = fixture().await;
    let pool = f.ctx.state.db.get_pool();
    let helper = f.ctx.auth_helper();
    let mut other = helper.create_test_user().await;
    let other_token = other.login(&helper).await.unwrap().to_string();
    let other_labels = default_labels(pool, other.user_response.id).await;

    let (status, _) = put_labels(&f.ctx, &f.token, f.document_id, &f.labels[..2]).await;
    assert_eq!(status, StatusCode::OK);

    // Another user, even naming only their own labels, cannot touch this document.
    for ids in [vec![], other_labels[..1].to_vec(), f.labels[2..3].to_vec()] {
        let (status, _) = put_labels(&f.ctx, &other_token, f.document_id, &ids).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{ids:?}");
        assert_eq!(labels_on(pool, f.document_id).await, sorted(&f.labels[..2]));
    }

    // A document that does not exist is also 404.
    let (status, _) = put_labels(&f.ctx, &f.token, Uuid::new_v4(), &f.labels[..1]).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn concurrent_replacements_never_mix_label_sets() {
    let f = fixture().await;
    let pool = f.ctx.state.db.get_pool();
    // Disjoint sets, so any interleaving of two replacements would show up as a union.
    let sets: Vec<Vec<Uuid>> = vec![
        f.labels[0..2].to_vec(),
        f.labels[2..4].to_vec(),
        f.labels[4..5].to_vec(),
        vec![],
    ];

    for round in 0..5 {
        let requests = sets
            .iter()
            .cycle()
            .take(12)
            .map(|set| put_labels(&f.ctx, &f.token, f.document_id, set));
        let results = futures::future::join_all(requests).await;

        for ((status, body), set) in results.iter().zip(sets.iter().cycle()) {
            assert_eq!(*status, StatusCode::OK, "round {round}: {body}");
            // Each response is read inside its own transaction: exactly its own set.
            assert_eq!(ids_in(body), sorted(set), "round {round}");
        }
        let committed = labels_on(pool, f.document_id).await;
        assert!(
            sets.iter().any(|set| sorted(set) == committed),
            "round {round}: the document ended with {committed:?}, a mix of several requests"
        );
    }
}
