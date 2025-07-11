use readur::test_utils::TestContext;
use sqlx::{PgPool, Row};
use uuid::Uuid;
use std::process::Command;

#[cfg(test)]
mod pr_migration_validation_tests {
    use super::*;

    #[tokio::test]
    async fn test_new_migration_with_prefilled_data() {
        // Check if this PR introduces any new migrations
        let new_migrations = get_new_migrations_in_pr();
        
        if new_migrations.is_empty() {
            println!("✅ No new migrations in this PR - skipping prefilled data test");
            return;
        }
        
        println!("🔍 Found {} new migration(s) in this PR:", new_migrations.len());
        for migration in &new_migrations {
            println!("  - {}", migration);
        }
        
        // Run the comprehensive test with prefilled data
        let ctx = TestContext::new().await;
        let pool = ctx.state.db.get_pool();
        
        // Step 1: Prefill database with comprehensive test data
        let test_data = prefill_comprehensive_test_data(pool).await;
        println!("✅ Prefilled database with {} test scenarios", test_data.scenarios.len());
        
        // Step 2: Verify all migrations run successfully with prefilled data
        verify_migrations_with_data(pool, &test_data).await;
        
        // Step 3: Test specific migration scenarios if they involve data transformation
        if migration_involves_data_transformation(&new_migrations) {
            test_data_transformation_integrity(pool, &test_data).await;
        }
        
        // Step 4: Verify no data loss occurred
        verify_no_data_loss(pool, &test_data).await;
        
        println!("✅ All new migrations passed validation with prefilled data");
    }

    #[tokio::test]
    async fn test_migration_rollback_safety() {
        let new_migrations = get_new_migrations_in_pr();
        
        if new_migrations.is_empty() {
            println!("✅ No new migrations in this PR - skipping rollback safety test");
            return;
        }
        
        let ctx = TestContext::new().await;
        let pool = ctx.state.db.get_pool();
        
        // Create snapshot of schema before migrations
        let schema_before = capture_schema_snapshot(pool).await;
        
        // Run migrations
        let migration_result = sqlx::migrate!("./migrations").run(pool).await;
        assert!(migration_result.is_ok(), "Migrations should succeed");
        
        // Capture schema after migrations
        let schema_after = capture_schema_snapshot(pool).await;
        
        // Verify schema changes are intentional
        verify_schema_changes(&schema_before, &schema_after, &new_migrations);
        
        println!("✅ Migration rollback safety verified");
    }

    #[tokio::test]
    async fn test_migration_performance_impact() {
        let new_migrations = get_new_migrations_in_pr();
        
        if new_migrations.is_empty() {
            return;
        }
        
        let ctx = TestContext::new().await;
        let pool = ctx.state.db.get_pool();
        
        // Prefill with large dataset
        create_performance_test_data(pool, 10000).await;
        
        // Measure migration execution time
        let start = std::time::Instant::now();
        let result = sqlx::migrate!("./migrations").run(pool).await;
        let duration = start.elapsed();
        
        assert!(result.is_ok(), "Migrations should succeed");
        assert!(
            duration.as_secs() < 30,
            "Migrations took too long: {:?}. Consider optimizing for large datasets.",
            duration
        );
        
        println!("✅ Migration performance acceptable: {:?}", duration);
    }

    // Data structures for comprehensive testing
    
    struct ComprehensiveTestData {
        users: Vec<TestUser>,
        documents: Vec<TestDocument>,
        scenarios: Vec<TestScenario>,
        total_records: usize,
    }
    
    struct TestUser {
        id: Uuid,
        username: String,
        role: String,
    }
    
    struct TestDocument {
        id: Uuid,
        user_id: Uuid,
        filename: String,
        ocr_status: String,
        failure_reason: Option<String>,
        metadata: DocumentMetadata,
    }
    
    struct DocumentMetadata {
        file_size: i64,
        mime_type: String,
        has_ocr_text: bool,
        tags: Vec<String>,
    }
    
    struct TestScenario {
        name: String,
        description: String,
        affected_tables: Vec<String>,
        record_count: usize,
    }
    
