#[cfg(test)]
mod ocr_retry_regression_tests {
    use sqlx::{PgPool, Row};
    use testcontainers::{runners::AsyncRunner, ContainerAsync};
    use testcontainers_modules::postgres::Postgres;
    use uuid::Uuid;
    use readur::routes::documents_ocr_retry::DocumentInfo;

    async fn setup_test_db() -> (ContainerAsync<Postgres>, PgPool) {
        let postgres_image = Postgres::default();
        let container = postgres_image.start().await.expect("Failed to start postgres container");
        let port = container.get_host_port_ipv4(5432).await.expect("Failed to get postgres port");
        
        let connection_string = format!(
            "postgres://postgres:postgres@127.0.0.1:{}/postgres",
            port
        );

        let pool = PgPool::connect(&connection_string).await.expect("Failed to connect to test database");
        
        // Skip migrations that require extensions and create minimal schema manually
        // This avoids needing uuid-ossp or other extensions for testing
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS users (
                id UUID PRIMARY KEY,
                username VARCHAR(255) UNIQUE NOT NULL,
                email VARCHAR(255) UNIQUE NOT NULL,
                password_hash VARCHAR(255) NOT NULL,
                role VARCHAR(50) NOT NULL DEFAULT 'user',
                created_at TIMESTAMPTZ DEFAULT NOW(),
                updated_at TIMESTAMPTZ DEFAULT NOW()
            )
        "#)
        .execute(&pool)
        .await
        .expect("Failed to create users table");

        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS documents (
                id UUID PRIMARY KEY,
                filename VARCHAR(255) NOT NULL,
                original_filename VARCHAR(255) NOT NULL,
                user_id UUID NOT NULL REFERENCES users(id),
                mime_type VARCHAR(100) NOT NULL,
                file_size BIGINT NOT NULL,
                ocr_status VARCHAR(50) DEFAULT 'pending',
                ocr_text TEXT,
                ocr_confidence DECIMAL(5,2),
                ocr_word_count INTEGER,
                ocr_processing_time_ms INTEGER,
                ocr_completed_at TIMESTAMPTZ,
                ocr_error TEXT,
                ocr_failure_reason VARCHAR(255),
                created_at TIMESTAMPTZ DEFAULT NOW(),
                updated_at TIMESTAMPTZ DEFAULT NOW()
            )
        "#)
        .execute(&pool)
        .await
        .expect("Failed to create documents table");
        
        (container, pool)
    }

    async fn create_test_user(pool: &PgPool) -> Uuid {
        let user_id = Uuid::new_v4();
        
        sqlx::query("INSERT INTO users (id, username, email, password_hash, role) VALUES ($1, $2, $3, 'test_hash', 'user')")
            .bind(user_id)
            .bind(format!("test_user_{}", user_id.simple().to_string()[0..8].to_string()))
            .bind(format!("test_{}@test.com", user_id.simple().to_string()[0..8].to_string()))
            .execute(pool)
            .await
            .expect("Failed to create test user");
            
        user_id
    }

