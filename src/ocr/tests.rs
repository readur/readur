#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::ocr::error::{OcrError, OcrDiagnostics, CpuFeatures};
    use crate::ocr::health::OcrHealthChecker;
    use crate::ocr::enhanced_processing::EnhancedOcrService;
    use std::env;
    use tempfile::TempDir;
    use std::fs;
    use std::time::Duration;


    #[test]
    fn test_ocr_error_types() {
        // Test error creation and properties
        let err = OcrError::TesseractNotInstalled;
        assert!(err.is_configuration_error());
        assert!(!err.is_recoverable());
        assert_eq!(err.error_code(), "OCR_NOT_INSTALLED");

        let err = OcrError::InsufficientMemory { required: 1000, available: 500 };
        assert!(!err.is_configuration_error());
        assert!(err.is_recoverable());
        assert_eq!(err.error_code(), "OCR_OUT_OF_MEMORY");

        let err = OcrError::LanguageDataNotFound { lang: "test".to_string() };
        assert!(err.is_configuration_error());
        assert!(!err.is_recoverable());
        assert_eq!(err.error_code(), "OCR_LANG_MISSING");
    }

    #[test]
    fn test_cpu_features_display() {
        let features = CpuFeatures {
            sse2: true,
            sse3: false,
            sse4_1: true,
            sse4_2: false,
            avx: false,
            avx2: true,
        };

        // Test that the structure can be created and accessed
        assert!(features.sse2);
        assert!(!features.sse3);
        assert!(features.sse4_1);
        assert!(!features.sse4_2);
        assert!(!features.avx);
        assert!(features.avx2);
    }

    #[test]
    fn test_health_checker_cpu_validation() {
        let checker = OcrHealthChecker::new();
        let _features = checker.check_cpu_features();
        // Just test that the method runs without panicking
        // Actual CPU features depend on the test environment
    }

    #[test]
    fn test_memory_estimation() {
        let checker = OcrHealthChecker::new();
        
        // Test memory estimation for different image sizes
        let small_image_mem = checker.estimate_memory_requirement(800, 600);
        let large_image_mem = checker.estimate_memory_requirement(4000, 3000);
        
        // Larger images should require more memory
        assert!(large_image_mem > small_image_mem);
        
        // Should include base overhead
        assert!(small_image_mem >= 100); // At least 100MB base
    }

    #[test]
    fn test_temp_space_check() {
        let checker = OcrHealthChecker::new();
        let space = checker.check_temp_space();
        
        // Should return some positive value
        assert!(space > 0);
    }

    // tessdata path detection test removed - no longer managing tessdata paths

    #[test]
    fn test_language_detection() {
        let checker = OcrHealthChecker::new();
        
        // Test that language detection methods exist and return proper types
        // These may fail in CI environments without tesseract, but should not panic
        let _available_languages_result = checker.get_available_languages();
        let _validate_result = checker.validate_language("eng");
    }

    #[tokio::test]
    async fn test_enhanced_ocr_timeout() {
        let _service = EnhancedOcrService::new()
            .with_timeout(1); // Very short timeout (1 second)
        
        // This should timeout quickly
        // Note: Actual test depends on having a test image file
    }

    #[tokio::test]
    async fn test_enhanced_ocr_image_validation() {
        let _service = EnhancedOcrService::new();
        
        // Test that the service can be created
        // Actual OCR tests would need test images
    }

    #[test]
    fn test_error_recovery_classification() {
        // Test which errors are considered recoverable
        assert!(OcrError::InsufficientMemory { required: 1000, available: 500 }.is_recoverable());
        assert!(OcrError::OcrTimeout { seconds: 30 }.is_recoverable());
        assert!(OcrError::LowConfidence { score: 0.3, threshold: 0.7 }.is_recoverable());
        
        // Test which errors are not recoverable
        assert!(!OcrError::TesseractNotInstalled.is_recoverable());
        assert!(!OcrError::LanguageDataNotFound { lang: "test".to_string() }.is_recoverable());
        assert!(!OcrError::MissingCpuInstruction { instruction: "SSE2".to_string() }.is_recoverable());
        
        // Test configuration errors
        assert!(OcrError::TesseractNotInstalled.is_configuration_error());
        assert!(OcrError::LanguageDataNotFound { lang: "test".to_string() }.is_configuration_error());
        assert!(OcrError::MissingCpuInstruction { instruction: "SSE2".to_string() }.is_configuration_error());
        
        assert!(!OcrError::InsufficientMemory { required: 1000, available: 500 }.is_configuration_error());
        assert!(!OcrError::OcrTimeout { seconds: 30 }.is_configuration_error());
    }

    #[test]
    fn test_image_size_validation() {
        let checker = OcrHealthChecker::new();
        
        // Test memory validation for different image sizes
        // Note: This might fail in low-memory environments, but shouldn't panic
        let result = checker.validate_memory_for_image(800, 600);
        
        // Should either succeed or fail with InsufficientMemory
        match result {
            Ok(_) => {
                // Memory validation passed
            }
            Err(OcrError::InsufficientMemory { required, available }) => {
                assert!(required > available);
            }
            Err(_) => {
                panic!("Expected InsufficientMemory error, got: {:?}", result);
            }
        }
    }

    #[test]
    fn test_get_language_display_name() {
        let health_checker = OcrHealthChecker::new();
        
        // Test known language display names
        assert_eq!(health_checker.get_language_display_name("eng"), "English");
        assert_eq!(health_checker.get_language_display_name("spa"), "Spanish");
        assert_eq!(health_checker.get_language_display_name("fra"), "French");
        assert_eq!(health_checker.get_language_display_name("deu"), "German");
        assert_eq!(health_checker.get_language_display_name("chi_sim"), "Chinese (Simplified)");
        
        // Test unknown language (should return the code itself)
        assert_eq!(health_checker.get_language_display_name("unknown"), "unknown");
    }

    #[test]
    fn test_language_validation_integration() {
        let health_checker = OcrHealthChecker::new();
        
        // Test that the new Tesseract-based validation methods exist and can be called
        // Note: These may fail if tesseract is not installed in test environment,
        // but we're testing the API exists and returns proper error types
        
        let result = health_checker.get_available_languages();
        match result {
            Ok(languages) => {
                // If tesseract is installed, we should get a list
                assert!(languages.len() >= 0);
            }
            Err(OcrError::TesseractNotInstalled) => {
                // This is expected in CI environments without tesseract
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
        
        let result = health_checker.check_language_data("eng");
        match result {
            Ok(_) => {
                // Language is available
            }
            Err(OcrError::TesseractNotInstalled) => {
                // Expected in CI without tesseract
            }
            Err(OcrError::LanguageDataNotFound { lang }) => {
                assert_eq!(lang, "eng");
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }
}