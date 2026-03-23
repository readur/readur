use anyhow::Result;
use sqlx::Row;
use uuid::Uuid;

use super::Database;
use crate::models::comment::{CommentWithAuthor, DocumentComment};

impl Database {
    pub async fn create_comment(
        &self,
        document_id: Uuid,
        user_id: Uuid,
        parent_id: Option<Uuid>,
        content: &str,
    ) -> Result<DocumentComment> {
        let comment = sqlx::query_as::<_, DocumentComment>(
            r#"INSERT INTO document_comments (document_id, user_id, parent_id, content)
               VALUES ($1, $2, $3, $4)
               RETURNING *"#,
        )
        .bind(document_id)
        .bind(user_id)
        .bind(parent_id)
        .bind(content)
        .fetch_one(&self.pool)
        .await?;

        Ok(comment)
    }

    pub async fn get_comments_by_document(
        &self,
        document_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<CommentWithAuthor>> {
        let comments = sqlx::query_as::<_, CommentWithAuthor>(
            r#"SELECT c.*, u.username, u.role as user_role
               FROM document_comments c
               JOIN users u ON c.user_id = u.id
               WHERE c.document_id = $1 AND c.parent_id IS NULL
               ORDER BY c.created_at DESC
               LIMIT $2 OFFSET $3"#,
        )
        .bind(document_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(comments)
    }

    pub async fn get_replies(
        &self,
        parent_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<CommentWithAuthor>> {
        let replies = sqlx::query_as::<_, CommentWithAuthor>(
            r#"SELECT c.*, u.username, u.role as user_role
               FROM document_comments c
               JOIN users u ON c.user_id = u.id
               WHERE c.parent_id = $1
               ORDER BY c.created_at ASC
               LIMIT $2 OFFSET $3"#,
        )
        .bind(parent_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(replies)
    }

    pub async fn get_comment_by_id(&self, comment_id: Uuid) -> Result<Option<DocumentComment>> {
        let comment = sqlx::query_as::<_, DocumentComment>(
            r#"SELECT * FROM document_comments WHERE id = $1"#,
        )
        .bind(comment_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(comment)
    }

    pub async fn update_comment(&self, comment_id: Uuid, content: &str) -> Result<DocumentComment> {
        let comment = sqlx::query_as::<_, DocumentComment>(
            r#"UPDATE document_comments
               SET content = $2, is_edited = TRUE, updated_at = NOW()
               WHERE id = $1
               RETURNING *"#,
        )
        .bind(comment_id)
        .bind(content)
        .fetch_one(&self.pool)
        .await?;

        Ok(comment)
    }

    pub async fn delete_comment(&self, comment_id: Uuid) -> Result<bool> {
        let result = sqlx::query(
            r#"DELETE FROM document_comments WHERE id = $1"#,
        )
        .bind(comment_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn get_comment_count(&self, document_id: Uuid) -> Result<i64> {
        let row = sqlx::query(
            r#"SELECT COUNT(*) as count FROM document_comments WHERE document_id = $1"#,
        )
        .bind(document_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.get("count"))
    }

    pub async fn get_reply_count(&self, parent_id: Uuid) -> Result<i64> {
        let row = sqlx::query(
            r#"SELECT COUNT(*) as count FROM document_comments WHERE parent_id = $1"#,
        )
        .bind(parent_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.get("count"))
    }
}
