use anyhow::Result;
use chrono::Utc;
use uuid::Uuid;
use sha2::{Sha256, Digest};
use tempfile::TempDir;

use readur::{
    db::Database,
    services::file_service::FileService,
    models::{Document, CreateUser, UserRole},
};

fn get_test_db_url() -> String {
    std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("TEST_DATABASE_URL"))
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/readur_test".to_string())
}

// Helper function to create a test user with unique identifier
async fn create_test_user(db: &Database, username: &str) -> Result<Uuid> {
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

// Helper function to calculate file hash
fn calculate_file_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    format!("{:x}", result)
}

// Helper function to create a test document
fn create_test_document(user_id: Uuid, filename: &str, file_hash: Option<String>) -> Document {
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
        file_hash,
        original_created_at: None,
        original_modified_at: None,
        source_metadata: None,
    }
}

#[tokio::test]
async fn test_get_document_by_user_and_hash_found() -> Result<()> {
    let db = Database::new(&get_test_db_url()).await?;
    let user_id = create_test_user(&db, "testuser1").await?;
    let file_hash = "abcd1234567890";

    // Create a document with the hash
    let document = create_test_document(user_id, "test.pdf", Some(file_hash.to_string()));
    let created_doc = db.create_document(document).await?;

    // Test finding the document by hash
    let found_doc = db.get_document_by_user_and_hash(user_id, file_hash).await?;

    assert!(found_doc.is_some());
    let found_doc = found_doc.unwrap();
    assert_eq!(found_doc.id, created_doc.id);
    assert_eq!(found_doc.file_hash, Some(file_hash.to_string()));
    assert_eq!(found_doc.user_id, user_id);

    Ok(())
}

#[tokio::test]
async fn test_get_document_by_user_and_hash_not_found() -> Result<()> {
    let db = Database::new(&get_test_db_url()).await?;
    let user_id = Uuid::new_v4();
    let non_existent_hash = "nonexistent1234567890";

    // Test finding a non-existent hash
    let found_doc = db.get_document_by_user_and_hash(user_id, non_existent_hash).await?;

    assert!(found_doc.is_none());

    Ok(())
}

#[tokio::test]
async fn test_get_document_by_user_and_hash_different_user() -> Result<()> {
    let db = Database::new(&get_test_db_url()).await?;
    let user1_id = create_test_user(&db, "testuser2").await?;
    let user2_id = create_test_user(&db, "testuser3").await?;
    let file_hash = "shared_hash_1234567890";

    // Create a document for user1 with the hash
    let document = create_test_document(user1_id, "test.pdf", Some(file_hash.to_string()));
    db.create_document(document).await?;

    // Test that user2 cannot find user1's document by hash
    let found_doc = db.get_document_by_user_and_hash(user2_id, file_hash).await?;

    assert!(found_doc.is_none(), "User should not be able to access another user's documents");

    Ok(())
}

#[tokio::test]
async fn test_duplicate_hash_prevention_same_user() -> Result<()> {
    let db = Database::new(&get_test_db_url()).await?;
    let user_id = create_test_user(&db, "testuser4").await?;
    let file_hash = "duplicate_hash_1234567890";

    // Create first document with the hash
    let document1 = create_test_document(user_id, "test1.pdf", Some(file_hash.to_string()));
    let result1 = db.create_document(document1).await;
    assert!(result1.is_ok(), "First document with hash should be created successfully");

    // Try to create second document with same hash for same user
    let document2 = create_test_document(user_id, "test2.pdf", Some(file_hash.to_string()));
    let result2 = db.create_document(document2).await;
    
    // This should fail due to unique constraint
    assert!(result2.is_err(), "Second document with same hash for same user should fail");

    Ok(())
}

