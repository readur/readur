//! Every account starts with the same five labels, and owns them: they can be renamed, deleted
//! and taken off documents like any other label, and one user's copies never reach another.

use readur::models::{CreateUser, UserRole};
use readur::test_utils::{
    document_helpers::{create_test_document, create_test_document_with_hash},
    TestAuthHelper, TestContext,
};
use serde_json::{json, Value};
use sqlx::{PgPool, Row};
use uuid::Uuid;

const DEFAULT_NAMES: [&str; 5] = ["Archive", "Important", "Personal", "To Review", "Work"];

async fn label_names(pool: &PgPool, user_id: Uuid) -> Vec<String> {
    sqlx::query_scalar("SELECT name FROM labels WHERE user_id = $1 ORDER BY name")
        .bind(user_id)
        .fetch_all(pool)
        .await
        .unwrap()
}

async fn label_id(pool: &PgPool, user_id: Uuid, name: &str) -> Uuid {
    sqlx::query_scalar("SELECT id FROM labels WHERE user_id = $1 AND name = $2")
        .bind(user_id)
        .bind(name)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn a_new_account_owns_the_five_default_labels() {
    let ctx = TestContext::new().await;
    let pool = ctx.state.db.get_pool();
    let auth = TestAuthHelper::new(ctx.app.clone());
    let user = auth.create_test_user().await;

    assert_eq!(
        label_names(pool, user.user_response.id).await,
        DEFAULT_NAMES
    );

    // Nothing is shared any more: no label is left without an owner.
    let unowned: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM labels WHERE user_id IS NULL")
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(unowned, 0);
}

#[tokio::test]
async fn a_single_sign_on_account_gets_them_too() {
    let ctx = TestContext::new().await;
    let pool = ctx.state.db.get_pool();
    let user = ctx
        .state
        .db
        .create_oidc_user(
            CreateUser {
                username: format!("oidc_{}", Uuid::new_v4().simple()),
                email: format!("oidc_{}@example.com", Uuid::new_v4().simple()),
                password: String::new(),
                role: Some(UserRole::User),
            },
            "subject-1",
            "https://issuer.example.com",
            "oidc@example.com",
        )
        .await
        .unwrap();

    assert_eq!(label_names(pool, user.id).await, DEFAULT_NAMES);
}

#[tokio::test]
async fn a_default_label_can_be_renamed_detached_and_deleted() {
    let ctx = TestContext::new().await;
    let pool = ctx.state.db.get_pool();
    let auth = TestAuthHelper::new(ctx.app.clone());
    let mut user = auth.create_test_user().await;
    let token = user.login(&auth).await.unwrap().to_string();
    let user_id = user.user_response.id;
    let important = label_id(pool, user_id, "Important").await;

    let renamed: Value = serde_json::from_slice(
        &auth
            .make_authenticated_request(
                "PUT",
                &format!("/api/labels/{important}"),
                Some(json!({ "name": "Urgent" })),
                &token,
            )
            .await,
    )
    .unwrap();
    assert_eq!(renamed["name"], "Urgent");

    let doc = ctx
        .state
        .db
        .create_document(create_test_document(user_id))
        .await
        .unwrap();
    let uri = format!("/api/labels/documents/{}", doc.id);
    auth.make_authenticated_request(
        "PUT",
        &uri,
        Some(json!({ "label_ids": [important] })),
        &token,
    )
    .await;
    auth.make_authenticated_request("PUT", &uri, Some(json!({ "label_ids": [] })), &token)
        .await;
    let on_doc: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM document_labels WHERE document_id = $1")
            .bind(doc.id)
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(on_doc, 0, "the label comes off the document");

    auth.make_authenticated_request("DELETE", &format!("/api/labels/{important}"), None, &token)
        .await;
    assert!(!label_names(pool, user_id)
        .await
        .contains(&"Urgent".to_string()));
}

#[tokio::test]
async fn one_users_defaults_and_counts_never_reach_another() {
    let ctx = TestContext::new().await;
    let pool = ctx.state.db.get_pool();
    let auth = TestAuthHelper::new(ctx.app.clone());
    let mut alice = auth.create_test_user().await;
    let bob = auth.create_test_user().await;
    let alice_token = alice.login(&auth).await.unwrap().to_string();

    // Bob tags two of his documents with his own "Important".
    let bobs_important = label_id(pool, bob.user_response.id, "Important").await;
    for n in 0..2 {
        let doc = ctx
            .state
            .db
            .create_document(create_test_document_with_hash(
                bob.user_response.id,
                "b.pdf",
                format!("hash-{n}"),
            ))
            .await
            .unwrap();
        sqlx::query("INSERT INTO document_labels (document_id, label_id) VALUES ($1, $2)")
            .bind(doc.id)
            .bind(bobs_important)
            .execute(pool)
            .await
            .unwrap();
    }

    let labels: Vec<Value> = serde_json::from_slice(
        &auth
            .make_authenticated_request(
                "GET",
                "/api/labels?include_counts=true",
                None,
                &alice_token,
            )
            .await,
    )
    .unwrap();
    assert_eq!(labels.len(), 5, "Alice sees only her own five: {labels:?}");
    assert!(labels
        .iter()
        .all(|l| l["user_id"] == json!(alice.user_response.id)));
    let important = labels.iter().find(|l| l["name"] == "Important").unwrap();
    assert_eq!(
        important["document_count"], 0,
        "Bob's documents are not counted on Alice's label"
    );

    // Alice cannot put Bob's label on her document.
    let doc = ctx
        .state
        .db
        .create_document(create_test_document(alice.user_response.id))
        .await
        .unwrap();
    auth.make_authenticated_request(
        "PUT",
        &format!("/api/labels/documents/{}", doc.id),
        Some(json!({ "label_ids": [bobs_important] })),
        &alice_token,
    )
    .await;
    let on_doc: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM document_labels WHERE document_id = $1")
            .bind(doc.id)
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(on_doc, 0);
}

/// Puts the database back in the shape it had before the conversion: five shared labels with no
/// owner, documents tagged with them. Returns (alice, bob, alice's doc, bob's doc, bob's own "Work").
async fn restore_shared_labels(pool: &PgPool) -> (Uuid, Uuid, Uuid, Uuid, Uuid) {
    sqlx::raw_sql(
        r#"
        DELETE FROM document_labels;
        DELETE FROM labels;
        ALTER TABLE labels ALTER COLUMN user_id DROP NOT NULL;
        ALTER TABLE labels ADD COLUMN is_system BOOLEAN NOT NULL DEFAULT FALSE;
        CREATE INDEX idx_labels_is_system ON labels(is_system);
        CREATE UNIQUE INDEX idx_system_labels_unique_name ON labels(name) WHERE user_id IS NULL;
        INSERT INTO labels (id, user_id, name, description, color, is_system) VALUES
            ('00000000-0000-0000-0000-000000000001', NULL, 'Important', 'High priority documents', '#d73a49', TRUE),
            ('00000000-0000-0000-0000-000000000002', NULL, 'To Review', 'Documents that need review', '#f66a0a', TRUE),
            ('00000000-0000-0000-0000-000000000003', NULL, 'Archive', 'Archived documents', '#6f42c1', TRUE),
            ('00000000-0000-0000-0000-000000000004', NULL, 'Work', 'Work-related documents', '#0969da', TRUE),
            ('00000000-0000-0000-0000-000000000005', NULL, 'Personal', 'Personal documents', '#1a7f37', TRUE);
        "#,
    )
    .execute(pool)
    .await
    .unwrap();

    let mut users = Vec::new();
    for name in ["alice", "bob"] {
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO users (username, email, password_hash, role) VALUES ($1, $2, 'x', 'user') RETURNING id",
        )
        .bind(format!("{name}_{}", Uuid::new_v4().simple()))
        .bind(format!("{name}_{}@example.com", Uuid::new_v4().simple()))
        .fetch_one(pool)
        .await
        .unwrap();
        users.push(id);
    }
    let (alice, bob) = (users[0], users[1]);

    let bobs_work: Uuid = sqlx::query_scalar(
        "INSERT INTO labels (user_id, name, color) VALUES ($1, 'Work', '#123456') RETURNING id",
    )
    .bind(bob)
    .fetch_one(pool)
    .await
    .unwrap();

    let mut docs = Vec::new();
    for owner in [alice, bob] {
        let doc_id: Uuid = sqlx::query_scalar(
            "INSERT INTO documents (filename, original_filename, file_path, file_size, mime_type, user_id)
             VALUES ('a.pdf', 'a.pdf', '/a.pdf', 1, 'application/pdf', $1) RETURNING id",
        )
        .bind(owner)
        .fetch_one(pool)
        .await
        .unwrap();
        // Shared "Important" and shared "Work" on both documents.
        for shared in [
            "00000000-0000-0000-0000-000000000001",
            "00000000-0000-0000-0000-000000000004",
        ] {
            sqlx::query("INSERT INTO document_labels (document_id, label_id, assigned_by) VALUES ($1, $2, $3)")
                .bind(doc_id)
                .bind(Uuid::parse_str(shared).unwrap())
                .bind(owner)
                .execute(pool)
                .await
                .unwrap();
        }
        docs.push(doc_id);
    }
    // Bob's document also already carries his own "Work": it must end up tagged once, not twice.
    sqlx::query("INSERT INTO document_labels (document_id, label_id) VALUES ($1, $2)")
        .bind(docs[1])
        .bind(bobs_work)
        .execute(pool)
        .await
        .unwrap();

    (alice, bob, docs[0], docs[1], bobs_work)
}

