use readur::test_utils::TestContext;
use sqlx::{PgPool, Row};
use std::collections::{HashMap, HashSet};

#[cfg(test)]
mod migration_schema_validation_tests {
    use super::*;

    #[tokio::test]
    async fn test_all_expected_tables_exist() {
        let ctx = TestContext::new().await;
        let pool = ctx.state.db.get_pool();
        
        let expected_tables = vec![
            "users",
            "documents", 
            "document_labels",
            "failed_documents",
            "ignored_files",
            "labels",
            "notifications",
            "ocr_metrics",
            "ocr_queue",
            "ocr_retry_history",
            "processed_images",
            "settings",
            "source_labels",
            "sources",
            "webdav_directories",
            "webdav_files",
            "webdav_sync_state",
            "_sqlx_migrations",
        ];
        
        let existing_tables = get_all_tables(pool).await;
        
        for table in expected_tables {
            assert!(
                existing_tables.contains(table),
                "Expected table '{}' not found in database schema",
                table
            );
        }
        
        println!("✅ All expected tables exist");
    }

    #[tokio::test]
    async fn test_table_columns_and_types() {
        let ctx = TestContext::new().await;
        let pool = ctx.state.db.get_pool();
        
        // Define expected columns for critical tables
        let table_schemas = vec![
            TableSchema {
                name: "documents",
                columns: vec![
                    ("id", "uuid", false),
                    ("user_id", "uuid", false),
                    ("filename", "text", false),
                    ("original_filename", "text", true),
                    ("file_path", "text", false),
                    ("file_size", "bigint", false),
                    ("file_hash", "character varying", true),
                    ("mime_type", "text", false),
                    ("content", "text", true),
                    ("tags", "ARRAY", true),
                    ("ocr_text", "text", true),
                    ("ocr_status", "character varying", false),
                    ("ocr_confidence", "real", true),
                    ("ocr_failure_reason", "text", true),
                    ("created_at", "timestamp with time zone", false),
                    ("updated_at", "timestamp with time zone", false),
                ],
            },
            TableSchema {
                name: "failed_documents",
                columns: vec![
                    ("id", "uuid", false),
                    ("user_id", "uuid", true),
                    ("filename", "text", false),
                    ("failure_reason", "text", false),
                    ("failure_stage", "text", false),
                    ("ingestion_source", "text", false),
                    ("error_message", "text", true),
                    ("retry_count", "integer", true),
                    ("created_at", "timestamp with time zone", true),
                    ("updated_at", "timestamp with time zone", true),
                ],
            },
            TableSchema {
                name: "ocr_queue",
                columns: vec![
                    ("id", "uuid", false),
                    ("document_id", "uuid", false),
                    ("priority", "integer", false),
                    ("status", "character varying", false),
                    ("error_message", "text", true),
                    ("processing_started_at", "timestamp with time zone", true),
                    ("processing_completed_at", "timestamp with time zone", true),
                    ("created_at", "timestamp with time zone", false),
                    ("updated_at", "timestamp with time zone", false),
                ],
            },
        ];
        
        for schema in table_schemas {
            validate_table_schema(pool, &schema).await;
        }
    }

    #[tokio::test]
    async fn test_all_constraints_exist() {
        let ctx = TestContext::new().await;
        let pool = ctx.state.db.get_pool();
        
        // Test primary keys
        let primary_keys = vec![
            ("documents", "documents_pkey"),
            ("users", "users_pkey"),
            ("failed_documents", "failed_documents_pkey"),
            ("ocr_queue", "ocr_queue_pkey"),
            ("labels", "labels_pkey"),
            ("settings", "settings_pkey"),
        ];
        
        for (table, constraint) in primary_keys {
            let exists = constraint_exists(pool, table, constraint, "PRIMARY KEY").await;
            assert!(exists, "Primary key '{}' not found on table '{}'", constraint, table);
        }
        
        // Test foreign keys
        let foreign_keys = vec![
            ("documents", "documents_user_id_fkey"),
            ("failed_documents", "failed_documents_user_id_fkey"),
            ("failed_documents", "failed_documents_existing_document_id_fkey"),
            ("ocr_queue", "ocr_queue_document_id_fkey"),
            ("document_labels", "document_labels_document_id_fkey"),
            ("document_labels", "document_labels_label_id_fkey"),
        ];
        
        for (table, constraint) in foreign_keys {
            let exists = constraint_exists(pool, table, constraint, "FOREIGN KEY").await;
            assert!(exists, "Foreign key '{}' not found on table '{}'", constraint, table);
        }
        
        // Test check constraints
        let check_constraints = vec![
            ("failed_documents", "check_failure_reason"),
            ("failed_documents", "check_failure_stage"),
            ("documents", "check_ocr_status"),
            ("users", "check_role"),
        ];
        
        for (table, constraint) in check_constraints {
            let exists = constraint_exists(pool, table, constraint, "CHECK").await;
            assert!(exists, "Check constraint '{}' not found on table '{}'", constraint, table);
        }
        
        println!("✅ All expected constraints exist");
    }

