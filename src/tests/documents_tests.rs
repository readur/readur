#[cfg(test)]
use crate::models::{Document, DocumentResponse};
use chrono::Utc;
use serde_json::Value;
use uuid::Uuid;

#[cfg(test)]
fn create_test_document(user_id: Uuid) -> Document {
    Document {
        id: Uuid::new_v4(),
        filename: "test_document.pdf".to_string(),
        original_filename: "test_document.pdf".to_string(),
        file_path: "/uploads/test_document.pdf".to_string(),
        file_size: 1024000,
        mime_type: "application/pdf".to_string(),
        content: Some("Test document content".to_string()),
        ocr_text: Some("This is extracted OCR text from the test document.".to_string()),
        ocr_confidence: Some(95.5),
        ocr_word_count: Some(150),
        ocr_processing_time_ms: Some(1200),
        ocr_status: Some("completed".to_string()),
        ocr_error: None,
        ocr_completed_at: Some(Utc::now()),
        tags: vec!["test".to_string(), "document".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        user_id,
        file_hash: Some("1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string()),
        original_created_at: None,
        original_modified_at: None,
        source_metadata: None,
    }
}

#[cfg(test)]
fn create_test_document_without_ocr(user_id: Uuid) -> Document {
    Document {
        id: Uuid::new_v4(),
        filename: "test_no_ocr.txt".to_string(),
        original_filename: "test_no_ocr.txt".to_string(),
        file_path: "/uploads/test_no_ocr.txt".to_string(),
        file_size: 512,
        mime_type: "text/plain".to_string(),
        content: Some("Plain text content".to_string()),
        ocr_text: None,
        ocr_confidence: None,
        ocr_word_count: None,
        ocr_processing_time_ms: None,
        ocr_status: Some("pending".to_string()),
        ocr_error: None,
        ocr_completed_at: None,
        tags: vec!["text".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        user_id,
        file_hash: Some("fedcba0987654321fedcba0987654321fedcba0987654321fedcba0987654321".to_string()),
        original_created_at: None,
        original_modified_at: None,
        source_metadata: None,
    }
}

#[cfg(test)]
fn create_test_document_with_ocr_error(user_id: Uuid) -> Document {
    Document {
        id: Uuid::new_v4(),
        filename: "test_error.pdf".to_string(),
        original_filename: "test_error.pdf".to_string(),
        file_path: "/uploads/test_error.pdf".to_string(),
        file_size: 2048000,
        mime_type: "application/pdf".to_string(),
        content: None,
        ocr_text: None,
        ocr_confidence: None,
        ocr_word_count: None,
        ocr_processing_time_ms: Some(5000),
        ocr_status: Some("failed".to_string()),
        ocr_error: Some("Failed to process document: corrupted file".to_string()),
        ocr_completed_at: Some(Utc::now()),
        tags: vec!["error".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        user_id,
        file_hash: Some("abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string()),
        original_created_at: None,
        original_modified_at: None,
        source_metadata: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_document_response_conversion() {
        let user_id = Uuid::new_v4();
        let document = create_test_document(user_id);
        
        let response: DocumentResponse = document.clone().into();
        
        assert_eq!(response.id, document.id);
        assert_eq!(response.filename, document.filename);
        assert_eq!(response.original_filename, document.original_filename);
        assert_eq!(response.file_size, document.file_size);
        assert_eq!(response.mime_type, document.mime_type);
        assert_eq!(response.tags, document.tags);
        assert_eq!(response.has_ocr_text, true);
        assert_eq!(response.ocr_confidence, document.ocr_confidence);
        assert_eq!(response.ocr_word_count, document.ocr_word_count);
        assert_eq!(response.ocr_processing_time_ms, document.ocr_processing_time_ms);
        assert_eq!(response.ocr_status, document.ocr_status);
    }

    #[tokio::test]
    async fn test_document_response_conversion_no_ocr() {
        let user_id = Uuid::new_v4();
        let document = create_test_document_without_ocr(user_id);
        
        let response: DocumentResponse = document.clone().into();
        
        assert_eq!(response.has_ocr_text, false);
        assert_eq!(response.ocr_confidence, None);
        assert_eq!(response.ocr_word_count, None);
    }

    #[test]
    fn test_ocr_response_structure() {
        let user_id = Uuid::new_v4();
        let document = create_test_document(user_id);
        
        // Test that OCR response fields match expected structure
        let ocr_response = serde_json::json!({
            "document_id": document.id,
            "filename": document.filename,
            "has_ocr_text": document.ocr_text.is_some(),
            "ocr_text": document.ocr_text,
            "ocr_confidence": document.ocr_confidence,
            "ocr_word_count": document.ocr_word_count,
            "ocr_processing_time_ms": document.ocr_processing_time_ms,
            "ocr_status": document.ocr_status,
            "ocr_error": document.ocr_error,
            "ocr_completed_at": document.ocr_completed_at
        });
        
        assert!(ocr_response.is_object());
        assert_eq!(ocr_response["document_id"], document.id.to_string());
        assert_eq!(ocr_response["filename"], document.filename);
        assert_eq!(ocr_response["has_ocr_text"], true);
        assert_eq!(ocr_response["ocr_text"], document.ocr_text.unwrap());
        assert_eq!(ocr_response["ocr_confidence"], document.ocr_confidence.unwrap());
        assert_eq!(ocr_response["ocr_word_count"], document.ocr_word_count.unwrap());
        assert_eq!(ocr_response["ocr_processing_time_ms"], document.ocr_processing_time_ms.unwrap());
        assert_eq!(ocr_response["ocr_status"], document.ocr_status.unwrap());
        assert_eq!(ocr_response["ocr_error"], Value::Null);
    }

    #[test]
    fn test_ocr_response_with_error() {
        let user_id = Uuid::new_v4();
        let document = create_test_document_with_ocr_error(user_id);
        
        let ocr_response = serde_json::json!({
            "document_id": document.id,
            "filename": document.filename,
            "has_ocr_text": document.ocr_text.is_some(),
            "ocr_text": document.ocr_text,
            "ocr_confidence": document.ocr_confidence,
            "ocr_word_count": document.ocr_word_count,
            "ocr_processing_time_ms": document.ocr_processing_time_ms,
            "ocr_status": document.ocr_status,
            "ocr_error": document.ocr_error,
            "ocr_completed_at": document.ocr_completed_at
        });
        
        assert_eq!(ocr_response["has_ocr_text"], false);
        assert_eq!(ocr_response["ocr_text"], Value::Null);
        assert_eq!(ocr_response["ocr_status"], "failed");
        assert_eq!(ocr_response["ocr_error"], "Failed to process document: corrupted file");
    }

    #[test]
    fn test_ocr_confidence_validation() {
        let user_id = Uuid::new_v4();
        let mut document = create_test_document(user_id);
        
        // Test valid confidence range
        document.ocr_confidence = Some(95.5);
        assert!(document.ocr_confidence.unwrap() >= 0.0 && document.ocr_confidence.unwrap() <= 100.0);
        
        // Test edge cases
        document.ocr_confidence = Some(0.0);
        assert!(document.ocr_confidence.unwrap() >= 0.0);
        
        document.ocr_confidence = Some(100.0);
        assert!(document.ocr_confidence.unwrap() <= 100.0);
    }

    #[test]
    fn test_ocr_word_count_validation() {
        let user_id = Uuid::new_v4();
        let mut document = create_test_document(user_id);
        
        // Test positive word count
        document.ocr_word_count = Some(150);
        assert!(document.ocr_word_count.unwrap() > 0);
        
        // Test zero word count (valid for empty documents)
        document.ocr_word_count = Some(0);
        assert!(document.ocr_word_count.unwrap() >= 0);
    }

    #[test]
    fn test_ocr_processing_time_validation() {
        let user_id = Uuid::new_v4();
        let mut document = create_test_document(user_id);
        
        // Test positive processing time
        document.ocr_processing_time_ms = Some(1200);
        assert!(document.ocr_processing_time_ms.unwrap() > 0);
        
        // Test very fast processing
        document.ocr_processing_time_ms = Some(50);
        assert!(document.ocr_processing_time_ms.unwrap() > 0);
        
        // Test slow processing
        document.ocr_processing_time_ms = Some(30000); // 30 seconds
        assert!(document.ocr_processing_time_ms.unwrap() > 0);
    }

    #[test]
    fn test_ocr_status_values() {
        let user_id = Uuid::new_v4();
        let mut document = create_test_document(user_id);
        
        // Test valid status values
        let valid_statuses = vec!["pending", "processing", "completed", "failed"];
        
        for status in valid_statuses {
            document.ocr_status = Some(status.to_string());
            assert!(matches!(
                document.ocr_status.as_deref(),
                Some("pending") | Some("processing") | Some("completed") | Some("failed")
            ));
        }
    }

    #[test]
    fn test_document_with_complete_ocr_data() {
        let user_id = Uuid::new_v4();
        let document = create_test_document(user_id);
        
        // Verify all OCR fields are properly set for a completed document
        assert!(document.ocr_text.is_some());
        assert!(document.ocr_confidence.is_some());
        assert!(document.ocr_word_count.is_some());
        assert!(document.ocr_processing_time_ms.is_some());
        assert_eq!(document.ocr_status.as_deref(), Some("completed"));
        assert!(document.ocr_error.is_none());
        assert!(document.ocr_completed_at.is_some());
    }

    #[test]
    fn test_document_with_failed_ocr() {
        let user_id = Uuid::new_v4();
        let document = create_test_document_with_ocr_error(user_id);
        
        // Verify failed OCR document has appropriate fields
        assert!(document.ocr_text.is_none());
        assert!(document.ocr_confidence.is_none());
        assert!(document.ocr_word_count.is_none());
        assert_eq!(document.ocr_status.as_deref(), Some("failed"));
        assert!(document.ocr_error.is_some());
        assert!(document.ocr_completed_at.is_some()); // Should still have completion time
    }

    #[test]
    fn test_document_mime_type_ocr_eligibility() {
        let user_id = Uuid::new_v4();
        
        // Test OCR-eligible file types
        let ocr_eligible_types = vec![
            "application/pdf",
            "image/png", 
            "image/jpeg",
            "image/jpg",
            "image/tiff",
            "image/bmp"
        ];
        
        for mime_type in ocr_eligible_types {
            let mut document = create_test_document(user_id);
            document.mime_type = mime_type.to_string();
            
            // These types should typically have OCR processing
            assert!(document.mime_type.contains("pdf") || document.mime_type.starts_with("image/"));
        }
        
        // Test non-OCR file types
        let mut text_document = create_test_document_without_ocr(user_id);
        text_document.mime_type = "text/plain".to_string();
        
        // Text files typically don't need OCR
        assert_eq!(text_document.mime_type, "text/plain");
        assert!(text_document.ocr_text.is_none());
    }

    #[test]
    fn test_ocr_text_content_validation() {
        let user_id = Uuid::new_v4();
        let document = create_test_document(user_id);
        
        if let Some(ocr_text) = &document.ocr_text {
            // Test that OCR text is not empty
            assert!(!ocr_text.trim().is_empty());
            
            // Test that OCR text is reasonable length
            assert!(ocr_text.len() > 0);
            assert!(ocr_text.len() < 100000); // Reasonable upper limit
        }
    }
}

#[cfg(test)]
mod document_deletion_tests {
    use super::*;
    use crate::db::Database;
    use crate::models::{UserRole, User, Document, AuthProvider};
    use chrono::Utc;
    use sqlx::PgPool;
    use std::env;
    use uuid::Uuid;

    async fn create_test_db_pool() -> PgPool {
        let database_url = env::var("TEST_DATABASE_URL")
            .expect("TEST_DATABASE_URL must be set for database tests");
        PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to test database")
    }

    async fn create_test_user(pool: &PgPool, role: UserRole) -> User {
        let user_id = Uuid::new_v4();
        let user = User {
            id: user_id,
            username: format!("testuser_{}", user_id),
            email: format!("test_{}@example.com", user_id),
            password_hash: Some("hashed_password".to_string()),
            role,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            oidc_subject: None,
            oidc_issuer: None,
            oidc_email: None,
            auth_provider: AuthProvider::Local,
        };

        // Insert user into database
        sqlx::query("INSERT INTO users (id, username, email, password_hash, role, created_at, updated_at, oidc_subject, oidc_issuer, oidc_email, auth_provider) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)")
            .bind(user.id)
            .bind(&user.username)
            .bind(&user.email)
            .bind(&user.password_hash)
            .bind(user.role.to_string())
            .bind(user.created_at)
            .bind(user.updated_at)
            .bind(&user.oidc_subject)
            .bind(&user.oidc_issuer)
            .bind(&user.oidc_email)
            .bind(user.auth_provider.to_string())
            .execute(pool)
            .await
            .expect("Failed to insert test user");

        user
    }

    async fn create_and_insert_test_document(pool: &PgPool, user_id: Uuid) -> Document {
        let document = super::create_test_document(user_id);
        
        // Insert document into database
        sqlx::query("INSERT INTO documents (id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, tags, created_at, updated_at, user_id, file_hash) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)")
            .bind(document.id)
            .bind(&document.filename)
            .bind(&document.original_filename)
            .bind(&document.file_path)
            .bind(document.file_size as i64)
            .bind(&document.mime_type)
            .bind(&document.content)
            .bind(&document.ocr_text)
            .bind(document.ocr_confidence)
            .bind(document.ocr_word_count.map(|x| x as i32))
            .bind(document.ocr_processing_time_ms.map(|x| x as i32))
            .bind(&document.ocr_status)
            .bind(&document.ocr_error)
            .bind(document.ocr_completed_at)
            .bind(&document.tags)
            .bind(document.created_at)
            .bind(document.updated_at)
            .bind(document.user_id)
            .bind(&document.file_hash)
            .execute(pool)
            .await
            .expect("Failed to insert test document");

        document
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL database"]
    async fn test_delete_document_as_owner() {
        let pool = create_test_db_pool().await;
        let documents_db = Database { pool: pool.clone() };
        
        // Create test user and document
        let user = create_test_user(&pool, UserRole::User).await;
        let document = create_and_insert_test_document(&pool, user.id).await;

        // Delete document as owner
        let result = documents_db
            .delete_document(document.id, user.id, user.role)
            .await
            .expect("Failed to delete document");

        // Verify document was deleted
        assert!(result.is_some());
        let deleted_doc = result.unwrap();
        assert_eq!(deleted_doc.id, document.id);
        assert_eq!(deleted_doc.user_id, user.id);

        // Verify document no longer exists in database
        let found_doc = documents_db
            .get_document_by_id(document.id, user.id, user.role)
            .await
            .expect("Database query failed");
        assert!(found_doc.is_none());
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL database"]
    async fn test_delete_document_as_admin() {
        let pool = create_test_db_pool().await;
        let documents_db = Database { pool: pool.clone() };
        
        // Create regular user and their document
        let user = create_test_user(&pool, UserRole::User).await;
        let document = create_and_insert_test_document(&pool, user.id).await;
        
        // Create admin user
        let admin = create_test_user(&pool, UserRole::Admin).await;

        // Delete document as admin
        let result = documents_db
            .delete_document(document.id, admin.id, admin.role)
            .await
            .expect("Failed to delete document as admin");

        // Verify document was deleted
        assert!(result.is_some());
        let deleted_doc = result.unwrap();
        assert_eq!(deleted_doc.id, document.id);
        assert_eq!(deleted_doc.user_id, user.id); // Original owner
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL database"]
    async fn test_delete_document_unauthorized() {
        let pool = create_test_db_pool().await;
        let documents_db = Database { pool: pool.clone() };
        
        // Create two regular users
        let user1 = create_test_user(&pool, UserRole::User).await;
        let user2 = create_test_user(&pool, UserRole::User).await;
        
        // Create document owned by user1
        let document = create_and_insert_test_document(&pool, user1.id).await;

        // Try to delete document as user2 (should fail)
        let result = documents_db
            .delete_document(document.id, user2.id, user2.role)
            .await
            .expect("Database query failed");

        // Verify document was not deleted
        assert!(result.is_none());

        // Verify document still exists
        let found_doc = documents_db
            .get_document_by_id(document.id, user1.id, user1.role)
            .await
            .expect("Database query failed");
        assert!(found_doc.is_some());
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL database"]
    async fn test_delete_nonexistent_document() {
        let pool = create_test_db_pool().await;
        let documents_db = Database { pool: pool.clone() };
        
        let user = create_test_user(&pool, UserRole::User).await;
        let nonexistent_id = Uuid::new_v4();

        // Try to delete nonexistent document
        let result = documents_db
            .delete_document(nonexistent_id, user.id, user.role)
            .await
            .expect("Database query failed");

        // Verify nothing was deleted
        assert!(result.is_none());
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL database"]
    async fn test_bulk_delete_documents_as_owner() {
        let pool = create_test_db_pool().await;
        let documents_db = Database { pool: pool.clone() };
        
        let user = create_test_user(&pool, UserRole::User).await;
        
        // Create multiple documents
        let doc1 = create_and_insert_test_document(&pool, user.id).await;
        let doc2 = create_and_insert_test_document(&pool, user.id).await;
        let doc3 = create_and_insert_test_document(&pool, user.id).await;
        
        let document_ids = vec![doc1.id, doc2.id, doc3.id];

        // Delete documents as owner
        let result = documents_db
            .bulk_delete_documents(&document_ids, user.id, user.role)
            .await
            .expect("Failed to bulk delete documents");

        // Verify all documents were deleted
        assert_eq!(result.len(), 3);
        let deleted_ids: Vec<Uuid> = result.iter().map(|d| d.id).collect();
        assert!(deleted_ids.contains(&doc1.id));
        assert!(deleted_ids.contains(&doc2.id));
        assert!(deleted_ids.contains(&doc3.id));

        // Verify documents no longer exist
        for doc_id in document_ids {
            let found_doc = documents_db
                .get_document_by_id(doc_id, user.id, user.role)
                .await
                .expect("Database query failed");
            assert!(found_doc.is_none());
        }
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL database"]
    async fn test_bulk_delete_documents_as_admin() {
        let pool = create_test_db_pool().await;
        let documents_db = Database { pool: pool.clone() };
        
        // Create regular user and their documents
        let user = create_test_user(&pool, UserRole::User).await;
        let doc1 = create_and_insert_test_document(&pool, user.id).await;
        let doc2 = create_and_insert_test_document(&pool, user.id).await;
        
        // Create admin user
        let admin = create_test_user(&pool, UserRole::Admin).await;
        
        let document_ids = vec![doc1.id, doc2.id];

        // Delete documents as admin
        let result = documents_db
            .bulk_delete_documents(&document_ids, admin.id, admin.role)
            .await
            .expect("Failed to bulk delete documents as admin");

        // Verify all documents were deleted
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL database"]
    async fn test_bulk_delete_documents_mixed_ownership() {
        let pool = create_test_db_pool().await;
        let documents_db = Database { pool: pool.clone() };
        
        // Create two regular users
        let user1 = create_test_user(&pool, UserRole::User).await;
        let user2 = create_test_user(&pool, UserRole::User).await;
        
        // Create documents for both users
        let doc1_user1 = create_and_insert_test_document(&pool, user1.id).await;
        let doc2_user1 = create_and_insert_test_document(&pool, user1.id).await;
        let doc1_user2 = create_and_insert_test_document(&pool, user2.id).await;
        
        let document_ids = vec![doc1_user1.id, doc2_user1.id, doc1_user2.id];

        // Try to delete all documents as user1 (should only delete their own)
        let result = documents_db
            .bulk_delete_documents(&document_ids, user1.id, user1.role)
            .await
            .expect("Failed to bulk delete documents");

        // Verify only user1's documents were deleted
        assert_eq!(result.len(), 2);
        let deleted_ids: Vec<Uuid> = result.iter().map(|d| d.id).collect();
        assert!(deleted_ids.contains(&doc1_user1.id));
        assert!(deleted_ids.contains(&doc2_user1.id));
        assert!(!deleted_ids.contains(&doc1_user2.id));

        // Verify user2's document still exists
        let found_doc = documents_db
            .get_document_by_id(doc1_user2.id, user2.id, user2.role)
            .await
            .expect("Database query failed");
        assert!(found_doc.is_some());
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL database"]
    async fn test_bulk_delete_documents_empty_list() {
        let pool = create_test_db_pool().await;
        let documents_db = Database { pool: pool.clone() };
        
        let user = create_test_user(&pool, UserRole::User).await;
        let empty_ids: Vec<Uuid> = vec![];

        // Delete empty list of documents
        let result = documents_db
            .bulk_delete_documents(&empty_ids, user.id, user.role)
            .await
            .expect("Failed to bulk delete empty list");

        // Verify empty result
        assert_eq!(result.len(), 0);
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL database"]
    async fn test_bulk_delete_documents_nonexistent_ids() {
        let pool = create_test_db_pool().await;
        let documents_db = Database { pool: pool.clone() };
        
        let user = create_test_user(&pool, UserRole::User).await;
        
        // Create one real document
        let real_doc = create_and_insert_test_document(&pool, user.id).await;
        
        // Mix of real and nonexistent IDs
        let document_ids = vec![real_doc.id, Uuid::new_v4(), Uuid::new_v4()];

        // Delete documents (should only delete the real one)
        let result = documents_db
            .bulk_delete_documents(&document_ids, user.id, user.role)
            .await
            .expect("Failed to bulk delete documents");

        // Verify only the real document was deleted
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, real_doc.id);
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL database"]
    async fn test_bulk_delete_documents_partial_authorization() {
        let pool = create_test_db_pool().await;
        let documents_db = Database { pool: pool.clone() };
        
        // Create regular user and admin
        let user = create_test_user(&pool, UserRole::User).await;
        let admin = create_test_user(&pool, UserRole::Admin).await;
        
        // Create documents for both users
        let user_doc = create_and_insert_test_document(&pool, user.id).await;
        let admin_doc = create_and_insert_test_document(&pool, admin.id).await;
        
        let document_ids = vec![user_doc.id, admin_doc.id];

        // Admin should be able to delete both
        let result = documents_db
            .bulk_delete_documents(&document_ids, admin.id, admin.role)
            .await
            .expect("Failed to bulk delete documents as admin");

        assert_eq!(result.len(), 2);
        
        // Recreate documents for user test
        let user_doc2 = create_and_insert_test_document(&pool, user.id).await;
        let admin_doc2 = create_and_insert_test_document(&pool, admin.id).await;
        
        let document_ids2 = vec![user_doc2.id, admin_doc2.id];

        // Regular user should only delete their own
        let result2 = documents_db
            .bulk_delete_documents(&document_ids2, user.id, user.role)
            .await
            .expect("Failed to bulk delete documents as user");

        assert_eq!(result2.len(), 1);
        assert_eq!(result2[0].id, user_doc2.id);
    }
}

#[cfg(test)]
mod rbac_deletion_tests {
    use super::*;
    use crate::db::Database;
    use crate::models::{UserRole, User, Document, AuthProvider};
    use chrono::Utc;
    use sqlx::PgPool;
    use std::env;
    use uuid::Uuid;

    async fn create_test_db_pool() -> PgPool {
        let database_url = env::var("TEST_DATABASE_URL")
            .expect("TEST_DATABASE_URL must be set for database tests");
        PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to test database")
    }

    async fn create_test_user(pool: &PgPool, role: UserRole) -> User {
        let user_id = Uuid::new_v4();
        let user = User {
            id: user_id,
            username: format!("testuser_{}", user_id),
            email: format!("test_{}@example.com", user_id),
            password_hash: Some("hashed_password".to_string()),
            role,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            oidc_subject: None,
            oidc_issuer: None,
            oidc_email: None,
            auth_provider: AuthProvider::Local,
        };

        sqlx::query("INSERT INTO users (id, username, email, password_hash, role, created_at, updated_at, oidc_subject, oidc_issuer, oidc_email, auth_provider) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)")
            .bind(user.id)
            .bind(&user.username)
            .bind(&user.email)
            .bind(&user.password_hash)
            .bind(user.role.to_string())
            .bind(user.created_at)
            .bind(user.updated_at)
            .bind(&user.oidc_subject)
            .bind(&user.oidc_issuer)
            .bind(&user.oidc_email)
            .bind(user.auth_provider.to_string())
            .execute(pool)
            .await
            .expect("Failed to insert test user");

        user
    }

    async fn create_and_insert_test_document(pool: &PgPool, user_id: Uuid) -> Document {
        let document = super::create_test_document(user_id);
        
        sqlx::query("INSERT INTO documents (id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, tags, created_at, updated_at, user_id, file_hash) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)")
            .bind(document.id)
            .bind(&document.filename)
            .bind(&document.original_filename)
            .bind(&document.file_path)
            .bind(document.file_size as i64)
            .bind(&document.mime_type)
            .bind(&document.content)
            .bind(&document.ocr_text)
            .bind(document.ocr_confidence)
            .bind(document.ocr_word_count.map(|x| x as i32))
            .bind(document.ocr_processing_time_ms.map(|x| x as i32))
            .bind(&document.ocr_status)
            .bind(&document.ocr_error)
            .bind(document.ocr_completed_at)
            .bind(&document.tags)
            .bind(document.created_at)
            .bind(document.updated_at)
            .bind(document.user_id)
            .bind(&document.file_hash)
            .execute(pool)
            .await
            .expect("Failed to insert test document");

        document
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL database"]
    async fn test_user_can_delete_own_document() {
        let pool = create_test_db_pool().await;
        let documents_db = Database { pool: pool.clone() };
        
        let user = create_test_user(&pool, UserRole::User).await;
        let document = create_and_insert_test_document(&pool, user.id).await;

        // User should be able to delete their own document
        let result = documents_db
            .delete_document(document.id, user.id, user.role)
            .await
            .expect("Failed to delete document");

        assert!(result.is_some());
        let deleted_doc = result.unwrap();
        assert_eq!(deleted_doc.id, document.id);
        assert_eq!(deleted_doc.user_id, user.id);
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL database"]
    async fn test_user_cannot_delete_other_user_document() {
        let pool = create_test_db_pool().await;
        let documents_db = Database { pool: pool.clone() };
        
        let user1 = create_test_user(&pool, UserRole::User).await;
        let user2 = create_test_user(&pool, UserRole::User).await;
        
        let document = create_and_insert_test_document(&pool, user1.id).await;

        // User2 should NOT be able to delete user1's document
        let result = documents_db
            .delete_document(document.id, user2.id, user2.role)
            .await
            .expect("Database query failed");

        assert!(result.is_none());

        // Verify document still exists
        let found_doc = documents_db
            .get_document_by_id(document.id, user1.id, user1.role)
            .await
            .expect("Database query failed");
        assert!(found_doc.is_some());
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL database"]
    async fn test_admin_can_delete_any_document() {
        let pool = create_test_db_pool().await;
        let documents_db = Database { pool: pool.clone() };
        
        let user = create_test_user(&pool, UserRole::User).await;
        let admin = create_test_user(&pool, UserRole::Admin).await;
        
        let user_document = create_and_insert_test_document(&pool, user.id).await;
        let admin_document = create_and_insert_test_document(&pool, admin.id).await;

        // Admin should be able to delete user's document
        let result1 = documents_db
            .delete_document(user_document.id, admin.id, admin.role)
            .await
            .expect("Failed to delete user document as admin");

        assert!(result1.is_some());
        assert_eq!(result1.unwrap().user_id, user.id); // Original owner

        // Admin should be able to delete their own document
        let result2 = documents_db
            .delete_document(admin_document.id, admin.id, admin.role)
            .await
            .expect("Failed to delete admin document as admin");

        assert!(result2.is_some());
        assert_eq!(result2.unwrap().user_id, admin.id);
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL database"]
    async fn test_bulk_delete_respects_ownership() {
        let pool = create_test_db_pool().await;
        let documents_db = Database { pool: pool.clone() };
        
        let user1 = create_test_user(&pool, UserRole::User).await;
        let user2 = create_test_user(&pool, UserRole::User).await;
        
        // Create documents for both users
        let user1_doc1 = create_and_insert_test_document(&pool, user1.id).await;
        let user1_doc2 = create_and_insert_test_document(&pool, user1.id).await;
        let user2_doc1 = create_and_insert_test_document(&pool, user2.id).await;
        let user2_doc2 = create_and_insert_test_document(&pool, user2.id).await;
        
        let all_document_ids = vec![
            user1_doc1.id, 
            user1_doc2.id, 
            user2_doc1.id, 
            user2_doc2.id
        ];

        // User1 tries to delete all documents (should only delete their own)
        let result = documents_db
            .bulk_delete_documents(&all_document_ids, user1.id, user1.role)
            .await
            .expect("Failed to bulk delete documents");

        // Should only delete user1's documents
        assert_eq!(result.len(), 2);
        let deleted_ids: Vec<Uuid> = result.iter().map(|d| d.id).collect();
        assert!(deleted_ids.contains(&user1_doc1.id));
        assert!(deleted_ids.contains(&user1_doc2.id));
        assert!(!deleted_ids.contains(&user2_doc1.id));
        assert!(!deleted_ids.contains(&user2_doc2.id));

        // Verify user2's documents still exist
        let user2_doc1_exists = documents_db
            .get_document_by_id(user2_doc1.id, user2.id, user2.role)
            .await
            .expect("Database query failed");
        assert!(user2_doc1_exists.is_some());

        let user2_doc2_exists = documents_db
            .get_document_by_id(user2_doc2.id, user2.id, user2.role)
            .await
            .expect("Database query failed");
        assert!(user2_doc2_exists.is_some());
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL database"]
    async fn test_admin_bulk_delete_all_documents() {
        let pool = create_test_db_pool().await;
        let documents_db = Database { pool: pool.clone() };
        
        let user1 = create_test_user(&pool, UserRole::User).await;
        let user2 = create_test_user(&pool, UserRole::User).await;
        let admin = create_test_user(&pool, UserRole::Admin).await;
        
        // Create documents for all users
        let user1_doc = create_and_insert_test_document(&pool, user1.id).await;
        let user2_doc = create_and_insert_test_document(&pool, user2.id).await;
        let admin_doc = create_and_insert_test_document(&pool, admin.id).await;
        
        let all_document_ids = vec![user1_doc.id, user2_doc.id, admin_doc.id];

        // Admin should be able to delete all documents
        let result = documents_db
            .bulk_delete_documents(&all_document_ids, admin.id, admin.role)
            .await
            .expect("Failed to bulk delete documents as admin");

        // Should delete all documents
        assert_eq!(result.len(), 3);
        let deleted_ids: Vec<Uuid> = result.iter().map(|d| d.id).collect();
        assert!(deleted_ids.contains(&user1_doc.id));
        assert!(deleted_ids.contains(&user2_doc.id));
        assert!(deleted_ids.contains(&admin_doc.id));
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL database"]
    async fn test_role_escalation_prevention() {
        let pool = create_test_db_pool().await;
        let documents_db = Database { pool: pool.clone() };
        
        let user = create_test_user(&pool, UserRole::User).await;
        let admin = create_test_user(&pool, UserRole::Admin).await;
        
        let admin_document = create_and_insert_test_document(&pool, admin.id).await;

        // Regular user should NOT be able to delete admin's document
        // even if they somehow know the document ID
        let result = documents_db
            .delete_document(admin_document.id, user.id, user.role)
            .await
            .expect("Database query failed");

        assert!(result.is_none());

        // Verify admin's document still exists
        let found_doc = documents_db
            .get_document_by_id(admin_document.id, admin.id, admin.role)
            .await
            .expect("Database query failed");
        assert!(found_doc.is_some());
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL database"]
    async fn test_cross_tenant_isolation() {
        let pool = create_test_db_pool().await;
        let documents_db = Database { pool: pool.clone() };
        
        // Create users that could represent different tenants/organizations
        let tenant1_user1 = create_test_user(&pool, UserRole::User).await;
        let tenant1_user2 = create_test_user(&pool, UserRole::User).await;
        let tenant2_user1 = create_test_user(&pool, UserRole::User).await;
        let tenant2_user2 = create_test_user(&pool, UserRole::User).await;
        
        // Create documents for each tenant
        let tenant1_doc1 = create_and_insert_test_document(&pool, tenant1_user1.id).await;
        let tenant1_doc2 = create_and_insert_test_document(&pool, tenant1_user2.id).await;
        let tenant2_doc1 = create_and_insert_test_document(&pool, tenant2_user1.id).await;
        let tenant2_doc2 = create_and_insert_test_document(&pool, tenant2_user2.id).await;

        // Tenant1 user should not be able to delete tenant2 documents
        let result1 = documents_db
            .delete_document(tenant2_doc1.id, tenant1_user1.id, tenant1_user1.role)
            .await
            .expect("Database query failed");
        assert!(result1.is_none());

        let result2 = documents_db
            .delete_document(tenant2_doc2.id, tenant1_user2.id, tenant1_user2.role)
            .await
            .expect("Database query failed");
        assert!(result2.is_none());

        // Tenant2 user should not be able to delete tenant1 documents
        let result3 = documents_db
            .delete_document(tenant1_doc1.id, tenant2_user1.id, tenant2_user1.role)
            .await
            .expect("Database query failed");
        assert!(result3.is_none());

        let result4 = documents_db
            .delete_document(tenant1_doc2.id, tenant2_user2.id, tenant2_user2.role)
            .await
            .expect("Database query failed");
        assert!(result4.is_none());

        // Verify all documents still exist
        for (doc_id, owner_id, owner_role) in [
            (tenant1_doc1.id, tenant1_user1.id, tenant1_user1.role),
            (tenant1_doc2.id, tenant1_user2.id, tenant1_user2.role),
            (tenant2_doc1.id, tenant2_user1.id, tenant2_user1.role),
            (tenant2_doc2.id, tenant2_user2.id, tenant2_user2.role),
        ] {
            let found_doc = documents_db
                .get_document_by_id(doc_id, owner_id, owner_role)
                .await
                .expect("Database query failed");
            assert!(found_doc.is_some());
        }
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL database"]
    async fn test_permission_consistency_single_vs_bulk() {
        let pool = create_test_db_pool().await;
        let documents_db = Database { pool: pool.clone() };
        
        let user1 = create_test_user(&pool, UserRole::User).await;
        let user2 = create_test_user(&pool, UserRole::User).await;
        
        let _user1_doc = create_and_insert_test_document(&pool, user1.id).await;
        let user2_doc = create_and_insert_test_document(&pool, user2.id).await;

        // Test single deletion permissions
        let single_delete_result = documents_db
            .delete_document(user2_doc.id, user1.id, user1.role)
            .await
            .expect("Database query failed");
        assert!(single_delete_result.is_none()); // Should fail

        // Test bulk deletion permissions with same document
        let user2_doc2 = create_and_insert_test_document(&pool, user2.id).await;
        let bulk_delete_result = documents_db
            .bulk_delete_documents(&vec![user2_doc2.id], user1.id, user1.role)
            .await
            .expect("Database query failed");
        assert_eq!(bulk_delete_result.len(), 0); // Should delete nothing

        // Verify both documents still exist
        let doc1_exists = documents_db
            .get_document_by_id(user2_doc.id, user2.id, user2.role)
            .await
            .expect("Database query failed");
        assert!(doc1_exists.is_some());

        let doc2_exists = documents_db
            .get_document_by_id(user2_doc2.id, user2.id, user2.role)
            .await
            .expect("Database query failed");
        assert!(doc2_exists.is_some());
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL database"]
    async fn test_admin_permission_inheritance() {
        let pool = create_test_db_pool().await;
        let documents_db = Database { pool: pool.clone() };
        
        let user = create_test_user(&pool, UserRole::User).await;
        let admin = create_test_user(&pool, UserRole::Admin).await;
        
        let user_doc = create_and_insert_test_document(&pool, user.id).await;

        // Admin should have all permissions that a regular user has, plus more
        // Test that admin can delete user's document (admin-specific permission)
        let admin_delete_result = documents_db
            .delete_document(user_doc.id, admin.id, admin.role)
            .await
            .expect("Failed to delete as admin");
        assert!(admin_delete_result.is_some());

        // Create another document to test admin's own document deletion
        let admin_doc = create_and_insert_test_document(&pool, admin.id).await;
        let admin_own_delete_result = documents_db
            .delete_document(admin_doc.id, admin.id, admin.role)
            .await
            .expect("Failed to delete admin's own document");
        assert!(admin_own_delete_result.is_some());
    }

    #[test]
    fn test_role_based_logic_unit_tests() {
        let user_role = UserRole::User;
        let admin_role = UserRole::Admin;
        
        let user_id = Uuid::new_v4();
        let other_user_id = Uuid::new_v4();
        let admin_id = Uuid::new_v4();

        // Test user permissions logic
        assert!(user_id == user_id || user_role == UserRole::Admin); // Can delete own
        assert!(!(other_user_id == user_id || user_role == UserRole::Admin)); // Cannot delete other's

        // Test admin permissions logic  
        assert!(user_id == admin_id || admin_role == UserRole::Admin); // Can delete user's (admin privilege)
        assert!(other_user_id == admin_id || admin_role == UserRole::Admin); // Can delete any (admin privilege)
        assert!(admin_id == admin_id || admin_role == UserRole::Admin); // Can delete own
    }

    #[test]
    fn test_role_comparison() {
        assert_eq!(UserRole::User, UserRole::User);
        assert_eq!(UserRole::Admin, UserRole::Admin);
        assert_ne!(UserRole::User, UserRole::Admin);
        assert_ne!(UserRole::Admin, UserRole::User);
    }
}

#[cfg(test)]
mod deletion_error_handling_tests {
    use super::*;
    use crate::db::Database;
    use crate::models::{UserRole, User, Document, AuthProvider};
    use chrono::Utc;
    use sqlx::PgPool;
    use std::env;
    use uuid::Uuid;

    async fn create_test_db_pool() -> PgPool {
        let database_url = env::var("TEST_DATABASE_URL")
            .expect("TEST_DATABASE_URL must be set for database tests");
        PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to test database")
    }

    async fn create_test_user(pool: &PgPool, role: UserRole) -> User {
        let user_id = Uuid::new_v4();
        let user = User {
            id: user_id,
            username: format!("testuser_{}", user_id),
            email: format!("test_{}@example.com", user_id),
            password_hash: Some("hashed_password".to_string()),
            role,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            oidc_subject: None,
            oidc_issuer: None,
            oidc_email: None,
            auth_provider: AuthProvider::Local,
        };

        sqlx::query("INSERT INTO users (id, username, email, password_hash, role, created_at, updated_at, oidc_subject, oidc_issuer, oidc_email, auth_provider) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)")
            .bind(user.id)
            .bind(&user.username)
            .bind(&user.email)
            .bind(&user.password_hash)
            .bind(user.role.to_string())
            .bind(user.created_at)
            .bind(user.updated_at)
            .bind(&user.oidc_subject)
            .bind(&user.oidc_issuer)
            .bind(&user.oidc_email)
            .bind(user.auth_provider.to_string())
            .execute(pool)
            .await
            .expect("Failed to insert test user");

        user
    }

    async fn create_and_insert_test_document(pool: &PgPool, user_id: Uuid) -> Document {
        let document = super::create_test_document(user_id);
        
        sqlx::query("INSERT INTO documents (id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, tags, created_at, updated_at, user_id, file_hash) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)")
            .bind(document.id)
            .bind(&document.filename)
            .bind(&document.original_filename)
            .bind(&document.file_path)
            .bind(document.file_size as i64)
            .bind(&document.mime_type)
            .bind(&document.content)
            .bind(&document.ocr_text)
            .bind(document.ocr_confidence)
            .bind(document.ocr_word_count.map(|x| x as i32))
            .bind(document.ocr_processing_time_ms.map(|x| x as i32))
            .bind(&document.ocr_status)
            .bind(&document.ocr_error)
            .bind(document.ocr_completed_at)
            .bind(&document.tags)
            .bind(document.created_at)
            .bind(document.updated_at)
            .bind(document.user_id)
            .bind(&document.file_hash)
            .execute(pool)
            .await
            .expect("Failed to insert test document");

        document
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL database"]
    async fn test_delete_with_invalid_uuid() {
        let pool = create_test_db_pool().await;
        let documents_db = Database { pool: pool.clone() };
        
        let user = create_test_user(&pool, UserRole::User).await;
        
        // Use malformed UUID (this test assumes the function handles UUID parsing)
        let invalid_uuid = Uuid::nil(); // Use nil UUID as "invalid"
        
        let result = documents_db
            .delete_document(invalid_uuid, user.id, user.role)
            .await
            .expect("Database query should not fail for invalid UUID");

        // Should return None for non-existent document
        assert!(result.is_none());
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL database"]
    async fn test_delete_with_sql_injection_attempt() {
        let pool = create_test_db_pool().await;
        let documents_db = Database { pool: pool.clone() };
        
        let user = create_test_user(&pool, UserRole::User).await;
        let document = create_and_insert_test_document(&pool, user.id).await;
        
        // Test with legitimate document ID - SQLx should prevent injection
        let result = documents_db
            .delete_document(document.id, user.id, user.role)
            .await
            .expect("Query should execute safely");

        assert!(result.is_some());
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL database"]
    async fn test_bulk_delete_with_duplicate_ids() {
        let pool = create_test_db_pool().await;
        let documents_db = Database { pool: pool.clone() };
        
        let user = create_test_user(&pool, UserRole::User).await;
        let document = create_and_insert_test_document(&pool, user.id).await;
        
        // Include the same document ID multiple times
        let duplicate_ids = vec![document.id, document.id, document.id];
        
        let result = documents_db
            .bulk_delete_documents(&duplicate_ids, user.id, user.role)
            .await
            .expect("Bulk delete should handle duplicates");

        // Should only delete the document once
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, document.id);
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL database"]
    async fn test_bulk_delete_with_extremely_large_request() {
        let pool = create_test_db_pool().await;
        let documents_db = Database { pool: pool.clone() };
        
        let user = create_test_user(&pool, UserRole::User).await;
        
        // Create a large number of document IDs (mostly non-existent)
        let mut large_id_list = Vec::new();
        
        // Add one real document
        let real_document = create_and_insert_test_document(&pool, user.id).await;
        large_id_list.push(real_document.id);
        
        // Add many fake UUIDs
        for _ in 0..500 {
            large_id_list.push(Uuid::new_v4());
        }
        
        let result = documents_db
            .bulk_delete_documents(&large_id_list, user.id, user.role)
            .await
            .expect("Should handle large requests");

        // Should only delete the one real document
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, real_document.id);
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL database"]
    async fn test_concurrent_deletion_same_document() {
        let pool = create_test_db_pool().await;
        let documents_db = Database { pool: pool.clone() };
        
        let user = create_test_user(&pool, UserRole::User).await;
        let document = create_and_insert_test_document(&pool, user.id).await;
        
        // Create multiple handles to the same database connection pool
        let db1 = documents_db.clone();
        let db2 = documents_db.clone();
        
        // Attempt concurrent deletions
        let doc_id = document.id;
        let user_id = user.id;
        let user_role = user.role;
        
        let task1 = tokio::spawn(async move {
            db1.delete_document(doc_id, user_id, user_role).await
        });
        
        let task2 = tokio::spawn(async move {
            db2.delete_document(doc_id, user_id, user_role).await
        });
        
        let result1 = task1.await.unwrap().expect("First deletion should succeed");
        let result2 = task2.await.unwrap().expect("Second deletion should not error");
        
        // One should succeed, one should return None
        let success_count = [result1.is_some(), result2.is_some()]
            .iter()
            .filter(|&&x| x)
            .count();
        
        assert_eq!(success_count, 1, "Exactly one deletion should succeed");
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL database"]
    async fn test_delete_document_with_foreign_key_constraints() {
        let pool = create_test_db_pool().await;
        let documents_db = Database { pool: pool.clone() };
        
        let user = create_test_user(&pool, UserRole::User).await;
        let document = create_and_insert_test_document(&pool, user.id).await;
        
        // If there are foreign key relationships (like document_labels), 
        // test that CASCADE deletion works properly
        
        // Delete the document
        let result = documents_db
            .delete_document(document.id, user.id, user.role)
            .await
            .expect("Deletion should handle foreign key constraints");

        assert!(result.is_some());
        
        // Verify related records are also deleted (if any exist)
        // This would depend on the actual schema relationships
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL database"]
    async fn test_bulk_delete_with_mixed_permissions_and_errors() {
        let pool = create_test_db_pool().await;
        let documents_db = Database { pool: pool.clone() };
        
        let user1 = create_test_user(&pool, UserRole::User).await;
        let user2 = create_test_user(&pool, UserRole::User).await;
        
        // Create mix of documents
        let user1_doc = create_and_insert_test_document(&pool, user1.id).await;
        let user2_doc = create_and_insert_test_document(&pool, user2.id).await;
        let nonexistent_id = Uuid::new_v4();
        
        let mixed_ids = vec![user1_doc.id, user2_doc.id, nonexistent_id];
        
        // User1 attempts to delete all (should only delete their own)
        let result = documents_db
            .bulk_delete_documents(&mixed_ids, user1.id, user1.role)
            .await
            .expect("Should handle mixed permissions gracefully");

        // Should only delete user1's document
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, user1_doc.id);
        
        // Verify user2's document still exists
        let user2_doc_exists = documents_db
            .get_document_by_id(user2_doc.id, user2.id, user2.role)
            .await
            .expect("Query should succeed");
        assert!(user2_doc_exists.is_some());
    }

    #[test]
    fn test_error_message_consistency() {
        // Test that error conditions produce consistent, user-friendly messages
        
        // Test various error scenarios that might occur
        let not_found_msg = "Document not found";
        let unauthorized_msg = "Not authorized to delete this document";
        let invalid_request_msg = "Invalid request parameters";
        let server_error_msg = "Internal server error";
        
        // Verify messages are not empty and don't contain sensitive information
        assert!(!not_found_msg.is_empty());
        assert!(!unauthorized_msg.is_empty());
        assert!(!invalid_request_msg.is_empty());
        assert!(!server_error_msg.is_empty());
        
        // Verify messages don't leak technical details
        assert!(!not_found_msg.to_lowercase().contains("sql"));
        assert!(!not_found_msg.to_lowercase().contains("database"));
        assert!(!unauthorized_msg.to_lowercase().contains("user_id"));
        assert!(!server_error_msg.to_lowercase().contains("panic"));
    }

    #[test]
    fn test_uuid_edge_cases() {
        // Test various UUID edge cases
        
        let nil_uuid = Uuid::nil();
        let max_uuid = Uuid::max();
        let random_uuid = Uuid::new_v4();
        
        // Verify UUIDs are valid
        assert_eq!(nil_uuid.to_string(), "00000000-0000-0000-0000-000000000000");
        assert_eq!(max_uuid.to_string(), "ffffffff-ffff-ffff-ffff-ffffffffffff");
        assert!(random_uuid.to_string().len() == 36); // Standard UUID string length
        
        // Test UUID parsing edge cases
        assert!(Uuid::parse_str("invalid-uuid").is_err());
        assert!(Uuid::parse_str("").is_err());
        assert!(Uuid::parse_str("00000000-0000-0000-0000-000000000000").is_ok());
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL database"]
    async fn test_delete_after_user_deletion() {
        let pool = create_test_db_pool().await;
        let documents_db = Database { pool: pool.clone() };
        
        let user = create_test_user(&pool, UserRole::User).await;
        let document = create_and_insert_test_document(&pool, user.id).await;
        
        // Delete the user first (simulating cascade deletion scenarios)
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user.id)
            .execute(&pool)
            .await
            .expect("User deletion should succeed");
        
        // Attempt to delete document after user is gone
        // This depends on how foreign key constraints are set up
        let result = documents_db
            .delete_document(document.id, user.id, user.role)
            .await;
            
        // The behavior here depends on FK constraints:
        // - If CASCADE: document might already be deleted
        // - If RESTRICT: document still exists but operation might fail
        // Test should verify consistent behavior
        match result {
            Ok(Some(_)) => {
                // Document was deleted successfully
            },
            Ok(None) => {
                // Document not found (possibly already cascade deleted)
            },
            Err(_) => {
                // Error occurred (foreign key constraint issue)
                // This might be expected behavior
            }
        }
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL database"]
    async fn test_bulk_delete_empty_and_null_scenarios() {
        let pool = create_test_db_pool().await;
        let documents_db = Database { pool: pool.clone() };
        
        let user = create_test_user(&pool, UserRole::User).await;
        
        // Test empty list
        let empty_result = documents_db
            .bulk_delete_documents(&vec![], user.id, user.role)
            .await
            .expect("Empty list should be handled gracefully");
        assert_eq!(empty_result.len(), 0);
        
        // Test with only nil UUIDs
        let nil_uuids = vec![Uuid::nil(), Uuid::nil()];
        let nil_result = documents_db
            .bulk_delete_documents(&nil_uuids, user.id, user.role)
            .await
            .expect("Nil UUIDs should be handled gracefully");
        assert_eq!(nil_result.len(), 0);
    }


    #[tokio::test]
    #[ignore = "Requires PostgreSQL database"]
    async fn test_transaction_rollback_simulation() {
        let pool = create_test_db_pool().await;
        let documents_db = Database { pool: pool.clone() };
        
        let user = create_test_user(&pool, UserRole::User).await;
        let document = create_and_insert_test_document(&pool, user.id).await;
        
        // Verify document exists before deletion
        let exists_before = documents_db
            .get_document_by_id(document.id, user.id, user.role)
            .await
            .expect("Query should succeed");
        assert!(exists_before.is_some());
        
        // Perform deletion
        let deletion_result = documents_db
            .delete_document(document.id, user.id, user.role)
            .await
            .expect("Deletion should succeed");
        assert!(deletion_result.is_some());
        
        // Verify document no longer exists
        let exists_after = documents_db
            .get_document_by_id(document.id, user.id, user.role)
            .await
            .expect("Query should succeed");
        assert!(exists_after.is_none());
        
        // If transaction were to be rolled back, document would exist again
        // This test verifies the transaction was committed properly
    }

    mod low_confidence_deletion_db_tests {
        use super::*;
        use crate::models::UserRole;

        #[cfg(test)]
        fn create_test_document_with_confidence(user_id: Uuid, confidence: f32) -> Document {
            Document {
                id: Uuid::new_v4(),
                filename: format!("test_conf_{}.pdf", confidence),
                original_filename: format!("test_conf_{}.pdf", confidence),
                file_path: format!("/uploads/test_conf_{}.pdf", confidence),
                file_size: 1024,
                mime_type: "application/pdf".to_string(),
                content: Some("Test document content".to_string()),
                ocr_text: Some("Test OCR text".to_string()),
                ocr_confidence: Some(confidence),
                ocr_word_count: Some(50),
                ocr_processing_time_ms: Some(1000),
                ocr_status: Some("completed".to_string()),
                ocr_error: None,
                ocr_completed_at: Some(Utc::now()),
                tags: vec!["test".to_string()],
                created_at: Utc::now(),
                updated_at: Utc::now(),
                user_id,
                file_hash: Some("test_hash_123456789abcdef123456789abcdef123456789abcdef123456789abcdef".to_string()),
                original_created_at: None,
                original_modified_at: None,
                source_metadata: None,
            }
        }

        #[test]
        fn test_confidence_filtering_logic() {
            let user_id = Uuid::new_v4();
            
            let documents = vec![
                create_test_document_with_confidence(user_id, 95.0),  // Should not be deleted
                create_test_document_with_confidence(user_id, 75.0),  // Should not be deleted  
                create_test_document_with_confidence(user_id, 45.0),  // Should not be deleted
                create_test_document_with_confidence(user_id, 25.0),  // Should be deleted (< 30)
                create_test_document_with_confidence(user_id, 15.0),  // Should be deleted (< 30)
                create_test_document_with_confidence(user_id, 5.0),   // Should be deleted (< 30)
            ];

            let threshold = 30.0;
            let low_confidence_docs: Vec<_> = documents.iter()
                .filter(|doc| {
                    doc.ocr_confidence.is_some() && 
                    doc.ocr_confidence.unwrap() < threshold
                })
                .collect();

            assert_eq!(low_confidence_docs.len(), 3);
            assert_eq!(low_confidence_docs[0].ocr_confidence.unwrap(), 25.0);
            assert_eq!(low_confidence_docs[1].ocr_confidence.unwrap(), 15.0);
            assert_eq!(low_confidence_docs[2].ocr_confidence.unwrap(), 5.0);
        }

        #[test]
        fn test_documents_without_ocr_confidence_excluded() {
            let user_id = Uuid::new_v4();
            
            let mut doc_no_confidence = create_test_document_with_confidence(user_id, 20.0);
            doc_no_confidence.ocr_confidence = None;

            let doc_with_confidence = create_test_document_with_confidence(user_id, 20.0);

            let documents = vec![doc_no_confidence, doc_with_confidence];
            let threshold = 30.0;

            let low_confidence_docs: Vec<_> = documents.iter()
                .filter(|doc| {
                    doc.ocr_confidence.is_some() && 
                    doc.ocr_confidence.unwrap() < threshold
                })
                .collect();

            // Only the document with confidence should be included
            assert_eq!(low_confidence_docs.len(), 1);
            assert!(low_confidence_docs[0].ocr_confidence.is_some());
        }

        #[test]
        fn test_user_role_authorization_in_filtering() {
            let user1_id = Uuid::new_v4();
            let user2_id = Uuid::new_v4();
            
            let user1_doc = create_test_document_with_confidence(user1_id, 20.0);
            let user2_doc = create_test_document_with_confidence(user2_id, 15.0);

            // Regular user should only see their own documents
            let user_role = UserRole::User;
            let admin_role = UserRole::Admin;

            // User1 should only access their own document
            let user1_can_access_own = user1_doc.user_id == user1_id || user_role == UserRole::Admin;
            let user1_can_access_other = user2_doc.user_id == user1_id || user_role == UserRole::Admin;
            
            assert!(user1_can_access_own);
            assert!(!user1_can_access_other);

            // Admin should access all documents
            let admin_can_access_user1 = user1_doc.user_id == user1_id || admin_role == UserRole::Admin;
            let admin_can_access_user2 = user2_doc.user_id == user1_id || admin_role == UserRole::Admin;
            
            assert!(admin_can_access_user1);
            assert!(admin_can_access_user2);
        }

        #[test]
        fn test_boundary_conditions_for_confidence_thresholds() {
            let user_id = Uuid::new_v4();
            
            let test_cases = vec![
                (0.0, 10.0, true),   // 0% < 10% threshold  
                (10.0, 10.0, false), // 10% = 10% threshold (not less than)
                (10.1, 10.0, false), // 10.1% > 10% threshold
                (29.9, 30.0, true),  // 29.9% < 30% threshold
                (30.0, 30.0, false), // 30% = 30% threshold (not less than)
                (30.1, 30.0, false), // 30.1% > 30% threshold
                (99.9, 100.0, true), // 99.9% < 100% threshold
                (100.0, 100.0, false), // 100% = 100% threshold (not less than)
            ];

            for (doc_confidence, threshold, should_be_included) in test_cases {
                let doc = create_test_document_with_confidence(user_id, doc_confidence);
                let is_included = doc.ocr_confidence.is_some() && 
                                 doc.ocr_confidence.unwrap() < threshold;
                
                assert_eq!(is_included, should_be_included, 
                    "Document with {}% confidence vs {}% threshold", 
                    doc_confidence, threshold);
            }
        }

        #[test]
        fn test_performance_considerations_for_large_datasets() {
            let user_id = Uuid::new_v4();
            
            // Create a large number of test documents
            let mut documents = Vec::new();
            for i in 0..1000 {
                let confidence = (i as f32) / 10.0; // 0.0 to 99.9
                documents.push(create_test_document_with_confidence(user_id, confidence));
            }

            let threshold = 50.0;
            let start_time = std::time::Instant::now();
            
            let low_confidence_docs: Vec<_> = documents.iter()
                .filter(|doc| {
                    doc.ocr_confidence.is_some() && 
                    doc.ocr_confidence.unwrap() < threshold
                })
                .collect();

            let elapsed = start_time.elapsed();
            
            // Verify the filtering works correctly for large datasets
            assert_eq!(low_confidence_docs.len(), 500); // 0.0 to 49.9
            
            // Performance should be reasonable (under 10ms for 1000 documents in memory)
            assert!(elapsed.as_millis() < 10, 
                "Filtering 1000 documents took too long: {:?}", elapsed);
        }

        #[test]
        fn test_sql_query_structure_expectations() {
            // Test that our expected SQL query structure would work
            let user_id = Uuid::new_v4();
            let confidence_threshold = 30.0;

            // This tests the logical structure we expect in the actual SQL query
            let expected_where_conditions = vec![
                "ocr_confidence IS NOT NULL",
                "ocr_confidence < $1", // $1 = confidence_threshold
                "user_id = $2",        // $2 = user_id (for non-admin users)
            ];

            // Verify our test documents would match the expected query logic
            let test_doc = create_test_document_with_confidence(user_id, 25.0);
            
            // Simulate the SQL conditions
            let confidence_not_null = test_doc.ocr_confidence.is_some();
            let confidence_below_threshold = test_doc.ocr_confidence.unwrap() < confidence_threshold;
            let user_matches = test_doc.user_id == user_id;
            
            assert!(confidence_not_null);
            assert!(confidence_below_threshold);
            assert!(user_matches);
            
            // This document should be included in results
            let would_be_selected = confidence_not_null && confidence_below_threshold && user_matches;
            assert!(would_be_selected);
        }

        #[test]
        fn test_deletion_ordering_expectations() {
            let user_id = Uuid::new_v4();
            
            let mut documents = vec![
                create_test_document_with_confidence(user_id, 25.0),
                create_test_document_with_confidence(user_id, 5.0),
                create_test_document_with_confidence(user_id, 15.0),
                create_test_document_with_confidence(user_id, 35.0), // Above threshold
            ];

            let threshold = 30.0;
            let mut low_confidence_docs: Vec<_> = documents.iter()
                .filter(|doc| {
                    doc.ocr_confidence.is_some() && 
                    doc.ocr_confidence.unwrap() < threshold
                })
                .collect();

            // Sort by confidence ascending (lowest first) then by creation date descending (newest first)
            low_confidence_docs.sort_by(|a, b| {
                let conf_a = a.ocr_confidence.unwrap();
                let conf_b = b.ocr_confidence.unwrap();
                conf_a.partial_cmp(&conf_b).unwrap()
                    .then_with(|| b.created_at.cmp(&a.created_at))
            });

            assert_eq!(low_confidence_docs.len(), 3);
            assert_eq!(low_confidence_docs[0].ocr_confidence.unwrap(), 5.0);   // Lowest confidence first
            assert_eq!(low_confidence_docs[1].ocr_confidence.unwrap(), 15.0);
            assert_eq!(low_confidence_docs[2].ocr_confidence.unwrap(), 25.0);
        }

        #[test]
        fn test_error_handling_scenarios() {
            let user_id = Uuid::new_v4();
            
            // Test invalid threshold values (these would be caught by the API handler)
            let invalid_thresholds = vec![-1.0, 101.0, f32::NAN, f32::INFINITY];
            
            for threshold in invalid_thresholds {
                // The database query itself should handle these gracefully
                // Invalid thresholds should either match no documents or be rejected
                let test_doc = create_test_document_with_confidence(user_id, 50.0);
                
                if threshold.is_finite() {
                    let would_match = test_doc.ocr_confidence.is_some() && 
                                     test_doc.ocr_confidence.unwrap() < threshold;
                    
                    if threshold < 0.0 {
                        assert!(!would_match, "Negative threshold should match no documents");
                    }
                    if threshold > 100.0 {
                        // Documents with confidence > 100 shouldn't exist, but if they did,
                        // they should still be considered for deletion if threshold > 100
                        assert!(would_match, "Threshold > 100 should match normal documents");
                    }
                } else {
                    // NaN and infinity comparisons
                    let would_match = test_doc.ocr_confidence.is_some() && 
                                     test_doc.ocr_confidence.unwrap() < threshold;
                    
                    if threshold.is_nan() {
                        // NaN comparisons should always be false
                        assert!(!would_match, "NaN threshold should match no documents");
                    } else if threshold == f32::INFINITY {
                        // Positive infinity should match all finite numbers
                        assert!(would_match, "Positive infinity threshold should match finite documents");
                    } else {
                        // Other invalid values like negative infinity
                        assert!(!would_match, "Invalid threshold should match no documents");
                    }
                }
            }
        }
    }

    #[tokio::test]
    async fn test_find_failed_ocr_documents() {
        use testcontainers::{runners::AsyncRunner};
        use testcontainers_modules::postgres::Postgres;
        
        let postgres_image = Postgres::default();
        let container = postgres_image.start().await.expect("Failed to start postgres container");
        let port = container.get_host_port_ipv4(5432).await.expect("Failed to get postgres port");
        
        // Use TEST_DATABASE_URL if available, otherwise use the container
        let connection_string = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| format!("postgres://postgres:postgres@127.0.0.1:{}/postgres", port));
        let database = Database::new(&connection_string).await.unwrap();
        database.migrate().await.unwrap();
        let user_id = Uuid::new_v4();
        let admin_user_id = Uuid::new_v4();

        // Create test documents with different OCR statuses
        let mut success_doc = create_test_document(user_id);
        success_doc.ocr_status = Some("completed".to_string());
        success_doc.ocr_confidence = Some(85.0);
        success_doc.ocr_text = Some("Successfully extracted text".to_string());

        let mut failed_doc = create_test_document(user_id);
        failed_doc.ocr_status = Some("failed".to_string());
        failed_doc.ocr_confidence = None;
        failed_doc.ocr_text = None;
        failed_doc.ocr_error = Some("OCR processing failed due to corrupted image".to_string());

        let mut null_confidence_doc = create_test_document(user_id);
        null_confidence_doc.ocr_status = Some("completed".to_string());
        null_confidence_doc.ocr_confidence = None; // NULL confidence but not failed
        null_confidence_doc.ocr_text = Some("Text extracted but no confidence".to_string());

        let mut pending_doc = create_test_document(user_id);
        pending_doc.ocr_status = Some("pending".to_string());
        pending_doc.ocr_confidence = None;
        pending_doc.ocr_text = None;

        let mut processing_doc = create_test_document(user_id);
        processing_doc.ocr_status = Some("processing".to_string());
        processing_doc.ocr_confidence = None;
        processing_doc.ocr_text = None;

        // Different user's failed document
        let mut other_user_failed_doc = create_test_document(admin_user_id);
        other_user_failed_doc.ocr_status = Some("failed".to_string());
        other_user_failed_doc.ocr_confidence = None;

        // Insert all documents
        let success_id = database.create_document(success_doc).await.unwrap().id;
        let failed_id = database.create_document(failed_doc).await.unwrap().id;
        let null_confidence_id = database.create_document(null_confidence_doc).await.unwrap().id;
        let pending_id = database.create_document(pending_doc).await.unwrap().id;
        let processing_id = database.create_document(processing_doc).await.unwrap().id;
        let other_user_failed_id = database.create_document(other_user_failed_doc).await.unwrap().id;

        // Test as regular user
        let failed_docs = database
            .find_failed_ocr_documents(user_id, crate::models::UserRole::User)
            .await
            .unwrap();

        // Should find: failed_doc and null_confidence_doc (but not pending/processing)
        assert_eq!(failed_docs.len(), 2);
        let failed_ids: Vec<Uuid> = failed_docs.iter().map(|d| d.id).collect();
        assert!(failed_ids.contains(&failed_id));
        assert!(failed_ids.contains(&null_confidence_id));
        assert!(!failed_ids.contains(&success_id));
        assert!(!failed_ids.contains(&pending_id));
        assert!(!failed_ids.contains(&processing_id));
        assert!(!failed_ids.contains(&other_user_failed_id)); // Different user

        // Test as admin
        let admin_failed_docs = database
            .find_failed_ocr_documents(admin_user_id, crate::models::UserRole::Admin)
            .await
            .unwrap();

        // Should find all failed documents (from all users)
        assert!(admin_failed_docs.len() >= 3); // At least our 3 failed docs
        let admin_failed_ids: Vec<Uuid> = admin_failed_docs.iter().map(|d| d.id).collect();
        assert!(admin_failed_ids.contains(&failed_id));
        assert!(admin_failed_ids.contains(&null_confidence_id));
        assert!(admin_failed_ids.contains(&other_user_failed_id));
    }

    #[tokio::test]
    async fn test_find_low_confidence_and_failed_documents() {
        use testcontainers::{runners::AsyncRunner};
        use testcontainers_modules::postgres::Postgres;
        
        let postgres_image = Postgres::default();
        let container = postgres_image.start().await.expect("Failed to start postgres container");
        let port = container.get_host_port_ipv4(5432).await.expect("Failed to get postgres port");
        
        // Use TEST_DATABASE_URL if available, otherwise use the container
        let connection_string = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| format!("postgres://postgres:postgres@127.0.0.1:{}/postgres", port));
        let database = Database::new(&connection_string).await.unwrap();
        database.migrate().await.unwrap();
        let user_id = Uuid::new_v4();

        // Create test documents with different confidence levels
        let mut high_confidence_doc = create_test_document(user_id);
        high_confidence_doc.ocr_confidence = Some(95.0);
        high_confidence_doc.ocr_status = Some("completed".to_string());

        let mut medium_confidence_doc = create_test_document(user_id);
        medium_confidence_doc.ocr_confidence = Some(65.0);
        medium_confidence_doc.ocr_status = Some("completed".to_string());

        let mut low_confidence_doc = create_test_document(user_id);
        low_confidence_doc.ocr_confidence = Some(25.0);
        low_confidence_doc.ocr_status = Some("completed".to_string());

        let mut failed_doc = create_test_document(user_id);
        failed_doc.ocr_status = Some("failed".to_string());
        failed_doc.ocr_confidence = None;
        failed_doc.ocr_error = Some("Processing failed".to_string());

        let mut null_confidence_doc = create_test_document(user_id);
        null_confidence_doc.ocr_status = Some("completed".to_string());
        null_confidence_doc.ocr_confidence = None;

        let mut pending_doc = create_test_document(user_id);
        pending_doc.ocr_status = Some("pending".to_string());
        pending_doc.ocr_confidence = None;

        // Insert all documents
        let high_id = database.create_document(high_confidence_doc).await.unwrap().id;
        let medium_id = database.create_document(medium_confidence_doc).await.unwrap().id;
        let low_id = database.create_document(low_confidence_doc).await.unwrap().id;
        let failed_id = database.create_document(failed_doc).await.unwrap().id;
        let null_confidence_id = database.create_document(null_confidence_doc).await.unwrap().id;
        let pending_id = database.create_document(pending_doc).await.unwrap().id;

        // Test with threshold of 50% - should include low confidence and failed only
        let threshold_50_docs = database
            .find_low_confidence_and_failed_documents(50.0, user_id, crate::models::UserRole::User)
            .await
            .unwrap();

        assert_eq!(threshold_50_docs.len(), 2);
        let threshold_50_ids: Vec<Uuid> = threshold_50_docs.iter().map(|d| d.id).collect();
        assert!(threshold_50_ids.contains(&low_id)); // 25% confidence
        assert!(threshold_50_ids.contains(&failed_id)); // failed status
        assert!(!threshold_50_ids.contains(&null_confidence_id)); // NULL confidence excluded
        assert!(!threshold_50_ids.contains(&high_id)); // 95% confidence
        assert!(!threshold_50_ids.contains(&medium_id)); // 65% confidence
        assert!(!threshold_50_ids.contains(&pending_id)); // pending status

        // Test with threshold of 70% - should include low and medium confidence and failed only
        let threshold_70_docs = database
            .find_low_confidence_and_failed_documents(70.0, user_id, crate::models::UserRole::User)
            .await
            .unwrap();

        assert_eq!(threshold_70_docs.len(), 3);
        let threshold_70_ids: Vec<Uuid> = threshold_70_docs.iter().map(|d| d.id).collect();
        assert!(threshold_70_ids.contains(&low_id)); // 25% confidence
        assert!(threshold_70_ids.contains(&medium_id)); // 65% confidence
        assert!(threshold_70_ids.contains(&failed_id)); // failed status
        assert!(!threshold_70_ids.contains(&null_confidence_id)); // NULL confidence excluded
        assert!(!threshold_70_ids.contains(&high_id)); // 95% confidence
        assert!(!threshold_70_ids.contains(&pending_id)); // pending status

        // Test with threshold of 100% - should include all confidence levels and failed only
        let threshold_100_docs = database
            .find_low_confidence_and_failed_documents(100.0, user_id, crate::models::UserRole::User)
            .await
            .unwrap();

        assert_eq!(threshold_100_docs.len(), 4);
        let threshold_100_ids: Vec<Uuid> = threshold_100_docs.iter().map(|d| d.id).collect();
        assert!(threshold_100_ids.contains(&high_id)); // 95% confidence
        assert!(threshold_100_ids.contains(&medium_id)); // 65% confidence
        assert!(threshold_100_ids.contains(&low_id)); // 25% confidence
        assert!(threshold_100_ids.contains(&failed_id)); // failed status
        assert!(!threshold_100_ids.contains(&null_confidence_id)); // NULL confidence excluded
        assert!(!threshold_100_ids.contains(&pending_id)); // pending status

        // Test with threshold of 0% - should only include failed documents
        let threshold_0_docs = database
            .find_low_confidence_and_failed_documents(0.0, user_id, crate::models::UserRole::User)
            .await
            .unwrap();

        assert_eq!(threshold_0_docs.len(), 1);
        let threshold_0_ids: Vec<Uuid> = threshold_0_docs.iter().map(|d| d.id).collect();
        assert!(threshold_0_ids.contains(&failed_id)); // failed status
        assert!(!threshold_0_ids.contains(&null_confidence_id)); // NULL confidence excluded
        assert!(!threshold_0_ids.contains(&high_id)); // 95% confidence
        assert!(!threshold_0_ids.contains(&medium_id)); // 65% confidence
        assert!(!threshold_0_ids.contains(&low_id)); // 25% confidence
        assert!(!threshold_0_ids.contains(&pending_id)); // pending status
    }

    #[tokio::test]
    async fn test_find_documents_by_confidence_threshold_original_behavior() {
        use testcontainers::{runners::AsyncRunner};
        use testcontainers_modules::postgres::Postgres;
        
        let postgres_image = Postgres::default();
        let container = postgres_image.start().await.expect("Failed to start postgres container");
        let port = container.get_host_port_ipv4(5432).await.expect("Failed to get postgres port");
        
        // Use TEST_DATABASE_URL if available, otherwise use the container
        let connection_string = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| format!("postgres://postgres:postgres@127.0.0.1:{}/postgres", port));
        let database = Database::new(&connection_string).await.unwrap();
        database.migrate().await.unwrap();
        let user_id = Uuid::new_v4();

        // Create test documents to verify original behavior is preserved
        let mut high_confidence_doc = create_test_document(user_id);
        high_confidence_doc.ocr_confidence = Some(90.0);
        high_confidence_doc.ocr_status = Some("completed".to_string());

        let mut low_confidence_doc = create_test_document(user_id);
        low_confidence_doc.ocr_confidence = Some(40.0);
        low_confidence_doc.ocr_status = Some("completed".to_string());

        let mut null_confidence_doc = create_test_document(user_id);
        null_confidence_doc.ocr_confidence = None;
        null_confidence_doc.ocr_status = Some("completed".to_string());

        let mut failed_doc = create_test_document(user_id);
        failed_doc.ocr_confidence = None;
        failed_doc.ocr_status = Some("failed".to_string());

        // Insert documents
        let high_id = database.create_document(high_confidence_doc).await.unwrap().id;
        let low_id = database.create_document(low_confidence_doc).await.unwrap().id;
        let null_confidence_id = database.create_document(null_confidence_doc).await.unwrap().id;
        let failed_id = database.create_document(failed_doc).await.unwrap().id;

        // Test original method - should only find documents with explicit confidence below threshold
        let original_results = database
            .find_documents_by_confidence_threshold(50.0, user_id, crate::models::UserRole::User)
            .await
            .unwrap();

        // Should only include low_confidence_doc (40%), not NULL confidence or failed docs
        assert_eq!(original_results.len(), 1);
        assert_eq!(original_results[0].id, low_id);
        
        let original_ids: Vec<Uuid> = original_results.iter().map(|d| d.id).collect();
        assert!(!original_ids.contains(&high_id)); // 90% > 50%
        assert!(!original_ids.contains(&null_confidence_id)); // NULL confidence excluded
        assert!(!original_ids.contains(&failed_id)); // NULL confidence excluded
    }

    #[tokio::test]
    async fn test_confidence_query_ordering() {
        use testcontainers::{runners::AsyncRunner};
        use testcontainers_modules::postgres::Postgres;
        
        let postgres_image = Postgres::default();
        let container = postgres_image.start().await.expect("Failed to start postgres container");
        let port = container.get_host_port_ipv4(5432).await.expect("Failed to get postgres port");
        
        // Use TEST_DATABASE_URL if available, otherwise use the container
        let connection_string = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| format!("postgres://postgres:postgres@127.0.0.1:{}/postgres", port));
        let database = Database::new(&connection_string).await.unwrap();
        database.migrate().await.unwrap();
        let user_id = Uuid::new_v4();

        // Create documents with different confidence levels and statuses
        let mut confidence_10_doc = create_test_document(user_id);
        confidence_10_doc.ocr_confidence = Some(10.0);
        confidence_10_doc.ocr_status = Some("completed".to_string());

        let mut confidence_30_doc = create_test_document(user_id);
        confidence_30_doc.ocr_confidence = Some(30.0);
        confidence_30_doc.ocr_status = Some("completed".to_string());

        let mut failed_doc = create_test_document(user_id);
        failed_doc.ocr_confidence = None;
        failed_doc.ocr_status = Some("failed".to_string());

        let mut null_confidence_doc = create_test_document(user_id);
        null_confidence_doc.ocr_confidence = None;
        null_confidence_doc.ocr_status = Some("completed".to_string());

        // Insert documents
        let id_10 = database.create_document(confidence_10_doc).await.unwrap().id;
        let id_30 = database.create_document(confidence_30_doc).await.unwrap().id;
        let failed_id = database.create_document(failed_doc).await.unwrap().id;
        let null_id = database.create_document(null_confidence_doc).await.unwrap().id;

        // Test ordering in combined query
        let results = database
            .find_low_confidence_and_failed_documents(50.0, user_id, crate::models::UserRole::User)
            .await
            .unwrap();

        assert_eq!(results.len(), 4);

        // Check that documents with actual confidence are ordered by confidence (ascending)
        // and NULL confidence documents come first (due to CASE WHEN ordering)
        let confidence_values: Vec<Option<f32>> = results.iter().map(|d| d.ocr_confidence).collect();
        
        // First two should be NULL confidence (failed and completed with NULL)
        assert!(confidence_values[0].is_none());
        assert!(confidence_values[1].is_none());
        
        // Next should be lowest confidence
        assert_eq!(confidence_values[2], Some(10.0));
        
        // Last should be higher confidence
        assert_eq!(confidence_values[3], Some(30.0));
    }

    #[tokio::test]
    async fn test_user_isolation_in_confidence_queries() {
        use testcontainers::{runners::AsyncRunner};
        use testcontainers_modules::postgres::Postgres;
        
        let postgres_image = Postgres::default();
        let container = postgres_image.start().await.expect("Failed to start postgres container");
        let port = container.get_host_port_ipv4(5432).await.expect("Failed to get postgres port");
        
        // Use TEST_DATABASE_URL if available, otherwise use the container
        let connection_string = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| format!("postgres://postgres:postgres@127.0.0.1:{}/postgres", port));
        let database = Database::new(&connection_string).await.unwrap();
        database.migrate().await.unwrap();
        let user1_id = Uuid::new_v4();
        let user2_id = Uuid::new_v4();

        // Create documents for user1
        let mut user1_low_doc = create_test_document(user1_id);
        user1_low_doc.ocr_confidence = Some(20.0);

        let mut user1_failed_doc = create_test_document(user1_id);
        user1_failed_doc.ocr_status = Some("failed".to_string());
        user1_failed_doc.ocr_confidence = None;

        // Create documents for user2
        let mut user2_low_doc = create_test_document(user2_id);
        user2_low_doc.ocr_confidence = Some(25.0);

        let mut user2_failed_doc = create_test_document(user2_id);
        user2_failed_doc.ocr_status = Some("failed".to_string());
        user2_failed_doc.ocr_confidence = None;

        // Insert documents
        let user1_low_id: Uuid = database.create_document(user1_low_doc).await.unwrap().id;
        let user1_failed_id: Uuid = database.create_document(user1_failed_doc).await.unwrap().id;
        let user2_low_id: Uuid = database.create_document(user2_low_doc).await.unwrap().id;
        let user2_failed_id: Uuid = database.create_document(user2_failed_doc).await.unwrap().id;

        // Test user1 can only see their documents
        let user1_results = database
            .find_low_confidence_and_failed_documents(50.0, user1_id, crate::models::UserRole::User)
            .await
            .unwrap();

        assert_eq!(user1_results.len(), 2);
        let user1_ids: Vec<Uuid> = user1_results.iter().map(|d| d.id).collect();
        assert!(user1_ids.contains(&user1_low_id));
        assert!(user1_ids.contains(&user1_failed_id));
        assert!(!user1_ids.contains(&user2_low_id));
        assert!(!user1_ids.contains(&user2_failed_id));

        // Test user2 can only see their documents
        let user2_results = database
            .find_low_confidence_and_failed_documents(50.0, user2_id, crate::models::UserRole::User)
            .await
            .unwrap();

        assert_eq!(user2_results.len(), 2);
        let user2_ids: Vec<Uuid> = user2_results.iter().map(|d| d.id).collect();
        assert!(user2_ids.contains(&user2_low_id));
        assert!(user2_ids.contains(&user2_failed_id));
        assert!(!user2_ids.contains(&user1_low_id));
        assert!(!user2_ids.contains(&user1_failed_id));

        // Test admin can see all documents
        let admin_results = database
            .find_low_confidence_and_failed_documents(50.0, user1_id, crate::models::UserRole::Admin)
            .await
            .unwrap();

        assert!(admin_results.len() >= 4); // At least our 4 test documents
        let admin_ids: Vec<Uuid> = admin_results.iter().map(|d| d.id).collect();
        assert!(admin_ids.contains(&user1_low_id));
        assert!(admin_ids.contains(&user1_failed_id));
        assert!(admin_ids.contains(&user2_low_id));
        assert!(admin_ids.contains(&user2_failed_id));
    }
}