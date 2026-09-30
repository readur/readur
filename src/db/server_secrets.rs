//! Secrets the server generates and keeps in the database (`server_secrets`).

use anyhow::Result;

use super::Database;

impl Database {
    /// The stored secret `name`, storing `candidate` first if there is none.
    /// Concurrent callers all get the one value that was stored first. The
    /// flag is true when this call stored `candidate`.
    pub async fn server_value_or_insert(&self, name: &str, candidate: &str) -> Result<(String, bool)> {
        let inserted = sqlx::query(
            "INSERT INTO server_secrets (name, value) VALUES ($1, $2) ON CONFLICT (name) DO NOTHING",
        )
        .bind(name)
        .bind(candidate)
        .execute(&self.pool)
        .await?
        .rows_affected()
            == 1;
        let value: String = sqlx::query_scalar("SELECT value FROM server_secrets WHERE name = $1")
            .bind(name)
            .fetch_one(&self.pool)
            .await?;
        Ok((value, inserted))
    }

    /// Replace (or create) the stored secret `name`.
    pub async fn replace_server_value(&self, name: &str, value: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO server_secrets (name, value) VALUES ($1, $2)
             ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value, created_at = NOW()",
        )
        .bind(name)
        .bind(value)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
