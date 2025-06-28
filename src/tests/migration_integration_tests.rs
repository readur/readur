use sqlx::PgPool;
use uuid::Uuid;

#[cfg(test)]
mod migration_integration_tests {
    use super::*;

    #[sqlx::test]
    async fn test_full_migration_workflow(pool: PgPool) {
        // Setup: Create sample documents with various OCR failure reasons
        let user_id = Uuid::new_v4();
        
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
            sqlx::query!(
                r#"
                INSERT INTO documents (
                    user_id, filename, original_filename, file_path, file_size, 
                    mime_type, ocr_status, ocr_failure_reason, ocr_error
                ) VALUES (
                    $1, $2, $2, '/fake/path', 1000, 'application/pdf', 
                    'failed', $3, $4
                )
                "#,
                user_id,
                filename,
                *failure_reason,
                error_msg
            )
            .execute(&pool)
            .await
            .expect("Failed to insert test document");
        }

        // Count documents before migration
        let before_count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM documents WHERE ocr_status = 'failed'"
        )
        .fetch_one(&pool)
        .await
        .expect("Failed to count documents")
        .unwrap_or(0);

        assert_eq!(before_count, test_documents.len() as i64);

        // Simulate the migration logic
        let migration_result = sqlx::query!(
            r#"
            INSERT INTO failed_documents (
                user_id, filename, original_filename, file_path, file_size,
                mime_type, ocr_error, failure_reason, failure_stage, ingestion_source,
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
        .execute(&pool)
        .await;

        assert!(migration_result.is_ok(), "Migration should succeed");

        // Verify all documents were migrated
        let migrated_count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM failed_documents WHERE ingestion_source = 'migration'"
        )
        .fetch_one(&pool)
        .await
        .expect("Failed to count migrated documents")
        .unwrap_or(0);

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
            let actual_reason = sqlx::query_scalar!(
                "SELECT failure_reason FROM failed_documents WHERE filename = $1",
                filename
            )
            .fetch_one(&pool)
            .await
            .expect("Failed to fetch failure reason");

            assert_eq!(
                actual_reason.as_deref(),
                Some(expected_reason),
                "Incorrect mapping for {}",
                filename
            );
        }

        // Test deletion of original failed documents
        let delete_result = sqlx::query!(
            "DELETE FROM documents WHERE ocr_status = 'failed'"
        )
        .execute(&pool)
        .await;

        assert!(delete_result.is_ok(), "Delete should succeed");

        // Verify cleanup
        let remaining_failed = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM documents WHERE ocr_status = 'failed'"
        )
        .fetch_one(&pool)
        .await
        .expect("Failed to count remaining documents")
        .unwrap_or(0);

        assert_eq!(remaining_failed, 0);

        // Verify failed_documents table integrity
        let failed_docs = sqlx::query!(
            "SELECT filename, failure_reason, failure_stage FROM failed_documents ORDER BY filename"
        )
        .fetch_all(&pool)
        .await
        .expect("Failed to fetch failed documents");

        assert_eq!(failed_docs.len(), test_documents.len());

        for doc in &failed_docs {
            // All should have proper stage
            assert_eq!(doc.failure_stage, "ocr");
            
            // All should have valid failure_reason
            assert!(matches!(
                doc.failure_reason.as_str(),
                "low_ocr_confidence" | "ocr_timeout" | "ocr_memory_limit" | 
                "file_corrupted" | "other"
            ));
        }
    }

    #[sqlx::test]
    async fn test_migration_with_edge_cases(pool: PgPool) {
        // Test migration with edge cases that previously caused issues
        let user_id = Uuid::new_v4();

        // Edge cases that might break migration
        let edge_cases = vec![
            ("empty_reason.pdf", Some(""), "Empty reason"),
            ("null_like.pdf", Some("null"), "Null-like value"),
            ("special_chars.pdf", Some("special!@#$%"), "Special characters"),
            ("very_long_reason.pdf", Some("this_is_a_very_long_failure_reason_that_might_cause_issues"), "Long reason"),
        ];

        for (filename, failure_reason, error_msg) in &edge_cases {
            sqlx::query!(
                r#"
                INSERT INTO documents (
                    user_id, filename, original_filename, file_path, file_size, 
                    mime_type, ocr_status, ocr_failure_reason, ocr_error
                ) VALUES (
                    $1, $2, $2, '/fake/path', 1000, 'application/pdf', 
                    'failed', $3, $4
                )
                "#,
                user_id,
                filename,
                *failure_reason,
                error_msg
            )
            .execute(&pool)
            .await
            .expect("Failed to insert edge case document");
        }

        // Run migration on edge cases
        let migration_result = sqlx::query!(
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
        .execute(&pool)
        .await;

        assert!(migration_result.is_ok(), "Migration should handle edge cases");

        // Verify all edge cases mapped to 'other' (since they're not in our mapping)
        let edge_case_mappings = sqlx::query!(
            "SELECT filename, failure_reason FROM failed_documents WHERE ingestion_source = 'migration_edge_test'"
        )
        .fetch_all(&pool)
        .await
        .expect("Failed to fetch edge case mappings");

        for mapping in edge_case_mappings {
            assert_eq!(mapping.failure_reason, "other", 
                      "Edge case '{}' should map to 'other'", mapping.filename);
        }
    }

    #[sqlx::test]
    async fn test_constraint_enforcement_during_migration(pool: PgPool) {
        // This test ensures that if we accidentally introduce invalid data
        // during migration, the constraints will catch it

        // Try to insert data that violates constraints
        let invalid_insert = sqlx::query!(
            r#"
            INSERT INTO failed_documents (
                user_id, filename, failure_reason, failure_stage, ingestion_source
            ) VALUES (
                gen_random_uuid(), 'invalid_test.pdf', 'migration_completed', 'migration', 'test'
            )
            "#
        )
        .execute(&pool)
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