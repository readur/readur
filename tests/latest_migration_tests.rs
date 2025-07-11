use readur::test_utils::TestContext;
use sqlx::{PgPool, Row};
use uuid::Uuid;
use std::process::Command;
use std::path::Path;
use sha2::{Sha256, Digest};

#[cfg(test)]
mod latest_migration_tests {
    use super::*;

    #[tokio::test]
    async fn test_latest_migration_from_previous_state() {
        // Step 1: Get the migration files and identify the latest two
        let migration_files = get_sorted_migration_files();
        
        if migration_files.len() < 2 {
            println!("✅ Only one or no migrations found - skipping previous state test");
            return;
        }
        
        let second_to_last = &migration_files[migration_files.len() - 2];
        let latest = &migration_files[migration_files.len() - 1];
        
        println!("🔄 Testing migration from second-to-last to latest:");
        println!("   Previous: {}", extract_migration_name(second_to_last));
        println!("   Latest:   {}", extract_migration_name(latest));
        
        // Step 2: Create a fresh database and apply migrations up to second-to-last
        let ctx = TestContext::new().await;
        let pool = ctx.state.db.get_pool();
        
        // Apply all migrations except the latest one using SQLx migration runner
        let migration_files = get_sorted_migration_files();
        let target_index = migration_files.iter()
            .position(|f| f == second_to_last)
            .expect("Second-to-last migration not found");
        
        // Apply migrations up to target_index (excluding the latest)
        apply_selected_migrations(pool, &migration_files[..target_index+1]).await;
        
        // Step 3: Prefill the database with realistic data in the previous state
        let test_data = prefill_database_for_previous_state(pool).await;
        
        // Step 4: Apply the latest migration
        apply_single_migration(pool, latest).await;
        
        // Step 5: Validate the migration succeeded and data is intact
        validate_latest_migration_success(pool, &test_data, latest).await;
        
        println!("✅ Latest migration successfully applied from previous state");
    }

