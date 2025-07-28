use anyhow::Result;
use chrono::Utc;
use sqlx::Row;
use uuid::Uuid;
use tracing::{info, warn, error};

use super::Database;

impl Database {
    pub async fn create_source(&self, user_id: Uuid, source: &crate::models::CreateSource) -> Result<crate::models::Source> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        
        let row = sqlx::query(
            r#"INSERT INTO sources (id, user_id, name, source_type, enabled, config, status, created_at, updated_at)
               VALUES ($1, $2, $3, $4, $5, $6, 'idle', $7, $8)
               RETURNING *"#
        )
        .bind(id)
        .bind(user_id)
        .bind(&source.name)
        .bind(source.source_type.to_string())
        .bind(source.enabled.unwrap_or(true))
        .bind(&source.config)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(crate::models::Source {
            id: row.get("id"),
            user_id: row.get("user_id"),
            name: row.get("name"),
            source_type: row.get::<String, _>("source_type").try_into().map_err(|e: String| anyhow::anyhow!(e))?,
            enabled: row.get("enabled"),
            config: row.get("config"),
            status: row.get::<String, _>("status").try_into().map_err(|e: String| anyhow::anyhow!(e))?,
            last_sync_at: row.get("last_sync_at"),
            last_error: row.get("last_error"),
            last_error_at: row.get("last_error_at"),
            total_files_synced: row.get("total_files_synced"),
            total_files_pending: row.get("total_files_pending"),
            total_size_bytes: row.get("total_size_bytes"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            validation_status: row.get("validation_status"),
            last_validation_at: row.get("last_validation_at"),
            validation_score: row.get("validation_score"),
            validation_issues: row.get("validation_issues"),
        })
    }

    pub async fn get_source(&self, user_id: Uuid, source_id: Uuid) -> Result<Option<crate::models::Source>> {
        let row = sqlx::query(
            r#"SELECT * FROM sources WHERE id = $1 AND user_id = $2"#
        )
        .bind(source_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(crate::models::Source {
                id: row.get("id"),
                user_id: row.get("user_id"),
                name: row.get("name"),
                source_type: row.get::<String, _>("source_type").try_into().map_err(|e: String| anyhow::anyhow!(e))?,
                enabled: row.get("enabled"),
                config: row.get("config"),
                status: row.get::<String, _>("status").try_into().map_err(|e: String| anyhow::anyhow!(e))?,
                last_sync_at: row.get("last_sync_at"),
                last_error: row.get("last_error"),
                last_error_at: row.get("last_error_at"),
                total_files_synced: row.get("total_files_synced"),
                total_files_pending: row.get("total_files_pending"),
                total_size_bytes: row.get("total_size_bytes"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                validation_status: row.get("validation_status"),
                last_validation_at: row.get("last_validation_at"),
                validation_score: row.get("validation_score"),
                validation_issues: row.get("validation_issues"),
            })),
            None => Ok(None),
        }
    }

