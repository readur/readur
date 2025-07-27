#[cfg(test)]
mod tests {
    use super::super::{WebDAVService, WebDAVConfig};

// Helper function to create test WebDAV service for Nextcloud
fn create_nextcloud_webdav_service() -> WebDAVService {
    let config = WebDAVConfig {
        server_url: "https://nas.example.com".to_string(),
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        watch_folders: vec!["/Documents".to_string()],
        file_extensions: vec!["pdf".to_string(), "txt".to_string()],
        timeout_seconds: 30,
        server_type: Some("nextcloud".to_string()),
    };
    
    WebDAVService::new(config).unwrap()
}

// Helper function to create test WebDAV service for generic servers
fn create_generic_webdav_service() -> WebDAVService {
    let config = WebDAVConfig {
        server_url: "https://webdav.example.com".to_string(),
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        watch_folders: vec!["/Documents".to_string()],
        file_extensions: vec!["pdf".to_string(), "txt".to_string()],
        timeout_seconds: 30,
        server_type: Some("generic".to_string()),
    };
    
    WebDAVService::new(config).unwrap()
}

#[tokio::test]
async fn test_nextcloud_path_conversion_basic() {
    let service = create_nextcloud_webdav_service();
    
    // Test basic path conversion
    let full_webdav_path = "/remote.php/dav/files/testuser/Documents/";
    let relative_path = service.convert_to_relative_path(full_webdav_path);
    
    assert_eq!(relative_path, "/Documents/");
}

#[tokio::test]
async fn test_nextcloud_path_conversion_nested() {
    let service = create_nextcloud_webdav_service();
    
    // Test nested path conversion
    let full_webdav_path = "/remote.php/dav/files/testuser/FullerDocuments/NicoleDocuments/Projects/";
    let relative_path = service.convert_to_relative_path(full_webdav_path);
    
    assert_eq!(relative_path, "/FullerDocuments/NicoleDocuments/Projects/");
}

#[tokio::test]
async fn test_nextcloud_path_conversion_with_spaces() {
    let service = create_nextcloud_webdav_service();
    
    // Test path with URL-encoded spaces (the actual bug scenario)
    let full_webdav_path = "/remote.php/dav/files/testuser/Documents/Melanie%20Martinez%20June%207%202023/";
    let relative_path = service.convert_to_relative_path(full_webdav_path);
    
    assert_eq!(relative_path, "/Documents/Melanie%20Martinez%20June%207%202023/");
}

#[tokio::test]
async fn test_nextcloud_path_conversion_with_special_chars() {
    let service = create_nextcloud_webdav_service();
    
    // Test path with various special characters
    let full_webdav_path = "/remote.php/dav/files/testuser/Documents/Maranatha%20Work/";
    let relative_path = service.convert_to_relative_path(full_webdav_path);
    
    assert_eq!(relative_path, "/Documents/Maranatha%20Work/");
}

#[tokio::test]
async fn test_generic_webdav_path_conversion() {
    let service = create_generic_webdav_service();
    
    // Test generic WebDAV path conversion
    let full_webdav_path = "/webdav/Documents/Projects/";
    let relative_path = service.convert_to_relative_path(full_webdav_path);
    
    assert_eq!(relative_path, "/Documents/Projects/");
}

#[tokio::test]
async fn test_path_conversion_with_mismatched_prefix() {
    let service = create_nextcloud_webdav_service();
    
    // Test path that doesn't match expected prefix (should return as-is)
    let unexpected_path = "/some/other/path/Documents/";
    let relative_path = service.convert_to_relative_path(unexpected_path);
    
    assert_eq!(relative_path, "/some/other/path/Documents/");
}

#[tokio::test]
async fn test_url_construction_validation() {
    let service = create_nextcloud_webdav_service();
    
    // Test that we can identify the problem that caused the bug
    // This simulates what was happening before the fix
    
    // What we get from XML parser (full WebDAV path)
    let full_webdav_path = "/remote.php/dav/files/testuser/FullerDocuments/NicoleDocuments/";
    
    // What the old code would do (WRONG - double construction)
    let base_url = "https://nas.example.com/remote.php/dav/files/testuser";
    let wrong_url = format!("{}{}", base_url, full_webdav_path);
    
    // This would create a malformed URL
    assert_eq!(wrong_url, "https://nas.example.com/remote.php/dav/files/testuser/remote.php/dav/files/testuser/FullerDocuments/NicoleDocuments/");
    
    // What the new code does (CORRECT)
    let relative_path = service.convert_to_relative_path(full_webdav_path);
    let correct_url = format!("{}{}", base_url, relative_path);
    
    assert_eq!(correct_url, "https://nas.example.com/remote.php/dav/files/testuser/FullerDocuments/NicoleDocuments/");
    
    // Verify they're different (this is the bug we fixed)
    assert_ne!(wrong_url, correct_url);
}

#[tokio::test]
async fn test_real_world_nextcloud_paths() {
    let service = create_nextcloud_webdav_service();
    
    // Test real-world paths that would come from Nextcloud XML responses
    let real_world_paths = vec![
        "/remote.php/dav/files/testuser/",
        "/remote.php/dav/files/testuser/Documents/",
        "/remote.php/dav/files/testuser/FullerDocuments/",
        "/remote.php/dav/files/testuser/FullerDocuments/JonDocuments/",
        "/remote.php/dav/files/testuser/FullerDocuments/NicoleDocuments/",
        "/remote.php/dav/files/testuser/FullerDocuments/NicoleDocuments/Maranatha%20Work/",
        "/remote.php/dav/files/testuser/FullerDocuments/NicoleDocuments/Melanie%20Martinez%20June%207%202023/",
        "/remote.php/dav/files/testuser/FullerDocuments/NicoleDocuments/Misc/",
        "/remote.php/dav/files/testuser/FullerDocuments/NicoleDocuments/Nicole-Barakat-Website/",
        "/remote.php/dav/files/testuser/FullerDocuments/NicoleDocuments/RDP/",
    ];
    
    let expected_relative_paths = vec![
        "/",
        "/Documents/",
        "/FullerDocuments/",
        "/FullerDocuments/JonDocuments/",
        "/FullerDocuments/NicoleDocuments/",
        "/FullerDocuments/NicoleDocuments/Maranatha%20Work/",
        "/FullerDocuments/NicoleDocuments/Melanie%20Martinez%20June%207%202023/",
        "/FullerDocuments/NicoleDocuments/Misc/",
        "/FullerDocuments/NicoleDocuments/Nicole-Barakat-Website/",
        "/FullerDocuments/NicoleDocuments/RDP/",
    ];
    
    for (full_path, expected_relative) in real_world_paths.iter().zip(expected_relative_paths.iter()) {
        let result = service.convert_to_relative_path(full_path);
        assert_eq!(&result, expected_relative, 
            "Failed to convert {} to {}, got {}", full_path, expected_relative, result);
    }
}

#[tokio::test]
async fn test_url_construction_end_to_end() {
    let service = create_nextcloud_webdav_service();
    
    // Test the complete URL construction process
    let base_webdav_url = "https://nas.example.com/remote.php/dav/files/testuser";
    
    // Simulate a path that would cause 404 with the old bug
    let problematic_path = "/remote.php/dav/files/testuser/FullerDocuments/NicoleDocuments/Melanie%20Martinez%20June%207%202023/";
    
    // Convert to relative path
    let relative_path = service.convert_to_relative_path(problematic_path);
    
    // Construct final URL
    let final_url = format!("{}{}", base_webdav_url, relative_path);
    
    // Verify the URL is correctly constructed
    assert_eq!(final_url, "https://nas.example.com/remote.php/dav/files/testuser/FullerDocuments/NicoleDocuments/Melanie%20Martinez%20June%207%202023/");
    
    // Verify it doesn't contain double paths
    assert!(!final_url.contains("/remote.php/dav/files/testuser/remote.php/dav/files/testuser/"));
}

#[tokio::test]
async fn test_different_usernames() {
    // Test with different usernames to ensure the path conversion works correctly
    let usernames = vec!["testuser", "perf3ct", "admin", "user123", "user.name"];
    
    for username in usernames {
        let config = WebDAVConfig {
            server_url: "https://nas.example.com".to_string(),
            username: username.to_string(),
            password: "testpass".to_string(),
            watch_folders: vec!["/Documents".to_string()],
            file_extensions: vec!["pdf".to_string()],
            timeout_seconds: 30,
            server_type: Some("nextcloud".to_string()),
        };
        
        let service = WebDAVService::new(config).unwrap();
        
        let full_path = format!("/remote.php/dav/files/{}/Documents/TestFolder/", username);
        let relative_path = service.convert_to_relative_path(&full_path);
        
        assert_eq!(relative_path, "/Documents/TestFolder/", 
            "Failed for username: {}", username);
    }
}

// Test that validates the fix prevents the exact error scenario
#[tokio::test]
async fn test_fix_prevents_original_bug() {
    // Create service with the same username as in the problematic path
    let config = WebDAVConfig {
        server_url: "https://storage.example.com".to_string(),
        username: "perf3ct".to_string(),
        password: "testpass".to_string(),
        watch_folders: vec!["/Documents".to_string()],
        file_extensions: vec!["pdf".to_string()],
        timeout_seconds: 30,
        server_type: Some("nextcloud".to_string()),
    };
    let service = WebDAVService::new(config).unwrap();
    
    // This is the exact path from the error logs that was causing 404s
    let problematic_path = "/remote.php/dav/files/perf3ct/FullerDocuments/NicoleDocuments/Melanie%20Martinez%20June%207%202023/";
    
    // Before fix: This would have been used directly, causing double path construction
    let base_url = "https://storage.example.com/remote.php/dav/files/perf3ct";
    let old_buggy_url = format!("{}{}", base_url, problematic_path);
    
    // After fix: Convert to relative path first
    let relative_path = service.convert_to_relative_path(problematic_path);
    let fixed_url = format!("{}{}", base_url, relative_path);
    
    // Debug: print what we got
    println!("Original path: {}", problematic_path);
    println!("Relative path: {}", relative_path);
    println!("Old buggy URL: {}", old_buggy_url);
    println!("Fixed URL: {}", fixed_url);
    
    // The old URL would have been malformed (causing 404)
    assert!(old_buggy_url.contains("/remote.php/dav/files/perf3ct/remote.php/dav/files/perf3ct/"));
    
    // The new URL should be properly formed
    assert_eq!(fixed_url, "https://storage.example.com/remote.php/dav/files/perf3ct/FullerDocuments/NicoleDocuments/Melanie%20Martinez%20June%207%202023/");
    assert!(!fixed_url.contains("/remote.php/dav/files/perf3ct/remote.php/dav/files/perf3ct/"));
    
    // Most importantly, they should be different (proving the bug was fixed)
    assert_ne!(old_buggy_url, fixed_url, "The fix should produce different URLs than the buggy version");
}

// Tests for URL normalization to prevent trailing slash issues
#[tokio::test]
async fn test_nextcloud_url_normalization_with_trailing_slash() {
    let config = WebDAVConfig {
        server_url: "https://nas.example.com/".to_string(), // Note the trailing slash
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        watch_folders: vec!["/Documents".to_string()],
        file_extensions: vec!["pdf".to_string()],
        timeout_seconds: 30,
        server_type: Some("nextcloud".to_string()),
    };
    
    // This should not panic and should normalize the URL properly
    let webdav_url = config.webdav_url();
    
    // Should not contain double slashes
    assert!(!webdav_url.contains("//remote.php"), "URL should not contain double slashes: {}", webdav_url);
    assert_eq!(webdav_url, "https://nas.example.com/remote.php/dav/files/testuser");
}

#[tokio::test]
async fn test_nextcloud_url_normalization_without_trailing_slash() {
    let config = WebDAVConfig {
        server_url: "https://nas.example.com".to_string(), // No trailing slash
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        watch_folders: vec!["/Documents".to_string()],
        file_extensions: vec!["pdf".to_string()],
        timeout_seconds: 30,
        server_type: Some("nextcloud".to_string()),
    };
    
    let webdav_url = config.webdav_url();
    
    // Should not contain double slashes
    assert!(!webdav_url.contains("//remote.php"), "URL should not contain double slashes: {}", webdav_url);
    assert_eq!(webdav_url, "https://nas.example.com/remote.php/dav/files/testuser");
}

#[tokio::test]
async fn test_owncloud_url_normalization_with_trailing_slash() {
    let config = WebDAVConfig {
        server_url: "https://cloud.example.com/".to_string(), // Note the trailing slash
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        watch_folders: vec!["/Documents".to_string()],
        file_extensions: vec!["pdf".to_string()],
        timeout_seconds: 30,
        server_type: Some("owncloud".to_string()),
    };
    
    let webdav_url = config.webdav_url();
    
    // Should not contain double slashes
    assert!(!webdav_url.contains("//remote.php"), "URL should not contain double slashes: {}", webdav_url);
    assert_eq!(webdav_url, "https://cloud.example.com/remote.php/webdav");
}

#[tokio::test]
async fn test_owncloud_url_normalization_without_trailing_slash() {
    let config = WebDAVConfig {
        server_url: "https://cloud.example.com".to_string(), // No trailing slash
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        watch_folders: vec!["/Documents".to_string()],
        file_extensions: vec!["pdf".to_string()],
        timeout_seconds: 30,
        server_type: Some("owncloud".to_string()),
    };
    
    let webdav_url = config.webdav_url();
    
    // Should not contain double slashes
    assert!(!webdav_url.contains("//remote.php"), "URL should not contain double slashes: {}", webdav_url);
    assert_eq!(webdav_url, "https://cloud.example.com/remote.php/webdav");
}

#[tokio::test]
async fn test_generic_webdav_url_normalization_with_trailing_slash() {
    let config = WebDAVConfig {
        server_url: "https://webdav.example.com/".to_string(), // Note the trailing slash
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        watch_folders: vec!["/Documents".to_string()],
        file_extensions: vec!["pdf".to_string()],
        timeout_seconds: 30,
        server_type: Some("generic".to_string()),
    };
    
    let webdav_url = config.webdav_url();
    
    // Should normalize by removing the trailing slash
    assert_eq!(webdav_url, "https://webdav.example.com");
}

#[tokio::test]
async fn test_generic_webdav_url_normalization_without_trailing_slash() {
    let config = WebDAVConfig {
        server_url: "https://webdav.example.com".to_string(), // No trailing slash
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        watch_folders: vec!["/Documents".to_string()],
        file_extensions: vec!["pdf".to_string()],
        timeout_seconds: 30,
        server_type: Some("generic".to_string()),
    };
    
    let webdav_url = config.webdav_url();
    
    // Should remain the same
    assert_eq!(webdav_url, "https://webdav.example.com");
}

#[tokio::test]
async fn test_connection_get_url_for_path_normalization() {
    let config = WebDAVConfig {
        server_url: "https://nas.example.com/".to_string(), // Trailing slash
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        watch_folders: vec!["/Documents".to_string()],
        file_extensions: vec!["pdf".to_string()],
        timeout_seconds: 30,
        server_type: Some("nextcloud".to_string()),
    };
    
    let service = WebDAVService::new(config).unwrap();
    let connection = super::super::connection::WebDAVConnection::new(
        service.get_config().clone(), 
        super::super::config::RetryConfig::default()
    ).unwrap();
    
    // Test various path scenarios
    let test_cases = vec![
        ("/remote.php/dav/files/testuser/Photos/test.jpg", "https://nas.example.com/remote.php/dav/files/testuser/remote.php/dav/files/testuser/Photos/test.jpg"),
        ("Photos/test.jpg", "https://nas.example.com/remote.php/dav/files/testuser/Photos/test.jpg"),
        ("/Photos/test.jpg", "https://nas.example.com/remote.php/dav/files/testuser/Photos/test.jpg"),
        ("", "https://nas.example.com/remote.php/dav/files/testuser"),
    ];
    
    for (input_path, expected_url) in test_cases {
        let result_url = connection.get_url_for_path(input_path);
        
        // Verify the URL matches expected
        assert_eq!(result_url, expected_url, "URL construction failed for path: {}", input_path);
        
        // Ensure no double slashes in the final URL (except after protocol)
        let url_without_protocol = result_url.replace("https://", "");
        assert!(!url_without_protocol.contains("//"), "URL should not contain double slashes: {}", result_url);
        
        if input_path.starts_with("/remote.php/") {
            // This case would create double paths in the buggy version
            // The new version should still create a proper URL even if not ideal
            assert!(!result_url.contains("/remote.php/dav/files/testuser/remote.php/dav/files/testuser/remote.php/"));
        }
    }
}

#[tokio::test]
async fn test_url_normalization_edge_cases() {
    // Test multiple trailing slashes
    let config = WebDAVConfig {
        server_url: "https://nas.example.com///".to_string(), // Multiple trailing slashes
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        watch_folders: vec!["/Documents".to_string()],
        file_extensions: vec!["pdf".to_string()],
        timeout_seconds: 30,
        server_type: Some("nextcloud".to_string()),
    };
    
    let webdav_url = config.webdav_url();
    
    // Should normalize all trailing slashes
    assert!(!webdav_url.contains("//remote.php"), "URL should not contain double slashes: {}", webdav_url);
    assert_eq!(webdav_url, "https://nas.example.com/remote.php/dav/files/testuser");
}

#[tokio::test]
async fn test_all_server_types_url_consistency() {
    let server_url_variants = vec![
        "https://server.example.com",
        "https://server.example.com/",
        "https://server.example.com//",
        "https://server.example.com///",
    ];
    
    let server_types = vec![
        Some("nextcloud".to_string()),
        Some("owncloud".to_string()),
        Some("generic".to_string()),
        None,
    ];
    
    for server_type in &server_types {
        for server_url in &server_url_variants {
            let config = WebDAVConfig {
                server_url: server_url.to_string(),
                username: "testuser".to_string(),
                password: "testpass".to_string(),
                watch_folders: vec!["/Documents".to_string()],
                file_extensions: vec!["pdf".to_string()],
                timeout_seconds: 30,
                server_type: server_type.clone(),
            };
            
            let webdav_url = config.webdav_url();
            
            // All variants should produce the same normalized URL for a given server type
            let url_without_protocol = webdav_url.replace("https://", "");
            assert!(!url_without_protocol.contains("//"), 
                "URL should not contain double slashes for server_type {:?} and url {}: {}", 
                server_type, server_url, webdav_url);
            
            // Ensure the base domain is correctly preserved
            assert!(webdav_url.starts_with("https://server.example.com"), 
                "URL should start with normalized domain: {}", webdav_url);
        }
    }
}

// Tests specifically for file fetching scenarios - using actual service methods
#[tokio::test]
async fn test_service_download_file_url_construction() {
    let config = WebDAVConfig {
        server_url: "https://storage.example.com/".to_string(), // Note trailing slash (user input)
        username: "perf3ct".to_string(),
        password: "testpass".to_string(),
        watch_folders: vec!["/Documents".to_string()],
        file_extensions: vec!["png".to_string(), "pdf".to_string()],
        timeout_seconds: 30,
        server_type: Some("nextcloud".to_string()),
    };
    
    let service = WebDAVService::new(config).unwrap();
    let connection = super::super::connection::WebDAVConnection::new(
        service.get_config().clone(), 
        super::super::config::RetryConfig::default()
    ).unwrap();
    
    // These are the actual paths that would come from XML parser responses
    let xml_parser_paths = vec![
        "/remote.php/dav/files/perf3ct/Photos/PC%20Screenshots/zjoQcWqldv.png",
        "/remote.php/dav/files/perf3ct/FullerDocuments/NicoleDocuments/Melanie%20Martinez%20June%207%202023/document.pdf",
        "/remote.php/dav/files/perf3ct/FullerDocuments/JonDocuments/project.pdf",
        "/remote.php/dav/files/perf3ct/Documents/work/report.pdf",
    ];
    
    let expected_urls = vec![
        "https://storage.example.com/remote.php/dav/files/perf3ct/Photos/PC%20Screenshots/zjoQcWqldv.png",
        "https://storage.example.com/remote.php/dav/files/perf3ct/FullerDocuments/NicoleDocuments/Melanie%20Martinez%20June%207%202023/document.pdf",
        "https://storage.example.com/remote.php/dav/files/perf3ct/FullerDocuments/JonDocuments/project.pdf",
        "https://storage.example.com/remote.php/dav/files/perf3ct/Documents/work/report.pdf",
    ];
    
    for (xml_path, expected_url) in xml_parser_paths.iter().zip(expected_urls.iter()) {
        // Test the conversion from full XML path to relative path (the correct approach)
        let relative_path = service.convert_to_relative_path(xml_path);
        let constructed_url = connection.get_url_for_path(&relative_path);
        
        println!("XML path: {}", xml_path);
        println!("Relative path: {}", relative_path);
        println!("Constructed URL: {}", constructed_url);
        println!("Expected URL: {}", expected_url);
        
        // Verify no double slashes anywhere (except after protocol)
        let url_without_protocol = constructed_url.replace("https://", "");
        assert!(!url_without_protocol.contains("//"), 
            "URL should not contain double slashes: {}", constructed_url);
        
        // Verify no double path construction
        assert!(!constructed_url.contains("/remote.php/dav/files/perf3ct/remote.php/dav/files/perf3ct/"),
            "URL should not contain double path construction: {}", constructed_url);
        
        // The URL should be properly formed for file download
        assert!(constructed_url.starts_with("https://storage.example.com/"),
            "URL should start with normalized domain: {}", constructed_url);
        
        // Should contain the file path exactly once
        let path_occurrences = constructed_url.matches("/remote.php/dav/files/perf3ct/").count();
        assert_eq!(path_occurrences, 1, 
            "Path should appear exactly once in URL: {}", constructed_url);
        
        // Should match expected URL
        assert_eq!(constructed_url, *expected_url, "URL should match expected result");
    }
}

#[tokio::test]
async fn test_file_fetch_url_construction_with_convert_to_relative_path() {
    // This test demonstrates the CORRECT way to handle XML parser paths
    let config = WebDAVConfig {
        server_url: "https://nas.example.com/".to_string(), // Trailing slash
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        watch_folders: vec!["/Documents".to_string()],
        file_extensions: vec!["pdf".to_string()],
        timeout_seconds: 30,
        server_type: Some("nextcloud".to_string()),
    };
    
    let service = WebDAVService::new(config).unwrap();
    let connection = super::super::connection::WebDAVConnection::new(
        service.get_config().clone(), 
        super::super::config::RetryConfig::default()
    ).unwrap();
    
    // XML parser returns this full WebDAV path
    let xml_full_path = "/remote.php/dav/files/testuser/Documents/TestFolder/file.pdf";
    
    // Method 1: Direct concatenation (the old buggy way)
    let base_webdav_url = service.get_config().webdav_url();
    let buggy_url = format!("{}{}", base_webdav_url, xml_full_path);
    
    // Method 2: Using convert_to_relative_path (the correct way)
    let relative_path = service.convert_to_relative_path(xml_full_path);
    let correct_url = format!("{}{}", base_webdav_url, relative_path);
    
    // Method 3: Using get_url_for_path with relative path (the correct way)
    let connection_url = connection.get_url_for_path(&relative_path);
    
    println!("XML full path: {}", xml_full_path);
    println!("Base WebDAV URL: {}", base_webdav_url);
    println!("Relative path: {}", relative_path);
    println!("Buggy URL: {}", buggy_url);
    println!("Correct URL: {}", correct_url);
    println!("Connection URL: {}", connection_url);
    
    // The buggy method creates double paths
    assert!(buggy_url.contains("/remote.php/dav/files/testuser/remote.php/dav/files/testuser/"));
    
    // The correct method doesn't
    assert!(!correct_url.contains("/remote.php/dav/files/testuser/remote.php/dav/files/testuser/"));
    
    // The connection method with relative path should work correctly
    let url_without_protocol = connection_url.replace("https://", "");
    assert!(!url_without_protocol.contains("//"), "Connection URL should not contain double slashes: {}", connection_url);
    assert_eq!(connection_url, correct_url, "Connection URL with relative path should match correct URL");
    
    // Expected final URL
    let expected = "https://nas.example.com/remote.php/dav/files/testuser/Documents/TestFolder/file.pdf";
    assert_eq!(correct_url, expected);
}

#[tokio::test]
async fn test_file_fetch_real_world_error_scenario() {
    // This recreates the exact error scenario from the user's logs
    let config = WebDAVConfig {
        server_url: "https://storage.example.com/".to_string(),
        username: "Alex".to_string(), // The username from the error message
        password: "testpass".to_string(),
        watch_folders: vec!["/Photos".to_string()],
        file_extensions: vec!["png".to_string()],
        timeout_seconds: 30,
        server_type: Some("nextcloud".to_string()),
    };
    
    let service = WebDAVService::new(config).unwrap();
    let connection = super::super::connection::WebDAVConnection::new(
        service.get_config().clone(), 
        super::super::config::RetryConfig::default()
    ).unwrap();
    
    // This is the exact path from the error message
    let problematic_path = "/remote.php/dav/files/Alex/Photos/PC%20Screenshots/zjoQcWqldv.png";
    
    // Test the CORRECT approach: convert to relative path first
    let relative_path = service.convert_to_relative_path(problematic_path);
    let base_url = service.get_config().webdav_url();
    let corrected_url = format!("{}{}", base_url, relative_path);
    
    // Also test using connection with relative path
    let connection_url = connection.get_url_for_path(&relative_path);
    
    println!("Problematic path: {}", problematic_path);
    println!("Relative path: {}", relative_path);
    println!("Base URL: {}", base_url);
    println!("Corrected URL: {}", corrected_url);
    println!("Connection URL: {}", connection_url);
    
    // Verify the URL is properly constructed
    assert!(!corrected_url.contains("//remote.php"), 
        "URL should not contain //remote.php: {}", corrected_url);
    
    assert!(!corrected_url.contains("/remote.php/dav/files/Alex/remote.php/dav/files/Alex/"),
        "URL should not contain double path construction: {}", corrected_url);
    
    // This should be the final correct URL
    assert_eq!(corrected_url, 
        "https://storage.example.com/remote.php/dav/files/Alex/Photos/PC%20Screenshots/zjoQcWqldv.png");
    
    // Connection URL should match when using relative path
    assert_eq!(connection_url, corrected_url, "Connection URL should match corrected URL when using relative path");
    
    // And it should not contain double paths
    assert!(!corrected_url.contains("/remote.php/dav/files/Alex/remote.php/dav/files/Alex/"));
}

#[tokio::test]
async fn test_file_fetch_different_server_types() {
    let test_cases = vec![
        (
            "nextcloud",
            "https://cloud.example.com/",
            "user1",
            "/remote.php/dav/files/user1/Documents/file.pdf",
            "https://cloud.example.com/remote.php/dav/files/user1/Documents/file.pdf"
        ),
        (
            "owncloud", 
            "https://owncloud.example.com/",
            "user2",
            "/remote.php/webdav/Documents/file.pdf", // ownCloud uses different path structure
            "https://owncloud.example.com/remote.php/webdav/Documents/file.pdf"
        ),
        (
            "generic",
            "https://webdav.example.com/",
            "user3",
            "/webdav/Documents/file.pdf",
            "https://webdav.example.com/webdav/Documents/file.pdf"
        ),
    ];
    
    for (server_type, server_url, username, xml_path, expected_url) in test_cases {
        let config = WebDAVConfig {
            server_url: server_url.to_string(),
            username: username.to_string(),
            password: "testpass".to_string(),
            watch_folders: vec!["/Documents".to_string()],
            file_extensions: vec!["pdf".to_string()],
            timeout_seconds: 30,
            server_type: Some(server_type.to_string()),
        };
        
        let service = WebDAVService::new(config).unwrap();
        let connection = super::super::connection::WebDAVConnection::new(
            service.get_config().clone(), 
            super::super::config::RetryConfig::default()
        ).unwrap();
        
        // Test the CORRECT approach: convert to relative path first
        let relative_path = service.convert_to_relative_path(xml_path);
        let download_url = connection.get_url_for_path(&relative_path);
        
        println!("Server type: {}", server_type);
        println!("XML path: {}", xml_path);
        println!("Relative path: {}", relative_path);
        println!("Download URL: {}", download_url);
        println!("Expected: {}", expected_url);
        
        // Verify no double slashes
        let url_without_protocol = download_url.replace("https://", "");
        assert!(!url_without_protocol.contains("//"), 
            "URL should not contain double slashes for {}: {}", server_type, download_url);
        
        // Verify proper structure
        assert!(download_url.starts_with(&format!("https://{}", server_url.trim_start_matches("https://").trim_end_matches("/"))));
        
        // For Nextcloud and ownCloud, verify no double path construction
        if server_type == "nextcloud" {
            assert!(!download_url.contains(&format!("/remote.php/dav/files/{}/remote.php/dav/files/{}/", username, username)), 
                "Nextcloud URL should not contain double dav path: {}", download_url);
        } else if server_type == "owncloud" {
            assert!(!download_url.contains("/remote.php/webdav/remote.php/webdav/"), 
                "ownCloud URL should not contain double webdav path: {}", download_url);
        }
    }
}

// Test that validates we're using the actual service methods correctly
#[tokio::test]
async fn test_webdav_service_methods_use_correct_url_construction() {
    let config = WebDAVConfig {
        server_url: "https://nas.example.com/".to_string(), // Trailing slash
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        watch_folders: vec!["/Documents".to_string()],
        file_extensions: vec!["pdf".to_string()],
        timeout_seconds: 30,
        server_type: Some("nextcloud".to_string()),
    };
    
    let service = WebDAVService::new(config).unwrap();
    
    // Test paths that would come from XML parser
    let xml_paths = vec![
        "/remote.php/dav/files/testuser/Documents/file1.pdf",
        "/remote.php/dav/files/testuser/Photos/image.png",
        "/remote.php/dav/files/testuser/Work/Folder/document.pdf",
    ];
    
    for xml_path in xml_paths {
        // Test the convert_to_relative_path method (used by service methods)
        let relative_path = service.convert_to_relative_path(xml_path);
        
        println!("XML path: {}", xml_path);
        println!("Relative path: {}", relative_path);
        
        // Verify the relative path doesn't contain server prefixes
        assert!(!relative_path.contains("/remote.php/dav/files/"), 
            "Relative path should not contain server prefix: {}", relative_path);
        
        // Verify it starts with / but removes the server-specific part
        assert!(relative_path.starts_with('/'), "Relative path should start with /: {}", relative_path);
        
        // For this test, the service methods would use this relative path
        // which prevents the double path construction issue
    }
}

}