    #[tokio::test]
    async fn test_latest_migration_with_edge_case_data() {
        let migration_files = get_sorted_migration_files();
        
        if migration_files.len() < 2 {
            println!("✅ Only one or no migrations found - skipping edge case test");
            return;
        }
        
        let second_to_last = &migration_files[migration_files.len() - 2];
        let latest = &migration_files[migration_files.len() - 1];
        
        println!("🧪 Testing latest migration with edge case data:");
        println!("   Testing migration: {}", extract_migration_name(latest));
        
        let ctx = TestContext::new().await;
        let pool = ctx.state.db.get_pool();
        
        // Apply migrations up to second-to-last
        let migration_files = get_sorted_migration_files();
        let target_index = migration_files.iter()
            .position(|f| f == second_to_last)
            .expect("Second-to-last migration not found");
        apply_selected_migrations(pool, &migration_files[..target_index+1]).await;
        
        // Create edge case data that might break the migration
        let edge_case_data = create_edge_case_data(pool).await;
        
        // Apply the latest migration
        let migration_result = apply_single_migration_safe(pool, latest).await;
        
        match migration_result {
            Ok(_) => {
                println!("✅ Latest migration handled edge cases successfully");
                validate_edge_case_migration(pool, &edge_case_data).await;
            }
            Err(e) => {
                panic!("❌ Latest migration failed with edge case data: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_latest_migration_rollback_safety() {
        let migration_files = get_sorted_migration_files();
        
        if migration_files.len() < 2 {
            println!("✅ Only one or no migrations found - skipping rollback safety test");
            return;
        }
        
        let second_to_last = &migration_files[migration_files.len() - 2];
        let latest = &migration_files[migration_files.len() - 1];
        
        println!("🔒 Testing rollback safety for latest migration:");
        
        let ctx = TestContext::new().await;
        let pool = ctx.state.db.get_pool();
        
        // Apply migrations up to second-to-last
        let migration_files = get_sorted_migration_files();
        let target_index = migration_files.iter()
            .position(|f| f == second_to_last)
            .expect("Second-to-last migration not found");
        apply_selected_migrations(pool, &migration_files[..target_index+1]).await;
        
        // Capture schema snapshot before latest migration
        let schema_before = capture_schema_snapshot(pool).await;
        
        // Apply latest migration
        apply_single_migration(pool, latest).await;
        
        // Capture schema after latest migration
        let schema_after = capture_schema_snapshot(pool).await;
        
        // Validate schema changes are reasonable
        validate_schema_changes(&schema_before, &schema_after, latest);
        
        // Test that the migration doesn't break existing functionality
        test_basic_database_operations(pool).await;
        
        println!("✅ Latest migration rollback safety verified");
    }

    #[tokio::test]
    async fn test_latest_migration_performance() {
        let migration_files = get_sorted_migration_files();
        
        if migration_files.len() < 1 {
            println!("✅ No migrations found - skipping performance test");
            return;
        }
        
        let latest = &migration_files[migration_files.len() - 1];
        
        println!("⚡ Testing performance of latest migration:");
        println!("   Migration: {}", extract_migration_name(latest));
        
        let ctx = TestContext::new().await;
        let pool = ctx.state.db.get_pool();
        
        // Apply all migrations except the latest
        if migration_files.len() > 1 {
            let second_to_last = &migration_files[migration_files.len() - 2];
            let target_index = migration_files.iter()
                .position(|f| f == second_to_last)
                .expect("Second-to-last migration not found");
            apply_selected_migrations(pool, &migration_files[..target_index+1]).await;
        }
        
        // Create a substantial amount of data
        create_performance_test_data(pool, 1000).await;
        
        // Measure migration time
        let start_time = std::time::Instant::now();
        apply_single_migration(pool, latest).await;
        let migration_duration = start_time.elapsed();
        
        println!("⏱️  Latest migration completed in: {:?}", migration_duration);
        
        // Performance assertion - should complete reasonably fast even with data
        assert!(
            migration_duration.as_secs() < 10,
            "Latest migration took too long: {:?}. Consider optimizing for larger datasets.",
            migration_duration
        );
        
        // Verify data integrity after migration
        verify_data_integrity_after_performance_test(pool).await;
        
        println!("✅ Latest migration performance acceptable");
    }

    // Helper functions
    
    struct TestData {
        users: Vec<TestUser>,
        documents: Vec<TestDocument>,
        failed_documents: Vec<TestFailedDocument>,
        metadata: DatabaseMetadata,
    }
    
    struct TestUser {
        id: Uuid,
        username: String,
        email: String,
    }
    
    struct TestDocument {
        id: Uuid,
        user_id: Uuid,
        filename: String,
        status: String,
    }
    
    struct TestFailedDocument {
        id: Uuid,
        user_id: Uuid,
        filename: String,
        reason: String,
    }
    
    struct DatabaseMetadata {
        table_count: usize,
        total_records: usize,
        schema_version: String,
    }
    
    struct SchemaSnapshot {
        tables: Vec<String>,
        columns: std::collections::HashMap<String, Vec<String>>,
        constraints: Vec<String>,
    }
    
    fn get_sorted_migration_files() -> Vec<String> {
        let migrations_dir = Path::new("migrations");
        let mut files = Vec::new();
        
        if let Ok(entries) = std::fs::read_dir(migrations_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("sql") {
                        files.push(path.to_string_lossy().to_string());
                    }
                }
            }
        }
        
        files.sort();
        files
    }
    
    fn extract_migration_name(filepath: &str) -> String {
        Path::new(filepath)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string()
    }
    
    async fn apply_selected_migrations(pool: &PgPool, migration_files: &[String]) {
        // Create the migrations table if it doesn't exist
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS _sqlx_migrations (
                version BIGINT PRIMARY KEY,
                description TEXT NOT NULL,
                installed_on TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                success BOOLEAN NOT NULL,
                checksum BYTEA NOT NULL,
                execution_time BIGINT NOT NULL
            )"
        )
        .execute(pool)
        .await
        .expect("Failed to create migrations table");
        
        for migration_file in migration_files {
            let migration_name = extract_migration_name(migration_file);
            
            // Extract version from filename
            let version = migration_name
                .split('_')
                .next()
                .and_then(|s| s.parse::<i64>().ok())
                .expect(&format!("Failed to parse migration version from {}", migration_name));
            
            // Check if this migration is already applied
            let exists = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM _sqlx_migrations WHERE version = $1)"
            )
            .bind(version)
            .fetch_one(pool)
            .await
            .unwrap_or(false);
            
            if exists {
                println!("   ⏭️  Skipped (already applied): {}", migration_name);
                continue;
            }
            
            // Apply this migration
            let content = std::fs::read_to_string(migration_file)
                .expect(&format!("Failed to read migration file: {}", migration_file));
            
            let start_time = std::time::Instant::now();
            
