use readur::test_utils::TestContext;
use sqlx::{PgPool, Row};
use uuid::Uuid;
use std::collections::HashMap;

#[cfg(test)]
mod comprehensive_migration_tests {
    use super::*;

    #[tokio::test]
    async fn test_migration_with_prefilled_data() {
        let ctx = TestContext::new().await;
        let pool = ctx.state.db.get_pool();
        
        // Step 1: Prefill the database with test data
        let test_data = prefill_test_data(pool).await;
        
        // Step 2: Verify the prefilled data exists
        verify_prefilled_data(pool, &test_data).await;
        
        // Step 3: Simulate and test the failed documents migration
        test_failed_documents_migration(pool, &test_data).await;
        
        // Step 4: Verify schema integrity after migration
        verify_schema_integrity(pool).await;
        
        // Step 5: Test data consistency after migration
        verify_data_consistency_after_migration(pool, &test_data).await;
    }

    #[tokio::test]
    async fn test_migration_preserves_data_integrity() {
        let ctx = TestContext::new().await;
        let pool = ctx.state.db.get_pool();
        
        // Create comprehensive test data covering all edge cases
        let user_id = create_test_user(pool).await;
        
        // Insert various types of documents
        let document_scenarios = vec![
            DocumentScenario {
                filename: "normal_success.pdf",
                ocr_status: "completed",
                ocr_failure_reason: None,
                ocr_error: None,
                ocr_confidence: Some(0.95),
                ocr_text: Some("This is a successful OCR"),
                file_size: 1024,
            },
            DocumentScenario {
                filename: "low_confidence_fail.pdf",
                ocr_status: "failed",
                ocr_failure_reason: Some("low_ocr_confidence"),
                ocr_error: Some("OCR confidence below threshold"),
                ocr_confidence: Some(0.3),
                ocr_text: Some("Partially recognized text"),
                file_size: 2048,
            },
            DocumentScenario {
                filename: "timeout_fail.pdf",
                ocr_status: "failed",
                ocr_failure_reason: Some("timeout"),
                ocr_error: Some("OCR processing timed out after 60 seconds"),
                ocr_confidence: None,
                ocr_text: None,
                file_size: 10485760, // 10MB
            },
            DocumentScenario {
                filename: "memory_fail.pdf",
                ocr_status: "failed",
                ocr_failure_reason: Some("memory_limit"),
                ocr_error: Some("Memory limit exceeded"),
                ocr_confidence: None,
                ocr_text: None,
                file_size: 52428800, // 50MB
            },
            DocumentScenario {
                filename: "corrupted_file.pdf",
                ocr_status: "failed",
                ocr_failure_reason: Some("file_corrupted"),
                ocr_error: Some("PDF file appears to be corrupted"),
                ocr_confidence: None,
                ocr_text: None,
                file_size: 512,
            },
            DocumentScenario {
                filename: "unsupported.xyz",
                ocr_status: "failed",
                ocr_failure_reason: Some("unsupported_format"),
                ocr_error: Some("File format not supported"),
                ocr_confidence: None,
                ocr_text: None,
                file_size: 256,
            },
            DocumentScenario {
                filename: "pending_ocr.pdf",
                ocr_status: "pending",
                ocr_failure_reason: None,
                ocr_error: None,
                ocr_confidence: None,
                ocr_text: None,
                file_size: 4096,
            },
        ];
        
        // Insert all test documents
        let mut document_ids = HashMap::new();
        for scenario in &document_scenarios {
            let doc_id = insert_test_document(pool, user_id, scenario).await;
            document_ids.insert(scenario.filename, doc_id);
        }
        
        // Count documents before migration
        let failed_count_before: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM documents WHERE ocr_status = 'failed'"
        )
        .fetch_one(pool)
        .await
        .unwrap();
        
