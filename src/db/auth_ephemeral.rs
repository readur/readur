//! Short-lived, single-use values used during login flows.
//!
//! Callers pass the plaintext lookup key; only its SHA-256 is stored, so a
//! database read alone cannot replay a pending login.

use anyhow::Result;
use chrono::{Duration, Utc};

use super::Database;
use crate::auth::sha256_hex;

impl Database {
    /// Store `payload` under `key` for `ttl`. Also opportunistically purges
    /// expired rows so the table cannot grow without bound.
    pub async fn put_auth_ephemeral(
        &self,
        key: &str,
        kind: &str,
        payload: &serde_json::Value,
        ttl: Duration,
    ) -> Result<()> {
        sqlx::query("DELETE FROM auth_ephemeral WHERE expires_at < NOW()")
            .execute(&self.pool)
            .await?;

        sqlx::query(
            "INSERT INTO auth_ephemeral (key_hash, kind, payload, expires_at)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(sha256_hex(key))
        .bind(kind)
        .bind(payload)
        .bind(Utc::now() + ttl)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Atomically consume the value stored under `key`. Returns `None` if it
    /// does not exist, has a different kind, or has expired. A value can be
    /// taken at most once.
    pub async fn take_auth_ephemeral(&self, key: &str, kind: &str) -> Result<Option<serde_json::Value>> {
        // Delete unconditionally (so an expired row is also removed), then
        // decide validity from the returned expiry.
        let row = sqlx::query_as::<_, (serde_json::Value, chrono::DateTime<Utc>)>(
            "DELETE FROM auth_ephemeral
             WHERE key_hash = $1 AND kind = $2
             RETURNING payload, expires_at",
        )
        .bind(sha256_hex(key))
        .bind(kind)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.and_then(|(payload, expires_at)| (expires_at > Utc::now()).then_some(payload)))
    }
}
