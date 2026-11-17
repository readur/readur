//! Ingestion keeps the storage directory consistent with the documents table:
//! duplicate content never gets a second row or file under any policy, and
//! the recorded size/hash follow the documented semantics (both of the source
//! bytes, so a sync can compare them with a listing).

use anyhow::Result;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use uuid::Uuid;

use readur::{
    ingestion::document_ingestion::{
        DeduplicationPolicy, DocumentIngestionRequest, DocumentIngestionService, IngestionResult,
    },
    models::{CreateUser, UpdateSettings, UserRole},
    test_utils::{TestConfigBuilder, TestContext},
};

async fn context_with_temp_storage() -> (TestContext, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("temp upload dir");
    let config = TestConfigBuilder::default().with_upload_path(dir.path().to_str().unwrap());
    let ctx = TestContext::with_config(config).await;
    (ctx, dir)
}

async fn new_user(ctx: &TestContext) -> Result<Uuid> {
    let suffix = Uuid::new_v4().simple();
    let user = ctx
        .state()
        .db
        .create_user(CreateUser {
            username: format!("ingest_cleanup_{suffix}"),
            email: format!("ingest_cleanup_{suffix}@example.com"),
            password: "test_password".to_string(),
            role: Some(UserRole::User),
        })
        .await?;
    Ok(user.id)
}

fn request(
    user_id: Uuid,
    name: &str,
    mime: &str,
    data: Vec<u8>,
    policy: DeduplicationPolicy,
) -> DocumentIngestionRequest {
    DocumentIngestionRequest {
        filename: name.to_string(),
        original_filename: name.to_string(),
        file_data: data,
        mime_type: mime.to_string(),
        user_id,
        deduplication_policy: policy,
        source_type: Some("test".to_string()),
        source_id: None,
        original_created_at: None,
        original_modified_at: None,
        source_path: None,
        file_permissions: None,
        file_owner: None,
        file_group: None,
        source_metadata: None,
    }
}

fn stored_document_files(upload_dir: &std::path::Path) -> Vec<PathBuf> {
    let documents = upload_dir.join("documents");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&documents)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.is_file())
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

fn sha256_hex(data: &[u8]) -> String {
    readur::utils::hash::to_hex(&Sha256::digest(data))
}

/// Every deduplication policy ends with exactly one stored document (row and
/// file) per user and content: the per-user hash index makes a second copy
/// impossible, which is why there is no "store a duplicate" policy. A
/// duplicate found up front is reported per policy before any file is
/// written. (A duplicate that only shows up as a unique violation on insert --
/// the concurrent-upload race, whose stored file must then be removed -- is
/// covered in `integration_unique_violation_classification_tests`.)
#[tokio::test]
async fn duplicate_content_is_never_stored_twice_under_any_policy() -> Result<()> {
    let (ctx, dir) = context_with_temp_storage().await;
    let state = ctx.state();
    let service = DocumentIngestionService::new(state.db.clone(), (*state.file_service).clone());
    let data = b"identical content".to_vec();

    for policy in [
        DeduplicationPolicy::Skip,
        DeduplicationPolicy::ReturnExisting,
        DeduplicationPolicy::TrackAsDuplicate,
    ] {
        let user_id = new_user(&ctx).await?;
        let files_before = stored_document_files(dir.path()).len();
        let created = match service
            .ingest_document(request(
                user_id,
                "first.txt",
                "text/plain",
                data.clone(),
                policy.clone(),
            ))
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?
        {
            IngestionResult::Created(doc) => doc,
            other => panic!("{policy:?}: expected Created, got {other:?}"),
        };
        let after_first = stored_document_files(dir.path());
        assert_eq!(
            after_first.len(),
            files_before + 1,
            "{policy:?}: one stored file per new document"
        );

        let second = service
            .ingest_document(request(
                user_id,
                "second.txt",
                "text/plain",
                data.clone(),
                policy.clone(),
            ))
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        match (&policy, &second) {
            (
                DeduplicationPolicy::Skip,
                IngestionResult::Skipped {
                    existing_document_id,
                    reason,
                },
            ) => {
                assert_eq!(*existing_document_id, created.id);
                assert_eq!(reason, "Content already exists as 'first.txt'");
            }
            (DeduplicationPolicy::ReturnExisting, IngestionResult::ExistingDocument(doc)) => {
                assert_eq!(doc.id, created.id)
            }
            (
                DeduplicationPolicy::TrackAsDuplicate,
                IngestionResult::TrackedAsDuplicate {
                    existing_document_id,
                },
            ) => {
                assert_eq!(*existing_document_id, created.id)
            }
            _ => panic!("{policy:?}: unexpected outcome {second:?}"),
        }

        let (rows,): (i64,) = sqlx::query_as("SELECT count(*) FROM documents WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&state.db.pool)
            .await?;
        assert_eq!(
            rows, 1,
            "{policy:?}: the duplicate did not create a second row"
        );
        assert_eq!(
            stored_document_files(dir.path()),
            after_first,
            "{policy:?}: nor a second file"
        );
    }

    Ok(())
}

/// `file_size` and `file_hash` both describe the bytes as received from the
/// source, even when auto-rotation re-encodes the stored image: the
/// unchanged-file check of source sync compares the listing's size with
/// `file_size`, so a rotated image must not look changed on every run.
#[tokio::test]
async fn file_size_and_hash_are_of_the_source_bytes() -> Result<()> {
    let (ctx, dir) = context_with_temp_storage().await;
    let state = ctx.state();
    let user_id = new_user(&ctx).await?;

    let settings: UpdateSettings =
        serde_json::from_value(serde_json::json!({ "auto_rotate_images": true }))?;
    state
        .db
        .create_or_update_settings(user_id, &settings)
        .await?;

    let source =
        std::fs::read("test_files/exif_orientation_6_rotate_90_cw.jpg").expect("read test image");
    let service = DocumentIngestionService::new(state.db.clone(), (*state.file_service).clone());

    let doc = match service
        .ingest_document(request(
            user_id,
            "rotated.jpg",
            "image/jpeg",
            source.clone(),
            DeduplicationPolicy::ReturnExisting,
        ))
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?
    {
        IngestionResult::Created(doc) => doc,
        other => panic!("expected Created, got {other:?}"),
    };

    let stored_path = dir.path().join("documents").join(format!("{}.jpg", doc.id));
    let stored = std::fs::read(&stored_path)?;
    #[cfg(feature = "ocr")]
    assert_ne!(stored, source, "auto-rotation re-encodes the image");

    assert_eq!(
        doc.file_size,
        source.len() as i64,
        "file_size is the size of the source bytes"
    );
    #[cfg(feature = "ocr")]
    assert_ne!(
        doc.file_size,
        stored.len() as i64,
        "the stored (rotated) size is not what a listing reports"
    );
    assert_eq!(
        doc.file_hash.as_deref(),
        Some(sha256_hex(&source).as_str()),
        "file_hash is the hash of the source bytes"
    );

    Ok(())
}
