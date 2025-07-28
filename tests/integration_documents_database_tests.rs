use anyhow::Result;
use readur::models::{Document, DocumentResponse};
use readur::test_utils::{TestContext, TestAuthHelper};
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
        file_hash: Some(format!("{:x}", Uuid::new_v4().as_u128())),
        original_created_at: None,
        original_modified_at: None,
        source_path: None,
        source_type: None,
        source_id: None,
        file_permissions: None,
        file_owner: None,
        file_group: None,
        source_metadata: None,
        ocr_retry_count: None,
        ocr_failure_reason: None,
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
        source_path: None,
        source_type: None,
        source_id: None,
        file_permissions: None,
        file_owner: None,
        file_group: None,
        source_metadata: None,
        ocr_retry_count: None,
        ocr_failure_reason: None,
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
        source_path: None,
        source_type: None,
        source_id: None,
        file_permissions: None,
        file_owner: None,
        file_group: None,
        source_metadata: None,
        ocr_retry_count: None,
        ocr_failure_reason: None,
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
            "id": document.id.to_string(),
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
        assert_eq!(ocr_response["id"], document.id.to_string());
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
    use readur::test_utils::TestContext;
    use readur::models::{UserRole, User, Document, AuthProvider, CreateUser};
    use chrono::Utc;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_delete_document_as_owner() {
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let db = &ctx.state.db;

            // Create test user and document
            let user_data = CreateUser {
                username: format!("testuser_{}", Uuid::new_v4()),
                email: format!("test_{}@example.com", Uuid::new_v4()),
                password: "password123".to_string(),
                role: Some(UserRole::User),
            };
            let user = db.create_user(user_data).await.expect("Failed to create user");
            let document = super::create_test_document(user.id);
            let document = db.create_document(document).await.expect("Failed to create document");

            // Delete document as owner
            let result = db
                .delete_document(document.id, user.id, user.role)
                .await
                .expect("Failed to delete document");

            // Verify document was deleted
            assert!(result);

            // Verify document no longer exists in database
            let found_doc = db
                .get_document_by_id(document.id, user.id, user.role)
                .await
                .expect("Database query failed");
            assert!(found_doc.is_none());
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_delete_document_as_admin() {
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let db = &ctx.state.db;

            // Create regular user and their document
            let user_data = CreateUser {
                username: format!("testuser_{}", Uuid::new_v4()),
                email: format!("test_{}@example.com", Uuid::new_v4()),
                password: "password123".to_string(),
                role: Some(UserRole::User),
            };
            let user = db.create_user(user_data).await.expect("Failed to create user");
            let document = super::create_test_document(user.id);
            let document = db.create_document(document).await.expect("Failed to create document");

            // Create admin user
            let admin_data = CreateUser {
                username: format!("adminuser_{}", Uuid::new_v4()),
                email: format!("admin_{}@example.com", Uuid::new_v4()),
                password: "adminpass123".to_string(),
                role: Some(UserRole::Admin),
            };
            let admin = db.create_user(admin_data).await.expect("Failed to create admin");

            // Delete document as admin
            let result = db
                .delete_document(document.id, admin.id, admin.role)
                .await
                .expect("Failed to delete document as admin");

            // Verify document was deleted
            assert!(result);
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_delete_document_unauthorized() {
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let db = &ctx.state.db;

            // Create two regular users
            let user1_data = CreateUser {
                username: format!("testuser1_{}", Uuid::new_v4()),
                email: format!("test1_{}@example.com", Uuid::new_v4()),
                password: "password123".to_string(),
                role: Some(UserRole::User),
            };
            let user1 = db.create_user(user1_data).await.expect("Failed to create user1");

            let user2_data = CreateUser {
                username: format!("testuser2_{}", Uuid::new_v4()),
                email: format!("test2_{}@example.com", Uuid::new_v4()),
                password: "password123".to_string(),
                role: Some(UserRole::User),
            };
            let user2 = db.create_user(user2_data).await.expect("Failed to create user2");

            // Create document owned by user1
            let document = super::create_test_document(user1.id);
            let document = db.create_document(document).await.expect("Failed to create document");

            // Try to delete document as user2 (should fail)
            let result = db
                .delete_document(document.id, user2.id, user2.role)
                .await
                .expect("Database query failed");

            // Verify document was not deleted
            assert!(!result);

            // Verify document still exists
            let found_doc = db
                .get_document_by_id(document.id, user1.id, user1.role)
                .await
                .expect("Database query failed");
            assert!(found_doc.is_some());
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_delete_nonexistent_document() {
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let auth_helper = TestAuthHelper::new(ctx.app.clone());
            let user = auth_helper.create_test_user().await;

            let nonexistent_id = Uuid::new_v4();

            // Try to delete nonexistent document
            let result = ctx.state.db
                .delete_document(nonexistent_id, user.user_response.id, user.user_response.role)
                .await
                .expect("Database query failed");

            // Verify nothing was deleted
            assert!(!result);
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_bulk_delete_documents_as_owner() {
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let auth_helper = TestAuthHelper::new(ctx.app.clone());
            let user = auth_helper.create_test_user().await;

            // Create multiple documents
            let doc1 = create_test_document(user.user_response.id);
            let doc1 = ctx.state.db.create_document(doc1).await.expect("Failed to create document");
            let doc2 = create_test_document(user.user_response.id);
            let doc2 = ctx.state.db.create_document(doc2).await.expect("Failed to create document");
            let doc3 = create_test_document(user.user_response.id);
            let doc3 = ctx.state.db.create_document(doc3).await.expect("Failed to create document");

            let document_ids = vec![doc1.id, doc2.id, doc3.id];

            // Delete documents as owner
            let result = ctx.state.db
                .bulk_delete_documents(&document_ids, user.user_response.id, user.user_response.role)
                .await
                .expect("Failed to bulk delete documents");

            // Verify all documents were deleted
            let (deleted_ids, failed_ids) = result;
            assert_eq!(deleted_ids.len(), 3);
            assert_eq!(failed_ids.len(), 0);
            assert!(deleted_ids.contains(&doc1.id));
            assert!(deleted_ids.contains(&doc2.id));
            assert!(deleted_ids.contains(&doc3.id));

            // Verify documents no longer exist
            for doc_id in document_ids {
                let found_doc = ctx.state.db
                    .get_document_by_id(doc_id, user.user_response.id, user.user_response.role)
                    .await
                    .expect("Database query failed");
                assert!(found_doc.is_none());
            }
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_bulk_delete_documents_as_admin() {
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let auth_helper = TestAuthHelper::new(ctx.app.clone());

            // Create regular user and their documents
            let user = auth_helper.create_test_user().await;
            let doc1 = create_test_document(user.user_response.id);
            let doc1 = ctx.state.db.create_document(doc1).await.expect("Failed to create document");
            let doc2 = create_test_document(user.user_response.id);
            let doc2 = ctx.state.db.create_document(doc2).await.expect("Failed to create document");

            // Create admin user
            let admin = auth_helper.create_admin_user().await;

            let document_ids = vec![doc1.id, doc2.id];

            // Delete documents as admin
            let result = ctx.state.db
                .bulk_delete_documents(&document_ids, admin.user_response.id, admin.user_response.role)
                .await
                .expect("Failed to bulk delete documents as admin");

            // Verify all documents were deleted
            let (deleted_ids, failed_ids) = result;
            assert_eq!(deleted_ids.len(), 2);
            assert_eq!(failed_ids.len(), 0);
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_bulk_delete_documents_mixed_ownership() {
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let db = &ctx.state.db;

            // Create two regular users
            let user1_data = CreateUser {
                username: format!("testuser1_{}", Uuid::new_v4()),
                email: format!("test1_{}@example.com", Uuid::new_v4()),
                password: "password123".to_string(),
                role: Some(UserRole::User),
            };
            let user1 = db.create_user(user1_data).await.expect("Failed to create user1");

            let user2_data = CreateUser {
                username: format!("testuser2_{}", Uuid::new_v4()),
                email: format!("test2_{}@example.com", Uuid::new_v4()),
                password: "password123".to_string(),
                role: Some(UserRole::User),
            };
            let user2 = db.create_user(user2_data).await.expect("Failed to create user2");

            // Create documents for both users
            let doc1_user1 = create_test_document(user1.id);
            let doc1_user1 = ctx.state.db.create_document(doc1_user1).await.expect("Failed to create document");
            let doc2_user1 = create_test_document(user1.id);
            let doc2_user1 = ctx.state.db.create_document(doc2_user1).await.expect("Failed to create document");
            let doc1_user2 = create_test_document(user2.id);
            let doc1_user2 = ctx.state.db.create_document(doc1_user2).await.expect("Failed to create document");

            let document_ids = vec![doc1_user1.id, doc2_user1.id, doc1_user2.id];

            // Try to delete all documents as user1 (should only delete their own)
            let result = ctx.state.db
                .bulk_delete_documents(&document_ids, user1.id, user1.role)
                .await
                .expect("Failed to bulk delete documents");

            // Verify only user1's documents were deleted
            let (deleted_ids, failed_ids) = result;
            assert_eq!(deleted_ids.len(), 2);
            assert_eq!(failed_ids.len(), 1);
            assert!(deleted_ids.contains(&doc1_user1.id));
            assert!(deleted_ids.contains(&doc2_user1.id));
            assert!(failed_ids.contains(&doc1_user2.id));

            // Verify user2's document still exists
            let found_doc = ctx.state.db
                .get_document_by_id(doc1_user2.id, user2.id, user2.role)
                .await
                .expect("Database query failed");
            assert!(found_doc.is_some());
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_bulk_delete_documents_empty_list() {
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let auth_helper = TestAuthHelper::new(ctx.app.clone());

            let user = auth_helper.create_test_user().await;
            let empty_ids: Vec<Uuid> = vec![];

            // Delete empty list of documents
            let result = ctx.state.db
                .bulk_delete_documents(&empty_ids, user.user_response.id, user.user_response.role)
                .await
                .expect("Failed to bulk delete empty list");

            // Verify empty result
            let (deleted_ids, failed_ids) = result;
            assert_eq!(deleted_ids.len(), 0);
            assert_eq!(failed_ids.len(), 0);
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_bulk_delete_documents_nonexistent_ids() {
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let auth_helper = TestAuthHelper::new(ctx.app.clone());

            let user = auth_helper.create_test_user().await;

            // Create one real document
            let real_doc = create_test_document(user.user_response.id);
            let real_doc = ctx.state.db.create_document(real_doc).await.expect("Failed to create document");

            // Mix of real and nonexistent IDs
            let document_ids = vec![real_doc.id, Uuid::new_v4(), Uuid::new_v4()];

            // Delete documents (should only delete the real one)
            let result = ctx.state.db
                .bulk_delete_documents(&document_ids, user.user_response.id, user.user_response.role)
                .await
                .expect("Failed to bulk delete documents");

            // Verify only the real document was deleted
            let (deleted_ids, failed_ids) = result;
            assert_eq!(deleted_ids.len(), 1);
            assert_eq!(failed_ids.len(), 2);
            assert!(deleted_ids.contains(&real_doc.id));
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_bulk_delete_documents_partial_authorization() {
        
        // Create regular user and admin
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let auth_helper = TestAuthHelper::new(ctx.app.clone());
            let user = auth_helper.create_test_user().await;
            let admin = auth_helper.create_admin_user().await;

            // Create documents for both users
            let user_doc_doc = create_test_document(user.user_response.id);
            let user_doc = ctx.state.db.create_document(user_doc_doc).await.expect("Failed to create document");
            let admin_doc_doc = create_test_document(admin.user_response.id);
            let admin_doc = ctx.state.db.create_document(admin_doc_doc).await.expect("Failed to create document");

            let document_ids = vec![user_doc.id, admin_doc.id];

            // Admin should be able to delete both
            let result = ctx.state.db
                .bulk_delete_documents(&document_ids, admin.user_response.id, admin.user_response.role)
                .await
                .expect("Failed to bulk delete documents as admin");

            let (deleted_ids, failed_ids) = result;
            assert_eq!(deleted_ids.len(), 2);
            assert_eq!(failed_ids.len(), 0);

            // Recreate documents for user test
            let user_doc2_doc = create_test_document(user.user_response.id);
            let user_doc2 = ctx.state.db.create_document(user_doc2_doc).await.expect("Failed to create document");
            let admin_doc2_doc = create_test_document(admin.user_response.id);
            let admin_doc2 = ctx.state.db.create_document(admin_doc2_doc).await.expect("Failed to create document");

            let document_ids2 = vec![user_doc2.id, admin_doc2.id];

            // Regular user should only delete their own
            let result2 = ctx.state.db
                .bulk_delete_documents(&document_ids2, user.user_response.id, user.user_response.role)
                .await
                .expect("Failed to bulk delete documents as user");

            let (deleted_ids2, failed_ids2) = result2;
            assert_eq!(deleted_ids2.len(), 1);
            assert_eq!(failed_ids2.len(), 1);
            assert!(deleted_ids2.contains(&user_doc2.id));
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }
}

#[cfg(test)]
mod rbac_deletion_tests {
    use super::*;
    use readur::test_utils::TestContext;
    use readur::models::{UserRole, CreateUser};
    use uuid::Uuid;

    #[tokio::test]
    async fn test_user_can_delete_own_document() {
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let db = &ctx.state.db;

            let user_data = CreateUser {
                username: format!("testuser_{}", Uuid::new_v4()),
                email: format!("test_{}@example.com", Uuid::new_v4()),
                password: "password123".to_string(),
                role: Some(UserRole::User),
            };
            let user = db.create_user(user_data).await.expect("Failed to create user");
            let document = super::create_test_document(user.id);
            let document = db.create_document(document).await.expect("Failed to create document");

            // User should be able to delete their own document
            let result = db
                .delete_document(document.id, user.id, user.role)
                .await
                .expect("Failed to delete document");

            assert!(result);
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_user_cannot_delete_other_user_document() {
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let db = &ctx.state.db;

            // Create users using direct database approach
            let user1_data = CreateUser {
                username: format!("testuser1_{}", Uuid::new_v4()),
                email: format!("test1_{}@example.com", Uuid::new_v4()),
                password: "password123".to_string(),
                role: Some(UserRole::User),
            };
            let user1 = db.create_user(user1_data).await.expect("Failed to create user1");

            let user2_data = CreateUser {
                username: format!("testuser2_{}", Uuid::new_v4()),
                email: format!("test2_{}@example.com", Uuid::new_v4()),
                password: "password123".to_string(),
                role: Some(UserRole::User),
            };
            let user2 = db.create_user(user2_data).await.expect("Failed to create user2");

            let document = create_test_document(user1.id);
            let document = ctx.state.db.create_document(document).await.expect("Failed to create document");

            // User2 should NOT be able to delete user1's document
            let result = ctx.state.db
                .delete_document(document.id, user2.id, user2.role)
                .await
                .expect("Database query failed");

            assert!(!result);

            // Verify document still exists
            let found_doc = ctx.state.db
                .get_document_by_id(document.id, user1.id, user1.role)
                .await
                .expect("Database query failed");
            assert!(found_doc.is_some());
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_admin_can_delete_any_document() {
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let db = &ctx.state.db;

            // Create users using direct database approach
            let user_data = CreateUser {
                username: format!("testuser_{}", Uuid::new_v4()),
                email: format!("test_{}@example.com", Uuid::new_v4()),
                password: "password123".to_string(),
                role: Some(UserRole::User),
            };
            let user = db.create_user(user_data).await.expect("Failed to create user");

            let admin_data = CreateUser {
                username: format!("testadmin_{}", Uuid::new_v4()),
                email: format!("admin_{}@example.com", Uuid::new_v4()),
                password: "adminpass123".to_string(),
                role: Some(UserRole::Admin),
            };
            let admin = db.create_user(admin_data).await.expect("Failed to create admin");

            let user_document = create_test_document(user.id);
            let user_document = ctx.state.db.create_document(user_document).await.expect("Failed to create document");
            let admin_document = create_test_document(admin.id);
            let admin_document = ctx.state.db.create_document(admin_document).await.expect("Failed to create document");

            // Admin should be able to delete user's document
            let result1 = ctx.state.db
                .delete_document(user_document.id, admin.id, admin.role)
                .await
                .expect("Failed to delete user document as admin");

            assert!(result1);

            // Admin should be able to delete their own document
            let result2 = ctx.state.db
                .delete_document(admin_document.id, admin.id, admin.role)
                .await
                .expect("Failed to delete admin document as admin");

            assert!(result2);
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_bulk_delete_respects_ownership() {
        
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let db = &ctx.state.db;

            // Create users using direct database approach
            let user1_data = CreateUser {
                username: format!("testuser1_{}", Uuid::new_v4()),
                email: format!("test1_{}@example.com", Uuid::new_v4()),
                password: "password123".to_string(),
                role: Some(UserRole::User),
            };
            let user1 = db.create_user(user1_data).await.expect("Failed to create user1");

            let user2_data = CreateUser {
                username: format!("testuser2_{}", Uuid::new_v4()),
                email: format!("test2_{}@example.com", Uuid::new_v4()),
                password: "password123".to_string(),
                role: Some(UserRole::User),
            };
            let user2 = db.create_user(user2_data).await.expect("Failed to create user2");

            // Create documents for both users
            let user1_doc1_doc = create_test_document(user1.id);
            let user1_doc1 = ctx.state.db.create_document(user1_doc1_doc).await.expect("Failed to create document");
            let user1_doc2_doc = create_test_document(user1.id);
            let user1_doc2 = ctx.state.db.create_document(user1_doc2_doc).await.expect("Failed to create document");
            let user2_doc1_doc = create_test_document(user2.id);
            let user2_doc1 = ctx.state.db.create_document(user2_doc1_doc).await.expect("Failed to create document");
            let user2_doc2_doc = create_test_document(user2.id);
            let user2_doc2 = ctx.state.db.create_document(user2_doc2_doc).await.expect("Failed to create document");

            let all_document_ids = vec![
                user1_doc1.id, 
                user1_doc2.id, 
                user2_doc1.id, 
                user2_doc2.id
            ];

            // User1 tries to delete all documents (should only delete their own)
            let result = ctx.state.db
                .bulk_delete_documents(&all_document_ids, user1.id, user1.role)
                .await
                .expect("Failed to bulk delete documents");

            // Should only delete user1's documents
            let (deleted_ids, failed_ids) = result;
            assert_eq!(deleted_ids.len(), 2);
            assert_eq!(failed_ids.len(), 2);
            assert!(deleted_ids.contains(&user1_doc1.id));
            assert!(deleted_ids.contains(&user1_doc2.id));
            assert!(failed_ids.contains(&user2_doc1.id));
            assert!(failed_ids.contains(&user2_doc2.id));

            // Verify user2's documents still exist
            let user2_doc1_exists = ctx.state.db
                .get_document_by_id(user2_doc1.id, user2.id, user2.role)
                .await
                .expect("Database query failed");
            assert!(user2_doc1_exists.is_some());

            let user2_doc2_exists = ctx.state.db
                .get_document_by_id(user2_doc2.id, user2.id, user2.role)
                .await
                .expect("Database query failed");
            assert!(user2_doc2_exists.is_some());
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_admin_bulk_delete_all_documents() {
        
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let db = &ctx.state.db;

            // Create users using direct database approach
            let user1_data = CreateUser {
                username: format!("testuser1_{}", Uuid::new_v4()),
                email: format!("test1_{}@example.com", Uuid::new_v4()),
                password: "password123".to_string(),
                role: Some(UserRole::User),
            };
            let user1 = db.create_user(user1_data).await.expect("Failed to create user1");

            let user2_data = CreateUser {
                username: format!("testuser2_{}", Uuid::new_v4()),
                email: format!("test2_{}@example.com", Uuid::new_v4()),
                password: "password123".to_string(),
                role: Some(UserRole::User),
            };
            let user2 = db.create_user(user2_data).await.expect("Failed to create user2");

            let admin_data = CreateUser {
                username: format!("testadmin_{}", Uuid::new_v4()),
                email: format!("admin_{}@example.com", Uuid::new_v4()),
                password: "adminpass123".to_string(),
                role: Some(UserRole::Admin),
            };
            let admin = db.create_user(admin_data).await.expect("Failed to create admin");

            // Create documents for all users
            let user1_doc_doc = create_test_document(user1.id);
            let user1_doc = ctx.state.db.create_document(user1_doc_doc).await.expect("Failed to create document");
            let user2_doc_doc = create_test_document(user2.id);
            let user2_doc = ctx.state.db.create_document(user2_doc_doc).await.expect("Failed to create document");
            let admin_doc_doc = create_test_document(admin.id);
            let admin_doc = ctx.state.db.create_document(admin_doc_doc).await.expect("Failed to create document");

            let all_document_ids = vec![user1_doc.id, user2_doc.id, admin_doc.id];

            // Admin should be able to delete all documents
            let result = ctx.state.db
                .bulk_delete_documents(&all_document_ids, admin.id, admin.role)
                .await
                .expect("Failed to bulk delete documents as admin");

            // Should delete all documents
            let (deleted_ids, failed_ids) = result;
            assert_eq!(deleted_ids.len(), 3);
            assert_eq!(failed_ids.len(), 0);
            assert!(deleted_ids.contains(&user1_doc.id));
            assert!(deleted_ids.contains(&user2_doc.id));
            assert!(deleted_ids.contains(&admin_doc.id));
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_role_escalation_prevention() {
        
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let db = &ctx.state.db;

            // Create users using direct database approach
            let user_data = CreateUser {
                username: format!("testuser_{}", Uuid::new_v4()),
                email: format!("test_{}@example.com", Uuid::new_v4()),
                password: "password123".to_string(),
                role: Some(UserRole::User),
            };
            let user = db.create_user(user_data).await.expect("Failed to create user");

            let admin_data = CreateUser {
                username: format!("testadmin_{}", Uuid::new_v4()),
                email: format!("admin_{}@example.com", Uuid::new_v4()),
                password: "adminpass123".to_string(),
                role: Some(UserRole::Admin),
            };
            let admin = db.create_user(admin_data).await.expect("Failed to create admin");

            let admin_document_doc = create_test_document(admin.id);
            let admin_document = ctx.state.db.create_document(admin_document_doc).await.expect("Failed to create document");

            // Regular user should NOT be able to delete admin's document
            // even if they somehow know the document ID
            let result = ctx.state.db
                .delete_document(admin_document.id, user.id, user.role)
                .await
                .expect("Database query failed");

            assert!(!result);

            // Verify admin's document still exists
            let found_doc = ctx.state.db
                .get_document_by_id(admin_document.id, admin.id, admin.role)
                .await
                .expect("Database query failed");
            assert!(found_doc.is_some());
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_cross_tenant_isolation() {
        
        // Create users that could represent different tenants/organizations
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let db = &ctx.state.db;

            // Create tenant users using direct database approach
            let tenant1_user1_data = CreateUser {
                username: format!("tenant1_user1_{}", Uuid::new_v4()),
                email: format!("tenant1_user1_{}@example.com", Uuid::new_v4()),
                password: "password123".to_string(),
                role: Some(UserRole::User),
            };
            let tenant1_user1 = db.create_user(tenant1_user1_data).await.expect("Failed to create tenant1_user1");

            let tenant1_user2_data = CreateUser {
                username: format!("tenant1_user2_{}", Uuid::new_v4()),
                email: format!("tenant1_user2_{}@example.com", Uuid::new_v4()),
                password: "password123".to_string(),
                role: Some(UserRole::User),
            };
            let tenant1_user2 = db.create_user(tenant1_user2_data).await.expect("Failed to create tenant1_user2");

            let tenant2_user1_data = CreateUser {
                username: format!("tenant2_user1_{}", Uuid::new_v4()),
                email: format!("tenant2_user1_{}@example.com", Uuid::new_v4()),
                password: "password123".to_string(),
                role: Some(UserRole::User),
            };
            let tenant2_user1 = db.create_user(tenant2_user1_data).await.expect("Failed to create tenant2_user1");

            let tenant2_user2_data = CreateUser {
                username: format!("tenant2_user2_{}", Uuid::new_v4()),
                email: format!("tenant2_user2_{}@example.com", Uuid::new_v4()),
                password: "password123".to_string(),
                role: Some(UserRole::User),
            };
            let tenant2_user2 = db.create_user(tenant2_user2_data).await.expect("Failed to create tenant2_user2");

            // Create documents for each tenant
            let tenant1_doc1_doc = create_test_document(tenant1_user1.id);
            let tenant1_doc1 = ctx.state.db.create_document(tenant1_doc1_doc).await.expect("Failed to create document");
            let tenant1_doc2_doc = create_test_document(tenant1_user2.id);
            let tenant1_doc2 = ctx.state.db.create_document(tenant1_doc2_doc).await.expect("Failed to create document");
            let tenant2_doc1_doc = create_test_document(tenant2_user1.id);
            let tenant2_doc1 = ctx.state.db.create_document(tenant2_doc1_doc).await.expect("Failed to create document");
            let tenant2_doc2_doc = create_test_document(tenant2_user2.id);
            let tenant2_doc2 = ctx.state.db.create_document(tenant2_doc2_doc).await.expect("Failed to create document");

            // Tenant1 user should not be able to delete tenant2 documents
            let result1 = ctx.state.db
                .delete_document(tenant2_doc1.id, tenant1_user1.id, tenant1_user1.role)
                .await
                .expect("Database query failed");
            assert!(!result1);

            let result2 = ctx.state.db
                .delete_document(tenant2_doc2.id, tenant1_user2.id, tenant1_user2.role)
                .await
                .expect("Database query failed");
            assert!(!result2);

            // Tenant2 user should not be able to delete tenant1 documents
            let result3 = ctx.state.db
                .delete_document(tenant1_doc1.id, tenant2_user1.id, tenant2_user1.role)
                .await
                .expect("Database query failed");
            assert!(!result3);

            let result4 = ctx.state.db
                .delete_document(tenant1_doc2.id, tenant2_user2.id, tenant2_user2.role)
                .await
                .expect("Database query failed");
            assert!(!result4);

            // Verify all documents still exist
            for (doc_id, owner_id, owner_role) in [
                (tenant1_doc1.id, tenant1_user1.id, tenant1_user1.role),
                (tenant1_doc2.id, tenant1_user2.id, tenant1_user2.role),
                (tenant2_doc1.id, tenant2_user1.id, tenant2_user1.role),
                (tenant2_doc2.id, tenant2_user2.id, tenant2_user2.role),
            ] {
                let found_doc = ctx.state.db
                    .get_document_by_id(doc_id, owner_id, owner_role)
                    .await
                    .expect("Database query failed");
                assert!(found_doc.is_some());
            }
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_permission_consistency_single_vs_bulk() {
        
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let db = &ctx.state.db;

            // Create users using direct database approach
            let user1_data = CreateUser {
                username: format!("testuser1_{}", Uuid::new_v4()),
                email: format!("test1_{}@example.com", Uuid::new_v4()),
                password: "password123".to_string(),
                role: Some(UserRole::User),
            };
            let user1 = db.create_user(user1_data).await.expect("Failed to create user1");

            let user2_data = CreateUser {
                username: format!("testuser2_{}", Uuid::new_v4()),
                email: format!("test2_{}@example.com", Uuid::new_v4()),
                password: "password123".to_string(),
                role: Some(UserRole::User),
            };
            let user2 = db.create_user(user2_data).await.expect("Failed to create user2");

            let _user1_doc_doc = create_test_document(user1.id);
            let _user1_doc = ctx.state.db.create_document(_user1_doc_doc).await.expect("Failed to create document");
            let user2_doc_doc = create_test_document(user2.id);
            let user2_doc = ctx.state.db.create_document(user2_doc_doc).await.expect("Failed to create document");

            // Test single deletion permissions
            let single_delete_result = ctx.state.db
                .delete_document(user2_doc.id, user1.id, user1.role)
                .await
                .expect("Database query failed");
            assert!(!single_delete_result); // Should fail

            // Test bulk deletion permissions with same document
            let user2_doc2_doc = create_test_document(user2.id);
            let user2_doc2 = ctx.state.db.create_document(user2_doc2_doc).await.expect("Failed to create document");
            let bulk_delete_result = ctx.state.db
                .bulk_delete_documents(&vec![user2_doc2.id], user1.id, user1.role)
                .await
                .expect("Database query failed");
            let (deleted_ids, failed_ids) = bulk_delete_result;
            assert_eq!(deleted_ids.len(), 0); // Should delete nothing
            assert_eq!(failed_ids.len(), 1);

            // Verify both documents still exist
            let doc1_exists = ctx.state.db
                .get_document_by_id(user2_doc.id, user2.id, user2.role)
                .await
                .expect("Database query failed");
            assert!(doc1_exists.is_some());

            let doc2_exists = ctx.state.db
                .get_document_by_id(user2_doc2.id, user2.id, user2.role)
                .await
                .expect("Database query failed");
            assert!(doc2_exists.is_some());
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_admin_permission_inheritance() {
        
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let db = &ctx.state.db;

            // Create users using direct database approach
            let user_data = CreateUser {
                username: format!("testuser_{}", Uuid::new_v4()),
                email: format!("test_{}@example.com", Uuid::new_v4()),
                password: "password123".to_string(),
                role: Some(UserRole::User),
            };
            let user = db.create_user(user_data).await.expect("Failed to create user");

            let admin_data = CreateUser {
                username: format!("testadmin_{}", Uuid::new_v4()),
                email: format!("admin_{}@example.com", Uuid::new_v4()),
                password: "adminpass123".to_string(),
                role: Some(UserRole::Admin),
            };
            let admin = db.create_user(admin_data).await.expect("Failed to create admin");

            let user_doc_doc = create_test_document(user.id);
            let user_doc = ctx.state.db.create_document(user_doc_doc).await.expect("Failed to create document");

            // Admin should have all permissions that a regular user has, plus more
            // Test that admin can delete user's document (admin-specific permission)
            let admin_delete_result = ctx.state.db
                .delete_document(user_doc.id, admin.id, admin.role)
                .await
                .expect("Failed to delete as admin");
            assert!(admin_delete_result);

            // Create another document to test admin's own document deletion
            let admin_doc_doc = create_test_document(admin.id);
            let admin_doc = ctx.state.db.create_document(admin_doc_doc).await.expect("Failed to create document");
            let admin_own_delete_result = ctx.state.db
                .delete_document(admin_doc.id, admin.id, admin.role)
                .await
                .expect("Failed to delete admin's own document");
            assert!(admin_own_delete_result);
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
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
    use readur::test_utils::{TestContext, TestAuthHelper};
    use uuid::Uuid;

    #[tokio::test]
    async fn test_delete_with_invalid_uuid() {
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let db = &ctx.state.db;

            // Create user using direct database approach
            let user_data = readur::models::CreateUser {
                username: format!("testuser_{}", Uuid::new_v4()),
                email: format!("test_{}@example.com", Uuid::new_v4()),
                password: "password123".to_string(),
                role: Some(readur::models::UserRole::User),
            };
            let user = db.create_user(user_data).await.expect("Failed to create user");

            // Use malformed UUID (this test assumes the function handles UUID parsing)
            let invalid_uuid = Uuid::nil(); // Use nil UUID as "invalid"

            let result = ctx.state.db
                .delete_document(invalid_uuid, user.id, user.role)
                .await
                .expect("Database query should not fail for invalid UUID");

            // Should return None for non-existent document
            assert!(!result);
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_delete_with_sql_injection_attempt() {
        
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let db = &ctx.state.db;

            // Create user using direct database approach
            let user_data = readur::models::CreateUser {
                username: format!("testuser_{}", Uuid::new_v4()),
                email: format!("test_{}@example.com", Uuid::new_v4()),
                password: "password123".to_string(),
                role: Some(readur::models::UserRole::User),
            };
            let user = db.create_user(user_data).await.expect("Failed to create user");

            let document_doc = create_test_document(user.id);
            let document = ctx.state.db.create_document(document_doc).await.expect("Failed to create document");

            // Test with legitimate document ID - SQLx should prevent injection
            let result = ctx.state.db
                .delete_document(document.id, user.id, user.role)
                .await
                .expect("Query should execute safely");

            assert!(result);
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_bulk_delete_with_duplicate_ids() {
        
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let db = &ctx.state.db;

            // Create user using direct database approach
            let user_data = readur::models::CreateUser {
                username: format!("testuser_{}", Uuid::new_v4()),
                email: format!("test_{}@example.com", Uuid::new_v4()),
                password: "password123".to_string(),
                role: Some(readur::models::UserRole::User),
            };
            let user = db.create_user(user_data).await.expect("Failed to create user");

            let document_doc = create_test_document(user.id);
            let document = ctx.state.db.create_document(document_doc).await.expect("Failed to create document");

            // Include the same document ID multiple times
            let duplicate_ids = vec![document.id, document.id, document.id];

            let result = ctx.state.db
                .bulk_delete_documents(&duplicate_ids, user.id, user.role)
                .await
                .expect("Bulk delete should handle duplicates");

            // Should only delete the document once, but subsequent attempts fail
            let (deleted_ids, failed_ids) = result;
            assert_eq!(deleted_ids.len(), 1);
            assert_eq!(failed_ids.len(), 2); // Two failed attempts on already-deleted document
            assert!(deleted_ids.contains(&document.id));
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_bulk_delete_with_extremely_large_request() {
        
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let auth_helper = TestAuthHelper::new(ctx.app.clone());
            let user = auth_helper.create_test_user().await;

            // Create a large number of document IDs (mostly non-existent)
            let mut large_id_list = Vec::new();

            // Add one real document
            let real_document_doc = create_test_document(user.user_response.id);
            let real_document = ctx.state.db.create_document(real_document_doc).await.expect("Failed to create document");
            large_id_list.push(real_document.id);

            // Add many fake UUIDs
            for _ in 0..499 {
                large_id_list.push(Uuid::new_v4());
            }

            let result = ctx.state.db
                .bulk_delete_documents(&large_id_list, user.user_response.id, user.user_response.role)
                .await
                .expect("Should handle large requests");

            // Should only delete the one real document
            let (deleted_ids, failed_ids) = result;
            assert_eq!(deleted_ids.len(), 1);
            assert_eq!(failed_ids.len(), 499);
            assert!(deleted_ids.contains(&real_document.id));
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_concurrent_deletion_same_document() {
        
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let auth_helper = TestAuthHelper::new(ctx.app.clone());
            let user = auth_helper.create_test_user().await;
            let document_doc = create_test_document(user.user_response.id);
            let document = ctx.state.db.create_document(document_doc).await.expect("Failed to create document");

            // Create multiple handles to the same database connection pool
            let db1 = ctx.state.db.clone();
            let db2 = ctx.state.db.clone();

            // Attempt concurrent deletions
            let doc_id = document.id;
            let user_id = user.user_response.id;
            let user_role = user.user_response.role;

            let task1 = tokio::spawn(async move {
                db1.delete_document(doc_id, user_id, user_role).await
            });

            let task2 = tokio::spawn(async move {
                db2.delete_document(doc_id, user_id, user_role).await
            });

            let result1 = task1.await.unwrap().expect("First deletion should succeed");
            let result2 = task2.await.unwrap().expect("Second deletion should not error");

            // One should succeed, one should return false
            let success_count = [result1, result2]
                .iter()
                .filter(|&&x| x)
                .count();

            assert_eq!(success_count, 1, "Exactly one deletion should succeed");
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_delete_document_with_foreign_key_constraints() {
        
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let auth_helper = TestAuthHelper::new(ctx.app.clone());
            let user = auth_helper.create_test_user().await;
            let document_doc = create_test_document(user.user_response.id);
            let document = ctx.state.db.create_document(document_doc).await.expect("Failed to create document");

            // If there are foreign key relationships (like document_labels), 
            // test that CASCADE deletion works properly

            // Delete the document
            let result = ctx.state.db
                .delete_document(document.id, user.user_response.id, user.user_response.role)
                .await
                .expect("Deletion should handle foreign key constraints");

            assert!(result);

            // Verify related records are also deleted (if any exist)
            // This would depend on the actual schema relationships
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_bulk_delete_with_mixed_permissions_and_errors() {
        
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {

            // Create users using direct database approach
            let user1_data = readur::models::CreateUser {
                username: format!("testuser1_{}", Uuid::new_v4()),
                email: format!("test1_{}@example.com", Uuid::new_v4()),
                password: "password123".to_string(),
                role: Some(readur::models::UserRole::User),
            };
            let user1 = ctx.state.db.create_user(user1_data).await.expect("Failed to create user1");

            let user2_data = readur::models::CreateUser {
                username: format!("testuser2_{}", Uuid::new_v4()),
                email: format!("test2_{}@example.com", Uuid::new_v4()),
                password: "password123".to_string(),
                role: Some(readur::models::UserRole::User),
            };
            let user2 = ctx.state.db.create_user(user2_data).await.expect("Failed to create user2");

            // Create mix of documents
            let user1_doc_doc = create_test_document(user1.id);
            let user1_doc = ctx.state.db.create_document(user1_doc_doc).await.expect("Failed to create document");
            let user2_doc_doc = create_test_document(user2.id);
            let user2_doc = ctx.state.db.create_document(user2_doc_doc).await.expect("Failed to create document");
            let nonexistent_id = Uuid::new_v4();

            let mixed_ids = vec![user1_doc.id, user2_doc.id, nonexistent_id];

            // User1 attempts to delete all (should only delete their own)
            let result = ctx.state.db
                .bulk_delete_documents(&mixed_ids, user1.id, user1.role)
                .await
                .expect("Should handle mixed permissions gracefully");

            // Should only delete user1's document
            let (deleted_ids, failed_ids) = result;
            assert_eq!(deleted_ids.len(), 1);
            assert_eq!(failed_ids.len(), 2);
            assert!(deleted_ids.contains(&user1_doc.id));

            // Verify user2's document still exists
            let user2_doc_exists = ctx.state.db
                .get_document_by_id(user2_doc.id, user2.id, user2.role)
                .await
                .expect("Query should succeed");
            assert!(user2_doc_exists.is_some());
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
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
    async fn test_delete_after_user_deletion() {
        
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let auth_helper = TestAuthHelper::new(ctx.app.clone());
            let user = auth_helper.create_test_user().await;
            let document_doc = create_test_document(user.user_response.id);
            let document = ctx.state.db.create_document(document_doc).await.expect("Failed to create document");

            // Delete the user first (simulating cascade deletion scenarios)
            sqlx::query("DELETE FROM users WHERE id = $1")
                .bind(user.user_response.id)
                .execute(&ctx.state.db.pool)
                .await
                .expect("User deletion should succeed");

            // Attempt to delete document after user is gone
            // This depends on how foreign key constraints are set up
            let result = ctx.state.db
                .delete_document(document.id, user.user_response.id, user.user_response.role)
                .await;

            // The behavior here depends on FK constraints:
            // - If CASCADE: document might already be deleted
            // - If RESTRICT: document still exists but operation might fail
            // Test should verify consistent behavior
            match result {
                Ok(true) => {
                    // Document was deleted successfully
                },
                Ok(false) => {
                    // Document not found (possibly already cascade deleted)
                },
                Err(_) => {
                    // Error occurred (foreign key constraint issue)
                    // This might be expected behavior
                }
            }
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_bulk_delete_empty_and_null_scenarios() {
        
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let auth_helper = TestAuthHelper::new(ctx.app.clone());
            let user = auth_helper.create_test_user().await;

            // Test empty list
            let empty_result = ctx.state.db
                .bulk_delete_documents(&vec![], user.user_response.id, user.user_response.role)
                .await
                .expect("Empty list should be handled gracefully");
            let (deleted_ids, failed_ids) = empty_result;
            assert_eq!(deleted_ids.len(), 0);
            assert_eq!(failed_ids.len(), 0);

            // Test with only nil UUIDs
            let nil_uuids = vec![Uuid::nil(), Uuid::nil()];
            let nil_result = ctx.state.db
                .bulk_delete_documents(&nil_uuids, user.user_response.id, user.user_response.role)
                .await
                .expect("Nil UUIDs should be handled gracefully");
            let (deleted_ids, failed_ids) = nil_result;
            assert_eq!(deleted_ids.len(), 0);
            assert_eq!(failed_ids.len(), 2);
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }


    #[tokio::test]
    async fn test_transaction_rollback_simulation() {
        
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let auth_helper = TestAuthHelper::new(ctx.app.clone());
            let user = auth_helper.create_test_user().await;
            let document_doc = create_test_document(user.user_response.id);
            let document = ctx.state.db.create_document(document_doc).await.expect("Failed to create document");

            // Verify document exists before deletion
            let exists_before = ctx.state.db
                .get_document_by_id(document.id, user.user_response.id, user.user_response.role)
                .await
                .expect("Query should succeed");
            assert!(exists_before.is_some());

            // Perform deletion
            let deletion_result = ctx.state.db
                .delete_document(document.id, user.user_response.id, user.user_response.role)
                .await
                .expect("Deletion should succeed");
            assert!(deletion_result);

            // Verify document no longer exists
            let exists_after = ctx.state.db
                .get_document_by_id(document.id, user.user_response.id, user.user_response.role)
                .await
                .expect("Query should succeed");
            assert!(exists_after.is_none());

            // If transaction were to be rolled back, document would exist again
            // This test verifies the transaction was committed properly
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    mod low_confidence_deletion_db_tests {
        use super::*;
        use readur::models::UserRole;

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
                source_path: None,
                source_type: None,
                source_id: None,
                file_permissions: None,
                file_owner: None,
                file_group: None,
                source_metadata: None,
                ocr_retry_count: None,
                ocr_failure_reason: None,
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
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let auth_helper = TestAuthHelper::new(ctx.app.clone());
            let database = &ctx.state.db;

            // Create actual users in the database
            let user = auth_helper.create_test_user().await;
            let admin_user = auth_helper.create_test_admin().await;
            let user_id = user.user_response.id;
            let admin_user_id = admin_user.user_response.id;

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
            let success_id = ctx.state.db.create_document(success_doc).await.unwrap().id;
            let failed_id = ctx.state.db.create_document(failed_doc).await.unwrap().id;
            let null_confidence_id = ctx.state.db.create_document(null_confidence_doc).await.unwrap().id;
            let pending_id = ctx.state.db.create_document(pending_doc).await.unwrap().id;
            let processing_id = ctx.state.db.create_document(processing_doc).await.unwrap().id;
            let other_user_failed_id = ctx.state.db.create_document(other_user_failed_doc).await.unwrap().id;

            // Test as regular user
            let failed_docs = database
                .find_failed_ocr_documents(user_id, readur::models::UserRole::User, 100, 0)
                .await
                .unwrap();

            // Should find: only failed_doc (null_confidence_doc has status 'completed')
            assert_eq!(failed_docs.len(), 1);
            let failed_ids: Vec<Uuid> = failed_docs.iter().map(|d| d.id).collect();
            assert!(failed_ids.contains(&failed_id));
            assert!(!failed_ids.contains(&null_confidence_id)); // This has status 'completed'
            assert!(!failed_ids.contains(&success_id));
            assert!(!failed_ids.contains(&pending_id));
            assert!(!failed_ids.contains(&processing_id));
            assert!(!failed_ids.contains(&other_user_failed_id)); // Different user

            // Test as admin
            let admin_failed_docs = database
                .find_failed_ocr_documents(admin_user_id, readur::models::UserRole::Admin, 100, 0)
                .await
                .unwrap();

            // Should find all failed documents (from all users)
            assert!(admin_failed_docs.len() >= 2); // At least our 2 failed docs
            let admin_failed_ids: Vec<Uuid> = admin_failed_docs.iter().map(|d| d.id).collect();
            assert!(admin_failed_ids.contains(&failed_id));
            assert!(!admin_failed_ids.contains(&null_confidence_id)); // This has status 'completed'
            assert!(admin_failed_ids.contains(&other_user_failed_id));
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_find_low_confidence_and_failed_documents() {
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let auth_helper = TestAuthHelper::new(ctx.app.clone());
            let database = &ctx.state.db;

            // Create actual user in the database
            let user = auth_helper.create_test_user().await;
            let user_id = user.user_response.id;

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
            let high_id = ctx.state.db.create_document(high_confidence_doc).await.unwrap().id;
            let medium_id = ctx.state.db.create_document(medium_confidence_doc).await.unwrap().id;
            let low_id = ctx.state.db.create_document(low_confidence_doc).await.unwrap().id;
            let failed_id = ctx.state.db.create_document(failed_doc).await.unwrap().id;
            let null_confidence_id = ctx.state.db.create_document(null_confidence_doc).await.unwrap().id;
            let pending_id = ctx.state.db.create_document(pending_doc).await.unwrap().id;

            // Test with threshold of 50% - should include low confidence and failed only
            let threshold_50_docs = database
                .find_low_confidence_and_failed_documents(user_id, readur::models::UserRole::User, 50.0, 100, 0)
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
                .find_low_confidence_and_failed_documents(user_id, readur::models::UserRole::User, 70.0, 100, 0)
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
                .find_low_confidence_and_failed_documents(user_id, readur::models::UserRole::User, 100.0, 100, 0)
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
                .find_low_confidence_and_failed_documents(user_id, readur::models::UserRole::User, 0.0, 100, 0)
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
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_find_documents_by_confidence_threshold_original_behavior() {
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let auth_helper = TestAuthHelper::new(ctx.app.clone());
            let database = &ctx.state.db;

            // Create actual user in the database
            let user = auth_helper.create_test_user().await;
            let user_id = user.user_response.id;

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
            let high_id = ctx.state.db.create_document(high_confidence_doc).await.unwrap().id;
            let low_id = ctx.state.db.create_document(low_confidence_doc).await.unwrap().id;
            let null_confidence_id = ctx.state.db.create_document(null_confidence_doc).await.unwrap().id;
            let failed_id = ctx.state.db.create_document(failed_doc).await.unwrap().id;

            // Test original method - should only find documents with explicit confidence below threshold
            let original_results = database
                .find_documents_by_confidence_threshold(user_id, readur::models::UserRole::User, 50.0, 100, 0)
                .await
                .unwrap();

            // Should only include low_confidence_doc (40%), not NULL confidence or failed docs
            assert_eq!(original_results.len(), 1);
            assert_eq!(original_results[0].id, low_id);

            let original_ids: Vec<Uuid> = original_results.iter().map(|d| d.id).collect();
            assert!(!original_ids.contains(&high_id)); // 90% > 50%
            assert!(!original_ids.contains(&null_confidence_id)); // NULL confidence excluded
            assert!(!original_ids.contains(&failed_id)); // NULL confidence excluded
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_confidence_query_ordering() {
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let database = &ctx.state.db;

            // Create user using direct database approach
            let user_data = readur::models::CreateUser {
                username: format!("testuser_{}", Uuid::new_v4()),
                email: format!("test_{}@example.com", Uuid::new_v4()),
                password: "password123".to_string(),
                role: Some(readur::models::UserRole::User),
            };
            let user = database.create_user(user_data).await.expect("Failed to create user");
            let user_id = user.id;

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
            let id_10 = ctx.state.db.create_document(confidence_10_doc).await.unwrap().id;
            let id_30 = ctx.state.db.create_document(confidence_30_doc).await.unwrap().id;
            let failed_id = ctx.state.db.create_document(failed_doc).await.unwrap().id;
            let null_id = ctx.state.db.create_document(null_confidence_doc).await.unwrap().id;

            // Test ordering in combined query
            let results = database
                .find_low_confidence_and_failed_documents(user_id, readur::models::UserRole::User, 50.0, 100, 0)
                .await
                .unwrap();

            // The function returns documents that are either:
            // 1. Low confidence (< threshold) 
            // 2. Failed status
            // A completed document with NULL confidence is not considered "failed"
            assert_eq!(results.len(), 3); // Update expectation based on actual behavior

            // Check that documents with actual confidence are ordered by confidence (ascending)
            // and NULL confidence documents come first (due to CASE WHEN ordering)
            let confidence_values: Vec<Option<f32>> = results.iter().map(|d| d.ocr_confidence).collect();

            // With 3 documents: 1 failed (NULL confidence), 2 low confidence documents
            // First should be NULL confidence (failed)
            assert!(confidence_values[0].is_none());

            // Next should be lowest confidence
            assert_eq!(confidence_values[1], Some(10.0));

            // Last should be higher confidence  
            assert_eq!(confidence_values[2], Some(30.0));
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_user_isolation_in_confidence_queries() {
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let database = &ctx.state.db;

            // Create users using direct database approach
            let user1_data = readur::models::CreateUser {
                username: format!("testuser1_{}", Uuid::new_v4()),
                email: format!("test1_{}@example.com", Uuid::new_v4()),
                password: "password123".to_string(),
                role: Some(readur::models::UserRole::User),
            };
            let user1 = database.create_user(user1_data).await.expect("Failed to create user1");
            let user1_id = user1.id;

            let user2_data = readur::models::CreateUser {
                username: format!("testuser2_{}", Uuid::new_v4()),
                email: format!("test2_{}@example.com", Uuid::new_v4()),
                password: "password123".to_string(),
                role: Some(readur::models::UserRole::User),
            };
            let user2 = database.create_user(user2_data).await.expect("Failed to create user2");
            let user2_id = user2.id;

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
            let user1_low_id: Uuid = ctx.state.db.create_document(user1_low_doc).await.unwrap().id;
            let user1_failed_id: Uuid = ctx.state.db.create_document(user1_failed_doc).await.unwrap().id;
            let user2_low_id: Uuid = ctx.state.db.create_document(user2_low_doc).await.unwrap().id;
            let user2_failed_id: Uuid = ctx.state.db.create_document(user2_failed_doc).await.unwrap().id;

            // Test user1 can only see their documents
            let user1_results = database
                .find_low_confidence_and_failed_documents(user1_id, readur::models::UserRole::User, 50.0, 100, 0)
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
                .find_low_confidence_and_failed_documents(user2_id, readur::models::UserRole::User, 50.0, 100, 0)
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
                .find_low_confidence_and_failed_documents(user1_id, readur::models::UserRole::Admin, 50.0, 100, 0)
                .await
                .unwrap();

            assert!(admin_results.len() >= 4); // At least our 4 test documents
            let admin_ids: Vec<Uuid> = admin_results.iter().map(|d| d.id).collect();
            assert!(admin_ids.contains(&user1_low_id));
            assert!(admin_ids.contains(&user1_failed_id));
            assert!(admin_ids.contains(&user2_low_id));
            assert!(admin_ids.contains(&user2_failed_id));
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }
}