    #[tokio::test]
    async fn test_indexes_for_performance() {
        let ctx = TestContext::new().await;
        let pool = ctx.state.db.get_pool();
        
        let expected_indexes = vec![
            ("documents", "idx_documents_user_id"),
            ("documents", "idx_documents_created_at"),
            ("documents", "idx_documents_ocr_status"),
            ("failed_documents", "idx_failed_documents_user_id"),
            ("failed_documents", "idx_failed_documents_created_at"),
            ("failed_documents", "idx_failed_documents_failure_reason"),
            ("failed_documents", "idx_failed_documents_failure_stage"),
            ("ocr_queue", "idx_ocr_queue_status"),
            ("ocr_queue", "idx_ocr_queue_document_id"),
        ];
        
        for (table, index) in expected_indexes {
            let exists = index_exists(pool, table, index).await;
            assert!(exists, "Performance index '{}' not found on table '{}'", index, table);
        }
        
        println!("✅ All performance indexes exist");
    }

    #[tokio::test]
    async fn test_views_and_functions() {
        let ctx = TestContext::new().await;
        let pool = ctx.state.db.get_pool();
        
        // Test views
        let expected_views = vec![
            "failed_documents_summary",
            "legacy_failed_ocr_documents",
            "ocr_analytics",
        ];
        
        let existing_views = get_all_views(pool).await;
        
        for view in expected_views {
            assert!(
                existing_views.contains(view),
                "Expected view '{}' not found in database",
                view
            );
        }
        
        // Test functions
        let expected_functions = vec![
            "add_document_to_ocr_queue",
            "get_ocr_queue_stats",
        ];
        
        let existing_functions = get_all_functions(pool).await;
        
        for func in expected_functions {
            assert!(
                existing_functions.contains(func),
                "Expected function '{}' not found in database",
                func
            );
        }
        
        println!("✅ All views and functions exist");
    }

