/*!
 * Database Monitoring and Alerting System
 * 
 * Provides real-time monitoring of database health, OCR processing,
 * and automatic alerting for potential issues.
 */

use sqlx::PgPool;
use serde::{Deserialize, Serialize};
use tokio::time::{Duration, interval};
use tracing::{error, warn, info, debug};
use anyhow::Result;
use std::sync::Arc;

/// Database monitoring service that runs in the background
pub struct DatabaseMonitor {
    pool: PgPool,
    config: MonitoringConfig,
}

#[derive(Debug, Clone)]
pub struct MonitoringConfig {
    pub check_interval_secs: u64,
    pub stuck_job_threshold_minutes: i32,
    pub high_queue_size_threshold: i32,
    pub low_confidence_threshold: f64,
    pub pool_utilization_threshold: u8,
    pub slow_query_threshold_ms: u64,
    pub enable_auto_recovery: bool,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            check_interval_secs: 60,
            stuck_job_threshold_minutes: 30,
            high_queue_size_threshold: 100,
            low_confidence_threshold: 70.0,
            pool_utilization_threshold: 80,
            slow_query_threshold_ms: 5000,
            enable_auto_recovery: true,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseHealth {
    pub overall_status: HealthStatus,
    pub ocr_processing: OcrProcessingHealth,
    pub queue_health: QueueHealth,
    pub connection_pool: PoolHealth,
    pub data_consistency: ConsistencyHealth,
    pub performance_metrics: PerformanceMetrics,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OcrProcessingHealth {
    pub status: HealthStatus,
    pub pending_jobs: i32,
    pub processing_jobs: i32,
    pub stuck_jobs: i32,
    pub failed_jobs_last_hour: i32,
    pub average_confidence: Option<f64>,
    pub average_processing_time_ms: Option<f64>,
    pub throughput_per_minute: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueueHealth {
    pub status: HealthStatus,
    pub queue_size: i32,
    pub oldest_pending_age_minutes: Option<i32>,
    pub worker_count: i32,
    pub queue_growth_rate: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PoolHealth {
    pub status: HealthStatus,
    pub total_connections: u32,
    pub active_connections: u32,
    pub idle_connections: u32,
    pub utilization_percent: u8,
    pub average_response_time_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConsistencyHealth {
    pub status: HealthStatus,
    pub orphaned_queue_items: i32,
    pub documents_without_files: i32,
    pub inconsistent_ocr_states: i32,
    pub data_integrity_score: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub queries_per_second: f64,
    pub slow_queries_count: i32,
    pub cache_hit_ratio: Option<f64>,
    pub index_usage_efficiency: f64,
    pub deadlock_count: i32,
}

impl DatabaseMonitor {
    pub fn new(pool: PgPool, config: MonitoringConfig) -> Self {
        Self { pool, config }
    }

    /// Start the monitoring service
    pub async fn start(self: Arc<Self>) {
        let mut interval = interval(Duration::from_secs(self.config.check_interval_secs));
        
        info!("Database monitoring started with {}s intervals", self.config.check_interval_secs);
        
        loop {
            interval.tick().await;
            
            match self.perform_health_check().await {
                Ok(health) => {
                    self.process_health_report(health).await;
                }
                Err(e) => {
                    error!("Database health check failed: {}", e);
                }
            }
        }
    }

    /// Perform comprehensive database health check
    async fn perform_health_check(&self) -> Result<DatabaseHealth> {
        let start_time = std::time::Instant::now();
        
        // Run all health checks concurrently
        let (ocr_health, queue_health, pool_health, consistency_health, perf_metrics) = tokio::try_join!(
            self.check_ocr_processing_health(),
            self.check_queue_health(),
            self.check_pool_health(),
            self.check_data_consistency(),
            self.check_performance_metrics()
        )?;

        let overall_status = self.determine_overall_status(&ocr_health, &queue_health, &pool_health, &consistency_health);
        
        let health_check_duration = start_time.elapsed();
        debug!("Health check completed in {:?}", health_check_duration);

        Ok(DatabaseHealth {
            overall_status,
            ocr_processing: ocr_health,
            queue_health,
            connection_pool: pool_health,
            data_consistency: consistency_health,
            performance_metrics: perf_metrics,
            timestamp: chrono::Utc::now(),
        })
    }

    async fn check_ocr_processing_health(&self) -> Result<OcrProcessingHealth> {
        let stats = sqlx::query!(
            r#"
            SELECT 
                COUNT(*) FILTER (WHERE ocr_status = 'pending') as pending,
                COUNT(*) FILTER (WHERE ocr_status = 'processing') as processing,
                COUNT(*) FILTER (WHERE ocr_status = 'processing' AND updated_at < NOW() - INTERVAL '30 minutes') as stuck,
                COUNT(*) FILTER (WHERE ocr_status = 'failed' AND updated_at > NOW() - INTERVAL '1 hour') as failed_recent,
                AVG(ocr_confidence) FILTER (WHERE ocr_status = 'completed' AND ocr_completed_at > NOW() - INTERVAL '1 hour') as avg_confidence,
                AVG(ocr_processing_time_ms) FILTER (WHERE ocr_status = 'completed' AND ocr_completed_at > NOW() - INTERVAL '1 hour') as avg_time,
                COUNT(*) FILTER (WHERE ocr_status = 'completed' AND ocr_completed_at > NOW() - INTERVAL '1 minute') as completed_last_minute
            FROM documents
            "#
        )
        .fetch_one(&self.pool)
        .await?;

        let pending = stats.pending.unwrap_or(0) as i32;
        let processing = stats.processing.unwrap_or(0) as i32;
        let stuck = stats.stuck.unwrap_or(0) as i32;
        let failed_recent = stats.failed_recent.unwrap_or(0) as i32;
        let avg_confidence = stats.avg_confidence;
        let avg_time = stats.avg_time;
        let throughput = stats.completed_last_minute.unwrap_or(0) as f64;

        let status = if stuck > 0 || failed_recent > 10 {
            HealthStatus::Critical
        } else if pending > self.config.high_queue_size_threshold || avg_confidence.unwrap_or(100.0) < self.config.low_confidence_threshold {
            HealthStatus::Warning
        } else {
            HealthStatus::Healthy
        };

        Ok(OcrProcessingHealth {
            status,
            pending_jobs: pending,
            processing_jobs: processing,
            stuck_jobs: stuck,
            failed_jobs_last_hour: failed_recent,
            average_confidence: avg_confidence,
            average_processing_time_ms: avg_time,
            throughput_per_minute: throughput,
        })
    }

    async fn check_queue_health(&self) -> Result<QueueHealth> {
        let queue_stats = sqlx::query!(
            r#"
            SELECT 
                COUNT(*) as total_items,
                MIN(EXTRACT(EPOCH FROM (NOW() - created_at))/60) as oldest_pending_minutes,
                COUNT(DISTINCT worker_id) FILTER (WHERE status = 'processing') as active_workers
            FROM ocr_queue
            WHERE status IN ('pending', 'processing')
            "#
        )
        .fetch_one(&self.pool)
        .await?;

        let queue_size = queue_stats.total_items.unwrap_or(0) as i32;
        let oldest_pending = queue_stats.oldest_pending_minutes.map(|m| m as i32);
        let worker_count = queue_stats.active_workers.unwrap_or(0) as i32;

        // Calculate queue growth rate (simplified)
        let growth_rate = 0.0; // Would need historical data for accurate calculation

        let status = if queue_size > self.config.high_queue_size_threshold {
            HealthStatus::Critical
        } else if queue_size > self.config.high_queue_size_threshold / 2 {
            HealthStatus::Warning
        } else {
            HealthStatus::Healthy
        };

        Ok(QueueHealth {
            status,
            queue_size,
            oldest_pending_age_minutes: oldest_pending,
            worker_count,
            queue_growth_rate: growth_rate,
        })
    }

    async fn check_pool_health(&self) -> Result<PoolHealth> {
        let start = std::time::Instant::now();
        
        // Test pool responsiveness
        sqlx::query!("SELECT 1")
            .fetch_one(&self.pool)
            .await?;
        
        let response_time = start.elapsed().as_millis() as u64;
        
        let total_connections = self.pool.size();
        let idle_connections = self.pool.num_idle();
        let active_connections = total_connections - idle_connections;
        let utilization = if total_connections > 0 {
            (active_connections as f64 / total_connections as f64 * 100.0) as u8
        } else {
            0
        };

        let status = if utilization > self.config.pool_utilization_threshold {
            HealthStatus::Critical
        } else if utilization > self.config.pool_utilization_threshold / 2 || response_time > self.config.slow_query_threshold_ms {
            HealthStatus::Warning
        } else {
            HealthStatus::Healthy
        };

        Ok(PoolHealth {
            status,
            total_connections,
            active_connections,
            idle_connections,
            utilization_percent: utilization,
            average_response_time_ms: response_time,
        })
    }

    async fn check_data_consistency(&self) -> Result<ConsistencyHealth> {
        let consistency_check = sqlx::query!(
            r#"
            SELECT 
                -- Orphaned queue items
                (SELECT COUNT(*) FROM ocr_queue q 
                 LEFT JOIN documents d ON q.document_id = d.id 
                 WHERE d.id IS NULL) as orphaned_queue,
                
                -- Documents without files (would need file system check)
                0 as missing_files,
                
                -- Inconsistent OCR states
                (SELECT COUNT(*) FROM documents d
                 JOIN ocr_queue q ON d.id = q.document_id
                 WHERE d.ocr_status = 'completed' AND q.status != 'completed') as inconsistent_states
            "#
        )
        .fetch_one(&self.pool)
        .await?;

        let orphaned = consistency_check.orphaned_queue.unwrap_or(0) as i32;
        let missing_files = consistency_check.missing_files.unwrap_or(0) as i32;
        let inconsistent = consistency_check.inconsistent_states.unwrap_or(0) as i32;

        let total_issues = orphaned + missing_files + inconsistent;
        let integrity_score = if total_issues == 0 { 100.0 } else { 100.0 - (total_issues as f64 * 10.0).min(100.0) };

        let status = if total_issues > 10 {
            HealthStatus::Critical
        } else if total_issues > 0 {
            HealthStatus::Warning
        } else {
            HealthStatus::Healthy
        };

        Ok(ConsistencyHealth {
            status,
            orphaned_queue_items: orphaned,
            documents_without_files: missing_files,
            inconsistent_ocr_states: inconsistent,
            data_integrity_score: integrity_score,
        })
    }

    async fn check_performance_metrics(&self) -> Result<PerformanceMetrics> {
        // These would need more sophisticated monitoring in production
        // For now, return basic metrics
        
        Ok(PerformanceMetrics {
            queries_per_second: 0.0,
            slow_queries_count: 0,
            cache_hit_ratio: None,
            index_usage_efficiency: 95.0,
            deadlock_count: 0,
        })
    }

    fn determine_overall_status(
        &self,
        ocr: &OcrProcessingHealth,
        queue: &QueueHealth,
        pool: &PoolHealth,
        consistency: &ConsistencyHealth,
    ) -> HealthStatus {
        let statuses = [&ocr.status, &queue.status, &pool.status, &consistency.status];
        
        if statuses.iter().any(|s| matches!(s, HealthStatus::Critical)) {
            HealthStatus::Critical
        } else if statuses.iter().any(|s| matches!(s, HealthStatus::Warning)) {
            HealthStatus::Warning
        } else {
            HealthStatus::Healthy
        }
    }

    /// Process health report and take actions
    async fn process_health_report(&self, health: DatabaseHealth) {
        match health.overall_status {
            HealthStatus::Critical => {
                error!("🚨 CRITICAL: Database health issues detected!");
                self.handle_critical_issues(&health).await;
            }
            HealthStatus::Warning => {
                warn!("⚠️ WARNING: Database health degraded");
                self.handle_warnings(&health).await;
            }
            HealthStatus::Healthy => {
                debug!("✅ Database health is good");
            }
            HealthStatus::Unknown => {
                warn!("❓ Database health status unknown");
            }
        }

        // Log key metrics
        info!(
            "DB Health: OCR pending={}, processing={}, stuck={}, pool={}%",
            health.ocr_processing.pending_jobs,
            health.ocr_processing.processing_jobs,
            health.ocr_processing.stuck_jobs,
            health.connection_pool.utilization_percent
        );
    }

    async fn handle_critical_issues(&self, health: &DatabaseHealth) {
        if self.config.enable_auto_recovery {
            // Reset stuck OCR jobs
            if health.ocr_processing.stuck_jobs > 0 {
                match self.reset_stuck_jobs().await {
                    Ok(reset_count) => {
                        warn!("Auto-recovery: Reset {} stuck OCR jobs", reset_count);
                    }
                    Err(e) => {
                        error!("Failed to reset stuck OCR jobs: {}", e);
                    }
                }
            }

            // Clean up orphaned queue items
            if health.data_consistency.orphaned_queue_items > 0 {
                match self.cleanup_orphaned_items().await {
                    Ok(cleanup_count) => {
                        warn!("Auto-recovery: Cleaned up {} orphaned queue items", cleanup_count);
                    }
                    Err(e) => {
                        error!("Failed to cleanup orphaned items: {}", e);
                    }
                }
            }
        }
    }

    async fn handle_warnings(&self, health: &DatabaseHealth) {
        // Log detailed warning information
        if health.ocr_processing.pending_jobs > self.config.high_queue_size_threshold / 2 {
            warn!("High OCR queue size: {} pending jobs", health.ocr_processing.pending_jobs);
        }

        if health.connection_pool.utilization_percent > self.config.pool_utilization_threshold / 2 {
            warn!("High connection pool utilization: {}%", health.connection_pool.utilization_percent);
        }
    }

    async fn reset_stuck_jobs(&self) -> Result<i32> {
        let result = sqlx::query!(
            "SELECT reset_stuck_ocr_jobs($1) as reset_count",
            self.config.stuck_job_threshold_minutes
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(result.reset_count.unwrap_or(0))
    }

    async fn cleanup_orphaned_items(&self) -> Result<i32> {
        let result = sqlx::query!(
            r#"
            DELETE FROM ocr_queue
            WHERE document_id NOT IN (SELECT id FROM documents)
            "#
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as i32)
    }

    /// Get current health status (for API endpoints)
    pub async fn get_current_health(&self) -> Result<DatabaseHealth> {
        self.perform_health_check().await
    }

    /// Force a consistency check and cleanup
    pub async fn force_cleanup(&self) -> Result<String> {
        let reset_count = self.reset_stuck_jobs().await?;
        let cleanup_count = self.cleanup_orphaned_items().await?;
        
        // Refresh OCR stats
        sqlx::query!("SELECT refresh_ocr_stats()")
            .execute(&self.pool)
            .await?;

        Ok(format!(
            "Cleanup completed: {} stuck jobs reset, {} orphaned items removed",
            reset_count, cleanup_count
        ))
    }
}

/// Alert configuration for different severity levels
#[derive(Debug, Clone)]
pub struct AlertConfig {
    pub email_notifications: bool,
    pub slack_webhook: Option<String>,
    pub critical_alert_cooldown_minutes: u64,
    pub warning_alert_cooldown_minutes: u64,
}

/// Alert manager for sending notifications
pub struct AlertManager {
    config: AlertConfig,
    last_critical_alert: std::sync::Mutex<Option<chrono::DateTime<chrono::Utc>>>,
    last_warning_alert: std::sync::Mutex<Option<chrono::DateTime<chrono::Utc>>>,
}

impl AlertManager {
    pub fn new(config: AlertConfig) -> Self {
        Self {
            config,
            last_critical_alert: std::sync::Mutex::new(None),
            last_warning_alert: std::sync::Mutex::new(None),
        }
    }

    pub async fn send_alert(&self, health: &DatabaseHealth) -> Result<()> {
        match health.overall_status {
            HealthStatus::Critical => {
                if self.should_send_critical_alert() {
                    self.send_critical_alert(health).await?;
                    self.update_last_critical_alert();
                }
            }
            HealthStatus::Warning => {
                if self.should_send_warning_alert() {
                    self.send_warning_alert(health).await?;
                    self.update_last_warning_alert();
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn should_send_critical_alert(&self) -> bool {
        let last_alert = self.last_critical_alert.lock().unwrap();
        match *last_alert {
            Some(last) => {
                let cooldown = chrono::Duration::minutes(self.config.critical_alert_cooldown_minutes as i64);
                chrono::Utc::now() - last > cooldown
            }
            None => true,
        }
    }

    fn should_send_warning_alert(&self) -> bool {
        let last_alert = self.last_warning_alert.lock().unwrap();
        match *last_alert {
            Some(last) => {
                let cooldown = chrono::Duration::minutes(self.config.warning_alert_cooldown_minutes as i64);
                chrono::Utc::now() - last > cooldown
            }
            None => true,
        }
    }

    async fn send_critical_alert(&self, health: &DatabaseHealth) -> Result<()> {
        let message = format!(
            "🚨 CRITICAL DATABASE ALERT 🚨\n\
            Stuck OCR jobs: {}\n\
            Pool utilization: {}%\n\
            Orphaned queue items: {}\n\
            Timestamp: {}",
            health.ocr_processing.stuck_jobs,
            health.connection_pool.utilization_percent,
            health.data_consistency.orphaned_queue_items,
            health.timestamp
        );

        error!("{}", message);
        // Add actual notification sending logic here (email, Slack, etc.)
        Ok(())
    }

    async fn send_warning_alert(&self, health: &DatabaseHealth) -> Result<()> {
        let message = format!(
            "⚠️ Database Warning\n\
            Pending OCR jobs: {}\n\
            Pool utilization: {}%\n\
            Average confidence: {:.1}%\n\
            Timestamp: {}",
            health.ocr_processing.pending_jobs,
            health.connection_pool.utilization_percent,
            health.ocr_processing.average_confidence.unwrap_or(0.0),
            health.timestamp
        );

        warn!("{}", message);
        Ok(())
    }

    fn update_last_critical_alert(&self) {
        let mut last_alert = self.last_critical_alert.lock().unwrap();
        *last_alert = Some(chrono::Utc::now());
    }

    fn update_last_warning_alert(&self) {
        let mut last_alert = self.last_warning_alert.lock().unwrap();
        *last_alert = Some(chrono::Utc::now());
    }
}