    pub async fn get_sources(&self, user_id: Uuid) -> Result<Vec<crate::models::Source>> {
        let rows = sqlx::query(
            r#"SELECT * FROM sources WHERE user_id = $1 ORDER BY created_at DESC"#
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let mut sources = Vec::new();
        for row in rows {
            sources.push(crate::models::Source {
                id: row.get("id"),
                user_id: row.get("user_id"),
                name: row.get("name"),
                source_type: row.get::<String, _>("source_type").try_into().map_err(|e: String| anyhow::anyhow!(e))?,
                enabled: row.get("enabled"),
                config: row.get("config"),
                status: row.get::<String, _>("status").try_into().map_err(|e: String| anyhow::anyhow!(e))?,
                last_sync_at: row.get("last_sync_at"),
                last_error: row.get("last_error"),
                last_error_at: row.get("last_error_at"),
                total_files_synced: row.get("total_files_synced"),
                total_files_pending: row.get("total_files_pending"),
                total_size_bytes: row.get("total_size_bytes"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                validation_status: row.get("validation_status"),
                last_validation_at: row.get("last_validation_at"),
                validation_score: row.get("validation_score"),
                validation_issues: row.get("validation_issues"),
            });
        }

        Ok(sources)
    }

    pub async fn update_source(&self, user_id: Uuid, source_id: Uuid, update: &crate::models::UpdateSource) -> Result<crate::models::Source> {
        let mut query = String::from("UPDATE sources SET updated_at = NOW()");
        let mut bind_count = 0;

        if update.name.is_some() {
            bind_count += 1;
            query.push_str(&format!(", name = ${}", bind_count));
        }
        if update.enabled.is_some() {
            bind_count += 1;
            query.push_str(&format!(", enabled = ${}", bind_count));
        }
        if update.config.is_some() {
            bind_count += 1;
            query.push_str(&format!(", config = ${}", bind_count));
        }

        bind_count += 1;
        query.push_str(&format!(" WHERE id = ${}", bind_count));
        bind_count += 1;
        query.push_str(&format!(" AND user_id = ${} RETURNING *", bind_count));

        let mut query_builder = sqlx::query(&query);

        // Bind values in order
        if let Some(name) = &update.name {
            query_builder = query_builder.bind(name);
        }
        if let Some(enabled) = &update.enabled {
            query_builder = query_builder.bind(enabled);
        }
        if let Some(config) = &update.config {
            query_builder = query_builder.bind(config);
        }
        query_builder = query_builder.bind(source_id);
        query_builder = query_builder.bind(user_id);

        let row = query_builder.fetch_one(&self.pool).await?;

        Ok(crate::models::Source {
            id: row.get("id"),
            user_id: row.get("user_id"),
            name: row.get("name"),
            source_type: row.get::<String, _>("source_type").try_into().map_err(|e: String| anyhow::anyhow!(e))?,
            enabled: row.get("enabled"),
            config: row.get("config"),
            status: row.get::<String, _>("status").try_into().map_err(|e: String| anyhow::anyhow!(e))?,
            last_sync_at: row.get("last_sync_at"),
            last_error: row.get("last_error"),
            last_error_at: row.get("last_error_at"),
            total_files_synced: row.get("total_files_synced"),
            total_files_pending: row.get("total_files_pending"),
            total_size_bytes: row.get("total_size_bytes"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            validation_status: row.get("validation_status"),
            last_validation_at: row.get("last_validation_at"),
            validation_score: row.get("validation_score"),
            validation_issues: row.get("validation_issues"),
        })
    }

    pub async fn delete_source(&self, user_id: Uuid, source_id: Uuid) -> Result<bool> {
        let result = sqlx::query(
            r#"DELETE FROM sources WHERE id = $1 AND user_id = $2"#
        )
        .bind(source_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn update_source_status(&self, source_id: Uuid, status: crate::models::SourceStatus, error: Option<String>) -> Result<()> {
        if let Some(error_msg) = error {
            sqlx::query(
                r#"UPDATE sources 
                   SET status = $1, last_error = $2, last_error_at = NOW(), updated_at = NOW()
                   WHERE id = $3"#
            )
            .bind(status.to_string())
            .bind(error_msg)
            .bind(source_id)
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query(
                r#"UPDATE sources 
                   SET status = $1, updated_at = NOW()
                   WHERE id = $2"#
            )
            .bind(status.to_string())
            .bind(source_id)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    pub async fn update_source_sync_stats(&self, source_id: Uuid, files_synced: i64, files_pending: i64, size_bytes: i64) -> Result<()> {
        sqlx::query(
            r#"UPDATE sources 
               SET total_files_synced = $1, total_files_pending = $2, total_size_bytes = $3, 
                   last_sync_at = NOW(), updated_at = NOW()
               WHERE id = $4"#
        )
        .bind(files_synced)
        .bind(files_pending)
        .bind(size_bytes)
        .bind(source_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // Source management operations
    pub async fn get_all_sources(&self) -> Result<Vec<crate::models::Source>> {
        let rows = sqlx::query(
            r#"SELECT id, user_id, name, source_type, enabled, config, status, 
               last_sync_at, last_error, last_error_at, total_files_synced, 
               total_files_pending, total_size_bytes, created_at, updated_at
               FROM sources ORDER BY created_at DESC"#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut sources = Vec::new();
        for row in rows {
            sources.push(crate::models::Source {
                id: row.get("id"),
                user_id: row.get("user_id"),
                name: row.get("name"),
                source_type: row.get::<String, _>("source_type").try_into()
                    .map_err(|e| anyhow::anyhow!("Invalid source type: {}", e))?,
                enabled: row.get("enabled"),
                config: row.get("config"),
                status: row.get::<String, _>("status").try_into()
                    .map_err(|e| anyhow::anyhow!("Invalid source status: {}", e))?,
                last_sync_at: row.get("last_sync_at"),
                last_error: row.get("last_error"),
                last_error_at: row.get("last_error_at"),
                total_files_synced: row.get("total_files_synced"),
                total_files_pending: row.get("total_files_pending"),
                total_size_bytes: row.get("total_size_bytes"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                validation_status: row.get("validation_status"),
                last_validation_at: row.get("last_validation_at"),
                validation_score: row.get("validation_score"),
                validation_issues: row.get("validation_issues"),
            });
        }

        Ok(sources)
    }

    pub async fn get_sources_for_sync(&self) -> Result<Vec<crate::models::Source>> {
        crate::debug_log!("DB_SOURCES", "🔍 Loading sources from database for sync check...");
        
        let rows = sqlx::query(
            r#"SELECT id, user_id, name, source_type, enabled, config, status, 
               last_sync_at, last_error, last_error_at, total_files_synced, 
               total_files_pending, total_size_bytes, created_at, updated_at,
               validation_status, last_validation_at, validation_score, validation_issues
               FROM sources 
               WHERE enabled = true AND status != 'syncing'
               ORDER BY last_sync_at ASC NULLS FIRST"#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            error!("❌ Failed to load sources from database: {}", e);
            e
        })?;
        
        crate::debug_log!("DB_SOURCES", "📊 Database query returned {} sources for sync processing", rows.len());

        let mut sources = Vec::new();
        for (index, row) in rows.iter().enumerate() {
            let source_id: uuid::Uuid = row.get("id");
            let source_name: String = row.get("name");
            let source_type_str: String = row.get("source_type");
            let config_json: serde_json::Value = row.get("config");
            
            crate::debug_log!("DB_SOURCES", "📋 Processing source {}: ID={}, Name='{}', Type={}", 
                  index + 1, source_id, source_name, source_type_str);
            
            // Log config structure for debugging
            if source_type_str == "WebDAV" {
                if let Some(config_obj) = config_json.as_object() {
                    if let Some(server_url) = config_obj.get("server_url").and_then(|v| v.as_str()) {
                        info!("  🔗 WebDAV server_url: '{}'", server_url);
                    } else {
                        warn!("  ⚠️  WebDAV config missing server_url field");
                    }
                } else {
                    warn!("  ⚠️  WebDAV config is not a JSON object");
                }
                
                // Pretty print the config for debugging
                if let Ok(pretty_config) = serde_json::to_string_pretty(&config_json) {
                    info!("  📄 Full config:\n{}", pretty_config);
                } else {
                    warn!("  ⚠️  Unable to serialize config JSON");
                }
            }
            
            let source = crate::models::Source {
                id: source_id,
                user_id: row.get("user_id"),
                name: source_name.clone(),
                source_type: source_type_str.clone().try_into()
                    .map_err(|e| anyhow::anyhow!("Invalid source type '{}' for source '{}': {}", source_type_str, source_name, e))?,
                enabled: row.get("enabled"),
                config: config_json,
                status: {
                    let status_str: String = row.get("status");
                    status_str.clone().try_into()
                        .map_err(|e| anyhow::anyhow!("Invalid source status '{}' for source '{}': {}", status_str, source_name, e))?
                },
                last_sync_at: row.get("last_sync_at"),
                last_error: row.get("last_error"),
                last_error_at: row.get("last_error_at"),
                total_files_synced: row.get("total_files_synced"),
                total_files_pending: row.get("total_files_pending"),
                total_size_bytes: row.get("total_size_bytes"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                validation_status: row.get("validation_status"),
                last_validation_at: row.get("last_validation_at"),
                validation_score: row.get("validation_score"),
                validation_issues: row.get("validation_issues"),
            };
            
            sources.push(source);
        }

        Ok(sources)
    }

    pub async fn get_source_by_id(&self, source_id: Uuid) -> Result<Option<crate::models::Source>> {
        let row = sqlx::query(
            r#"SELECT id, user_id, name, source_type, enabled, config, status, 
               last_sync_at, last_error, last_error_at, total_files_synced, 
               total_files_pending, total_size_bytes, created_at, updated_at,
               validation_status, last_validation_at, validation_score, validation_issues
               FROM sources WHERE id = $1"#
        )
        .bind(source_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(crate::models::Source {
                id: row.get("id"),
                user_id: row.get("user_id"),
                name: row.get("name"),
                source_type: row.get::<String, _>("source_type").try_into()
                    .map_err(|e| anyhow::anyhow!("Invalid source type: {}", e))?,
                enabled: row.get("enabled"),
                config: row.get("config"),
                status: row.get::<String, _>("status").try_into()
                    .map_err(|e| anyhow::anyhow!("Invalid source status: {}", e))?,
                last_sync_at: row.get("last_sync_at"),
                last_error: row.get("last_error"),
                last_error_at: row.get("last_error_at"),
                total_files_synced: row.get("total_files_synced"),
                total_files_pending: row.get("total_files_pending"),
                total_size_bytes: row.get("total_size_bytes"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                validation_status: row.get("validation_status"),
                last_validation_at: row.get("last_validation_at"),
                validation_score: row.get("validation_score"),
                validation_issues: row.get("validation_issues"),
            }))
        } else {
            Ok(None)
        }
    }

    /// Atomically update source status with optimistic locking to prevent race conditions
    /// This method checks the current status before updating to ensure consistency
    pub async fn update_source_status_atomic(
        &self, 
        source_id: Uuid, 
        expected_current_status: Option<crate::models::SourceStatus>,
        new_status: crate::models::SourceStatus, 
        error_msg: Option<&str>
    ) -> Result<bool> {
        let mut tx = self.pool.begin().await?;
        
        // First, check current status if expected status is provided
        if let Some(expected) = expected_current_status {
            let current_status: Option<String> = sqlx::query_scalar(
                "SELECT status FROM sources WHERE id = $1"
            )
            .bind(source_id)
            .fetch_optional(&mut *tx)
            .await?;
            
            if let Some(current) = current_status {
                let current_status: crate::models::SourceStatus = current.try_into()
                    .map_err(|e: String| anyhow::anyhow!("Invalid status in database: {}", e))?;
                
                if current_status != expected {
                    // Status has changed, abort transaction
                    tx.rollback().await?;
                    return Ok(false);
                }
            } else {
                // Source doesn't exist
                tx.rollback().await?;
                return Ok(false);
            }
        }
        
        // Update the status
        let affected_rows = if let Some(error_msg) = error_msg {
            sqlx::query(
                r#"UPDATE sources 
                   SET status = $1, last_error = $2, last_error_at = NOW(), updated_at = NOW()
                   WHERE id = $3"#
            )
            .bind(new_status.to_string())
            .bind(error_msg)
            .bind(source_id)
            .execute(&mut *tx)
            .await?
            .rows_affected()
        } else {
            sqlx::query(
                r#"UPDATE sources 
                   SET status = $1, last_error = NULL, last_error_at = NULL, updated_at = NOW()
                   WHERE id = $2"#
            )
            .bind(new_status.to_string())
            .bind(source_id)
            .execute(&mut *tx)
            .await?
            .rows_affected()
        };
        
        if affected_rows > 0 {
            tx.commit().await?;
            Ok(true)
        } else {
            tx.rollback().await?;
            Ok(false)
        }
    }

    /// Atomically start a sync operation by checking current status and updating to syncing
    /// Returns true if sync was successfully started, false if already syncing or error
    pub async fn start_sync_atomic(&self, source_id: Uuid) -> Result<bool> {
        let mut tx = self.pool.begin().await?;
        
        // Check current status - only allow starting sync from idle or error states
        let current_status: Option<String> = sqlx::query_scalar(
            "SELECT status FROM sources WHERE id = $1"
        )
        .bind(source_id)
        .fetch_optional(&mut *tx)
        .await?;
        
        if let Some(status_str) = current_status {
            let current_status: crate::models::SourceStatus = status_str.clone().try_into()
                .map_err(|e: String| anyhow::anyhow!("Invalid status in database: {}", e))?;
            
            // Only allow sync to start from idle or error states
            if !matches!(current_status, crate::models::SourceStatus::Idle | crate::models::SourceStatus::Error) {
                tx.rollback().await?;
                return Ok(false);
            }
            
            // Update to syncing status
            let affected_rows = sqlx::query(
                r#"UPDATE sources 
                   SET status = 'syncing', last_error = NULL, last_error_at = NULL, updated_at = NOW()
                   WHERE id = $1 AND status = $2"#
            )
            .bind(source_id)
            .bind(status_str)
            .execute(&mut *tx)
            .await?
            .rows_affected();
            
            if affected_rows > 0 {
                tx.commit().await?;
                Ok(true)
            } else {
                tx.rollback().await?;
                Ok(false)
            }
        } else {
            // Source doesn't exist
            tx.rollback().await?;
            Ok(false)
        }
    }

    /// Atomically complete a sync operation by updating status and stats
    pub async fn complete_sync_atomic(
        &self, 
        source_id: Uuid, 
        success: bool, 
        files_processed: Option<i64>,
        error_msg: Option<&str>
    ) -> Result<bool> {
        let mut tx = self.pool.begin().await?;
        
        // Verify that the source is currently syncing
        let current_status: Option<String> = sqlx::query_scalar(
            "SELECT status FROM sources WHERE id = $1"
        )
        .bind(source_id)
        .fetch_optional(&mut *tx)
        .await?;
        
        if let Some(status_str) = current_status {
            let current_status: crate::models::SourceStatus = status_str.try_into()
                .map_err(|e: String| anyhow::anyhow!("Invalid status in database: {}", e))?;
            
            // Only allow completion if currently syncing
            if current_status != crate::models::SourceStatus::Syncing {
                tx.rollback().await?;
                return Ok(false);
            }
            
            let new_status = if success { "idle" } else { "error" };
            
            // Update status and optionally sync stats
            let affected_rows = if let Some(files) = files_processed {
                if let Some(error_msg) = error_msg {
                    sqlx::query(
                        r#"UPDATE sources 
                           SET status = $1, last_sync_at = NOW(), total_files_synced = total_files_synced + $2,
                               last_error = $3, last_error_at = NOW(), updated_at = NOW()
                           WHERE id = $4"#
                    )
                    .bind(new_status)
                    .bind(files)
                    .bind(error_msg)
                    .bind(source_id)
                    .execute(&mut *tx)
                    .await?
                    .rows_affected()
                } else {
                    sqlx::query(
                        r#"UPDATE sources 
                           SET status = $1, last_sync_at = NOW(), total_files_synced = total_files_synced + $2,
                               last_error = NULL, last_error_at = NULL, updated_at = NOW()
                           WHERE id = $3"#
                    )
                    .bind(new_status)
                    .bind(files)
                    .bind(source_id)
                    .execute(&mut *tx)
                    .await?
                    .rows_affected()
                }
            } else {
                if let Some(error_msg) = error_msg {
                    sqlx::query(
                        r#"UPDATE sources 
                           SET status = $1, last_error = $2, last_error_at = NOW(), updated_at = NOW()
                           WHERE id = $3"#
                    )
                    .bind(new_status)
                    .bind(error_msg)
                    .bind(source_id)
                    .execute(&mut *tx)
                    .await?
                    .rows_affected()
                } else {
                    sqlx::query(
                        r#"UPDATE sources 
                           SET status = $1, last_error = NULL, last_error_at = NULL, updated_at = NOW()
                           WHERE id = $2"#
                    )
                    .bind(new_status)
                    .bind(source_id)
                    .execute(&mut *tx)
                    .await?
                    .rows_affected()
                }
            };
            
            if affected_rows > 0 {
                tx.commit().await?;
                Ok(true)
            } else {
                tx.rollback().await?;
                Ok(false)
            }
        } else {
            // Source doesn't exist
            tx.rollback().await?;
            Ok(false)
        }
    }

    /// Reset stuck syncing sources back to idle (for cleanup during startup)
    pub async fn reset_stuck_syncing_sources(&self) -> Result<u64> {
        let affected_rows = sqlx::query(
            r#"UPDATE sources 
               SET status = 'idle', 
                   last_error = 'Sync was interrupted by server restart', 
                   last_error_at = NOW(), 
                   updated_at = NOW()
               WHERE status = 'syncing'"#
        )
        .execute(&self.pool)
        .await?
        .rows_affected();
        
        if affected_rows > 0 {
            warn!("Reset {} sources that were stuck in syncing state", affected_rows);
        }
        
        Ok(affected_rows)
    }
}