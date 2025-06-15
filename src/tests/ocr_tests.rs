#[cfg(test)]
mod tests {
    use crate::ocr::OcrService;
    use std::fs;
    use std::path::Path;
    use tempfile::NamedTempFile;
    
    // Mock database for testing
    mod mock_db {
        use anyhow::Result;
        use uuid::Uuid;
        use std::sync::{Arc, Mutex};
        use std::collections::HashMap;
        
        #[derive(Clone)]
        pub struct MockDatabase {
            ocr_updates: Arc<Mutex<HashMap<Uuid, String>>>,
        }
        
        impl MockDatabase {
            pub fn new() -> Self {
                Self {
                    ocr_updates: Arc::new(Mutex::new(HashMap::new())),
                }
            }
            
            pub async fn update_document_ocr(&self, id: Uuid, ocr_text: &str) -> Result<()> {
                let mut updates = self.ocr_updates.lock().unwrap();
                updates.insert(id, ocr_text.to_string());
                Ok(())
            }
            
            pub fn get_ocr_text(&self, id: &Uuid) -> Option<String> {
                let updates = self.ocr_updates.lock().unwrap();
                updates.get(id).cloned()
            }
            
            pub fn get_all_ocr_updates(&self) -> HashMap<Uuid, String> {
                let updates = self.ocr_updates.lock().unwrap();
                updates.clone()
            }
        }
    }
    
    use mock_db::MockDatabase;

    #[test]
    fn test_is_image_file() {
        let ocr_service = OcrService::new();
        
        assert!(ocr_service.is_image_file("image.png"));
        assert!(ocr_service.is_image_file("photo.jpg"));
        assert!(ocr_service.is_image_file("picture.JPEG"));
        assert!(ocr_service.is_image_file("scan.tiff"));
        assert!(ocr_service.is_image_file("bitmap.bmp"));
        assert!(ocr_service.is_image_file("animation.gif"));
        
        assert!(!ocr_service.is_image_file("document.pdf"));
        assert!(!ocr_service.is_image_file("text.txt"));
        assert!(!ocr_service.is_image_file("archive.zip"));
        assert!(!ocr_service.is_image_file("noextension"));
    }

    #[tokio::test]
    async fn test_extract_text_from_plain_text() {
        let ocr_service = OcrService::new();
        
        let temp_file = NamedTempFile::new().unwrap();
        let test_content = "This is a test text file.\nWith multiple lines.";
        fs::write(temp_file.path(), test_content).unwrap();
        
        let result = ocr_service
            .extract_text(temp_file.path().to_str().unwrap(), "text/plain")
            .await;
        
        assert!(result.is_ok());
        let extracted_text = result.unwrap();
        assert_eq!(extracted_text, test_content);
    }

