use anyhow::Result;
use std::sync::Arc;
use uuid::Uuid;

use readur::{
    AppState,
    db::Database,
    config::Config,
    models::{CreateUser, UserRole, CreateIgnoredFile},
};

async fn create_test_app_state() -> Result<Arc<AppState>> {
    let config = Config::from_env().unwrap_or_else(|_| {
        let database_url = std::env::var("DATABASE_URL")
            .or_else(|_| std::env::var("TEST_DATABASE_URL"))
            .unwrap_or_else(|_| "postgresql://readur:readur@localhost:5432/readur".to_string());
        
        Config {
            database_url,
            server_address: "127.0.0.1:0".to_string(),
            jwt_secret: "test_secret".to_string(),
            upload_path: "./test_uploads".to_string(),
            watch_folder: "./test_watch".to_string(),
            allowed_file_types: vec!["pdf".to_string(), "txt".to_string()],
            watch_interval_seconds: Some(30),
            file_stability_check_ms: Some(500),
            max_file_age_hours: None,
            ocr_language: "eng".to_string(),
            concurrent_ocr_jobs: 1,
            ocr_timeout_seconds: 30,
            max_file_size_mb: 10,
            memory_limit_mb: 256,
            cpu_priority: "normal".to_string(),
            oidc_enabled: false,
            oidc_client_id: None,
            oidc_client_secret: None,
            oidc_issuer_url: None,
            oidc_redirect_uri: None,
        }
    });

    let db = Database::new(&config.database_url).await?;
    let queue_service = Arc::new(readur::ocr::queue::OcrQueueService::new(db.clone(), db.pool.clone(), 1));
    
    Ok(Arc::new(AppState {
        db: db.clone(),
        config,
        webdav_scheduler: None,
        source_scheduler: None,
        queue_service,
        oidc_client: None,
        sync_progress_tracker: std::sync::Arc::new(readur::services::sync_progress_tracker::SyncProgressTracker::new()),
    }))
}

fn create_test_user_with_suffix(suffix: &str) -> CreateUser {
    CreateUser {
        username: format!("testuser_{}", suffix),
        email: format!("test_{}@example.com", suffix),
        password: "test_password".to_string(),
        role: Some(UserRole::User),
    }
}

#[tokio::test]
async fn test_ignored_files_crud_operations() -> Result<()> {
    let state = create_test_app_state().await?;
    let user = create_test_user_with_suffix(&format!("ignored_{}", Uuid::new_v4().simple()));
    
    // Create user in database
    let created_user = state.db.create_user(user).await?;
    let user_id = created_user.id;
    
    // Test creating an ignored file
    let ignored_file = CreateIgnoredFile {
        file_hash: "test_hash_123".to_string(),
        filename: "test_file.pdf".to_string(),
        original_filename: "original_test_file.pdf".to_string(),
        file_path: "/test/path/test_file.pdf".to_string(),
        file_size: 1024,
        mime_type: "application/pdf".to_string(),
        source_type: Some("webdav".to_string()),
        source_path: Some("/webdav/test_file.pdf".to_string()),
        source_identifier: Some("test-webdav".to_string()),
        ignored_by: user_id,
        reason: Some("test deletion".to_string()),
    };
    
    // Test that we can create and retrieve ignored files
    let created = readur::db::ignored_files::create_ignored_file(&state.db.pool, ignored_file).await?;
    assert_eq!(created.file_hash, "test_hash_123");
    assert_eq!(created.filename, "test_file.pdf");
    assert_eq!(created.ignored_by, user_id);
    
    // Test listing ignored files
    let query = readur::models::IgnoredFilesQuery {
        limit: Some(10),
        offset: Some(0),
        source_type: None,
        source_identifier: None,
        ignored_by: None,
        filename: None,
    };
    
    let ignored_files = readur::db::ignored_files::list_ignored_files(&state.db.pool, user_id, &query).await?;
    assert_eq!(ignored_files.len(), 1);
    assert_eq!(ignored_files[0].file_hash, "test_hash_123");
    
    // Test is_file_ignored function
    let is_ignored = readur::db::ignored_files::is_file_ignored(
        &state.db.pool,
        "test_hash_123",
        Some("webdav"),
        Some("/webdav/test_file.pdf")
    ).await?;
    assert!(is_ignored);
    
    // Test deleting ignored file
    let deleted = readur::db::ignored_files::delete_ignored_file(&state.db.pool, created.id, user_id).await?;
    assert!(deleted);
    
    // Verify it's deleted
    let ignored_files_after = readur::db::ignored_files::list_ignored_files(&state.db.pool, user_id, &query).await?;
    assert_eq!(ignored_files_after.len(), 0);
    
    Ok(())
}

