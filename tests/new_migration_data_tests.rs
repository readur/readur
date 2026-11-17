//! Before/after tests for migrations that change existing data or behaviour.
//!
//! Each test migrates an empty database to the version just before the
//! migration under test, inserts data shaped for it, applies the remaining
//! migrations, and checks the outcome. (The shared test template is already
//! fully migrated, so it cannot show what a migration does to existing rows.)

use sqlx::PgPool;
use uuid::Uuid;

use readur::test_db::TestDatabase;

const OCR_QUEUE_DUPLICATES_VIA_INDEX: i64 = 20261116000001;
const FUZZY_THRESHOLD_WORD_SIMILARITY: i64 = 20261116120000;

/// The newest migration version older than `version`.
fn version_before(version: i64) -> i64 {
    sqlx::migrate!("./migrations")
        .iter()
        .map(|migration| migration.version)
        .filter(|&v| v < version)
        .max()
        .expect("no migration precedes the one under test")
}

/// An empty database migrated up to (and including) `version`.
async fn database_at(version: i64) -> (TestDatabase, PgPool) {
    let database = TestDatabase::empty().await;
    let pool = PgPool::connect(database.url())
        .await
        .expect("connect to test database");
    sqlx::migrate!("./migrations")
        .run_to(version, &pool)
        .await
        .expect("migrate to the version before the one under test");
    (database, pool)
}

async fn finish_migrations(pool: &PgPool) {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .expect("apply the remaining migrations");
}

async fn insert_user(pool: &PgPool) -> Uuid {
    let name = format!("migration_{}", Uuid::new_v4().simple());
    sqlx::query_scalar(
        "INSERT INTO users (username, email, password_hash) VALUES ($1, $2, 'x') RETURNING id",
    )
    .bind(&name)
    .bind(format!("{name}@example.com"))
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn insert_document(pool: &PgPool, user_id: Uuid) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO documents (user_id, filename, original_filename, file_path, file_size, mime_type) \
         VALUES ($1, 'a.txt', 'a.txt', '/tmp/a.txt', 1, 'text/plain') RETURNING id",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap()
}

/// `INSERT ... ON CONFLICT DO NOTHING` against the partial unique index, as
/// `OcrQueueService::enqueue_document` does: whether a job row was added.
async fn enqueue_unless_queued(pool: &PgPool, document_id: Uuid) -> Result<bool, sqlx::Error> {
    let inserted: Option<Uuid> = sqlx::query_scalar(
        "INSERT INTO ocr_queue (document_id, priority, file_size) VALUES ($1, 5, 1) \
         ON CONFLICT (document_id) WHERE status IN ('pending', 'processing') DO NOTHING RETURNING id",
    )
    .bind(document_id)
    .fetch_optional(pool)
    .await?;
    Ok(inserted.is_some())
}

#[tokio::test]
async fn ocr_queue_duplicates_are_left_to_the_unique_index() {
    let (_database, pool) = database_at(version_before(OCR_QUEUE_DUPLICATES_VIA_INDEX)).await;
    let user = insert_user(&pool).await;
    let document = insert_document(&pool, user).await;
    assert!(
        enqueue_unless_queued(&pool, document).await.unwrap(),
        "first job is added"
    );

    // Before: the guardrail trigger raised before ON CONFLICT could arbitrate.
    assert!(
        enqueue_unless_queued(&pool, document).await.is_err(),
        "expected the old trigger to reject the duplicate with an error"
    );

    finish_migrations(&pool).await;

    // After: the duplicate is a no-op, and the guarantees still hold.
    assert!(
        !enqueue_unless_queued(&pool, document).await.unwrap(),
        "duplicate is a no-op"
    );
    let plain_duplicate =
        sqlx::query("INSERT INTO ocr_queue (document_id, priority, file_size) VALUES ($1, 5, 1)")
            .bind(document)
            .execute(&pool)
            .await;
    let error = plain_duplicate.expect_err("a plain duplicate insert must still be rejected");
    assert!(
        error
            .as_database_error()
            .is_some_and(|e| e.is_unique_violation()),
        "{error}"
    );
    assert!(
        enqueue_unless_queued(&pool, Uuid::new_v4()).await.is_err(),
        "a job for a missing document must still be rejected"
    );
    let jobs: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ocr_queue WHERE document_id = $1")
        .bind(document)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(jobs, 1);
}

#[tokio::test]
async fn fuzzy_threshold_defaults_move_to_the_word_level_scale() {
    let (_database, pool) = database_at(version_before(FUZZY_THRESHOLD_WORD_SIMILARITY)).await;

    // (threshold stored before the migration, expected after it)
    let cases: [(Option<f32>, f32); 4] = [
        (Some(0.8), 0.5),
        (None, 0.5),
        (Some(0.3), 0.3),
        (Some(0.95), 0.95),
    ];
    let mut users = Vec::new();
    for (stored, expected) in cases {
        let user = insert_user(&pool).await;
        sqlx::query("INSERT INTO settings (user_id, fuzzy_search_threshold) VALUES ($1, $2)")
            .bind(user)
            .bind(stored)
            .execute(&pool)
            .await
            .unwrap();
        users.push((user, stored, expected));
    }

    finish_migrations(&pool).await;

    for (user, stored, expected) in users {
        let after: Option<f32> =
            sqlx::query_scalar("SELECT fuzzy_search_threshold FROM settings WHERE user_id = $1")
                .bind(user)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(after, Some(expected), "stored {stored:?}");
    }

    // New rows get the new default.
    let fresh = insert_user(&pool).await;
    let default: Option<f32> = sqlx::query_scalar(
        "INSERT INTO settings (user_id) VALUES ($1) RETURNING fuzzy_search_threshold",
    )
    .bind(fresh)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(default, Some(0.5));
}

#[test]
fn the_migrations_under_test_exist() {
    let versions: Vec<i64> = sqlx::migrate!("./migrations")
        .iter()
        .map(|m| m.version)
        .collect();
    for version in [
        OCR_QUEUE_DUPLICATES_VIA_INDEX,
        FUZZY_THRESHOLD_WORD_SIMILARITY,
    ] {
        assert!(
            versions.contains(&version),
            "migration {version} is gone; update this test"
        );
    }
}
