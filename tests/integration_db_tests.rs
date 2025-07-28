#[cfg(test)]
mod tests {
    use anyhow::Result;
    use readur::test_utils::TestContext;
    use readur::models::{CreateUser, Document, SearchRequest};
    use chrono::Utc;
    use uuid::Uuid;

    fn create_test_user_data() -> CreateUser {
        let test_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .to_string();
        let unique_suffix = &test_id[test_id.len().saturating_sub(8)..];
        
        CreateUser {
            username: format!("testuser_{}", unique_suffix),
            email: format!("test_{}@example.com", unique_suffix),
            password: "password123".to_string(),
            role: Some(readur::models::UserRole::User),
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
            file_hash: Some(format!("{:x}", Uuid::new_v4().as_u128())), // Generate unique file hash
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

    #[tokio::test]
    async fn test_create_user() {
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let db = &ctx.state.db;
            let user_data = create_test_user_data();

            let result = db.create_user(user_data).await;
            assert!(result.is_ok());

            let user = result.unwrap();
            assert!(user.username.starts_with("testuser_"));
            assert!(user.email.starts_with("test_") && user.email.ends_with("@example.com"));
            assert!(user.password_hash.is_some());
            assert_ne!(user.password_hash.as_ref().unwrap(), "password123"); // Should be hashed
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_get_user_by_username() {
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let db = &ctx.state.db;
            let user_data = create_test_user_data();

            let created_user = db.create_user(user_data).await.unwrap();

            let result = db.get_user_by_username(&created_user.username).await;
            assert!(result.is_ok());

            let found_user = result.unwrap();
            assert!(found_user.is_some());

            let user = found_user.unwrap();
            assert_eq!(user.id, created_user.id);
            assert_eq!(user.username, created_user.username);
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_get_user_by_username_not_found() {
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let db = &ctx.state.db;

            let result = db.get_user_by_username("nonexistent").await;
            assert!(result.is_ok());

            let found_user = result.unwrap();
            assert!(found_user.is_none());
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_create_document() {
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let db = &ctx.state.db;
            let user_data = create_test_user_data();
            let user = db.create_user(user_data).await.unwrap();

            let document = create_test_document(user.id);

            let result = db.create_document(document.clone()).await;
            assert!(result.is_ok());

            let created_doc = result.unwrap();
            assert_eq!(created_doc.filename, document.filename);
            assert_eq!(created_doc.user_id, user.id);
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_get_documents_by_user() {
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let db = &ctx.state.db;
            let user_data = create_test_user_data();
            let user = db.create_user(user_data).await.unwrap();

            let document1 = create_test_document(user.id);
            let document2 = create_test_document(user.id);

            db.create_document(document1).await.unwrap();
            db.create_document(document2).await.unwrap();

            let result = db.get_documents_by_user(user.id, 10, 0).await;
            assert!(result.is_ok());

            let documents = result.unwrap();
            assert_eq!(documents.len(), 2);
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_search_documents() {
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let db = &ctx.state.db;
            let user_data = create_test_user_data();
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

            let result = db.search_documents(user.id, &search_request).await;
            assert!(result.is_ok());

            let documents = result.unwrap();
            assert_eq!(documents.len(), 1);
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_update_document_ocr() {
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let db = &ctx.state.db;
            let user_data = create_test_user_data();
            let user = db.create_user(user_data).await.unwrap();

            let document = create_test_document(user.id);
            let created_doc = db.create_document(document).await.unwrap();

            let new_ocr_text = "Updated OCR text";
            let result = db.update_document_ocr(created_doc.id, Some(new_ocr_text.to_string()), None, None, None, None).await;
            assert!(result.is_ok());

            // Verify the update by searching
            let documents = db.get_documents_by_user(user.id, 10, 0).await.unwrap();
            let updated_doc = documents.iter().find(|d| d.id == created_doc.id).unwrap();
            assert_eq!(updated_doc.ocr_text.as_ref().unwrap(), new_ocr_text);
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }
}