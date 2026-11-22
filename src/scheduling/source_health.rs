//! The scheduler's periodic health check of a source: configuration,
//! connectivity, sync pattern and error analysis, scored and stored on the
//! source, with a notification when the source is in poor health.

use sqlx::Row;
use std::sync::Arc;
use tracing::{error, info};

use super::source_connectivity;
use super::source_scheduler::SourceScheduler;
use crate::AppState;

struct SyncHealthAnalysis {
    score_penalty: i32,
    issues: Vec<serde_json::Value>,
}

struct ErrorAnalysis {
    score_penalty: i32,
    issues: Vec<serde_json::Value>,
}

impl SourceScheduler {
    /// Perform automatic validation of source health and connectivity
    pub async fn validate_source_health(
        source: &crate::models::Source,
        state: &Arc<AppState>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("🔍 Starting validation check for source: {}", source.name);

        let mut validation_score = 100;
        let mut validation_issues = Vec::<serde_json::Value>::new();
        let mut validation_status = "healthy";

        // 1. Configuration validation
        if let Err(config_error) = Self::validate_source_config_detailed(source) {
            validation_score -= 30;
            validation_status = "critical";
            validation_issues.push(serde_json::json!({
                "type": "configuration",
                "severity": "critical",
                "message": format!("Configuration error: {}", config_error),
                "recommendation": "Check and fix source configuration in settings"
            }));
        }

        // 2. Connectivity validation
        match source.source_type {
            crate::models::SourceType::WebDAV => {
                if let Err(e) = source_connectivity::check_webdav(source).await {
                    validation_score -= 25;
                    if validation_status == "healthy" {
                        validation_status = "warning";
                    }
                    validation_issues.push(serde_json::json!({
                        "type": "connectivity",
                        "severity": "warning",
                        "message": format!("WebDAV connectivity issue: {}", e),
                        "recommendation": "Check server URL, credentials, and network connectivity"
                    }));
                }
            }
            crate::models::SourceType::LocalFolder => {
                if let Err(e) = source_connectivity::check_local_folder(source).await {
                    validation_score -= 25;
                    if validation_status == "healthy" {
                        validation_status = "warning";
                    }
                    validation_issues.push(serde_json::json!({
                        "type": "connectivity",
                        "severity": "warning",
                        "message": format!("Local folder access issue: {}", e),
                        "recommendation": "Check folder permissions and path accessibility"
                    }));
                }
            }
            crate::models::SourceType::S3 => {
                if let Err(e) = source_connectivity::check_s3(source).await {
                    validation_score -= 25;
                    if validation_status == "healthy" {
                        validation_status = "warning";
                    }
                    validation_issues.push(serde_json::json!({
                        "type": "connectivity",
                        "severity": "warning",
                        "message": format!("S3 connectivity issue: {}", e),
                        "recommendation": "Check AWS credentials, bucket access, and permissions"
                    }));
                }
            }
        }

        // 3. Sync pattern analysis
        if let Ok(sync_health) = Self::analyze_sync_patterns(source, state).await {
            validation_score -= sync_health.score_penalty;
            if sync_health.score_penalty > 15 && validation_status == "healthy" {
                validation_status = "warning";
            }
            for issue in sync_health.issues {
                validation_issues.push(issue);
            }
        }

        // 4. Error rate analysis
        if let Ok(error_analysis) = Self::analyze_error_patterns(source, state).await {
            validation_score -= error_analysis.score_penalty;
            if error_analysis.score_penalty > 20 {
                validation_status = "warning";
            }
            for issue in error_analysis.issues {
                validation_issues.push(issue);
            }
        }

        // Cap the minimum score at 0
        validation_score = validation_score.max(0);

        // Update validation status in database
        let validation_issues_json =
            serde_json::to_string(&validation_issues).unwrap_or_else(|_| "[]".to_string());

        if let Err(e) = sqlx::query(
            r#"
            UPDATE sources 
            SET validation_status = $1, 
                last_validation_at = NOW(), 
                validation_score = $2,
                validation_issues = $3,
                updated_at = NOW()
            WHERE id = $4
            "#,
        )
        .bind(validation_status)
        .bind(validation_score)
        .bind(validation_issues_json)
        .bind(source.id)
        .execute(state.db.get_pool())
        .await
        {
            error!("Failed to update validation status: {}", e);
        }

        // Send notification if there are critical issues
        if validation_status == "critical" || validation_score < 50 {
            let notification = crate::models::CreateNotification {
                notification_type: if validation_status == "critical" {
                    "error"
                } else {
                    "warning"
                }
                .to_string(),
                title: format!(
                    "Source Validation {}",
                    if validation_status == "critical" {
                        "Failed"
                    } else {
                        "Warning"
                    }
                ),
                message: format!(
                    "Source {} has validation issues (score: {})",
                    source.name, validation_score
                ),
                action_url: Some("/sources".to_string()),
                metadata: Some(serde_json::json!({
                    "source_type": source.source_type.to_string(),
                    "source_id": source.id,
                    "validation_type": "health_check",
                    "validation_score": validation_score,
                    "validation_status": validation_status,
                    "issue_count": validation_issues.len()
                })),
            };

            if let Err(e) = state
                .db
                .create_notification(source.user_id, &notification)
                .await
            {
                error!("Failed to create validation notification: {}", e);
            }
        }

        info!(
            "✅ Validation completed for {}: {} (score: {})",
            source.name, validation_status, validation_score
        );
        Ok(())
    }