        let successful_count_before: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM documents WHERE ocr_status = 'completed'"
        )
        .fetch_one(pool)
        .await
        .unwrap();
        
        // Verify the migration query works correctly (simulate the migration)
        let migration_preview = sqlx::query(
            r#"
            SELECT 
                d.filename,
                d.ocr_failure_reason,
                CASE 
                    WHEN d.ocr_failure_reason = 'low_ocr_confidence' THEN 'low_ocr_confidence'
                    WHEN d.ocr_failure_reason = 'timeout' THEN 'ocr_timeout'
                    WHEN d.ocr_failure_reason = 'memory_limit' THEN 'ocr_memory_limit'
                    WHEN d.ocr_failure_reason = 'pdf_parsing_error' THEN 'pdf_parsing_error'
                    WHEN d.ocr_failure_reason = 'corrupted' OR d.ocr_failure_reason = 'file_corrupted' THEN 'file_corrupted'
                    WHEN d.ocr_failure_reason = 'unsupported_format' THEN 'unsupported_format'
                    WHEN d.ocr_failure_reason = 'access_denied' THEN 'access_denied'
                    ELSE 'other'
                END as mapped_failure_reason
            FROM documents d
            WHERE d.ocr_status = 'failed'
            "#
        )
        .fetch_all(pool)
        .await
        .unwrap();
        
        // Verify mappings are correct
        for row in migration_preview {
            let filename: String = row.get("filename");
            let original_reason: Option<String> = row.get("ocr_failure_reason");
            let mapped_reason: String = row.get("mapped_failure_reason");
            
            println!("Migration mapping: {} - {:?} -> {}", filename, original_reason, mapped_reason);
            
            // Verify specific mappings
            match original_reason.as_deref() {
                Some("low_ocr_confidence") => assert_eq!(mapped_reason, "low_ocr_confidence"),
                Some("timeout") => assert_eq!(mapped_reason, "ocr_timeout"),
                Some("memory_limit") => assert_eq!(mapped_reason, "ocr_memory_limit"),
                Some("file_corrupted") => assert_eq!(mapped_reason, "file_corrupted"),
                Some("unsupported_format") => assert_eq!(mapped_reason, "unsupported_format"),
                _ => assert_eq!(mapped_reason, "other"),
            }
        }
        
        // Verify that successful and pending documents are not affected
        assert_eq!(successful_count_before, 1, "Should have 1 successful document");
        assert_eq!(failed_count_before, 5, "Should have 5 failed documents");
    }

    #[tokio::test]
    async fn test_migration_with_ocr_queue_data() {
        let ctx = TestContext::new().await;
        let pool = ctx.state.db.get_pool();
        
        let user_id = create_test_user(pool).await;
        
        // Create a document with OCR queue history
        let doc_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO documents (id, user_id, filename, original_filename, file_path, file_size, mime_type, ocr_status, ocr_failure_reason, ocr_error)
            VALUES ($1, $2, $3, $3, '/test/path', 1000, 'application/pdf', 'failed', 'timeout', 'OCR timeout after retries')
            "#
        )
        .bind(doc_id)
        .bind(user_id)
        .bind("retry_test.pdf")
        .execute(pool)
        .await
        .unwrap();
        
        // Add OCR queue entries to simulate retry history
        for i in 0..3 {
            sqlx::query(
                r#"
                INSERT INTO ocr_queue (document_id, priority, status, error_message, created_at)
                VALUES ($1, $2, $3, $4, NOW() - INTERVAL '1 hour' * $5)
                "#
            )
            .bind(doc_id)
            .bind(1)
            .bind(if i < 2 { "failed" } else { "processing" })
            .bind(if i < 2 { Some("Retry attempt failed") } else { None })
            .bind((3 - i) as i32)
            .execute(pool)
            .await
            .unwrap();
        }
        
        // Test the migration query with retry count
        let result = sqlx::query(
            r#"
            SELECT 
                d.filename,
                d.ocr_failure_reason,
                COALESCE(q.retry_count, 0) as retry_count
            FROM documents d
            LEFT JOIN (
                SELECT document_id, COUNT(*) as retry_count
                FROM ocr_queue 
                WHERE status IN ('failed', 'completed')
                GROUP BY document_id
            ) q ON d.id = q.document_id
            WHERE d.id = $1
            "#
        )
        .bind(doc_id)
        .fetch_one(pool)
        .await
        .unwrap();
        
        let retry_count: i64 = result.get("retry_count");
        assert_eq!(retry_count, 2, "Should have 2 failed retry attempts");
    }

    #[tokio::test]
    async fn test_migration_handles_null_values() {
        let ctx = TestContext::new().await;
        let pool = ctx.state.db.get_pool();
        
        let user_id = create_test_user(pool).await;
        
        // Insert documents with various NULL values
        let null_scenarios = vec![
            ("null_reason.pdf", None, Some("Error without reason")),
            ("null_error.pdf", Some("unknown"), None),
            ("all_nulls.pdf", None, None),
        ];
        
        for (filename, reason, error) in &null_scenarios {
            sqlx::query(
                r#"
                INSERT INTO documents (user_id, filename, original_filename, file_path, file_size, mime_type, ocr_status, ocr_failure_reason, ocr_error)
                VALUES ($1, $2, $2, '/test/path', 1000, 'application/pdf', 'failed', $3, $4)
                "#
            )
            .bind(user_id)
            .bind(filename)
            .bind(reason)
            .bind(error)
            .execute(pool)
            .await
            .unwrap();
        }
        
        // Verify migration handles NULLs correctly
        let migrated_data = sqlx::query(
            r#"
            SELECT 
                filename,
                ocr_failure_reason,
                CASE 
                    WHEN ocr_failure_reason = 'low_ocr_confidence' THEN 'low_ocr_confidence'
                    WHEN ocr_failure_reason = 'timeout' THEN 'ocr_timeout'
                    WHEN ocr_failure_reason = 'memory_limit' THEN 'ocr_memory_limit'
                    WHEN ocr_failure_reason = 'pdf_parsing_error' THEN 'pdf_parsing_error'
                    WHEN ocr_failure_reason = 'corrupted' OR ocr_failure_reason = 'file_corrupted' THEN 'file_corrupted'
                    WHEN ocr_failure_reason = 'unsupported_format' THEN 'unsupported_format'
                    WHEN ocr_failure_reason = 'access_denied' THEN 'access_denied'
                    ELSE 'other'
                END as mapped_reason,
                ocr_error
            FROM documents 
            WHERE user_id = $1 AND ocr_status = 'failed'
            ORDER BY filename
            "#
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
        .unwrap();
        
        assert_eq!(migrated_data.len(), 3);
        for row in migrated_data {
            let mapped_reason: String = row.get("mapped_reason");
            assert_eq!(mapped_reason, "other", "NULL or unknown reasons should map to 'other'");
        }
    }

    #[tokio::test]
    async fn test_migration_performance_with_large_dataset() {
        let ctx = TestContext::new().await;
        let pool = ctx.state.db.get_pool();
        
        let user_id = create_test_user(pool).await;
        
        // Insert a large number of failed documents
        let batch_size = 100;
        let start_time = std::time::Instant::now();
        
        for batch in 0..10 {
            let mut query = String::from(
                "INSERT INTO documents (user_id, filename, original_filename, file_path, file_size, mime_type, ocr_status, ocr_failure_reason, ocr_error) VALUES "
            );
            let mut _values: Vec<String> = Vec::new();
            
            for i in 0..batch_size {
                let doc_num = batch * batch_size + i;
                let filename = format!("bulk_doc_{}.pdf", doc_num);
                let reason = match doc_num % 5 {
                    0 => "low_ocr_confidence",
                    1 => "timeout",
                    2 => "memory_limit",
                    3 => "file_corrupted",
                    _ => "unknown_error",
                };
                
                if i > 0 {
                    query.push_str(", ");
                }
                query.push_str(&format!("($1, '{}', '{}', '/test/path', 1000, 'application/pdf', 'failed', '{}', 'Test error')", 
                    filename, filename, reason));
            }
            
            sqlx::query(&query)
                .bind(user_id)
                .execute(pool)
                .await
                .unwrap();
        }
        
        let insert_duration = start_time.elapsed();
        println!("Inserted 1000 documents in {:?}", insert_duration);
        
        // Measure migration query performance
        let migration_start = std::time::Instant::now();
        
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM documents WHERE ocr_status = 'failed'"
        )
        .fetch_one(pool)
        .await
        .unwrap();
        
        assert_eq!(count, 1000, "Should have 1000 failed documents");
        
        // Simulate the migration SELECT
        let _migration_data = sqlx::query(
            r#"
            SELECT * FROM documents WHERE ocr_status = 'failed'
            "#
        )
        .fetch_all(pool)
        .await
        .unwrap();
        
        let migration_duration = migration_start.elapsed();
        println!("Migration query completed in {:?}", migration_duration);
        
        // Performance assertion - migration should complete reasonably fast
        assert!(migration_duration.as_secs() < 5, "Migration query should complete within 5 seconds");
    }

    // Helper functions
    
    struct TestData {
        user_id: Uuid,
        document_ids: HashMap<String, Uuid>,
        failure_scenarios: Vec<(String, String, String)>,
    }
    
    struct DocumentScenario {
        filename: &'static str,
        ocr_status: &'static str,
        ocr_failure_reason: Option<&'static str>,
        ocr_error: Option<&'static str>,
        ocr_confidence: Option<f32>,
        ocr_text: Option<&'static str>,
        file_size: i64,
    }
    
    async fn create_test_user(pool: &PgPool) -> Uuid {
        let user_id = Uuid::new_v4();
        let unique_suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let username = format!("test_migration_user_{}", unique_suffix);
        let email = format!("test_migration_{}@example.com", unique_suffix);
        
        sqlx::query(
            "INSERT INTO users (id, username, email, password_hash, role) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(user_id)
        .bind(&username)
        .bind(&email)
        .bind("test_hash")
        .bind("user")
        .execute(pool)
        .await
        .unwrap();
        
        user_id
    }
    
    async fn insert_test_document(pool: &PgPool, user_id: Uuid, scenario: &DocumentScenario) -> Uuid {
        let doc_id = Uuid::new_v4();
        
        sqlx::query(
            r#"
            INSERT INTO documents (
                id, user_id, filename, original_filename, file_path, file_size, 
                mime_type, ocr_status, ocr_failure_reason, ocr_error, 
                ocr_confidence, ocr_text
            ) VALUES (
                $1, $2, $3, $3, '/test/path', $4, $5, $6, $7, $8, $9, $10
            )
            "#
        )
        .bind(doc_id)
        .bind(user_id)
        .bind(scenario.filename)
        .bind(scenario.file_size)
        .bind(if scenario.filename.ends_with(".pdf") { "application/pdf" } else { "application/octet-stream" })
        .bind(scenario.ocr_status)
        .bind(scenario.ocr_failure_reason)
        .bind(scenario.ocr_error)
        .bind(scenario.ocr_confidence)
        .bind(scenario.ocr_text)
        .execute(pool)
        .await
        .unwrap();
        
        doc_id
    }
    
    async fn prefill_test_data(pool: &PgPool) -> TestData {
        let user_id = create_test_user(pool).await;
        let mut document_ids = HashMap::new();
        
        let failure_scenarios = vec![
            ("timeout_doc.pdf".to_string(), "timeout".to_string(), "OCR processing timed out".to_string()),
            ("memory_doc.pdf".to_string(), "memory_limit".to_string(), "Memory limit exceeded".to_string()),
            ("corrupt_doc.pdf".to_string(), "file_corrupted".to_string(), "File is corrupted".to_string()),
            ("low_conf_doc.pdf".to_string(), "low_ocr_confidence".to_string(), "Confidence too low".to_string()),
        ];
        
        // Insert test documents
        for (filename, reason, error) in &failure_scenarios {
            let doc_id = Uuid::new_v4();
            sqlx::query(
                r#"
                INSERT INTO documents (
                    id, user_id, filename, original_filename, file_path, file_size, 
                    mime_type, ocr_status, ocr_failure_reason, ocr_error
                ) VALUES (
                    $1, $2, $3, $3, '/test/path', 1000, 'application/pdf', 
                    'failed', $4, $5
                )
                "#
            )
            .bind(doc_id)
            .bind(user_id)
            .bind(filename)
            .bind(reason)
            .bind(error)
            .execute(pool)
            .await
            .unwrap();
            
            document_ids.insert(filename.clone(), doc_id);
        }
        
        TestData {
            user_id,
            document_ids,
            failure_scenarios,
        }
    }
    
    async fn verify_prefilled_data(pool: &PgPool, test_data: &TestData) {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM documents WHERE user_id = $1 AND ocr_status = 'failed'"
        )
        .bind(test_data.user_id)
        .fetch_one(pool)
        .await
        .unwrap();
        
        assert_eq!(count, test_data.failure_scenarios.len() as i64, 
                   "All test documents should be inserted");
    }
    
    async fn test_failed_documents_migration(pool: &PgPool, test_data: &TestData) {
        // Simulate the migration
        let result = sqlx::query(
            r#"
            INSERT INTO failed_documents (
                user_id, filename, original_filename, file_path, file_size,
                mime_type, error_message, failure_reason, failure_stage, ingestion_source
            )
            SELECT 
                d.user_id, d.filename, d.original_filename, d.file_path, d.file_size,
                d.mime_type, d.ocr_error,
                CASE 
                    WHEN d.ocr_failure_reason = 'low_ocr_confidence' THEN 'low_ocr_confidence'
                    WHEN d.ocr_failure_reason = 'timeout' THEN 'ocr_timeout'
                    WHEN d.ocr_failure_reason = 'memory_limit' THEN 'ocr_memory_limit'
                    WHEN d.ocr_failure_reason = 'pdf_parsing_error' THEN 'pdf_parsing_error'
                    WHEN d.ocr_failure_reason = 'corrupted' OR d.ocr_failure_reason = 'file_corrupted' THEN 'file_corrupted'
                    WHEN d.ocr_failure_reason = 'unsupported_format' THEN 'unsupported_format'
                    WHEN d.ocr_failure_reason = 'access_denied' THEN 'access_denied'
                    ELSE 'other'
                END as failure_reason,
                'ocr' as failure_stage,
                'test_migration' as ingestion_source
            FROM documents d
            WHERE d.ocr_status = 'failed' AND d.user_id = $1
            "#
        )
        .bind(test_data.user_id)
        .execute(pool)
        .await;
        
        assert!(result.is_ok(), "Migration should succeed");
        
        // Verify all documents were migrated
        let migrated_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM failed_documents WHERE user_id = $1 AND ingestion_source = 'test_migration'"
        )
        .bind(test_data.user_id)
        .fetch_one(pool)
        .await
        .unwrap();
        
        assert_eq!(migrated_count, test_data.failure_scenarios.len() as i64,
                   "All failed documents should be migrated");
    }
    
    async fn verify_schema_integrity(pool: &PgPool) {
        // Check that all expected tables exist
        let tables = sqlx::query(
            "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public'"
        )
        .fetch_all(pool)
        .await
        .unwrap();
        
        let table_names: Vec<String> = tables.iter()
            .map(|row| row.get("table_name"))
            .collect();
        
        assert!(table_names.contains(&"documents".to_string()));
        assert!(table_names.contains(&"failed_documents".to_string()));
        assert!(table_names.contains(&"users".to_string()));
        assert!(table_names.contains(&"ocr_queue".to_string()));
        
        // Check that constraints exist on failed_documents
        let constraints = sqlx::query(
            r#"
            SELECT constraint_name, constraint_type 
            FROM information_schema.table_constraints 
            WHERE table_name = 'failed_documents' AND constraint_type = 'CHECK'
            "#
        )
        .fetch_all(pool)
        .await
        .unwrap();
        
        let constraint_names: Vec<String> = constraints.iter()
            .map(|row| row.get("constraint_name"))
            .collect();
        
        assert!(constraint_names.iter().any(|name| name.contains("failure_reason")),
                "Should have check constraint for failure_reason");
        assert!(constraint_names.iter().any(|name| name.contains("failure_stage")),
                "Should have check constraint for failure_stage");
    }
    
    async fn verify_data_consistency_after_migration(pool: &PgPool, test_data: &TestData) {
        // Verify specific failure reason mappings
        let mappings = vec![
            ("timeout_doc.pdf", "ocr_timeout"),
            ("memory_doc.pdf", "ocr_memory_limit"),
            ("corrupt_doc.pdf", "file_corrupted"),
            ("low_conf_doc.pdf", "low_ocr_confidence"),
        ];
        
        for (filename, expected_reason) in mappings {
            let result = sqlx::query(
                "SELECT failure_reason FROM failed_documents WHERE filename = $1 AND user_id = $2"
            )
            .bind(filename)
            .bind(test_data.user_id)
            .fetch_optional(pool)
            .await
            .unwrap();
            
            assert!(result.is_some(), "Document {} should exist in failed_documents", filename);
            
            let actual_reason: String = result.unwrap().get("failure_reason");
            assert_eq!(actual_reason, expected_reason, 
                      "Failure reason for {} should be mapped correctly", filename);
        }
        
        // Verify all migrated documents have proper metadata
        let all_migrated = sqlx::query(
            "SELECT * FROM failed_documents WHERE user_id = $1"
        )
        .bind(test_data.user_id)
        .fetch_all(pool)
        .await
        .unwrap();
        
        for row in all_migrated {
            let failure_stage: String = row.get("failure_stage");
            assert_eq!(failure_stage, "ocr", "All migrated documents should have 'ocr' as failure_stage");
            
            let filename: String = row.get("filename");
            assert!(test_data.document_ids.contains_key(&filename), 
                    "Migrated document should be from our test data");
        }
    }
}