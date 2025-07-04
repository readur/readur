use readur::test_utils::TestContext;
use uuid;

#[cfg(test)]
mod migration_constraint_tests {
    use super::*;

    #[tokio::test]
    async fn test_failed_documents_constraint_validation() {
        let ctx = TestContext::new().await;
        let pool = ctx.state.db.get_pool();
        
        // Create a test user first to avoid foreign key constraint violations
        let user_id = uuid::Uuid::new_v4();
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
            let result = sqlx::query(
                r#"
                INSERT INTO failed_documents (
                    user_id, filename, failure_reason, failure_stage, ingestion_source
                ) VALUES (
                    $1, $2, $3, 'validation', 'test'
                )
                "#
            )
            .bind(user_id)
            .bind(format!("test_file_{}.txt", reason))
            .bind(reason)
            .execute(pool)
            .await;

            assert!(result.is_ok(), "Valid failure_reason '{}' should be accepted", reason);
        }
    }

    #[tokio::test]
    async fn test_failed_documents_invalid_constraint_rejection() {
        let ctx = TestContext::new().await;
        let pool = ctx.state.db.get_pool();
        
        // Create a test user first to avoid foreign key constraint violations
        let user_id = uuid::Uuid::new_v4();
        sqlx::query(
            "INSERT INTO users (id, username, email, password_hash, role) 
             VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(user_id)
        .bind("test_invalid_user")
        .bind("test_invalid@example.com")
        .bind("hash")
        .bind("user")
        .execute(pool)
        .await
        .unwrap();
        
        // Test that invalid failure_reason values are rejected
        let invalid_reasons = vec![
            "invalid_reason", "unknown", "timeout", "memory_limit", 
            "migration_completed", "corrupted", "unsupported"
        ];

        for reason in invalid_reasons {
            let result = sqlx::query(
                r#"
                INSERT INTO failed_documents (
                    user_id, filename, failure_reason, failure_stage, ingestion_source
                ) VALUES (
                    $1, $2, $3, 'validation', 'test'
                )
                "#
            )
            .bind(user_id)
            .bind(format!("test_file_{}.txt", reason))
            .bind(reason)
            .execute(pool)
            .await;

            assert!(result.is_err(), "Invalid failure_reason '{}' should be rejected", reason);
        }
    }

    #[tokio::test]
    async fn test_failed_documents_stage_constraint_validation() {
        let ctx = TestContext::new().await;
        let pool = ctx.state.db.get_pool();
        
        // Create a test user first to avoid foreign key constraint violations
        let user_id = uuid::Uuid::new_v4();
        sqlx::query(
            "INSERT INTO users (id, username, email, password_hash, role) 
             VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(user_id)
        .bind("test_stage_user")
        .bind("test_stage@example.com")
        .bind("hash")
        .bind("user")
        .execute(pool)
        .await
        .unwrap();
        
        // Test that all allowed failure_stage values work
        let valid_stages = vec![
            "ingestion", "validation", "ocr", "storage", "processing", "sync"
        ];

        for stage in valid_stages {
            let result = sqlx::query(
                r#"
                INSERT INTO failed_documents (
                    user_id, filename, failure_reason, failure_stage, ingestion_source
                ) VALUES (
                    $1, $2, 'other', $3, 'test'
                )
                "#
            )
            .bind(user_id)
            .bind(format!("test_file_{}.txt", stage))
            .bind(stage)
            .execute(pool)
            .await;

            assert!(result.is_ok(), "Valid failure_stage '{}' should be accepted", stage);
        }
    }

    #[tokio::test]
    async fn test_migration_mapping_compatibility() {
        let ctx = TestContext::new().await;
        let pool = ctx.state.db.get_pool();
        
        // Create a test user first to avoid foreign key constraint violations
        let user_id = uuid::Uuid::new_v4();
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
            let result = sqlx::query(
                r#"
                INSERT INTO failed_documents (
                    user_id, filename, failure_reason, failure_stage, ingestion_source
                ) VALUES (
                    $1, $2, $3, 'ocr', 'migration'
                )
                "#
            )
            .bind(user_id)
            .bind(format!("migration_test_{}.txt", input_reason.replace("/", "_")))
            .bind(mapped_reason)
            .execute(pool)
            .await;

            assert!(result.is_ok(), 
                   "Mapped failure_reason '{}' (from '{}') should be accepted by constraints", 
                   mapped_reason, input_reason);
        }
    }
}