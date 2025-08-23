use anyhow::Result;
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;
use sha2::{Sha256, Digest};

use readur::{
    AppState,
    db::Database,
    config::Config,
    models::{Document, CreateUser, UserRole},
    services::file_service::FileService,
    storage::{StorageConfig, factory::create_storage_backend},
};

// Helper function to calculate file hash
fn calculate_file_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    format!("{:x}", result)
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

// Helper function to create test user with unique identifier
fn create_test_user_with_suffix(suffix: &str) -> CreateUser {
    CreateUser {
        username: format!("testuser_{}", suffix),
        email: format!("test_{}@example.com", suffix),
        password: "test_password".to_string(),
        role: Some(UserRole::User),
    }
}

async fn create_test_app_state() -> Result<Arc<AppState>> {
    let config = Config::from_env().unwrap_or_else(|_| {
        // Create a test config if env fails - use DATABASE_URL env var or fallback
        let database_url = std::env::var("DATABASE_URL")
            .or_else(|_| std::env::var("TEST_DATABASE_URL"))
            .unwrap_or_else(|_| "postgresql://readur:readur@localhost:5432/readur".to_string());
        Config {
            database_url,
            server_address: "127.0.0.1:8000".to_string(),
            jwt_secret: "test-secret".to_string(),
            upload_path: "./test-uploads".to_string(),
            watch_folder: "./test-watch".to_string(),
            user_watch_base_dir: "./user_watch".to_string(),
            enable_per_user_watch: false,
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
            s3_enabled: false,
            s3_config: None,
        }
    });
    let db = Database::new(&config.database_url).await?;
    
    // Create file service
    let storage_config = StorageConfig::Local { 
        upload_path: config.upload_path.clone() 
    };
    let storage_backend = create_storage_backend(storage_config)
        .await
        .expect("Failed to create test storage backend");
    let file_service = Arc::new(FileService::with_storage(config.upload_path.clone(), storage_backend));
    
    let queue_service = std::sync::Arc::new(
        readur::ocr::queue::OcrQueueService::new(db.clone(), db.get_pool().clone(), 1, file_service.clone())
    );
    
    Ok(Arc::new(AppState {
        db: db.clone(),
        config,
        file_service,
        webdav_scheduler: None,
        source_scheduler: None,
        queue_service,
        oidc_client: None,
        sync_progress_tracker: std::sync::Arc::new(readur::services::sync_progress_tracker::SyncProgressTracker::new()),
        user_watch_service: None,
        webdav_metrics_collector: None,
    }))
}

#[tokio::test]
async fn test_document_upload_duplicate_detection_returns_existing() -> Result<()> {
    let state = create_test_app_state().await?;
    let user = create_test_user_with_suffix(&format!("upload_{}", uuid::Uuid::new_v4().simple()));
    
    // Create user in database
    let created_user = state.db.create_user(user).await?;
    let user_id = created_user.id;
    
    // Test content
    let test_content = b"This is test PDF content for upload duplicate detection";
    let file_hash = calculate_file_hash(test_content);
    
    // Create existing document with same hash
    let existing_doc = create_test_document(user_id, "existing.pdf", file_hash.clone());
    let created_doc = state.db.create_document(existing_doc).await?;
    
    // Test that the hash lookup would find the existing document
    let duplicate_check = state.db.get_document_by_user_and_hash(user_id, &file_hash).await?;
    assert!(duplicate_check.is_some(), "Should find existing document with same hash");
    
    let found_doc = duplicate_check.unwrap();
    assert_eq!(found_doc.id, created_doc.id);
    assert_eq!(found_doc.file_hash, Some(file_hash));

    Ok(())
}

#[tokio::test]
async fn test_document_upload_unique_content_processed() -> Result<()> {
    let state = create_test_app_state().await?;
    let user = create_test_user_with_suffix(&format!("upload_{}", uuid::Uuid::new_v4().simple()));
    
    // Create user in database
    let created_user = state.db.create_user(user).await?;
    let user_id = created_user.id;
    
    // Test content
    let test_content = b"This is unique PDF content for upload processing";
    let file_hash = calculate_file_hash(test_content);
    
    // Verify no existing document with this hash
    let duplicate_check = state.db.get_document_by_user_and_hash(user_id, &file_hash).await?;
    assert!(duplicate_check.is_none(), "Should not find any existing document with this hash");

    Ok(())
}