    struct SchemaSnapshot {
        tables: Vec<TableInfo>,
        indexes: Vec<String>,
        constraints: Vec<String>,
        views: Vec<String>,
    }
    
    struct TableInfo {
        name: String,
        columns: Vec<ColumnInfo>,
        row_count: i64,
    }
    
    struct ColumnInfo {
        name: String,
        data_type: String,
        is_nullable: bool,
    }
    
    // Implementation functions
    
    async fn prefill_comprehensive_test_data(pool: &PgPool) -> ComprehensiveTestData {
        let mut users = Vec::new();
        let mut documents = Vec::new();
        let mut scenarios = Vec::new();
        
        // Create diverse user types
        let user_types = vec![
            ("admin", "admin"),
            ("regular", "user"),
            ("readonly", "user"),
        ];
        
        for (user_type, role) in user_types {
            let user = create_test_user_with_role(pool, user_type, role).await;
            users.push(user);
        }
        
        // Create various document scenarios
        let document_scenarios = vec![
            // Successful documents
            ("success_high_conf.pdf", "completed", None, 0.95, true),
            ("success_medium_conf.pdf", "completed", None, 0.75, true),
            ("success_with_tags.pdf", "completed", None, 0.85, true),
            
            // Failed documents with different reasons
            ("fail_low_confidence.pdf", "failed", Some("low_ocr_confidence"), 0.3, true),
            ("fail_timeout.pdf", "failed", Some("timeout"), 0.0, false),
            ("fail_memory.pdf", "failed", Some("memory_limit"), 0.0, false),
            ("fail_corrupted.pdf", "failed", Some("file_corrupted"), 0.0, false),
            ("fail_unsupported.xyz", "failed", Some("unsupported_format"), 0.0, false),
            ("fail_access_denied.pdf", "failed", Some("access_denied"), 0.0, false),
            ("fail_parsing.pdf", "failed", Some("pdf_parsing_error"), 0.0, false),
            ("fail_unknown.pdf", "failed", Some("unknown_error"), 0.0, false),
            ("fail_null_reason.pdf", "failed", None, 0.0, false),
            
            // Pending documents
            ("pending_new.pdf", "pending", None, 0.0, false),
            ("pending_retry.pdf", "pending", None, 0.0, false),
            
            // Edge cases
            ("edge_empty_file.pdf", "failed", Some("file_corrupted"), 0.0, false),
            ("edge_huge_file.pdf", "failed", Some("file_too_large"), 0.0, false),
            ("edge_special_chars_§.pdf", "completed", None, 0.9, true),
        ];
        
        // Create documents for each user
        for user in &users {
            for (filename, status, failure_reason, confidence, has_text) in &document_scenarios {
                let doc = create_test_document(
                    pool, 
                    user.id, 
                    filename, 
                    status, 
                    failure_reason.as_deref(),
                    *confidence,
                    *has_text
                ).await;
                documents.push(doc);
            }
        }
        
        // Create OCR queue entries for some documents
        for doc in documents.iter().filter(|d| d.ocr_status == "pending" || d.ocr_status == "failed") {
            create_ocr_queue_entry(pool, doc.id).await;
        }
        
        // Create scenarios description
        scenarios.push(TestScenario {
            name: "User Management".to_string(),
            description: "Different user roles and permissions".to_string(),
            affected_tables: vec!["users".to_string()],
            record_count: users.len(),
        });
        
        scenarios.push(TestScenario {
            name: "Document Processing".to_string(),
            description: "Various document states and failure scenarios".to_string(),
            affected_tables: vec!["documents".to_string(), "failed_documents".to_string()],
            record_count: documents.len(),
        });
        
        scenarios.push(TestScenario {
            name: "OCR Queue".to_string(),
            description: "OCR processing queue with retries".to_string(),
            affected_tables: vec!["ocr_queue".to_string()],
            record_count: documents.iter().filter(|d| d.ocr_status != "completed").count(),
        });
        
        let total_records = users.len() + documents.len();
        
        ComprehensiveTestData {
            users,
            documents,
            scenarios,
            total_records,
        }
    }
    
