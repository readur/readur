use crate::test_utils::TestContext;
use sqlx::Row;
use uuid::Uuid;

#[cfg(test)]
mod migration_integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_full_migration_workflow() {
        let ctx = TestContext::new().await;
        let pool = ctx.state.db.get_pool();
        // Setup: Create a test user first
        let user_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO users (id, username, email, password_hash, role) 
             VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(user_id)
        .bind("test_migration_user")
        .bind("test_migration@example.com")
        .bind("hash")
        .bind("user")
        .execute(pool)
        .await
        .unwrap();
        
        // Create test documents with different failure scenarios
        let test_documents = vec![
            ("doc1.pdf", Some("low_ocr_confidence"), "Quality below threshold"),
            ("doc2.pdf", Some("timeout"), "OCR processing timed out"),
            ("doc3.pdf", Some("memory_limit"), "Out of memory"),
            ("doc4.pdf", Some("corrupted"), "File appears corrupted"),
            ("doc5.pdf", Some("unknown"), "Unknown error occurred"),
            ("doc6.pdf", None, "Generic failure message"),
        ];

        // Insert test documents
        for (filename, failure_reason, error_msg) in &test_documents {
            sqlx::query(
                r#"
                INSERT INTO documents (
                    user_id, filename, original_filename, file_path, file_size, 
                    mime_type, ocr_status, ocr_failure_reason, ocr_error
                ) VALUES (
                    $1, $2, $2, '/fake/path', 1000, 'application/pdf', 
                    'failed', $3, $4
                )
                "#
            )
            .bind(user_id)
            .bind(filename)
            .bind(*failure_reason)
            .bind(error_msg)
            .execute(pool)
            .await
            .expect("Failed to insert test document");
        }

