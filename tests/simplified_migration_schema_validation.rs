use readur::test_utils::TestContext;
use sqlx::Row;
use std::collections::HashSet;

#[cfg(test)]
mod simplified_migration_schema_validation_tests {
    use super::*;

    #[tokio::test]
    async fn test_core_tables_exist() {
        let ctx = TestContext::new().await;
        let pool = ctx.state.db.get_pool();
        
        let core_tables = vec![
            "users",
            "documents", 
            "failed_documents",
            "ocr_queue",
            "settings",
        ];
        
        let existing_tables = get_all_tables(pool).await;
        
        for table in core_tables {
            assert!(
                existing_tables.contains(table),
                "Core table '{}' not found in database schema",
                table
            );
        }
        
        println!("✅ All core tables exist");
        println!("Found {} total tables in database", existing_tables.len());
    }

    #[tokio::test]
    async fn test_basic_schema_integrity() {
        let ctx = TestContext::new().await;
        let pool = ctx.state.db.get_pool();
        
        // Test that we can query key tables without errors
        let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(pool)
            .await
            .unwrap();
        
        let doc_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM documents")
            .fetch_one(pool)
            .await
            .unwrap();
        
        let failed_doc_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM failed_documents")
            .fetch_one(pool)
            .await
            .unwrap();
        
        println!("✅ Basic schema integrity verified");
        println!("   - Users: {}", user_count);
        println!("   - Documents: {}", doc_count);
        println!("   - Failed documents: {}", failed_doc_count);
        
        // All counts should be non-negative (basic sanity check)
        assert!(user_count >= 0);
        assert!(doc_count >= 0);
        assert!(failed_doc_count >= 0);
    }

    #[tokio::test]
    async fn test_migration_tables_structure() {
        let ctx = TestContext::new().await;
        let pool = ctx.state.db.get_pool();
        
        // Test that failed_documents table has the expected columns for migration
        let columns = sqlx::query(
            "SELECT column_name FROM information_schema.columns 
             WHERE table_schema = 'public' AND table_name = 'failed_documents'"
        )
        .fetch_all(pool)
        .await
        .unwrap();
        
        let column_names: Vec<String> = columns.iter()
            .map(|row| row.get("column_name"))
            .collect();
        
        let migration_critical_columns = vec![
            "id", "user_id", "filename", "failure_reason", "failure_stage", "ingestion_source"
        ];
        
        for col in migration_critical_columns {
            assert!(
                column_names.contains(&col.to_string()),
                "Critical column '{}' not found in failed_documents table",
                col
            );
        }
        
        println!("✅ Migration-critical table structure verified");
        println!("   failed_documents has {} columns", column_names.len());
    }

    #[tokio::test]
    async fn test_constraint_sampling() {
        let ctx = TestContext::new().await;
        let pool = ctx.state.db.get_pool();
        
        // Test a few key constraints exist
        let constraints = sqlx::query(
            "SELECT constraint_name, constraint_type 
             FROM information_schema.table_constraints 
             WHERE table_schema = 'public'"
        )
        .fetch_all(pool)
        .await
        .unwrap();
        
        let primary_keys: Vec<String> = constraints.iter()
            .filter(|row| row.get::<String, _>("constraint_type") == "PRIMARY KEY")
            .map(|row| row.get("constraint_name"))
            .collect();
        
        let foreign_keys: Vec<String> = constraints.iter()
            .filter(|row| row.get::<String, _>("constraint_type") == "FOREIGN KEY")
            .map(|row| row.get("constraint_name"))
            .collect();
        
        let check_constraints: Vec<String> = constraints.iter()
            .filter(|row| row.get::<String, _>("constraint_type") == "CHECK")
            .map(|row| row.get("constraint_name"))
            .collect();
        
        println!("✅ Database constraints verified");
        println!("   - Primary keys: {}", primary_keys.len());
        println!("   - Foreign keys: {}", foreign_keys.len());
        println!("   - Check constraints: {}", check_constraints.len());
        
        // Basic sanity checks
        assert!(primary_keys.len() > 0, "Should have at least one primary key");
        assert!(foreign_keys.len() > 0, "Should have at least one foreign key");
    }

    #[tokio::test]
    async fn test_migration_workflow_readiness() {
        let ctx = TestContext::new().await;
        let pool = ctx.state.db.get_pool();
        
        // Test that the database is ready for the migration workflow we test
        // This includes checking that we can insert test data successfully
        
        // Create a test user
        let user_id = uuid::Uuid::new_v4();
        let username = format!("migration_test_{}", user_id.to_string().split('-').next().unwrap());
        
        let user_result = sqlx::query(
            "INSERT INTO users (id, username, email, password_hash, role) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(user_id)
        .bind(&username)
        .bind(format!("{}@test.com", username))
        .bind("test_hash")
        .bind("user")
        .execute(pool)
        .await;
        
        assert!(user_result.is_ok(), "Should be able to create test user");
        
        // Test that failed_documents accepts valid data
        let failed_doc_result = sqlx::query(
            "INSERT INTO failed_documents (user_id, filename, failure_reason, failure_stage, ingestion_source) 
             VALUES ($1, 'test.pdf', 'other', 'ocr', 'test')"
        )
        .bind(user_id)
        .execute(pool)
        .await;
        
        assert!(failed_doc_result.is_ok(), "Should be able to insert into failed_documents");
        
        // Clean up
        sqlx::query("DELETE FROM failed_documents WHERE user_id = $1")
            .bind(user_id)
            .execute(pool)
            .await
            .unwrap();
        
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(pool)
            .await
            .unwrap();
        
        println!("✅ Migration workflow readiness verified");
    }

    // Helper functions
    
    async fn get_all_tables(pool: &sqlx::PgPool) -> HashSet<String> {
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
}