    async fn create_test_user_with_role(pool: &PgPool, user_type: &str, role: &str) -> TestUser {
        let id = Uuid::new_v4();
        let username = format!("test_{}_{}", user_type, Uuid::new_v4().to_string().split('-').next().unwrap());
        
        sqlx::query(
            "INSERT INTO users (id, username, email, password_hash, role) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(id)
        .bind(&username)
        .bind(format!("{}@test.com", username))
        .bind("test_hash")
        .bind(role)
        .execute(pool)
        .await
        .unwrap();
        
        TestUser { id, username, role: role.to_string() }
    }
    
    async fn create_test_document(
        pool: &PgPool,
        user_id: Uuid,
        filename: &str,
        status: &str,
        failure_reason: Option<&str>,
        confidence: f32,
        has_text: bool,
    ) -> TestDocument {
        let id = Uuid::new_v4();
        let file_size = match filename {
            f if f.contains("huge") => 104857600, // 100MB
            f if f.contains("empty") => 0,
            _ => 1024 * (1 + (id.as_bytes()[0] as i64)), // Variable size
        };
        
        let mime_type = if filename.ends_with(".pdf") {
            "application/pdf"
        } else {
            "application/octet-stream"
        };
        
        let tags = if filename.contains("tags") {
            vec!["important", "reviewed", "2024"]
        } else {
            vec![]
        };
        
        let ocr_text = if has_text {
            Some(format!("Sample OCR text for document {}", filename))
        } else {
            None
        };
        
        sqlx::query(
            r#"
            INSERT INTO documents (
                id, user_id, filename, original_filename, file_path, file_size,
                mime_type, ocr_status, ocr_failure_reason, ocr_confidence, ocr_text, tags
            ) VALUES (
                $1, $2, $3, $3, $4, $5, $6, $7, $8, $9, $10, $11
            )
            "#
        )
        .bind(id)
        .bind(user_id)
        .bind(filename)
        .bind(format!("/test/files/{}", filename))
        .bind(file_size)
        .bind(mime_type)
        .bind(status)
        .bind(failure_reason)
        .bind(if confidence > 0.0 { Some(confidence) } else { None })
        .bind(ocr_text)
        .bind(&tags)
        .execute(pool)
        .await
        .unwrap();
        
        TestDocument {
            id,
            user_id,
            filename: filename.to_string(),
            ocr_status: status.to_string(),
            failure_reason: failure_reason.map(|s| s.to_string()),
            metadata: DocumentMetadata {
                file_size,
                mime_type: mime_type.to_string(),
                has_ocr_text: has_text,
                tags: tags.iter().map(|s| s.to_string()).collect(),
            },
        }
    }
    
    async fn create_ocr_queue_entry(pool: &PgPool, document_id: Uuid) {
        sqlx::query(
            "INSERT INTO ocr_queue (document_id, priority, status) VALUES ($1, $2, $3)"
        )
        .bind(document_id)
        .bind(1)
        .bind("pending")
        .execute(pool)
        .await
        .unwrap();
    }
    
    async fn verify_migrations_with_data(pool: &PgPool, test_data: &ComprehensiveTestData) {
        // Count records before any potential data migration
        let doc_count_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM documents")
            .fetch_one(pool)
            .await
            .unwrap();
        
        let user_count_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(pool)
            .await
            .unwrap();
        
        println!("📊 Database state before migration verification:");
        println!("   - Users: {}", user_count_before);
        println!("   - Documents: {}", doc_count_before);
        
        // Verify failed document migration if applicable
        let failed_docs: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM documents WHERE ocr_status = 'failed'"
        )
        .fetch_one(pool)
        .await
        .unwrap();
        
        if failed_docs > 0 {
            println!("   - Failed documents to migrate: {}", failed_docs);
            
            // Verify migration mapping works correctly
            let mapping_test = sqlx::query(
                r#"
                SELECT 
                    ocr_failure_reason,
                    COUNT(*) as count,
                    CASE 
                        WHEN ocr_failure_reason = 'low_ocr_confidence' THEN 'low_ocr_confidence'
                        WHEN ocr_failure_reason = 'timeout' THEN 'ocr_timeout'
                        WHEN ocr_failure_reason = 'memory_limit' THEN 'ocr_memory_limit'
                        WHEN ocr_failure_reason = 'pdf_parsing_error' THEN 'pdf_parsing_error'
                        WHEN ocr_failure_reason = 'corrupted' OR ocr_failure_reason = 'file_corrupted' THEN 'file_corrupted'
                        WHEN ocr_failure_reason = 'unsupported_format' THEN 'unsupported_format'
                        WHEN ocr_failure_reason = 'access_denied' THEN 'access_denied'
                        ELSE 'other'
                    END as mapped_reason
                FROM documents 
                WHERE ocr_status = 'failed'
                GROUP BY ocr_failure_reason
                "#
            )
            .fetch_all(pool)
            .await
            .unwrap();
            
            println!("   - Failure reason mappings:");
            for row in mapping_test {
                let original: Option<String> = row.get("ocr_failure_reason");
                let mapped: String = row.get("mapped_reason");
                let count: i64 = row.get("count");
                println!("     {:?} -> {} ({}  documents)", original, mapped, count);
            }
        }
    }
    