    fn validate_source_config_detailed(source: &crate::models::Source) -> Result<(), String> {
        // Reuse existing validation logic but return more detailed errors
        Self::validate_source_config_static(source)
    }

    fn validate_source_config_static(source: &crate::models::Source) -> Result<(), String> {
        use crate::models::{
            LocalFolderSourceConfig, S3SourceConfig, SourceType, WebDAVSourceConfig,
        };

        match source.source_type {
            SourceType::WebDAV => {
                let config: WebDAVSourceConfig = serde_json::from_value(source.config.clone())
                    .map_err(|e| format!("Failed to parse WebDAV configuration: {}", e))?;

                if config.server_url.trim().is_empty() {
                    return Err("WebDAV server URL is empty".to_string());
                }
                if config.username.trim().is_empty() {
                    return Err("WebDAV username is empty".to_string());
                }
                if config.password.trim().is_empty() {
                    return Err("WebDAV password is empty".to_string());
                }
                if config.watch_folders.is_empty() {
                    return Err("WebDAV watch folders list is empty".to_string());
                }
                Ok(())
            }
            SourceType::S3 => {
                let _config: S3SourceConfig = serde_json::from_value(source.config.clone())
                    .map_err(|e| format!("Failed to parse S3 configuration: {}", e))?;
                Ok(())
            }
            SourceType::LocalFolder => {
                let _config: LocalFolderSourceConfig =
                    serde_json::from_value(source.config.clone()).map_err(|e| {
                        format!("Failed to parse Local Folder configuration: {}", e)
                    })?;
                Ok(())
            }
        }
    }

    async fn analyze_sync_patterns(
        source: &crate::models::Source,
        state: &Arc<AppState>,
    ) -> Result<SyncHealthAnalysis, Box<dyn std::error::Error + Send + Sync>> {
        let mut score_penalty = 0;
        let mut issues = Vec::new();

        // Check recent sync history
        let sync_stats = sqlx::query(
            r#"
            SELECT 
                COUNT(*) as total_syncs,
                SUM(CASE WHEN total_files_synced = 0 THEN 1 ELSE 0 END) as empty_syncs,
                MAX(last_sync_at) as last_sync,
                AVG(total_files_synced) as avg_files_per_sync
            FROM sources 
            WHERE id = $1 AND last_sync_at >= NOW() - INTERVAL '7 days'
            "#,
        )
        .bind(source.id)
        .fetch_one(state.db.get_pool())
        .await?;

        let total_syncs: i64 = sync_stats.try_get("total_syncs").unwrap_or(0);
        let empty_syncs: i64 = sync_stats.try_get("empty_syncs").unwrap_or(0);

        if total_syncs > 0 {
            let empty_sync_ratio = (empty_syncs as f64) / (total_syncs as f64);

            if empty_sync_ratio > 0.8 {
                score_penalty += 20;
                issues.push(serde_json::json!({
                    "type": "sync_pattern",
                    "severity": "warning",
                    "message": format!("High empty sync ratio: {:.1}% of recent syncs found no files", empty_sync_ratio * 100.0),
                    "recommendation": "This may indicate connectivity issues or that the source has no new content"
                }));
            }

            if total_syncs < 2
                && chrono::Utc::now()
                    .signed_duration_since(source.created_at)
                    .num_days()
                    > 1
            {
                score_penalty += 10;
                issues.push(serde_json::json!({
                    "type": "sync_pattern",
                    "severity": "info",
                    "message": "Very few syncs performed since source creation",
                    "recommendation": "Consider enabling auto-sync or manually triggering sync to ensure content is up to date"
                }));
            }
        }

        Ok(SyncHealthAnalysis {
            score_penalty,
            issues,
        })
    }

    async fn analyze_error_patterns(
        source: &crate::models::Source,
        _state: &Arc<AppState>,
    ) -> Result<ErrorAnalysis, Box<dyn std::error::Error + Send + Sync>> {
        let mut score_penalty = 0;
        let mut issues = Vec::new();

        // Check if source has recent errors
        if let Some(last_error_at) = source.last_error_at {
            let hours_since_error = chrono::Utc::now()
                .signed_duration_since(last_error_at)
                .num_hours();

            if hours_since_error < 24 {
                score_penalty += 15;
                issues.push(serde_json::json!({
                    "type": "error_pattern", 
                    "severity": "warning",
                    "message": format!("Recent error occurred {} hours ago", hours_since_error),
                    "recommendation": format!("Last error: {}", source.last_error.as_deref().unwrap_or("Unknown error"))
                }));
            }
        }

        // Check if source is in error state
        if source.status == crate::models::SourceStatus::Error {
            score_penalty += 25;
            issues.push(serde_json::json!({
                "type": "error_pattern",
                "severity": "critical",
                "message": "Source is currently in error state",
                "recommendation": "Review and fix the configuration or connectivity issues"
            }));
        }

        Ok(ErrorAnalysis {
            score_penalty,
            issues,
        })
    }
}
