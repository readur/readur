#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::UserRole;
    use crate::routes::labels::{CreateLabel, UpdateLabel, LabelAssignment, Label};
    use axum::http::StatusCode;
    use chrono::Utc;
    use serde_json::json;
    use sqlx::{PgPool, Row};
    use std::collections::HashMap;
    use testcontainers::{runners::AsyncRunner, ContainerAsync};
    use testcontainers_modules::postgres::Postgres;
    use uuid::Uuid;

    struct TestContext {
        db: PgPool,
        _container: ContainerAsync<Postgres>,
        user_id: Uuid,
        admin_user_id: Uuid,
    }

    async fn setup_test_db() -> TestContext {
        // Start PostgreSQL container
        let postgres_image = Postgres::default();
        let container = postgres_image.start().await.expect("Failed to start postgres container");
        let port = container.get_host_port_ipv4(5432).await.expect("Failed to get postgres port");
        
        let connection_string = format!(
            "postgres://postgres:postgres@127.0.0.1:{}/postgres",
            port
        );

        // Connect to database
        let db = PgPool::connect(&connection_string)
            .await
            .expect("Failed to connect to test database");

        // Enable required extensions
        sqlx::query("CREATE EXTENSION IF NOT EXISTS pgcrypto;")
            .execute(&db)
            .await
            .expect("Failed to create pgcrypto extension");

        // Run migrations
        sqlx::migrate!("./migrations")
            .run(&db)
            .await
            .expect("Failed to run migrations");

        // Create test users
        let user_id = Uuid::new_v4();
        let admin_user_id = Uuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO users (id, username, email, password_hash, role, created_at, updated_at)
            VALUES 
                ($1, 'testuser', 'test@example.com', 'hashed_password', 'user', NOW(), NOW()),
                ($2, 'admin', 'admin@example.com', 'hashed_password', 'admin', NOW(), NOW())
            "#,
        )
        .bind(user_id)
        .bind(admin_user_id)
        .execute(&db)
        .await
        .expect("Failed to create test users");

        TestContext {
            db,
            _container: container,
            user_id,
            admin_user_id,
        }
    }

    #[tokio::test]
    async fn test_create_label_success() {
        let ctx = setup_test_db().await;

        let label_data = CreateLabel {
            name: "Test Label".to_string(),
            description: Some("A test label".to_string()),
            color: "#ff0000".to_string(),
            background_color: None,
            icon: Some("star".to_string()),
        };

        let result = sqlx::query_scalar::<_, uuid::Uuid>(
            r#"
            INSERT INTO labels (user_id, name, description, color, icon)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id
            "#,
        )
        .bind(ctx.user_id)
        .bind(&label_data.name)
        .bind(&label_data.description)
        .bind(&label_data.color)
        .bind(&label_data.icon)
        .fetch_one(&ctx.db)
        .await;

        assert!(result.is_ok());
        let label_id = result.unwrap();

        // Verify label was created
        let created_label = sqlx::query_as::<_, Label>(
            "SELECT id, user_id, name, description, color, background_color, icon, is_system, created_at, updated_at, 0::bigint as document_count, 0::bigint as source_count FROM labels WHERE id = $1"
        )
        .bind(label_id)
        .fetch_one(&ctx.db)
        .await
        .expect("Failed to fetch created label");

        assert_eq!(created_label.name, "Test Label");
        assert_eq!(created_label.description.as_ref().unwrap(), "A test label");
        assert_eq!(created_label.color, "#ff0000");
        assert_eq!(created_label.icon.as_ref().unwrap(), "star");
        assert_eq!(created_label.user_id, Some(ctx.user_id));
        assert!(!created_label.is_system);
    }

    #[tokio::test]
    async fn test_create_label_duplicate_name_fails() {
        let ctx = setup_test_db().await;

        // Create first label
        sqlx::query(
            r#"
            INSERT INTO labels (user_id, name, color)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(ctx.user_id)
        .bind("Duplicate Name")
        .bind("#ff0000")
        .execute(&ctx.db)
        .await
        .expect("Failed to create first label");

        // Try to create duplicate
        let result = sqlx::query(
            r#"
            INSERT INTO labels (user_id, name, color)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(ctx.user_id)
        .bind("Duplicate Name")
        .bind("#00ff00")
        .execute(&ctx.db)
        .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("duplicate key"));
    }

    #[tokio::test]
    async fn test_update_label_success() {
        let ctx = setup_test_db().await;

        // Create label
        let label_id = sqlx::query_scalar::<_, uuid::Uuid>(
            r#"
            INSERT INTO labels (user_id, name, color)
            VALUES ($1, $2, $3)
            RETURNING id
            "#,
        )
        .bind(ctx.user_id)
        .bind("Original Name")
        .bind("#ff0000")
        .fetch_one(&ctx.db)
        .await
        .unwrap();

        // Update label
        let update_data = UpdateLabel {
            name: Some("Updated Name".to_string()),
            description: Some("Updated description".to_string()),
            color: Some("#00ff00".to_string()),
            background_color: None,
            icon: Some("edit".to_string()),
        };

        let result = sqlx::query_as::<_, Label>(
            r#"
            UPDATE labels 
            SET 
                name = COALESCE($2, name),
                description = COALESCE($3, description),
                color = COALESCE($4, color),
                icon = COALESCE($5, icon),
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $1 AND user_id = $6
            RETURNING id, user_id, name, description, color, background_color, icon, is_system, created_at, updated_at, 0::bigint as document_count, 0::bigint as source_count
            "#,
        )
        .bind(label_id)
        .bind(&update_data.name)
        .bind(&update_data.description)
        .bind(&update_data.color)
        .bind(&update_data.icon)
        .bind(ctx.user_id)
        .fetch_one(&ctx.db)
        .await;

        assert!(result.is_ok());
        let updated_label = result.unwrap();

        assert_eq!(updated_label.name, "Updated Name");
        assert_eq!(updated_label.description.as_ref().unwrap(), "Updated description");
        assert_eq!(updated_label.color, "#00ff00");
        assert_eq!(updated_label.icon.as_ref().unwrap(), "edit");
    }

    #[tokio::test]
    async fn test_delete_label_success() {
        let ctx = setup_test_db().await;

        // Create label
        let label_id = sqlx::query_scalar::<_, uuid::Uuid>(
            r#"
            INSERT INTO labels (user_id, name, color)
            VALUES ($1, $2, $3)
            RETURNING id
            "#,
        )
        .bind(ctx.user_id)
        .bind("To Delete")
        .bind("#ff0000")
        .fetch_one(&ctx.db)
        .await
        .unwrap();

        // Delete label
        let result = sqlx::query(
            "DELETE FROM labels WHERE id = $1 AND user_id = $2 AND is_system = FALSE"
        )
        .bind(label_id)
        .bind(ctx.user_id)
        .execute(&ctx.db)
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().rows_affected(), 1);

        // Verify deletion
        let deleted_label = sqlx::query_scalar::<_, uuid::Uuid>(
            "SELECT id FROM labels WHERE id = $1"
        )
        .bind(label_id)
        .fetch_optional(&ctx.db)
        .await
        .expect("Query failed");

        assert!(deleted_label.is_none());
    }

    #[tokio::test]
    async fn test_cannot_delete_system_label() {
        let ctx = setup_test_db().await;

        // Create system label
        let label_id = sqlx::query_scalar::<_, uuid::Uuid>(
            r#"
            INSERT INTO labels (user_id, name, color, is_system)
            VALUES ($1, $2, $3, $4)
            RETURNING id
            "#,
        )
        .bind(None::<Uuid>) // System labels have NULL user_id
        .bind("System Label")
        .bind("#ff0000")
        .bind(true)
        .fetch_one(&ctx.db)
        .await
        .unwrap();

        // Try to delete system label
        let result = sqlx::query(
            "DELETE FROM labels WHERE id = $1 AND user_id = $2 AND is_system = FALSE"
        )
        .bind(label_id)
        .bind(ctx.user_id)
        .execute(&ctx.db)
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().rows_affected(), 0); // No rows affected

        // Verify system label still exists
        let system_label = sqlx::query_scalar::<_, uuid::Uuid>(
            "SELECT id FROM labels WHERE id = $1"
        )
        .bind(label_id)
        .fetch_one(&ctx.db)
        .await;

        assert!(system_label.is_ok());
    }

    #[tokio::test]
    async fn test_document_label_assignment() {
        let ctx = setup_test_db().await;

        // Create document
        let document_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO documents (
                id, user_id, filename, original_filename, file_path, 
                file_size, mime_type, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())
            "#,
        )
        .bind(document_id)
        .bind(ctx.user_id)
        .bind("test.txt")
        .bind("test.txt")
        .bind("/test/test.txt")
        .bind(1024)
        .bind("text/plain")
        .execute(&ctx.db)
        .await
        .expect("Failed to create test document");

        // Create label
        let label_id = sqlx::query_scalar::<_, uuid::Uuid>(
            r#"
            INSERT INTO labels (user_id, name, color)
            VALUES ($1, $2, $3)
            RETURNING id
            "#,
        )
        .bind(ctx.user_id)
        .bind("Document Label")
        .bind("#ff0000")
        .fetch_one(&ctx.db)
        .await
        .unwrap();

        // Assign label to document
        let result = sqlx::query(
            r#"
            INSERT INTO document_labels (document_id, label_id, assigned_by)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(document_id)
        .bind(label_id)
        .bind(ctx.user_id)
        .execute(&ctx.db)
        .await;

        assert!(result.is_ok());

        // Verify assignment
        let assignment = sqlx::query(
            r#"
            SELECT dl.document_id, dl.label_id, dl.assigned_by, dl.created_at, l.name as label_name
            FROM document_labels dl
            JOIN labels l ON dl.label_id = l.id
            WHERE dl.document_id = $1 AND dl.label_id = $2
            "#,
        )
        .bind(document_id)
        .bind(label_id)
        .fetch_one(&ctx.db)
        .await;

        assert!(assignment.is_ok());
        let assignment = assignment.unwrap();
        let label_name: String = assignment.get("label_name");
        let assigned_by: Option<uuid::Uuid> = assignment.get("assigned_by");
        assert_eq!(label_name, "Document Label");
        assert_eq!(assigned_by.unwrap(), ctx.user_id);
    }

    #[tokio::test]
    async fn test_document_label_removal() {
        let ctx = setup_test_db().await;

        // Create document and label
        let document_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO documents (
                id, user_id, filename, original_filename, file_path, 
                file_size, mime_type, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())
            "#,
        )
        .bind(document_id)
        .bind(ctx.user_id)
        .bind("test.txt")
        .bind("test.txt")
        .bind("/test/test.txt")
        .bind(1024)
        .bind("text/plain")
        .execute(&ctx.db)
        .await
        .expect("Failed to create test document");

        let label_id = sqlx::query_scalar::<_, uuid::Uuid>(
            r#"
            INSERT INTO labels (user_id, name, color)
            VALUES ($1, $2, $3)
            RETURNING id
            "#,
        )
        .bind(ctx.user_id)
        .bind("Document Label")
        .bind("#ff0000")
        .fetch_one(&ctx.db)
        .await
        .unwrap();

        // Assign label
        sqlx::query(
            r#"
            INSERT INTO document_labels (document_id, label_id, assigned_by)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(document_id)
        .bind(label_id)
        .bind(ctx.user_id)
        .execute(&ctx.db)
        .await
        .expect("Failed to assign label");

        // Remove label
        let result = sqlx::query(
            "DELETE FROM document_labels WHERE document_id = $1 AND label_id = $2"
        )
        .bind(document_id)
        .bind(label_id)
        .execute(&ctx.db)
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().rows_affected(), 1);

        // Verify removal
        let assignment = sqlx::query(
            "SELECT document_id FROM document_labels WHERE document_id = $1 AND label_id = $2"
        )
        .bind(document_id)
        .bind(label_id)
        .fetch_optional(&ctx.db)
        .await
        .expect("Query failed");

        assert!(assignment.is_none());
    }

    #[tokio::test]
    async fn test_get_document_labels() {
        let ctx = setup_test_db().await;

        // Create document
        let document_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO documents (
                id, user_id, filename, original_filename, file_path, 
                file_size, mime_type, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())
            "#,
        )
        .bind(document_id)
        .bind(ctx.user_id)
        .bind("test.txt")
        .bind("test.txt")
        .bind("/test/test.txt")
        .bind(1024)
        .bind("text/plain")
        .execute(&ctx.db)
        .await
        .expect("Failed to create test document");

        // Create multiple labels
        let mut label_ids = Vec::new();
        for (i, name) in vec!["Label 1", "Label 2", "Label 3"].iter().enumerate() {
            let label_id = sqlx::query_scalar::<_, uuid::Uuid>(
                r#"
                INSERT INTO labels (user_id, name, color)
                VALUES ($1, $2, $3)
                RETURNING id
                "#,
            )
            .bind(ctx.user_id)
            .bind(name)
            .bind(format!("#ff{:02x}00", i * 50))
            .fetch_one(&ctx.db)
            .await
            .unwrap();
            label_ids.push(label_id);
        }

        // Assign labels to document
        for label_id in &label_ids {
            sqlx::query(
                r#"
                INSERT INTO document_labels (document_id, label_id, assigned_by)
                VALUES ($1, $2, $3)
                "#,
            )
            .bind(document_id)
            .bind(label_id)
            .bind(ctx.user_id)
            .execute(&ctx.db)
            .await
            .expect("Failed to assign label");
        }

        // Get document labels
        let document_labels = sqlx::query(
            r#"
            SELECT l.id, l.name, l.color, l.icon, l.description, l.is_system
            FROM labels l
            INNER JOIN document_labels dl ON l.id = dl.label_id
            WHERE dl.document_id = $1
            ORDER BY l.name
            "#,
        )
        .bind(document_id)
        .fetch_all(&ctx.db)
        .await
        .expect("Failed to fetch document labels");

        assert_eq!(document_labels.len(), 3);
        let name1: String = document_labels[0].get("name");
        let name2: String = document_labels[1].get("name");
        let name3: String = document_labels[2].get("name");
        assert_eq!(name1, "Label 1");
        assert_eq!(name2, "Label 2");
        assert_eq!(name3, "Label 3");
    }

    #[tokio::test]
    async fn test_label_usage_counts() {
        let ctx = setup_test_db().await;

        // Create label
        let label_id = sqlx::query_scalar::<_, uuid::Uuid>(
            r#"
            INSERT INTO labels (user_id, name, color)
            VALUES ($1, 'Usage Test', '#ff0000')
            RETURNING id
            "#,
        )
        .bind(ctx.user_id)
        .fetch_one(&ctx.db)
        .await
        .unwrap();

        // Create multiple documents
        let mut document_ids = Vec::new();
        for i in 0..3 {
            let doc_id = Uuid::new_v4();
            sqlx::query(
                r#"
                INSERT INTO documents (
                    id, user_id, filename, original_filename, file_path, 
                    file_size, mime_type, created_at, updated_at
                )
                VALUES ($1, $2, $3, $3, $4, 1024, 'text/plain', NOW(), NOW())
                "#,
            )
            .bind(doc_id)
            .bind(ctx.user_id)
            .bind(format!("test{}.txt", i))
            .bind(format!("/test/test{}.txt", i))
            .execute(&ctx.db)
            .await
            .expect("Failed to create test document");
            document_ids.push(doc_id);
        }

        // Assign label to documents
        for doc_id in &document_ids {
            sqlx::query(
                r#"
                INSERT INTO document_labels (document_id, label_id, assigned_by)
                VALUES ($1, $2, $3)
                "#,
            )
            .bind(doc_id)
            .bind(label_id)
            .bind(ctx.user_id)
            .execute(&ctx.db)
            .await
            .expect("Failed to assign label");
        }

        // Get usage count
        let usage_count = sqlx::query(
            r#"
            SELECT 
                l.id,
                l.name,
                COUNT(DISTINCT dl.document_id) as document_count
            FROM labels l
            LEFT JOIN document_labels dl ON l.id = dl.label_id
            WHERE l.id = $1
            GROUP BY l.id, l.name
            "#,
        )
        .bind(label_id)
        .fetch_one(&ctx.db)
        .await
        .expect("Failed to get usage count");

        let document_count: i64 = usage_count.get("document_count");
        assert_eq!(document_count, 3);
    }

    #[tokio::test]
    async fn test_label_color_validation() {
        let ctx = setup_test_db().await;

        // Test valid color
        let valid_result = sqlx::query(
            r#"
            INSERT INTO labels (user_id, name, color)
            VALUES ($1, 'Valid Color', '#ff0000')
            RETURNING id
            "#,
        )
        .bind(ctx.user_id)
        .execute(&ctx.db)
        .await;

        assert!(valid_result.is_ok());

        // Note: Database-level color validation would need to be added as a constraint
        // For now, we rely on application-level validation
    }

    #[tokio::test]
    async fn test_system_labels_migration() {
        let ctx = setup_test_db().await;

        // Check that system labels were created by migration
        let system_labels = sqlx::query(
            "SELECT name FROM labels WHERE is_system = TRUE ORDER BY name"
        )
        .fetch_all(&ctx.db)
        .await
        .expect("Failed to fetch system labels");

        // Verify expected system labels exist
        let expected_labels = vec![
            "Important", "To Review", "Archive", "Work", "Personal"
        ];

        assert!(system_labels.len() >= expected_labels.len());

        for expected_label in expected_labels {
            assert!(
                system_labels.iter().any(|label| {
                    let name: String = label.get("name");
                    name == expected_label
                }),
                "System label '{}' not found",
                expected_label
            );
        }
    }

    #[tokio::test]
    async fn test_cascade_delete_on_document_removal() {
        let ctx = setup_test_db().await;

        // Create document and label
        let document_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO documents (
                id, user_id, filename, original_filename, file_path, 
                file_size, mime_type, created_at, updated_at
            )
            VALUES ($1, $2, 'test.txt', 'test.txt', '/test/test.txt', 1024, 'text/plain', NOW(), NOW())
            "#,
        )
        .bind(document_id)
        .bind(ctx.user_id)
        .execute(&ctx.db)
        .await
        .expect("Failed to create test document");

        let label_id = sqlx::query_scalar::<_, uuid::Uuid>(
            r#"
            INSERT INTO labels (user_id, name, color)
            VALUES ($1, 'Test Label', '#ff0000')
            RETURNING id
            "#,
        )
        .bind(ctx.user_id)
        .fetch_one(&ctx.db)
        .await
        .unwrap();

        // Assign label to document
        sqlx::query(
            r#"
            INSERT INTO document_labels (document_id, label_id, assigned_by)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(document_id)
        .bind(label_id)
        .bind(ctx.user_id)
        .execute(&ctx.db)
        .await
        .expect("Failed to assign label");

        // Delete document
        sqlx::query(
            "DELETE FROM documents WHERE id = $1"
        )
        .bind(document_id)
        .execute(&ctx.db)
        .await
        .expect("Failed to delete document");

        // Verify document_labels entry was cascade deleted
        let assignments = sqlx::query(
            "SELECT document_id FROM document_labels WHERE document_id = $1"
        )
        .bind(document_id)
        .fetch_all(&ctx.db)
        .await
        .expect("Query failed");

        assert!(assignments.is_empty());

        // Verify label still exists
        let label = sqlx::query(
            "SELECT id FROM labels WHERE id = $1"
        )
        .bind(label_id)
        .fetch_one(&ctx.db)
        .await;

        assert!(label.is_ok());
    }
}