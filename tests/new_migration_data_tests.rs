//! Before/after tests for migrations that change existing data or behaviour.
//!
//! Each test migrates an empty database to the version just before the
//! migration under test, inserts data shaped for it, applies the remaining
//! migrations, and checks the outcome. (The shared test template is already
//! fully migrated, so it cannot show what a migration does to existing rows.)

use sqlx::PgPool;
use uuid::Uuid;

use readur::test_db::TestDatabase;

const FAILED_OCR_TO_FAILED_DOCUMENTS: i64 = 20250628000004;
const OCR_QUEUE_DUPLICATES_VIA_INDEX: i64 = 20261116000001;
const FUZZY_THRESHOLD_WORD_SIMILARITY: i64 = 20261116120000;
const OCR_QUEUE_CLAIM_AND_HEARTBEAT: i64 = 20261117120000;
const RESET_STUCK_FAILS_EXHAUSTED_DOCUMENTS: i64 = 20261117130000;
const RESET_STUCK_RECORDS_EVERY_EXHAUSTED_JOB: i64 = 20261117140000;

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

/// A job running when the migration applies gets its start as its last
/// heartbeat and no claim (no new worker owns it); stale-job recovery then
/// judges it by that heartbeat. The guardrail `reset_stuck_ocr_jobs` stops
/// resetting a live job, and a live job's document, by start time.
#[tokio::test]
async fn ocr_queue_jobs_gain_claims_and_heartbeats() {
    let (_db, pool) = database_at(version_before(OCR_QUEUE_CLAIM_AND_HEARTBEAT)).await;
    let user = insert_user(&pool).await;
    let running = insert_document(&pool, user).await;
    let waiting = insert_document(&pool, user).await;
    for (document, status) in [(running, "processing"), (waiting, "pending")] {
        sqlx::query(
            "INSERT INTO ocr_queue (document_id, priority, file_size, status, started_at, attempts) \
             VALUES ($1, 5, 1, $2, CASE WHEN $2 = 'processing' THEN NOW() - INTERVAL '40 minutes' END, 1)",
        )
        .bind(document)
        .bind(status)
        .execute(&pool)
        .await
        .unwrap();
    }

    finish_migrations(&pool).await;

    let (claim, beat_is_start): (Option<Uuid>, bool) = sqlx::query_as(
        "SELECT claim_id, heartbeat_at = started_at FROM ocr_queue WHERE document_id = $1",
    )
    .bind(running)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(claim, None);
    assert!(beat_is_start, "a running job's last heartbeat is its start");
    let waiting_beat: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT heartbeat_at FROM ocr_queue WHERE document_id = $1")
            .bind(waiting)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(waiting_beat, None, "a queued job has no heartbeat");

    // A live job claimed 40 minutes ago, heartbeating now: not stuck.
    sqlx::query("UPDATE ocr_queue SET heartbeat_at = NOW() WHERE document_id = $1")
        .bind(running)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE documents SET ocr_status = 'processing', updated_at = NOW() - INTERVAL '40 minutes' \
         WHERE id = $1",
    )
    .bind(running)
    .execute(&pool)
    .await
    .unwrap();
    let reset: i32 = sqlx::query_scalar("SELECT reset_stuck_ocr_jobs(30)")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(reset, 0, "a heartbeating job is not stuck");
    let status: Option<String> =
        sqlx::query_scalar("SELECT ocr_status FROM documents WHERE id = $1")
            .bind(running)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status.as_deref(), Some("processing"));

    // Silent for the threshold: reset, its document with it.
    sqlx::query(
        "UPDATE ocr_queue SET heartbeat_at = NOW() - INTERVAL '31 minutes' WHERE document_id = $1",
    )
    .bind(running)
    .execute(&pool)
    .await
    .unwrap();
    let reset: i32 = sqlx::query_scalar("SELECT reset_stuck_ocr_jobs(30)")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(reset, 1);
    let (job_status, doc_status): (String, Option<String>) = sqlx::query_as(
        "SELECT q.status, d.ocr_status FROM ocr_queue q JOIN documents d ON d.id = q.document_id \
         WHERE q.document_id = $1",
    )
    .bind(running)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        (job_status.as_str(), doc_status.as_deref()),
        ("pending", Some("pending"))
    );
}

#[test]
fn the_migrations_under_test_exist() {
    let versions: Vec<i64> = sqlx::migrate!("./migrations")
        .iter()
        .map(|m| m.version)
        .collect();
    for version in [
        FAILED_OCR_TO_FAILED_DOCUMENTS,
        OCR_QUEUE_DUPLICATES_VIA_INDEX,
        FUZZY_THRESHOLD_WORD_SIMILARITY,
        OCR_QUEUE_CLAIM_AND_HEARTBEAT,
        RESET_STUCK_FAILS_EXHAUSTED_DOCUMENTS,
        RESET_STUCK_RECORDS_EVERY_EXHAUSTED_JOB,
    ] {
        assert!(
            versions.contains(&version),
            "migration {version} is gone; update this test"
        );
    }
}

