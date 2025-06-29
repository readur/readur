#[cfg(test)]
mod language_validation_tests {
    use super::super::health::{OcrHealthChecker, OcrError};
    use std::path::Path;
    use tempfile::TempDir;
    use std::fs;

    fn create_test_health_checker() -> (OcrHealthChecker, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let tessdata_path = temp_dir.path().join("tessdata");
        fs::create_dir_all(&tessdata_path).expect("Failed to create tessdata directory");
        
        // Create mock language files
        let language_files = vec![
            "eng.traineddata",
            "spa.traineddata", 
            "fra.traineddata",
            "deu.traineddata",
            "chi_sim.traineddata",
        ];
        
        for file in language_files {
            fs::write(tessdata_path.join(file), "mock data")
                .expect("Failed to create mock language file");
        }
        
        let health_checker = OcrHealthChecker::new(tessdata_path);
        (health_checker, temp_dir)
    }

    #[test]
    fn test_get_available_languages_success() {
        let (health_checker, _temp_dir) = create_test_health_checker();
        
        let result = health_checker.get_available_languages();
        assert!(result.is_ok());
        
        let languages = result.unwrap();
        assert_eq!(languages.len(), 5);
        assert!(languages.contains(&"eng".to_string()));
        assert!(languages.contains(&"spa".to_string()));
        assert!(languages.contains(&"fra".to_string()));
        assert!(languages.contains(&"deu".to_string()));
        assert!(languages.contains(&"chi_sim".to_string()));
    }

    #[test]
    fn test_get_available_languages_empty_directory() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let tessdata_path = temp_dir.path().join("tessdata");
        fs::create_dir_all(&tessdata_path).expect("Failed to create tessdata directory");
        
        let health_checker = OcrHealthChecker::new(tessdata_path);
        let result = health_checker.get_available_languages();
        