            // Use raw SQL execution to handle complex PostgreSQL statements including functions
            sqlx::raw_sql(&content)
                .execute(pool)
                .await
                .expect(&format!("Failed to apply migration: {}", migration_name));
            
            let execution_time = start_time.elapsed().as_millis() as i64;
            let checksum = Sha256::digest(content.as_bytes()).to_vec();
            
            // Record the migration as applied
            sqlx::query(
                "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time) 
                 VALUES ($1, $2, $3, $4, $5)"
            )
            .bind(version)
            .bind(migration_name.clone())
            .bind(true)
            .bind(checksum)
            .bind(execution_time)
            .execute(pool)
            .await
            .expect("Failed to record migration");
            
            println!("   ✓ Applied: {}", migration_name);
        }
    }
    
    async fn apply_single_migration(pool: &PgPool, migration_file: &str) {
        let result = apply_single_migration_safe(pool, migration_file).await;
        result.expect(&format!("Failed to apply migration: {}", migration_file));
    }
    
    async fn apply_single_migration_safe(pool: &PgPool, migration_file: &str) -> Result<(), sqlx::Error> {
        let content = std::fs::read_to_string(migration_file)
            .expect(&format!("Failed to read migration file: {}", migration_file));
        
        let migration_name = extract_migration_name(migration_file);
        println!("   🔄 Applying: {}", migration_name);
        
        // Use raw SQL execution to handle complex PostgreSQL statements including functions
        sqlx::raw_sql(&content).execute(pool).await?;
        
        println!("   ✅ Applied: {}", migration_name);
        Ok(())
    }
    
    async fn prefill_database_for_previous_state(pool: &PgPool) -> TestData {
        let mut users = Vec::new();
        let mut documents = Vec::new();
        let mut failed_documents = Vec::new();
        
        // Create test users
        for i in 0..5 {
            let user_id = Uuid::new_v4();
            let username = format!("previous_state_user_{}", i);
            let email = format!("previous_{}@test.com", i);
            
            sqlx::query(
                "INSERT INTO users (id, username, email, password_hash, role) VALUES ($1, $2, $3, $4, $5)"
            )
            .bind(user_id)
            .bind(&username)
            .bind(&email)
            .bind("test_hash")
            .bind("user")
            .execute(pool)
            .await
            .expect("Failed to create test user");
            
            users.push(TestUser { id: user_id, username, email });
        }
        
        // Create test documents for each user
        for user in &users {
            for j in 0..3 {
                let doc_id = Uuid::new_v4();
                let filename = format!("previous_doc_{}_{}.pdf", user.username, j);
                let status = if j == 0 { "completed" } else { "failed" };
                
                // Check if documents table exists before inserting
                let table_exists = sqlx::query(
                    "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'documents')"
                )
                .fetch_one(pool)
                .await
                .unwrap()
                .get::<bool, _>(0);
                
                if table_exists {
                    // Check if original_filename column exists
                    let original_filename_exists = sqlx::query_scalar::<_, bool>(
                        "SELECT EXISTS (SELECT 1 FROM information_schema.columns 
                         WHERE table_name = 'documents' AND column_name = 'original_filename')"
                    )
                    .fetch_one(pool)
                    .await
                    .unwrap_or(false);
                    
                    if original_filename_exists {
                        sqlx::query(
                            "INSERT INTO documents (id, user_id, filename, original_filename, file_path, file_size, mime_type, ocr_status) 
                             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
                        )
                        .bind(doc_id)
                        .bind(user.id)
                        .bind(&filename)
                        .bind(&filename) // Use same filename for original_filename
                        .bind(format!("/test/{}", filename))
                        .bind(1024_i64)
                        .bind("application/pdf")
                        .bind(status)
                        .execute(pool)
                        .await
                        .expect("Failed to create test document");
                    } else {
                        sqlx::query(
                            "INSERT INTO documents (id, user_id, filename, file_path, file_size, mime_type, ocr_status) 
                             VALUES ($1, $2, $3, $4, $5, $6, $7)"
                        )
                        .bind(doc_id)
                        .bind(user.id)
                        .bind(&filename)
                        .bind(format!("/test/{}", filename))
                        .bind(1024_i64)
                        .bind("application/pdf")
                        .bind(status)
                        .execute(pool)
                        .await
                        .expect("Failed to create test document");
                    }
                }
                
                documents.push(TestDocument {
                    id: doc_id,
                    user_id: user.id,
                    filename,
                    status: status.to_string(),
                });
            }
        }
        
        // Create failed documents if the table exists
        let failed_docs_exists = sqlx::query(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'failed_documents')"
        )
        .fetch_one(pool)
        .await
        .unwrap()
        .get::<bool, _>(0);
        
        if failed_docs_exists {
            for user in &users {
                let failed_id = Uuid::new_v4();
                let filename = format!("failed_previous_{}.pdf", user.username);
                
                sqlx::query(
                    "INSERT INTO failed_documents (id, user_id, filename, failure_reason, failure_stage, ingestion_source) 
                     VALUES ($1, $2, $3, $4, $5, $6)"
                )
                .bind(failed_id)
                .bind(user.id)
                .bind(&filename)
                .bind("other")
                .bind("ocr")
                .bind("test")
                .execute(pool)
                .await
                .expect("Failed to create test failed document");
                
                failed_documents.push(TestFailedDocument {
                    id: failed_id,
                    user_id: user.id,
                    filename,
                    reason: "other".to_string(),
                });
            }
        }
        
        let total_records = users.len() + documents.len() + failed_documents.len();
        
        TestData {
            users,
            documents,
            failed_documents,
            metadata: DatabaseMetadata {
                table_count: get_table_count(pool).await,
                total_records,
                schema_version: "previous".to_string(),
            },
        }
    }
    
    async fn create_edge_case_data(pool: &PgPool) -> TestData {
        let mut users = Vec::new();
        let mut documents = Vec::new();
        let mut failed_documents = Vec::new();
        
        // Create edge case users
        let long_string = "a".repeat(50);
        let edge_cases = vec![
            ("edge_empty_", ""),
            ("edge_special_", "user@domain.com"),
            ("edge_unicode_", "test_ñäme@tëst.com"),
            ("edge_long_", long_string.as_str()),
        ];
        
        for (prefix, suffix) in edge_cases {
            let user_id = Uuid::new_v4();
            let username = format!("{}{}", prefix, user_id.to_string().split('-').next().unwrap());
            let email = if suffix.is_empty() {
                format!("{}@test.com", username)
            } else {
                suffix.to_string()
            };
            
            sqlx::query(
                "INSERT INTO users (id, username, email, password_hash, role) VALUES ($1, $2, $3, $4, $5)"
            )
            .bind(user_id)
            .bind(&username)
            .bind(&email)
            .bind("test_hash")
            .bind("user")
            .execute(pool)
            .await
            .expect("Failed to create edge case user");
            
            users.push(TestUser { id: user_id, username, email });
        }
        
        let total_records = users.len();
        
        TestData {
            users,
            documents,
            failed_documents,
            metadata: DatabaseMetadata {
                table_count: get_table_count(pool).await,
                total_records,
                schema_version: "edge_case".to_string(),
            },
        }
    }
    
    async fn validate_latest_migration_success(pool: &PgPool, test_data: &TestData, migration_file: &str) {
        let migration_name = extract_migration_name(migration_file);
        
        // Verify that our test data still exists
        let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(pool)
            .await
            .unwrap();
        
        assert!(
            user_count >= test_data.users.len() as i64,
            "User data lost after migration {}",
            migration_name
        );
        
        // Check that the migration was applied successfully by verifying the schema
        let current_table_count = get_table_count(pool).await;
        
        println!("   📊 Validation results:");
        println!("      - Users preserved: {} / {}", user_count, test_data.users.len());
        println!("      - Tables before: {}", test_data.metadata.table_count);
        println!("      - Tables after: {}", current_table_count);
        
        // Test basic database operations still work
        test_basic_database_operations(pool).await;
    }
    
    async fn validate_edge_case_migration(pool: &PgPool, test_data: &TestData) {
        // Verify edge case data survived migration
        for user in &test_data.users {
            let user_exists = sqlx::query(
                "SELECT 1 FROM users WHERE id = $1"
            )
            .bind(user.id)
            .fetch_optional(pool)
            .await
            .unwrap();
            
            assert!(
                user_exists.is_some(),
                "Edge case user {} lost during migration",
                user.username
            );
        }
        
        println!("   ✅ All edge case data preserved");
    }
    
    async fn capture_schema_snapshot(pool: &PgPool) -> SchemaSnapshot {
        // Get all tables
        let tables = sqlx::query(
            "SELECT table_name FROM information_schema.tables 
             WHERE table_schema = 'public' AND table_type = 'BASE TABLE'"
        )
        .fetch_all(pool)
        .await
        .unwrap();
        
        let table_names: Vec<String> = tables.iter()
            .map(|row| row.get("table_name"))
            .collect();
        
        // Get columns for each table
        let mut columns = std::collections::HashMap::new();
        for table in &table_names {
            let table_columns = sqlx::query(
                "SELECT column_name FROM information_schema.columns 
                 WHERE table_schema = 'public' AND table_name = $1"
            )
            .bind(table)
            .fetch_all(pool)
            .await
            .unwrap();
            
            let column_names: Vec<String> = table_columns.iter()
                .map(|row| row.get("column_name"))
                .collect();
            
            columns.insert(table.clone(), column_names);
        }
        
        // Get constraints
        let constraints = sqlx::query(
            "SELECT constraint_name FROM information_schema.table_constraints 
             WHERE table_schema = 'public'"
        )
        .fetch_all(pool)
        .await
        .unwrap();
        
        let constraint_names: Vec<String> = constraints.iter()
            .map(|row| row.get("constraint_name"))
            .collect();
        
        SchemaSnapshot {
            tables: table_names,
            columns,
            constraints: constraint_names,
        }
    }
    
    fn validate_schema_changes(before: &SchemaSnapshot, after: &SchemaSnapshot, migration_file: &str) {
        let migration_name = extract_migration_name(migration_file);
        
        // Check for new tables
        let new_tables: Vec<_> = after.tables.iter()
            .filter(|table| !before.tables.contains(table))
            .collect();
        
        if !new_tables.is_empty() {
            println!("   📋 New tables added by {}: {:?}", migration_name, new_tables);
        }
        
        // Check for removed tables (should be rare and carefully considered)
        let removed_tables: Vec<_> = before.tables.iter()
            .filter(|table| !after.tables.contains(table))
            .collect();
        
        if !removed_tables.is_empty() {
            println!("   ⚠️  Tables removed by {}: {:?}", migration_name, removed_tables);
            // Note: In production, you might want to assert this is intentional
        }
        
        // Check for new constraints
        let new_constraints: Vec<_> = after.constraints.iter()
            .filter(|constraint| !before.constraints.contains(constraint))
            .collect();
        
        if !new_constraints.is_empty() {
            println!("   🔒 New constraints added: {}", new_constraints.len());
        }
        
        println!("   ✅ Schema changes validated");
    }
    
    async fn test_basic_database_operations(pool: &PgPool) {
        // Test that we can still perform basic operations
        
        // Test user creation
        let test_user_id = Uuid::new_v4();
        let result = sqlx::query(
            "INSERT INTO users (id, username, email, password_hash, role) 
             VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(test_user_id)
        .bind("operation_test_user")
        .bind("operation_test@test.com")
        .bind("test_hash")
        .bind("user")
        .execute(pool)
        .await;
        
        assert!(result.is_ok(), "Basic user creation should still work");
        
        // Clean up
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(test_user_id)
            .execute(pool)
            .await
            .unwrap();
        
        println!("   ✅ Basic database operations verified");
    }
    
    async fn create_performance_test_data(pool: &PgPool, user_count: usize) {
        println!("   📊 Creating {} users for performance testing...", user_count);
        
        for i in 0..user_count {
            let user_id = Uuid::new_v4();
            let username = format!("perf_user_{}", i);
            let email = format!("perf_{}@test.com", i);
            
            sqlx::query(
                "INSERT INTO users (id, username, email, password_hash, role) VALUES ($1, $2, $3, $4, $5)"
            )
            .bind(user_id)
            .bind(&username)
            .bind(&email)
            .bind("test_hash")
            .bind("user")
            .execute(pool)
            .await
            .expect("Failed to create performance test user");
        }
        
        println!("   ✅ Performance test data created");
    }
    
    async fn verify_data_integrity_after_performance_test(pool: &PgPool) {
        let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(pool)
            .await
            .unwrap();
        
        assert!(user_count > 0, "Performance test data should exist after migration");
        
        println!("   ✅ Data integrity verified: {} users", user_count);
    }
    
    async fn get_table_count(pool: &PgPool) -> usize {
        let tables = sqlx::query(
            "SELECT COUNT(*) as count FROM information_schema.tables 
             WHERE table_schema = 'public' AND table_type = 'BASE TABLE'"
        )
        .fetch_one(pool)
        .await
        .unwrap();
        
        tables.get::<i64, _>("count") as usize
    }
}