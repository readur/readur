#[cfg(test)]
mod path_processing_tests {
    use crate::models::FileIngestionInfo;
    use crate::services::webdav::{WebDAVConfig, WebDAVService};
    use crate::webdav_xml_parser::parse_propfind_response_with_directories;
    use wiremock::{
        matchers::{method, path, header},
        Mock, MockServer, ResponseTemplate,
    };

    /// Creates a test WebDAV service with mock server
    fn create_test_service(mock_server_url: &str) -> WebDAVService {
        let config = WebDAVConfig {
            server_url: mock_server_url.to_string(),
            username: "testuser".to_string(),
            password: "testpass".to_string(),
            watch_folders: vec!["/TestDocuments".to_string()],
            file_extensions: vec!["pdf".to_string(), "txt".to_string()],
            timeout_seconds: 30,
            server_type: Some("nextcloud".to_string()),
        };
        WebDAVService::new(config).expect("Failed to create test service")
    }

    /// Mock WebDAV PROPFIND response with directories and files
    fn mock_propfind_response() -> String {
        r#"<?xml version="1.0"?>
<d:multistatus xmlns:d="DAV:" xmlns:s="http://sabredav.org/ns" xmlns:oc="http://owncloud.org/ns" xmlns:nc="http://nextcloud.org/ns">
    <d:response>
        <d:href>/remote.php/dav/files/testuser/TestDocuments/</d:href>
        <d:propstat>
            <d:prop>
                <d:displayname>TestDocuments</d:displayname>
                <d:getlastmodified>Tue, 29 Jul 2025 01:34:17 GMT</d:getlastmodified>
                <d:getetag>"parent123etag"</d:getetag>
                <d:resourcetype><d:collection/></d:resourcetype>
            </d:prop>
            <d:status>HTTP/1.1 200 OK</d:status>
        </d:propstat>
    </d:response>
    <d:response>
        <d:href>/remote.php/dav/files/testuser/TestDocuments/SubDir1/</d:href>
        <d:propstat>
            <d:prop>
                <d:displayname>SubDir1</d:displayname>
                <d:getlastmodified>Fri, 20 Jun 2025 23:35:17 GMT</d:getlastmodified>
                <d:getetag>"subdir1etag"</d:getetag>
                <d:resourcetype><d:collection/></d:resourcetype>
            </d:prop>
            <d:status>HTTP/1.1 200 OK</d:status>
        </d:propstat>
    </d:response>
    <d:response>
        <d:href>/remote.php/dav/files/testuser/TestDocuments/SubDir2/</d:href>
        <d:propstat>
            <d:prop>
                <d:displayname>SubDir2</d:displayname>
                <d:getlastmodified>Tue, 29 Jul 2025 01:34:17 GMT</d:getlastmodified>
                <d:getetag>"subdir2etag"</d:getetag>
                <d:resourcetype><d:collection/></d:resourcetype>
            </d:prop>
            <d:status>HTTP/1.1 200 OK</d:status>
        </d:propstat>
    </d:response>
    <d:response>
        <d:href>/remote.php/dav/files/testuser/TestDocuments/test.pdf</d:href>
        <d:propstat>
            <d:prop>
                <d:displayname>test.pdf</d:displayname>
                <d:getlastmodified>Thu, 24 Jul 2025 19:16:19 GMT</d:getlastmodified>
                <d:getetag>"fileetag123"</d:getetag>
                <d:getcontentlength>1234567</d:getcontentlength>
                <d:resourcetype/>
            </d:prop>
            <d:status>HTTP/1.1 200 OK</d:status>
        </d:propstat>
    </d:response>
</d:multistatus>"#.to_string()
    }

    /// Mock WebDAV response for empty directory
    fn mock_empty_directory_response() -> String {
        r#"<?xml version="1.0"?>
<d:multistatus xmlns:d="DAV:" xmlns:s="http://sabredav.org/ns" xmlns:oc="http://owncloud.org/ns" xmlns:nc="http://nextcloud.org/ns">
    <d:response>
        <d:href>/remote.php/dav/files/testuser/TestDocuments/SubDir1/</d:href>
        <d:propstat>
            <d:prop>
                <d:displayname>SubDir1</d:displayname>
                <d:getlastmodified>Fri, 20 Jun 2025 23:35:17 GMT</d:getlastmodified>
                <d:getetag>"subdir1etag"</d:getetag>
                <d:resourcetype><d:collection/></d:resourcetype>
            </d:prop>
            <d:status>HTTP/1.1 200 OK</d:status>
        </d:propstat>
    </d:response>
</d:multistatus>"#.to_string()
    }