#[tokio::test]
async fn test_ignored_files_filtering() -> Result<()> {
    let state = create_test_app_state().await?;
    let user = create_test_user_with_suffix(&format!("filter_{}", Uuid::new_v4().simple()));
    
    // Create user in database
    let created_user = state.db.create_user(user).await?;
    let user_id = created_user.id;
    
    // Create multiple ignored files with different properties
    let ignored_files = vec![
        CreateIgnoredFile {
            file_hash: "hash1".to_string(),
            filename: "webdav_file.pdf".to_string(),
            original_filename: "webdav_file.pdf".to_string(),
            file_path: "/path1/webdav_file.pdf".to_string(),
            file_size: 1024,
            mime_type: "application/pdf".to_string(),
            source_type: Some("webdav".to_string()),
            source_path: Some("/webdav/file.pdf".to_string()),
            source_identifier: Some("webdav-1".to_string()),
            ignored_by: user_id,
            reason: Some("deleted".to_string()),
        },
        CreateIgnoredFile {
            file_hash: "hash2".to_string(),
            filename: "s3_file.pdf".to_string(),
            original_filename: "s3_file.pdf".to_string(),
            file_path: "/path2/s3_file.pdf".to_string(),
            file_size: 2048,
            mime_type: "application/pdf".to_string(),
            source_type: Some("s3".to_string()),
            source_path: Some("/s3/file.pdf".to_string()),
            source_identifier: Some("s3-bucket".to_string()),
            ignored_by: user_id,
            reason: Some("deleted".to_string()),
        }
    ];
    
    // Create ignored files
    for ignored_file in ignored_files {
        readur::db::ignored_files::create_ignored_file(&state.db.pool, ignored_file).await?;
    }
    
    // Test filtering by source type
    let webdav_query = readur::models::IgnoredFilesQuery {
        limit: Some(10),
        offset: Some(0),
        source_type: Some("webdav".to_string()),
        source_identifier: None,
        ignored_by: None,
        filename: None,
    };
    
    let webdav_files = readur::db::ignored_files::list_ignored_files(&state.db.pool, user_id, &webdav_query).await?;
    assert_eq!(webdav_files.len(), 1);
    assert_eq!(webdav_files[0].source_type, Some("webdav".to_string()));
    
    // Test filtering by filename
    let filename_query = readur::models::IgnoredFilesQuery {
        limit: Some(10),
        offset: Some(0),
        source_type: None,
        source_identifier: None,
        ignored_by: None,
        filename: Some("s3_file".to_string()),
    };
    
    let s3_files = readur::db::ignored_files::list_ignored_files(&state.db.pool, user_id, &filename_query).await?;
    assert_eq!(s3_files.len(), 1);
    assert!(s3_files[0].filename.contains("s3_file"));
    
    Ok(())
}

