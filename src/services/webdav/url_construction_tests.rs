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
        server_url: "https://nas.jonathonfuller.com".to_string(),
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
    let base_url = "https://nas.jonathonfuller.com/remote.php/dav/files/perf3ct";
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
    assert_eq!(fixed_url, "https://nas.jonathonfuller.com/remote.php/dav/files/perf3ct/FullerDocuments/NicoleDocuments/Melanie%20Martinez%20June%207%202023/");
    assert!(!fixed_url.contains("/remote.php/dav/files/perf3ct/remote.php/dav/files/perf3ct/"));
    
    // Most importantly, they should be different (proving the bug was fixed)
    assert_ne!(old_buggy_url, fixed_url, "The fix should produce different URLs than the buggy version");
}

}