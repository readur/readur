#[cfg(test)]
mod tests {
    use crate::file_service::FileService;
    use std::fs;
    use tempfile::TempDir;
    use uuid::Uuid;

    fn create_test_file_service() -> (FileService, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let upload_path = temp_dir.path().to_string_lossy().to_string();
        let service = FileService::new(upload_path);
        (service, temp_dir)
    }

    #[tokio::test]
    async fn test_save_file() {
        let (service, _temp_dir) = create_test_file_service();
        let filename = "test.txt";
        let data = b"Hello, World!";
        
        let result = service.save_file(filename, data).await;
        assert!(result.is_ok());
        
        let file_path = result.unwrap();
        assert!(fs::metadata(&file_path).is_ok());
        
        let saved_content = fs::read(&file_path).unwrap();
        assert_eq!(saved_content, data);
    }

    #[tokio::test]
    async fn test_save_file_with_extension() {
        let (service, _temp_dir) = create_test_file_service();
        let filename = "document.pdf";
        let data = b"PDF content";
        
        let result = service.save_file(filename, data).await;
        assert!(result.is_ok());
        
        let file_path = result.unwrap();
        assert!(file_path.ends_with(".pdf"));
    }

    #[tokio::test]
    async fn test_save_file_without_extension() {
        let (service, _temp_dir) = create_test_file_service();
        let filename = "document";
        let data = b"Some content";
        
        let result = service.save_file(filename, data).await;
        assert!(result.is_ok());
        
        let file_path = result.unwrap();
        // Should not have an extension (check just the filename part)
        let filename_part = std::path::Path::new(&file_path)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap();
        assert!(!filename_part.contains('.'));
    }

    #[test]
    fn test_create_document() {
        let (service, _temp_dir) = create_test_file_service();
        let user_id = Uuid::new_v4();
        
        let document = service.create_document(
            "saved_file.pdf",
            "original_file.pdf",
            "/path/to/saved_file.pdf",
            1024,
            "application/pdf",
            user_id,
            Some("abcd1234hash".to_string()),
        );
        
        assert_eq!(document.filename, "saved_file.pdf");
        assert_eq!(document.original_filename, "original_file.pdf");
        assert_eq!(document.file_path, "/path/to/saved_file.pdf");
        assert_eq!(document.file_size, 1024);
        assert_eq!(document.mime_type, "application/pdf");
        assert_eq!(document.user_id, user_id);
        assert_eq!(document.file_hash, Some("abcd1234hash".to_string()));
        assert!(document.content.is_none());
        assert!(document.ocr_text.is_none());
        assert!(document.tags.is_empty());
    }

    #[test]
    fn test_is_allowed_file_type() {
        let (service, _temp_dir) = create_test_file_service();
        let allowed_types = vec![
            "pdf".to_string(),
            "txt".to_string(),
            "png".to_string(),
            "jpg".to_string(),
        ];
        
        assert!(service.is_allowed_file_type("document.pdf", &allowed_types));
        assert!(service.is_allowed_file_type("text.txt", &allowed_types));
        assert!(service.is_allowed_file_type("image.PNG", &allowed_types)); // Case insensitive
        assert!(service.is_allowed_file_type("photo.JPG", &allowed_types)); // Case insensitive
        
        assert!(!service.is_allowed_file_type("document.doc", &allowed_types));
        assert!(!service.is_allowed_file_type("archive.zip", &allowed_types));
        assert!(!service.is_allowed_file_type("noextension", &allowed_types));
    }

    #[tokio::test]
    async fn test_read_file() {
        let (service, _temp_dir) = create_test_file_service();
        let filename = "test.txt";
        let original_data = b"Hello, World!";
        
        let file_path = service.save_file(filename, original_data).await.unwrap();
        
        let result = service.read_file(&file_path).await;
        assert!(result.is_ok());
        
        let read_data = result.unwrap();
        assert_eq!(read_data, original_data);
    }

    #[tokio::test]
    async fn test_read_nonexistent_file() {
        let (service, _temp_dir) = create_test_file_service();
        let nonexistent_path = "/path/to/nonexistent/file.txt";
        
        let result = service.read_file(nonexistent_path).await;
        assert!(result.is_err());
    }
}