#[cfg(test)]
mod tests {
    use crate::db::ignored_files::{
        create_ignored_file, list_ignored_files, get_ignored_file_by_id, delete_ignored_file,
        is_file_ignored, count_ignored_files, bulk_delete_ignored_files,
        create_ignored_file_from_document
    };
    use crate::models::{CreateIgnoredFile, IgnoredFilesQuery, User, UserRole, Document, AuthProvider};
    use uuid::Uuid;
    use chrono::Utc;
    use sqlx::PgPool;
    use std::env;

    async fn create_test_db_pool() -> PgPool {
        let database_url = env::var("TEST_DATABASE_URL")
            .or_else(|_| env::var("DATABASE_URL"))
            .unwrap_or_else(|_| {
                // Skip tests if no database URL is available
                println!("Skipping database tests: TEST_DATABASE_URL or DATABASE_URL not set");
                std::process::exit(0);
            });
        PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to test database")
    }

    async fn create_test_user(pool: &PgPool) -> User {
        let user_id = Uuid::new_v4();
        let user = User {
            id: user_id,
            username: format!("testuser_{}", user_id),
            email: format!("test_{}@example.com", user_id),
            password_hash: Some("hashed_password".to_string()),
            role: UserRole::User,
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

    async fn create_test_document(pool: &PgPool, user_id: Uuid) -> Document {
        let document_id = Uuid::new_v4();
        let document = Document {
            id: document_id,
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
        };

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
    async fn test_create_ignored_file() {
        let pool = create_test_db_pool().await;
        let user = create_test_user(&pool).await;

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
            ignored_by: user.id,
            reason: Some("deleted by user".to_string()),
        };

        let result = create_ignored_file(&pool, ignored_file).await;
        assert!(result.is_ok());

        let created = result.unwrap();
        assert_eq!(created.file_hash, "abc123");
        assert_eq!(created.filename, "test.pdf");
        assert_eq!(created.ignored_by, user.id);
        assert_eq!(created.source_type, Some("webdav".to_string()));
    }

    #[tokio::test]
    async fn test_list_ignored_files() {
        let pool = create_test_db_pool().await;
        let user = create_test_user(&pool).await;

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
                ignored_by: user.id,
                reason: Some("deleted by user".to_string()),
            };

            create_ignored_file(&pool, ignored_file).await.unwrap();
        }

        let query = IgnoredFilesQuery {
            limit: Some(10),
            offset: Some(0),
            source_type: None,
            source_identifier: None,
            ignored_by: None,
            filename: None,
        };

        let result = list_ignored_files(&pool, user.id, &query).await;
        assert!(result.is_ok());

        let ignored_files = result.unwrap();
        assert_eq!(ignored_files.len(), 3);
        assert!(ignored_files.iter().all(|f| f.ignored_by == user.id));
    }

    #[tokio::test]
    async fn test_get_ignored_file_by_id() {
        let pool = create_test_db_pool().await;
        let user = create_test_user(&pool).await;

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
            ignored_by: user.id,
            reason: Some("deleted by user".to_string()),
        };

        let created = create_ignored_file(&pool, ignored_file).await.unwrap();

        let result = get_ignored_file_by_id(&pool, created.id, user.id).await;
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
        let pool = create_test_db_pool().await;
        let user = create_test_user(&pool).await;

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
            ignored_by: user.id,
            reason: Some("deleted by user".to_string()),
        };

        let created = create_ignored_file(&pool, ignored_file).await.unwrap();

        let result = delete_ignored_file(&pool, created.id, user.id).await;
        assert!(result.is_ok());
        assert!(result.unwrap());

        // Verify it's deleted
        let fetched = get_ignored_file_by_id(&pool, created.id, user.id).await;
        assert!(fetched.is_ok());
        assert!(fetched.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_is_file_ignored() {
        let pool = create_test_db_pool().await;
        let user = create_test_user(&pool).await;

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
            ignored_by: user.id,
            reason: Some("deleted by user".to_string()),
        };

        create_ignored_file(&pool, ignored_file).await.unwrap();

        // Test with exact match
        let result = is_file_ignored(
            &pool,
            "test_hash",
            Some("webdav"),
            Some("/webdav/test.pdf")
        ).await;
        assert!(result.is_ok());
        assert!(result.unwrap());

        // Test with just hash
        let result = is_file_ignored(&pool, "test_hash", None, None).await;
        assert!(result.is_ok());
        assert!(result.unwrap());

        // Test with non-existing hash
        let result = is_file_ignored(&pool, "non_existing", None, None).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn test_create_ignored_file_from_document() {
        let pool = create_test_db_pool().await;
        let user = create_test_user(&pool).await;
        let document = create_test_document(&pool, user.id).await;

        let result = create_ignored_file_from_document(
            &pool,
            document.id,
            user.id,
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
        assert_eq!(ignored_file.ignored_by, user.id);
        assert_eq!(ignored_file.source_type, Some("webdav".to_string()));
        assert_eq!(ignored_file.reason, Some("deleted by user".to_string()));
    }
}