    #[test]
    fn test_xml_parser_returns_temp_paths() {
        // This test ensures the XML parser behavior is documented
        let xml_response = mock_propfind_response();
        let parsed_items = parse_propfind_response_with_directories(&xml_response)
            .expect("Failed to parse XML response");
        
        // All parsed items should have relative_path as "TEMP" initially
        for item in &parsed_items {
            assert_eq!(item.relative_path, "TEMP", 
                "XML parser should set relative_path to TEMP for processing by discovery layer");
        }
        
        // Should find the correct number of items
        assert_eq!(parsed_items.len(), 4, "Should parse all 4 items from XML");
        
        // Verify we get both directories and files
        let directories: Vec<_> = parsed_items.iter().filter(|i| i.is_directory).collect();
        let files: Vec<_> = parsed_items.iter().filter(|i| !i.is_directory).collect();
        
        assert_eq!(directories.len(), 3, "Should find 3 directories");
        assert_eq!(files.len(), 1, "Should find 1 file");
    }

    #[test]
    fn test_path_processing_converts_temp_to_relative_paths() {
        let service = create_test_service("http://test.example.com");
        
        // Create mock parsed items with TEMP paths (simulating XML parser output)
        let mock_items = vec![
            FileIngestionInfo {
                relative_path: "TEMP".to_string(),
                full_path: "/remote.php/dav/files/testuser/TestDocuments/".to_string(),
                #[allow(deprecated)]
                path: "/remote.php/dav/files/testuser/TestDocuments/".to_string(),
                name: "TestDocuments".to_string(),
                size: 0,
                mime_type: "application/octet-stream".to_string(),
                last_modified: None,
                etag: "parent123etag".to_string(),
                is_directory: true,
                created_at: None,
                permissions: None,
                owner: None,
                group: None,
                metadata: None,
            },
            FileIngestionInfo {
                relative_path: "TEMP".to_string(),
                full_path: "/remote.php/dav/files/testuser/TestDocuments/SubDir1/".to_string(),
                #[allow(deprecated)]
                path: "/remote.php/dav/files/testuser/TestDocuments/SubDir1/".to_string(),
                name: "SubDir1".to_string(),
                size: 0,
                mime_type: "application/octet-stream".to_string(),
                last_modified: None,
                etag: "subdir1etag".to_string(),
                is_directory: true,
                created_at: None,
                permissions: None,
                owner: None,
                group: None,
                metadata: None,
            },
        ];
        
        // Process the items
        let processed_items = service.process_file_infos(mock_items);
        
        // Verify paths are correctly converted
        assert_eq!(processed_items[0].relative_path, "/TestDocuments/");
        assert_eq!(processed_items[1].relative_path, "/TestDocuments/SubDir1/");
        
        // Verify full_path remains unchanged
        assert_eq!(processed_items[0].full_path, "/remote.php/dav/files/testuser/TestDocuments/");
        assert_eq!(processed_items[1].full_path, "/remote.php/dav/files/testuser/TestDocuments/SubDir1/");
    }

