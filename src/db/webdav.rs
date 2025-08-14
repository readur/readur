use anyhow::Result;
use sqlx::Row;
use uuid::Uuid;

use super::Database;

impl Database {
    pub async fn get_webdav_sync_state(&self, user_id: Uuid) -> Result<Option<crate::models::WebDAVSyncState>> {
        self.with_retry(|| async {
            let row = sqlx::query(
                r#"SELECT id, user_id, last_sync_at, sync_cursor, is_running, files_processed, 
                   files_remaining, current_folder, errors, created_at, updated_at
                   FROM webdav_sync_state WHERE user_id = $1"#
            )
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("Database query failed: {}", e))?;

        match row {
            Some(row) => Ok(Some(crate::models::WebDAVSyncState {
                id: row.get("id"),
                user_id: row.get("user_id"),
                last_sync_at: row.get("last_sync_at"),
                sync_cursor: row.get("sync_cursor"),
                is_running: row.get("is_running"),
                files_processed: row.get("files_processed"),
                files_remaining: row.get("files_remaining"),
                current_folder: row.get("current_folder"),
                errors: row.get("errors"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })),
            None => Ok(None),
        }
        }).await
    }

    pub async fn update_webdav_sync_state(&self, user_id: Uuid, state: &crate::models::UpdateWebDAVSyncState) -> Result<()> {
        self.with_retry(|| async {
            sqlx::query(
                r#"INSERT INTO webdav_sync_state (user_id, last_sync_at, sync_cursor, is_running, 
                   files_processed, files_remaining, current_folder, errors, updated_at)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
                   ON CONFLICT (user_id) DO UPDATE SET
                   last_sync_at = EXCLUDED.last_sync_at,
                   sync_cursor = EXCLUDED.sync_cursor,
                   is_running = EXCLUDED.is_running,
                   files_processed = EXCLUDED.files_processed,
                   files_remaining = EXCLUDED.files_remaining,
                   current_folder = EXCLUDED.current_folder,
                   errors = EXCLUDED.errors,
                   updated_at = NOW()"#
            )
            .bind(user_id)
            .bind(state.last_sync_at)
            .bind(&state.sync_cursor)
            .bind(state.is_running)
            .bind(state.files_processed)
            .bind(state.files_remaining)
            .bind(&state.current_folder)
            .bind(&state.errors)
            .execute(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("Database update failed: {}", e))?;

            Ok(())
        }).await
    }

    // Reset any running WebDAV syncs on startup (handles server restart during sync)
    pub async fn reset_running_webdav_syncs(&self) -> Result<i64> {
        let result = sqlx::query(
            r#"UPDATE webdav_sync_state 
               SET is_running = false, 
                   current_folder = NULL,
                   errors = CASE 
                       WHEN array_length(errors, 1) IS NULL OR array_length(errors, 1) = 0 
                       THEN ARRAY['Sync interrupted by server restart']
                       ELSE array_append(errors, 'Sync interrupted by server restart')
                   END,
                   updated_at = NOW()
               WHERE is_running = true"#
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as i64)
    }

    // Reset any running source syncs on startup (handles server restart during sync)
    pub async fn reset_running_source_syncs(&self) -> Result<i64> {
        let result = sqlx::query(
            r#"UPDATE sources 
               SET status = 'idle',
                   last_error = CASE 
                       WHEN last_error IS NULL OR last_error = ''
                       THEN 'Sync interrupted by server restart'
                       ELSE last_error || '; Sync interrupted by server restart'
                   END,
                   last_error_at = NOW(),
                   updated_at = NOW()
               WHERE status = 'syncing'"#
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as i64)
    }

    // WebDAV file tracking operations
    pub async fn get_webdav_file_by_path(&self, user_id: Uuid, webdav_path: &str) -> Result<Option<crate::models::WebDAVFile>> {
        let row = sqlx::query(
            r#"SELECT id, user_id, webdav_path, etag, last_modified, file_size, 
               mime_type, document_id, sync_status, sync_error, created_at, updated_at
               FROM webdav_files WHERE user_id = $1 AND webdav_path = $2"#
        )
        .bind(user_id)
        .bind(webdav_path)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(crate::models::WebDAVFile {
                id: row.get("id"),
                user_id: row.get("user_id"),
                webdav_path: row.get("webdav_path"),
                etag: row.get("etag"),
                last_modified: row.get("last_modified"),
                file_size: row.get("file_size"),
                mime_type: row.get("mime_type"),
                document_id: row.get("document_id"),
                sync_status: row.get("sync_status"),
                sync_error: row.get("sync_error"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })),
            None => Ok(None),
        }
    }

    pub async fn create_or_update_webdav_file(&self, file: &crate::models::CreateWebDAVFile) -> Result<crate::models::WebDAVFile> {
        let row = sqlx::query(
            r#"INSERT INTO webdav_files (user_id, webdav_path, etag, last_modified, file_size, 
               mime_type, document_id, sync_status, sync_error)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
               ON CONFLICT (user_id, webdav_path) DO UPDATE SET
               etag = EXCLUDED.etag,
               last_modified = EXCLUDED.last_modified,
               file_size = EXCLUDED.file_size,
               mime_type = EXCLUDED.mime_type,
               document_id = EXCLUDED.document_id,
               sync_status = EXCLUDED.sync_status,
               sync_error = EXCLUDED.sync_error,
               updated_at = NOW()
               RETURNING id, user_id, webdav_path, etag, last_modified, file_size, 
               mime_type, document_id, sync_status, sync_error, created_at, updated_at"#
        )
        .bind(file.user_id)
        .bind(&file.webdav_path)
        .bind(&file.etag)
        .bind(file.last_modified)
        .bind(file.file_size)
        .bind(&file.mime_type)
        .bind(file.document_id)
        .bind(&file.sync_status)
        .bind(&file.sync_error)
        .fetch_one(&self.pool)
        .await?;

        Ok(crate::models::WebDAVFile {
            id: row.get("id"),
            user_id: row.get("user_id"),
            webdav_path: row.get("webdav_path"),
            etag: row.get("etag"),
            last_modified: row.get("last_modified"),
            file_size: row.get("file_size"),
            mime_type: row.get("mime_type"),
            document_id: row.get("document_id"),
            sync_status: row.get("sync_status"),
            sync_error: row.get("sync_error"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    pub async fn get_pending_webdav_files(&self, user_id: Uuid, limit: i64) -> Result<Vec<crate::models::WebDAVFile>> {
        let rows = sqlx::query(
            r#"SELECT id, user_id, webdav_path, etag, last_modified, file_size, 
               mime_type, document_id, sync_status, sync_error, created_at, updated_at
               FROM webdav_files 
               WHERE user_id = $1 AND sync_status = 'pending'
               ORDER BY created_at ASC
               LIMIT $2"#
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut files = Vec::new();
        for row in rows {
            files.push(crate::models::WebDAVFile {
                id: row.get("id"),
                user_id: row.get("user_id"),
                webdav_path: row.get("webdav_path"),
                etag: row.get("etag"),
                last_modified: row.get("last_modified"),
                file_size: row.get("file_size"),
                mime_type: row.get("mime_type"),
                document_id: row.get("document_id"),
                sync_status: row.get("sync_status"),
                sync_error: row.get("sync_error"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(files)
    }

    // Directory tracking functions for efficient sync optimization
    pub async fn get_webdav_directory(&self, user_id: Uuid, directory_path: &str) -> Result<Option<crate::models::WebDAVDirectory>> {
        self.with_retry(|| async {
            let row = sqlx::query(
                r#"SELECT id, user_id, directory_path, directory_etag, last_scanned_at, 
                   file_count, total_size_bytes, created_at, updated_at
                   FROM webdav_directories WHERE user_id = $1 AND directory_path = $2"#
            )
            .bind(user_id)
            .bind(directory_path)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("Database query failed: {}", e))?;

            match row {
                Some(row) => Ok(Some(crate::models::WebDAVDirectory {
                    id: row.get("id"),
                    user_id: row.get("user_id"),
                    directory_path: row.get("directory_path"),
                    directory_etag: row.get("directory_etag"),
                    last_scanned_at: row.get("last_scanned_at"),
                    file_count: row.get("file_count"),
                    total_size_bytes: row.get("total_size_bytes"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                })),
                None => Ok(None),
            }
        }).await
    }

    pub async fn create_or_update_webdav_directory(&self, directory: &crate::models::CreateWebDAVDirectory) -> Result<crate::models::WebDAVDirectory> {
        let row = sqlx::query(
            r#"INSERT INTO webdav_directories (user_id, directory_path, directory_etag, 
               file_count, total_size_bytes, last_scanned_at, updated_at)
               VALUES ($1, $2, $3, $4, $5, NOW(), NOW())
               ON CONFLICT (user_id, directory_path) DO UPDATE SET
               directory_etag = EXCLUDED.directory_etag,
               file_count = EXCLUDED.file_count,
               total_size_bytes = EXCLUDED.total_size_bytes,
               last_scanned_at = NOW(),
               updated_at = NOW()
               RETURNING id, user_id, directory_path, directory_etag, last_scanned_at,
               file_count, total_size_bytes, created_at, updated_at"#
        )
        .bind(directory.user_id)
        .bind(&directory.directory_path)
        .bind(&directory.directory_etag)
        .bind(directory.file_count)
        .bind(directory.total_size_bytes)
        .fetch_one(&self.pool)
        .await?;

        Ok(crate::models::WebDAVDirectory {
            id: row.get("id"),
            user_id: row.get("user_id"),
            directory_path: row.get("directory_path"),
            directory_etag: row.get("directory_etag"),
            last_scanned_at: row.get("last_scanned_at"),
            file_count: row.get("file_count"),
            total_size_bytes: row.get("total_size_bytes"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    pub async fn update_webdav_directory(&self, user_id: Uuid, directory_path: &str, update: &crate::models::UpdateWebDAVDirectory) -> Result<()> {
        self.with_retry(|| async {
            sqlx::query(
                r#"UPDATE webdav_directories SET 
                   directory_etag = $3,
                   last_scanned_at = $4,
                   file_count = $5,
                   total_size_bytes = $6,
                   updated_at = NOW()
                   WHERE user_id = $1 AND directory_path = $2"#
            )
            .bind(user_id)
            .bind(directory_path)
            .bind(&update.directory_etag)
            .bind(update.last_scanned_at)
            .bind(update.file_count)
            .bind(update.total_size_bytes)
            .execute(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("Database update failed: {}", e))?;

            Ok(())
        }).await
    }

    pub async fn list_webdav_directories(&self, user_id: Uuid) -> Result<Vec<crate::models::WebDAVDirectory>> {
        let rows = sqlx::query(
            r#"SELECT id, user_id, directory_path, directory_etag, last_scanned_at,
               file_count, total_size_bytes, created_at, updated_at
               FROM webdav_directories 
               WHERE user_id = $1
               ORDER BY directory_path ASC"#
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let mut directories = Vec::new();
        for row in rows {
            directories.push(crate::models::WebDAVDirectory {
                id: row.get("id"),
                user_id: row.get("user_id"),
                directory_path: row.get("directory_path"),
                directory_etag: row.get("directory_etag"),
                last_scanned_at: row.get("last_scanned_at"),
                file_count: row.get("file_count"),
                total_size_bytes: row.get("total_size_bytes"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(directories)
    }

    /// Clear all WebDAV directory tracking for a user (used for deep scan)
    pub async fn clear_webdav_directories(&self, user_id: Uuid) -> Result<i64> {
        let result = sqlx::query(
            r#"DELETE FROM webdav_directories WHERE user_id = $1"#
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as i64)
    }

    /// Delete a specific WebDAV directory by path
    pub async fn delete_webdav_directory(&self, user_id: Uuid, directory_path: &str) -> Result<bool> {
        let result = sqlx::query(
            r#"DELETE FROM webdav_directories WHERE user_id = $1 AND directory_path = $2"#
        )
        .bind(user_id)
        .bind(directory_path)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Find directories with incomplete scans that need recovery
    pub async fn get_incomplete_webdav_scans(&self, user_id: Uuid) -> Result<Vec<String>> {
        let rows = sqlx::query(
            r#"SELECT directory_path FROM webdav_directories 
               WHERE user_id = $1 AND scan_in_progress = TRUE
               ORDER BY scan_started_at ASC"#
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|row| row.get("directory_path")).collect())
    }

    /// Find scans that have been running too long (possible crashes)
    pub async fn get_stale_webdav_scans(&self, user_id: Uuid, timeout_minutes: i64) -> Result<Vec<String>> {
        let rows = sqlx::query(
            r#"SELECT directory_path FROM webdav_directories 
               WHERE user_id = $1 AND scan_in_progress = TRUE 
               AND scan_started_at < NOW() - INTERVAL '1 minute' * $2
               ORDER BY scan_started_at ASC"#
        )
        .bind(user_id)
        .bind(timeout_minutes)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|row| row.get("directory_path")).collect())
    }

    /// Mark a directory scan as in progress
    pub async fn mark_webdav_scan_in_progress(&self, user_id: Uuid, directory_path: &str) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO webdav_directories (user_id, directory_path, directory_etag, scan_in_progress, scan_started_at, last_scanned_at)
               VALUES ($1, $2, '', TRUE, NOW(), NOW())
               ON CONFLICT (user_id, directory_path)
               DO UPDATE SET scan_in_progress = TRUE, scan_started_at = NOW(), scan_error = NULL"#
        )
        .bind(user_id)
        .bind(directory_path)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Mark a directory scan as complete
    pub async fn mark_webdav_scan_complete(&self, user_id: Uuid, directory_path: &str) -> Result<()> {
        sqlx::query(
            r#"UPDATE webdav_directories 
               SET scan_in_progress = FALSE, scan_started_at = NULL, scan_error = NULL,
                   last_scanned_at = NOW(), updated_at = NOW()
               WHERE user_id = $1 AND directory_path = $2"#
        )
        .bind(user_id)
        .bind(directory_path)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Mark a directory scan as failed with error
    pub async fn mark_webdav_scan_failed(&self, user_id: Uuid, directory_path: &str, error: &str) -> Result<()> {
        sqlx::query(
            r#"UPDATE webdav_directories 
               SET scan_in_progress = FALSE, scan_error = $3, updated_at = NOW()
               WHERE user_id = $1 AND directory_path = $2"#
        )
        .bind(user_id)
        .bind(directory_path)
        .bind(error)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Bulk create or update WebDAV directories in a single transaction
    /// This ensures atomic updates and prevents race conditions during directory sync
    pub async fn bulk_create_or_update_webdav_directories(&self, directories: &[crate::models::CreateWebDAVDirectory]) -> Result<Vec<crate::models::WebDAVDirectory>> {
        if directories.is_empty() {
            return Ok(Vec::new());
        }

        let mut tx = self.pool.begin().await?;
        let mut results = Vec::new();

        for directory in directories {
            let row = sqlx::query(
                r#"INSERT INTO webdav_directories (user_id, directory_path, directory_etag, 
                   file_count, total_size_bytes, last_scanned_at, updated_at)
                   VALUES ($1, $2, $3, $4, $5, NOW(), NOW())
                   ON CONFLICT (user_id, directory_path) DO UPDATE SET
                   directory_etag = EXCLUDED.directory_etag,
                   file_count = EXCLUDED.file_count,
                   total_size_bytes = EXCLUDED.total_size_bytes,
                   last_scanned_at = NOW(),
                   updated_at = NOW()
                   RETURNING id, user_id, directory_path, directory_etag, last_scanned_at,
                   file_count, total_size_bytes, created_at, updated_at"#
            )
            .bind(directory.user_id)
            .bind(&directory.directory_path)
            .bind(&directory.directory_etag)
            .bind(directory.file_count)
            .bind(directory.total_size_bytes)
            .fetch_one(&mut *tx)
            .await?;

            results.push(crate::models::WebDAVDirectory {
                id: row.get("id"),
                user_id: row.get("user_id"),
                directory_path: row.get("directory_path"),
                directory_etag: row.get("directory_etag"),
                last_scanned_at: row.get("last_scanned_at"),
                file_count: row.get("file_count"),
                total_size_bytes: row.get("total_size_bytes"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        tx.commit().await?;
        Ok(results)
    }

    /// Delete directories that no longer exist on the WebDAV server
    /// Returns the number of directories deleted
    pub async fn delete_missing_webdav_directories(&self, user_id: Uuid, existing_paths: &[String]) -> Result<i64> {
        if existing_paths.is_empty() {
            // If no directories exist, delete all for this user
            return self.clear_webdav_directories(user_id).await;
        }

        // Build the NOT IN clause with placeholders
        let placeholders = (0..existing_paths.len())
            .map(|i| format!("${}", i + 2))
            .collect::<Vec<_>>()
            .join(",");

        let query = format!(
            r#"DELETE FROM webdav_directories 
               WHERE user_id = $1 AND directory_path NOT IN ({})"#,
            placeholders
        );

        let mut query_builder = sqlx::query(&query);
        query_builder = query_builder.bind(user_id);
        
        for path in existing_paths {
            query_builder = query_builder.bind(path);
        }

        let result = query_builder.execute(&self.pool).await?;
        Ok(result.rows_affected() as i64)
    }

    /// Perform a complete atomic sync of directory state
    /// This combines creation/updates and deletion in a single transaction
    pub async fn sync_webdav_directories(
        &self, 
        user_id: Uuid, 
        discovered_directories: &[crate::models::CreateWebDAVDirectory]
    ) -> Result<(Vec<crate::models::WebDAVDirectory>, i64)> {
        let mut tx = self.pool.begin().await?;
        let mut updated_directories = Vec::new();

        // First, update/create all discovered directories
        for directory in discovered_directories {
            let row = sqlx::query(
                r#"INSERT INTO webdav_directories (user_id, directory_path, directory_etag, 
                   file_count, total_size_bytes, last_scanned_at, updated_at)
                   VALUES ($1, $2, $3, $4, $5, NOW(), NOW())
                   ON CONFLICT (user_id, directory_path) DO UPDATE SET
                   directory_etag = EXCLUDED.directory_etag,
                   file_count = EXCLUDED.file_count,
                   total_size_bytes = EXCLUDED.total_size_bytes,
                   last_scanned_at = NOW(),
                   updated_at = NOW()
                   RETURNING id, user_id, directory_path, directory_etag, last_scanned_at,
                   file_count, total_size_bytes, created_at, updated_at"#
            )
            .bind(directory.user_id)
            .bind(&directory.directory_path)
            .bind(&directory.directory_etag)
            .bind(directory.file_count)
            .bind(directory.total_size_bytes)
            .fetch_one(&mut *tx)
            .await?;

            updated_directories.push(crate::models::WebDAVDirectory {
                id: row.get("id"),
                user_id: row.get("user_id"),
                directory_path: row.get("directory_path"),
                directory_etag: row.get("directory_etag"),
                last_scanned_at: row.get("last_scanned_at"),
                file_count: row.get("file_count"),
                total_size_bytes: row.get("total_size_bytes"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        // Then, delete directories that are no longer present
        let discovered_paths: Vec<String> = discovered_directories
            .iter()
            .map(|d| d.directory_path.clone())
            .collect();

        let deleted_count = if discovered_paths.is_empty() {
            // If no directories discovered, delete all for this user
            let result = sqlx::query(
                r#"DELETE FROM webdav_directories WHERE user_id = $1"#
            )
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
            result.rows_affected() as i64
        } else {
            // Build the NOT IN clause
            let placeholders = (0..discovered_paths.len())
                .map(|i| format!("${}", i + 2))
                .collect::<Vec<_>>()
                .join(",");

            let query = format!(
                r#"DELETE FROM webdav_directories 
                   WHERE user_id = $1 AND directory_path NOT IN ({})"#,
                placeholders
            );

            let mut query_builder = sqlx::query(&query);
            query_builder = query_builder.bind(user_id);
            
            for path in &discovered_paths {
                query_builder = query_builder.bind(path);
            }

            let result = query_builder.execute(&mut *tx).await?;
            result.rows_affected() as i64
        };

        tx.commit().await?;
        Ok((updated_directories, deleted_count))
    }

    // ===== WebDAV Scan Failure Tracking Methods =====
    
    /// Record a new scan failure or increment existing failure count
    pub async fn record_scan_failure(&self, failure: &crate::models::CreateWebDAVScanFailure) -> Result<Uuid> {
        let failure_type_str = failure.failure_type.to_string();
        
        // Classify the error to determine appropriate failure type and severity
        let (failure_type, severity) = self.classify_scan_error(&failure);
        
        let mut diagnostic_data = failure.diagnostic_data.clone().unwrap_or(serde_json::json!({}));
        
        // Add additional diagnostic information
        if let Some(data) = diagnostic_data.as_object_mut() {
            data.insert("path_length".to_string(), serde_json::json!(failure.directory_path.len()));
            data.insert("directory_depth".to_string(), serde_json::json!(failure.directory_path.matches('/').count()));
            
            if let Some(server_type) = &failure.server_type {
                data.insert("server_type".to_string(), serde_json::json!(server_type));
            }
            if let Some(server_version) = &failure.server_version {
                data.insert("server_version".to_string(), serde_json::json!(server_version));
            }
        }
        
        let row = sqlx::query(
            r#"SELECT record_webdav_scan_failure($1, $2, $3, $4, $5, $6, $7, $8, $9) as failure_id"#
        )
        .bind(failure.user_id)
        .bind(&failure.directory_path)
        .bind(failure_type_str)
        .bind(&failure.error_message)
        .bind(&failure.error_code)
        .bind(failure.http_status_code)
        .bind(failure.response_time_ms)
        .bind(failure.response_size_bytes)
        .bind(&diagnostic_data)
        .fetch_one(&self.pool)
        .await?;
        
        Ok(row.get("failure_id"))
    }
    
    /// Get all scan failures for a user
    pub async fn get_scan_failures(&self, user_id: Uuid, include_resolved: bool) -> Result<Vec<crate::models::WebDAVScanFailure>> {
        let query = if include_resolved {
            r#"SELECT * FROM webdav_scan_failures 
               WHERE user_id = $1 
               ORDER BY last_failure_at DESC"#
        } else {
            r#"SELECT * FROM webdav_scan_failures 
               WHERE user_id = $1 AND NOT resolved AND NOT user_excluded
               ORDER BY failure_severity DESC, last_failure_at DESC"#
        };
        
        let rows = sqlx::query_as::<_, crate::models::WebDAVScanFailure>(query)
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;
        
        Ok(rows)
    }
    
    /// Get failure count for a specific directory
    pub async fn get_failure_count(&self, user_id: Uuid, directory_path: &str) -> Result<Option<i32>> {
        let row = sqlx::query(
            r#"SELECT failure_count FROM webdav_scan_failures 
               WHERE user_id = $1 AND directory_path = $2"#
        )
        .bind(user_id)
        .bind(directory_path)
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(row.map(|r| r.get("failure_count")))
    }
    
    /// Check if a directory is a known failure that should be skipped
    pub async fn is_known_failure(&self, user_id: Uuid, directory_path: &str) -> Result<bool> {
        let row = sqlx::query(
            r#"SELECT 1 FROM webdav_scan_failures 
               WHERE user_id = $1 AND directory_path = $2 
               AND NOT resolved 
               AND (user_excluded = TRUE OR 
                    (failure_severity IN ('critical', 'high') AND failure_count > 3) OR
                    (next_retry_at IS NULL OR next_retry_at > NOW()))"#
        )
        .bind(user_id)
        .bind(directory_path)
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(row.is_some())
    }
    
    /// Get directories ready for retry
    pub async fn get_directories_ready_for_retry(&self, user_id: Uuid) -> Result<Vec<String>> {
        let rows = sqlx::query(
            r#"SELECT directory_path FROM webdav_scan_failures 
               WHERE user_id = $1 
               AND NOT resolved 
               AND NOT user_excluded
               AND next_retry_at <= NOW()
               AND failure_count < max_retries
               ORDER BY failure_severity ASC, next_retry_at ASC
               LIMIT 10"#
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        
        Ok(rows.into_iter().map(|row| row.get("directory_path")).collect())
    }
    
    /// Reset a failure for retry
    pub async fn reset_scan_failure(&self, user_id: Uuid, directory_path: &str) -> Result<bool> {
        let row = sqlx::query(
            r#"SELECT reset_webdav_scan_failure($1, $2) as success"#
        )
        .bind(user_id)
        .bind(directory_path)
        .fetch_one(&self.pool)
        .await?;
        
        Ok(row.get("success"))
    }
    
    /// Mark a failure as resolved
    pub async fn resolve_scan_failure(&self, user_id: Uuid, directory_path: &str, resolution_method: &str) -> Result<bool> {
        let row = sqlx::query(
            r#"SELECT resolve_webdav_scan_failure($1, $2, $3) as success"#
        )
        .bind(user_id)
        .bind(directory_path)
        .bind(resolution_method)
        .fetch_one(&self.pool)
        .await?;
        
        Ok(row.get("success"))
    }
    
    /// Mark a directory as permanently excluded by user
    pub async fn exclude_directory_from_scan(&self, user_id: Uuid, directory_path: &str, user_notes: Option<&str>) -> Result<()> {
        sqlx::query(
            r#"UPDATE webdav_scan_failures 
               SET user_excluded = TRUE,
                   user_notes = COALESCE($3, user_notes),
                   updated_at = NOW()
               WHERE user_id = $1 AND directory_path = $2"#
        )
        .bind(user_id)
        .bind(directory_path)
        .bind(user_notes)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    /// Get scan failure statistics for a user
    pub async fn get_scan_failure_stats(&self, user_id: Uuid) -> Result<serde_json::Value> {
        let row = sqlx::query(
            r#"SELECT 
                COUNT(*) FILTER (WHERE NOT resolved) as active_failures,
                COUNT(*) FILTER (WHERE resolved) as resolved_failures,
                COUNT(*) FILTER (WHERE user_excluded) as excluded_directories,
                COUNT(*) FILTER (WHERE failure_severity = 'critical' AND NOT resolved) as critical_failures,
                COUNT(*) FILTER (WHERE failure_severity = 'high' AND NOT resolved) as high_failures,
                COUNT(*) FILTER (WHERE failure_severity = 'medium' AND NOT resolved) as medium_failures,
                COUNT(*) FILTER (WHERE failure_severity = 'low' AND NOT resolved) as low_failures,
                COUNT(*) FILTER (WHERE next_retry_at <= NOW() AND NOT resolved AND NOT user_excluded) as ready_for_retry
               FROM webdav_scan_failures
               WHERE user_id = $1"#
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        
        Ok(serde_json::json!({
            "active_failures": row.get::<i64, _>("active_failures"),
            "resolved_failures": row.get::<i64, _>("resolved_failures"),
            "excluded_directories": row.get::<i64, _>("excluded_directories"),
            "critical_failures": row.get::<i64, _>("critical_failures"),
            "high_failures": row.get::<i64, _>("high_failures"),
            "medium_failures": row.get::<i64, _>("medium_failures"),
            "low_failures": row.get::<i64, _>("low_failures"),
            "ready_for_retry": row.get::<i64, _>("ready_for_retry"),
        }))
    }
    
    /// Helper function to classify scan errors
    fn classify_scan_error(&self, failure: &crate::models::CreateWebDAVScanFailure) -> (String, String) {
        use crate::models::WebDAVScanFailureType;
        
        let failure_type = &failure.failure_type;
        let error_msg = failure.error_message.to_lowercase();
        let status_code = failure.http_status_code;
        
        // Determine severity based on error characteristics
        let severity = match failure_type {
            WebDAVScanFailureType::PathTooLong | 
            WebDAVScanFailureType::InvalidCharacters => "critical",
            
            WebDAVScanFailureType::PermissionDenied |
            WebDAVScanFailureType::XmlParseError |
            WebDAVScanFailureType::TooManyItems |
            WebDAVScanFailureType::DepthLimit |
            WebDAVScanFailureType::SizeLimit => "high",
            
            WebDAVScanFailureType::Timeout |
            WebDAVScanFailureType::ServerError => {
                if let Some(code) = status_code {
                    if code == 404 {
                        "critical"
                    } else if code >= 500 {
                        "medium"
                    } else {
                        "medium"
                    }
                } else {
                    "medium"
                }
            },
            
            WebDAVScanFailureType::NetworkError => "low",
            
            WebDAVScanFailureType::Unknown => {
                // Try to infer from error message
                if error_msg.contains("timeout") || error_msg.contains("timed out") {
                    "medium"
                } else if error_msg.contains("permission") || error_msg.contains("forbidden") {
                    "high"
                } else if error_msg.contains("not found") || error_msg.contains("404") {
                    "critical"
                } else {
                    "medium"
                }
            }
        };
        
        (failure_type.to_string(), severity.to_string())
    }
    
    /// Get detailed failure information with diagnostics
    pub async fn get_scan_failure_with_diagnostics(&self, user_id: Uuid, failure_id: Uuid) -> Result<Option<crate::models::WebDAVScanFailureResponse>> {
        let failure = sqlx::query_as::<_, crate::models::WebDAVScanFailure>(
            r#"SELECT * FROM webdav_scan_failures 
               WHERE user_id = $1 AND id = $2"#
        )
        .bind(user_id)
        .bind(failure_id)
        .fetch_optional(&self.pool)
        .await?;
        
        match failure {
            Some(f) => {
                let diagnostics = self.build_failure_diagnostics(&f);
                
                Ok(Some(crate::models::WebDAVScanFailureResponse {
                    id: f.id,
                    directory_path: f.directory_path,
                    failure_type: f.failure_type,
                    failure_severity: f.failure_severity,
                    failure_count: f.failure_count,
                    consecutive_failures: f.consecutive_failures,
                    first_failure_at: f.first_failure_at,
                    last_failure_at: f.last_failure_at,
                    next_retry_at: f.next_retry_at,
                    error_message: f.error_message,
                    http_status_code: f.http_status_code,
                    user_excluded: f.user_excluded,
                    user_notes: f.user_notes,
                    resolved: f.resolved,
                    diagnostic_summary: diagnostics,
                }))
            },
            None => Ok(None)
        }
    }
    
    /// Build diagnostic summary for a failure
    fn build_failure_diagnostics(&self, failure: &crate::models::WebDAVScanFailure) -> crate::models::WebDAVFailureDiagnostics {
        use crate::models::{WebDAVScanFailureType, WebDAVScanFailureSeverity};
        
        let response_size_mb = failure.response_size_bytes.map(|b| b as f64 / 1_048_576.0);
        
        let (recommended_action, can_retry, user_action_required) = match (&failure.failure_type, &failure.failure_severity) {
            (WebDAVScanFailureType::PathTooLong, _) => (
                "Path exceeds system limits. Consider reorganizing directory structure.".to_string(),
                false,
                true
            ),
            (WebDAVScanFailureType::InvalidCharacters, _) => (
                "Path contains invalid characters. Rename the directory to remove special characters.".to_string(),
                false,
                true
            ),
            (WebDAVScanFailureType::PermissionDenied, _) => (
                "Access denied. Check WebDAV permissions for this directory.".to_string(),
                false,
                true
            ),
            (WebDAVScanFailureType::TooManyItems, _) => (
                "Directory contains too many items. Consider splitting into subdirectories.".to_string(),
                false,
                true
            ),
            (WebDAVScanFailureType::Timeout, _) if failure.failure_count > 3 => (
                "Repeated timeouts. Directory may be too large or server is slow.".to_string(),
                true,
                false
            ),
            (WebDAVScanFailureType::NetworkError, _) => (
                "Network error. Will retry automatically.".to_string(),
                true,
                false
            ),
            (WebDAVScanFailureType::ServerError, _) if failure.http_status_code == Some(404) => (
                "Directory not found on server. It may have been deleted.".to_string(),
                false,
                false
            ),
            (WebDAVScanFailureType::ServerError, _) => (
                "Server error. Will retry when server is available.".to_string(),
                true,
                false
            ),
            _ if failure.failure_severity == WebDAVScanFailureSeverity::Critical => (
                "Critical error that requires manual intervention.".to_string(),
                false,
                true
            ),
            _ if failure.failure_count > 10 => (
                "Multiple failures. Consider excluding this directory.".to_string(),
                true,
                true
            ),
            _ => (
                "Temporary error. Will retry automatically.".to_string(),
                true,
                false
            )
        };
        
        crate::models::WebDAVFailureDiagnostics {
            path_length: failure.path_length,
            directory_depth: failure.directory_depth,
            estimated_item_count: failure.estimated_item_count,
            response_time_ms: failure.response_time_ms,
            response_size_mb,
            server_type: failure.server_type.clone(),
            recommended_action,
            can_retry,
            user_action_required,
        }
    }
}