use sqlx::PgPool;
use crate::tests::helpers::setup_test_db;

#[cfg(test)]
mod migration_constraint_tests {
    use super::*;

    #[sqlx::test]
    async fn test_failed_documents_constraint_validation(pool: PgPool) {
        // Test that all allowed failure_reason values work
        let valid_reasons = vec![
            "duplicate_content", "duplicate_filename", "unsupported_format",
            "file_too_large", "file_corrupted", "access_denied", 
            "low_ocr_confidence", "ocr_timeout", "ocr_memory_limit",
            "pdf_parsing_error", "storage_quota_exceeded", "network_error",
            "permission_denied", "virus_detected", "invalid_structure",
            "policy_violation", "other"
        ];

        for reason in valid_reasons {
            let result = sqlx::query!(
                r#"
                INSERT INTO failed_documents (
                    user_id, filename, failure_reason, failure_stage, ingestion_source
                ) VALUES (
                    gen_random_uuid(), $1, $2, 'validation', 'test'
                )
                "#,
                format!("test_file_{}.txt", reason),
                reason
            )
            .execute(&pool)
            .await;

            assert!(result.is_ok(), "Valid failure_reason '{}' should be accepted", reason);
        }
    }

    #[sqlx::test]
    async fn test_failed_documents_invalid_constraint_rejection(pool: PgPool) {
        // Test that invalid failure_reason values are rejected
        let invalid_reasons = vec![
            "invalid_reason", "unknown", "timeout", "memory_limit", 
            "migration_completed", "corrupted", "unsupported"
        ];

        for reason in invalid_reasons {
            let result = sqlx::query!(
                r#"
                INSERT INTO failed_documents (
                    user_id, filename, failure_reason, failure_stage, ingestion_source
                ) VALUES (
                    gen_random_uuid(), $1, $2, 'validation', 'test'
                )
                "#,
                format!("test_file_{}.txt", reason),
                reason
            )
            .execute(&pool)
            .await;

            assert!(result.is_err(), "Invalid failure_reason '{}' should be rejected", reason);
        }
    }

    #[sqlx::test]
    async fn test_failed_documents_stage_constraint_validation(pool: PgPool) {
        // Test that all allowed failure_stage values work
        let valid_stages = vec![
            "ingestion", "validation", "ocr", "storage", "processing", "sync"
        ];

        for stage in valid_stages {
            let result = sqlx::query!(
                r#"
                INSERT INTO failed_documents (
                    user_id, filename, failure_reason, failure_stage, ingestion_source
                ) VALUES (
                    gen_random_uuid(), $1, 'other', $2, 'test'
                )
                "#,
                format!("test_file_{}.txt", stage),
                stage
            )
            .execute(&pool)
            .await;

            assert!(result.is_ok(), "Valid failure_stage '{}' should be accepted", stage);
        }
    }

    #[sqlx::test]
    async fn test_migration_mapping_compatibility(pool: PgPool) {
        // Test that the migration mapping logic matches our constraints
        let migration_mappings = vec![
            ("low_ocr_confidence", "low_ocr_confidence"),
            ("timeout", "ocr_timeout"),
            ("memory_limit", "ocr_memory_limit"),
            ("pdf_parsing_error", "pdf_parsing_error"),
            ("corrupted", "file_corrupted"),
            ("file_corrupted", "file_corrupted"),
            ("unsupported_format", "unsupported_format"),
            ("access_denied", "access_denied"),
            ("unknown_value", "other"), // fallback case
            ("", "other"), // empty case
        ];

        for (input_reason, expected_output) in migration_mappings {
            // Simulate the migration CASE logic
            let mapped_reason = match input_reason {
                "low_ocr_confidence" => "low_ocr_confidence",
                "timeout" => "ocr_timeout",
                "memory_limit" => "ocr_memory_limit",
                "pdf_parsing_error" => "pdf_parsing_error",
                "corrupted" | "file_corrupted" => "file_corrupted",
                "unsupported_format" => "unsupported_format",
                "access_denied" => "access_denied",
                _ => "other",
            };

            assert_eq!(mapped_reason, expected_output, 
                      "Migration mapping for '{}' should produce '{}'", 
                      input_reason, expected_output);

            // Test that the mapped value works in the database
            let result = sqlx::query!(
                r#"
                INSERT INTO failed_documents (
                    user_id, filename, failure_reason, failure_stage, ingestion_source
                ) VALUES (
                    gen_random_uuid(), $1, $2, 'ocr', 'migration'
                )
                "#,
                format!("migration_test_{}.txt", input_reason.replace("/", "_")),
                mapped_reason
            )
            .execute(&pool)
            .await;

            assert!(result.is_ok(), 
                   "Mapped failure_reason '{}' (from '{}') should be accepted by constraints", 
                   mapped_reason, input_reason);
        }
    }
}