        assert!(result.is_ok());
        let languages = result.unwrap();
        assert!(languages.is_empty());
    }

    #[test]
    fn test_get_available_languages_nonexistent_directory() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let nonexistent_path = temp_dir.path().join("nonexistent");
        
        let health_checker = OcrHealthChecker::new(nonexistent_path);
        let result = health_checker.get_available_languages();
        
        assert!(result.is_err());
        match result.unwrap_err() {
            OcrError::TessdataPathNotFound { .. } => {},
            _ => panic!("Expected TessdataPathNotFound error"),
        }
    }

    #[test]
    fn test_validate_language_success() {
        let (health_checker, _temp_dir) = create_test_health_checker();
        
        // Test valid languages
        assert!(health_checker.validate_language("eng").is_ok());
        assert!(health_checker.validate_language("spa").is_ok());
        assert!(health_checker.validate_language("fra").is_ok());
        assert!(health_checker.validate_language("deu").is_ok());
        assert!(health_checker.validate_language("chi_sim").is_ok());
    }

    #[test]
    fn test_validate_language_invalid() {
        let (health_checker, _temp_dir) = create_test_health_checker();
        
        // Test invalid languages
        let result = health_checker.validate_language("invalid");
        assert!(result.is_err());
        match result.unwrap_err() {
            OcrError::LanguageDataNotFound { lang } => {
                assert_eq!(lang, "invalid");
            },
            _ => panic!("Expected LanguageDataNotFound error"),
        }
    }

    #[test]
    fn test_validate_language_empty_string() {
        let (health_checker, _temp_dir) = create_test_health_checker();
        
        let result = health_checker.validate_language("");
        assert!(result.is_err());
        match result.unwrap_err() {
            OcrError::LanguageDataNotFound { lang } => {
                assert_eq!(lang, "");
            },
            _ => panic!("Expected LanguageDataNotFound error"),
        }
    }

    #[test]
    fn test_validate_language_case_sensitive() {
        let (health_checker, _temp_dir) = create_test_health_checker();
        
        // Should be case sensitive
        assert!(health_checker.validate_language("eng").is_ok());
        
        let result = health_checker.validate_language("ENG");
        assert!(result.is_err());
        match result.unwrap_err() {
            OcrError::LanguageDataNotFound { lang } => {
                assert_eq!(lang, "ENG");
            },
            _ => panic!("Expected LanguageDataNotFound error"),
        }
    }

    #[test]
    fn test_validate_language_with_special_characters() {
        let (health_checker, _temp_dir) = create_test_health_checker();
        
        // chi_sim contains underscore
        assert!(health_checker.validate_language("chi_sim").is_ok());
        
        // Test invalid special characters
        let result = health_checker.validate_language("chi-sim");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_language_whitespace() {
        let (health_checker, _temp_dir) = create_test_health_checker();
        
        // Test with leading/trailing whitespace
        let result = health_checker.validate_language(" eng ");
        assert!(result.is_err());
        
        let result = health_checker.validate_language("eng ");
        assert!(result.is_err());
        
        let result = health_checker.validate_language(" eng");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_language_display_name() {
        let (health_checker, _temp_dir) = create_test_health_checker();
        
        // Test known language codes
        assert_eq!(health_checker.get_language_display_name("eng"), "English");
        assert_eq!(health_checker.get_language_display_name("spa"), "Spanish");
        assert_eq!(health_checker.get_language_display_name("fra"), "French");
        assert_eq!(health_checker.get_language_display_name("deu"), "German");
        assert_eq!(health_checker.get_language_display_name("chi_sim"), "Chinese (Simplified)");
        
        // Test unknown language code (should return the code itself)
        assert_eq!(health_checker.get_language_display_name("unknown"), "unknown");
    }

    #[test]
    fn test_concurrent_language_validation() {
        use std::sync::Arc;
        use std::thread;
        
        let (health_checker, _temp_dir) = create_test_health_checker();
        let health_checker = Arc::new(health_checker);
        
        let mut handles = vec![];
        
        // Test concurrent validation of different languages
        for lang in &["eng", "spa", "fra", "deu", "chi_sim"] {
            let hc = Arc::clone(&health_checker);
            let lang = lang.to_string();
            let handle = thread::spawn(move || {
                hc.validate_language(&lang)
            });
            handles.push(handle);
        }
        
        // All validations should succeed
        for handle in handles {
            let result = handle.join().expect("Thread panicked");
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_languages_alphabetically_sorted() {
        let (health_checker, _temp_dir) = create_test_health_checker();
        
        let languages = health_checker.get_available_languages().unwrap();
        let mut sorted_languages = languages.clone();
        sorted_languages.sort();
        
        assert_eq!(languages, sorted_languages, "Languages should be sorted alphabetically");
    }

    #[test]
    fn test_ignore_non_traineddata_files() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let tessdata_path = temp_dir.path().join("tessdata");
        fs::create_dir_all(&tessdata_path).expect("Failed to create tessdata directory");
        
        // Create mix of valid and invalid files
        let files = vec![
            "eng.traineddata",    // Valid
            "readme.txt",         // Invalid - not .traineddata
            "spa.traineddata",    // Valid
            "config.json",        // Invalid - not .traineddata
            "fra.backup",         // Invalid - not .traineddata
            "deu.traineddata",    // Valid
        ];
        
        for file in files {
            fs::write(tessdata_path.join(file), "mock data")
                .expect("Failed to create mock file");
        }
        
        let health_checker = OcrHealthChecker::new(tessdata_path);
        let languages = health_checker.get_available_languages().unwrap();
        
        // Should only include .traineddata files
        assert_eq!(languages.len(), 3);
        assert!(languages.contains(&"eng".to_string()));
        assert!(languages.contains(&"spa".to_string()));
        assert!(languages.contains(&"deu".to_string()));
    }

    #[test]
    fn test_handle_permission_errors() {
        // This test simulates permission errors by using a non-readable directory
        // Note: This may not work on all systems, particularly Windows
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            
            let temp_dir = TempDir::new().expect("Failed to create temp directory");
            let tessdata_path = temp_dir.path().join("tessdata");
            fs::create_dir_all(&tessdata_path).expect("Failed to create tessdata directory");
            
            // Remove read permissions
            let mut perms = fs::metadata(&tessdata_path).unwrap().permissions();
            perms.set_mode(0o000);
            fs::set_permissions(&tessdata_path, perms).unwrap();
            
            let health_checker = OcrHealthChecker::new(&tessdata_path);
            let result = health_checker.get_available_languages();
            
            // Should handle permission error gracefully
            assert!(result.is_err());
            
            // Restore permissions for cleanup
            let mut perms = fs::metadata(&tessdata_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&tessdata_path, perms).unwrap();
        }
    }

    #[test]
    fn test_validate_multiple_languages_batch() {
        let (health_checker, _temp_dir) = create_test_health_checker();
        
        let languages_to_test = vec![
            ("eng", true),
            ("spa", true),
            ("fra", true),
            ("invalid", false),
            ("", false),
            ("ENG", false),
            ("chi_sim", true),
        ];
        
        for (lang, should_be_valid) in languages_to_test {
            let result = health_checker.validate_language(lang);
            if should_be_valid {
                assert!(result.is_ok(), "Language '{}' should be valid", lang);
            } else {
                assert!(result.is_err(), "Language '{}' should be invalid", lang);
            }
        }
    }
}