        // Count documents before migration
        let before_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM documents WHERE ocr_status = 'failed'"
        )
        .fetch_one(pool)
        .await
        .expect("Failed to count documents");

        assert_eq!(before_count, test_documents.len() as i64);

        // Simulate the migration logic
        let migration_result = sqlx::query(
            r#"
            INSERT INTO failed_documents (
                user_id, filename, original_filename, file_path, file_size,
                mime_type, error_message, failure_reason, failure_stage, ingestion_source,
                created_at, updated_at
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
                'migration' as ingestion_source,
                d.created_at, d.updated_at
            FROM documents d
            WHERE d.ocr_status = 'failed'
            "#
        )
        .execute(pool)
        .await;

        match migration_result {
            Ok(_) => {},
            Err(e) => panic!("Migration failed: {:?}", e),
        }

        // Verify all documents were migrated
        let migrated_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM failed_documents WHERE ingestion_source = 'migration'"
        )
        .fetch_one(pool)
        .await
        .expect("Failed to count migrated documents");

        assert_eq!(migrated_count, test_documents.len() as i64);

        // Verify specific mappings
        let mapping_tests = vec![
            ("doc1.pdf", "low_ocr_confidence"),
            ("doc2.pdf", "ocr_timeout"),
            ("doc3.pdf", "ocr_memory_limit"),
            ("doc4.pdf", "file_corrupted"),
            ("doc5.pdf", "other"),
            ("doc6.pdf", "other"),
        ];

        for (filename, expected_reason) in mapping_tests {
            let actual_reason: String = sqlx::query_scalar(
                "SELECT failure_reason FROM failed_documents WHERE filename = $1"
            )
            .bind(filename)
            .fetch_one(pool)
            .await
            .expect("Failed to fetch failure reason");

            assert_eq!(
                actual_reason,
                expected_reason,
                "Incorrect mapping for {}",
                filename
            );
        }

        // Test deletion of original failed documents
        let delete_result = sqlx::query(
            "DELETE FROM documents WHERE ocr_status = 'failed'"
        )
        .execute(pool)
        .await;

        assert!(delete_result.is_ok(), "Delete should succeed");

        // Verify cleanup
        let remaining_failed: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM documents WHERE ocr_status = 'failed'"
        )
        .fetch_one(pool)
        .await
        .expect("Failed to count remaining documents");

        assert_eq!(remaining_failed, 0);

        // Verify failed_documents table integrity
        let failed_docs = sqlx::query(
            "SELECT filename, failure_reason, failure_stage FROM failed_documents ORDER BY filename"
        )
        .fetch_all(pool)
        .await
        .expect("Failed to fetch failed documents");

        assert_eq!(failed_docs.len(), test_documents.len());

        for doc in &failed_docs {
            // All should have proper stage
            let stage: String = doc.get("failure_stage");
            assert_eq!(stage, "ocr");
            
            // All should have valid failure_reason
            let reason: String = doc.get("failure_reason");
            assert!(matches!(
                reason.as_str(),
                "low_ocr_confidence" | "ocr_timeout" | "ocr_memory_limit" | 
                "file_corrupted" | "other"
            ));
        }
    }

    #[tokio::test]
    async fn test_migration_with_edge_cases() {
        let ctx = TestContext::new().await;
        let pool = ctx.state.db.get_pool();
        
        // Create a test user first
        let user_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO users (id, username, email, password_hash, role) 
             VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(user_id)
        .bind("test_edge_user")
        .bind("test_edge@example.com")
        .bind("hash")
        .bind("user")
        .execute(pool)
        .await
        .unwrap();

        // Edge cases that might break migration
        let edge_cases = vec![
            ("empty_reason.pdf", Some(""), "Empty reason"),
            ("null_like.pdf", Some("null"), "Null-like value"),
            ("special_chars.pdf", Some("special!@#$%"), "Special characters"),
            ("very_long_reason.pdf", Some("this_is_a_very_long_failure_reason_that_might_cause_issues"), "Long reason"),
        ];

        for (filename, failure_reason, error_msg) in &edge_cases {
            sqlx::query(
                r#"
                INSERT INTO documents (
                    user_id, filename, original_filename, file_path, file_size, 
                    mime_type, ocr_status, ocr_failure_reason, ocr_error
                ) VALUES (
                    $1, $2, $2, '/fake/path', 1000, 'application/pdf', 
                    'failed', $3, $4
                )
                "#
            )
            .bind(user_id)
            .bind(filename)
            .bind(*failure_reason)
            .bind(error_msg)
            .execute(pool)
            .await
            .expect("Failed to insert edge case document");
        }

        // Run migration on edge cases
        let migration_result = sqlx::query(
            r#"
            INSERT INTO failed_documents (
                user_id, filename, failure_reason, failure_stage, ingestion_source
            )
            SELECT 
                d.user_id, d.filename,
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
                'migration_edge_test' as ingestion_source
            FROM documents d
            WHERE d.ocr_status = 'failed'
            "#
        )
        .execute(pool)
        .await;

        assert!(migration_result.is_ok(), "Migration should handle edge cases");

        // Verify all edge cases mapped to 'other' (since they're not in our mapping)
        let edge_case_mappings = sqlx::query(
            "SELECT filename, failure_reason FROM failed_documents WHERE ingestion_source = 'migration_edge_test'"
        )
        .fetch_all(pool)
        .await
        .expect("Failed to fetch edge case mappings");

        for mapping in edge_case_mappings {
            let filename: String = mapping.get("filename");
            let failure_reason: String = mapping.get("failure_reason");
            assert_eq!(failure_reason, "other", 
                      "Edge case '{}' should map to 'other'", filename);
        }
    }

    #[tokio::test]
    async fn test_constraint_enforcement_during_migration() {
        let ctx = TestContext::new().await;
        let pool = ctx.state.db.get_pool();
        
        // Create a test user first to avoid foreign key constraint violations
        let user_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO users (id, username, email, password_hash, role) 
             VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(user_id)
        .bind("test_constraint_user")
        .bind("test_constraint@example.com")
        .bind("hash")
        .bind("user")
        .execute(pool)
        .await
        .unwrap();

        // Try to insert data that violates constraints
        let invalid_insert = sqlx::query(
            r#"
            INSERT INTO failed_documents (
                user_id, filename, failure_reason, failure_stage, ingestion_source
            ) VALUES (
                $1, 'invalid_test.pdf', 'migration_completed', 'migration', 'test'
            )
            "#
        )
        .bind(user_id)
        .execute(pool)
        .await;

        // This should fail due to constraint violation
        assert!(invalid_insert.is_err(), "Invalid failure_reason should be rejected");

        // Verify the specific constraint that caught it
        if let Err(sqlx::Error::Database(db_err)) = invalid_insert {
            let error_message = db_err.message();
            assert!(
                error_message.contains("check_failure_reason") || 
                error_message.contains("constraint"),
                "Error should mention constraint violation: {}",
                error_message
            );
        }
    }
}