#[tokio::test]
async fn test_document_upload_different_users_same_content() -> Result<()> {
    let state = create_test_app_state().await?;
    
    // Create two users
    let user1 = create_test_user_with_suffix(&format!("different_users_1_{}", Uuid::new_v4().simple()));
    let created_user1 = state.db.create_user(user1).await?;
    let user1_id = created_user1.id;
    
    let user2 = create_test_user_with_suffix(&format!("different_users_2_{}", Uuid::new_v4().simple()));
    let created_user2 = state.db.create_user(user2).await?;
    let user2_id = created_user2.id;
    
    // Test content
    let test_content = b"Shared content between different users for upload";
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
async fn test_document_upload_hash_calculation_accuracy() -> Result<()> {
    // Test various file contents and ensure hash calculation is accurate
    let test_cases = vec![
        (b"" as &[u8], "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"), // Empty
        (b"a", "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb"), // Single char
        (b"Hello, World!", "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f"), // Text
    ];
    
    for (content, expected_hash) in test_cases {
        let calculated_hash = calculate_file_hash(content);
        assert_eq!(calculated_hash, expected_hash, "Hash mismatch for content: {:?}", content);
    }

    Ok(())
}

#[tokio::test]
async fn test_document_upload_large_file_hash() -> Result<()> {
    // Test hash calculation for larger files
    let large_content = vec![b'X'; 1_000_000]; // 1MB of 'X' characters
    
    let hash1 = calculate_file_hash(&large_content);
    let hash2 = calculate_file_hash(&large_content);
    
    // Hash should be consistent
    assert_eq!(hash1, hash2);
    assert_eq!(hash1.len(), 64); // SHA256 hex length
    assert!(hash1.chars().all(|c| c.is_ascii_hexdigit()));

    Ok(())
}

#[tokio::test]
async fn test_document_upload_binary_content_hash() -> Result<()> {
    // Test hash calculation for binary content
    let mut binary_content = Vec::new();
    for i in 0..256 {
        binary_content.push(i as u8);
    }
    
    let hash = calculate_file_hash(&binary_content);
    
    assert_eq!(hash.len(), 64);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    
    // Same binary content should produce same hash
    let hash2 = calculate_file_hash(&binary_content);
    assert_eq!(hash, hash2);

    Ok(())
}

#[tokio::test]
async fn test_document_upload_duplicate_prevention_database_constraint() -> Result<()> {
    let state = create_test_app_state().await?;
    let user = create_test_user_with_suffix(&format!("upload_{}", uuid::Uuid::new_v4().simple()));
    // Create user in database and get the created user
    let created_user = state.db.create_user(user).await?;
    let user_id = created_user.id;
    
    let test_hash = "duplicate_upload_test_hash_123456789012345678901234567890123456";
    
    // Create first document with the hash
    let doc1 = create_test_document(user_id, "test1.pdf", test_hash.to_string());
    let result1 = state.db.create_document(doc1).await;
    assert!(result1.is_ok(), "First document should be created successfully");
    
    // Try to create second document with same hash for same user
    let doc2 = create_test_document(user_id, "test2.pdf", test_hash.to_string());
    let result2 = state.db.create_document(doc2).await;
    
    // This should fail due to unique constraint
    assert!(result2.is_err(), "Second document with same hash should fail");

    Ok(())
}

#[tokio::test]
async fn test_document_upload_filename_vs_content_duplicate() -> Result<()> {
    let state = create_test_app_state().await?;
    let user = create_test_user_with_suffix(&format!("upload_{}", uuid::Uuid::new_v4().simple()));
    // Create user in database and get the created user
    let created_user = state.db.create_user(user).await?;
    let user_id = created_user.id;
    
    // Same content, different filenames
    let content = b"Same content, different names";
    let hash = calculate_file_hash(content);
    
    // Create first document
    let doc1 = create_test_document(user_id, "document_v1.pdf", hash.clone());
    state.db.create_document(doc1).await?;
    
    // Check that same content is detected as duplicate regardless of filename
    let duplicate_check = state.db.get_document_by_user_and_hash(user_id, &hash).await?;
    assert!(duplicate_check.is_some(), "Same content should be detected as duplicate regardless of filename");

    Ok(())
}

#[tokio::test]
async fn test_document_upload_unicode_content_hash() -> Result<()> {
    // Test hash calculation with unicode content
    let unicode_content = "Hello 世界 🌍 café naïve résumé".as_bytes();
    
    let hash1 = calculate_file_hash(unicode_content);
    let hash2 = calculate_file_hash(unicode_content);
    
    // Hash should be consistent for unicode content
    assert_eq!(hash1, hash2);
    assert_eq!(hash1.len(), 64);
    assert!(hash1.chars().all(|c| c.is_ascii_hexdigit()));

    Ok(())
}

#[tokio::test]
async fn test_document_upload_concurrent_same_content() -> Result<()> {
    let state = create_test_app_state().await?;
    let user = create_test_user_with_suffix(&format!("upload_{}", uuid::Uuid::new_v4().simple()));
    // Create user in database and get the created user
    let created_user = state.db.create_user(user).await?;
    let user_id = created_user.id;
    
    let test_content = b"Concurrent upload test content";
    let file_hash = calculate_file_hash(test_content);
    
    // Simulate concurrent uploads of same content
    let mut handles = Vec::new();
    
    for i in 0..5 {
        let state_clone = state.clone();
        let hash_clone = file_hash.clone();
        
        let handle = tokio::spawn(async move {
            let doc = create_test_document(user_id, &format!("concurrent{}.pdf", i), hash_clone);
            state_clone.db.create_document(doc).await
        });
        
        handles.push(handle);
    }
    
    // Wait for all operations and count results
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

#[tokio::test]
async fn test_document_upload_mime_type_independence() -> Result<()> {
    let state = create_test_app_state().await?;
    let user = create_test_user_with_suffix(&format!("upload_{}", uuid::Uuid::new_v4().simple()));
    // Create user in database and get the created user
    let created_user = state.db.create_user(user).await?;
    let user_id = created_user.id;
    
    let content = b"Same content, different perceived types";
    let hash = calculate_file_hash(content);
    
    // Create document as PDF
    let mut pdf_doc = create_test_document(user_id, "test.pdf", hash.clone());
    pdf_doc.mime_type = "application/pdf".to_string();
    state.db.create_document(pdf_doc).await?;
    
    // Try to upload same content as text file - should be detected as duplicate
    let duplicate_check = state.db.get_document_by_user_and_hash(user_id, &hash).await?;
    assert!(duplicate_check.is_some(), "Same content should be detected as duplicate regardless of MIME type");

    Ok(())
}

#[tokio::test]
async fn test_document_upload_performance_hash_lookup() -> Result<()> {
    let state = create_test_app_state().await?;
    let user = create_test_user_with_suffix(&format!("upload_{}", uuid::Uuid::new_v4().simple()));
    // Create user in database and get the created user
    let created_user = state.db.create_user(user).await?;
    let user_id = created_user.id;
    
    // Create multiple documents with different hashes
    let mut test_hashes = Vec::new();
    
    for i in 0..50 {
        let content = format!("Performance test content {}", i);
        let hash = calculate_file_hash(content.as_bytes());
        test_hashes.push(hash.clone());
        
        let doc = create_test_document(user_id, &format!("perf_test_{}.pdf", i), hash);
        state.db.create_document(doc).await?;
    }
    
    // Measure hash lookup performance
    let start = std::time::Instant::now();
    
    for hash in &test_hashes {
        let result = state.db.get_document_by_user_and_hash(user_id, hash).await?;
        assert!(result.is_some(), "Should find document with hash: {}", hash);
    }
    
    let duration = start.elapsed();
    
    // Hash lookups should be very fast
    assert!(duration.as_millis() < 2000, "Hash lookups should be fast even with many documents: {:?}", duration);

    Ok(())
}