#[tokio::test]
async fn test_ignored_files_user_isolation() -> Result<()> {
    let state = create_test_app_state().await?;
    
    // Create two different users
    let user1 = create_test_user_with_suffix(&format!("user1_{}", Uuid::new_v4().simple()));
    let user2 = create_test_user_with_suffix(&format!("user2_{}", Uuid::new_v4().simple()));
    
    let created_user1 = state.db.create_user(user1).await?;
    let created_user2 = state.db.create_user(user2).await?;
    
    let user1_id = created_user1.id;
    let user2_id = created_user2.id;
    
    // Create ignored file for user1
    let ignored_file1 = CreateIgnoredFile {
        file_hash: "user1_hash".to_string(),
        filename: "user1_file.pdf".to_string(),
        original_filename: "user1_file.pdf".to_string(),
        file_path: "/user1/file.pdf".to_string(),
        file_size: 1024,
        mime_type: "application/pdf".to_string(),
        source_type: Some("webdav".to_string()),
        source_path: Some("/webdav/user1_file.pdf".to_string()),
        source_identifier: Some("webdav-1".to_string()),
        ignored_by: user1_id,
        reason: Some("deleted by user1".to_string()),
    };
    
    // Create ignored file for user2
    let ignored_file2 = CreateIgnoredFile {
        file_hash: "user2_hash".to_string(),
        filename: "user2_file.pdf".to_string(),
        original_filename: "user2_file.pdf".to_string(),
        file_path: "/user2/file.pdf".to_string(),
        file_size: 2048,
        mime_type: "application/pdf".to_string(),
        source_type: Some("s3".to_string()),
        source_path: Some("/s3/user2_file.pdf".to_string()),
        source_identifier: Some("s3-bucket".to_string()),
        ignored_by: user2_id,
        reason: Some("deleted by user2".to_string()),
    };
    
    readur::db::ignored_files::create_ignored_file(&state.db.pool, ignored_file1).await?;
    readur::db::ignored_files::create_ignored_file(&state.db.pool, ignored_file2).await?;
    
    let query = readur::models::IgnoredFilesQuery {
        limit: Some(10),
        offset: Some(0),
        source_type: None,
        source_identifier: None,
        ignored_by: None,
        filename: None,
    };
    
    // User1 should only see their own ignored files
    let user1_files = readur::db::ignored_files::list_ignored_files(&state.db.pool, user1_id, &query).await?;
    assert_eq!(user1_files.len(), 1);
    assert_eq!(user1_files[0].ignored_by, user1_id);
    assert_eq!(user1_files[0].filename, "user1_file.pdf");
    
    // User2 should only see their own ignored files
    let user2_files = readur::db::ignored_files::list_ignored_files(&state.db.pool, user2_id, &query).await?;
    assert_eq!(user2_files.len(), 1);
    assert_eq!(user2_files[0].ignored_by, user2_id);
    assert_eq!(user2_files[0].filename, "user2_file.pdf");
    
    Ok(())
}

#[tokio::test]
async fn test_ignored_files_bulk_operations() -> Result<()> {
    let state = create_test_app_state().await?;
    let user = create_test_user_with_suffix(&format!("bulk_{}", Uuid::new_v4().simple()));
    
    // Create user in database
    let created_user = state.db.create_user(user).await?;
    let user_id = created_user.id;
    
    // Create multiple ignored files
    let mut created_ids = Vec::new();
    for i in 0..3 {
        let ignored_file = CreateIgnoredFile {
            file_hash: format!("bulk_hash_{}", i),
            filename: format!("bulk_file_{}.pdf", i),
            original_filename: format!("bulk_file_{}.pdf", i),
            file_path: format!("/bulk/file_{}.pdf", i),
            file_size: 1024 * (i + 1) as i64,
            mime_type: "application/pdf".to_string(),
            source_type: Some("webdav".to_string()),
            source_path: Some(format!("/webdav/bulk_{}.pdf", i)),
            source_identifier: Some("webdav-bulk".to_string()),
            ignored_by: user_id,
            reason: Some("bulk test".to_string()),
        };
        
        let created = readur::db::ignored_files::create_ignored_file(&state.db.pool, ignored_file).await?;
        created_ids.push(created.id);
    }
    
    // Test bulk delete
    let deleted_count = readur::db::ignored_files::bulk_delete_ignored_files(
        &state.db.pool,
        created_ids.clone(),
        user_id
    ).await?;
    
    assert_eq!(deleted_count, 3);
    
    // Verify all are deleted
    let query = readur::models::IgnoredFilesQuery {
        limit: Some(10),
        offset: Some(0),
        source_type: None,
        source_identifier: None,
        ignored_by: None,
        filename: None,
    };
    
    let remaining_files = readur::db::ignored_files::list_ignored_files(&state.db.pool, user_id, &query).await?;
    assert_eq!(remaining_files.len(), 0);
    
    Ok(())
}

