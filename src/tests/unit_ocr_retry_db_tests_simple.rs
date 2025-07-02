#[cfg(test)]
mod tests {
    use crate::db::ocr_retry::*;
    use sqlx::{PgPool, Row};
    use testcontainers::{runners::AsyncRunner, ContainerAsync};
    use testcontainers_modules::postgres::Postgres;
    use uuid::Uuid;

    async fn setup_test_db() -> (ContainerAsync<Postgres>, PgPool) {
        let postgres_image = Postgres::default();
        let container = postgres_image.start().await.expect("Failed to start postgres container");
        let port = container.get_host_port_ipv4(5432).await.expect("Failed to get postgres port");
        
        let connection_string = format!(
            "postgres://postgres:postgres@127.0.0.1:{}/postgres",
            port
        );

        let pool = PgPool::connect(&connection_string).await.expect("Failed to connect to test database");
        sqlx::migrate!("./migrations").run(&pool).await.expect("Failed to run migrations");
        
        (container, pool)
    }

    #[tokio::test]
    async fn test_simple_retry_record() {
        let (_container, pool) = setup_test_db().await;
        
        // Create a simple test document entry first
        let doc_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        
        sqlx::query("INSERT INTO users (id, username, email, password_hash) VALUES ($1, 'test', 'test@test.com', 'test')")
            .bind(user_id)
            .execute(&pool)
            .await
            .expect("Failed to create test user");
            
        sqlx::query("INSERT INTO documents (id, filename, original_filename, user_id, mime_type, file_size, created_at, updated_at) VALUES ($1, 'test.pdf', 'test.pdf', $2, 'application/pdf', 1024, NOW(), NOW())")
            .bind(doc_id)
            .bind(user_id)
            .execute(&pool)
            .await
            .expect("Failed to create test document");
        
        // Test the record_ocr_retry function
        let retry_id = record_ocr_retry(
            &pool,
            doc_id,
            user_id,
            "manual_retry",
            10,
            None,
        ).await.expect("Failed to record retry");
        
        // Verify the retry was recorded
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ocr_retry_history WHERE id = $1")
            .bind(retry_id)
            .fetch_one(&pool)
            .await
            .expect("Failed to count retries");
            
        assert_eq!(count, 1);
    }
}