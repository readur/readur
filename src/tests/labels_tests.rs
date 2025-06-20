#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::UserRole;
    use crate::routes::labels::{CreateLabel, UpdateLabel, LabelAssignment};
    use axum::http::StatusCode;
    use chrono::Utc;
    use serde_json::json;
    use sqlx::PgPool;
    use std::collections::HashMap;
    use testcontainers::{clients::Cli, images::postgres::Postgres, Container};
    use uuid::Uuid;

    struct TestContext {
        db: PgPool,
        _container: Container<'static, Postgres>,
        user_id: Uuid,
        admin_user_id: Uuid,
    }

    async fn setup_test_db() -> TestContext {
        // Start PostgreSQL container
        let docker = Cli::default();
        let postgres_image = Postgres::default();
        let container = docker.run(postgres_image);
        
        let connection_string = format!(
            "postgres://postgres:postgres@127.0.0.1:{}/postgres",
            container.get_host_port_ipv4(5432)
        );

        // Connect to database
        let db = PgPool::connect(&connection_string)
            .await
            .expect("Failed to connect to test database");

        // Run migrations
        sqlx::migrate!("./migrations")
            .run(&db)
            .await
            .expect("Failed to run migrations");

        // Create test users
        let user_id = Uuid::new_v4();
        let admin_user_id = Uuid::new_v4();

        sqlx::query!(
            r#"
            INSERT INTO users (id, username, email, password_hash, role, created_at, updated_at)
            VALUES 
                ($1, 'testuser', 'test@example.com', 'hashed_password', 'user', NOW(), NOW()),
                ($2, 'admin', 'admin@example.com', 'hashed_password', 'admin', NOW(), NOW())
            "#,
            user_id,
            admin_user_id
        )
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

        let result = sqlx::query!(
            r#"
            INSERT INTO labels (user_id, name, description, color, icon)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id
            "#,
            ctx.user_id,
            label_data.name,
            label_data.description,
            label_data.color,
            label_data.icon
        )
        .fetch_one(&ctx.db)
        .await;

        assert!(result.is_ok());
        let label_id = result.unwrap().id;

        // Verify label was created
        let created_label = sqlx::query!(
            "SELECT * FROM labels WHERE id = $1",
            label_id
        )
        .fetch_one(&ctx.db)
        .await
        .expect("Failed to fetch created label");

        assert_eq!(created_label.name, "Test Label");
        assert_eq!(created_label.description.unwrap(), "A test label");
        assert_eq!(created_label.color, "#ff0000");
        assert_eq!(created_label.icon.unwrap(), "star");
        assert_eq!(created_label.user_id, ctx.user_id);
        assert!(!created_label.is_system);
    }

    #[tokio::test]
    async fn test_create_label_duplicate_name_fails() {
        let ctx = setup_test_db().await;

        // Create first label
        sqlx::query!(
            r#"
            INSERT INTO labels (user_id, name, color)
            VALUES ($1, 'Duplicate Name', '#ff0000')
            "#,
            ctx.user_id
        )
        .execute(&ctx.db)
        .await
        .expect("Failed to create first label");

        // Try to create duplicate
        let result = sqlx::query!(
            r#"
            INSERT INTO labels (user_id, name, color)
            VALUES ($1, 'Duplicate Name', '#00ff00')
            "#,
            ctx.user_id
        )
        .execute(&ctx.db)
        .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("duplicate key"));
    }

    #[tokio::test]
    async fn test_update_label_success() {
        let ctx = setup_test_db().await;

        // Create label
        let label_id = sqlx::query!(
            r#"
            INSERT INTO labels (user_id, name, color)
            VALUES ($1, 'Original Name', '#ff0000')
            RETURNING id
            "#,
            ctx.user_id
        )
        .fetch_one(&ctx.db)
        .await
        .unwrap()
        .id;

        // Update label
        let update_data = UpdateLabel {
            name: Some("Updated Name".to_string()),
            description: Some("Updated description".to_string()),
            color: Some("#00ff00".to_string()),
            background_color: None,
            icon: Some("edit".to_string()),
        };

        let result = sqlx::query!(
            r#"
            UPDATE labels 
            SET 
                name = COALESCE($2, name),
                description = COALESCE($3, description),
                color = COALESCE($4, color),
                icon = COALESCE($5, icon),
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $1 AND user_id = $6
            RETURNING *
            "#,
            label_id,
            update_data.name,
            update_data.description,
            update_data.color,
            update_data.icon,
            ctx.user_id
        )
        .fetch_one(&ctx.db)
        .await;

        assert!(result.is_ok());
        let updated_label = result.unwrap();

        assert_eq!(updated_label.name, "Updated Name");
        assert_eq!(updated_label.description.unwrap(), "Updated description");
        assert_eq!(updated_label.color, "#00ff00");
        assert_eq!(updated_label.icon.unwrap(), "edit");
    }

    #[tokio::test]
    async fn test_delete_label_success() {
        let ctx = setup_test_db().await;

        // Create label
        let label_id = sqlx::query!(
            r#"
            INSERT INTO labels (user_id, name, color)
            VALUES ($1, 'To Delete', '#ff0000')
            RETURNING id
            "#,
            ctx.user_id
        )
        .fetch_one(&ctx.db)
        .await
        .unwrap()
        .id;

        // Delete label
        let result = sqlx::query!(
            "DELETE FROM labels WHERE id = $1 AND user_id = $2 AND is_system = FALSE",
            label_id,
            ctx.user_id
        )
        .execute(&ctx.db)
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().rows_affected(), 1);

        // Verify deletion
        let deleted_label = sqlx::query!(
            "SELECT id FROM labels WHERE id = $1",
            label_id
        )
        .fetch_optional(&ctx.db)
        .await
        .expect("Query failed");

        assert!(deleted_label.is_none());
    }

    #[tokio::test]
    async fn test_cannot_delete_system_label() {
        let ctx = setup_test_db().await;

        // Create system label
        let label_id = sqlx::query!(
            r#"
            INSERT INTO labels (user_id, name, color, is_system)
            VALUES ($1, 'System Label', '#ff0000', TRUE)
            RETURNING id
            "#,
            Uuid::nil() // System labels use nil UUID
        )
        .fetch_one(&ctx.db)
        .await
        .unwrap()
        .id;

        // Try to delete system label
        let result = sqlx::query!(
            "DELETE FROM labels WHERE id = $1 AND user_id = $2 AND is_system = FALSE",
            label_id,
            ctx.user_id
        )
        .execute(&ctx.db)
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().rows_affected(), 0); // No rows affected

        // Verify system label still exists
        let system_label = sqlx::query!(
            "SELECT id FROM labels WHERE id = $1",
            label_id
        )
        .fetch_one(&ctx.db)
        .await;

        assert!(system_label.is_ok());
    }

    #[tokio::test]
    async fn test_document_label_assignment() {
        let ctx = setup_test_db().await;

        // Create document
        let document_id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO documents (
                id, user_id, filename, original_filename, file_path, 
                file_size, mime_type, created_at, updated_at
            )
            VALUES ($1, $2, 'test.txt', 'test.txt', '/test/test.txt', 1024, 'text/plain', NOW(), NOW())
            "#,
            document_id,
            ctx.user_id
        )
        .execute(&ctx.db)
        .await
        .expect("Failed to create test document");

        // Create label
        let label_id = sqlx::query!(
            r#"
            INSERT INTO labels (user_id, name, color)
            VALUES ($1, 'Document Label', '#ff0000')
            RETURNING id
            "#,
            ctx.user_id
        )
        .fetch_one(&ctx.db)
        .await
        .unwrap()
        .id;

        // Assign label to document
        let result = sqlx::query!(
            r#"
            INSERT INTO document_labels (document_id, label_id, assigned_by)
            VALUES ($1, $2, $3)
            "#,
            document_id,
            label_id,
            ctx.user_id
        )
        .execute(&ctx.db)
        .await;

        assert!(result.is_ok());

        // Verify assignment
        let assignment = sqlx::query!(
            r#"
            SELECT dl.*, l.name as label_name
            FROM document_labels dl
            JOIN labels l ON dl.label_id = l.id
            WHERE dl.document_id = $1 AND dl.label_id = $2
            "#,
            document_id,
            label_id
        )
        .fetch_one(&ctx.db)
        .await;

        assert!(assignment.is_ok());
        let assignment = assignment.unwrap();
        assert_eq!(assignment.label_name, "Document Label");
        assert_eq!(assignment.assigned_by.unwrap(), ctx.user_id);
    }

    #[tokio::test]
    async fn test_document_label_removal() {
        let ctx = setup_test_db().await;

        // Create document and label
        let document_id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO documents (
                id, user_id, filename, original_filename, file_path, 
                file_size, mime_type, created_at, updated_at
            )
            VALUES ($1, $2, 'test.txt', 'test.txt', '/test/test.txt', 1024, 'text/plain', NOW(), NOW())
            "#,
            document_id,
            ctx.user_id
        )
        .execute(&ctx.db)
        .await
        .expect("Failed to create test document");

        let label_id = sqlx::query!(
            r#"
            INSERT INTO labels (user_id, name, color)
            VALUES ($1, 'Document Label', '#ff0000')
            RETURNING id
            "#,
            ctx.user_id
        )
        .fetch_one(&ctx.db)
        .await
        .unwrap()
        .id;

        // Assign label
        sqlx::query!(
            r#"
            INSERT INTO document_labels (document_id, label_id, assigned_by)
            VALUES ($1, $2, $3)
            "#,
            document_id,
            label_id,
            ctx.user_id
        )
        .execute(&ctx.db)
        .await
        .expect("Failed to assign label");

        // Remove label
        let result = sqlx::query!(
            "DELETE FROM document_labels WHERE document_id = $1 AND label_id = $2",
            document_id,
            label_id
        )
        .execute(&ctx.db)
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().rows_affected(), 1);

        // Verify removal
        let assignment = sqlx::query!(
            "SELECT * FROM document_labels WHERE document_id = $1 AND label_id = $2",
            document_id,
            label_id
        )
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
        sqlx::query!(
            r#"
            INSERT INTO documents (
                id, user_id, filename, original_filename, file_path, 
                file_size, mime_type, created_at, updated_at
            )
            VALUES ($1, $2, 'test.txt', 'test.txt', '/test/test.txt', 1024, 'text/plain', NOW(), NOW())
            "#,
            document_id,
            ctx.user_id
        )
        .execute(&ctx.db)
        .await
        .expect("Failed to create test document");

        // Create multiple labels
        let label_ids: Vec<Uuid> = vec!["Label 1", "Label 2", "Label 3"]
            .into_iter()
            .enumerate()
            .map(|(i, name)| async {
                sqlx::query!(
                    r#"
                    INSERT INTO labels (user_id, name, color)
                    VALUES ($1, $2, $3)
                    RETURNING id
                    "#,
                    ctx.user_id,
                    name,
                    format!("#ff{:02x}00", i * 50)
                )
                .fetch_one(&ctx.db)
                .await
                .unwrap()
                .id
            })
            .collect::<futures::stream::FuturesUnordered<_>>()
            .collect::<Vec<_>>()
            .await;

        // Assign labels to document
        for label_id in &label_ids {
            sqlx::query!(
                r#"
                INSERT INTO document_labels (document_id, label_id, assigned_by)
                VALUES ($1, $2, $3)
                "#,
                document_id,
                label_id,
                ctx.user_id
            )
            .execute(&ctx.db)
            .await
            .expect("Failed to assign label");
        }

        // Get document labels
        let document_labels = sqlx::query!(
            r#"
            SELECT l.id, l.name, l.color, l.icon, l.description, l.is_system
            FROM labels l
            INNER JOIN document_labels dl ON l.id = dl.label_id
            WHERE dl.document_id = $1
            ORDER BY l.name
            "#,
            document_id
        )
        .fetch_all(&ctx.db)
        .await
        .expect("Failed to fetch document labels");

        assert_eq!(document_labels.len(), 3);
        assert_eq!(document_labels[0].name, "Label 1");
        assert_eq!(document_labels[1].name, "Label 2");
        assert_eq!(document_labels[2].name, "Label 3");
    }

    #[tokio::test]
    async fn test_label_usage_counts() {
        let ctx = setup_test_db().await;

        // Create label
        let label_id = sqlx::query!(
            r#"
            INSERT INTO labels (user_id, name, color)
            VALUES ($1, 'Usage Test', '#ff0000')
            RETURNING id
            "#,
            ctx.user_id
        )
        .fetch_one(&ctx.db)
        .await
        .unwrap()
        .id;

        // Create multiple documents
        let mut document_ids = Vec::new();
        for i in 0..3 {
            let doc_id = Uuid::new_v4();
            sqlx::query!(
                r#"
                INSERT INTO documents (
                    id, user_id, filename, original_filename, file_path, 
                    file_size, mime_type, created_at, updated_at
                )
                VALUES ($1, $2, $3, $3, $4, 1024, 'text/plain', NOW(), NOW())
                "#,
                doc_id,
                ctx.user_id,
                format!("test{}.txt", i),
                format!("/test/test{}.txt", i)
            )
            .execute(&ctx.db)
            .await
            .expect("Failed to create test document");
            document_ids.push(doc_id);
        }

        // Assign label to documents
        for doc_id in &document_ids {
            sqlx::query!(
                r#"
                INSERT INTO document_labels (document_id, label_id, assigned_by)
                VALUES ($1, $2, $3)
                "#,
                doc_id,
                label_id,
                ctx.user_id
            )
            .execute(&ctx.db)
            .await
            .expect("Failed to assign label");
        }

        // Get usage count
        let usage_count = sqlx::query!(
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
            label_id
        )
        .fetch_one(&ctx.db)
        .await
        .expect("Failed to get usage count");

        assert_eq!(usage_count.document_count.unwrap(), 3);
    }

    #[tokio::test]
    async fn test_label_color_validation() {
        let ctx = setup_test_db().await;

        // Test valid color
        let valid_result = sqlx::query!(
            r#"
            INSERT INTO labels (user_id, name, color)
            VALUES ($1, 'Valid Color', '#ff0000')
            RETURNING id
            "#,
            ctx.user_id
        )
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
        let system_labels = sqlx::query!(
            "SELECT * FROM labels WHERE is_system = TRUE ORDER BY name"
        )
        .fetch_all(&ctx.db)
        .await
        .expect("Failed to fetch system labels");

        // Verify expected system labels exist
        let expected_labels = vec![
            "Archive", "Financial", "Important", "Legal", 
            "Medical", "Personal", "Receipt", "Work"
        ];

        assert!(system_labels.len() >= expected_labels.len());

        for expected_label in expected_labels {
            assert!(
                system_labels.iter().any(|label| label.name == expected_label),
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
        sqlx::query!(
            r#"
            INSERT INTO documents (
                id, user_id, filename, original_filename, file_path, 
                file_size, mime_type, created_at, updated_at
            )
            VALUES ($1, $2, 'test.txt', 'test.txt', '/test/test.txt', 1024, 'text/plain', NOW(), NOW())
            "#,
            document_id,
            ctx.user_id
        )
        .execute(&ctx.db)
        .await
        .expect("Failed to create test document");

        let label_id = sqlx::query!(
            r#"
            INSERT INTO labels (user_id, name, color)
            VALUES ($1, 'Test Label', '#ff0000')
            RETURNING id
            "#,
            ctx.user_id
        )
        .fetch_one(&ctx.db)
        .await
        .unwrap()
        .id;

        // Assign label to document
        sqlx::query!(
            r#"
            INSERT INTO document_labels (document_id, label_id, assigned_by)
            VALUES ($1, $2, $3)
            "#,
            document_id,
            label_id,
            ctx.user_id
        )
        .execute(&ctx.db)
        .await
        .expect("Failed to assign label");

        // Delete document
        sqlx::query!(
            "DELETE FROM documents WHERE id = $1",
            document_id
        )
        .execute(&ctx.db)
        .await
        .expect("Failed to delete document");

        // Verify document_labels entry was cascade deleted
        let assignments = sqlx::query!(
            "SELECT * FROM document_labels WHERE document_id = $1",
            document_id
        )
        .fetch_all(&ctx.db)
        .await
        .expect("Query failed");

        assert!(assignments.is_empty());

        // Verify label still exists
        let label = sqlx::query!(
            "SELECT * FROM labels WHERE id = $1",
            label_id
        )
        .fetch_one(&ctx.db)
        .await;

        assert!(label.is_ok());
    }
}