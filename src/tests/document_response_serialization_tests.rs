#[cfg(test)]
mod tests {
    use crate::models::DocumentResponse;
    use serde_json;
    use uuid::Uuid;
    use chrono::{DateTime, Utc};

    #[test]
    fn test_document_response_deserializes_without_new_metadata_fields() {
        // Simulate an old server response without the new metadata fields
        let old_response_json = r#"{
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "filename": "test.pdf",
            "original_filename": "test.pdf",
            "file_size": 1024,
            "mime_type": "application/pdf",
            "tags": [],
            "labels": [],
            "created_at": "2024-01-01T12:00:00Z",
            "has_ocr_text": false,
            "ocr_confidence": null,
            "ocr_word_count": null,
            "ocr_processing_time_ms": null,
            "ocr_status": null
        }"#;

        // Test deserialization - this should work with our serde defaults
        let result: Result<DocumentResponse, _> = serde_json::from_str(old_response_json);
        
        assert!(result.is_ok(), "Should deserialize successfully without metadata fields");
        
        let doc = result.unwrap();
        assert_eq!(doc.filename, "test.pdf");
        assert!(doc.original_created_at.is_none());
        assert!(doc.original_modified_at.is_none());
        assert!(doc.source_metadata.is_none());
    }

    #[test]
    fn test_document_response_deserializes_with_new_metadata_fields() {
        // Simulate a new server response with the metadata fields
        let new_response_json = r#"{
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "filename": "test.pdf",
            "original_filename": "test.pdf",
            "file_size": 1024,
            "mime_type": "application/pdf",
            "tags": [],
            "labels": [],
            "created_at": "2024-01-01T12:00:00Z",
            "has_ocr_text": false,
            "ocr_confidence": null,
            "ocr_word_count": null,
            "ocr_processing_time_ms": null,
            "ocr_status": null,
            "original_created_at": "2023-12-01T10:00:00Z",
            "original_modified_at": "2023-12-15T15:30:00Z",
            "source_metadata": {"permissions": "644", "owner": "user1"}
        }"#;

        // Test deserialization
        let result: Result<DocumentResponse, _> = serde_json::from_str(new_response_json);
        
        assert!(result.is_ok(), "Should deserialize successfully with metadata fields");
        
        let doc = result.unwrap();
        assert_eq!(doc.filename, "test.pdf");
        assert!(doc.original_created_at.is_some());
        assert!(doc.original_modified_at.is_some());
        assert!(doc.source_metadata.is_some());
        
        // Verify metadata content
        let metadata = doc.source_metadata.unwrap();
        assert_eq!(metadata["permissions"], "644");
        assert_eq!(metadata["owner"], "user1");
    }

    #[test]
    fn test_document_response_serializes_with_metadata_fields() {
        use crate::models::Document;
        
        // Create a test document with metadata
        let doc = Document {
            id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
            filename: "test.pdf".to_string(),
            original_filename: "test.pdf".to_string(),
            file_path: "/test/test.pdf".to_string(),
            file_size: 1024,
            mime_type: "application/pdf".to_string(),
            content: None,
            ocr_text: None,
            ocr_confidence: None,
            ocr_word_count: None,
            ocr_processing_time_ms: None,
            ocr_status: None,
            ocr_error: None,
            ocr_completed_at: None,
            tags: vec![],
            created_at: DateTime::parse_from_rfc3339("2024-01-01T12:00:00Z").unwrap().with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339("2024-01-01T12:00:00Z").unwrap().with_timezone(&Utc),
            user_id: Uuid::new_v4(),
            file_hash: Some("abcd1234".to_string()),
            original_created_at: Some(DateTime::parse_from_rfc3339("2023-12-01T10:00:00Z").unwrap().with_timezone(&Utc)),
            original_modified_at: Some(DateTime::parse_from_rfc3339("2023-12-15T15:30:00Z").unwrap().with_timezone(&Utc)),
            source_metadata: Some(serde_json::json!({"permissions": "644", "owner": "user1"})),
        };

        // Convert to DocumentResponse
        let response: DocumentResponse = doc.into();
        
        // Serialize to JSON
        let json = serde_json::to_string(&response).unwrap();
        
        // Should contain the metadata fields
        assert!(json.contains("original_created_at"));
        assert!(json.contains("original_modified_at"));
        assert!(json.contains("source_metadata"));
        assert!(json.contains("permissions"));
        assert!(json.contains("644"));
    }
}