    #[test]
    fn test_directory_filtering_excludes_parent() {
        // Create processed items including parent directory
        let processed_items = vec![
            FileIngestionInfo {
                relative_path: "/TestDocuments/".to_string(),
                full_path: "/remote.php/dav/files/testuser/TestDocuments/".to_string(),
                #[allow(deprecated)]
                path: "/TestDocuments/".to_string(),
                name: "TestDocuments".to_string(),
                size: 0,
                mime_type: "application/octet-stream".to_string(),
                last_modified: None,
                etag: "parent123etag".to_string(),
                is_directory: true,
                created_at: None,
                permissions: None,
                owner: None,
                group: None,
                metadata: None,
            },
            FileIngestionInfo {
                relative_path: "/TestDocuments/SubDir1/".to_string(),
                full_path: "/remote.php/dav/files/testuser/TestDocuments/SubDir1/".to_string(),
                #[allow(deprecated)]
                path: "/TestDocuments/SubDir1/".to_string(),
                name: "SubDir1".to_string(),
                size: 0,
                mime_type: "application/octet-stream".to_string(),
                last_modified: None,
                etag: "subdir1etag".to_string(),
                is_directory: true,
                created_at: None,
                permissions: None,
                owner: None,
                group: None,
                metadata: None,
            },
        ];
        
        // Simulate the filtering logic from discover_files_and_directories_single_with_url
        let directory_path = "/TestDocuments";
        let mut files = Vec::new();
        let mut directories = Vec::new();
        
        for item in processed_items {
            // Skip the directory itself (handle both with and without trailing slash)
            let normalized_item_path = item.relative_path.trim_end_matches('/');
            let normalized_directory_path = directory_path.trim_end_matches('/');
            
            if normalized_item_path == normalized_directory_path {
                continue; // Skip the directory itself
            }
            
            if item.is_directory {
                directories.push(item);
            } else {
                files.push(item);
            }
        }
        
        // Should exclude parent directory but include subdirectory
        assert_eq!(files.len(), 0);
        assert_eq!(directories.len(), 1);
        assert_eq!(directories[0].relative_path, "/TestDocuments/SubDir1/");
    }

    #[tokio::test]
    async fn test_single_directory_discovery_integration() {
        let mock_server = MockServer::start().await;
        
        // Mock the PROPFIND request
        Mock::given(method("PROPFIND"))
            .and(path("/remote.php/dav/files/testuser/TestDocuments"))
            .and(header("depth", "1"))
            .and(header("content-type", "application/xml"))
            .respond_with(
                ResponseTemplate::new(207)
                    .set_body_string(mock_propfind_response())
                    .insert_header("content-type", "application/xml")
            )
            .mount(&mock_server)
            .await;
        
        let service = create_test_service(&mock_server.uri());
        
        // Test single directory discovery
        let result = service.discover_files_and_directories("/TestDocuments", false).await
            .expect("Single directory discovery should succeed");
        
        // Verify results
        assert_eq!(result.files.len(), 1, "Should find 1 file");
        assert_eq!(result.directories.len(), 2, "Should find 2 directories (excluding parent)");
        
        // Verify directory paths are correct (not TEMP)
        let dir_paths: Vec<&String> = result.directories.iter().map(|d| &d.relative_path).collect();
        assert!(dir_paths.contains(&&"/TestDocuments/SubDir1/".to_string()));
        assert!(dir_paths.contains(&&"/TestDocuments/SubDir2/".to_string()));
        
        // Verify no directory has TEMP path
        for dir in &result.directories {
            assert_ne!(dir.relative_path, "TEMP", "Directory path should not be TEMP");
        }
        
        // Verify file path is correct
        assert_eq!(result.files[0].relative_path, "/TestDocuments/test.pdf");
        assert_ne!(result.files[0].relative_path, "TEMP", "File path should not be TEMP");
    }

    #[tokio::test]
    async fn test_recursive_directory_discovery_integration() {
        let mock_server = MockServer::start().await;
        
        // Mock the initial PROPFIND request for root directory
        Mock::given(method("PROPFIND"))
            .and(path("/remote.php/dav/files/testuser/TestDocuments"))
            .and(header("depth", "1"))
            .and(header("content-type", "application/xml"))
            .respond_with(
                ResponseTemplate::new(207)
                    .set_body_string(mock_propfind_response())
                    .insert_header("content-type", "application/xml")
            )
            .mount(&mock_server)
            .await;
        
        // Mock PROPFIND requests for subdirectories (return empty for simplicity)
        Mock::given(method("PROPFIND"))
            .and(path("/remote.php/dav/files/testuser/TestDocuments/SubDir1"))
            .and(header("depth", "1"))
            .and(header("content-type", "application/xml"))
            .respond_with(
                ResponseTemplate::new(207)
                    .set_body_string(mock_empty_directory_response())
                    .insert_header("content-type", "application/xml")
            )
            .mount(&mock_server)
            .await;
        
        Mock::given(method("PROPFIND"))
            .and(path("/remote.php/dav/files/testuser/TestDocuments/SubDir2"))
            .and(header("depth", "1"))
            .and(header("content-type", "application/xml"))
            .respond_with(
                ResponseTemplate::new(207)
                    .set_body_string(mock_empty_directory_response())
                    .insert_header("content-type", "application/xml")
            )
            .mount(&mock_server)
            .await;
        
        let service = create_test_service(&mock_server.uri());
        
        // Test recursive directory discovery
        let result = service.discover_files_and_directories("/TestDocuments", true).await
            .expect("Recursive directory discovery should succeed");
        
        // Verify results
        assert_eq!(result.files.len(), 1, "Should find 1 file");
        assert_eq!(result.directories.len(), 2, "Should find 2 directories (excluding parents)");
        
        // Verify no paths are TEMP
        for item in result.files.iter().chain(result.directories.iter()) {
            assert_ne!(item.relative_path, "TEMP", "Paths should be processed, not TEMP");
            assert!(item.relative_path.starts_with("/TestDocuments"), 
                "All paths should start with /TestDocuments, got: {}", item.relative_path);
        }
    }

