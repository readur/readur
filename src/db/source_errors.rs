use anyhow::Result;
use sqlx::Row;
use uuid::Uuid;
use std::collections::HashMap;

use super::Database;
use crate::models::{
    CreateSourceScanFailure, SourceScanFailure, SourceScanFailureStats,
    ErrorSourceType, ListFailuresQuery,
};

impl Database {
    /// Record a new source scan failure or increment existing failure count
    pub async fn record_source_scan_failure(&self, failure: &CreateSourceScanFailure) -> Result<Uuid> {
        self.with_retry(|| async {
            let row = sqlx::query(
                r#"SELECT record_source_scan_failure($1, $2::source_error_source_type, $3, $4, $5::source_error_type, $6, $7, $8, $9, $10, $11, $12) as failure_id"#
            )
            .bind(failure.user_id)
            .bind(failure.source_type.to_string())
            .bind(failure.source_id)
            .bind(&failure.resource_path)
            .bind(failure.error_type.to_string())
            .bind(&failure.error_message)
            .bind(&failure.error_code)
            .bind(failure.http_status_code)
            .bind(failure.response_time_ms)
            .bind(failure.response_size_bytes)
            .bind(failure.resource_size_bytes)
            .bind(&failure.diagnostic_data.clone().unwrap_or(serde_json::json!({})))
            .fetch_one(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("Database query failed: {}", e))?;
            
            Ok(row.get("failure_id"))
        }).await
    }

    /// Get all source scan failures for a user with optional filtering
    pub async fn list_source_scan_failures(
        &self,
        user_id: Uuid,
        query: &ListFailuresQuery,
    ) -> Result<Vec<SourceScanFailure>> {
        self.with_retry(|| async {
            let mut sql = String::from(
                r#"SELECT id, user_id, source_type, source_id, resource_path,
                   error_type, error_severity, failure_count, consecutive_failures,
                   first_failure_at, last_failure_at, last_retry_at, next_retry_at,
                   error_message, error_code, http_status_code,
                   response_time_ms, response_size_bytes, resource_size_bytes,
                   resource_depth, estimated_item_count, diagnostic_data,
                   user_excluded, user_notes, retry_strategy, max_retries, retry_delay_seconds,
                   resolved, resolved_at, resolution_method, resolution_notes,
                   created_at, updated_at
                   FROM source_scan_failures WHERE user_id = $1"#
            );

            let mut bind_index = 2;
            let mut conditions = Vec::new();

            if let Some(_source_type) = &query.source_type {
                conditions.push(format!("source_type = ${}::source_error_source_type", bind_index));
                bind_index += 1;
            }

            if let Some(_source_id) = &query.source_id {
                conditions.push(format!("source_id = ${}", bind_index));
                bind_index += 1;
            }

            if let Some(_error_type) = &query.error_type {
                conditions.push(format!("error_type = ${}::source_error_type", bind_index));
                bind_index += 1;
            }

            if let Some(_severity) = &query.severity {
                conditions.push(format!("error_severity = ${}::source_error_severity", bind_index));
                bind_index += 1;
            }

            if let Some(include_resolved) = query.include_resolved {
                if !include_resolved {
                    conditions.push("NOT resolved".to_string());
                }
            }

            if let Some(include_excluded) = query.include_excluded {
                if !include_excluded {
                    conditions.push("NOT user_excluded".to_string());
                }
            }

            if let Some(ready_for_retry) = query.ready_for_retry {
                if ready_for_retry {
                    conditions.push("next_retry_at <= NOW() AND NOT resolved AND NOT user_excluded".to_string());
                }
            }

            if !conditions.is_empty() {
                sql.push_str(" AND ");
                sql.push_str(&conditions.join(" AND "));
            }

            sql.push_str(" ORDER BY error_severity DESC, last_failure_at DESC");

            if let Some(_limit) = query.limit {
                sql.push_str(&format!(" LIMIT ${}", bind_index));
                bind_index += 1;
            }

            if let Some(_offset) = query.offset {
                sql.push_str(&format!(" OFFSET ${}", bind_index));
            }

            let mut query_builder = sqlx::query_as::<_, SourceScanFailure>(&sql);
            query_builder = query_builder.bind(user_id);

            if let Some(source_type) = &query.source_type {
                query_builder = query_builder.bind(source_type.to_string());
            }

            if let Some(source_id) = &query.source_id {
                query_builder = query_builder.bind(source_id);
            }

            if let Some(error_type) = &query.error_type {
                query_builder = query_builder.bind(error_type.to_string());
            }

            if let Some(severity) = &query.severity {
                query_builder = query_builder.bind(severity.to_string());
            }

            if let Some(limit) = query.limit {
                query_builder = query_builder.bind(limit);
            }

            if let Some(offset) = query.offset {
                query_builder = query_builder.bind(offset);
            }

            let rows = query_builder
                .fetch_all(&self.pool)
                .await
                .map_err(|e| anyhow::anyhow!("Database query failed: {}", e))?;

            Ok(rows)
        }).await
    }

