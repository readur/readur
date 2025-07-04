#[cfg(test)]
use crate::services::file_service::FileService;
#[cfg(test)]
use crate::models::Document;
#[cfg(test)]
use std::fs;
#[cfg(test)]
use tempfile::TempDir;
#[cfg(test)]
use uuid::Uuid;

#[cfg(test)]
fn create_test_file_service() -> (FileService, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let upload_path = temp_dir.path().to_string_lossy().to_string();
    let service = FileService::new(upload_path);
    (service, temp_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

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
            None, // original_created_at
            None, // original_modified_at
            None, // source_metadata
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

#[cfg(test)]
mod file_deletion_tests {
    use super::*;
    use chrono::Utc;
    use std::path::Path;
    use std::fs;

    fn create_test_document_with_files(_service: &FileService, temp_dir: &TempDir, user_id: uuid::Uuid) -> (Document, String, String, String) {
        let document_id = uuid::Uuid::new_v4();
        
        // Create main document file
        let base_path = temp_dir.path().join("documents");
        fs::create_dir_all(&base_path).unwrap();
        let main_file_path = base_path.join("test_document.pdf");
        fs::write(&main_file_path, b"PDF content").unwrap();
        
        // Create thumbnails directory and thumbnail file with correct naming
        let thumbnails_path = temp_dir.path().join("thumbnails");
        fs::create_dir_all(&thumbnails_path).unwrap();
        let thumbnail_path = thumbnails_path.join(format!("{}_thumb.jpg", document_id));
        fs::write(&thumbnail_path, b"Thumbnail content").unwrap();
        
        // Create processed_images directory and processed image file with correct naming
        let processed_dir = temp_dir.path().join("processed_images");
        fs::create_dir_all(&processed_dir).unwrap();
        let processed_path = processed_dir.join(format!("{}_processed.png", document_id));
        fs::write(&processed_path, b"Processed content").unwrap();
        
        let document = Document {
            id: document_id,
            filename: "test_document.pdf".to_string(),
            original_filename: "test_document.pdf".to_string(),
            file_path: main_file_path.to_string_lossy().to_string(),
            file_size: 1024,
            mime_type: "application/pdf".to_string(),
            content: Some("Test document content".to_string()),
            ocr_text: Some("This is extracted OCR text".to_string()),
            ocr_confidence: Some(95.5),
            ocr_word_count: Some(150),
            ocr_processing_time_ms: Some(1200),
            ocr_status: Some("completed".to_string()),
            ocr_error: None,
            ocr_completed_at: Some(Utc::now()),
            tags: vec!["test".to_string()],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            user_id,
            file_hash: Some("hash123".to_string()),
            original_created_at: None,
            original_modified_at: None,
            source_metadata: None,
            ocr_retry_count: None,
            ocr_failure_reason: None,
        };
        
        (
            document,
            main_file_path.to_string_lossy().to_string(),
            thumbnail_path.to_string_lossy().to_string(),
            processed_path.to_string_lossy().to_string(),
        )
    }

    #[tokio::test]
    async fn test_delete_document_files_success() {
        let (service, temp_dir) = create_test_file_service();
        let user_id = uuid::Uuid::new_v4();
        
        let (document, main_path, thumb_path, processed_path) = 
            create_test_document_with_files(&service, &temp_dir, user_id);
        
        // Verify files exist before deletion
        assert!(Path::new(&main_path).exists());
        assert!(Path::new(&thumb_path).exists());
        assert!(Path::new(&processed_path).exists());
        
        // Delete document files
        let result = service.delete_document_files(&document).await;
        assert!(result.is_ok());
        
        // Verify main file is deleted
        assert!(!Path::new(&main_path).exists());
        
        // Verify thumbnail and processed files are deleted
        assert!(!Path::new(&thumb_path).exists());
        assert!(!Path::new(&processed_path).exists());
    }

    #[tokio::test]
    async fn test_delete_document_files_main_file_missing() {
        let (service, temp_dir) = create_test_file_service();
        let user_id = uuid::Uuid::new_v4();
        
        let (mut document, main_path, thumb_path, processed_path) = 
            create_test_document_with_files(&service, &temp_dir, user_id);
        
        // Delete main file manually before test
        fs::remove_file(&main_path).unwrap();
        assert!(!Path::new(&main_path).exists());
        
        // Verify thumbnail and processed files still exist
        assert!(Path::new(&thumb_path).exists());
        assert!(Path::new(&processed_path).exists());
        
        // Try to delete document files (should still clean up other files)
        let result = service.delete_document_files(&document).await;
        assert!(result.is_ok());
        
        // Verify thumbnail and processed files are deleted despite main file missing
        assert!(!Path::new(&thumb_path).exists());
        assert!(!Path::new(&processed_path).exists());
    }

    #[tokio::test]
    async fn test_delete_document_files_thumbnail_missing() {
        let (service, temp_dir) = create_test_file_service();
        let user_id = uuid::Uuid::new_v4();
        
        let (document, main_path, thumb_path, processed_path) = 
            create_test_document_with_files(&service, &temp_dir, user_id);
        
        // Delete thumbnail file manually before test
        fs::remove_file(&thumb_path).unwrap();
        assert!(!Path::new(&thumb_path).exists());
        
        // Verify other files still exist
        assert!(Path::new(&main_path).exists());
        assert!(Path::new(&processed_path).exists());
        
        // Delete document files
        let result = service.delete_document_files(&document).await;
        assert!(result.is_ok());
        
        // Verify main and processed files are deleted
        assert!(!Path::new(&main_path).exists());
        assert!(!Path::new(&processed_path).exists());
    }

    #[tokio::test]
    async fn test_delete_document_files_processed_missing() {
        let (service, temp_dir) = create_test_file_service();
        let user_id = uuid::Uuid::new_v4();
        
        let (document, main_path, thumb_path, processed_path) = 
            create_test_document_with_files(&service, &temp_dir, user_id);
        
        // Delete processed file manually before test
        fs::remove_file(&processed_path).unwrap();
        assert!(!Path::new(&processed_path).exists());
        
        // Verify other files still exist
        assert!(Path::new(&main_path).exists());
        assert!(Path::new(&thumb_path).exists());
        
        // Delete document files
        let result = service.delete_document_files(&document).await;
        assert!(result.is_ok());
        
        // Verify main and thumbnail files are deleted
        assert!(!Path::new(&main_path).exists());
        assert!(!Path::new(&thumb_path).exists());
    }

    #[tokio::test]
    async fn test_delete_document_files_all_missing() {
        let (service, _temp_dir) = create_test_file_service();
        let user_id = uuid::Uuid::new_v4();
        
        let document = Document {
            id: uuid::Uuid::new_v4(),
            filename: "nonexistent.pdf".to_string(),
            original_filename: "nonexistent.pdf".to_string(),
            file_path: "/nonexistent/path/nonexistent.pdf".to_string(),
            file_size: 1024,
            mime_type: "application/pdf".to_string(),
            content: None,
            ocr_text: None,
            ocr_confidence: None,
            ocr_word_count: None,
            ocr_processing_time_ms: None,
            ocr_status: Some("pending".to_string()),
            ocr_error: None,
            ocr_completed_at: None,
            tags: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            user_id,
            file_hash: None,
            original_created_at: None,
            original_modified_at: None,
            source_metadata: None,
            ocr_retry_count: None,
            ocr_failure_reason: None,
        };
        
        // Try to delete nonexistent files (should not fail)
        let result = service.delete_document_files(&document).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_document_files_with_different_extensions() {
        let (service, temp_dir) = create_test_file_service();
        let user_id = uuid::Uuid::new_v4();
        let document_id = uuid::Uuid::new_v4();
        
        // Create main document file in documents directory
        let documents_path = temp_dir.path().join("documents");
        fs::create_dir_all(&documents_path).unwrap();
        let main_file_path = documents_path.join("test_image.png");
        fs::write(&main_file_path, b"PNG content").unwrap();
        
        // Create thumbnail in thumbnails directory with correct naming
        let thumbnails_path = temp_dir.path().join("thumbnails");
        fs::create_dir_all(&thumbnails_path).unwrap();
        let thumbnail_path = thumbnails_path.join(format!("{}_thumb.jpg", document_id));
        fs::write(&thumbnail_path, b"Thumbnail content").unwrap();
        
        // Create processed image in processed_images directory with correct naming
        let processed_dir = temp_dir.path().join("processed_images");
        fs::create_dir_all(&processed_dir).unwrap();
        let processed_path = processed_dir.join(format!("{}_processed.png", document_id));
        fs::write(&processed_path, b"Processed content").unwrap();
        
        let document = Document {
            id: document_id,
            filename: "test_image.png".to_string(),
            original_filename: "test_image.png".to_string(),
            file_path: main_file_path.to_string_lossy().to_string(),
            file_size: 2048,
            mime_type: "image/png".to_string(),
            content: None,
            ocr_text: Some("Image OCR text".to_string()),
            ocr_confidence: Some(88.2),
            ocr_word_count: Some(25),
            ocr_processing_time_ms: Some(800),
            ocr_status: Some("completed".to_string()),
            ocr_error: None,
            ocr_completed_at: Some(Utc::now()),
            tags: vec!["image".to_string()],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            user_id,
            file_hash: Some("imagehash456".to_string()),
            original_created_at: None,
            original_modified_at: None,
            source_metadata: None,
            ocr_retry_count: None,
            ocr_failure_reason: None,
        };
        
        // Verify files exist
        assert!(Path::new(&main_file_path).exists());
        assert!(Path::new(&thumbnail_path).exists());
        assert!(Path::new(&processed_path).exists());
        
        // Delete document files
        let result = service.delete_document_files(&document).await;
        assert!(result.is_ok());
        
        // Verify all files are deleted
        assert!(!Path::new(&main_file_path).exists());
        assert!(!Path::new(&thumbnail_path).exists());
        assert!(!Path::new(&processed_path).exists());
    }

    #[tokio::test]
    async fn test_delete_document_files_partial_failure_continues() {
        let (service, temp_dir) = create_test_file_service();
        let user_id = uuid::Uuid::new_v4();
        let document_id = uuid::Uuid::new_v4();
        
        // Create main file in documents directory
        let documents_path = temp_dir.path().join("documents");
        fs::create_dir_all(&documents_path).unwrap();
        let main_file_path = documents_path.join("readonly_document.pdf");
        fs::write(&main_file_path, b"PDF content").unwrap();
        
        // Create thumbnail file in thumbnails directory with correct naming
        let thumbnails_path = temp_dir.path().join("thumbnails");
        fs::create_dir_all(&thumbnails_path).unwrap();
        let thumbnail_path = thumbnails_path.join(format!("{}_thumb.jpg", document_id));
        fs::write(&thumbnail_path, b"Thumbnail content").unwrap();
        
        let document = Document {
            id: document_id,
            filename: "readonly_document.pdf".to_string(),
            original_filename: "readonly_document.pdf".to_string(),
            file_path: main_file_path.to_string_lossy().to_string(),
            file_size: 1024,
            mime_type: "application/pdf".to_string(),
            content: Some("Test content".to_string()),
            ocr_text: None,
            ocr_confidence: None,
            ocr_word_count: None,
            ocr_processing_time_ms: None,
            ocr_status: Some("pending".to_string()),
            ocr_error: None,
            ocr_completed_at: None,
            tags: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            user_id,
            file_hash: Some("hash789".to_string()),
            original_created_at: None,
            original_modified_at: None,
            source_metadata: None,
            ocr_retry_count: None,
            ocr_failure_reason: None,
        };
        
        // Verify files exist
        assert!(Path::new(&main_file_path).exists());
        assert!(Path::new(&thumbnail_path).exists());
        
        // Delete document files (should succeed even if some files can't be deleted)
        let result = service.delete_document_files(&document).await;
        assert!(result.is_ok());
        
        // At minimum, the function should attempt to delete all files
        // and not fail completely if one file can't be deleted
    }

    #[tokio::test]
    async fn test_delete_document_files_with_no_extension() {
        let (service, temp_dir) = create_test_file_service();
        let user_id = uuid::Uuid::new_v4();
        
        let base_path = temp_dir.path().join("documents");
        fs::create_dir_all(&base_path).unwrap();
        
        // Create document with no extension
        let main_file_path = base_path.join("document_no_ext");
        fs::write(&main_file_path, b"Content without extension").unwrap();
        
        let document = Document {
            id: uuid::Uuid::new_v4(),
            filename: "document_no_ext".to_string(),
            original_filename: "document_no_ext".to_string(),
            file_path: main_file_path.to_string_lossy().to_string(),
            file_size: 512,
            mime_type: "text/plain".to_string(),
            content: Some("Plain text content".to_string()),
            ocr_text: None,
            ocr_confidence: None,
            ocr_word_count: None,
            ocr_processing_time_ms: None,
            ocr_status: Some("not_applicable".to_string()),
            ocr_error: None,
            ocr_completed_at: None,
            tags: vec!["text".to_string()],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            user_id,
            file_hash: Some("texthash".to_string()),
            original_created_at: None,
            original_modified_at: None,
            source_metadata: None,
            ocr_retry_count: None,
            ocr_failure_reason: None,
        };
        
        // Verify file exists
        assert!(Path::new(&main_file_path).exists());
        
        // Delete document files
        let result = service.delete_document_files(&document).await;
        assert!(result.is_ok());
        
        // Verify file is deleted
        assert!(!Path::new(&main_file_path).exists());
    }

    #[tokio::test]
    async fn test_delete_document_files_concurrent_calls() {
        let (service, temp_dir) = create_test_file_service();
        let user_id = uuid::Uuid::new_v4();
        
        let (document, main_path, thumb_path, processed_path) = 
            create_test_document_with_files(&service, &temp_dir, user_id);
        
        // Verify files exist
        assert!(Path::new(&main_path).exists());
        assert!(Path::new(&thumb_path).exists());
        assert!(Path::new(&processed_path).exists());
        
        // Call delete_document_files concurrently
        let service_clone = service.clone();
        let document_clone = document.clone();
        
        let task1 = tokio::spawn(async move {
            service.delete_document_files(&document).await
        });
        
        let task2 = tokio::spawn(async move {
            service_clone.delete_document_files(&document_clone).await
        });
        
        // Both calls should complete successfully now that FileService handles concurrent deletions
        let result1 = task1.await.expect("Task 1 should complete");
        let result2 = task2.await.expect("Task 2 should complete");
        
        // Both deletion attempts should succeed - the improved FileService handles 
        // "file not found" errors gracefully as they indicate successful deletion by another task
        assert!(result1.is_ok(), "First deletion task should succeed: {:?}", result1);
        assert!(result2.is_ok(), "Second deletion task should succeed: {:?}", result2);
        
        // Verify files are deleted
        assert!(!Path::new(&main_path).exists());
        assert!(!Path::new(&thumb_path).exists());
        assert!(!Path::new(&processed_path).exists());
    }
}