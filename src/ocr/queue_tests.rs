//! Unit tests for `crate::ocr::queue`, kept in a sibling file so the module
//! itself stays under its file-length baseline.

use super::*;
use sqlx::postgres::PgPoolOptions;

fn queue_with(max_concurrent_jobs: usize) -> OcrQueueService {
    // A lazy pool never connects, so no database is needed.
    let pool = PgPoolOptions::new().connect_lazy("postgres://localhost/unused").unwrap();
    OcrQueueService::new(
        Database { pool: pool.clone() },
        pool,
        max_concurrent_jobs,
        Arc::new(crate::services::file_service::FileService::new("/tmp".to_string())),
        100,
        100,
        300,
    )
}

#[tokio::test]
async fn uses_configured_concurrency() {
    assert_eq!(queue_with(1).max_concurrent_jobs, 1);
    assert_eq!(queue_with(4).max_concurrent_jobs, 4);
}

#[tokio::test]
async fn clamps_concurrency_to_supported_range() {
    assert_eq!(queue_with(0).max_concurrent_jobs, 1);
    assert_eq!(queue_with(50).max_concurrent_jobs, MAX_CONCURRENT_OCR_JOBS);
}