#[tokio::test]
async fn test_same_hash_different_users_allowed() -> Result<()> {
    let db = Database::new(&get_test_db_url()).await?;
    let user1_id = create_test_user(&db, "testuser5").await?;
    let user2_id = create_test_user(&db, "testuser6").await?;
    let file_hash = "shared_content_hash_1234567890";

    // Create document for user1 with the hash
    let document1 = create_test_document(user1_id, "test1.pdf", Some(file_hash.to_string()));
    let result1 = db.create_document(document1).await;
    assert!(result1.is_ok(), "First user's document should be created successfully");

    // Create document for user2 with same hash
    let document2 = create_test_document(user2_id, "test2.pdf", Some(file_hash.to_string()));
    let result2 = db.create_document(document2).await;
    assert!(result2.is_ok(), "Second user's document with same hash should be allowed");

    // Verify both users can find their respective documents
    let found_doc1 = db.get_document_by_user_and_hash(user1_id, file_hash).await?;
    let found_doc2 = db.get_document_by_user_and_hash(user2_id, file_hash).await?;

    assert!(found_doc1.is_some());
    assert!(found_doc2.is_some());
    assert_ne!(found_doc1.unwrap().id, found_doc2.unwrap().id);

    Ok(())
}

#[tokio::test]
async fn test_null_hash_allowed_multiple() -> Result<()> {
    let db = Database::new(&get_test_db_url()).await?;
    let user_id = create_test_user(&db, "testuser7").await?;

    // Create multiple documents with null hash (should be allowed)
    let document1 = create_test_document(user_id, "test1.pdf", None);
    let result1 = db.create_document(document1).await;
    assert!(result1.is_ok(), "First document with null hash should be created");

    let document2 = create_test_document(user_id, "test2.pdf", None);
    let result2 = db.create_document(document2).await;
    assert!(result2.is_ok(), "Second document with null hash should be created");

    Ok(())
}

#[test]
fn test_calculate_file_hash_consistency() {
    let test_data = b"Hello, World! This is test content for hash calculation.";
    
    // Calculate hash multiple times
    let hash1 = calculate_file_hash(test_data);
    let hash2 = calculate_file_hash(test_data);
    let hash3 = calculate_file_hash(test_data);

    // All hashes should be identical
    assert_eq!(hash1, hash2);
    assert_eq!(hash2, hash3);
    
    // Hash should be 64 characters (SHA256 hex)
    assert_eq!(hash1.len(), 64);
    
    // Should be valid hex
    assert!(hash1.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn test_calculate_file_hash_different_content() {
    let data1 = b"Content 1";
    let data2 = b"Content 2";
    let data3 = b"content 1"; // Different case

    let hash1 = calculate_file_hash(data1);
    let hash2 = calculate_file_hash(data2);
    let hash3 = calculate_file_hash(data3);

    // All hashes should be different
    assert_ne!(hash1, hash2);
    assert_ne!(hash1, hash3);
    assert_ne!(hash2, hash3);
}

#[test]
fn test_calculate_file_hash_empty_content() {
    let empty_data = b"";
    let hash = calculate_file_hash(empty_data);
    
    // Should produce a valid hash even for empty content
    assert_eq!(hash.len(), 64);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    
    // Known SHA256 hash of empty string
    assert_eq!(hash, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
}

#[tokio::test]
async fn test_file_service_create_document_with_hash() {
    let temp_dir = TempDir::new().unwrap();
    let upload_path = temp_dir.path().to_string_lossy().to_string();
    let file_service = FileService::new(upload_path);
    let user_id = Uuid::new_v4();
    let test_hash = "test_hash_1234567890";

    let document = file_service.create_document(
        "test.pdf",
        "original.pdf",
        "/path/to/file.pdf",
        1024,
        "application/pdf",
        user_id,
        Some(test_hash.to_string()),
        None, // original_created_at
        None, // original_modified_at
        None, // source_metadata
    );

    assert_eq!(document.filename, "test.pdf");
    assert_eq!(document.original_filename, "original.pdf");
    assert_eq!(document.file_hash, Some(test_hash.to_string()));
    assert_eq!(document.user_id, user_id);
}

#[tokio::test]
async fn test_file_service_create_document_without_hash() {
    let temp_dir = TempDir::new().unwrap();
    let upload_path = temp_dir.path().to_string_lossy().to_string();
    let file_service = FileService::new(upload_path);
    let user_id = Uuid::new_v4();

    let document = file_service.create_document(
        "test.pdf",
        "original.pdf",
        "/path/to/file.pdf",
        1024,
        "application/pdf",
        user_id,
        None,
        None, // original_created_at
        None, // original_modified_at
        None, // source_metadata
    );

    assert_eq!(document.filename, "test.pdf");
    assert_eq!(document.original_filename, "original.pdf");
    assert_eq!(document.file_hash, None);
    assert_eq!(document.user_id, user_id);
}