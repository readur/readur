/// Tests specifically designed to catch SQL type mismatches and import issues
/// These tests target the exact problems that weren't caught before:
/// 1. NUMERIC vs BIGINT type mismatches in aggregate functions  
/// 2. Missing Row trait imports
/// 3. SQL compilation issues that only appear with real database queries

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use readur::test_utils::TestContext;
    use sqlx::Row;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_row_trait_import_is_available() {
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let pool = ctx.state.db.get_pool();

            // This test ensures Row trait is imported and available
            // The .get() method would fail to compile if Row trait is missing
            let result = sqlx::query("SELECT 1::BIGINT as test_value")
                .fetch_one(pool)
                .await
                .unwrap();

            // These calls require Row trait to be in scope
            let _value: i64 = result.get("test_value");
            let _value_by_index: i64 = result.get(0);
            let _optional_value: Option<i64> = result.get("test_value");
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_sum_aggregate_type_safety() {
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let pool = ctx.state.db.get_pool();

            // Create test data with unique username
            let user_id = Uuid::new_v4();
            let unique_suffix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let username = format!("test_aggregate_user_{}", unique_suffix);
            let email = format!("test_agg_{}@example.com", unique_suffix);

            sqlx::query(
                "INSERT INTO users (id, username, email, password_hash, role) 
                 VALUES ($1, $2, $3, $4, $5)"
            )
            .bind(user_id)
            .bind(&username)
            .bind(&email)
            .bind("hash")
            .bind("user")
            .execute(pool)
            .await
            .unwrap();

            // Insert test documents
            for i in 0..3 {
                let doc_id = Uuid::new_v4();
                sqlx::query(
                    r#"
                    INSERT INTO documents (id, filename, original_filename, file_path, file_size, mime_type, user_id) 
                    VALUES ($1, $2, $3, $4, $5, $6, $7)
                    "#
                )
                .bind(doc_id)
                .bind(format!("test_{}.pdf", i))
                .bind(format!("test_{}.pdf", i))
                .bind(format!("/test/test_{}.pdf", i))
                .bind(1024i64 * (i + 1) as i64)  // Different file sizes
                .bind("application/pdf")
                .bind(user_id)
                .execute(pool)
                .await
                .unwrap();
            }

            // Test the exact SQL pattern from ignored_files.rs that was failing
            let result = sqlx::query(
                r#"
                SELECT 
                    COUNT(*) as total_files,
                    COALESCE(SUM(file_size), 0)::BIGINT as total_size_bytes
                FROM documents 
                WHERE user_id = $1
                "#
            )
            .bind(user_id)
            .fetch_one(pool)
            .await
            .unwrap();

            // This extraction would fail if ::BIGINT cast was missing
            let total_files: i64 = result.get("total_files");
            let total_size_bytes: i64 = result.get("total_size_bytes");

            assert_eq!(total_files, 3);
            assert_eq!(total_size_bytes, 1024 + 2048 + 3072); // Sum of file sizes
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_group_by_aggregate_type_safety() {
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let pool = ctx.state.db.get_pool();

            // Test the exact SQL pattern from ignored_files.rs GROUP BY query
            let results = sqlx::query(
                r#"
                SELECT 
                    mime_type,
                    COUNT(*) as count,
                    COALESCE(SUM(file_size), 0)::BIGINT as total_size_bytes
                FROM documents 
                GROUP BY mime_type
                ORDER BY count DESC
                "#
            )
            .fetch_all(pool)
            .await
            .unwrap();

            // Test that we can extract all values without type errors
            for row in results {
                let _mime_type: String = row.get("mime_type");
                let _count: i64 = row.get("count");
                let _total_size_bytes: i64 = row.get("total_size_bytes");
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
    async fn test_numeric_vs_bigint_difference() {
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let pool = ctx.state.db.get_pool();

            // Demonstrate the difference between NUMERIC and BIGINT return types

            // This query returns NUMERIC (the original problematic pattern)
            let numeric_result = sqlx::query("SELECT COALESCE(SUM(file_size), 0) as total_size FROM documents")
                .fetch_one(pool)
                .await
                .unwrap();

            // This query returns BIGINT (the fixed pattern)
            let bigint_result = sqlx::query("SELECT COALESCE(SUM(file_size), 0)::BIGINT as total_size FROM documents")
                .fetch_one(pool)
                .await
                .unwrap();

            // The BIGINT version should work with i64 extraction
            let _bigint_value: i64 = bigint_result.get("total_size");

            // The NUMERIC version would fail with i64 extraction but works with f64
            let _numeric_as_f64: Option<f64> = numeric_result.try_get("total_size").ok();

            // Trying to get NUMERIC as i64 would fail (this is what was causing the original error)
            let numeric_as_i64_result: Result<i64, _> = numeric_result.try_get("total_size");
            assert!(numeric_as_i64_result.is_err()); // This demonstrates the original problem
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_ignored_files_aggregate_queries() {
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let pool = ctx.state.db.get_pool();

            // Create test user
            let user_id = Uuid::new_v4();
            let unique_suffix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let username = format!("test_ignored_user_{}", unique_suffix);
            let email = format!("test_ignored_{}@example.com", unique_suffix);

            sqlx::query(
                "INSERT INTO users (id, username, email, password_hash, role) 
                 VALUES ($1, $2, $3, $4, $5)"
            )
            .bind(user_id)
            .bind(&username)
            .bind(&email)
            .bind("hash")
            .bind("admin")
            .execute(pool)
            .await
            .unwrap();

            // Add test ignored files
            for i in 0..2 {
                let file_id = Uuid::new_v4();
                sqlx::query(
                    r#"
                    INSERT INTO ignored_files (id, ignored_by, filename, original_filename, file_path, file_size, mime_type, source_type, reason, file_hash)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                    "#
                )
                .bind(file_id)
                .bind(user_id)
                .bind(format!("ignored_{}.pdf", i))
                .bind(format!("ignored_{}.pdf", i)) // Add original_filename
                .bind(format!("/test/ignored_{}.pdf", i))
                .bind(1024i64 * (i + 1) as i64)
                .bind("application/pdf")
                .bind("source_sync")
                .bind(Some("Test reason"))
                .bind(format!("{:x}", Uuid::new_v4().as_u128())) // Add unique file_hash
                .execute(pool)
                .await
                .unwrap();
            }

            // Test the exact queries from ignored_files.rs that were failing

            // Main stats query
            let stats_result = sqlx::query(
                r#"
                SELECT 
                    COUNT(*) as total_ignored_files,
                    COALESCE(SUM(file_size), 0)::BIGINT as total_size_bytes,
                    MAX(ignored_at) as most_recent_ignored_at
                FROM ignored_files 
                WHERE ignored_by = $1
                "#
            )
            .bind(user_id)
            .fetch_one(pool)
            .await
            .unwrap();

            // These extractions would fail without proper type casting
            let total_files: i64 = stats_result.get("total_ignored_files");
            let total_size: i64 = stats_result.get("total_size_bytes");

            assert_eq!(total_files, 2);
            assert_eq!(total_size, 1024 + 2048);

            // Group by source type query
            let by_source_results = sqlx::query(
                r#"
                SELECT 
                    source_type,
                    COUNT(*) as count,
                    COALESCE(SUM(file_size), 0)::BIGINT as total_size_bytes
                FROM ignored_files 
                WHERE ignored_by = $1
                GROUP BY source_type
                ORDER BY count DESC
                "#
            )
            .bind(user_id)
            .fetch_all(pool)
            .await
            .unwrap();

            // Test extraction from GROUP BY results
            for row in by_source_results {
                let _source_type: String = row.get("source_type");
                let _count: i64 = row.get("count");
                let _total_size_bytes: i64 = row.get("total_size_bytes");
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
    async fn test_queue_enqueue_pending_sql_patterns() {
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let pool = ctx.state.db.get_pool();

            // Test the SQL patterns from queue.rs that need Row trait
            let pending_documents = sqlx::query(
                r#"
                SELECT d.id, d.file_size
                FROM documents d
                LEFT JOIN ocr_queue oq ON d.id = oq.document_id
                WHERE d.ocr_status = 'pending'
                  AND oq.document_id IS NULL
                  AND d.file_path IS NOT NULL
                  AND (d.mime_type LIKE 'image/%' OR d.mime_type = 'application/pdf' OR d.mime_type = 'text/plain')
                ORDER BY d.created_at ASC
                "#
            )
            .fetch_all(pool)
            .await
            .unwrap();

            // Test that Row trait methods work (these would fail without proper import)
            for row in pending_documents {
                let _document_id: uuid::Uuid = row.get("id");
                let _file_size: i64 = row.get("file_size");
            }
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }
}