/// A document whose stale job had no attempts left and is processing.
async fn exhausted_stale_job(pool: &PgPool, user: Uuid) -> Uuid {
    let document = insert_document(pool, user).await;
    set_ocr_status(pool, document, "processing").await;
    insert_exhausted_stale_job(pool, document).await;
    document
}

async fn set_ocr_status(pool: &PgPool, document: Uuid, ocr_status: &str) {
    sqlx::query("UPDATE documents SET ocr_status = $2 WHERE id = $1")
        .bind(document)
        .bind(ocr_status)
        .execute(pool)
        .await
        .unwrap();
}

/// A 'processing' job for `document` with no attempts left and no heartbeat
/// for over 30 minutes.
async fn insert_exhausted_stale_job(pool: &PgPool, document: Uuid) {
    sqlx::query(
        "INSERT INTO ocr_queue (document_id, priority, file_size, status, started_at, \
         heartbeat_at, attempts, max_attempts) \
         VALUES ($1, 5, 1, 'processing', NOW() - INTERVAL '2 hours', \
         NOW() - INTERVAL '31 minutes', 3, 3)",
    )
    .bind(document)
    .execute(pool)
    .await
    .unwrap();
}

/// `(job status, document ocr_status, ocr_failure_reason, failure records)`.
async fn exhausted_state(
    pool: &PgPool,
    document: Uuid,
) -> (String, Option<String>, Option<String>, i64) {
    sqlx::query_as(
        "SELECT q.status, d.ocr_status, d.ocr_failure_reason, \
         (SELECT COUNT(*) FROM failed_documents f \
          WHERE f.user_id = d.user_id AND f.file_path = d.file_path) \
         FROM ocr_queue q JOIN documents d ON d.id = q.document_id WHERE q.document_id = $1",
    )
    .bind(document)
    .fetch_one(pool)
    .await
    .unwrap()
}

/// A stale job with no attempts left is failed by `reset_stuck_ocr_jobs`;
/// before this migration its document stayed 'processing' forever, after it
/// the document fails too ('ocr_timeout', with a failure record).
#[tokio::test]
async fn reset_stuck_ocr_jobs_fails_the_documents_of_exhausted_jobs() {
    let (_db, pool) = database_at(version_before(RESET_STUCK_FAILS_EXHAUSTED_DOCUMENTS)).await;
    let before = insert_user(&pool).await;
    let stranded = exhausted_stale_job(&pool, before).await;
    let reset: i32 = sqlx::query_scalar("SELECT reset_stuck_ocr_jobs(30)")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(reset, 1);
    let (job, doc, _, _) = exhausted_state(&pool, stranded).await;
    assert_eq!(
        (job.as_str(), doc.as_deref()),
        ("failed", Some("processing")),
        "the old function strands the document"
    );

    finish_migrations(&pool).await;

    let (job, doc, reason, records) = exhausted_state(&pool, stranded).await;
    assert_eq!(
        (job.as_str(), doc.as_deref(), reason.as_deref(), records),
        ("failed", Some("failed"), Some("ocr_timeout"), 1),
        "a document the old function stranded is repaired"
    );

    let after = insert_user(&pool).await;
    let exhausted = exhausted_stale_job(&pool, after).await;
    // Another owner: failure records are counted per owner and file.
    let requeued = insert_document(&pool, insert_user(&pool).await).await;
    sqlx::query("UPDATE documents SET ocr_status = 'processing' WHERE id = $1")
        .bind(requeued)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO ocr_queue (document_id, priority, file_size, status, started_at, \
         heartbeat_at, attempts, max_attempts) \
         VALUES ($1, 5, 1, 'processing', NOW() - INTERVAL '2 hours', \
         NOW() - INTERVAL '31 minutes', 1, 3)",
    )
    .bind(requeued)
    .execute(&pool)
    .await
    .unwrap();
    let reset: i32 = sqlx::query_scalar("SELECT reset_stuck_ocr_jobs(30)")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(reset, 2);
    let (job, doc, reason, records) = exhausted_state(&pool, exhausted).await;
    assert_eq!(
        (job.as_str(), doc.as_deref(), reason.as_deref(), records),
        ("failed", Some("failed"), Some("ocr_timeout"), 1)
    );
    let (job, doc, _, records) = exhausted_state(&pool, requeued).await;
    assert_eq!(
        (job.as_str(), doc.as_deref(), records),
        ("pending", Some("pending"), 0),
        "a job with attempts left still goes back to the queue"
    );
}

