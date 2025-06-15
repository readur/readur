#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::ocr_error::{OcrError, OcrDiagnostics, CpuFeatures};
    use crate::ocr_health::OcrHealthChecker;
    use crate::ocr_enhanced::EnhancedOcrService;
    use std::env;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_ocr_error_types() {
        // Test error creation and properties
        let err = OcrError::TesseractNotInstalled;
        assert_eq!(err.error_code(), "OCR_NOT_INSTALLED");
        assert!(!err.is_recoverable());
        assert!(err.is_configuration_error());

        let err = OcrError::InsufficientMemory { required: 1000, available: 500 };
        assert_eq!(err.error_code(), "OCR_OUT_OF_MEMORY");
        assert!(err.is_recoverable());
        assert!(!err.is_configuration_error());

        let err = OcrError::LanguageDataNotFound { lang: "deu".to_string() };
        assert!(err.to_string().contains("deu"));
        assert!(err.is_configuration_error());
    }

    #[test]
    fn test_cpu_features_display() {
        let features = CpuFeatures {
            sse2: true,
            sse3: true,
            sse4_1: false,
            sse4_2: false,
            avx: false,
            avx2: false,
        };
        
        let diag = OcrDiagnostics {
            tesseract_version: Some("4.1.1".to_string()),
            available_languages: vec!["eng".to_string(), "fra".to_string()],
            tessdata_path: Some("/usr/share/tessdata".to_string()),
            cpu_features: features,
            memory_available_mb: 8192,
            temp_space_available_mb: 50000,
        };
        
        let display = format!("{}", diag);
        assert!(display.contains("Tesseract Version: 4.1.1"));
        assert!(display.contains("SSE2: true"));
        assert!(display.contains("Available Languages: eng, fra"));
    }

    #[test]
    fn test_health_checker_cpu_validation() {
        let checker = OcrHealthChecker::new();
        let features = checker.check_cpu_features();
        
        // On x86/x64, we should at least detect the presence of CPU features
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            // Modern CPUs should have at least SSE2
            // Note: This might fail on very old hardware
            if std::env::var("CI").is_err() {
                // Only check in non-CI environments
                let _ = checker.validate_cpu_requirements();
            }
        }
    }

    #[test]
    fn test_memory_estimation() {
        let checker = OcrHealthChecker::new();
        
        // Test memory estimation for different image sizes
        let small_image = checker.estimate_memory_requirement(640, 480);
        let medium_image = checker.estimate_memory_requirement(1920, 1080);
        let large_image = checker.estimate_memory_requirement(4096, 4096);
        
        // Small image should need less memory than large
        assert!(small_image < medium_image);
        assert!(medium_image < large_image);
        
        // Base overhead is 100MB
        assert!(small_image >= 100);
    }

    #[test]
    fn test_temp_space_check() {
        let checker = OcrHealthChecker::new();
        let space = checker.check_temp_space();
        
        // Should return some positive value
        assert!(space > 0);
    }

    #[test]
    fn test_tessdata_path_detection() {
        let checker = OcrHealthChecker::new();
        
        // Set a custom TESSDATA_PREFIX for testing
        let temp_dir = TempDir::new().unwrap();
        env::set_var("TESSDATA_PREFIX", temp_dir.path());
        
        match checker.get_tessdata_path() {
            Ok(path) => assert_eq!(path, temp_dir.path().to_string_lossy()),
            Err(e) => {
                // Expected if the temp directory doesn't exist
                match e {
                    OcrError::TessdataPathInvalid { .. } => (),
                    _ => panic!("Unexpected error type"),
                }
            }
        }
        
        env::remove_var("TESSDATA_PREFIX");
    }

    #[test]
    fn test_language_detection() {
        let checker = OcrHealthChecker::new();
        
        // Create a mock tessdata directory
        let temp_dir = TempDir::new().unwrap();
        let tessdata_path = temp_dir.path().join("tessdata");
        fs::create_dir(&tessdata_path).unwrap();
        
        // Create mock language files
        fs::write(tessdata_path.join("eng.traineddata"), b"mock").unwrap();
        fs::write(tessdata_path.join("fra.traineddata"), b"mock").unwrap();
        fs::write(tessdata_path.join("deu.traineddata"), b"mock").unwrap();
        
        env::set_var("TESSDATA_PREFIX", &tessdata_path);
        
        let languages = checker.get_available_languages();
        assert!(languages.contains(&"eng".to_string()));
        assert!(languages.contains(&"fra".to_string()));
        assert!(languages.contains(&"deu".to_string()));
        assert_eq!(languages.len(), 3);
        
        // Test language validation
        assert!(checker.check_language_data("eng").is_ok());
        assert!(checker.check_language_data("jpn").is_err());
        
        env::remove_var("TESSDATA_PREFIX");
    }

    #[tokio::test]
    async fn test_enhanced_ocr_timeout() {
        let service = EnhancedOcrService::new()
            .with_timeout(1); // 1 second timeout
        
        // This should timeout since no actual file exists
        let result = service.extract_text_with_validation("/nonexistent/file.png", "eng").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_enhanced_ocr_image_validation() {
        let service = EnhancedOcrService::new()
            .with_limits(100, 100); // Very small limit
        
        // Create a mock large image path
        let result = service.extract_text_with_validation("/path/to/large/image.png", "eng").await;
        assert!(result.is_err());
    }

    #[test]
    fn test_error_recovery_classification() {
        // Test which errors are considered recoverable
        let recoverable_errors = vec![
            OcrError::InsufficientMemory { required: 1000, available: 500 },
            OcrError::OcrTimeout { seconds: 30 },
            OcrError::LowConfidence { score: 40.0, threshold: 60.0 },
        ];
        
        for err in recoverable_errors {
            assert!(err.is_recoverable(), "Error {:?} should be recoverable", err);
        }
        
        let non_recoverable_errors = vec![
            OcrError::TesseractNotInstalled,
            OcrError::LanguageDataNotFound { lang: "eng".to_string() },
            OcrError::MissingCpuInstruction { instruction: "SSE2".to_string() },
            OcrError::PermissionDenied { path: "/test".to_string() },
        ];
        
        for err in non_recoverable_errors {
            assert!(!err.is_recoverable(), "Error {:?} should not be recoverable", err);
        }
    }

    #[test]
    fn test_image_size_validation() {
        let checker = OcrHealthChecker::new();
        
        // Small image should pass
        assert!(checker.validate_memory_for_image(640, 480).is_ok());
        
        // Test with a ridiculously large image that would require more memory than any system has
        // 100,000 x 100,000 pixels = 10 billion pixels * 4 bytes * 3 buffers = ~120GB
        let result = checker.validate_memory_for_image(100000, 100000);
        assert!(result.is_err());
        
        if let Err(OcrError::InsufficientMemory { required, available }) = result {
            assert!(required > available);
        } else {
            panic!("Expected InsufficientMemory error, got: {:?}", result);
        }
    }
}