    /// Get a specific source scan failure
    pub async fn get_source_scan_failure(&self, user_id: Uuid, failure_id: Uuid) -> Result<Option<SourceScanFailure>> {
        self.with_retry(|| async {
            let row = sqlx::query_as::<_, SourceScanFailure>(
                r#"SELECT id, user_id, source_type, source_id, resource_path,
                   error_type, error_severity, failure_count, consecutive_failures,
                   first_failure_at, last_failure_at, last_retry_at, next_retry_at,
                   error_message, error_code, http_status_code,
                   response_time_ms, response_size_bytes, resource_size_bytes,
                   resource_depth, estimated_item_count, diagnostic_data,
                   user_excluded, user_notes, retry_strategy, max_retries, retry_delay_seconds,
                   resolved, resolved_at, resolution_method, resolution_notes,
                   created_at, updated_at
                   FROM source_scan_failures 
                   WHERE user_id = $1 AND id = $2"#
            )
            .bind(user_id)
            .bind(failure_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("Database query failed: {}", e))?;

            Ok(row)
        }).await
    }

    /// Check if a source resource is a known failure that should be skipped
    pub async fn is_source_known_failure(
        &self,
        user_id: Uuid,
        source_type: ErrorSourceType,
        source_id: Option<Uuid>,
        resource_path: &str,
    ) -> Result<bool> {
        self.with_retry(|| async {
            let row = sqlx::query(
                r#"SELECT 1 FROM source_scan_failures 
                   WHERE user_id = $1 AND source_type = $2::source_error_source_type 
                   AND (source_id = $3 OR (source_id IS NULL AND $3 IS NULL))
                   AND resource_path = $4
                   AND NOT resolved 
                   AND (user_excluded = TRUE OR 
                        (error_severity IN ('critical', 'high') AND failure_count > 3) OR
                        (next_retry_at IS NULL OR next_retry_at > NOW()))"#
            )
            .bind(user_id)
            .bind(source_type.to_string())
            .bind(source_id)
            .bind(resource_path)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("Database query failed: {}", e))?;

            Ok(row.is_some())
        }).await
    }

    /// Get source resources ready for retry
    pub async fn get_source_retry_candidates(
        &self,
        user_id: Uuid,
        source_type: Option<ErrorSourceType>,
        limit: i32,
    ) -> Result<Vec<SourceScanFailure>> {
        self.with_retry(|| async {
            let mut sql = String::from(
                r#"SELECT id, user_id, source_type, source_id, resource_path,
                   error_type, error_severity, failure_count, consecutive_failures,
                   first_failure_at, last_failure_at, last_retry_at, next_retry_at,
                   error_message, error_code, http_status_code,
                   response_time_ms, response_size_bytes, resource_size_bytes,
                   resource_depth, estimated_item_count, diagnostic_data,
                   user_excluded, user_notes, retry_strategy, max_retries, retry_delay_seconds,
                   resolved, resolved_at, resolution_method, resolution_notes,
                   created_at, updated_at
                   FROM source_scan_failures 
                   WHERE user_id = $1 
                   AND NOT resolved 
                   AND NOT user_excluded
                   AND next_retry_at <= NOW()
                   AND failure_count < max_retries"#
            );

            let mut bind_index = 2;
            if let Some(_) = source_type {
                sql.push_str(&format!(" AND source_type = ${}::source_error_source_type", bind_index));
                bind_index += 1;
            }

            sql.push_str(&format!(" ORDER BY error_severity ASC, next_retry_at ASC LIMIT ${}", bind_index));

            let mut query_builder = sqlx::query_as::<_, SourceScanFailure>(&sql);
            query_builder = query_builder.bind(user_id);

            if let Some(source_type) = source_type {
                query_builder = query_builder.bind(source_type.to_string());
            }

            query_builder = query_builder.bind(limit);

            let rows = query_builder
                .fetch_all(&self.pool)
                .await
                .map_err(|e| anyhow::anyhow!("Database query failed: {}", e))?;

            Ok(rows)
        }).await
    }

    /// Reset a source scan failure for retry
    pub async fn reset_source_scan_failure(
        &self,
        user_id: Uuid,
        source_type: ErrorSourceType,
        source_id: Option<Uuid>,
        resource_path: &str,
    ) -> Result<bool> {
        self.with_retry(|| async {
            let row = sqlx::query(
                r#"SELECT reset_source_scan_failure($1, $2::source_error_source_type, $3, $4) as success"#
            )
            .bind(user_id)
            .bind(source_type.to_string())
            .bind(source_id)
            .bind(resource_path)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("Database query failed: {}", e))?;

            Ok(row.get("success"))
        }).await
    }

    /// Mark a source scan failure as resolved
    pub async fn resolve_source_scan_failure(
        &self,
        user_id: Uuid,
        source_type: ErrorSourceType,
        source_id: Option<Uuid>,
        resource_path: &str,
        resolution_method: &str,
    ) -> Result<bool> {
        self.with_retry(|| async {
            let row = sqlx::query(
                r#"SELECT resolve_source_scan_failure($1, $2::source_error_source_type, $3, $4, $5) as success"#
            )
            .bind(user_id)
            .bind(source_type.to_string())
            .bind(source_id)
            .bind(resource_path)
            .bind(resolution_method)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("Database query failed: {}", e))?;

            Ok(row.get("success"))
        }).await
    }

    /// Mark a source resource as permanently excluded by user
    pub async fn exclude_source_from_scan(
        &self,
        user_id: Uuid,
        source_type: ErrorSourceType,
        source_id: Option<Uuid>,
        resource_path: &str,
        user_notes: Option<&str>,
    ) -> Result<bool> {
        self.with_retry(|| async {
            let result = sqlx::query(
                r#"UPDATE source_scan_failures 
                   SET user_excluded = TRUE,
                       user_notes = COALESCE($5, user_notes),
                       updated_at = NOW()
                   WHERE user_id = $1 AND source_type = $2::source_error_source_type 
                   AND (source_id = $3 OR (source_id IS NULL AND $3 IS NULL))
                   AND resource_path = $4"#
            )
            .bind(user_id)
            .bind(source_type.to_string())
            .bind(source_id)
            .bind(resource_path)
            .bind(user_notes)
            .execute(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("Database update failed: {}", e))?;

            Ok(result.rows_affected() > 0)
        }).await
    }

    /// Get source scan failure statistics for a user
    pub async fn get_source_scan_failure_stats(
        &self,
        user_id: Uuid,
        source_type: Option<ErrorSourceType>,
    ) -> Result<SourceScanFailureStats> {
        self.with_retry(|| async {
            let mut sql = String::from(
                r#"SELECT 
                    COUNT(*) FILTER (WHERE NOT resolved) as active_failures,
                    COUNT(*) FILTER (WHERE resolved) as resolved_failures,
                    COUNT(*) FILTER (WHERE user_excluded) as excluded_resources,
                    COUNT(*) FILTER (WHERE error_severity = 'critical' AND NOT resolved) as critical_failures,
                    COUNT(*) FILTER (WHERE error_severity = 'high' AND NOT resolved) as high_failures,
                    COUNT(*) FILTER (WHERE error_severity = 'medium' AND NOT resolved) as medium_failures,
                    COUNT(*) FILTER (WHERE error_severity = 'low' AND NOT resolved) as low_failures,
                    COUNT(*) FILTER (WHERE next_retry_at <= NOW() AND NOT resolved AND NOT user_excluded) as ready_for_retry
                   FROM source_scan_failures
                   WHERE user_id = $1"#
            );

            let bind_index = 2;
            if let Some(_) = source_type {
                sql.push_str(&format!(" AND source_type = ${}::source_error_source_type", bind_index));
            }

            let mut query_builder = sqlx::query(&sql);
            query_builder = query_builder.bind(user_id);

            if let Some(source_type) = source_type {
                query_builder = query_builder.bind(source_type.to_string());
            }

            let row = query_builder
                .fetch_one(&self.pool)
                .await
                .map_err(|e| anyhow::anyhow!("Database query failed: {}", e))?;

            // Get breakdown by source type
            let by_source_type_rows = sqlx::query(
                r#"SELECT source_type::TEXT as source_type, COUNT(*) as count
                   FROM source_scan_failures
                   WHERE user_id = $1 AND NOT resolved
                   GROUP BY source_type"#
            )
            .bind(user_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("Database query failed: {}", e))?;

            let mut by_source_type = HashMap::new();
            for row in by_source_type_rows {
                let source_type: String = row.get("source_type");
                let count: i64 = row.get("count");
                by_source_type.insert(source_type, count);
            }

            // Get breakdown by error type
            let by_error_type_rows = sqlx::query(
                r#"SELECT error_type::TEXT as error_type, COUNT(*) as count
                   FROM source_scan_failures
                   WHERE user_id = $1 AND NOT resolved
                   GROUP BY error_type"#
            )
            .bind(user_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("Database query failed: {}", e))?;

            let mut by_error_type = HashMap::new();
            for row in by_error_type_rows {
                let error_type: String = row.get("error_type");
                let count: i64 = row.get("count");
                by_error_type.insert(error_type, count);
            }

            Ok(SourceScanFailureStats {
                active_failures: row.get("active_failures"),
                resolved_failures: row.get("resolved_failures"),
                excluded_resources: row.get("excluded_resources"),
                critical_failures: row.get("critical_failures"),
                high_failures: row.get("high_failures"),
                medium_failures: row.get("medium_failures"),
                low_failures: row.get("low_failures"),
                ready_for_retry: row.get("ready_for_retry"),
                by_source_type,
                by_error_type,
            })
        }).await
    }
}