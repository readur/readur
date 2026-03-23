use anyhow::Result;
use sqlx::Row;
use uuid::Uuid;

use super::Database;
use crate::models::shared_link::SharedLink;

impl Database {
    pub async fn create_shared_link(
        &self,
        document_id: Uuid,
        created_by: Uuid,
        token: &str,
        password_hash: Option<&str>,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
        max_views: Option<i32>,
    ) -> Result<SharedLink> {
        let link = sqlx::query_as::<_, SharedLink>(
            r#"INSERT INTO shared_links (document_id, created_by, token, password_hash, expires_at, max_views)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING *"#,
        )
        .bind(document_id)
        .bind(created_by)
        .bind(token)
        .bind(password_hash)
        .bind(expires_at)
        .bind(max_views)
        .fetch_one(&self.pool)
        .await?;

        Ok(link)
    }

    pub async fn get_shared_link_by_token(&self, token: &str) -> Result<Option<SharedLink>> {
        let link = sqlx::query_as::<_, SharedLink>(
            r#"SELECT * FROM shared_links WHERE token = $1"#,
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await?;

        Ok(link)
    }

    pub async fn get_shared_links_by_document(
        &self,
        document_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<SharedLink>> {
        let links = sqlx::query_as::<_, SharedLink>(
            r#"SELECT * FROM shared_links
               WHERE document_id = $1 AND created_by = $2
               ORDER BY created_at DESC"#,
        )
        .bind(document_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(links)
    }

    pub async fn get_shared_links_by_user(&self, user_id: Uuid) -> Result<Vec<SharedLink>> {
        let links = sqlx::query_as::<_, SharedLink>(
            r#"SELECT * FROM shared_links
               WHERE created_by = $1
               ORDER BY created_at DESC"#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(links)
    }

    pub async fn revoke_shared_link(&self, link_id: Uuid, user_id: Uuid) -> Result<bool> {
        let result = sqlx::query(
            r#"UPDATE shared_links
               SET is_revoked = TRUE, updated_at = NOW()
               WHERE id = $1 AND created_by = $2 AND is_revoked = FALSE"#,
        )
        .bind(link_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn increment_shared_link_view_count(&self, link_id: Uuid) -> Result<()> {
        sqlx::query(
            r#"UPDATE shared_links
               SET view_count = view_count + 1, updated_at = NOW()
               WHERE id = $1"#,
        )
        .bind(link_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get the document ID for a shared link, used by public access routes.
    /// Returns None if the link doesn't exist (caller checks validity separately).
    pub async fn get_document_id_for_shared_link(&self, token: &str) -> Result<Option<Uuid>> {
        let row = sqlx::query(
            r#"SELECT document_id FROM shared_links WHERE token = $1"#,
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.get("document_id")))
    }
}
