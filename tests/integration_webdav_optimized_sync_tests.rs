use readur::models::{FileInfo, CreateWebDAVDirectory, UpdateWebDAVDirectory, User, UserRole, AuthProvider};
use readur::{AppState};
use tokio;
use chrono::Utc;
use uuid::Uuid;
use std::sync::Arc;
use std::collections::HashMap;

// Test utilities for mocking WebDAV responses
struct MockWebDAVServer {
    directory_etags: HashMap<String, String>,
    directory_files: HashMap<String, Vec<FileInfo>>,
    request_count: std::sync::atomic::AtomicUsize,
}

impl MockWebDAVServer {
    fn new() -> Self {
        Self {
            directory_etags: HashMap::new(),
            directory_files: HashMap::new(),
            request_count: std::sync::atomic::AtomicUsize::new(0),
        }
    }
    
    fn set_directory_etag(&mut self, path: &str, etag: &str) {
        self.directory_etags.insert(path.to_string(), etag.to_string());
    }
    
    fn set_directory_files(&mut self, path: &str, files: Vec<FileInfo>) {
        self.directory_files.insert(path.to_string(), files);
    }
    
    fn get_request_count(&self) -> usize {
        self.request_count.load(std::sync::atomic::Ordering::SeqCst)
    }
    
    fn increment_request_count(&self) {
        self.request_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

// Helper function to setup test database
async fn setup_test_database() -> readur::db::Database {
    let db_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "sqlite::memory:".to_string());
    
    let db = readur::db::Database::new(&db_url).await.expect("Failed to create test database");
    
    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&db.pool)
        .await
        .expect("Failed to run migrations");
    
    db
}

// Helper function to create test user
async fn create_test_user(db: &readur::db::Database) -> Uuid {
    let user_id = Uuid::new_v4();
    let user = User {
        id: user_id,
        username: "testuser".to_string(),
        email: "test@example.com".to_string(),
        password_hash: Some("test_hash".to_string()),
        role: UserRole::User,
        auth_provider: AuthProvider::Local,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        oidc_subject: None,
        oidc_issuer: None,
        oidc_email: None,
    };
    
    // Insert user into database
    sqlx::query!(
        "INSERT INTO users (id, username, email, password_hash, role, auth_provider, created_at, updated_at, oidc_subject, oidc_issuer, oidc_email) 
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        user.id,
        user.username,
        user.email,
        user.password_hash,
        user.role.to_string(),
        user.auth_provider.to_string(),
        user.created_at,
        user.updated_at,
        user.oidc_subject,
        user.oidc_issuer,
        user.oidc_email
    )
    .execute(&db.pool)
    .await
    .expect("Failed to insert test user");
    
    user_id
}

// Helper function to create AppState for testing
async fn create_test_app_state() -> Arc<AppState> {
    let db = setup_test_database().await;
    let config = readur::config::Config {
        database_url: "sqlite::memory:".to_string(),
        upload_path: "/tmp/test_uploads".to_string(),
        jwt_secret: "test_secret".to_string(),
        server_host: "127.0.0.1".to_string(),
        server_port: 8080,
        log_level: "info".to_string(),
        ..Default::default()
    };
    
    Arc::new(AppState {
        db,
        config,
        queue_service: std::sync::Arc::new(readur::ocr::queue::OcrQueueService::new(std::sync::Arc::new(readur::db::Database::new("sqlite::memory:").await.unwrap()))),
        webdav_scheduler: None,
        source_scheduler: None,
        oidc_client: None,
    })
}

