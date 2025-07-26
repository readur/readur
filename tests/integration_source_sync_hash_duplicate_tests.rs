use anyhow::Result;
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;
use sha2::{Sha256, Digest};

use readur::{
    AppState,
    db::Database,
    config::Config,
    models::{FileIngestionInfo, Document, Source, SourceType, SourceStatus},
};

// Helper function to calculate file hash
fn calculate_file_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    format!("{:x}", result)
}

// Helper function to create test file info
fn create_test_file_info(name: &str, path: &str, content: &[u8]) -> FileIngestionInfo {
    FileIngestionInfo {
        name: name.to_string(),
        relative_path: path.to_string(),
        full_path: path.to_string(),
        #[allow(deprecated)]
        path: path.to_string(),
        size: content.len() as i64,
        last_modified: Some(Utc::now()),
        etag: "test-etag".to_string(),
        mime_type: "application/pdf".to_string(),
        is_directory: false,
        created_at: None,
        permissions: None,
        owner: None,
        group: None,
        metadata: None,
    }
}

// Helper function to create test document
fn create_test_document(user_id: Uuid, filename: &str, file_hash: String) -> Document {
    Document {
        id: Uuid::new_v4(),
        filename: filename.to_string(),
        original_filename: filename.to_string(),
        file_path: format!("/tmp/{}", filename),
        file_size: 1024,
        mime_type: "application/pdf".to_string(),
        content: None,
        ocr_text: None,
        ocr_confidence: None,
        ocr_word_count: None,
        ocr_processing_time_ms: None,
        ocr_status: Some("pending".to_string()),
        ocr_error: None,
        ocr_completed_at: None,
        ocr_retry_count: None,
        ocr_failure_reason: None,
        tags: Vec::new(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        user_id,
        file_hash: Some(file_hash),
        original_created_at: None,
        original_modified_at: None,
        source_path: None,
        source_type: None,
        source_id: None,
        file_permissions: None,
        file_owner: None,
        file_group: None,
        source_metadata: None,
    }
}

// Helper function to create test source
fn create_test_source(user_id: Uuid, source_type: SourceType) -> Source {
    Source {
        id: Uuid::new_v4(),
        user_id,
        name: "Test Source".to_string(),
        source_type,
        config: serde_json::json!({}),
        status: SourceStatus::Idle,
        enabled: true,
        last_sync_at: None,
        last_error: None,
        last_error_at: None,
        total_files_synced: 0,
        total_files_pending: 0,
        total_size_bytes: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        validation_status: None,
        last_validation_at: None,
        validation_score: None,
        validation_issues: None,
    }
}

// Helper function to create a test user with unique identifier
async fn create_test_user(db: &Database, username: &str) -> Result<Uuid> {
    use readur::models::{CreateUser, UserRole};
    let unique_suffix = Uuid::new_v4().simple();
    let user = CreateUser {
        username: format!("{}_{}", username, unique_suffix),
        email: format!("{}_{}@example.com", username, unique_suffix),
        password: "password123".to_string(),
        role: Some(UserRole::User),
    };
    let created_user = db.create_user(user).await?;
    Ok(created_user.id)
}

async fn create_test_app_state() -> Result<Arc<AppState>> {
    let config = Config::from_env().unwrap_or_else(|_| {
        let database_url = std::env::var("DATABASE_URL")
            .or_else(|_| std::env::var("TEST_DATABASE_URL"))
            .unwrap_or_else(|_| "postgresql://readur:readur@localhost:5432/readur".to_string());
        Config {
            database_url,
            server_address: "127.0.0.1:8000".to_string(),
            jwt_secret: "test-secret".to_string(),
            upload_path: "./test-uploads".to_string(),
            watch_folder: "./test-watch".to_string(),
            allowed_file_types: vec!["pdf".to_string(), "txt".to_string()],
            watch_interval_seconds: Some(30),
            file_stability_check_ms: Some(500),
            max_file_age_hours: None,
            ocr_language: "eng".to_string(),
            concurrent_ocr_jobs: 2,
            ocr_timeout_seconds: 60,
            max_file_size_mb: 10,
            memory_limit_mb: 256,
            cpu_priority: "normal".to_string(),
            oidc_enabled: false,
            oidc_client_id: None,
            oidc_client_secret: None,
            oidc_issuer_url: None,
            oidc_redirect_uri: None,
        }
    });
    let db = Database::new(&config.database_url).await?;
    let queue_service = std::sync::Arc::new(
        readur::ocr::queue::OcrQueueService::new(db.clone(), db.get_pool().clone(), 1)
    );
    
    Ok(Arc::new(AppState {
        db,
        config,
        webdav_scheduler: None,
        source_scheduler: None,
        queue_service,
        oidc_client: None,
    }))
}

#[tokio::test]
async fn test_source_sync_duplicate_detection_skips_duplicate() -> Result<()> {
    let state = create_test_app_state().await?;
    let user_id = create_test_user(&state.db, "source_sync_test").await?;
    
    // Test content
    let test_content = b"This is test content for source sync duplicate detection";
    let file_hash = calculate_file_hash(test_content);
    
    // Create existing document with same hash
    let existing_doc = create_test_document(user_id, "existing.pdf", file_hash.clone());
    state.db.create_document(existing_doc).await?;
    
    // Check if duplicate exists using the efficient method
    let duplicate_check = state.db.get_document_by_user_and_hash(user_id, &file_hash).await?;
    
    assert!(duplicate_check.is_some(), "Should find existing document with same hash");
    
    let found_doc = duplicate_check.unwrap();
    assert_eq!(found_doc.file_hash, Some(file_hash));
    assert_eq!(found_doc.user_id, user_id);

    Ok(())
}

#[tokio::test]
async fn test_source_sync_duplicate_detection_processes_unique() -> Result<()> {
    let state = create_test_app_state().await?;
    let user_id = create_test_user(&state.db, "source_sync_test").await?;
    
    // Test content
    let test_content = b"This is unique content that should be processed by source sync";
    let file_hash = calculate_file_hash(test_content);
    
    // Verify no existing document with this hash
    let duplicate_check = state.db.get_document_by_user_and_hash(user_id, &file_hash).await?;
    assert!(duplicate_check.is_none(), "Should not find any existing document with this hash");
    
    // This indicates the file would be processed normally
    Ok(())
}

#[tokio::test]
async fn test_source_sync_duplicate_different_users() -> Result<()> {
    let state = create_test_app_state().await?;
    let user1_id = create_test_user(&state.db, "source_sync_user1").await?;
    let user2_id = create_test_user(&state.db, "source_sync_user2").await?;
    
    // Test content
    let test_content = b"Shared content between different users in source sync";
    let file_hash = calculate_file_hash(test_content);
    
    // Create document for user1 with this hash
    let user1_doc = create_test_document(user1_id, "user1.pdf", file_hash.clone());
    state.db.create_document(user1_doc).await?;
    
    // Check that user2 doesn't see user1's document as duplicate
    let duplicate_check = state.db.get_document_by_user_and_hash(user2_id, &file_hash).await?;
    assert!(duplicate_check.is_none(), "User2 should not see user1's document as duplicate");
    
    // User2 should be able to create their own document with same hash
    let user2_doc = create_test_document(user2_id, "user2.pdf", file_hash.clone());
    let result = state.db.create_document(user2_doc).await;
    assert!(result.is_ok(), "User2 should be able to create document with same hash");

    Ok(())
}

#[tokio::test]
async fn test_source_sync_hash_calculation_consistency() -> Result<()> {
    let test_content = b"Test content for hash consistency in source sync";
    
    // Calculate hash multiple times
    let hash1 = calculate_file_hash(test_content);
    let hash2 = calculate_file_hash(test_content);
    let hash3 = calculate_file_hash(test_content);
    
    // All hashes should be identical
    assert_eq!(hash1, hash2);
    assert_eq!(hash2, hash3);
    
    // Hash should be 64 characters (SHA256 hex)
    assert_eq!(hash1.len(), 64);
    
    // Should be valid hex
    assert!(hash1.chars().all(|c| c.is_ascii_hexdigit()));

    Ok(())
}

#[tokio::test]
async fn test_source_sync_duplicate_detection_performance() -> Result<()> {
    let state = create_test_app_state().await?;
    let user_id = create_test_user(&state.db, "source_sync_test").await?;
    
    // Create multiple documents with different hashes
    let mut created_hashes = Vec::new();
    
    for i in 0..10 {
        let content = format!("Test content number {}", i);
        let hash = calculate_file_hash(content.as_bytes());
        created_hashes.push(hash.clone());
        
        let doc = create_test_document(user_id, &format!("test{}.pdf", i), hash);
        state.db.create_document(doc).await?;
    }
    
    // Test lookup performance - should be fast even with multiple documents
    let start = std::time::Instant::now();
    
    for hash in &created_hashes {
        let result = state.db.get_document_by_user_and_hash(user_id, hash).await?;
        assert!(result.is_some(), "Should find document with hash: {}", hash);
    }
    
    let duration = start.elapsed();
    assert!(duration.as_millis() < 1000, "Hash lookups should be fast: {:?}", duration);

    Ok(())
}

#[tokio::test]
async fn test_source_sync_file_modification_detection() -> Result<()> {
    let state = create_test_app_state().await?;
    let user_id = create_test_user(&state.db, "source_sync_test").await?;
    
    // Original content
    let original_content = b"Original file content";
    let original_hash = calculate_file_hash(original_content);
    
    // Modified content (same file, different content)
    let modified_content = b"Modified file content";
    let modified_hash = calculate_file_hash(modified_content);
    
    // Create document with original content
    let original_doc = create_test_document(user_id, "test.pdf", original_hash.clone());
    state.db.create_document(original_doc).await?;
    
    // Check original content is found
    let original_check = state.db.get_document_by_user_and_hash(user_id, &original_hash).await?;
    assert!(original_check.is_some(), "Should find document with original hash");
    
    // Check modified content is not found (different hash)
    let modified_check = state.db.get_document_by_user_and_hash(user_id, &modified_hash).await?;
    assert!(modified_check.is_none(), "Should not find document with modified hash");
    
    // Verify hashes are actually different
    assert_ne!(original_hash, modified_hash, "Original and modified content should have different hashes");

    Ok(())
}

#[tokio::test]
async fn test_source_sync_edge_case_empty_files() -> Result<()> {
    let state = create_test_app_state().await?;
    let user_id = create_test_user(&state.db, "source_sync_test").await?;
    
    // Empty file content
    let empty_content = b"";
    let empty_hash = calculate_file_hash(empty_content);
    
    // Create document with empty content
    let empty_doc = create_test_document(user_id, "empty.pdf", empty_hash.clone());
    state.db.create_document(empty_doc).await?;
    
    // Check empty file is found
    let empty_check = state.db.get_document_by_user_and_hash(user_id, &empty_hash).await?;
    assert!(empty_check.is_some(), "Should find document with empty content hash");
    
    // Verify empty hash is the known SHA256 empty string hash
    assert_eq!(empty_hash, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");

    Ok(())
}

#[tokio::test]
async fn test_source_sync_large_file_hash_consistency() -> Result<()> {
    // Simulate large file content
    let large_content = vec![b'A'; 10_000_000]; // 10MB of 'A' characters
    
    // Calculate hash
    let hash = calculate_file_hash(&large_content);
    
    // Hash should still be 64 characters
    assert_eq!(hash.len(), 64);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    
    // Calculate same hash again to ensure consistency
    let hash2 = calculate_file_hash(&large_content);
    assert_eq!(hash, hash2);

    Ok(())
}

#[tokio::test]
async fn test_source_sync_binary_file_handling() -> Result<()> {
    let state = create_test_app_state().await?;
    let user_id = create_test_user(&state.db, "source_sync_test").await?;
    
    // Binary content (PDF header + some binary data)
    let mut binary_content = b"%PDF-1.4\n".to_vec();
    binary_content.extend_from_slice(&[0u8, 1u8, 2u8, 3u8, 255u8, 254u8, 253u8]);
    
    let binary_hash = calculate_file_hash(&binary_content);
    
    // Create document with binary content
    let binary_doc = create_test_document(user_id, "binary.pdf", binary_hash.clone());
    state.db.create_document(binary_doc).await?;
    
    // Check binary file is found
    let binary_check = state.db.get_document_by_user_and_hash(user_id, &binary_hash).await?;
    assert!(binary_check.is_some(), "Should find document with binary content hash");

    Ok(())
}

#[tokio::test]
async fn test_source_sync_unicode_filename_handling() -> Result<()> {
    let state = create_test_app_state().await?;
    let user_id = create_test_user(&state.db, "source_sync_test").await?;
    
    // Unicode content and filename
    let unicode_content = "Test content with unicode: 测试内容 🚀 café".as_bytes();
    let unicode_hash = calculate_file_hash(unicode_content);
    
    // Create document with unicode filename
    let unicode_doc = create_test_document(user_id, "测试文档🚀.pdf", unicode_hash.clone());
    state.db.create_document(unicode_doc).await?;
    
    // Check unicode file is found
    let unicode_check = state.db.get_document_by_user_and_hash(user_id, &unicode_hash).await?;
    assert!(unicode_check.is_some(), "Should find document with unicode content hash");
    
    let found_doc = unicode_check.unwrap();
    assert_eq!(found_doc.filename, "测试文档🚀.pdf");

    Ok(())
}

#[tokio::test]
async fn test_source_sync_concurrent_hash_operations() -> Result<()> {
    let state = create_test_app_state().await?;
    let user_id = create_test_user(&state.db, "source_sync_test").await?;
    
    // Create multiple concurrent hash lookup operations
    let mut handles = Vec::new();
    
    for i in 0..20 {
        let state_clone = state.clone();
        let hash = format!("{}test_hash_concurrent_{}", "a".repeat(40), i);
        
        let handle = tokio::spawn(async move {
            state_clone.db.get_document_by_user_and_hash(user_id, &hash).await
        });
        
        handles.push(handle);
    }
    
    // Wait for all concurrent operations
    let mut results = Vec::new();
    for handle in handles {
        let result = handle.await??;
        results.push(result);
    }
    
    // All should return None (no documents exist with these hashes)
    for (i, result) in results.iter().enumerate() {
        assert!(result.is_none(), "Concurrent operation {} should return None", i);
    }

    Ok(())
}

#[tokio::test]
async fn test_source_sync_duplicate_prevention_race_condition() -> Result<()> {
    let state = create_test_app_state().await?;
    let user_id = create_test_user(&state.db, "source_sync_test").await?;
    
    let test_hash = "race_condition_test_hash_123456789012345678901234567890123456";
    
    // Try to create multiple documents with same hash concurrently
    let mut handles = Vec::new();
    
    for i in 0..5 {
        let state_clone = state.clone();
        let hash_clone = test_hash.to_string();
        
        let handle = tokio::spawn(async move {
            let doc = create_test_document(user_id, &format!("test{}.pdf", i), hash_clone);
            state_clone.db.create_document(doc).await
        });
        
        handles.push(handle);
    }
    
    // Wait for all operations and count successes
    let mut success_count = 0;
    let mut error_count = 0;
    
    for handle in handles {
        match handle.await? {
            Ok(_) => success_count += 1,
            Err(_) => error_count += 1,
        }
    }
    
    // Only one should succeed due to unique constraint
    assert_eq!(success_count, 1, "Only one document should be created successfully");
    assert_eq!(error_count, 4, "Four operations should fail due to duplicate hash");

    Ok(())
}