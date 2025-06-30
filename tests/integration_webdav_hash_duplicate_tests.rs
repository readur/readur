use anyhow::Result;
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;
use sha2::{Sha256, Digest};

use readur::{
    AppState,
    db::Database,
    config::Config,
    models::{FileInfo, CreateWebDAVFile, Document},
};

// Helper function to calculate file hash
fn calculate_file_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    format!("{:x}", result)
}

// Helper function to create test file info
fn create_test_file_info(name: &str, path: &str, size: i64) -> FileInfo {
    FileInfo {
        name: name.to_string(),
        path: path.to_string(),
        size,
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
        tags: Vec::new(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        user_id,
        file_hash: Some(file_hash),
        original_created_at: None,
        original_modified_at: None,
        source_metadata: None,
    }
}

// Mock WebDAV service for testing
#[derive(Clone)]
struct MockWebDAVService {
    pub test_files: std::collections::HashMap<String, Vec<u8>>,
}

impl MockWebDAVService {
    fn new() -> Self {
        Self {
            test_files: std::collections::HashMap::new(),
        }
    }

    fn add_test_file(&mut self, path: &str, content: Vec<u8>) {
        self.test_files.insert(path.to_string(), content);
    }

    async fn download_file(&self, path: &str) -> Result<Vec<u8>> {
        self.test_files
            .get(path)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("File not found: {}", path))
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
            .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/readur_test".to_string());
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
async fn test_webdav_sync_duplicate_detection_skips_duplicate() -> Result<()> {
    let state = create_test_app_state().await?;
    let user_id = create_test_user(&state.db, "webdav_test").await?;
    
    // Test content
    let test_content = b"This is test PDF content for duplicate detection";
    let file_hash = calculate_file_hash(test_content);
    
    // Create existing document with same hash
    let existing_doc = create_test_document(user_id, "existing.pdf", file_hash.clone());
    state.db.create_document(existing_doc).await?;
    
    // Setup mock WebDAV service
    let mut webdav_service = MockWebDAVService::new();
    webdav_service.add_test_file("/test/duplicate.pdf", test_content.to_vec());
    
    // Create file info for the duplicate file
    let file_info = create_test_file_info("duplicate.pdf", "/test/duplicate.pdf", test_content.len() as i64);
    
    // Create a mock process_single_file function (since the actual one is private)
    // We'll test the duplicate detection logic directly
    
    // Check if duplicate exists using the new efficient method
    let duplicate_check = state.db.get_document_by_user_and_hash(user_id, &file_hash).await?;
    
    assert!(duplicate_check.is_some(), "Should find existing document with same hash");
    
    let found_doc = duplicate_check.unwrap();
    assert_eq!(found_doc.file_hash, Some(file_hash));
    assert_eq!(found_doc.user_id, user_id);
    
    // Verify that WebDAV tracking would record this as a duplicate
    let webdav_file = CreateWebDAVFile {
        user_id,
        webdav_path: file_info.path.clone(),
        etag: file_info.etag.clone(),
        last_modified: file_info.last_modified,
        file_size: file_info.size,
        mime_type: file_info.mime_type.clone(),
        document_id: Some(found_doc.id),
        sync_status: "duplicate_content".to_string(),
        sync_error: None,
    };
    
    let created_webdav_file = state.db.create_or_update_webdav_file(&webdav_file).await?;
    assert_eq!(created_webdav_file.sync_status, "duplicate_content");
    assert_eq!(created_webdav_file.document_id, Some(found_doc.id));

    Ok(())
}

#[tokio::test]
async fn test_webdav_sync_duplicate_detection_processes_unique() -> Result<()> {
    let state = create_test_app_state().await?;
    let user_id = create_test_user(&state.db, "webdav_test").await?;
    
    // Test content
    let test_content = b"This is unique PDF content that should be processed";
    let file_hash = calculate_file_hash(test_content);
    
    // Verify no existing document with this hash
    let duplicate_check = state.db.get_document_by_user_and_hash(user_id, &file_hash).await?;
    assert!(duplicate_check.is_none(), "Should not find any existing document with this hash");
    
    // This indicates the file would be processed normally
    // In the actual sync, this would proceed to save the file and create a new document
    
    Ok(())
}

#[tokio::test]
async fn test_webdav_sync_duplicate_different_users() -> Result<()> {
    let state = create_test_app_state().await?;
    let user1_id = create_test_user(&state.db, "webdav_user1").await?;
    let user2_id = create_test_user(&state.db, "webdav_user2").await?;
    
    // Test content
    let test_content = b"Shared content between different users";
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
async fn test_webdav_sync_etag_change_detection() -> Result<()> {
    let state = create_test_app_state().await?;
    let user_id = create_test_user(&state.db, "webdav_test").await?;
    
    let webdav_path = "/test/updated.pdf";
    let old_etag = "old-etag-123";
    let new_etag = "new-etag-456";
    
    // Create a document first
    let test_doc = create_test_document(user_id, "updated.pdf", "etag_test_hash_1234567890".to_string());
    let created_doc = state.db.create_document(test_doc).await?;
    
    // Create initial WebDAV file record
    let initial_webdav_file = CreateWebDAVFile {
        user_id,
        webdav_path: webdav_path.to_string(),
        etag: old_etag.to_string(),
        last_modified: Some(Utc::now()),
        file_size: 1024,
        mime_type: "application/pdf".to_string(),
        document_id: Some(created_doc.id),
        sync_status: "synced".to_string(),
        sync_error: None,
    };
    
    state.db.create_or_update_webdav_file(&initial_webdav_file).await?;
    
    // Check existing WebDAV file
    let existing_file = state.db.get_webdav_file_by_path(user_id, webdav_path).await?;
    assert!(existing_file.is_some());
    
    let existing_file = existing_file.unwrap();
    assert_eq!(existing_file.etag, old_etag);
    
    // Simulate file with new ETag (indicating change)
    let file_info = FileInfo {
        name: "updated.pdf".to_string(),
        path: webdav_path.to_string(),
        size: 1024,
        last_modified: Some(Utc::now()),
        etag: new_etag.to_string(),
        mime_type: "application/pdf".to_string(),
        is_directory: false,
        created_at: None,
        permissions: None,
        owner: None,
        group: None,
        metadata: None,
    };
    
    // ETag comparison should detect change
    assert_ne!(existing_file.etag, file_info.etag, "ETag change should be detected");

    Ok(())
}

#[tokio::test]
async fn test_webdav_sync_hash_collision_prevention() -> Result<()> {
    let state = create_test_app_state().await?;
    let user_id = create_test_user(&state.db, "webdav_test").await?;
    
    // Create document with specific hash
    let test_hash = "abcd1234567890123456789012345678901234567890123456789012345678";
    let document = create_test_document(user_id, "original.pdf", test_hash.to_string());
    state.db.create_document(document).await?;
    
    // Try to create another document with same hash (should fail due to unique constraint)
    let duplicate_document = create_test_document(user_id, "duplicate.pdf", test_hash.to_string());
    let result = state.db.create_document(duplicate_document).await;
    
    assert!(result.is_err(), "Should not be able to create duplicate hash for same user");

    Ok(())
}

#[tokio::test]
async fn test_webdav_sync_file_content_vs_metadata_change() -> Result<()> {
    let state = create_test_app_state().await?;
    let user_id = create_test_user(&state.db, "webdav_test").await?;
    
    // Original content and hash
    let original_content = b"Original file content";
    let original_hash = calculate_file_hash(original_content);
    
    // Create original document
    let original_doc = create_test_document(user_id, "test.pdf", original_hash.clone());
    state.db.create_document(original_doc).await?;
    
    // Same content but different metadata (name, etc.) - should still be detected as duplicate
    let duplicate_check = state.db.get_document_by_user_and_hash(user_id, &original_hash).await?;
    assert!(duplicate_check.is_some(), "Same content should be detected as duplicate regardless of filename");
    
    // Different content - should not be duplicate
    let different_content = b"Different file content";
    let different_hash = calculate_file_hash(different_content);
    
    let unique_check = state.db.get_document_by_user_and_hash(user_id, &different_hash).await?;
    assert!(unique_check.is_none(), "Different content should not be detected as duplicate");

    Ok(())
}

#[tokio::test]
async fn test_webdav_sync_error_handling_invalid_hash() -> Result<()> {
    let state = create_test_app_state().await?;
    let user_id = create_test_user(&state.db, "webdav_test").await?;
    
    // Test with invalid hash formats
    let invalid_g_hash = "g".repeat(64);
    let invalid_hashes = vec![
        "", // Empty
        "short", // Too short
        "invalid_characters_!@#$", // Invalid characters
        &invalid_g_hash, // Invalid hex (contains 'g')
    ];
    
    for invalid_hash in invalid_hashes {
        let result = state.db.get_document_by_user_and_hash(user_id, invalid_hash).await;
        // Should handle gracefully - either return None or proper error
        match result {
            Ok(doc) => assert!(doc.is_none(), "Invalid hash should not match any document"),
            Err(_) => {} // Acceptable to return error for invalid input
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_webdav_sync_concurrent_duplicate_detection() -> Result<()> {
    let state = create_test_app_state().await?;
    let user_id = create_test_user(&state.db, "webdav_test").await?;
    
    let test_content = b"Concurrent test content";
    let file_hash = calculate_file_hash(test_content);
    
    // Simulate concurrent duplicate checks
    let mut handles = Vec::new();
    
    for i in 0..5 {
        let state_clone = state.clone();
        let hash_clone = file_hash.clone();
        
        let handle = tokio::spawn(async move {
            state_clone.db.get_document_by_user_and_hash(user_id, &hash_clone).await
        });
        
        handles.push(handle);
    }
    
    // Wait for all concurrent operations
    let mut all_none = true;
    for handle in handles {
        let result = handle.await??;
        if result.is_some() {
            all_none = false;
        }
    }
    
    // Since no document exists with this hash, all should return None
    assert!(all_none, "All concurrent checks should return None for non-existent hash");

    Ok(())
}