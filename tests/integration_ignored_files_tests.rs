#[cfg(test)]
mod tests {
    use readur::db::ignored_files::{
        create_ignored_file, list_ignored_files, get_ignored_file_by_id, delete_ignored_file,
        is_file_ignored, count_ignored_files, bulk_delete_ignored_files,
        create_ignored_file_from_document
    };
    use readur::models::{CreateIgnoredFile, IgnoredFilesQuery, User, UserRole, Document, AuthProvider};
    use readur::test_utils::{TestContext, TestAuthHelper};
    use uuid::Uuid;
    use chrono::Utc;


    #[tokio::test]
    async fn test_create_ignored_file() {
        let ctx = TestContext::new().await;
        let auth_helper = TestAuthHelper::new(ctx.app.clone());
        let user = auth_helper.create_test_user().await;

        let ignored_file = CreateIgnoredFile {
            file_hash: "abc123".to_string(),
            filename: "test.pdf".to_string(),
            original_filename: "original_test.pdf".to_string(),
            file_path: "/path/to/test.pdf".to_string(),
            file_size: 1024,
            mime_type: "application/pdf".to_string(),
            source_type: Some("webdav".to_string()),
            source_path: Some("/webdav/test.pdf".to_string()),
            source_identifier: Some("webdav-server-1".to_string()),
            ignored_by: user.user_response.id,
            reason: Some("deleted by user".to_string()),
        };

        let result = create_ignored_file(&ctx.state.db.pool, ignored_file).await;
        assert!(result.is_ok());

        let created = result.unwrap();
        assert_eq!(created.file_hash, "abc123");
        assert_eq!(created.filename, "test.pdf");
        assert_eq!(created.ignored_by, user.user_response.id);
        assert_eq!(created.source_type, Some("webdav".to_string()));
    }

    #[tokio::test]
    async fn test_list_ignored_files() {
        let ctx = TestContext::new().await;
        let auth_helper = TestAuthHelper::new(ctx.app.clone());
        let user = auth_helper.create_test_user().await;

        // Create multiple ignored files
        for i in 0..3 {
            let ignored_file = CreateIgnoredFile {
                file_hash: format!("hash{}", i),
                filename: format!("test{}.pdf", i),
                original_filename: format!("original_test{}.pdf", i),
                file_path: format!("/path/to/test{}.pdf", i),
                file_size: 1024 * (i + 1) as i64,
                mime_type: "application/pdf".to_string(),
                source_type: Some("webdav".to_string()),
                source_path: Some(format!("/webdav/test{}.pdf", i)),
                source_identifier: Some("webdav-server-1".to_string()),
                ignored_by: user.user_response.id,
                reason: Some("deleted by user".to_string()),
            };

            create_ignored_file(&ctx.state.db.pool, ignored_file).await.unwrap();
        }

        let query = IgnoredFilesQuery {
            limit: Some(10),
            offset: Some(0),
            source_type: None,
            source_identifier: None,
            ignored_by: None,
            filename: None,
        };

        let result = list_ignored_files(&ctx.state.db.pool, user.user_response.id, &query).await;
        assert!(result.is_ok());

        let ignored_files = result.unwrap();
        assert_eq!(ignored_files.len(), 3);
        assert!(ignored_files.iter().all(|f| f.ignored_by == user.user_response.id));
    }