#[tokio::test]
async fn test_create_ignored_file_from_document() -> Result<()> {
    let state = create_test_app_state().await?;
    let user = create_test_user_with_suffix(&format!("doc_{}", Uuid::new_v4().simple()));
    
    // Create user in database
    let created_user = state.db.create_user(user).await?;
    let user_id = created_user.id;
    
    // Create a test document first
    let document = readur::models::Document {
        id: Uuid::new_v4(),
        filename: "test_document.pdf".to_string(),
        original_filename: "test_document.pdf".to_string(),
        file_path: "/uploads/test_document.pdf".to_string(),
        file_size: 1024000,
        mime_type: "application/pdf".to_string(),
        content: Some("Test document content".to_string()),
        ocr_text: Some("OCR text".to_string()),
        ocr_confidence: Some(95.5),
        ocr_word_count: Some(150),
        ocr_processing_time_ms: Some(1200),
        ocr_status: Some("completed".to_string()),
        ocr_error: None,
        ocr_completed_at: Some(chrono::Utc::now()),
        ocr_retry_count: None,
        ocr_failure_reason: None,
        tags: vec!["test".to_string()],
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        user_id,
        file_hash: Some("document_hash_123".to_string()),
        original_created_at: None,
        original_modified_at: None,
        source_path: None,
        source_type: None,
        source_id: None,
        file_permissions: None,
        file_owner: None,
        file_group: None,
        source_metadata: None,
    };
    
    // Insert document into database
    let _document_id = state.db.create_document(document.clone()).await?;
    
    // Test creating ignored file from document
    let ignored_file = readur::db::ignored_files::create_ignored_file_from_document(
        &state.db.pool,
        document.id,
        user_id,
        Some("deleted by user".to_string()),
        Some("webdav".to_string()),
        Some("/webdav/test_document.pdf".to_string()),
        Some("webdav-server".to_string()),
    ).await?;
    
    assert!(ignored_file.is_some());
    let ignored_file = ignored_file.unwrap();
    assert_eq!(ignored_file.filename, document.filename);
    assert_eq!(ignored_file.file_size, document.file_size);
    assert_eq!(ignored_file.mime_type, document.mime_type);
    assert_eq!(ignored_file.ignored_by, user_id);
    assert_eq!(ignored_file.source_type, Some("webdav".to_string()));
    
    Ok(())
}

#[tokio::test]
async fn test_ignored_files_count_functionality() -> Result<()> {
    let state = create_test_app_state().await?;
    let user = create_test_user_with_suffix(&format!("count_{}", Uuid::new_v4().simple()));
    
    // Create user in database
    let created_user = state.db.create_user(user).await?;
    let user_id = created_user.id;
    
    // Initially should have no ignored files
    let query = readur::models::IgnoredFilesQuery {
        limit: Some(10),
        offset: Some(0),
        source_type: None,
        source_identifier: None,
        ignored_by: None,
        filename: None,
    };
    
    let count = readur::db::ignored_files::count_ignored_files(&state.db.pool, user_id, &query).await?;
    assert_eq!(count, 0);
    
    // Create some ignored files
    for i in 0..5 {
        let ignored_file = CreateIgnoredFile {
            file_hash: format!("count_hash_{}", i),
            filename: format!("count_file_{}.pdf", i),
            original_filename: format!("count_file_{}.pdf", i),
            file_path: format!("/count/file_{}.pdf", i),
            file_size: 1024,
            mime_type: "application/pdf".to_string(),
            source_type: Some(if i % 2 == 0 { "webdav" } else { "s3" }.to_string()),
            source_path: Some(format!("/source/file_{}.pdf", i)),
            source_identifier: Some("test-source".to_string()),
            ignored_by: user_id,
            reason: Some("count test".to_string()),
        };
        
        readur::db::ignored_files::create_ignored_file(&state.db.pool, ignored_file).await?;
    }
    
    // Count should now be 5
    let count = readur::db::ignored_files::count_ignored_files(&state.db.pool, user_id, &query).await?;
    assert_eq!(count, 5);
    
    // Test filtered count (only webdav)
    let webdav_query = readur::models::IgnoredFilesQuery {
        limit: Some(10),
        offset: Some(0),
        source_type: Some("webdav".to_string()),
        source_identifier: None,
        ignored_by: None,
        filename: None,
    };
    
    let webdav_count = readur::db::ignored_files::count_ignored_files(&state.db.pool, user_id, &webdav_query).await?;
    assert_eq!(webdav_count, 3); // Files 0, 2, 4 are webdav
    
    Ok(())
}


