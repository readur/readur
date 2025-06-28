use sqlx::PgPool;
use std::collections::HashSet;

/// Utility functions for validating database constraints at runtime
/// These help catch constraint violations early in development
pub struct ConstraintValidator;

impl ConstraintValidator {
    /// Validates that a failure_reason value is allowed by the failed_documents table constraint
    pub fn validate_failure_reason(reason: &str) -> Result<(), String> {
        let valid_reasons: HashSet<&str> = [
            "duplicate_content", "duplicate_filename", "unsupported_format",
            "file_too_large", "file_corrupted", "access_denied", 
            "low_ocr_confidence", "ocr_timeout", "ocr_memory_limit",
            "pdf_parsing_error", "storage_quota_exceeded", "network_error",
            "permission_denied", "virus_detected", "invalid_structure",
            "policy_violation", "other"
        ].iter().cloned().collect();

        if valid_reasons.contains(reason) {
            Ok(())
        } else {
            Err(format!(
                "Invalid failure_reason '{}'. Valid values are: {}",
                reason,
                valid_reasons.iter().cloned().collect::<Vec<_>>().join(", ")
            ))
        }
    }

    /// Validates that a failure_stage value is allowed by the failed_documents table constraint
    pub fn validate_failure_stage(stage: &str) -> Result<(), String> {
        let valid_stages: HashSet<&str> = [
            "ingestion", "validation", "ocr", "storage", "processing", "sync"
        ].iter().cloned().collect();

        if valid_stages.contains(stage) {
            Ok(())
        } else {
            Err(format!(
                "Invalid failure_stage '{}'. Valid values are: {}",
                stage,
                valid_stages.iter().cloned().collect::<Vec<_>>().join(", ")
            ))
        }
    }

    /// Maps legacy ocr_failure_reason values to new constraint-compliant values
    /// This ensures migration compatibility and prevents constraint violations
    pub fn map_legacy_ocr_failure_reason(legacy_reason: Option<&str>) -> &'static str {
        match legacy_reason {
            Some("low_ocr_confidence") => "low_ocr_confidence",
            Some("timeout") => "ocr_timeout",
            Some("memory_limit") => "ocr_memory_limit", 
            Some("pdf_parsing_error") => "pdf_parsing_error",
            Some("corrupted") | Some("file_corrupted") => "file_corrupted",
            Some("unsupported_format") => "unsupported_format",
            Some("access_denied") => "access_denied",
            Some("unknown") | None => "other",
            _ => "other", // Fallback for any unmapped values
        }
    }

    /// Validates that all values in a collection are valid failure reasons
    pub fn validate_failure_reasons_batch(reasons: &[&str]) -> Result<(), Vec<String>> {
        let errors: Vec<String> = reasons
            .iter()
            .filter_map(|&reason| Self::validate_failure_reason(reason).err())
            .collect();

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Tests database constraint enforcement by attempting to insert invalid data
    pub async fn test_constraint_enforcement(pool: &PgPool) -> Result<(), sqlx::Error> {
        // Test that invalid failure_reason is rejected
        let invalid_result = sqlx::query!(
            r#"
            INSERT INTO failed_documents (
                user_id, filename, failure_reason, failure_stage, ingestion_source
            ) VALUES (
                gen_random_uuid(), 'constraint_test.txt', 'invalid_reason', 'validation', 'test'
            )
            "#
        )
        .execute(pool)
        .await;

        // This should fail - if it succeeds, our constraints aren't working
        if invalid_result.is_ok() {
            return Err(sqlx::Error::Protocol("Database constraint validation failed - invalid data was accepted".into()));
        }

        // Test that valid data is accepted
        let valid_result = sqlx::query!(
            r#"
            INSERT INTO failed_documents (
                user_id, filename, failure_reason, failure_stage, ingestion_source
            ) VALUES (
                gen_random_uuid(), 'constraint_test_valid.txt', 'other', 'validation', 'test'
            )
            "#
        )
        .execute(pool)
        .await;

        if valid_result.is_err() {
            return Err(sqlx::Error::Protocol("Database constraint validation failed - valid data was rejected".into()));
        }

        // Clean up test data
        sqlx::query!(
            "DELETE FROM failed_documents WHERE filename LIKE 'constraint_test%'"
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_failure_reason_valid() {
        let valid_reasons = [
            "duplicate_content", "low_ocr_confidence", "other", "pdf_parsing_error"
        ];

        for reason in valid_reasons {
            assert!(ConstraintValidator::validate_failure_reason(reason).is_ok());
        }
    }

    #[test]
    fn test_validate_failure_reason_invalid() {
        let invalid_reasons = [
            "invalid_reason", "unknown", "timeout", "migration_completed"
        ];

        for reason in invalid_reasons {
            assert!(ConstraintValidator::validate_failure_reason(reason).is_err());
        }
    }

    #[test]
    fn test_map_legacy_ocr_failure_reason() {
        let test_cases = [
            (Some("low_ocr_confidence"), "low_ocr_confidence"),
            (Some("timeout"), "ocr_timeout"),
            (Some("memory_limit"), "ocr_memory_limit"),
            (Some("corrupted"), "file_corrupted"),
            (Some("unknown"), "other"),
            (None, "other"),
            (Some("unmapped_value"), "other"),
        ];

        for (input, expected) in test_cases {
            assert_eq!(
                ConstraintValidator::map_legacy_ocr_failure_reason(input),
                expected,
                "Failed for input: {:?}",
                input
            );
        }
    }

    #[test]
    fn test_validate_failure_reasons_batch() {
        let valid_batch = ["other", "low_ocr_confidence", "pdf_parsing_error"];
        assert!(ConstraintValidator::validate_failure_reasons_batch(&valid_batch).is_ok());

        let invalid_batch = ["other", "invalid_reason", "timeout"];
        assert!(ConstraintValidator::validate_failure_reasons_batch(&invalid_batch).is_err());
    }

    #[test]
    fn test_validate_failure_stage() {
        let valid_stages = ["ingestion", "validation", "ocr", "storage"];
        for stage in valid_stages {
            assert!(ConstraintValidator::validate_failure_stage(stage).is_ok());
        }

        let invalid_stages = ["invalid_stage", "processing_error", "unknown"];
        for stage in invalid_stages {
            assert!(ConstraintValidator::validate_failure_stage(stage).is_err());
        }
    }
}