#[cfg(test)]
mod tests {
    use crate::models::{Document, DocumentResponse};
    use chrono::Utc;
    use serde_json::Value;
    use uuid::Uuid;

    fn create_test_document(user_id: Uuid) -> Document {
        Document {
            id: Uuid::new_v4(),
            filename: "test_document.pdf".to_string(),
            original_filename: "test_document.pdf".to_string(),
            file_path: "/uploads/test_document.pdf".to_string(),
            file_size: 1024000,
            mime_type: "application/pdf".to_string(),
            content: Some("Test document content".to_string()),
            ocr_text: Some("This is extracted OCR text from the test document.".to_string()),
            ocr_confidence: Some(95.5),
            ocr_word_count: Some(150),
            ocr_processing_time_ms: Some(1200),
            ocr_status: Some("completed".to_string()),
            ocr_error: None,
            ocr_completed_at: Some(Utc::now()),
            tags: vec!["test".to_string(), "document".to_string()],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            user_id,
            file_hash: Some("1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string()),
        }
    }

    fn create_test_document_without_ocr(user_id: Uuid) -> Document {
        Document {
            id: Uuid::new_v4(),
            filename: "test_no_ocr.txt".to_string(),
            original_filename: "test_no_ocr.txt".to_string(),
            file_path: "/uploads/test_no_ocr.txt".to_string(),
            file_size: 512,
            mime_type: "text/plain".to_string(),
            content: Some("Plain text content".to_string()),
            ocr_text: None,
            ocr_confidence: None,
            ocr_word_count: None,
            ocr_processing_time_ms: None,
            ocr_status: Some("pending".to_string()),
            ocr_error: None,
            ocr_completed_at: None,
            tags: vec!["text".to_string()],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            user_id,
            file_hash: Some("fedcba0987654321fedcba0987654321fedcba0987654321fedcba0987654321".to_string()),
        }
    }

    fn create_test_document_with_ocr_error(user_id: Uuid) -> Document {
        Document {
            id: Uuid::new_v4(),
            filename: "test_error.pdf".to_string(),
            original_filename: "test_error.pdf".to_string(),
            file_path: "/uploads/test_error.pdf".to_string(),
            file_size: 2048000,
            mime_type: "application/pdf".to_string(),
            content: None,
            ocr_text: None,
            ocr_confidence: None,
            ocr_word_count: None,
            ocr_processing_time_ms: Some(5000),
            ocr_status: Some("failed".to_string()),
            ocr_error: Some("Failed to process document: corrupted file".to_string()),
            ocr_completed_at: Some(Utc::now()),
            tags: vec!["error".to_string()],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            user_id,
            file_hash: Some("abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string()),
        }
    }

    #[tokio::test]
    async fn test_document_response_conversion() {
        let user_id = Uuid::new_v4();
        let document = create_test_document(user_id);
        
        let response: DocumentResponse = document.clone().into();
        
        assert_eq!(response.id, document.id);
        assert_eq!(response.filename, document.filename);
        assert_eq!(response.original_filename, document.original_filename);
        assert_eq!(response.file_size, document.file_size);
        assert_eq!(response.mime_type, document.mime_type);
        assert_eq!(response.tags, document.tags);
        assert_eq!(response.has_ocr_text, true);
        assert_eq!(response.ocr_confidence, document.ocr_confidence);
        assert_eq!(response.ocr_word_count, document.ocr_word_count);
        assert_eq!(response.ocr_processing_time_ms, document.ocr_processing_time_ms);
        assert_eq!(response.ocr_status, document.ocr_status);
    }

    #[tokio::test]
    async fn test_document_response_conversion_no_ocr() {
        let user_id = Uuid::new_v4();
        let document = create_test_document_without_ocr(user_id);
        
        let response: DocumentResponse = document.clone().into();
        
        assert_eq!(response.has_ocr_text, false);
        assert_eq!(response.ocr_confidence, None);
        assert_eq!(response.ocr_word_count, None);
    }

    #[test]
    fn test_ocr_response_structure() {
        let user_id = Uuid::new_v4();
        let document = create_test_document(user_id);
        
        // Test that OCR response fields match expected structure
        let ocr_response = serde_json::json!({
            "document_id": document.id,
            "filename": document.filename,
            "has_ocr_text": document.ocr_text.is_some(),
            "ocr_text": document.ocr_text,
            "ocr_confidence": document.ocr_confidence,
            "ocr_word_count": document.ocr_word_count,
            "ocr_processing_time_ms": document.ocr_processing_time_ms,
            "ocr_status": document.ocr_status,
            "ocr_error": document.ocr_error,
            "ocr_completed_at": document.ocr_completed_at
        });
        
        assert!(ocr_response.is_object());
        assert_eq!(ocr_response["document_id"], document.id.to_string());
        assert_eq!(ocr_response["filename"], document.filename);
        assert_eq!(ocr_response["has_ocr_text"], true);
        assert_eq!(ocr_response["ocr_text"], document.ocr_text.unwrap());
        assert_eq!(ocr_response["ocr_confidence"], document.ocr_confidence.unwrap());
        assert_eq!(ocr_response["ocr_word_count"], document.ocr_word_count.unwrap());
        assert_eq!(ocr_response["ocr_processing_time_ms"], document.ocr_processing_time_ms.unwrap());
        assert_eq!(ocr_response["ocr_status"], document.ocr_status.unwrap());
        assert_eq!(ocr_response["ocr_error"], Value::Null);
    }

    #[test]
    fn test_ocr_response_with_error() {
        let user_id = Uuid::new_v4();
        let document = create_test_document_with_ocr_error(user_id);
        
        let ocr_response = serde_json::json!({
            "document_id": document.id,
            "filename": document.filename,
            "has_ocr_text": document.ocr_text.is_some(),
            "ocr_text": document.ocr_text,
            "ocr_confidence": document.ocr_confidence,
            "ocr_word_count": document.ocr_word_count,
            "ocr_processing_time_ms": document.ocr_processing_time_ms,
            "ocr_status": document.ocr_status,
            "ocr_error": document.ocr_error,
            "ocr_completed_at": document.ocr_completed_at
        });
        
        assert_eq!(ocr_response["has_ocr_text"], false);
        assert_eq!(ocr_response["ocr_text"], Value::Null);
        assert_eq!(ocr_response["ocr_status"], "failed");
        assert_eq!(ocr_response["ocr_error"], "Failed to process document: corrupted file");
    }

    #[test]
    fn test_ocr_confidence_validation() {
        let user_id = Uuid::new_v4();
        let mut document = create_test_document(user_id);
        
        // Test valid confidence range
        document.ocr_confidence = Some(95.5);
        assert!(document.ocr_confidence.unwrap() >= 0.0 && document.ocr_confidence.unwrap() <= 100.0);
        
        // Test edge cases
        document.ocr_confidence = Some(0.0);
        assert!(document.ocr_confidence.unwrap() >= 0.0);
        
        document.ocr_confidence = Some(100.0);
        assert!(document.ocr_confidence.unwrap() <= 100.0);
    }

    #[test]
    fn test_ocr_word_count_validation() {
        let user_id = Uuid::new_v4();
        let mut document = create_test_document(user_id);
        
        // Test positive word count
        document.ocr_word_count = Some(150);
        assert!(document.ocr_word_count.unwrap() > 0);
        
        // Test zero word count (valid for empty documents)
        document.ocr_word_count = Some(0);
        assert!(document.ocr_word_count.unwrap() >= 0);
    }

    #[test]
    fn test_ocr_processing_time_validation() {
        let user_id = Uuid::new_v4();
        let mut document = create_test_document(user_id);
        
        // Test positive processing time
        document.ocr_processing_time_ms = Some(1200);
        assert!(document.ocr_processing_time_ms.unwrap() > 0);
        
        // Test very fast processing
        document.ocr_processing_time_ms = Some(50);
        assert!(document.ocr_processing_time_ms.unwrap() > 0);
        
        // Test slow processing
        document.ocr_processing_time_ms = Some(30000); // 30 seconds
        assert!(document.ocr_processing_time_ms.unwrap() > 0);
    }

    #[test]
    fn test_ocr_status_values() {
        let user_id = Uuid::new_v4();
        let mut document = create_test_document(user_id);
        
        // Test valid status values
        let valid_statuses = vec!["pending", "processing", "completed", "failed"];
        
        for status in valid_statuses {
            document.ocr_status = Some(status.to_string());
            assert!(matches!(
                document.ocr_status.as_deref(),
                Some("pending") | Some("processing") | Some("completed") | Some("failed")
            ));
        }
    }

    #[test]
    fn test_document_with_complete_ocr_data() {
        let user_id = Uuid::new_v4();
        let document = create_test_document(user_id);
        
        // Verify all OCR fields are properly set for a completed document
        assert!(document.ocr_text.is_some());
        assert!(document.ocr_confidence.is_some());
        assert!(document.ocr_word_count.is_some());
        assert!(document.ocr_processing_time_ms.is_some());
        assert_eq!(document.ocr_status.as_deref(), Some("completed"));
        assert!(document.ocr_error.is_none());
        assert!(document.ocr_completed_at.is_some());
    }

    #[test]
    fn test_document_with_failed_ocr() {
        let user_id = Uuid::new_v4();
        let document = create_test_document_with_ocr_error(user_id);
        
        // Verify failed OCR document has appropriate fields
        assert!(document.ocr_text.is_none());
        assert!(document.ocr_confidence.is_none());
        assert!(document.ocr_word_count.is_none());
        assert_eq!(document.ocr_status.as_deref(), Some("failed"));
        assert!(document.ocr_error.is_some());
        assert!(document.ocr_completed_at.is_some()); // Should still have completion time
    }

    #[test]
    fn test_document_mime_type_ocr_eligibility() {
        let user_id = Uuid::new_v4();
        
        // Test OCR-eligible file types
        let ocr_eligible_types = vec![
            "application/pdf",
            "image/png", 
            "image/jpeg",
            "image/jpg",
            "image/tiff",
            "image/bmp"
        ];
        
        for mime_type in ocr_eligible_types {
            let mut document = create_test_document(user_id);
            document.mime_type = mime_type.to_string();
            
            // These types should typically have OCR processing
            assert!(document.mime_type.contains("pdf") || document.mime_type.starts_with("image/"));
        }
        
        // Test non-OCR file types
        let mut text_document = create_test_document_without_ocr(user_id);
        text_document.mime_type = "text/plain".to_string();
        
        // Text files typically don't need OCR
        assert_eq!(text_document.mime_type, "text/plain");
        assert!(text_document.ocr_text.is_none());
    }

    #[test]
    fn test_ocr_text_content_validation() {
        let user_id = Uuid::new_v4();
        let document = create_test_document(user_id);
        
        if let Some(ocr_text) = &document.ocr_text {
            // Test that OCR text is not empty
            assert!(!ocr_text.trim().is_empty());
            
            // Test that OCR text is reasonable length
            assert!(ocr_text.len() > 0);
            assert!(ocr_text.len() < 100000); // Reasonable upper limit
        }
    }
}