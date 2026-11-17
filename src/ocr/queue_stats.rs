//! OCR queue statistics.

use anyhow::Result;
use sqlx::{Column, Row};

use super::{OcrQueueService, QueueStats};

impl OcrQueueService {
    /// Get queue statistics
    pub async fn get_stats(&self) -> Result<QueueStats> {
        tracing::debug!("OCR Queue: Starting get_stats() call");

        // First, let's check the function signature/return type
        let function_info = sqlx::query(
            r#"
            SELECT
                p.proname as function_name,
                pg_get_function_result(p.oid) as return_type,
                pg_get_function_arguments(p.oid) as arguments
            FROM pg_proc p
            JOIN pg_namespace n ON p.pronamespace = n.oid
            WHERE n.nspname = 'public' AND p.proname = 'get_queue_statistics'
            "#,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get function info: {}", e);
            e
        })?;

        if let Some(info) = function_info {
            let function_name: String = info.get("function_name");
            let return_type: String = info.get("return_type");
            let arguments: String = info.get("arguments");
            tracing::debug!(
                "Function info - name: {}, return_type: {}, arguments: {}",
                function_name,
                return_type,
                arguments
            );
        } else {
            tracing::error!("get_queue_statistics function not found!");
            return Err(anyhow::anyhow!("get_queue_statistics function not found"));
        }

        tracing::debug!("OCR Queue: Calling get_queue_statistics() function");

        let stats = sqlx::query(
            r#"
            SELECT * FROM get_queue_statistics()
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get OCR queue stats: {}", e);
            tracing::debug!("This indicates a function structure mismatch error");
            e
        })?;

        tracing::debug!("OCR Queue: Successfully got result from function, analyzing structure...");

        // Debug the actual columns returned
        let columns = stats.columns();
        tracing::debug!("Function returned {} columns:", columns.len());
        for (i, column) in columns.iter().enumerate() {
            let column_name = column.name();
            let column_type = column.type_info();
            tracing::debug!(
                "  Column {}: name='{}', type='{:?}'",
                i,
                column_name,
                column_type
            );
        }

        // Try to extract values with detailed logging
        tracing::debug!("Attempting to extract pending_count...");
        let pending_count = match stats.try_get::<i64, _>("pending_count") {
            Ok(val) => {
                tracing::debug!("Successfully got pending_count: {}", val);
                val
            }
            Err(e) => {
                tracing::error!("Failed to get pending_count: {}", e);
                tracing::debug!("Trying different type for pending_count...");
                stats
                    .try_get::<Option<i64>, _>("pending_count")
                    .map_err(|e2| {
                        tracing::error!("Also failed with Option<i64>: {}", e2);
                        e
                    })?
                    .unwrap_or(0)
            }
        };

        tracing::debug!("Attempting to extract processing_count...");
        let processing_count = match stats.try_get::<i64, _>("processing_count") {
            Ok(val) => {
                tracing::debug!("Successfully got processing_count: {}", val);
                val
            }
            Err(e) => {
                tracing::error!("Failed to get processing_count: {}", e);
                stats
                    .try_get::<Option<i64>, _>("processing_count")?
                    .unwrap_or(0)
            }
        };

        tracing::debug!("Attempting to extract failed_count...");
        let failed_count = match stats.try_get::<i64, _>("failed_count") {
            Ok(val) => {
                tracing::debug!("Successfully got failed_count: {}", val);
                val
            }
            Err(e) => {
                tracing::error!("Failed to get failed_count: {}", e);
                stats
                    .try_get::<Option<i64>, _>("failed_count")?
                    .unwrap_or(0)
            }
        };

        tracing::debug!("Attempting to extract completed_today...");
        let completed_today = match stats.try_get::<i64, _>("completed_today") {
            Ok(val) => {
                tracing::debug!("Successfully got completed_today: {}", val);
                val
            }
            Err(e) => {
                tracing::error!("Failed to get completed_today: {}", e);
                stats
                    .try_get::<Option<i64>, _>("completed_today")?
                    .unwrap_or(0)
            }
        };

        tracing::debug!("Attempting to extract avg_wait_time_minutes...");
        let avg_wait_time_minutes = match stats.try_get::<Option<f64>, _>("avg_wait_time_minutes") {
            Ok(val) => {
                tracing::debug!("Successfully got avg_wait_time_minutes: {:?}", val);
                val
            }
            Err(e) => {
                tracing::error!("Failed to get avg_wait_time_minutes: {}", e);
                // Try as string and convert
                match stats.try_get::<Option<String>, _>("avg_wait_time_minutes") {
                    Ok(Some(str_val)) => {
                        let float_val = str_val.parse::<f64>().ok();
                        tracing::debug!("Converted string '{}' to f64: {:?}", str_val, float_val);
                        float_val
                    }
                    Ok(None) => None,
                    Err(e2) => {
                        tracing::error!("Also failed with String: {}", e2);
                        return Err(anyhow::anyhow!(
                            "Failed to get avg_wait_time_minutes: {}",
                            e
                        ));
                    }
                }
            }
        };

        tracing::debug!("Attempting to extract oldest_pending_minutes...");
        let oldest_pending_minutes = match stats.try_get::<Option<f64>, _>("oldest_pending_minutes")
        {
            Ok(val) => {
                tracing::debug!("Successfully got oldest_pending_minutes: {:?}", val);
                val
            }
            Err(e) => {
                tracing::error!("Failed to get oldest_pending_minutes: {}", e);
                // Try as string and convert
                match stats.try_get::<Option<String>, _>("oldest_pending_minutes") {
                    Ok(Some(str_val)) => {
                        let float_val = str_val.parse::<f64>().ok();
                        tracing::debug!("Converted string '{}' to f64: {:?}", str_val, float_val);
                        float_val
                    }
                    Ok(None) => None,
                    Err(e2) => {
                        tracing::error!("Also failed with String: {}", e2);
                        return Err(anyhow::anyhow!(
                            "Failed to get oldest_pending_minutes: {}",
                            e
                        ));
                    }
                }
            }
        };

        tracing::debug!("OCR Queue: Successfully extracted all values, creating QueueStats");

        Ok(QueueStats {
            pending_count,
            processing_count,
            failed_count,
            completed_today,
            avg_wait_time_minutes,
            oldest_pending_minutes,
        })
    }
}