    #[tokio::test]
    async fn test_get_ignored_file_by_id() {
        let ctx = TestContext::new().await;
        let auth_helper = TestAuthHelper::new(ctx.app.clone());
        let user = auth_helper.create_test_user().await;

        let ignored_file = CreateIgnoredFile {
            file_hash: "test_hash".to_string(),
            filename: "test.pdf".to_string(),
            original_filename: "original_test.pdf".to_string(),
            file_path: "/path/to/test.pdf".to_string(),
            file_size: 1024,
            mime_type: "application/pdf".to_string(),
            source_type: Some("webdav".to_string()),
            source_path: Some("/webdav/test.pdf".to_string()),
            source_identifier: Some("webdav-server-1".to_string()),
            ignored_by: user.user_response.id,
            reason: Some("deleted by user".to_string()),
        };

        let created = create_ignored_file(&ctx.state.db.pool, ignored_file).await.unwrap();

        let result = get_ignored_file_by_id(&ctx.state.db.pool, created.id, user.user_response.id).await;
        assert!(result.is_ok());

        let fetched = result.unwrap();
        assert!(fetched.is_some());

        let fetched = fetched.unwrap();
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.file_hash, "test_hash");
        assert_eq!(fetched.filename, "test.pdf");
    }

    #[tokio::test]
    async fn test_delete_ignored_file() {
        let ctx = TestContext::new().await;
        let auth_helper = TestAuthHelper::new(ctx.app.clone());
        let user = auth_helper.create_test_user().await;

        let ignored_file = CreateIgnoredFile {
            file_hash: "test_hash".to_string(),
            filename: "test.pdf".to_string(),
            original_filename: "original_test.pdf".to_string(),
            file_path: "/path/to/test.pdf".to_string(),
            file_size: 1024,
            mime_type: "application/pdf".to_string(),
            source_type: Some("webdav".to_string()),
            source_path: Some("/webdav/test.pdf".to_string()),
            source_identifier: Some("webdav-server-1".to_string()),
            ignored_by: user.user_response.id,
            reason: Some("deleted by user".to_string()),
        };

        let created = create_ignored_file(&ctx.state.db.pool, ignored_file).await.unwrap();

        let result = delete_ignored_file(&ctx.state.db.pool, created.id, user.user_response.id).await;
        assert!(result.is_ok());
        assert!(result.unwrap());

        // Verify it's deleted
        let fetched = get_ignored_file_by_id(&ctx.state.db.pool, created.id, user.user_response.id).await;
        assert!(fetched.is_ok());
        assert!(fetched.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_is_file_ignored() {
        let ctx = TestContext::new().await;
        let auth_helper = TestAuthHelper::new(ctx.app.clone());
        let user = auth_helper.create_test_user().await;

        let ignored_file = CreateIgnoredFile {
            file_hash: "test_hash".to_string(),
            filename: "test.pdf".to_string(),
            original_filename: "original_test.pdf".to_string(),
            file_path: "/path/to/test.pdf".to_string(),
            file_size: 1024,
            mime_type: "application/pdf".to_string(),
            source_type: Some("webdav".to_string()),
            source_path: Some("/webdav/test.pdf".to_string()),
            source_identifier: Some("webdav-server-1".to_string()),
            ignored_by: user.user_response.id,
            reason: Some("deleted by user".to_string()),
        };

        create_ignored_file(&ctx.state.db.pool, ignored_file).await.unwrap();

        // Test with exact match
        let result = is_file_ignored(
            &ctx.state.db.pool,
            "test_hash",
            Some("webdav"),
            Some("/webdav/test.pdf")
        ).await;
        assert!(result.is_ok());
        assert!(result.unwrap());

        // Test with just hash
        let result = is_file_ignored(&ctx.state.db.pool, "test_hash", None, None).await;
        assert!(result.is_ok());
        assert!(result.unwrap());

        // Test with non-existing hash
        let result = is_file_ignored(&ctx.state.db.pool, "non_existing", None, None).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn test_create_ignored_file_from_document() {
        let ctx = TestContext::new().await;
        let auth_helper = TestAuthHelper::new(ctx.app.clone());
        let user = auth_helper.create_test_user().await;
        let document = ctx.state.db.create_document(readur::models::Document {
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
            user_id: user.user_response.id,
            file_hash: Some("1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string()),
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
        }).await.unwrap();

        let result = create_ignored_file_from_document(
            &ctx.state.db.pool,
            document.id,
            user.user_response.id,
            Some("deleted by user".to_string()),
            Some("webdav".to_string()),
            Some("/webdav/test.pdf".to_string()),
            Some("webdav-server-1".to_string()),
        ).await;

        assert!(result.is_ok());
        let ignored_file = result.unwrap();
        assert!(ignored_file.is_some());

        let ignored_file = ignored_file.unwrap();
        assert_eq!(ignored_file.filename, document.filename);
        assert_eq!(ignored_file.file_size, document.file_size);
        assert_eq!(ignored_file.mime_type, document.mime_type);
        assert_eq!(ignored_file.ignored_by, user.user_response.id);
        assert_eq!(ignored_file.source_type, Some("webdav".to_string()));
        assert_eq!(ignored_file.reason, Some("deleted by user".to_string()));
    }
}