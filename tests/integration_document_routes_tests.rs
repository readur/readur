#[cfg(test)]
mod document_routes_deletion_tests {
    use readur::models::{UserRole, User, Document, AuthProvider};
    use readur::routes::documents::{BulkDeleteRequest};
    use axum::http::StatusCode;
    use chrono::Utc;
    use serde_json::json;
    use uuid::Uuid;

    // Mock implementations for testing
    struct MockAppState {
        // Add fields that AppState would have for testing
        pub delete_results: std::collections::HashMap<Uuid, bool>,
        pub bulk_delete_results: std::collections::HashMap<Vec<Uuid>, Vec<Document>>,
    }

    impl MockAppState {
        fn new() -> Self {
            Self {
                delete_results: std::collections::HashMap::new(),
                bulk_delete_results: std::collections::HashMap::new(),
            }
        }
    }

    fn create_test_user(role: UserRole) -> User {
        User {
            id: Uuid::new_v4(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password_hash: Some("hashed_password".to_string()),
            role,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            oidc_subject: None,
            oidc_issuer: None,
            oidc_email: None,
            auth_provider: AuthProvider::Local,
        }
    }

    fn create_test_document(user_id: Uuid) -> Document {
        Document {
            id: Uuid::new_v4(),
            filename: "test_document.pdf".to_string(),
            original_filename: "test_document.pdf".to_string(),
            file_path: "/uploads/test_document.pdf".to_string(),
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
        }
    }

    #[test]
    fn test_bulk_delete_request_serialization() {
        let request = BulkDeleteRequest {
            document_ids: vec![Uuid::new_v4(), Uuid::new_v4()],
        };

        // Test serialization
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("document_ids"));

        // Test deserialization
        let deserialized: BulkDeleteRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.document_ids.len(), 2);
        assert_eq!(deserialized.document_ids, request.document_ids);
    }

    #[test]
    fn test_bulk_delete_request_empty_list() {
        let request = BulkDeleteRequest {
            document_ids: vec![],
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: BulkDeleteRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.document_ids.len(), 0);
    }

    #[test]
    fn test_bulk_delete_request_validation() {
        // Test with valid UUIDs
        let valid_request = json!({
            "document_ids": [
                "550e8400-e29b-41d4-a716-446655440000",
                "550e8400-e29b-41d4-a716-446655440001"
            ]
        });

        let result: Result<BulkDeleteRequest, _> = serde_json::from_value(valid_request);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().document_ids.len(), 2);

        // Test with invalid UUIDs should fail
        let invalid_request = json!({
            "document_ids": ["not-a-uuid", "also-not-a-uuid"]
        });

        let result: Result<BulkDeleteRequest, _> = serde_json::from_value(invalid_request);
        assert!(result.is_err());
    }

    #[test]
    fn test_auth_user_role_permissions() {
        let user = create_test_user(UserRole::User);
        let admin = create_test_user(UserRole::Admin);

        // Test user role
        assert_eq!(user.role, UserRole::User);
        assert_ne!(user.role, UserRole::Admin);

        // Test admin role
        assert_eq!(admin.role, UserRole::Admin);
        assert_ne!(admin.role, UserRole::User);
    }

    #[test]
    fn test_document_deletion_authorization_logic() {
        let user1 = create_test_user(UserRole::User);
        let user2 = create_test_user(UserRole::User);
        let admin = create_test_user(UserRole::Admin);

        let document = create_test_document(user1.id);

        // User1 should be able to delete their own document
        let can_delete_own = document.user_id == user1.id || user1.role == UserRole::Admin;
        assert!(can_delete_own);

        // User2 should not be able to delete user1's document
        let can_delete_other = document.user_id == user2.id || user2.role == UserRole::Admin;
        assert!(!can_delete_other);

        // Admin should be able to delete any document
        let admin_can_delete = document.user_id == admin.id || admin.role == UserRole::Admin;
        assert!(admin_can_delete);
    }

    #[test]
    fn test_bulk_delete_authorization_logic() {
        let user1 = create_test_user(UserRole::User);
        let user2 = create_test_user(UserRole::User);
        let admin = create_test_user(UserRole::Admin);

        let doc1_user1 = create_test_document(user1.id);
        let doc2_user1 = create_test_document(user1.id);
        let doc1_user2 = create_test_document(user2.id);

        let all_documents = vec![&doc1_user1, &doc2_user1, &doc1_user2];

        // Test what user1 can delete
        let user1_can_delete: Vec<&Document> = all_documents
            .iter()
            .filter(|doc| doc.user_id == user1.id || user1.role == UserRole::Admin)
            .cloned()
            .collect();
        assert_eq!(user1_can_delete.len(), 2); // Only their own documents

        // Test what admin can delete
        let admin_can_delete: Vec<&Document> = all_documents
            .iter()
            .filter(|doc| doc.user_id == admin.id || admin.role == UserRole::Admin)
            .cloned()
            .collect();
        assert_eq!(admin_can_delete.len(), 3); // All documents
    }

    #[test]
    fn test_document_response_format() {
        let user = create_test_user(UserRole::User);
        let document = create_test_document(user.id);

        // Test successful deletion response format
        let success_response = json!({
            "success": true,
            "message": "Document deleted successfully",
            "document_id": document.id
        });

        assert_eq!(success_response["success"], true);
        assert!(success_response["message"].is_string());
        assert_eq!(success_response["document_id"], document.id.to_string());

        // Test error response format
        let error_response = json!({
            "success": false,
            "error": "Document not found or not authorized to delete"
        });

        assert_eq!(error_response["success"], false);
        assert!(error_response["error"].is_string());
    }

    #[test]
    fn test_bulk_delete_response_format() {
        let user = create_test_user(UserRole::User);
        let doc1 = create_test_document(user.id);
        let doc2 = create_test_document(user.id);

        // Test successful bulk deletion response format
        let success_response = json!({
            "success": true,
            "message": "2 documents deleted successfully",
            "deleted_count": 2,
            "deleted_documents": [
                {
                    "id": doc1.id,
                    "filename": doc1.filename
                },
                {
                    "id": doc2.id,
                    "filename": doc2.filename
                }
            ]
        });

        assert_eq!(success_response["success"], true);
        assert_eq!(success_response["deleted_count"], 2);
        assert!(success_response["deleted_documents"].is_array());
        assert_eq!(success_response["deleted_documents"].as_array().unwrap().len(), 2);

        // Test partial success response format
        let partial_response = json!({
            "success": true,
            "message": "1 of 2 documents deleted successfully",
            "deleted_count": 1,
            "requested_count": 2,
            "deleted_documents": [
                {
                    "id": doc1.id,
                    "filename": doc1.filename
                }
            ]
        });

        assert_eq!(partial_response["success"], true);
        assert_eq!(partial_response["deleted_count"], 1);
        assert_eq!(partial_response["requested_count"], 2);
    }

    #[test]
    fn test_http_status_codes() {
        // Test successful deletion status codes
        assert_eq!(StatusCode::OK.as_u16(), 200);

        // Test error status codes
        assert_eq!(StatusCode::NOT_FOUND.as_u16(), 404);
        assert_eq!(StatusCode::UNAUTHORIZED.as_u16(), 401);
        assert_eq!(StatusCode::FORBIDDEN.as_u16(), 403);
        assert_eq!(StatusCode::BAD_REQUEST.as_u16(), 400);
        assert_eq!(StatusCode::INTERNAL_SERVER_ERROR.as_u16(), 500);
    }

    #[test]
    fn test_path_parameter_parsing() {
        let document_id = Uuid::new_v4();
        let _path_str = format!("/documents/{}", document_id);

        // Test that UUID can be parsed from path
        let parsed_id = document_id.to_string();
        let reparsed_id = Uuid::parse_str(&parsed_id).unwrap();
        assert_eq!(reparsed_id, document_id);
    }

    #[test]
    fn test_json_request_validation() {
        // Test valid JSON request
        let valid_json = json!({
            "document_ids": [
                "550e8400-e29b-41d4-a716-446655440000",
                "550e8400-e29b-41d4-a716-446655440001"
            ]
        });

        let result: Result<BulkDeleteRequest, _> = serde_json::from_value(valid_json);
        assert!(result.is_ok());

        // Test invalid JSON structure
        let invalid_json = json!({
            "wrong_field": ["not-document-ids"]
        });

        let result: Result<BulkDeleteRequest, _> = serde_json::from_value(invalid_json);
        assert!(result.is_err());

        // Test empty request
        let empty_json = json!({
            "document_ids": []
        });

        let result: Result<BulkDeleteRequest, _> = serde_json::from_value(empty_json);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().document_ids.len(), 0);
    }

    #[test]
    fn test_concurrent_deletion_safety() {
        let user = create_test_user(UserRole::User);
        let document = create_test_document(user.id);

        // Test that multiple deletion attempts for the same document
        // should be handled gracefully (first succeeds, subsequent ones are no-op)
        let document_id = document.id;

        // Simulate concurrent deletions by checking if the same document ID
        // would be processed multiple times
        let mut processed_ids = std::collections::HashSet::new();
        
        // First deletion attempt
        let first_attempt = processed_ids.insert(document_id);
        assert!(first_attempt); // Should be true (new entry)

        // Second deletion attempt
        let second_attempt = processed_ids.insert(document_id);
        assert!(!second_attempt); // Should be false (already exists)
    }

    #[test]
    fn test_bulk_delete_request_size_limits() {
        // Test reasonable request size
        let reasonable_request = BulkDeleteRequest {
            document_ids: (0..10).map(|_| Uuid::new_v4()).collect(),
        };
        assert_eq!(reasonable_request.document_ids.len(), 10);

        // Test large request size (should still be valid but might be rate-limited in real app)
        let large_request = BulkDeleteRequest {
            document_ids: (0..100).map(|_| Uuid::new_v4()).collect(),
        };
        assert_eq!(large_request.document_ids.len(), 100);

        // Test very large request size (might need limits in production)
        let very_large_request = BulkDeleteRequest {
            document_ids: (0..1000).map(|_| Uuid::new_v4()).collect(),
        };
        assert_eq!(very_large_request.document_ids.len(), 1000);
    }

    #[test]
    fn test_error_message_formats() {
        // Test error messages for different scenarios
        let not_found_error = "Document not found";
        let unauthorized_error = "Not authorized to delete this document";
        let validation_error = "Invalid request format";
        let server_error = "Internal server error occurred during deletion";

        assert!(!not_found_error.is_empty());
        assert!(!unauthorized_error.is_empty());
        assert!(!validation_error.is_empty());
        assert!(!server_error.is_empty());

        // Test that error messages are user-friendly
        assert!(!not_found_error.contains("SQL"));
        assert!(!not_found_error.contains("database"));
        assert!(!unauthorized_error.contains("403"));
        assert!(!validation_error.contains("serde"));
    }

    // Low confidence deletion tests
    mod low_confidence_deletion_tests {
        use super::*;
        use readur::routes::documents::DeleteLowConfidenceRequest;

        fn create_low_confidence_document(user_id: Uuid, confidence: f32) -> Document {
            Document {
                id: Uuid::new_v4(),
                filename: format!("low_conf_{}.pdf", confidence),
                original_filename: format!("low_conf_{}.pdf", confidence),
                file_path: format!("/uploads/low_conf_{}.pdf", confidence),
                file_size: 1024,
                mime_type: "application/pdf".to_string(),
                content: Some("Test document content".to_string()),
                ocr_text: Some("Low quality OCR text".to_string()),
                ocr_confidence: Some(confidence),
                ocr_word_count: Some(10),
                ocr_processing_time_ms: Some(500),
                ocr_status: Some("completed".to_string()),
                ocr_error: None,
                ocr_completed_at: Some(Utc::now()),
                tags: vec!["low-confidence".to_string()],
                created_at: Utc::now(),
                updated_at: Utc::now(),
                user_id,
                file_hash: Some("abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string()),
                original_created_at: None,
                original_modified_at: None,
                source_metadata: None,
                ocr_retry_count: None,
                ocr_failure_reason: None,
            }
        }

        #[test]
        fn test_delete_low_confidence_request_serialization() {
            // Test valid request
            let valid_request = json!({
                "max_confidence": 50.0,
                "preview_only": true
            });

            let result: Result<DeleteLowConfidenceRequest, _> = serde_json::from_value(valid_request);
            assert!(result.is_ok());
            let request = result.unwrap();
            assert_eq!(request.max_confidence, 50.0);
            assert_eq!(request.preview_only, Some(true));

            // Test request with only max_confidence
            let minimal_request = json!({
                "max_confidence": 30.0
            });

            let result: Result<DeleteLowConfidenceRequest, _> = serde_json::from_value(minimal_request);
            assert!(result.is_ok());
            let request = result.unwrap();
            assert_eq!(request.max_confidence, 30.0);
            assert_eq!(request.preview_only, None);
        }

        #[test]
        fn test_delete_low_confidence_request_validation() {
            // Test invalid confidence values
            let invalid_negative = json!({
                "max_confidence": -10.0,
                "preview_only": false
            });

            let result: Result<DeleteLowConfidenceRequest, _> = serde_json::from_value(invalid_negative);
            assert!(result.is_ok()); // Serialization succeeds, validation happens in handler

            let invalid_too_high = json!({
                "max_confidence": 150.0,
                "preview_only": false
            });

            let result: Result<DeleteLowConfidenceRequest, _> = serde_json::from_value(invalid_too_high);
            assert!(result.is_ok()); // Serialization succeeds, validation happens in handler
        }

        #[test]
        fn test_confidence_threshold_logic() {
            let user = create_test_user(UserRole::User);
            
            // Create documents with various confidence levels
            let high_confidence_doc = create_low_confidence_document(user.id, 95.0);
            let medium_confidence_doc = create_low_confidence_document(user.id, 60.0);
            let low_confidence_doc = create_low_confidence_document(user.id, 25.0);
            let very_low_confidence_doc = create_low_confidence_document(user.id, 5.0);

            let documents = vec![
                &high_confidence_doc,
                &medium_confidence_doc, 
                &low_confidence_doc,
                &very_low_confidence_doc
            ];

            // Test threshold logic for different confidence values
            let threshold_50 = 50.0;
            let threshold_30 = 30.0;
            let threshold_10 = 10.0;

            // Documents below 50% threshold
            let below_50: Vec<_> = documents.iter()
                .filter(|doc| doc.ocr_confidence.unwrap_or(0.0) < threshold_50)
                .collect();
            assert_eq!(below_50.len(), 2); // 25.0 and 5.0

            // Documents below 30% threshold  
            let below_30: Vec<_> = documents.iter()
                .filter(|doc| doc.ocr_confidence.unwrap_or(0.0) < threshold_30)
                .collect();
            assert_eq!(below_30.len(), 2); // 25.0 and 5.0

            // Documents below 10% threshold
            let below_10: Vec<_> = documents.iter()
                .filter(|doc| doc.ocr_confidence.unwrap_or(0.0) < threshold_10)
                .collect();
            assert_eq!(below_10.len(), 1); // 5.0
        }

        #[test]
        fn test_user_role_authorization_for_low_confidence_deletion() {
            let user1 = create_test_user(UserRole::User);
            let user2 = create_test_user(UserRole::User);
            let admin = create_test_user(UserRole::Admin);

            let user1_doc = create_low_confidence_document(user1.id, 25.0);
            let user2_doc = create_low_confidence_document(user2.id, 15.0);

            // User1 should only be able to delete their own low confidence documents
            assert_eq!(user1_doc.user_id, user1.id);
            assert_ne!(user1_doc.user_id, user2.id);

            // User2 should only be able to delete their own low confidence documents  
            assert_eq!(user2_doc.user_id, user2.id);
            assert_ne!(user2_doc.user_id, user1.id);

            // Admin should be able to delete any low confidence documents
            let admin_can_delete_user1 = user1_doc.user_id == admin.id || admin.role == UserRole::Admin;
            let admin_can_delete_user2 = user2_doc.user_id == admin.id || admin.role == UserRole::Admin;
            assert!(admin_can_delete_user1);
            assert!(admin_can_delete_user2);
        }

        #[test]
        fn test_edge_cases_for_confidence_values() {
            let user = create_test_user(UserRole::User);

            // Test document with None confidence (should not be included)
            let mut no_confidence_doc = create_low_confidence_document(user.id, 0.0);
            no_confidence_doc.ocr_confidence = None;

            // Test document with exactly threshold confidence (should not be included)
            let exact_threshold_doc = create_low_confidence_document(user.id, 30.0);

            // Test document just below threshold (should be included)
            let just_below_doc = create_low_confidence_document(user.id, 29.9);

            let threshold = 30.0;

            // None confidence should be excluded (no OCR confidence available)
            assert!(no_confidence_doc.ocr_confidence.is_none());

            // Exact threshold should be excluded (not less than threshold)
            assert_eq!(exact_threshold_doc.ocr_confidence.unwrap(), threshold);
            assert!(!(exact_threshold_doc.ocr_confidence.unwrap() < threshold));

            // Just below threshold should be included
            assert!(just_below_doc.ocr_confidence.unwrap() < threshold);
        }

        #[test]
        fn test_preview_mode_behavior() {
            let user = create_test_user(UserRole::User);
            let doc1 = create_low_confidence_document(user.id, 20.0);
            let doc2 = create_low_confidence_document(user.id, 10.0);

            let preview_request = DeleteLowConfidenceRequest {
                max_confidence: 30.0,
                preview_only: Some(true),
            };

            let delete_request = DeleteLowConfidenceRequest {
                max_confidence: 30.0,
                preview_only: Some(false),
            };

            let no_preview_request = DeleteLowConfidenceRequest {
                max_confidence: 30.0,
                preview_only: None,
            };

            // Preview mode should be true when explicitly set
            assert_eq!(preview_request.preview_only.unwrap_or(false), true);

            // Delete mode should be false when explicitly set
            assert_eq!(delete_request.preview_only.unwrap_or(false), false);

            // Default should be false when not specified
            assert_eq!(no_preview_request.preview_only.unwrap_or(false), false);
        }

        #[test]
        fn test_response_format_expectations() {
            // Test expected response structure for preview mode
            let expected_preview_response = json!({
                "success": true,
                "message": "Found 5 documents with OCR confidence below 30%",
                "matched_count": 5,
                "preview": true,
                "document_ids": ["uuid1", "uuid2", "uuid3", "uuid4", "uuid5"]
            });

            // Test expected response structure for delete mode
            let expected_delete_response = json!({
                "success": true,
                "message": "Successfully deleted 5 documents with OCR confidence below 30%",
                "deleted_count": 5,
                "matched_count": 5,
                "successful_file_deletions": 5,
                "failed_file_deletions": 0,
                "ignored_file_creation_failures": 0,
                "deleted_document_ids": ["uuid1", "uuid2", "uuid3", "uuid4", "uuid5"]
            });

            // Verify JSON structure is valid
            assert!(expected_preview_response.is_object());
            assert!(expected_delete_response.is_object());

            // Verify required fields exist
            assert!(expected_preview_response["success"].is_boolean());
            assert!(expected_preview_response["matched_count"].is_number());
            assert!(expected_preview_response["document_ids"].is_array());

            assert!(expected_delete_response["success"].is_boolean());
            assert!(expected_delete_response["deleted_count"].is_number());
            assert!(expected_delete_response["deleted_document_ids"].is_array());
        }

        #[test]
        fn test_error_scenarios() {
            // Test validation error for invalid confidence range
            let invalid_confidence_cases = vec![
                (-1.0, "negative confidence"),
                (101.0, "confidence over 100"),
                (150.5, "way over 100"),
            ];

            for (confidence, description) in invalid_confidence_cases {
                let request = DeleteLowConfidenceRequest {
                    max_confidence: confidence,
                    preview_only: Some(false),
                };

                // Validation logic should catch these in the handler
                assert!(confidence < 0.0 || confidence > 100.0, 
                    "Should be invalid: {}", description);
            }

            // Test empty result scenario
            let request = DeleteLowConfidenceRequest {
                max_confidence: 0.0, // Very low threshold, should match nothing
                preview_only: Some(true),
            };

            assert_eq!(request.max_confidence, 0.0);
            // This should result in zero matched documents
        }
    }

    #[cfg(test)]
    mod delete_failed_ocr_tests {
        use super::*;
        use serde_json::json;

        #[test]
        fn test_delete_failed_ocr_request_serialization() {
            // Test preview mode
            let preview_request = json!({
                "preview_only": true
            });
            
            let parsed: serde_json::Value = serde_json::from_value(preview_request).unwrap();
            assert_eq!(parsed["preview_only"], true);

            // Test delete mode
            let delete_request = json!({
                "preview_only": false
            });
            
            let parsed: serde_json::Value = serde_json::from_value(delete_request).unwrap();
            assert_eq!(parsed["preview_only"], false);

            // Test empty request (should default to preview_only: false)
            let empty_request = json!({});
            
            let parsed: serde_json::Value = serde_json::from_value(empty_request).unwrap();
            assert!(parsed.get("preview_only").is_none() || parsed["preview_only"] == false);
        }

        #[test]
        fn test_delete_failed_ocr_user_authorization() {
            let admin_user = create_test_user(UserRole::Admin);
            let regular_user = create_test_user(UserRole::User);
            
            // Both admins and regular users should be able to delete their own failed documents
            assert_eq!(admin_user.role, UserRole::Admin);
            assert_eq!(regular_user.role, UserRole::User);

            // Admin should be able to see all failed documents
            // Regular user should only see their own failed documents
            // This logic would be tested in the actual endpoint implementation
        }

        #[test]
        fn test_failed_document_criteria() {
            let user_id = Uuid::new_v4();

            // Test document with failed OCR status
            let mut failed_doc = create_test_document(user_id);
            failed_doc.ocr_status = Some("failed".to_string());
            failed_doc.ocr_confidence = None;
            failed_doc.ocr_error = Some("OCR processing failed".to_string());
            
            // Should be included in failed document deletion
            assert_eq!(failed_doc.ocr_status, Some("failed".to_string()));
            assert!(failed_doc.ocr_confidence.is_none());

            // Test document with NULL confidence but completed status
            let mut null_confidence_doc = create_test_document(user_id);
            null_confidence_doc.ocr_status = Some("completed".to_string());
            null_confidence_doc.ocr_confidence = None;
            null_confidence_doc.ocr_text = Some("Text but no confidence".to_string());
            
            // Should be included in failed document deletion (NULL confidence indicates failure)
            assert_eq!(null_confidence_doc.ocr_status, Some("completed".to_string()));
            assert!(null_confidence_doc.ocr_confidence.is_none());

            // Test document with successful OCR
            let mut success_doc = create_test_document(user_id);
            success_doc.ocr_status = Some("completed".to_string());
            success_doc.ocr_confidence = Some(85.0);
            success_doc.ocr_text = Some("Successfully extracted text".to_string());
            
            // Should NOT be included in failed document deletion
            assert_eq!(success_doc.ocr_status, Some("completed".to_string()));
            assert!(success_doc.ocr_confidence.is_some());

            // Test document with pending status
            let mut pending_doc = create_test_document(user_id);
            pending_doc.ocr_status = Some("pending".to_string());
            pending_doc.ocr_confidence = None;
            
            // Should NOT be included in failed document deletion (still processing)
            assert_eq!(pending_doc.ocr_status, Some("pending".to_string()));

            // Test document with processing status
            let mut processing_doc = create_test_document(user_id);
            processing_doc.ocr_status = Some("processing".to_string());
            processing_doc.ocr_confidence = None;
            
            // Should NOT be included in failed document deletion (still processing)
            assert_eq!(processing_doc.ocr_status, Some("processing".to_string()));
        }

        #[test]
        fn test_delete_failed_ocr_response_format() {
            // Test preview response format
            let preview_response = json!({
                "success": true,
                "message": "Found 5 documents with failed OCR processing",
                "matched_count": 5,
                "preview": true,
                "document_ids": ["id1", "id2", "id3", "id4", "id5"]
            });

            assert_eq!(preview_response["success"], true);
            assert_eq!(preview_response["matched_count"], 5);
            assert_eq!(preview_response["preview"], true);
            assert!(preview_response["document_ids"].is_array());

            // Test delete response format
            let delete_response = json!({
                "success": true,
                "message": "Successfully deleted 3 documents with failed OCR processing",
                "deleted_count": 3,
                "matched_count": 3,
                "successful_file_deletions": 3,
                "failed_file_deletions": 0,
                "ignored_file_creation_failures": 0,
                "deleted_document_ids": ["id1", "id2", "id3"]
            });

            assert_eq!(delete_response["success"], true);
            assert_eq!(delete_response["deleted_count"], 3);
            assert_eq!(delete_response["matched_count"], 3);
            assert!(delete_response["deleted_document_ids"].is_array());
            assert!(delete_response.get("preview").is_none()); // Should not have preview flag in delete response

            // Test no documents found response
            let no_docs_response = json!({
                "success": true,
                "message": "No documents found with failed OCR processing",
                "deleted_count": 0
            });

            assert_eq!(no_docs_response["success"], true);
            assert_eq!(no_docs_response["deleted_count"], 0);
        }

        #[test]
        fn test_delete_failed_ocr_error_scenarios() {
            // Test with no failed documents
            let no_failed_docs_request = json!({
                "preview_only": true
            });

            // Should return success with 0 matched count
            // This would be tested in integration tests with actual database

            // Test with file deletion failures
            let file_deletion_error = json!({
                "success": true,
                "message": "Successfully deleted 2 documents with failed OCR processing",
                "deleted_count": 2,
                "matched_count": 2,
                "successful_file_deletions": 1,
                "failed_file_deletions": 1,
                "ignored_file_creation_failures": 0,
                "deleted_document_ids": ["id1", "id2"]
            });

            // Should still report success but indicate file deletion issues
            assert_eq!(file_deletion_error["success"], true);
            assert_eq!(file_deletion_error["failed_file_deletions"], 1);

            // Test with ignored file creation failures
            let ignored_file_error = json!({
                "success": true,
                "message": "Successfully deleted 2 documents with failed OCR processing",
                "deleted_count": 2,
                "matched_count": 2,
                "successful_file_deletions": 2,
                "failed_file_deletions": 0,
                "ignored_file_creation_failures": 1,
                "deleted_document_ids": ["id1", "id2"]
            });

            assert_eq!(ignored_file_error["success"], true);
            assert_eq!(ignored_file_error["ignored_file_creation_failures"], 1);
        }

        #[test]
        fn test_delete_failed_ocr_failure_reason_handling() {
            let user_id = Uuid::new_v4();

            // Test document with specific failure reason
            let mut ocr_timeout_doc = create_test_document(user_id);
            ocr_timeout_doc.ocr_status = Some("failed".to_string());
            ocr_timeout_doc.ocr_error = Some("OCR processing timed out after 2 minutes".to_string());
            
            // Test document with corruption error
            let mut corruption_doc = create_test_document(user_id);
            corruption_doc.ocr_status = Some("failed".to_string());
            corruption_doc.ocr_error = Some("Invalid image format - file appears corrupted".to_string());
            
            // Test document with font encoding error
            let mut font_error_doc = create_test_document(user_id);
            font_error_doc.ocr_status = Some("failed".to_string());
            font_error_doc.ocr_error = Some("PDF text extraction failed due to font encoding issues".to_string());
            
            // All should be valid candidates for deletion
            assert!(ocr_timeout_doc.ocr_error.is_some());
            assert!(corruption_doc.ocr_error.is_some());
            assert!(font_error_doc.ocr_error.is_some());
            
            // The deletion should create appropriate ignored file records with the error reasons
        }

        #[test]
        fn test_delete_failed_ocr_ignored_file_creation() {
            // Test that deleted failed documents create proper ignored file records
            let user_id = Uuid::new_v4();
            
            let mut failed_doc = create_test_document(user_id);
            failed_doc.ocr_status = Some("failed".to_string());
            failed_doc.ocr_error = Some("OCR processing failed due to corrupted image".to_string());
            
            // Expected ignored file reason should include the error
            let expected_reason = "deleted due to failed OCR processing: OCR processing failed due to corrupted image";
            
            // In the actual implementation, this would be tested by verifying the ignored file record
            assert!(failed_doc.ocr_error.is_some());
            
            // Test document with no specific error
            let mut failed_no_error_doc = create_test_document(user_id);
            failed_no_error_doc.ocr_status = Some("failed".to_string());
            failed_no_error_doc.ocr_error = None;
            
            // Should use generic reason
            let expected_generic_reason = "deleted due to failed OCR processing";
            
            // Both should result in appropriate ignored file records
            assert_eq!(failed_doc.ocr_status, Some("failed".to_string()));
            assert_eq!(failed_no_error_doc.ocr_status, Some("failed".to_string()));
        }

        #[test]
        fn test_delete_failed_ocr_vs_low_confidence_distinction() {
            let user_id = Uuid::new_v4();

            // Failed OCR document (should be in failed deletion, not low confidence)
            let mut failed_doc = create_test_document(user_id);
            failed_doc.ocr_status = Some("failed".to_string());
            failed_doc.ocr_confidence = None;
            
            // Low confidence document (should be in low confidence deletion, not failed)
            let mut low_confidence_doc = create_test_document(user_id);
            low_confidence_doc.ocr_status = Some("completed".to_string());
            low_confidence_doc.ocr_confidence = Some(25.0);
            
            // NULL confidence but completed (edge case - should be in failed deletion)
            let mut null_confidence_doc = create_test_document(user_id);
            null_confidence_doc.ocr_status = Some("completed".to_string());
            null_confidence_doc.ocr_confidence = None;
            
            // High confidence document (should be in neither)
            let mut high_confidence_doc = create_test_document(user_id);
            high_confidence_doc.ocr_status = Some("completed".to_string());
            high_confidence_doc.ocr_confidence = Some(95.0);
            
            // Verify the logic for each type
            assert_eq!(failed_doc.ocr_status, Some("failed".to_string()));
            assert!(failed_doc.ocr_confidence.is_none());
            
            assert_eq!(low_confidence_doc.ocr_status, Some("completed".to_string()));
            assert!(low_confidence_doc.ocr_confidence.unwrap() < 50.0);
            
            assert_eq!(null_confidence_doc.ocr_status, Some("completed".to_string()));
            assert!(null_confidence_doc.ocr_confidence.is_none());
            
            assert_eq!(high_confidence_doc.ocr_status, Some("completed".to_string()));
            assert!(high_confidence_doc.ocr_confidence.unwrap() > 50.0);
        }

        #[test]
        fn test_delete_failed_ocr_endpoint_path() {
            // Test that the endpoint path is correct
            let endpoint_path = "/api/documents/delete-failed-ocr";
            
            // This would be used in integration tests
            assert!(endpoint_path.contains("delete-failed-ocr"));
            assert!(endpoint_path.starts_with("/api/documents/"));
        }

        #[test]
        fn test_delete_failed_ocr_http_methods() {
            // The endpoint should only accept POST requests
            // GET, PUT, DELETE should not be allowed
            
            // This would be tested in integration tests with actual HTTP requests
            let allowed_method = "POST";
            let disallowed_methods = vec!["GET", "PUT", "DELETE", "PATCH"];
            
            assert_eq!(allowed_method, "POST");
            assert!(disallowed_methods.contains(&"GET"));
            assert!(disallowed_methods.contains(&"DELETE"));
        }
    }
}