    #[tokio::test]
    async fn test_enum_values_match_constraints() {
        let ctx = TestContext::new().await;
        let pool = ctx.state.db.get_pool();
        
        // Test failure_reason enum values
        let failure_reasons = vec![
            "duplicate_content", "duplicate_filename", "unsupported_format",
            "file_too_large", "file_corrupted", "access_denied", 
            "low_ocr_confidence", "ocr_timeout", "ocr_memory_limit",
            "pdf_parsing_error", "storage_quota_exceeded", "network_error",
            "permission_denied", "virus_detected", "invalid_structure",
            "policy_violation", "other"
        ];
        
        for reason in &failure_reasons {
            let result = sqlx::query(
                "SELECT 1 WHERE $1::text IN (SELECT unnest(enum_range(NULL::text)::text[]))"
            )
            .bind(reason)
            .fetch_optional(pool)
            .await;
            
            // If this is not an enum type, test the CHECK constraint instead
            if result.is_err() || result.unwrap().is_none() {
                // Test by attempting insert with valid value (should succeed)
                // We'll use a transaction that we rollback to avoid polluting test data
                let mut tx = pool.begin().await.unwrap();
                
                // First create a test user
                let test_user_id = uuid::Uuid::new_v4();
                sqlx::query(
                    "INSERT INTO users (id, username, email, password_hash, role) VALUES ($1, $2, $3, $4, $5)"
                )
                .bind(test_user_id)
                .bind(format!("enum_test_{}", uuid::Uuid::new_v4()))
                .bind(format!("enum_test_{}@test.com", uuid::Uuid::new_v4()))
                .bind("test")
                .bind("user")
                .execute(&mut *tx)
                .await
                .unwrap();
                
                let insert_result = sqlx::query(
                    "INSERT INTO failed_documents (user_id, filename, failure_reason, failure_stage, ingestion_source) 
                     VALUES ($1, 'test.pdf', $2, 'ocr', 'test')"
                )
                .bind(test_user_id)
                .bind(reason)
                .execute(&mut *tx)
                .await;
                
                assert!(insert_result.is_ok(), 
                    "Valid failure_reason '{}' should be accepted by constraint", reason);
                
                tx.rollback().await.unwrap();
            }
        }
        
        // Test failure_stage enum values
        let failure_stages = vec![
            "ingestion", "validation", "ocr", "storage", "processing", "sync"
        ];
        
        for stage in &failure_stages {
            let mut tx = pool.begin().await.unwrap();
            
            // Create test user
            let test_user_id = uuid::Uuid::new_v4();
            sqlx::query(
                "INSERT INTO users (id, username, email, password_hash, role) VALUES ($1, $2, $3, $4, $5)"
            )
            .bind(test_user_id)
            .bind(format!("stage_test_{}", uuid::Uuid::new_v4()))
            .bind(format!("stage_test_{}@test.com", uuid::Uuid::new_v4()))
            .bind("test")
            .bind("user")
            .execute(&mut *tx)
            .await
            .unwrap();
            
            let insert_result = sqlx::query(
                "INSERT INTO failed_documents (user_id, filename, failure_reason, failure_stage, ingestion_source) 
                 VALUES ($1, 'test.pdf', 'other', $2, 'test')"
            )
            .bind(test_user_id)
            .bind(stage)
            .execute(&mut *tx)
            .await;
            
            assert!(insert_result.is_ok(), 
                "Valid failure_stage '{}' should be accepted by constraint", stage);
            
            tx.rollback().await.unwrap();
        }
        
        println!("✅ All enum values match constraints");
    }

    #[tokio::test]
    async fn test_migration_specific_changes() {
        let ctx = TestContext::new().await;
        let pool = ctx.state.db.get_pool();
        
        // Test that failed_documents table has all columns from migration
        let failed_docs_columns = get_table_columns(pool, "failed_documents").await;
        
        let migration_columns = vec![
            "id", "user_id", "filename", "original_filename", "original_path",
            "file_path", "file_size", "file_hash", "mime_type", "content", "tags",
            "ocr_text", "ocr_confidence", "ocr_word_count", "ocr_processing_time_ms",
            "failure_reason", "failure_stage", "existing_document_id", 
            "ingestion_source", "error_message", "retry_count", "last_retry_at",
            "created_at", "updated_at"
        ];
        
        for col in migration_columns {
            assert!(
                failed_docs_columns.contains(&col.to_string()),
                "Column '{}' not found in failed_documents table",
                col
            );
        }
        
        // Test that documents table has ocr_failure_reason column
        let docs_columns = get_table_columns(pool, "documents").await;
        assert!(
            docs_columns.contains(&"ocr_failure_reason".to_string()),
            "ocr_failure_reason column not found in documents table"
        );
        
        // Test that the legacy view exists
        let views = get_all_views(pool).await;
        assert!(
            views.contains("legacy_failed_ocr_documents"),
            "legacy_failed_ocr_documents view not found"
        );
        
        println!("✅ Migration-specific changes verified");
    }

    // Helper functions
    
    struct TableSchema {
        name: &'static str,
        columns: Vec<(&'static str, &'static str, bool)>, // (name, type, nullable)
    }
    