fn create_sample_files_with_directories() -> Vec<FileInfo> {
    vec![
        // Root directory
        FileInfo {
            path: "/Documents".to_string(),
            name: "Documents".to_string(),
            size: 0,
            mime_type: "".to_string(),
            last_modified: Some(Utc::now()),
            etag: "documents-etag-v1".to_string(),
            is_directory: true,
            created_at: Some(Utc::now()),
            permissions: Some(755),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        // Subdirectory
        FileInfo {
            path: "/Documents/Projects".to_string(),
            name: "Projects".to_string(),
            size: 0,
            mime_type: "".to_string(),
            last_modified: Some(Utc::now()),
            etag: "projects-etag-v1".to_string(),
            is_directory: true,
            created_at: Some(Utc::now()),
            permissions: Some(755),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        // Files
        FileInfo {
            path: "/Documents/readme.pdf".to_string(),
            name: "readme.pdf".to_string(),
            size: 1024000,
            mime_type: "application/pdf".to_string(),
            last_modified: Some(Utc::now()),
            etag: "readme-etag-v1".to_string(),
            is_directory: false,
            created_at: Some(Utc::now()),
            permissions: Some(644),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        FileInfo {
            path: "/Documents/Projects/project1.pdf".to_string(),
            name: "project1.pdf".to_string(),
            size: 2048000,
            mime_type: "application/pdf".to_string(),
            last_modified: Some(Utc::now()),
            etag: "project1-etag-v1".to_string(),
            is_directory: false,
            created_at: Some(Utc::now()),
            permissions: Some(644),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
    ]
}

#[tokio::test]
async fn test_directory_tracking_database_operations() {
    let state = create_test_app_state().await;
    let user_id = create_test_user(&state.db).await;
    
    // Test creating directory record
    let create_dir = CreateWebDAVDirectory {
        user_id,
        directory_path: "/Documents".to_string(),
        directory_etag: "test-etag-123".to_string(),
        file_count: 5,
        total_size_bytes: 1024000,
    };
    
    let created_dir = state.db.create_or_update_webdav_directory(&create_dir)
        .await
        .expect("Failed to create directory record");
    
    assert_eq!(created_dir.directory_path, "/Documents");
    assert_eq!(created_dir.directory_etag, "test-etag-123");
    assert_eq!(created_dir.file_count, 5);
    assert_eq!(created_dir.total_size_bytes, 1024000);
    
    // Test retrieving directory record
    let retrieved_dir = state.db.get_webdav_directory(user_id, "/Documents")
        .await
        .expect("Failed to retrieve directory")
        .expect("Directory not found");
    
    assert_eq!(retrieved_dir.directory_etag, "test-etag-123");
    assert_eq!(retrieved_dir.file_count, 5);
    
    // Test updating directory record
    let update_dir = UpdateWebDAVDirectory {
        directory_etag: "updated-etag-456".to_string(),
        last_scanned_at: Utc::now(),
        file_count: 7,
        total_size_bytes: 2048000,
    };
    
    state.db.update_webdav_directory(user_id, "/Documents", &update_dir)
        .await
        .expect("Failed to update directory");
    
    // Verify update
    let updated_dir = state.db.get_webdav_directory(user_id, "/Documents")
        .await
        .expect("Failed to retrieve updated directory")
        .expect("Directory not found after update");
    
    assert_eq!(updated_dir.directory_etag, "updated-etag-456");
    assert_eq!(updated_dir.file_count, 7);
    assert_eq!(updated_dir.total_size_bytes, 2048000);
}

#[tokio::test]
async fn test_multiple_directory_tracking() {
    let state = create_test_app_state().await;
    let user_id = create_test_user(&state.db).await;
    
    // Create multiple directory records
    let directories = vec![
        CreateWebDAVDirectory {
            user_id,
            directory_path: "/Documents".to_string(),
            directory_etag: "docs-etag".to_string(),
            file_count: 3,
            total_size_bytes: 1024000,
        },
        CreateWebDAVDirectory {
            user_id,
            directory_path: "/Documents/Projects".to_string(),
            directory_etag: "projects-etag".to_string(),
            file_count: 2,
            total_size_bytes: 2048000,
        },
        CreateWebDAVDirectory {
            user_id,
            directory_path: "/Documents/Archive".to_string(),
            directory_etag: "archive-etag".to_string(),
            file_count: 10,
            total_size_bytes: 5120000,
        },
    ];
    
    for dir in directories {
        state.db.create_or_update_webdav_directory(&dir)
            .await
            .expect("Failed to create directory");
    }
    
    // List all directories
    let all_dirs = state.db.list_webdav_directories(user_id)
        .await
        .expect("Failed to list directories");
    
    assert_eq!(all_dirs.len(), 3);
    
    // Verify they're sorted by path
    assert_eq!(all_dirs[0].directory_path, "/Documents");
    assert_eq!(all_dirs[1].directory_path, "/Documents/Archive");
    assert_eq!(all_dirs[2].directory_path, "/Documents/Projects");
}

#[tokio::test]
async fn test_directory_isolation_between_users() {
    let state = create_test_app_state().await;
    let user1_id = create_test_user(&state.db).await;
    
    // Create second user
    let user2_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, username, email, password_hash, role, auth_provider, created_at, updated_at, oidc_subject, oidc_issuer, oidc_email) 
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        user2_id,
        "testuser2",
        "test2@example.com",
        Some("test_hash2".to_string()),
        UserRole::User.to_string(),
        AuthProvider::Local.to_string(),
        Utc::now(),
        Utc::now(),
        None::<String>,
        None::<String>,
        None::<String>
    )
    .execute(&state.db.pool)
    .await
    .expect("Failed to insert second test user");
    
    // Create directory for user1
    let dir1 = CreateWebDAVDirectory {
        user_id: user1_id,
        directory_path: "/Documents".to_string(),
        directory_etag: "user1-etag".to_string(),
        file_count: 5,
        total_size_bytes: 1024000,
    };
    
    state.db.create_or_update_webdav_directory(&dir1)
        .await
        .expect("Failed to create directory for user1");
    
    // Create directory for user2
    let dir2 = CreateWebDAVDirectory {
        user_id: user2_id,
        directory_path: "/Documents".to_string(),
        directory_etag: "user2-etag".to_string(),
        file_count: 3,
        total_size_bytes: 512000,
    };
    
    state.db.create_or_update_webdav_directory(&dir2)
        .await
        .expect("Failed to create directory for user2");
    
    // Verify user1 can only see their directory
    let user1_dirs = state.db.list_webdav_directories(user1_id)
        .await
        .expect("Failed to list user1 directories");
    
    assert_eq!(user1_dirs.len(), 1);
    assert_eq!(user1_dirs[0].directory_etag, "user1-etag");
    
    // Verify user2 can only see their directory
    let user2_dirs = state.db.list_webdav_directories(user2_id)
        .await
        .expect("Failed to list user2 directories");
    
    assert_eq!(user2_dirs.len(), 1);
    assert_eq!(user2_dirs[0].directory_etag, "user2-etag");
    
    // Verify user1 cannot access user2's directory
    let user1_access_user2 = state.db.get_webdav_directory(user1_id, "/Documents")
        .await
        .expect("Database query failed");
    
    assert!(user1_access_user2.is_some());
    assert_eq!(user1_access_user2.unwrap().directory_etag, "user1-etag");
}

#[tokio::test]
async fn test_etag_change_detection() {
    let state = create_test_app_state().await;
    let user_id = create_test_user(&state.db).await;
    
    // Create initial directory
    let initial_dir = CreateWebDAVDirectory {
        user_id,
        directory_path: "/Documents".to_string(),
        directory_etag: "initial-etag".to_string(),
        file_count: 3,
        total_size_bytes: 1024000,
    };
    
    state.db.create_or_update_webdav_directory(&initial_dir)
        .await
        .expect("Failed to create initial directory");
    
    // Simulate checking current directory ETag
    let stored_dir = state.db.get_webdav_directory(user_id, "/Documents")
        .await
        .expect("Failed to get directory")
        .expect("Directory not found");
    
    // Simulate server returning different ETag (directory changed)
    let current_etag = "changed-etag";
    let directory_changed = stored_dir.directory_etag != current_etag;
    
    assert!(directory_changed, "Directory should be detected as changed");
    
    // Update with new ETag after processing changes
    let update = UpdateWebDAVDirectory {
        directory_etag: current_etag.to_string(),
        last_scanned_at: Utc::now(),
        file_count: 5, // Files were added
        total_size_bytes: 2048000, // Size increased
    };
    
    state.db.update_webdav_directory(user_id, "/Documents", &update)
        .await
        .expect("Failed to update directory");
    
    // Verify update
    let updated_dir = state.db.get_webdav_directory(user_id, "/Documents")
        .await
        .expect("Failed to get updated directory")
        .expect("Directory not found");
    
    assert_eq!(updated_dir.directory_etag, "changed-etag");
    assert_eq!(updated_dir.file_count, 5);
    assert_eq!(updated_dir.total_size_bytes, 2048000);
    
    // Simulate next sync with same ETag (no changes)
    let same_etag = "changed-etag";
    let directory_unchanged = updated_dir.directory_etag == same_etag;
    
    assert!(directory_unchanged, "Directory should be detected as unchanged");
}

#[tokio::test]
async fn test_subdirectory_filtering() {
    let state = create_test_app_state().await;
    let user_id = create_test_user(&state.db).await;
    
    // Create nested directory structure
    let directories = vec![
        ("/Documents", "docs-etag"),
        ("/Documents/2024", "2024-etag"),
        ("/Documents/2024/Q1", "q1-etag"),
        ("/Documents/2024/Q2", "q2-etag"),
        ("/Documents/Archive", "archive-etag"),
        ("/Other", "other-etag"), // Different root
    ];
    
    for (path, etag) in directories {
        let dir = CreateWebDAVDirectory {
            user_id,
            directory_path: path.to_string(),
            directory_etag: etag.to_string(),
            file_count: 1,
            total_size_bytes: 1024,
        };
        
        state.db.create_or_update_webdav_directory(&dir)
            .await
            .expect("Failed to create directory");
    }
    
    // Get all directories and filter subdirectories of /Documents
    let all_dirs = state.db.list_webdav_directories(user_id)
        .await
        .expect("Failed to list directories");
    
    let documents_subdirs: Vec<_> = all_dirs.iter()
        .filter(|dir| dir.directory_path.starts_with("/Documents") && dir.directory_path != "/Documents")
        .collect();
    
    assert_eq!(documents_subdirs.len(), 4); // 2024, Q1, Q2, Archive
    
    // Verify specific subdirectories
    let subdir_paths: Vec<&str> = documents_subdirs.iter()
        .map(|dir| dir.directory_path.as_str())
        .collect();
    
    assert!(subdir_paths.contains(&"/Documents/2024"));
    assert!(subdir_paths.contains(&"/Documents/2024/Q1"));
    assert!(subdir_paths.contains(&"/Documents/2024/Q2"));
    assert!(subdir_paths.contains(&"/Documents/Archive"));
    assert!(!subdir_paths.contains(&"/Other")); // Should not include different root
}

#[tokio::test]
async fn test_performance_metrics() {
    let state = create_test_app_state().await;
    let user_id = create_test_user(&state.db).await;
    
    // Create a large number of directories to test performance
    let start_time = std::time::Instant::now();
    
    for i in 0..100 {
        let dir = CreateWebDAVDirectory {
            user_id,
            directory_path: format!("/Documents/Dir{:03}", i),
            directory_etag: format!("etag-{}", i),
            file_count: i as i64,
            total_size_bytes: (i * 1024) as i64,
        };
        
        state.db.create_or_update_webdav_directory(&dir)
            .await
            .expect("Failed to create directory");
    }
    
    let create_time = start_time.elapsed();
    println!("Created 100 directories in: {:?}", create_time);
    
    // Test bulk retrieval performance
    let retrieval_start = std::time::Instant::now();
    let all_dirs = state.db.list_webdav_directories(user_id)
        .await
        .expect("Failed to list directories");
    let retrieval_time = retrieval_start.elapsed();
    
    println!("Retrieved {} directories in: {:?}", all_dirs.len(), retrieval_time);
    assert_eq!(all_dirs.len(), 100);
    
    // Test individual directory access performance
    let individual_start = std::time::Instant::now();
    for i in 0..10 {
        let path = format!("/Documents/Dir{:03}", i);
        let dir = state.db.get_webdav_directory(user_id, &path)
            .await
            .expect("Failed to get directory")
            .expect("Directory not found");
        assert_eq!(dir.directory_etag, format!("etag-{}", i));
    }
    let individual_time = individual_start.elapsed();
    
    println!("Retrieved 10 individual directories in: {:?}", individual_time);
    
    // Performance assertions (adjust these based on acceptable performance)
    assert!(create_time.as_millis() < 5000, "Directory creation too slow: {:?}", create_time);
    assert!(retrieval_time.as_millis() < 100, "Directory retrieval too slow: {:?}", retrieval_time);
    assert!(individual_time.as_millis() < 100, "Individual directory access too slow: {:?}", individual_time);
}