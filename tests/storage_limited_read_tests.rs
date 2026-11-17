//! Size-bounded reads (`retrieve_file_limited` / `read_file_limited`) stop
//! one byte past the limit on every path, so a size limit bounds memory even
//! when the stored file is larger than the size recorded for it.

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use readur::services::file_service::FileService;
use readur::services::s3_service::prefix_range;
use readur::storage::local::LocalStorageBackend;
use readur::storage::{LimitedRead, StorageBackend};
use std::fs;
use std::sync::Arc;
use uuid::Uuid;

fn backend() -> (LocalStorageBackend, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("documents")).unwrap();
    (
        LocalStorageBackend::new(dir.path().to_string_lossy().to_string()),
        dir,
    )
}

/// Write `len` bytes to `documents/<name>` and return its full path.
fn write_document(dir: &tempfile::TempDir, name: &str, len: usize) -> String {
    let path = dir.path().join("documents").join(name);
    fs::write(&path, vec![7u8; len]).unwrap();
    path.to_string_lossy().to_string()
}

#[tokio::test]
async fn local_read_within_the_limit_returns_the_whole_file() {
    let (backend, dir) = backend();
    let path = write_document(&dir, "small.txt", 10);
    assert_eq!(
        backend.retrieve_file_limited(&path, 16).await.unwrap(),
        LimitedRead::Within(vec![7u8; 10])
    );
}

#[tokio::test]
async fn local_read_of_exactly_the_limit_is_within() {
    let (backend, dir) = backend();
    let path = write_document(&dir, "exact.txt", 16);
    assert_eq!(
        backend.retrieve_file_limited(&path, 16).await.unwrap(),
        LimitedRead::Within(vec![7u8; 16])
    );
}

#[tokio::test]
async fn local_read_one_byte_over_the_limit_is_too_large() {
    let (backend, dir) = backend();
    let path = write_document(&dir, "over.txt", 17);
    assert_eq!(
        backend.retrieve_file_limited(&path, 16).await.unwrap(),
        LimitedRead::TooLarge
    );
}

#[tokio::test]
async fn local_read_of_an_empty_file_is_within() {
    let (backend, dir) = backend();
    let path = write_document(&dir, "empty.txt", 0);
    assert_eq!(
        backend.retrieve_file_limited(&path, 16).await.unwrap(),
        LimitedRead::Within(Vec::new())
    );
    assert_eq!(
        backend.retrieve_file_limited(&path, 0).await.unwrap(),
        LimitedRead::Within(Vec::new())
    );
}

#[tokio::test]
async fn local_read_still_enforces_path_validation() {
    let (backend, dir) = backend();
    // Traversal is rejected outright.
    assert!(backend
        .retrieve_file_limited("../../etc/passwd", 16)
        .await
        .is_err());
    // An existing file outside the upload directory is refused.
    let outside = tempfile::NamedTempFile::new().unwrap();
    fs::write(outside.path(), b"secret").unwrap();
    assert!(backend
        .retrieve_file_limited(&outside.path().to_string_lossy(), 16)
        .await
        .is_err());
    drop(dir);
}

/// A storage backend that never has the file, forcing FileService onto its
/// legacy path resolution.
struct EmptyBackend;

#[async_trait]
impl StorageBackend for EmptyBackend {
    async fn store_document(&self, _: Uuid, _: Uuid, _: &str, _: &[u8]) -> Result<String> {
        Err(anyhow!("unused"))
    }
    async fn store_thumbnail(&self, _: Uuid, _: Uuid, _: &[u8]) -> Result<String> {
        Err(anyhow!("unused"))
    }
    async fn store_processed_image(&self, _: Uuid, _: Uuid, _: &[u8]) -> Result<String> {
        Err(anyhow!("unused"))
    }
    async fn retrieve_file(&self, _: &str) -> Result<Vec<u8>> {
        Err(anyhow!("not in storage"))
    }
    async fn retrieve_file_limited(&self, _: &str, _: u64) -> Result<LimitedRead> {
        Err(anyhow!("not in storage"))
    }
    async fn delete_document_files(&self, _: Uuid, _: Uuid, _: &str) -> Result<()> {
        Ok(())
    }
    async fn file_exists(&self, _: &str) -> Result<bool> {
        Ok(false)
    }
    fn storage_type(&self) -> &'static str {
        "empty"
    }
    async fn initialize(&self) -> Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn file_service_legacy_fallback_is_bounded_too() {
    let dir = tempfile::tempdir().unwrap();
    let service = FileService::with_storage(
        dir.path().to_string_lossy().to_string(),
        Arc::new(EmptyBackend),
    );
    let small = dir.path().join("small.bin");
    let large = dir.path().join("large.bin");
    fs::write(&small, vec![1u8; 8]).unwrap();
    fs::write(&large, vec![1u8; 9]).unwrap();

    assert_eq!(
        service
            .read_file_limited(&small.to_string_lossy(), 8)
            .await
            .unwrap(),
        LimitedRead::Within(vec![1u8; 8])
    );
    assert_eq!(
        service
            .read_file_limited(&large.to_string_lossy(), 8)
            .await
            .unwrap(),
        LimitedRead::TooLarge
    );
}

#[tokio::test]
async fn file_service_reads_through_the_storage_backend_bounded() {
    let (backend, dir) = backend();
    let service =
        FileService::with_storage(dir.path().to_string_lossy().to_string(), Arc::new(backend));
    let path = write_document(&dir, "doc.pdf", 33);
    assert_eq!(
        service.read_file_limited(&path, 32).await.unwrap(),
        LimitedRead::TooLarge
    );
    assert_eq!(
        service.read_file_limited(&path, 33).await.unwrap(),
        LimitedRead::Within(vec![7u8; 33])
    );
}

/// The S3 backend asks for `bytes=0-<max>` (inclusive end, so `max + 1`
/// bytes) and classifies what comes back with `LimitedRead::from_prefix`.
#[test]
fn s3_prefix_range_and_classification() {
    assert_eq!(prefix_range(10), "bytes=0-10");
    assert_eq!(prefix_range(0), "bytes=0-0");
    assert_eq!(
        LimitedRead::from_prefix(vec![0; 11], 10),
        LimitedRead::TooLarge
    );
    assert_eq!(
        LimitedRead::from_prefix(vec![0; 10], 10),
        LimitedRead::Within(vec![0; 10])
    );
    // An empty object (a 416 answered with no bytes) is within any limit.
    assert_eq!(
        LimitedRead::from_prefix(Vec::new(), 0),
        LimitedRead::Within(Vec::new())
    );
}