    async fn get_all_tables(pool: &PgPool) -> HashSet<String> {
        let rows = sqlx::query(
            "SELECT table_name FROM information_schema.tables 
             WHERE table_schema = 'public' AND table_type = 'BASE TABLE'"
        )
        .fetch_all(pool)
        .await
        .unwrap();
        
        rows.into_iter()
            .map(|row| row.get("table_name"))
            .collect()
    }
    
    async fn get_all_views(pool: &PgPool) -> HashSet<String> {
        let rows = sqlx::query(
            "SELECT table_name FROM information_schema.views WHERE table_schema = 'public'"
        )
        .fetch_all(pool)
        .await
        .unwrap();
        
        rows.into_iter()
            .map(|row| row.get("table_name"))
            .collect()
    }
    
    async fn get_all_functions(pool: &PgPool) -> HashSet<String> {
        let rows = sqlx::query(
            "SELECT routine_name FROM information_schema.routines 
             WHERE routine_schema = 'public' AND routine_type = 'FUNCTION'"
        )
        .fetch_all(pool)
        .await
        .unwrap();
        
        rows.into_iter()
            .map(|row| row.get("routine_name"))
            .collect()
    }
    
    async fn get_table_columns(pool: &PgPool, table_name: &str) -> Vec<String> {
        let rows = sqlx::query(
            "SELECT column_name FROM information_schema.columns 
             WHERE table_schema = 'public' AND table_name = $1"
        )
        .bind(table_name)
        .fetch_all(pool)
        .await
        .unwrap();
        
        rows.into_iter()
            .map(|row| row.get("column_name"))
            .collect()
    }
    
    async fn validate_table_schema(pool: &PgPool, schema: &TableSchema) {
        let columns = sqlx::query(
            "SELECT column_name, data_type, is_nullable 
             FROM information_schema.columns 
             WHERE table_schema = 'public' AND table_name = $1"
        )
        .bind(schema.name)
        .fetch_all(pool)
        .await
        .unwrap();
        
        let column_map: HashMap<String, (String, bool)> = columns.into_iter()
            .map(|row| {
                let name: String = row.get("column_name");
                let data_type: String = row.get("data_type");
                let is_nullable: String = row.get("is_nullable");
                (name, (data_type, is_nullable == "YES"))
            })
            .collect();
        
        for (col_name, expected_type, nullable) in &schema.columns {
            let column_info = column_map.get(*col_name);
            assert!(
                column_info.is_some(),
                "Column '{}' not found in table '{}'",
                col_name, schema.name
            );
            
            let (actual_type, actual_nullable) = column_info.unwrap();
            
            // Type checking (handle array types specially)
            if expected_type == &"ARRAY" {
                assert!(
                    actual_type.contains("ARRAY") || actual_type.contains("[]"),
                    "Column '{}' in table '{}' expected array type but got '{}'",
                    col_name, schema.name, actual_type
                );
            } else {
                assert!(
                    actual_type.to_lowercase().contains(&expected_type.to_lowercase()),
                    "Column '{}' in table '{}' expected type '{}' but got '{}'",
                    col_name, schema.name, expected_type, actual_type
                );
            }
            
            assert_eq!(
                actual_nullable, nullable,
                "Column '{}' in table '{}' nullable mismatch",
                col_name, schema.name
            );
        }
        
        println!("✅ Schema validated for table '{}'", schema.name);
    }
    
    async fn constraint_exists(pool: &PgPool, table: &str, constraint: &str, constraint_type: &str) -> bool {
        let result = sqlx::query(
            "SELECT 1 FROM information_schema.table_constraints 
             WHERE table_schema = 'public' 
             AND table_name = $1 
             AND constraint_name = $2 
             AND constraint_type = $3"
        )
        .bind(table)
        .bind(constraint)
        .bind(constraint_type)
        .fetch_optional(pool)
        .await
        .unwrap();
        
        result.is_some()
    }
    
    async fn index_exists(pool: &PgPool, table: &str, index: &str) -> bool {
        let result = sqlx::query(
            "SELECT 1 FROM pg_indexes 
             WHERE schemaname = 'public' 
             AND tablename = $1 
             AND indexname = $2"
        )
        .bind(table)
        .bind(index)
        .fetch_optional(pool)
        .await
        .unwrap();
        
        result.is_some()
    }
}