    #[test]
    fn test_href_to_relative_path_conversion() {
        let service = create_test_service("http://test.example.com");
        
        // Test Nextcloud path conversion
        assert_eq!(
            service.href_to_relative_path("/remote.php/dav/files/testuser/Documents/file.pdf"),
            "/Documents/file.pdf"
        );
        
        assert_eq!(
            service.href_to_relative_path("/remote.php/dav/files/testuser/"),
            "/"
        );
        
        assert_eq!(
            service.href_to_relative_path("/remote.php/dav/files/testuser/Deep/Nested/Path/"),
            "/Deep/Nested/Path/"
        );
    }

    #[test]
    fn test_url_construction() {
        let service = create_test_service("http://test.example.com");
        
        // Test URL construction for different paths
        assert_eq!(
            service.get_url_for_path("/TestDocuments"),
            "http://test.example.com/remote.php/dav/files/testuser/TestDocuments"
        );
        
        assert_eq!(
            service.get_url_for_path("/TestDocuments/SubDir"),
            "http://test.example.com/remote.php/dav/files/testuser/TestDocuments/SubDir"
        );
        
        assert_eq!(
            service.get_url_for_path("/"),
            "http://test.example.com/remote.php/dav/files/testuser"
        );
    }

    #[test]
    fn test_regression_temp_paths_are_processed() {
        // Regression test: Ensure TEMP paths from XML parser are always processed
        let service = create_test_service("http://test.example.com");
        
        // Simulate the exact scenario that caused the bug
        let raw_xml_items = vec![
            FileIngestionInfo {
                relative_path: "TEMP".to_string(), // This is what XML parser returns
                full_path: "/remote.php/dav/files/testuser/TestDocuments/ImportantFolder/".to_string(),
                #[allow(deprecated)]
                path: "/remote.php/dav/files/testuser/TestDocuments/ImportantFolder/".to_string(),
                name: "ImportantFolder".to_string(),
                size: 0,
                mime_type: "application/octet-stream".to_string(),
                last_modified: None,
                etag: "folder123etag".to_string(),
                is_directory: true,
                created_at: None,
                permissions: None,
                owner: None,
                group: None,
                metadata: None,
            }
        ];
        
        // Process items as the service should do
        let processed_items = service.process_file_infos(raw_xml_items);
        
        // Verify the bug is fixed
        assert_eq!(processed_items.len(), 1);
        assert_ne!(processed_items[0].relative_path, "TEMP", 
            "REGRESSION: relative_path should not remain as TEMP after processing");
        assert_eq!(processed_items[0].relative_path, "/TestDocuments/ImportantFolder/",
            "relative_path should be properly converted from href");
    }

    #[tokio::test]
    async fn test_discover_files_and_directories_processes_paths() {
        // Integration test to ensure discover_files_and_directories always processes paths
        let mock_server = MockServer::start().await;
        
        Mock::given(method("PROPFIND"))
            .and(path("/remote.php/dav/files/testuser/TestDocuments"))
            .respond_with(
                ResponseTemplate::new(207)
                    .set_body_string(mock_propfind_response())
                    .insert_header("content-type", "application/xml")
            )
            .mount(&mock_server)
            .await;
        
        let service = create_test_service(&mock_server.uri());
        
        let result = service.discover_files_and_directories("/TestDocuments", false).await
            .expect("Discovery should succeed");
        
        // Ensure no items have TEMP paths (regression test)
        for item in result.files.iter().chain(result.directories.iter()) {
            assert_ne!(item.relative_path, "TEMP", 
                "REGRESSION: No items should have TEMP paths after discovery");
        }
    }
}