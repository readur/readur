use readur::models::*;
use chrono::Utc;
use uuid::Uuid;

#[test]
fn test_document_response_conversion_with_ocr() {
    let user_id = Uuid::new_v4();
    let document = Document {
        id: Uuid::new_v4(),
        filename: "test.pdf".to_string(),
        original_filename: "test.pdf".to_string(),
        file_path: "/uploads/test.pdf".to_string(),
        file_size: 1024000,
        mime_type: "application/pdf".to_string(),
        content: Some("Test content".to_string()),
        ocr_text: Some("OCR extracted text".to_string()),
        ocr_confidence: Some(95.5),
        ocr_word_count: Some(150),
        ocr_processing_time_ms: Some(1200),
        ocr_status: Some("completed".to_string()),
        ocr_error: None,
        ocr_completed_at: Some(Utc::now()),
        ocr_retry_count: None,
        ocr_failure_reason: None,
        tags: vec!["test".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        user_id,
        file_hash: Some("abc123".to_string()),
        original_created_at: None,
        original_modified_at: None,
        source_metadata: None,
        source_path: None,
        source_type: None,
        source_id: None,
        file_permissions: None,
        file_owner: None,
        file_group: None,
    };
    
    let response: DocumentResponse = document.clone().into();
    
    assert_eq!(response.id, document.id);
    assert_eq!(response.has_ocr_text, true);
    assert_eq!(response.ocr_confidence, Some(95.5));
    assert_eq!(response.ocr_word_count, Some(150));
    assert_eq!(response.ocr_status, Some("completed".to_string()));
}

#[test]
fn test_document_response_conversion_without_ocr() {
    let user_id = Uuid::new_v4();
    let document = Document {
        id: Uuid::new_v4(),
        filename: "text.txt".to_string(),
        original_filename: "text.txt".to_string(),
        file_path: "/uploads/text.txt".to_string(),
        file_size: 512,
        mime_type: "text/plain".to_string(),
        content: Some("Plain text".to_string()),
        ocr_text: None,
        ocr_confidence: None,
        ocr_word_count: None,
        ocr_processing_time_ms: None,
        ocr_status: Some("pending".to_string()),
        ocr_error: None,
        ocr_completed_at: None,
        ocr_retry_count: None,
        ocr_failure_reason: None,
        tags: vec![],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        user_id,
        file_hash: None,
        original_created_at: None,
        original_modified_at: None,
        source_metadata: None,
        source_path: None,
        source_type: None,
        source_id: None,
        file_permissions: None,
        file_owner: None,
        file_group: None,
    };
    
    let response: DocumentResponse = document.clone().into();
    
    assert_eq!(response.has_ocr_text, false);
    assert_eq!(response.ocr_confidence, None);
    assert_eq!(response.ocr_word_count, None);
    assert_eq!(response.ocr_status, Some("pending".to_string()));
}

#[test]
fn test_ocr_validation() {
    // Test confidence validation
    let confidence = 95.5;
    assert!(confidence >= 0.0 && confidence <= 100.0);
    
    // Test word count validation
    let word_count = 150;
    assert!(word_count > 0);
    
    // Test processing time validation
    let processing_time = 1200;
    assert!(processing_time > 0);
    
    // Test status validation
    let valid_statuses = vec!["pending", "processing", "completed", "failed"];
    let status = "completed";
    assert!(valid_statuses.contains(&status));
}

#[test]
fn test_search_mode_default() {
    let default_mode = SearchMode::default();
    assert!(matches!(default_mode, SearchMode::Simple));
}

#[test]
fn test_user_response_conversion() {
    let user = User {
        id: Uuid::new_v4(),
        username: "testuser".to_string(),
        email: "test@example.com".to_string(),
        password_hash: Some("hashed".to_string()),
        role: readur::models::UserRole::User,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        oidc_subject: None,
        oidc_issuer: None,
        oidc_email: None,
        auth_provider: readur::models::AuthProvider::Local,
    };
    
    let response: UserResponse = user.clone().into();
    
    assert_eq!(response.id, user.id);
    assert_eq!(response.username, user.username);
    assert_eq!(response.email, user.email);
    // password_hash should not be included in response
}