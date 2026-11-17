//! `migrate_to_s3` pages through every local document exactly once (not just
//! each user's newest few hundred), and a re-run finds nothing it already
//! moved. The paging lives in `s3_service::migration` so it can be tested
//! against a real database here.

use std::collections::HashSet;

use chrono::{DateTime, TimeZone, Utc};
use uuid::Uuid;

use readur::{
    models::{CreateUser, UserRole},
    services::s3_service::migration::{LocalDocumentPager, MigrationCandidate},
    test_utils::{document_helpers::create_test_document_with_hash, TestContext},
};

async fn create_user(ctx: &TestContext, username: &str) -> Uuid {
    ctx.state
        .db
        .create_user(CreateUser {
            username: username.to_string(),
            email: format!("{username}@example.com"),
            password: "password123".to_string(),
            role: Some(UserRole::User),
        })
        .await
        .unwrap()
        .id
}

/// Insert a document with this path and creation time.
async fn insert_document(
    ctx: &TestContext,
    user_id: Uuid,
    n: usize,
    file_path: &str,
    created_at: DateTime<Utc>,
) -> Uuid {
    let mut doc = create_test_document_with_hash(
        user_id,
        &format!("doc{n}.pdf"),
        format!("hash-{user_id}-{n}"),
    );
    doc.file_path = file_path.to_string();
    let id = ctx.state.db.create_document(doc).await.unwrap().id;
    sqlx::query("UPDATE documents SET created_at = $2 WHERE id = $1")
        .bind(id)
        .bind(created_at)
        .execute(ctx.state.db.get_pool())
        .await
        .unwrap();
    id
}

fn at(seconds: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(1_700_000_000 + seconds, 0).unwrap()
}

/// Every document the pager returns, page by page, asserting no page is
/// larger than `page_size`.
async fn drain_pages(pager: &mut LocalDocumentPager, page_size: usize) -> Vec<MigrationCandidate> {
    let mut all = Vec::new();
    loop {
        let page = pager.next_page().await.unwrap();
        assert!(page.len() <= page_size);
        if page.is_empty() {
            return all;
        }
        all.extend(page);
    }
}

async fn mark_migrated(ctx: &TestContext, id: Uuid) {
    ctx.state
        .db
        .update_document_file_path(id, &format!("s3://documents/{id}.pdf"))
        .await
        .unwrap();
}

/// Two users, more local documents than several pages, timestamps shared
/// across page boundaries, and documents already on S3 mixed in.
struct Fixture {
    ctx: TestContext,
    alice: Uuid,
    /// Local documents in (created_at, id) order.
    local: Vec<Uuid>,
    alice_local: Vec<Uuid>,
}

async fn fixture() -> Fixture {
    let ctx = TestContext::new().await;
    let alice = create_user(&ctx, "alice-migrate").await;
    let bob = create_user(&ctx, "bob-migrate").await;
    let mut local = Vec::new();
    let mut alice_local = Vec::new();
    for n in 0..13 {
        let user = if n % 3 == 0 { bob } else { alice };
        // Pairs share a timestamp, so ordering must fall back to the id.
        let created = at((n / 2) as i64);
        if n % 5 == 4 {
            insert_document(
                &ctx,
                user,
                n,
                &format!("s3://documents/already-{n}.pdf"),
                created,
            )
            .await;
            continue;
        }
        let id = insert_document(&ctx, user, n, &format!("./uploads/doc{n}.pdf"), created).await;
        local.push((created, id, user));
    }
    local.sort();
    for &(_, id, user) in &local {
        if user == alice {
            alice_local.push(id);
        }
    }
    Fixture {
        ctx,
        alice,
        local: local.into_iter().map(|(_, id, _)| id).collect(),
        alice_local,
    }
}

fn ids(docs: &[MigrationCandidate]) -> Vec<Uuid> {
    docs.iter().map(|d| d.id).collect()
}

#[tokio::test]
async fn pager_visits_every_local_document_exactly_once_in_order() {
    let f = fixture().await;
    assert!(f.local.len() > 3 * 3, "fixture must span several pages");
    let pool = f.ctx.state.db.get_pool().clone();

    let mut pager = LocalDocumentPager::new(pool, None, 3);
    assert_eq!(
        pager.count_remaining().await.unwrap() as usize,
        f.local.len()
    );
    let visited = drain_pages(&mut pager, 3).await;

    assert_eq!(ids(&visited), f.local);
    assert!(visited.iter().all(|d| !d.file_path.starts_with("s3://")));
    // Once exhausted it stays exhausted.
    assert!(pager.next_page().await.unwrap().is_empty());
}

#[tokio::test]
async fn migrating_while_paging_skips_nothing_and_a_rerun_finds_nothing() {
    let f = fixture().await;
    let pool = f.ctx.state.db.get_pool().clone();

    // Migrate each document as soon as it is handed out, the way the tool
    // does; an OFFSET-based pager would skip rows here.
    let mut pager = LocalDocumentPager::new(pool.clone(), None, 4);
    let mut seen = Vec::new();
    while let Some(doc) = pager.next_document().await.unwrap() {
        mark_migrated(&f.ctx, doc.id).await;
        seen.push(doc.id);
    }
    assert_eq!(seen, f.local);
    assert_eq!(seen.iter().collect::<HashSet<_>>().len(), seen.len());

    let mut rerun = LocalDocumentPager::new(pool, None, 4);
    assert_eq!(rerun.count_remaining().await.unwrap(), 0);
    assert!(rerun.next_document().await.unwrap().is_none());
}

#[tokio::test]
async fn rerun_after_a_partial_run_visits_only_what_is_left() {
    let f = fixture().await;
    let pool = f.ctx.state.db.get_pool().clone();
    let (done, left) = f.local.split_at(5);
    for &id in done {
        mark_migrated(&f.ctx, id).await;
    }

    let mut pager = LocalDocumentPager::new(pool, None, 2);
    assert_eq!(ids(&drain_pages(&mut pager, 2).await), left);
}

#[tokio::test]
async fn user_filter_pages_through_all_of_that_users_documents() {
    let f = fixture().await;
    let pool = f.ctx.state.db.get_pool().clone();

    let mut pager = LocalDocumentPager::new(pool, Some(f.alice), 2);
    assert_eq!(
        pager.count_remaining().await.unwrap() as usize,
        f.alice_local.len()
    );
    assert_eq!(ids(&drain_pages(&mut pager, 2).await), f.alice_local);
}

#[tokio::test]
async fn resume_starts_at_the_given_document_inclusive() {
    let f = fixture().await;
    let pool = f.ctx.state.db.get_pool().clone();
    let start = 6;

    let mut pager = LocalDocumentPager::new(pool.clone(), None, 3)
        .resume_from(f.local[start])
        .await
        .unwrap();
    assert_eq!(
        pager.count_remaining().await.unwrap() as usize,
        f.local.len() - start
    );
    assert_eq!(ids(&drain_pages(&mut pager, 3).await), f.local[start..]);

    let unknown = LocalDocumentPager::new(pool, None, 3)
        .resume_from(Uuid::new_v4())
        .await;
    assert!(
        unknown.is_err(),
        "an unknown resume point must not silently restart"
    );
}

#[tokio::test]
async fn page_size_below_one_still_makes_progress() {
    let f = fixture().await;
    let mut pager = LocalDocumentPager::new(f.ctx.state.db.get_pool().clone(), None, 0);
    assert_eq!(ids(&drain_pages(&mut pager, 1).await), f.local);
}