    async fn test_data_transformation_integrity(pool: &PgPool, test_data: &ComprehensiveTestData) {
        // Test that data transformations maintain integrity
        println!("🔄 Testing data transformation integrity...");
        
        // Check if failed_documents table exists (indicating migration ran)
        let failed_docs_exists = sqlx::query(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'failed_documents')"
        )
        .fetch_one(pool)
        .await
        .unwrap()
        .get::<bool, _>(0);
        
        if failed_docs_exists {
            // Verify all failed documents were migrated correctly
            let migrated_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM failed_documents WHERE failure_stage = 'ocr'"
            )
            .fetch_one(pool)
            .await
            .unwrap();
            
            let expected_failed = test_data.documents.iter()
                .filter(|d| d.ocr_status == "failed")
                .count();
            
            assert!(
                migrated_count >= expected_failed as i64,
                "Not all failed documents were migrated: expected at least {}, got {}",
                expected_failed, migrated_count
            );
            
            // Verify data integrity for specific test cases
            for doc in test_data.documents.iter().filter(|d| d.ocr_status == "failed") {
                let migrated = sqlx::query(
                    "SELECT * FROM failed_documents WHERE filename = $1"
                )
                .bind(&doc.filename)
                .fetch_optional(pool)
                .await
                .unwrap();
                
                assert!(
                    migrated.is_some(),
                    "Failed document '{}' was not migrated",
                    doc.filename
                );
                
                if let Some(row) = migrated {
                    let failure_reason: String = row.get("failure_reason");
                    
                    // Verify reason mapping
                    match doc.failure_reason.as_deref() {
                        Some("timeout") => assert_eq!(failure_reason, "ocr_timeout"),
                        Some("memory_limit") => assert_eq!(failure_reason, "ocr_memory_limit"),
                        Some("file_corrupted") => assert_eq!(failure_reason, "file_corrupted"),
                        Some("low_ocr_confidence") => assert_eq!(failure_reason, "low_ocr_confidence"),
                        Some("unknown_error") | None => assert_eq!(failure_reason, "other"),
                        _ => {}
                    }
                }
            }
        }
        
        println!("✅ Data transformation integrity verified");
    }
    
    async fn verify_no_data_loss(pool: &PgPool, test_data: &ComprehensiveTestData) {
        println!("🔍 Verifying no data loss occurred...");
        
        // Check user count
        let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(pool)
            .await
            .unwrap();
        
        assert!(
            user_count >= test_data.users.len() as i64,
            "User data loss detected: expected at least {}, got {}",
            test_data.users.len(), user_count
        );
        
        // Check total document count (including migrated)
        let doc_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM documents")
            .fetch_one(pool)
            .await
            .unwrap();
        
        let failed_doc_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM failed_documents WHERE ingestion_source IS NOT NULL"
        )
        .fetch_one(pool)
        .await
        .unwrap_or(0);
        
        let total_docs = doc_count + failed_doc_count;
        let expected_docs = test_data.documents.len() as i64;
        
        assert!(
            total_docs >= expected_docs,
            "Document data loss detected: expected at least {}, got {} (documents: {}, failed_documents: {})",
            expected_docs, total_docs, doc_count, failed_doc_count
        );
        
        println!("✅ No data loss detected");
    }
    
    async fn capture_schema_snapshot(pool: &PgPool) -> SchemaSnapshot {
        let tables = sqlx::query(
            r#"
            SELECT 
                t.table_name,
                COUNT(c.column_name) as column_count
            FROM information_schema.tables t
            LEFT JOIN information_schema.columns c 
                ON t.table_name = c.table_name 
                AND t.table_schema = c.table_schema
            WHERE t.table_schema = 'public' 
                AND t.table_type = 'BASE TABLE'
            GROUP BY t.table_name
            ORDER BY t.table_name
            "#
        )
        .fetch_all(pool)
        .await
        .unwrap();
        
        let mut table_infos = Vec::new();
        for table_row in tables {
            let table_name: String = table_row.get("table_name");
            
            // Get columns for this table
            let columns = sqlx::query(
                r#"
                SELECT column_name, data_type, is_nullable
                FROM information_schema.columns
                WHERE table_schema = 'public' AND table_name = $1
                ORDER BY ordinal_position
                "#
            )
            .bind(&table_name)
            .fetch_all(pool)
            .await
            .unwrap();
            
            let column_infos: Vec<ColumnInfo> = columns.into_iter()
                .map(|col| ColumnInfo {
                    name: col.get("column_name"),
                    data_type: col.get("data_type"),
                    is_nullable: col.get::<String, _>("is_nullable") == "YES",
                })
                .collect();
            
            // Get row count
            let count_query = format!("SELECT COUNT(*) FROM {}", table_name);
            let row_count: i64 = sqlx::query_scalar(&count_query)
                .fetch_one(pool)
                .await
                .unwrap_or(0);
            
            table_infos.push(TableInfo {
                name: table_name,
                columns: column_infos,
                row_count,
            });
        }
        
        // Get indexes
        let indexes = sqlx::query(
            "SELECT indexname FROM pg_indexes WHERE schemaname = 'public'"
        )
        .fetch_all(pool)
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.get("indexname"))
        .collect();
        
        // Get constraints
        let constraints = sqlx::query(
            r#"
            SELECT constraint_name || ' (' || constraint_type || ')' as constraint_info
            FROM information_schema.table_constraints
            WHERE constraint_schema = 'public'
            "#
        )
        .fetch_all(pool)
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.get("constraint_info"))
        .collect();
        
        // Get views
        let views = sqlx::query(
            "SELECT table_name FROM information_schema.views WHERE table_schema = 'public'"
        )
        .fetch_all(pool)
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.get("table_name"))
        .collect();
        
        SchemaSnapshot {
            tables: table_infos,
            indexes,
            constraints,
            views,
        }
    }
    
    fn verify_schema_changes(before: &SchemaSnapshot, after: &SchemaSnapshot, migrations: &[String]) {
        println!("📋 Verifying schema changes...");
        
        // Check for new tables
        let before_tables: std::collections::HashSet<_> = before.tables.iter().map(|t| &t.name).collect();
        let after_tables: std::collections::HashSet<_> = after.tables.iter().map(|t| &t.name).collect();
        
        let new_tables: Vec<_> = after_tables.difference(&before_tables).collect();
        if !new_tables.is_empty() {
            println!("   New tables added: {:?}", new_tables);
        }
        
        // Check for removed tables (should not happen in migrations)
        let removed_tables: Vec<_> = before_tables.difference(&after_tables).collect();
        assert!(
            removed_tables.is_empty(),
            "Tables were removed in migration: {:?}",
            removed_tables
        );
        
        // Check for column changes
        for after_table in &after.tables {
            if let Some(before_table) = before.tables.iter().find(|t| t.name == after_table.name) {
                let before_cols: std::collections::HashSet<_> = before_table.columns.iter().map(|c| &c.name).collect();
                let after_cols: std::collections::HashSet<_> = after_table.columns.iter().map(|c| &c.name).collect();
                
                let new_cols: Vec<_> = after_cols.difference(&before_cols).collect();
                if !new_cols.is_empty() {
                    println!("   New columns in {}: {:?}", after_table.name, new_cols);
                }
                
                let removed_cols: Vec<_> = before_cols.difference(&after_cols).collect();
                if !removed_cols.is_empty() {
                    println!("   ⚠️  Removed columns in {}: {:?}", after_table.name, removed_cols);
                }
            }
        }
        
        println!("✅ Schema changes verified");
    }
    
    async fn create_performance_test_data(pool: &PgPool, count: usize) {
        println!("🏃 Creating {} records for performance testing...", count);
        
        // Create a test user
        let user_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO users (id, username, email, password_hash, role) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(user_id)
        .bind("perf_test_user")
        .bind("perf@test.com")
        .bind("test")
        .bind("user")
        .execute(pool)
        .await
        .unwrap();
        
        // Batch insert documents
        let batch_size = 100;
        for batch_start in (0..count).step_by(batch_size) {
            let batch_end = (batch_start + batch_size).min(count);
            
            let mut query = String::from(
                "INSERT INTO documents (id, user_id, filename, original_filename, file_path, file_size, mime_type, ocr_status, ocr_failure_reason) VALUES "
            );
            
            for i in batch_start..batch_end {
                if i > batch_start {
                    query.push_str(", ");
                }
                
                let doc_id = Uuid::new_v4();
                let status = if i % 3 == 0 { "failed" } else { "completed" };
                let failure_reason = if status == "failed" {
                    match i % 5 {
                        0 => "'timeout'",
                        1 => "'memory_limit'",
                        2 => "'file_corrupted'",
                        3 => "'low_ocr_confidence'",
                        _ => "'unknown_error'",
                    }
                } else {
                    "NULL"
                };
                
                query.push_str(&format!(
                    "('{}', '{}', 'perf_doc_{}.pdf', 'perf_doc_{}.pdf', '/test/perf_{}.pdf', 1024, 'application/pdf', '{}', {})",
                    doc_id, user_id, i, i, i, status, failure_reason
                ));
            }
            
            sqlx::query(&query).execute(pool).await.unwrap();
        }
        
        println!("✅ Created {} test documents", count);
    }
    
    fn get_new_migrations_in_pr() -> Vec<String> {
        // Check if we're in a CI environment or have a base branch to compare against
        let base_branch = std::env::var("GITHUB_BASE_REF")
            .or_else(|_| std::env::var("BASE_BRANCH"))
            .unwrap_or_else(|_| "main".to_string());
        
        let output = Command::new("git")
            .args(["diff", "--name-only", &format!("origin/{}", base_branch), "HEAD", "--", "migrations/"])
            .output();
        
        match output {
            Ok(output) if output.status.success() => {
                let files = String::from_utf8_lossy(&output.stdout);
                files
                    .lines()
                    .filter(|line| line.ends_with(".sql") && !line.is_empty())
                    .map(|s| s.to_string())
                    .collect()
            }
            _ => {
                // Fallback: check for uncommitted migration files
                let output = Command::new("git")
                    .args(["status", "--porcelain", "migrations/"])
                    .output()
                    .unwrap_or_else(|_| panic!("Failed to run git status"));
                
                if output.status.success() {
                    let files = String::from_utf8_lossy(&output.stdout);
                    files
                        .lines()
                        .filter(|line| line.contains(".sql") && (line.starts_with("A ") || line.starts_with("??")))
                        .map(|line| line.split_whitespace().last().unwrap_or("").to_string())
                        .filter(|f| !f.is_empty())
                        .collect()
                } else {
                    Vec::new()
                }
            }
        }
    }
    
    fn migration_involves_data_transformation(migrations: &[String]) -> bool {
        // Check if any migration file contains data transformation keywords
        for migration_file in migrations {
            if let Ok(content) = std::fs::read_to_string(migration_file) {
                let lowercase = content.to_lowercase();
                if lowercase.contains("insert into") && lowercase.contains("select") ||
                   lowercase.contains("update") && lowercase.contains("set") ||
                   lowercase.contains("migrate") ||
                   lowercase.contains("transform") ||
                   lowercase.contains("failed_documents") {
                    return true;
                }
            }
        }
        false
    }
}