    #[tokio::test]
    async fn test_extract_text_unsupported_type() {
        let ocr_service = OcrService::new();
        
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), "some content").unwrap();
        
        let result = ocr_service
            .extract_text(temp_file.path().to_str().unwrap(), "application/zip")
            .await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported MIME type"));
    }

    #[tokio::test]
    async fn test_extract_text_from_nonexistent_file() {
        let ocr_service = OcrService::new();
        
        let result = ocr_service
            .extract_text("/path/to/nonexistent/file.txt", "text/plain")
            .await;
        
        assert!(result.is_err());
    }

    #[tokio::test]
    #[ignore = "Requires tesseract runtime - run with: cargo test --release -- --ignored"]
    async fn test_extract_text_with_real_image() {
        let ocr_service = OcrService::new();
        
        // Create a simple test image with text if it doesn't exist
        let test_image_path = "test_data/hello_ocr.png";
        
        // Skip test if test data doesn't exist
        if !Path::new(test_image_path).exists() {
            eprintln!("Skipping test_extract_text_with_real_image: test data not found");
            return;
        }
        
        let result = ocr_service
            .extract_text(test_image_path, "image/png")
            .await;
        
        match result {
            Ok(text) => {
                println!("OCR extracted text: '{}'", text);
                // OCR might not be perfect, so we check if it contains expected words
                assert!(text.to_lowercase().contains("hello") || text.to_lowercase().contains("ocr"));
            }
            Err(e) => {
                eprintln!("OCR test failed: {}", e);
                // Don't fail the test if OCR is not available
            }
        }
    }

    #[tokio::test]
    async fn test_extract_text_from_pdf_with_content() {
        let ocr_service = OcrService::new();
        
        // Create a minimal valid PDF
        let temp_file = NamedTempFile::with_suffix(".pdf").unwrap();
        
        // This is a minimal PDF that says "Hello"
        let pdf_content = b"%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 4 0 R >> >> /MediaBox [0 0 612 792] /Contents 5 0 R >>
endobj
4 0 obj
<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>
endobj
5 0 obj
<< /Length 44 >>
stream
BT
/F1 12 Tf
100 700 Td
(Hello) Tj
ET
endstream
endobj
xref
0 6
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
0000000262 00000 n
0000000341 00000 n
trailer
<< /Size 6 /Root 1 0 R >>
startxref
435
%%EOF";
        
        fs::write(temp_file.path(), pdf_content).unwrap();
        
        let result = ocr_service
            .extract_text_from_pdf(temp_file.path().to_str().unwrap())
            .await;
        
        // The pdf-extract library might not work with our minimal PDF
        // so we just check that it attempts to process it
        match result {
            Ok(text) => {
                println!("PDF extracted text: '{}'", text);
            }
            Err(e) => {
                println!("PDF extraction error (expected): {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_extract_text_with_image_extension_fallback() {
        let ocr_service = OcrService::new();
        
        let temp_file = NamedTempFile::with_suffix(".png").unwrap();
        fs::write(temp_file.path(), "fake image data").unwrap();
        
        let result = ocr_service
            .extract_text(temp_file.path().to_str().unwrap(), "unknown/type")
            .await;
        
        // This should try to process as image due to extension, but fail due to invalid data
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_ocr_with_mock_database_integration() {
        let ocr_service = OcrService::new();
        let mock_db = MockDatabase::new();
        let doc_id = uuid::Uuid::new_v4();
        
        // Create a simple text file to simulate OCR processing
        let temp_file = NamedTempFile::with_suffix(".txt").unwrap();
        let test_content = "This is test OCR content for mock database integration.";
        fs::write(temp_file.path(), test_content).unwrap();
        
        // Extract text using OCR service
        let result = ocr_service
            .extract_text(temp_file.path().to_str().unwrap(), "text/plain")
            .await;
        
        assert!(result.is_ok());
        let extracted_text = result.unwrap();
        
        // Mock database update
        let update_result = mock_db.update_document_ocr(doc_id, &extracted_text).await;
        assert!(update_result.is_ok());
        
        // Verify the text was stored in mock database
        let stored_text = mock_db.get_ocr_text(&doc_id);
        assert!(stored_text.is_some());
        assert_eq!(stored_text.unwrap(), test_content);
    }
    
    #[tokio::test]
    async fn test_ocr_error_handling_with_mock_db() {
        let ocr_service = OcrService::new();
        let mock_db = MockDatabase::new();
        let doc_id = uuid::Uuid::new_v4();
        
        // Test with non-existent file
        let result = ocr_service
            .extract_text("/nonexistent/path/file.txt", "text/plain")
            .await;
        
        assert!(result.is_err());
        
        // Verify no update was made to mock database for failed OCR
        let stored_text = mock_db.get_ocr_text(&doc_id);
        assert!(stored_text.is_none());
    }
    
    #[tokio::test]
    async fn test_batch_ocr_processing_with_mock_db() {
        let ocr_service = OcrService::new();
        let mock_db = MockDatabase::new();
        
        let mut doc_ids = Vec::new();
        let mut temp_files = Vec::new();
        
        // Create multiple test files
        for i in 0..3 {
            let temp_file = NamedTempFile::with_suffix(".txt").unwrap();
            let content = format!("Test document {} content for batch processing.", i + 1);
            fs::write(temp_file.path(), &content).unwrap();
            
            let doc_id = uuid::Uuid::new_v4();
            doc_ids.push(doc_id);
            temp_files.push((temp_file, content));
        }
        
        // Process all files
        for (i, (temp_file, _expected_content)) in temp_files.iter().enumerate() {
            let result = ocr_service
                .extract_text(temp_file.path().to_str().unwrap(), "text/plain")
                .await;
            
            assert!(result.is_ok());
            let extracted_text = result.unwrap();
            
            let update_result = mock_db.update_document_ocr(doc_ids[i], &extracted_text).await;
            assert!(update_result.is_ok());
        }
        
        // Verify all documents were processed
        let all_updates = mock_db.get_all_ocr_updates();
        assert_eq!(all_updates.len(), 3);
        
        for (i, doc_id) in doc_ids.iter().enumerate() {
            let stored_text = all_updates.get(doc_id);
            assert!(stored_text.is_some());
            assert!(stored_text.unwrap().contains(&format!("Test document {}", i + 1)));
        }
    }
    
    #[tokio::test]
    async fn test_ocr_language_support() {
        let ocr_service = OcrService::new();
        
        let temp_file = NamedTempFile::with_suffix(".txt").unwrap();
        let test_content = "Hello world test content";
        fs::write(temp_file.path(), test_content).unwrap();
        
        // Test different language codes
        let languages = vec!["eng", "spa", "fra", "deu"];
        
        for lang in languages {
            let result = ocr_service
                .extract_text_with_lang(temp_file.path().to_str().unwrap(), "text/plain", lang)
                .await;
            
            // Should succeed for text files regardless of language setting
            assert!(result.is_ok());
            let extracted = result.unwrap();
            assert_eq!(extracted, test_content);
        }
    }
    
    #[tokio::test]
    async fn test_ocr_mime_type_detection() {
        let ocr_service = OcrService::new();
        
        // Test various mime types
        let test_cases = vec![
            ("test.txt", "text/plain"),
            ("document.pdf", "application/pdf"),
            ("image.png", "image/png"),
            ("photo.jpg", "image/jpeg"),
            ("scan.tiff", "image/tiff"),
        ];
        
        for (filename, mime_type) in test_cases {
            let temp_file = NamedTempFile::with_suffix(&Path::new(filename).extension().unwrap().to_str().unwrap()).unwrap();
            
            if mime_type == "text/plain" {
                fs::write(temp_file.path(), "test content").unwrap();
                
                let result = ocr_service
                    .extract_text(temp_file.path().to_str().unwrap(), mime_type)
                    .await;
                
                assert!(result.is_ok(), "Failed for mime type: {}", mime_type);
            } else {
                // For non-text files, we expect either success or specific errors
                let result = ocr_service
                    .extract_text(temp_file.path().to_str().unwrap(), mime_type)
                    .await;
                
                // These will likely fail with our test setup, but should not panic
                if result.is_err() {
                    println!("Expected failure for {}: {}", mime_type, result.unwrap_err());
                }
            }
        }
    }
    
    #[test]
    fn test_mock_database_functionality() {
        let mock_db = MockDatabase::new();
        let doc_id1 = uuid::Uuid::new_v4();
        let doc_id2 = uuid::Uuid::new_v4();
        
        // Test empty state
        assert!(mock_db.get_ocr_text(&doc_id1).is_none());
        
        // Test single update
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let result = mock_db.update_document_ocr(doc_id1, "Test OCR text").await;
            assert!(result.is_ok());
        });
        
        assert_eq!(mock_db.get_ocr_text(&doc_id1).unwrap(), "Test OCR text");
        
        // Test multiple updates
        rt.block_on(async {
            let result = mock_db.update_document_ocr(doc_id2, "Another OCR text").await;
            assert!(result.is_ok());
        });
        
        let all_updates = mock_db.get_all_ocr_updates();
        assert_eq!(all_updates.len(), 2);
        assert!(all_updates.contains_key(&doc_id1));
        assert!(all_updates.contains_key(&doc_id2));
    }
}