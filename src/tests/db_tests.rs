#[cfg(test)]
mod tests {
    use crate::db::Database;
    use crate::models::{CreateUser, Document, SearchRequest};
    use chrono::Utc;
    use uuid::Uuid;

    async fn create_test_db() -> Database {
        // Use an in-memory database URL for testing
        // This will require PostgreSQL to be running for integration tests
        let db_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/readur_test".to_string());
        
        let db = Database::new(&db_url).await.expect("Failed to connect to test database");
        
        // Run migrations for test database
        db.migrate().await.expect("Failed to migrate test database");
        
        db
    }

    fn create_test_user_data(suffix: &str) -> CreateUser {
        CreateUser {
            username: format!("testuser_{}", suffix),
            email: format!("test_{}@example.com", suffix),
            password: "password123".to_string(),
            role: Some(crate::models::UserRole::User),
        }
    }

    fn create_test_document(user_id: Uuid) -> Document {
        Document {
            id: Uuid::new_v4(),
            filename: "test.pdf".to_string(),
            original_filename: "test.pdf".to_string(),
            file_path: "/path/to/test.pdf".to_string(),
            file_size: 1024,
            mime_type: "application/pdf".to_string(),
            content: Some("Test content".to_string()),
            ocr_text: Some("OCR extracted text".to_string()),
            ocr_confidence: Some(95.0),
            ocr_word_count: Some(10),
            ocr_processing_time_ms: Some(800),
            ocr_status: Some("completed".to_string()),
            ocr_error: None,
            ocr_completed_at: Some(Utc::now()),
            tags: vec!["test".to_string(), "document".to_string()],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            user_id,
            file_hash: Some("abcd1234567890123456789012345678901234567890123456789012345678".to_string()),
            original_created_at: None,
            original_modified_at: None,
            source_metadata: None,
            ocr_retry_count: None,
            ocr_failure_reason: None,
        }
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL database"]
    async fn test_create_user() {
        let db = create_test_db().await;
        let user_data = create_test_user_data("1");
        
        let result = db.create_user(user_data).await;
        assert!(result.is_ok());
        
        let user = result.unwrap();
        assert_eq!(user.username, "testuser_1");
        assert_eq!(user.email, "test@example.com");
        assert!(user.password_hash.is_some());
        assert_ne!(user.password_hash.as_ref().unwrap(), "password123"); // Should be hashed
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL database"]
    async fn test_get_user_by_username() {
        let db = create_test_db().await;
        let user_data = create_test_user_data("1");
        
        let created_user = db.create_user(user_data).await.unwrap();
        
        let result = db.get_user_by_username("testuser_1").await;
        assert!(result.is_ok());
        
        let found_user = result.unwrap();
        assert!(found_user.is_some());
        
        let user = found_user.unwrap();
        assert_eq!(user.id, created_user.id);
        assert_eq!(user.username, "testuser_1");
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL database"]
    async fn test_get_user_by_username_not_found() {
        let db = create_test_db().await;
        
        let result = db.get_user_by_username("nonexistent").await;
        assert!(result.is_ok());
        
        let found_user = result.unwrap();
        assert!(found_user.is_none());
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL database"]
    async fn test_create_document() {
        let db = create_test_db().await;
        let user_data = create_test_user_data("1");
        let user = db.create_user(user_data).await.unwrap();
        
        let document = create_test_document(user.id);
        
        let result = db.create_document(document.clone()).await;
        assert!(result.is_ok());
        
        let created_doc = result.unwrap();
        assert_eq!(created_doc.filename, document.filename);
        assert_eq!(created_doc.user_id, user.id);
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL database"]
    async fn test_get_documents_by_user() {
        let db = create_test_db().await;
        let user_data = create_test_user_data("1");
        let user = db.create_user(user_data).await.unwrap();
        
        let document1 = create_test_document(user.id);
        let document2 = create_test_document(user.id);
        
        db.create_document(document1).await.unwrap();
        db.create_document(document2).await.unwrap();
        
        let result = db.get_documents_by_user(user.id, 10, 0).await;
        assert!(result.is_ok());
        
        let documents = result.unwrap();
        assert_eq!(documents.len(), 2);
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL database"]
    async fn test_search_documents() {
        let db = create_test_db().await;
        let user_data = create_test_user_data("1");
        let user = db.create_user(user_data).await.unwrap();
        
        let mut document = create_test_document(user.id);
        document.content = Some("This is a searchable document".to_string());
        document.ocr_text = Some("OCR searchable text".to_string());
        
        db.create_document(document).await.unwrap();
        
        let search_request = SearchRequest {
            query: "searchable".to_string(),
            tags: None,
            mime_types: None,
            limit: Some(10),
            offset: Some(0),
            include_snippets: Some(true),
            snippet_length: Some(200),
            search_mode: None,
        };
        
        let result = db.search_documents(user.id, search_request).await;
        assert!(result.is_ok());
        
        let (documents, total) = result.unwrap();
        assert_eq!(documents.len(), 1);
        assert_eq!(total, 1);
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL database"]
    async fn test_update_document_ocr() {
        let db = create_test_db().await;
        let user_data = create_test_user_data("1");
        let user = db.create_user(user_data).await.unwrap();
        
        let document = create_test_document(user.id);
        let created_doc = db.create_document(document).await.unwrap();
        
        let new_ocr_text = "Updated OCR text";
        let result = db.update_document_ocr(created_doc.id, new_ocr_text).await;
        assert!(result.is_ok());
        
        // Verify the update by searching
        let documents = db.get_documents_by_user(user.id, 10, 0).await.unwrap();
        let updated_doc = documents.iter().find(|d| d.id == created_doc.id).unwrap();
        assert_eq!(updated_doc.ocr_text.as_ref().unwrap(), new_ocr_text);
    }
}