#[cfg(test)]
mod tests {
    use anyhow::Result;
    use readur::db::ocr_retry::*;
    use readur::test_utils::{TestContext, TestAuthHelper};
    use sqlx::Row;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_simple_retry_record() {
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let auth_helper = TestAuthHelper::new(ctx.app.clone());
            let user = auth_helper.create_test_user().await;

            // Create a test document using the TestContext database
            let doc_id = Uuid::new_v4();
            sqlx::query("INSERT INTO documents (id, filename, original_filename, user_id, mime_type, file_size, created_at, updated_at, file_path) VALUES ($1, 'test.pdf', 'test.pdf', $2, 'application/pdf', 1024, NOW(), NOW(), '/test/test.pdf')")
                .bind(doc_id)
                .bind(user.user_response.id)
                .execute(&ctx.state.db.pool)
                .await
                .expect("Failed to create test document");

            // Test the record_ocr_retry function
            let retry_id = record_ocr_retry(
                &ctx.state.db.pool,
                doc_id,
                user.user_response.id,
                "manual_retry",
                10,
                None,
            ).await.expect("Failed to record retry");

            // Verify the retry was recorded
            let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ocr_retry_history WHERE id = $1")
                .bind(retry_id)
                .fetch_one(&ctx.state.db.pool)
                .await
                .expect("Failed to count retries");

            assert_eq!(count, 1);
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }
}