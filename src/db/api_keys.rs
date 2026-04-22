use anyhow::Result;
use uuid::Uuid;

use super::Database;
use crate::models::api_key::ApiKey;

impl Database {
    pub async fn create_api_key(
        &self,
        user_id: Uuid,
        name: &str,
        key_hash: &str,
        key_prefix: &str,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<ApiKey> {
        let key = sqlx::query_as::<_, ApiKey>(
            r#"INSERT INTO api_keys (user_id, name, key_hash, key_prefix, expires_at)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING *"#,
        )
        .bind(user_id)
        .bind(name)
        .bind(key_hash)
        .bind(key_prefix)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(key)
    }

    /// Look up an API key by its SHA-256 hash. Only returns keys that have not
    /// been revoked; expiration is checked by the caller so it can return a
    /// distinct error message.
    pub async fn get_api_key_by_hash(&self, key_hash: &str) -> Result<Option<ApiKey>> {
        let key = sqlx::query_as::<_, ApiKey>(
            r#"SELECT * FROM api_keys WHERE key_hash = $1 AND revoked_at IS NULL"#,
        )
        .bind(key_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(key)
    }

    pub async fn list_api_keys_for_user(&self, user_id: Uuid) -> Result<Vec<ApiKey>> {
        let keys = sqlx::query_as::<_, ApiKey>(
            r#"SELECT * FROM api_keys
               WHERE user_id = $1
               ORDER BY created_at DESC"#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(keys)
    }

    /// Admin-only: list every API key in the system.
    pub async fn list_all_api_keys(&self) -> Result<Vec<ApiKey>> {
        let keys = sqlx::query_as::<_, ApiKey>(
            r#"SELECT * FROM api_keys ORDER BY created_at DESC"#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(keys)
    }

    pub async fn revoke_api_key(&self, key_id: Uuid, user_id: Uuid) -> Result<bool> {
        let result = sqlx::query(
            r#"UPDATE api_keys
               SET revoked_at = NOW()
               WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL"#,
        )
        .bind(key_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Admin-only: revoke any key regardless of owner.
    pub async fn admin_revoke_api_key(&self, key_id: Uuid) -> Result<bool> {
        let result = sqlx::query(
            r#"UPDATE api_keys
               SET revoked_at = NOW()
               WHERE id = $1 AND revoked_at IS NULL"#,
        )
        .bind(key_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Update `last_used_at`, but only if more than 60 seconds have passed since
    /// the last update. This avoids write amplification on hot keys while still
    /// giving users an actionable "last seen" value.
    pub async fn touch_api_key_last_used(&self, key_id: Uuid) -> Result<()> {
        sqlx::query(
            r#"UPDATE api_keys
               SET last_used_at = NOW()
               WHERE id = $1
                 AND (last_used_at IS NULL OR last_used_at < NOW() - INTERVAL '60 seconds')"#,
        )
        .bind(key_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Count active (non-revoked, non-expired) keys for a user. Used to
    /// enforce the max keys-per-user cap at creation time. Expired keys are
    /// excluded so a user with lapsed keys isn't stuck at the cap with zero
    /// usable keys.
    pub async fn count_active_api_keys_for_user(&self, user_id: Uuid) -> Result<i64> {
        let count: (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*) FROM api_keys
               WHERE user_id = $1
                 AND revoked_at IS NULL
                 AND (expires_at IS NULL OR expires_at > NOW())"#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count.0)
    }
}
