//! `POST /api/labels/bulk/documents` and `DELETE /api/labels/{id}` enforce ownership inside the
//! statements that write, not in a separate check beforehand: a foreign or missing document or
//! label changes nothing, concurrent bulk updates never interleave into a mixed state, and a label
//! deleted while it is being assigned never turns the assignment into a 500.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use readur::test_utils::{document_helpers::create_test_document_with_hash, TestContext};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

async fn send(
    ctx: &TestContext,
    token: &str,
    method: &str,
    uri: &str,
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
        .app()
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

async fn bulk(
    ctx: &TestContext,
    token: &str,
    mode: &str,
    document_ids: &[Uuid],
    label_ids: &[Uuid],
) -> (StatusCode, Value) {
    let body = json!({ "document_ids": document_ids, "label_ids": label_ids, "mode": mode });
    send(ctx, token, "POST", "/api/labels/bulk/documents", Some(body)).await
}

async fn delete_label(ctx: &TestContext, token: &str, label_id: Uuid) -> StatusCode {
    send(
        ctx,
        token,
        "DELETE",
        &format!("/api/labels/{label_id}"),
        None,
    )
    .await
    .0
}

async fn assign(ctx: &TestContext, token: &str, document_id: Uuid, label_id: Uuid) -> StatusCode {
    let uri = format!("/api/labels/documents/{document_id}/labels/{label_id}");
    send(ctx, token, "POST", &uri, None).await.0
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

async fn label_exists(pool: &PgPool, label_id: Uuid) -> bool {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM labels WHERE id = $1)")
        .bind(label_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn assignments_of(pool: &PgPool, label_id: Uuid) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM document_labels WHERE label_id = $1")
        .bind(label_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

fn sorted(ids: &[Uuid]) -> Vec<Uuid> {
    let mut ids = ids.to_vec();
    ids.sort();
    ids
}

struct Account {
    id: Uuid,
    token: String,
    /// The five default labels, sorted by id.
    labels: Vec<Uuid>,
    documents: Vec<Uuid>,
}

async fn account(ctx: &TestContext, documents: usize) -> Account {
    let helper = ctx.auth_helper();
    let mut user = helper.create_test_user().await;
    let token = user.login(&helper).await.unwrap().to_string();
    let id = user.user_response.id;
    let labels = sqlx::query_scalar("SELECT id FROM labels WHERE user_id = $1 ORDER BY id")
        .bind(id)
        .fetch_all(ctx.state.db.get_pool())
        .await
        .unwrap();
    let mut ids = Vec::new();
    for n in 0..documents {
        let document =
            create_test_document_with_hash(id, &format!("doc{n}.pdf"), format!("hash-{id}-{n}"));
        ids.push(ctx.state.db.create_document(document).await.unwrap().id);
    }
    Account {
        id,
        token,
        labels,
        documents: ids,
    }
}

/// A fresh label of `owner`'s, created through the API.
async fn new_label(ctx: &TestContext, owner: &Account, name: &str) -> Uuid {
    let (status, body) = send(
        ctx,
        &owner.token,
        "POST",
        "/api/labels",
        Some(json!({ "name": name })),
    )
    .await;
    assert!(status.is_success(), "{status}: {body}");
    body["id"].as_str().unwrap().parse().unwrap()
}

#[tokio::test]
async fn bulk_modes_keep_their_response_shape() {
    let ctx = TestContext::new().await;
    let pool = ctx.state.db.get_pool();
    let a = account(&ctx, 2).await;

    let (status, body) = bulk(&ctx, &a.token, "replace", &a.documents, &a.labels[..2]).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body,
        json!({ "message": "Labels replaced successfully", "documents_updated": 2 })
    );
    for doc in &a.documents {
        assert_eq!(labels_on(pool, *doc).await, sorted(&a.labels[..2]));
    }

    let (status, body) = bulk(&ctx, &a.token, "add", &a.documents[..1], &a.labels[2..3]).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["documents_updated"], 1);
    assert_eq!(
        labels_on(pool, a.documents[0]).await,
        sorted(&a.labels[..3])
    );
    assert_eq!(
        labels_on(pool, a.documents[1]).await,
        sorted(&a.labels[..2])
    );

    let (status, body) = bulk(&ctx, &a.token, "remove", &a.documents, &a.labels[..1]).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["documents_updated"], 2);
    assert_eq!(
        labels_on(pool, a.documents[0]).await,
        sorted(&a.labels[1..3])
    );
    assert_eq!(
        labels_on(pool, a.documents[1]).await,
        sorted(&a.labels[1..2])
    );

    // Replacing with no labels clears them; the mode defaults to replace.
    let body = json!({ "document_ids": a.documents, "label_ids": [] });
    let (status, _) = send(
        &ctx,
        &a.token,
        "POST",
        "/api/labels/bulk/documents",
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    for doc in &a.documents {
        assert!(labels_on(pool, *doc).await.is_empty());
    }

    let (status, _) = bulk(&ctx, &a.token, "delete", &a.documents, &a.labels[..1]).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_foreign_missing_or_repeated_document_changes_nothing() {
    let ctx = TestContext::new().await;
    let pool = ctx.state.db.get_pool();
    let a = account(&ctx, 2).await;
    let b = account(&ctx, 1).await;
    let (status, _) = bulk(&ctx, &a.token, "replace", &a.documents, &a.labels[..1]).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = bulk(&ctx, &b.token, "replace", &b.documents, &b.labels[..1]).await;
    assert_eq!(status, StatusCode::OK);

    let bad_document_sets = [
        vec![a.documents[0], b.documents[0]],
        vec![a.documents[0], Uuid::new_v4()],
        vec![a.documents[0], a.documents[0]],
        b.documents.clone(),
    ];
    for documents in &bad_document_sets {
        for mode in ["replace", "add", "remove"] {
            for labels in [&a.labels[1..3], &a.labels[..1], &[][..]] {
                let (status, body) = bulk(&ctx, &a.token, mode, documents, labels).await;
                assert_eq!(
                    status,
                    StatusCode::BAD_REQUEST,
                    "{mode} {documents:?}: {body}"
                );
            }
        }
    }
    for doc in &a.documents {
        assert_eq!(labels_on(pool, *doc).await, vec![a.labels[0]]);
    }
    assert_eq!(labels_on(pool, b.documents[0]).await, vec![b.labels[0]]);
}

#[tokio::test]
async fn a_foreign_missing_or_repeated_label_changes_nothing() {
    let ctx = TestContext::new().await;
    let pool = ctx.state.db.get_pool();
    let a = account(&ctx, 2).await;
    let b = account(&ctx, 0).await;
    let (status, _) = bulk(&ctx, &a.token, "replace", &a.documents, &a.labels[..2]).await;
    assert_eq!(status, StatusCode::OK);

    let bad_label_sets = [
        vec![a.labels[2], b.labels[0]],
        vec![b.labels[0]],
        vec![a.labels[0], Uuid::new_v4()],
        vec![a.labels[3], a.labels[3]],
    ];
    for labels in &bad_label_sets {
        for mode in ["replace", "add", "remove"] {
            let (status, body) = bulk(&ctx, &a.token, mode, &a.documents, labels).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{mode} {labels:?}: {body}");
        }
    }
    for doc in &a.documents {
        assert_eq!(labels_on(pool, *doc).await, sorted(&a.labels[..2]));
    }
    assert_eq!(assignments_of(pool, b.labels[0]).await, 0);
}

#[tokio::test]
async fn concurrent_bulk_replacements_never_mix_label_sets() {
    let ctx = TestContext::new().await;
    let pool = ctx.state.db.get_pool();
    let a = account(&ctx, 4).await;
    // Disjoint sets, so any interleaving shows up as a union or as documents that disagree.
    let sets: Vec<Vec<Uuid>> = vec![
        a.labels[0..2].to_vec(),
        a.labels[2..4].to_vec(),
        a.labels[4..5].to_vec(),
        vec![],
    ];
    let forward = a.documents.clone();
    let mut backward = a.documents.clone();
    backward.reverse();

    for round in 0..5 {
        // Half the requests list the documents in reverse: rows locked in request order would
        // deadlock against the other half.
        let requests = (0..12).map(|n| {
            let documents = if n % 2 == 0 { &forward } else { &backward };
            bulk(&ctx, &a.token, "replace", documents, &sets[n % sets.len()])
        });
        for (status, body) in futures::future::join_all(requests).await {
            assert_eq!(status, StatusCode::OK, "round {round}: {body}");
        }

        let first = labels_on(pool, a.documents[0]).await;
        assert!(
            sets.iter().any(|set| sorted(set) == first),
            "round {round}: {first:?} is a mix of several requests"
        );
        for doc in &a.documents[1..] {
            assert_eq!(
                labels_on(pool, *doc).await,
                first,
                "round {round}: the documents of one bulk update ended with different labels"
            );
        }
    }
}

#[tokio::test]
async fn deleting_a_foreign_or_missing_label_is_404_and_deletes_nothing() {
    let ctx = TestContext::new().await;
    let pool = ctx.state.db.get_pool();
    let a = account(&ctx, 1).await;
    let b = account(&ctx, 0).await;
    let (status, _) = bulk(&ctx, &a.token, "replace", &a.documents, &a.labels[..1]).await;
    assert_eq!(status, StatusCode::OK);

    assert_eq!(
        delete_label(&ctx, &b.token, a.labels[0]).await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        delete_label(&ctx, &a.token, Uuid::new_v4()).await,
        StatusCode::NOT_FOUND
    );
    assert!(label_exists(pool, a.labels[0]).await);
    assert_eq!(labels_on(pool, a.documents[0]).await, vec![a.labels[0]]);
    let b_labels: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM labels WHERE user_id = $1")
        .bind(b.id)
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(b_labels, 5);

    // The owner can delete it; its assignments go with it, and a second delete is 404.
    assert_eq!(
        delete_label(&ctx, &a.token, a.labels[0]).await,
        StatusCode::NO_CONTENT
    );
    assert!(!label_exists(pool, a.labels[0]).await);
    assert!(labels_on(pool, a.documents[0]).await.is_empty());
    assert_eq!(
        delete_label(&ctx, &a.token, a.labels[0]).await,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn deleting_a_label_while_it_is_assigned_never_errors() {
    let ctx = TestContext::new().await;
    let pool = ctx.state.db.get_pool();
    let a = account(&ctx, 4).await;

    for round in 0..15 {
        let label = new_label(&ctx, &a, &format!("racing-{round}")).await;
        let assignments = a
            .documents
            .iter()
            .map(|doc| assign(&ctx, &a.token, *doc, label));
        let only_label = [label];
        let bulk_add = bulk(&ctx, &a.token, "add", &a.documents, &only_label);
        let (deleted, assigned, (bulk_status, bulk_body)) = tokio::join!(
            delete_label(&ctx, &a.token, label),
            futures::future::join_all(assignments),
            bulk_add,
        );

        assert_eq!(deleted, StatusCode::NO_CONTENT, "round {round}");
        for status in assigned {
            assert!(
                status == StatusCode::CREATED || status == StatusCode::NOT_FOUND,
                "round {round}: assigning a label being deleted returned {status}"
            );
        }
        assert!(
            bulk_status == StatusCode::OK || bulk_status == StatusCode::BAD_REQUEST,
            "round {round}: bulk add of a label being deleted returned {bulk_status}: {bulk_body}"
        );
        assert!(!label_exists(pool, label).await);
        assert_eq!(
            assignments_of(pool, label).await,
            0,
            "round {round}: orphaned assignment"
        );
    }
}

#[tokio::test]
async fn attaching_upload_labels_skips_anything_not_the_users() {
    let ctx = TestContext::new().await;
    let pool = ctx.state.db.get_pool();
    let a = account(&ctx, 1).await;
    let b = account(&ctx, 1).await;
    let db = &ctx.state.db;

    db.add_labels_to_document(
        a.documents[0],
        &[a.labels[0], b.labels[0], Uuid::new_v4()],
        a.id,
    )
    .await
    .unwrap();
    assert_eq!(labels_on(pool, a.documents[0]).await, vec![a.labels[0]]);

    // Someone else's document is left alone, whichever labels are named.
    db.add_labels_to_document(b.documents[0], &[a.labels[1], b.labels[1]], a.id)
        .await
        .unwrap();
    assert!(labels_on(pool, b.documents[0]).await.is_empty());

    // Repeating an attachment is a no-op.
    db.add_labels_to_document(a.documents[0], &[a.labels[0], a.labels[0]], a.id)
        .await
        .unwrap();
    assert_eq!(labels_on(pool, a.documents[0]).await, vec![a.labels[0]]);
}