    async fn create_test_document(pool: &PgPool, user_id: Uuid, ocr_status: &str) -> Uuid {
        let doc_id = Uuid::new_v4();
        
        sqlx::query(r#"
            INSERT INTO documents (
                id, filename, original_filename, user_id, mime_type, file_size, 
                ocr_status, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, 'application/pdf', 1024, $5, NOW(), NOW())
        "#)
            .bind(doc_id)
            .bind(format!("test_{}.pdf", doc_id.simple().to_string()[0..8].to_string()))
            .bind(format!("original_{}.pdf", doc_id.simple().to_string()[0..8].to_string()))
            .bind(user_id)
            .bind(ocr_status)
            .execute(pool)
            .await
            .expect("Failed to create test document");
            
        doc_id
    }

    #[tokio::test]
    async fn test_sql_query_only_returns_failed_documents() {
        let (_container, pool) = setup_test_db().await;
        let user_id = create_test_user(&pool).await;
        
        // Create documents with different OCR statuses
        let failed_doc1 = create_test_document(&pool, user_id, "failed").await;
        let failed_doc2 = create_test_document(&pool, user_id, "failed").await;
        let completed_doc = create_test_document(&pool, user_id, "completed").await;
        let pending_doc = create_test_document(&pool, user_id, "pending").await;
        let processing_doc = create_test_document(&pool, user_id, "processing").await;

        // Test the corrected SQL query that should be used in get_all_failed_ocr_documents
        let documents = sqlx::query_as::<_, DocumentInfo>(
            r#"
            SELECT id, filename, file_size, mime_type, ocr_failure_reason
            FROM documents
            WHERE ocr_status = 'failed'
              AND ($1::uuid IS NULL OR user_id = $1)
            ORDER BY created_at DESC
            "#
        )
        .bind(Some(user_id))
        .fetch_all(&pool)
        .await
        .expect("Failed to execute SQL query");
        
        // Should only return the 2 failed documents
        assert_eq!(documents.len(), 2, "SQL query should only return failed documents, but returned {}", documents.len());
        
        let returned_ids: Vec<Uuid> = documents.iter().map(|d| d.id).collect();
        assert!(returned_ids.contains(&failed_doc1), "Should contain first failed document");
        assert!(returned_ids.contains(&failed_doc2), "Should contain second failed document");
        assert!(!returned_ids.contains(&completed_doc), "Should NOT contain completed document");
        assert!(!returned_ids.contains(&pending_doc), "Should NOT contain pending document");
        assert!(!returned_ids.contains(&processing_doc), "Should NOT contain processing document");
    }

    #[tokio::test]
    async fn test_broken_sql_query_returns_all_documents() {
        let (_container, pool) = setup_test_db().await;
        let user_id = create_test_user(&pool).await;
        
        // Create documents with different OCR statuses
        let _failed_doc1 = create_test_document(&pool, user_id, "failed").await;
        let _failed_doc2 = create_test_document(&pool, user_id, "failed").await;
        let _completed_doc = create_test_document(&pool, user_id, "completed").await;
        let _pending_doc = create_test_document(&pool, user_id, "pending").await;
        let _processing_doc = create_test_document(&pool, user_id, "processing").await;

        // Test the BROKEN SQL query (what it was before the fix)
        let documents = sqlx::query_as::<_, DocumentInfo>(
            r#"
            SELECT id, filename, file_size, mime_type, ocr_failure_reason
            FROM documents
            WHERE ($1::uuid IS NULL OR user_id = $1)
            ORDER BY created_at DESC
            "#
        )
        .bind(Some(user_id))
        .fetch_all(&pool)
        .await
        .expect("Failed to execute broken SQL query");
        
        // This demonstrates the bug - it returns ALL documents (5), not just failed ones (2)
        assert_eq!(documents.len(), 5, "Broken SQL query returns all documents, demonstrating the bug");
    }


    #[tokio::test]
    async fn test_admin_vs_user_document_visibility() {
        let (_container, pool) = setup_test_db().await;
        
        // Create admin and regular users
        let admin_id = Uuid::new_v4();
        let user1_id = Uuid::new_v4();
        let user2_id = Uuid::new_v4();
        
        // Create admin user
        sqlx::query("INSERT INTO users (id, username, email, password_hash, role) VALUES ($1, 'admin', 'admin@test.com', 'test', 'admin')")
            .bind(admin_id)
            .execute(&pool)
            .await
            .expect("Failed to create admin user");
            
        // Create regular users  
        for (user_id, username) in [(user1_id, "user1"), (user2_id, "user2")] {
            sqlx::query("INSERT INTO users (id, username, email, password_hash, role) VALUES ($1, $2, $3, 'test', 'user')")
                .bind(user_id)
                .bind(username)
                .bind(format!("{}@test.com", username))
                .execute(&pool)
                .await
                .expect("Failed to create user");
        }
        
        // Create failed documents for different users
        let _admin_failed_doc = create_test_document(&pool, admin_id, "failed").await;
        let _user1_failed_doc = create_test_document(&pool, user1_id, "failed").await;
        let _user2_failed_doc = create_test_document(&pool, user2_id, "failed").await;

        // Test admin sees all failed documents (user_filter = NULL)
        let admin_docs = sqlx::query_as::<_, DocumentInfo>(
            r#"
            SELECT id, filename, file_size, mime_type, ocr_failure_reason
            FROM documents
            WHERE ocr_status = 'failed'
              AND ($1::uuid IS NULL OR user_id = $1)
            ORDER BY created_at DESC
            "#
        )
        .bind(None::<Uuid>) // Admin filter - NULL means see all
        .fetch_all(&pool)
        .await
        .expect("Failed to fetch admin documents");
        assert_eq!(admin_docs.len(), 3, "Admin should see all 3 failed documents");

        // Test regular user sees only their own
        let user1_docs = sqlx::query_as::<_, DocumentInfo>(
            r#"
            SELECT id, filename, file_size, mime_type, ocr_failure_reason
            FROM documents
            WHERE ocr_status = 'failed'
              AND ($1::uuid IS NULL OR user_id = $1)
            ORDER BY created_at DESC
            "#
        )
        .bind(Some(user1_id)) // User filter - only their documents
        .fetch_all(&pool)
        .await
        .expect("Failed to fetch user documents");
        assert_eq!(user1_docs.len(), 1, "User should only see their own failed document");
    }
}