fn conversion_migration() -> String {
    let path = std::fs::read_dir("migrations")
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.to_string_lossy()
                .ends_with("_per_user_default_labels.sql")
        })
        .expect("the per-user default labels migration exists");
    std::fs::read_to_string(path).unwrap()
}

async fn labels_on(pool: &PgPool, doc: Uuid) -> Vec<(Uuid, String)> {
    sqlx::query("SELECT l.user_id, l.name FROM document_labels dl JOIN labels l ON l.id = dl.label_id WHERE dl.document_id = $1 ORDER BY l.name")
        .bind(doc)
        .fetch_all(pool)
        .await
        .unwrap()
        .iter()
        .map(|r| (r.get("user_id"), r.get("name")))
        .collect()
}

#[tokio::test]
async fn the_migration_gives_each_user_their_own_copy_and_keeps_every_tag() {
    let ctx = TestContext::new().await;
    let pool = ctx.state.db.get_pool();
    let (alice, bob, alice_doc, bob_doc, bobs_work) = restore_shared_labels(pool).await;

    sqlx::raw_sql(&conversion_migration())
        .execute(pool)
        .await
        .unwrap();

    assert_eq!(label_names(pool, alice).await, DEFAULT_NAMES);
    assert_eq!(label_names(pool, bob).await, DEFAULT_NAMES);
    assert_eq!(
        label_id(pool, bob, "Work").await,
        bobs_work,
        "Bob's own Work is kept, not duplicated"
    );

    assert_eq!(
        labels_on(pool, alice_doc).await,
        vec![(alice, "Important".into()), (alice, "Work".into())]
    );
    assert_eq!(
        labels_on(pool, bob_doc).await,
        vec![(bob, "Important".into()), (bob, "Work".into())]
    );
    let assigned_by: Option<Uuid> = sqlx::query_scalar(
        "SELECT assigned_by FROM document_labels WHERE document_id = $1 AND label_id = $2",
    )
    .bind(alice_doc)
    .bind(label_id(pool, alice, "Important").await)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(assigned_by, Some(alice), "who tagged it is kept");

    let leftovers: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM labels WHERE user_id IS NULL")
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(leftovers, 0);
    let column: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM information_schema.columns WHERE table_name = 'labels' AND column_name = 'is_system'",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(column, 0, "the is_system column is gone");

    // Running it again (as the latest-migration tests do) changes nothing.
    sqlx::raw_sql(&conversion_migration())
        .execute(pool)
        .await
        .unwrap();
    assert_eq!(label_names(pool, alice).await, DEFAULT_NAMES);
}
