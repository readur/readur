use readur::db::constraint_validation::ConstraintValidator;

/// Simple unit tests for failed_documents functionality
/// These tests focus on business logic and constraint validation
/// without requiring live database connections during compilation
#[cfg(test)]
mod failed_documents_unit_tests {
    use super::*;

    #[test]
    fn test_constraint_validator_failure_reasons() {
        // Test all valid failure reasons
        let valid_reasons = [
            "duplicate_content", "duplicate_filename", "unsupported_format",
            "file_too_large", "file_corrupted", "access_denied", 
            "low_ocr_confidence", "ocr_timeout", "ocr_memory_limit",
            "pdf_parsing_error", "storage_quota_exceeded", "network_error",
            "permission_denied", "virus_detected", "invalid_structure",
            "policy_violation", "other"
        ];

        for reason in valid_reasons {
            assert!(
                ConstraintValidator::validate_failure_reason(reason).is_ok(),
                "Expected '{}' to be valid",
                reason
            );
        }

        // Test invalid failure reasons
        let invalid_reasons = [
            "invalid_reason", "unknown", "timeout", "migration_completed",
            "", "random_text", "failure", "error"
        ];

        for reason in invalid_reasons {
            assert!(
                ConstraintValidator::validate_failure_reason(reason).is_err(),
                "Expected '{}' to be invalid",
                reason
            );
        }
    }

    #[test]
    fn test_constraint_validator_failure_stages() {
        // Test all valid failure stages
        let valid_stages = [
            "ingestion", "validation", "ocr", "storage", "processing", "sync"
        ];

        for stage in valid_stages {
            assert!(
                ConstraintValidator::validate_failure_stage(stage).is_ok(),
                "Expected '{}' to be valid",
                stage
            );
        }

        // Test invalid failure stages
        let invalid_stages = [
            "invalid_stage", "unknown", "failed", "error", "", "random_text"
        ];

        for stage in invalid_stages {
            assert!(
                ConstraintValidator::validate_failure_stage(stage).is_err(),
                "Expected '{}' to be invalid",
                stage
            );
        }
    }

    #[test]
    fn test_legacy_ocr_failure_mapping() {
        let test_cases = [
            (Some("low_ocr_confidence"), "low_ocr_confidence"),
            (Some("timeout"), "ocr_timeout"),
            (Some("memory_limit"), "ocr_memory_limit"),
            (Some("pdf_parsing_error"), "pdf_parsing_error"),
            (Some("corrupted"), "file_corrupted"),
            (Some("file_corrupted"), "file_corrupted"),
            (Some("unsupported_format"), "unsupported_format"),
            (Some("access_denied"), "access_denied"),
            (Some("unknown"), "other"),
            (None, "other"),
            (Some("unmapped_value"), "other"),
            (Some(""), "other"),
        ];

        for (input, expected) in test_cases {
            let result = ConstraintValidator::map_legacy_ocr_failure_reason(input);
            assert_eq!(
                result, expected,
                "Failed for input: {:?}. Expected '{}', got '{}'",
                input, expected, result
            );
        }
    }

    #[test]
    fn test_mapped_legacy_values_are_valid() {
        // Ensure all mapped legacy values are actually valid according to our constraints
        let legacy_values = [
            Some("low_ocr_confidence"),
            Some("timeout"), 
            Some("memory_limit"),
            Some("pdf_parsing_error"),
            Some("corrupted"),
            Some("file_corrupted"),
            Some("unsupported_format"),
            Some("access_denied"),
            Some("unknown"),
            None,
            Some("random_unmapped_value"),
        ];

        for legacy_value in legacy_values {
            let mapped = ConstraintValidator::map_legacy_ocr_failure_reason(legacy_value);
            assert!(
                ConstraintValidator::validate_failure_reason(mapped).is_ok(),
                "Mapped value '{}' from legacy '{:?}' should be valid",
                mapped, legacy_value
            );
        }
    }

    #[test]
    fn test_batch_validation() {
        // Test valid batch
        let valid_batch = ["other", "low_ocr_confidence", "pdf_parsing_error", "duplicate_content"];
        assert!(ConstraintValidator::validate_failure_reasons_batch(&valid_batch).is_ok());

        // Test invalid batch
        let invalid_batch = ["other", "invalid_reason", "timeout", "low_ocr_confidence"];
        let result = ConstraintValidator::validate_failure_reasons_batch(&invalid_batch);
        assert!(result.is_err());
        
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 2); // Should have 2 invalid reasons
        assert!(errors.iter().any(|e| e.contains("invalid_reason")));
        assert!(errors.iter().any(|e| e.contains("timeout")));