/// Before 20261117140000 an exhausted stale job got a failure record only
/// while its document was 'processing'; a document already 'failed' by an
/// earlier attempt (dequeuing leaves it 'failed') was skipped. Now every
/// exhausted job is recorded, as `record_failed_attempt` records it -- but a
/// completed document keeps its status and text.
#[tokio::test]
async fn reset_stuck_ocr_jobs_records_every_exhausted_job() {
    let (_db, pool) = database_at(version_before(RESET_STUCK_RECORDS_EVERY_EXHAUSTED_JOB)).await;
    let skipped = exhausted_stale_job_of(&pool, "failed").await;
    reset_stuck(&pool, 1).await;
    let (_, doc, _, records) = exhausted_state(&pool, skipped).await;
    assert_eq!(
        (doc.as_deref(), records),
        (Some("failed"), 0),
        "the old function skips an already-failed document"
    );

    finish_migrations(&pool).await;

    let failed_before = exhausted_stale_job_of(&pool, "failed").await;
    let completed = exhausted_stale_job_of(&pool, "completed").await;
    reset_stuck(&pool, 2).await;
    let (job, doc, reason, records) = exhausted_state(&pool, failed_before).await;
    assert_eq!(
        (job.as_str(), doc.as_deref(), reason.as_deref(), records),
        ("failed", Some("failed"), Some("ocr_timeout"), 1)
    );
    let (job, doc, reason, records) = exhausted_state(&pool, completed).await;
    assert_eq!(
        (job.as_str(), doc.as_deref(), reason.as_deref(), records),
        ("failed", Some("completed"), None, 1),
        "a completed document is not failed, though its job's failure is recorded"
    );
}

/// `reset_stuck_ocr_jobs` used to RAISE NOTICE on every call, and the OCR
/// queue's stale recovery calls it every few minutes; the Rust caller logs
/// when it recovers anything instead.
#[tokio::test]
async fn reset_stuck_ocr_jobs_raises_no_notice() {
    let (_db, pool) = database_at(version_before(RESET_STUCK_RECORDS_EVERY_EXHAUSTED_JOB)).await;
    assert!(reset_stuck_source(&pool).await.contains("RAISE NOTICE"));
    finish_migrations(&pool).await;
    assert!(!reset_stuck_source(&pool).await.contains("RAISE NOTICE"));
}

async fn reset_stuck_source(pool: &PgPool) -> String {
    sqlx::query_scalar("SELECT prosrc FROM pg_proc WHERE proname = 'reset_stuck_ocr_jobs'")
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn reset_stuck(pool: &PgPool, expected: i32) {
    let reset: i32 = sqlx::query_scalar("SELECT reset_stuck_ocr_jobs(30)")
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(reset, expected);
}

/// An exhausted stale job whose document (of a new owner) has `ocr_status`.
/// The status is set first: completing a document deletes its jobs.
async fn exhausted_stale_job_of(pool: &PgPool, ocr_status: &str) -> Uuid {
    let document = insert_document(pool, insert_user(pool).await).await;
    set_ocr_status(pool, document, ocr_status).await;
    insert_exhausted_stale_job(pool, document).await;
    document
}

#[tokio::test]
async fn failed_ocr_documents_move_to_failed_documents() {
    let (_database, pool) = database_at(version_before(FAILED_OCR_TO_FAILED_DOCUMENTS)).await;
    let user = insert_user(&pool).await;

    // (stored ocr_failure_reason, expected failure_reason after the move)
    let cases = [
        (Some("timeout"), "ocr_timeout"),
        (Some("memory_limit"), "ocr_memory_limit"),
        (Some("file_corrupted"), "file_corrupted"),
        (Some("low_ocr_confidence"), "low_ocr_confidence"),
        (Some("something_new"), "other"),
        (None, "other"),
    ];
    let mut failed = Vec::new();
    for (index, (reason, expected)) in cases.into_iter().enumerate() {
        let document = insert_document(&pool, user).await;
        let filename = format!("failed_{index}.pdf");
        sqlx::query(
            "UPDATE documents SET filename = $2, ocr_status = 'failed', ocr_failure_reason = $3, \
             ocr_error = 'boom' WHERE id = $1",
        )
        .bind(document)
        .bind(&filename)
        .bind(reason)
        .execute(&pool)
        .await
        .unwrap();
        failed.push((filename, expected));
    }
    let completed = insert_document(&pool, user).await;
    sqlx::query("UPDATE documents SET ocr_status = 'completed' WHERE id = $1")
        .bind(completed)
        .execute(&pool)
        .await
        .unwrap();

    sqlx::migrate!("./migrations")
        .run_to(FAILED_OCR_TO_FAILED_DOCUMENTS, &pool)
        .await
        .expect("apply the backfill");

    for (filename, expected) in failed {
        let (reason, stage, source, error): (String, String, String, Option<String>) =
            sqlx::query_as(
                "SELECT failure_reason, failure_stage, ingestion_source, error_message \
             FROM failed_documents WHERE filename = $1",
            )
            .bind(&filename)
            .fetch_one(&pool)
            .await
            .unwrap_or_else(|e| panic!("{filename} was not moved: {e}"));
        assert_eq!(
            (
                reason.as_str(),
                stage.as_str(),
                source.as_str(),
                error.as_deref()
            ),
            (expected, "ocr", "migration", Some("boom")),
            "{filename}"
        );
    }
    let left: Vec<String> =
        sqlx::query_scalar("SELECT ocr_status FROM documents WHERE user_id = $1")
            .bind(user)
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        left,
        ["completed"],
        "failed documents leave documents; others stay"
    );
    let moved: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM failed_documents")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(moved, 6);
}