        // Test empty batch
        let empty_batch: &[&str] = &[];
        assert!(ConstraintValidator::validate_failure_reasons_batch(empty_batch).is_ok());
    }

    #[test]
    fn test_constraint_error_messages() {
        let result = ConstraintValidator::validate_failure_reason("invalid_reason");
        assert!(result.is_err());
        
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("Invalid failure_reason 'invalid_reason'"));
        assert!(error_msg.contains("Valid values are:"));
        assert!(error_msg.contains("low_ocr_confidence"));
        assert!(error_msg.contains("other"));

        let stage_result = ConstraintValidator::validate_failure_stage("invalid_stage");
        assert!(stage_result.is_err());
        
        let stage_error = stage_result.unwrap_err();
        assert!(stage_error.contains("Invalid failure_stage 'invalid_stage'"));
        assert!(stage_error.contains("Valid values are:"));
        assert!(stage_error.contains("ingestion"));
        assert!(stage_error.contains("ocr"));
    }

    #[test]
    fn test_constraint_validation_comprehensive() {
        // Test that our enum values comprehensively cover expected failure scenarios
        
        // OCR-related failures
        assert!(ConstraintValidator::validate_failure_reason("low_ocr_confidence").is_ok());
        assert!(ConstraintValidator::validate_failure_reason("ocr_timeout").is_ok());
        assert!(ConstraintValidator::validate_failure_reason("ocr_memory_limit").is_ok());
        assert!(ConstraintValidator::validate_failure_reason("pdf_parsing_error").is_ok());

        // File-related failures
        assert!(ConstraintValidator::validate_failure_reason("file_too_large").is_ok());
        assert!(ConstraintValidator::validate_failure_reason("file_corrupted").is_ok());
        assert!(ConstraintValidator::validate_failure_reason("unsupported_format").is_ok());
        assert!(ConstraintValidator::validate_failure_reason("access_denied").is_ok());

        // Duplicate detection
        assert!(ConstraintValidator::validate_failure_reason("duplicate_content").is_ok());
        assert!(ConstraintValidator::validate_failure_reason("duplicate_filename").is_ok());

        // System-related failures
        assert!(ConstraintValidator::validate_failure_reason("storage_quota_exceeded").is_ok());
        assert!(ConstraintValidator::validate_failure_reason("network_error").is_ok());
        assert!(ConstraintValidator::validate_failure_reason("permission_denied").is_ok());

        // Security-related failures
        assert!(ConstraintValidator::validate_failure_reason("virus_detected").is_ok());
        assert!(ConstraintValidator::validate_failure_reason("policy_violation").is_ok());
        assert!(ConstraintValidator::validate_failure_reason("invalid_structure").is_ok());

        // Fallback
        assert!(ConstraintValidator::validate_failure_reason("other").is_ok());
    }

    #[test]
    fn test_failure_stages_comprehensive() {
        // Test that our stage enum covers the document processing pipeline
        
        // Initial processing stages
        assert!(ConstraintValidator::validate_failure_stage("ingestion").is_ok());
        assert!(ConstraintValidator::validate_failure_stage("validation").is_ok());
        
        // Core processing stages
        assert!(ConstraintValidator::validate_failure_stage("ocr").is_ok());
        assert!(ConstraintValidator::validate_failure_stage("processing").is_ok());
        
        // Storage and sync stages
        assert!(ConstraintValidator::validate_failure_stage("storage").is_ok());
        assert!(ConstraintValidator::validate_failure_stage("sync").is_ok());
    }

    #[test]
    fn test_legacy_mapping_completeness() {
        // Ensure we handle all possible legacy OCR failure reasons that could exist
        let legacy_ocr_reasons = [
            "low_ocr_confidence",
            "timeout", 
            "memory_limit",
            "pdf_parsing_error",
            "corrupted",
            "file_corrupted", 
            "unsupported_format",
            "access_denied",
            "unknown",
            "some_new_unmapped_reason"
        ];

        for legacy_reason in legacy_ocr_reasons {
            let mapped = ConstraintValidator::map_legacy_ocr_failure_reason(Some(legacy_reason));
            
            // All mapped values should be valid
            assert!(
                ConstraintValidator::validate_failure_reason(mapped).is_ok(),
                "Legacy reason '{}' maps to '{}' which should be valid",
                legacy_reason, mapped
            );
            
            // Unmapped values should fall back to "other"
            if !["low_ocr_confidence", "timeout", "memory_limit", "pdf_parsing_error", 
                  "corrupted", "file_corrupted", "unsupported_format", "access_denied", "unknown"]
                .contains(&legacy_reason) {
                assert_eq!(mapped, "other", "Unmapped legacy reason should fall back to 'other'");
            }
        }
    }

    #[test]
    fn test_case_sensitivity() {
        // Our validation should be case-sensitive
        assert!(ConstraintValidator::validate_failure_reason("Low_OCR_Confidence").is_err());
        assert!(ConstraintValidator::validate_failure_reason("LOW_OCR_CONFIDENCE").is_err());
        assert!(ConstraintValidator::validate_failure_reason("OCR").is_err());
        assert!(ConstraintValidator::validate_failure_reason("INGESTION").is_err());
        
        // Only exact lowercase matches should work
        assert!(ConstraintValidator::validate_failure_reason("low_ocr_confidence").is_ok());
        assert!(ConstraintValidator::validate_failure_stage("ocr").is_ok());
        assert!(ConstraintValidator::validate_failure_stage("ingestion").is_ok());
    }

    #[test]
    fn test_whitespace_handling() {
        // Validation should not accept values with extra whitespace
        assert!(ConstraintValidator::validate_failure_reason(" low_ocr_confidence").is_err());
        assert!(ConstraintValidator::validate_failure_reason("low_ocr_confidence ").is_err());
        assert!(ConstraintValidator::validate_failure_reason(" low_ocr_confidence ").is_err());
        assert!(ConstraintValidator::validate_failure_stage(" ocr").is_err());
        assert!(ConstraintValidator::validate_failure_stage("ocr ").is_err());
        
        // Only exact matches should work
        assert!(ConstraintValidator::validate_failure_reason("low_ocr_confidence").is_ok());
        assert!(ConstraintValidator::validate_failure_stage("ocr").is_ok());
    }
}