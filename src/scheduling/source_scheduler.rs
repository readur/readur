use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::collections::HashMap;
use std::future::Future;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use chrono::Utc;
use uuid::Uuid;
use sqlx::Row;

use crate::{
    AppState,
    db::sources::{SyncCompletion, SYNC_CANCELLED_MESSAGE},
    models::{Source, SourceStatus, SourceType, LocalFolderSourceConfig, S3SourceConfig, WebDAVSourceConfig},
    services::sync_progress_tracker::SyncProgressTracker,
    services::webdav::SyncProgress,
};
use super::source_connectivity;
use super::source_sync::{SourceSyncService, SyncOutcome};

struct SyncHealthAnalysis {
    score_penalty: i32,
    issues: Vec<serde_json::Value>,
}

struct ErrorAnalysis {
    score_penalty: i32,
    issues: Vec<serde_json::Value>,
}

/// A sync task the scheduler started and has not yet seen exit.
#[derive(Debug, Clone)]
struct RunningSync {
    run_id: Uuid,
    token: CancellationToken,
    /// A stop was requested and the task is still winding down.
    stopping: bool,
}

type RunRegistry = Arc<Mutex<HashMap<Uuid, RunningSync>>>;

fn lock_registry(registry: &RunRegistry) -> std::sync::MutexGuard<'_, HashMap<Uuid, RunningSync>> {
    registry.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Why a sync could not be started.
#[derive(Debug, thiserror::Error)]
pub enum SyncStartError {
    #[error("Sync already running for this source")]
    AlreadyRunning,
    #[error("Sync is stopping for this source; try again once it has exited")]
    Stopping,
    #[error("Could not start sync - source is already syncing")]
    NotClaimed,
    #[error("Source not found")]
    NotFound,
    #[error("Configuration error: {0}")]
    Configuration(String),
    #[error("Database error: {0}")]
    Db(#[from] anyhow::Error),
}

/// Owned by the sync task's supervisor: once the task has ended, by return
/// or by panic, and its end has been recorded, the run's progress entry and
/// registry slot are released. Only the slot that still belongs to this run
/// is removed, so a later run is never affected.
struct SyncRunGuard {
    registry: RunRegistry,
    tracker: Arc<SyncProgressTracker>,
    source_id: Uuid,
    run_id: Uuid,
}

impl Drop for SyncRunGuard {
    fn drop(&mut self) {
        // Progress first: a new run can only register once the slot is free.
        self.tracker.unregister_sync(self.source_id);
        let mut registry = lock_registry(&self.registry);
        if registry.get(&self.source_id).is_some_and(|run| run.run_id == self.run_id) {
            registry.remove(&self.source_id);
        }
    }
}

pub struct SourceScheduler {
    state: Arc<AppState>,
    sync_service: SourceSyncService,
    check_interval: Duration,
    /// One entry per source with a sync task alive, whatever its state.
    running_syncs: RunRegistry,
}

impl SourceScheduler {
    pub fn new(state: Arc<AppState>) -> Self {
        let sync_service = SourceSyncService::new(state.clone());

        Self {
            state,
            sync_service,
            check_interval: Duration::from_secs(60), // Check every minute for due syncs
            running_syncs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn start(&self) {
        info!("Starting universal source sync scheduler");
        
        // First, check for any interrupted syncs that need to be resumed
        if let Err(e) = self.resume_interrupted_syncs().await {
            error!("Error resuming interrupted syncs: {}", e);
        }
        
        let mut interval_timer = interval(self.check_interval);
        
        loop {
            interval_timer.tick().await;
            
            if let Err(e) = self.check_and_sync_sources().await {
                error!("Error in source sync scheduler: {}", e);
            }
            
            // Run periodic validation checks for all sources
            if let Err(e) = self.run_periodic_validations().await {
                error!("Error in periodic validation checks: {}", e);
            }
        }
    }

    async fn resume_interrupted_syncs(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Checking for interrupted source syncs to resume");
        
        // First, reset any sources that are stuck in syncing state
        match self.state.db.reset_stuck_syncing_sources().await {
            Ok(reset_count) => {
                if reset_count > 0 {
                    info!("Reset {} sources that were stuck in syncing state from previous session", reset_count);
                }
            }
            Err(e) => {
                error!("Failed to reset stuck syncing sources: {}", e);
            }
        }
        
        // Get all enabled sources that might have been interrupted
        let sources = match self.state.db.get_sources_for_sync().await {
            Ok(sources) => {
                info!("Successfully loaded {} sources from database for sync check", sources.len());
                sources
            }
            Err(e) => {
                error!("Failed to load sources from database during startup: {}", e);
                return Err(e.into());
            }
        };
        
        for source in sources {
            info!("Processing source during startup check: ID={}, Name='{}', Type={}, Status={}", 
                  source.id, source.name, source.source_type.to_string(), source.status.to_string());
            
            // Validate source configuration before attempting any operations
            if let Err(e) = self.validate_source_config(&source) {
                error!("❌ CONFIGURATION ERROR for source '{}' (ID: {}): {}", 
                       source.name, source.id, e);
                error!("Source config JSON: {}", serde_json::to_string_pretty(&source.config).unwrap_or_else(|_| "Invalid JSON".to_string()));
                continue;
            }
            
            // Sources are already reset to idle by reset_stuck_syncing_sources
            // We could add logic here to resume specific sources if needed
            info!("Source {} is now ready for normal scheduling", source.name);
        }
        
        Ok(())
    }

    async fn check_and_sync_sources(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Get all sources that might need syncing
        let sources = self.state.db.get_sources_for_sync().await?;

        for source in sources {
            // Skip sources that are already in error status due to configuration issues
            if source.status == crate::models::SourceStatus::Error &&
               source.last_error.as_ref().map(|e| e.contains("Configuration error")).unwrap_or(false) {
                // Only log this once every hour to reduce spam
                if source.last_error_at.map(|t| chrono::Utc::now() - t > chrono::Duration::hours(1)).unwrap_or(true) {
                    warn!("⚠️ Skipping source '{}' (ID: {}) due to persistent configuration error: {}",
                          source.name, source.id, source.last_error.as_ref().unwrap_or(&"Unknown error".to_string()));
                }
                continue;
            }

            // Validate source configuration before checking if sync is due
            if let Err(e) = self.validate_source_config(&source) {
                error!("❌ CONFIGURATION ERROR during background sync check for source '{}' (ID: {}): {}",
                       source.name, source.id, e);
                error!("Source config JSON: {}", serde_json::to_string_pretty(&source.config).unwrap_or_else(|_| "Invalid JSON".to_string()));

                // Update source with error status
                if let Err(update_err) = sqlx::query(
                    r#"UPDATE sources SET status = 'error', last_error = $1, last_error_at = NOW(), updated_at = NOW() WHERE id = $2"#
                )
                .bind(crate::db::sanitize::DbText(&format!("Configuration error: {}", e)))
                .bind(source.id)
                .execute(self.state.db.get_pool())
                .await {
                    error!("Failed to update source error status: {}", update_err);
                }

                continue;
            }

            if self.is_sync_due(&source).await? {
                info!("Starting background sync for source: {} ({})", source.name, source.source_type);
                match self.launch_sync(source).await {
                    Ok(()) => {}
                    Err(e @ (SyncStartError::AlreadyRunning | SyncStartError::Stopping | SyncStartError::NotClaimed)) => {
                        info!("Background sync not started: {}", e);
                    }
                    Err(e) => error!("Background sync could not be started: {}", e),
                }
            }
        }

        Ok(())
    }

    async fn is_sync_due(&self, source: &crate::models::Source) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        // Get sync interval from source config
        let sync_interval_minutes = match source.source_type {
            SourceType::WebDAV => {
                let config: WebDAVSourceConfig = serde_json::from_value(source.config.clone())?;
                if !config.auto_sync { return Ok(false); }
                config.sync_interval_minutes
            }
            SourceType::LocalFolder => {
                let config: LocalFolderSourceConfig = serde_json::from_value(source.config.clone())?;
                if !config.auto_sync { return Ok(false); }
                config.sync_interval_minutes
            }
            SourceType::S3 => {
                let config: S3SourceConfig = serde_json::from_value(source.config.clone())?;
                if !config.auto_sync { return Ok(false); }
                config.sync_interval_minutes
            }
        };
        
        if sync_interval_minutes <= 0 {
            warn!("Invalid sync interval for source {}: {} minutes", source.name, sync_interval_minutes);
            return Ok(false);
        }

        // Check if a sync is already running
        if source.status.to_string() == "syncing" {
            info!("Sync already running for source {}", source.name);
            return Ok(false);
        }

        // Check last sync time
        if let Some(last_sync) = source.last_sync_at {
            let elapsed = Utc::now() - last_sync;
            let elapsed_minutes = elapsed.num_minutes();
            
            if elapsed_minutes < sync_interval_minutes as i64 {
                // Only log this occasionally to avoid spam
                if elapsed_minutes % 10 == 0 {
                    crate::debug_log!("SOURCE_SCHEDULER", "Sync not due for source {} (last sync {} minutes ago, interval {} minutes)", 
                        source.name, elapsed_minutes, sync_interval_minutes);
                }
                return Ok(false);
            }
            
            info!("Sync is due for source {} (last sync {} minutes ago, interval {} minutes)", 
                source.name, elapsed_minutes, sync_interval_minutes);
        } else {
            info!("No previous sync found for source {}, sync is due", source.name);
        }

        // Sync is due
        Ok(true)
    }

    pub async fn trigger_sync(&self, source_id: uuid::Uuid) -> Result<(), SyncStartError> {
        info!("Triggering manual sync for source {}", source_id);

        let source = self.state.db.get_source_by_id(source_id).await?.ok_or(SyncStartError::NotFound)?;
        self.validate_source_config(&source).map_err(SyncStartError::Configuration)?;
        self.launch_sync(source).await
    }

    /// Claims `source` and runs a full sync for it in the background. The
    /// scheduled and the manual path both come through here, so they behave
    /// the same: no wall-clock limit (the protocol clients bound each request),
    /// one completion record, notifications only for what actually happened.
    async fn launch_sync(&self, source: Source) -> Result<(), SyncStartError> {
        let sync_service = self.sync_service.clone();
        let state = self.state.clone();
        self.run_claimed(source.id, move |token, progress| async move {
            let enable_background_ocr = true; // Could be made configurable per source
            let result = sync_service.sync_claimed_source(&source, enable_background_ocr, token, &progress).await;
            Self::report_sync_result(&state, &source, &result).await;
        })
        .await
    }

    /// Post-run follow-up: a health check and a notification. A cancelled run
    /// is only logged; a run with failures gets a warning, a clean run with
    /// new files a success.
    async fn report_sync_result(state: &Arc<AppState>, source: &Source, result: &anyhow::Result<SyncOutcome>) {
        let notification = match result {
            Ok(outcome) if outcome.cancelled => {
                info!("Sync for source {} was cancelled after {} files", source.name, outcome.processed);
                None
            }
            Ok(outcome) => {
                if let Err(e) = Self::validate_source_health(source, state).await {
                    error!("Failed to perform validation check: {}", e);
                }
                let metadata = serde_json::json!({
                    "source_type": source.source_type.to_string(),
                    "source_id": source.id,
                    "files_processed": outcome.processed,
                    "files_failed": outcome.failed,
                    "folders_failed": outcome.folders_failed
                });
                match outcome.failure_summary() {
                    Some(summary) => Some(crate::models::CreateNotification {
                        notification_type: "warning".to_string(),
                        title: "Source Sync Finished With Errors".to_string(),
                        message: format!("{}: {}", source.name, summary),
                        action_url: Some("/sources".to_string()),
                        metadata: Some(metadata),
                    }),
                    None if outcome.processed > 0 => Some(crate::models::CreateNotification {
                        notification_type: "success".to_string(),
                        title: "Source Sync Completed".to_string(),
                        message: format!("Successfully processed {} files from {}", outcome.processed, source.name),
                        action_url: Some("/documents".to_string()),
                        metadata: Some(metadata),
                    }),
                    None => None,
                }
            }
            Err(e) => {
                error!("Sync failed for source {}: {}", source.name, e);
                Some(crate::models::CreateNotification {
                    notification_type: "error".to_string(),
                    title: "Source Sync Failed".to_string(),
                    message: format!("Sync failed for {}: {}", source.name, e),
                    action_url: Some("/sources".to_string()),
                    metadata: Some(serde_json::json!({
                        "source_type": source.source_type.to_string(),
                        "source_id": source.id,
                        "error": e.to_string()
                    })),
                })
            }
        };
        if let Some(notification) = notification {
            if let Err(e) = state.db.create_notification(source.user_id, &notification).await {
                error!("Failed to create sync notification: {}", e);
            }
        }
    }

    /// Claims the source (registry slot plus `start_sync_atomic`) and spawns
    /// `run` as its sync task, handing it the run's cancellation token and
    /// progress. A supervisor task awaits it, records a panicking task as a
    /// failed sync so the source does not stay `syncing`, and only then
    /// releases the slot and progress: no new run can claim the source
    /// while the abnormal end of the previous one is still being written.
    /// Refused while a task for the source is alive, including one that is
    /// still stopping.
    pub async fn run_claimed<F, Fut>(&self, source_id: Uuid, run: F) -> Result<(), SyncStartError>
    where
        F: FnOnce(CancellationToken, Arc<SyncProgress>) -> Fut,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let run_id = Uuid::new_v4();
        let token = CancellationToken::new();

        // Reserve the slot before writing the claim, so a stop request that
        // arrives in between already finds the token.
        {
            let mut registry = lock_registry(&self.running_syncs);
            if let Some(existing) = registry.get(&source_id) {
                return Err(if existing.stopping { SyncStartError::Stopping } else { SyncStartError::AlreadyRunning });
            }
            registry.insert(source_id, RunningSync { run_id, token: token.clone(), stopping: false });
        }
        let claimed = match self.state.db.start_sync_atomic(source_id).await {
            Ok(claimed) => claimed,
            Err(e) => {
                self.release_run(source_id, run_id);
                return Err(SyncStartError::Db(e));
            }
        };
        if !claimed {
            self.release_run(source_id, run_id);
            return Err(SyncStartError::NotClaimed);
        }

        let progress = Arc::new(SyncProgress::new());
        self.state.sync_progress_tracker.register_sync(source_id, progress.clone());
        let guard = SyncRunGuard {
            registry: self.running_syncs.clone(),
            tracker: self.state.sync_progress_tracker.clone(),
            source_id,
            run_id,
        };
        let task = tokio::spawn(run(token, progress));

        let db = self.state.db.clone();
        tokio::spawn(async move {
            // Held until the abnormal end, if any, has been recorded.
            let _guard = guard;
            if let Err(join_error) = task.await {
                let reason = if join_error.is_panic() { "sync task panicked" } else { "sync task was aborted" };
                error!("{} for source {}: {}", reason, source_id, join_error);
                let message = format!("{}: {}", reason, join_error);
                let completion = SyncCompletion { status: SourceStatus::Error, error: Some(&message), files_synced: 0, files_pending: None };
                match db.finish_sync(source_id, &completion).await {
                    Ok(true) => {}
                    Ok(false) => warn!("Source {} was no longer marked as syncing after its task ended abnormally", source_id),
                    Err(e) => error!("Failed to record the abnormal end of sync for source {}: {}", source_id, e),
                }
            }
        });

        Ok(())
    }

    fn release_run(&self, source_id: Uuid, run_id: Uuid) {
        let mut registry = lock_registry(&self.running_syncs);
        if registry.get(&source_id).is_some_and(|run| run.run_id == run_id) {
            registry.remove(&source_id);
        }
    }

    /// Whether a sync task for the source is alive (running or stopping).
    pub fn is_running(&self, source_id: Uuid) -> bool {
        lock_registry(&self.running_syncs).contains_key(&source_id)
    }

    /// Asks the running sync to stop. The task keeps its registry slot until
    /// it has actually exited, so a new sync cannot start underneath it; the
    /// status and progress are updated right away for the UI, and the task
    /// records its own counters when it ends.
    pub async fn stop_sync(&self, source_id: Uuid) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Stopping sync for source {}", source_id);

        let token = {
            let mut registry = lock_registry(&self.running_syncs);
            registry.get_mut(&source_id).map(|run| {
                run.stopping = true;
                run.token.clone()
            })
        };
        let Some(token) = token else {
            return Err("No running sync found for this source".into());
        };

        token.cancel();
        info!("Cancellation signal sent for source {}", source_id);

        if let Err(e) = self.state.db.update_source_status_atomic(
            source_id,
            Some(SourceStatus::Syncing),
            SourceStatus::Idle,
            Some(SYNC_CANCELLED_MESSAGE),
        ).await {
            error!("Failed to update source status after cancellation: {}", e);
        }
        self.state.sync_progress_tracker.unregister_sync(source_id);

        Ok(())
    }

    /// Stops the running sync, if any, and waits up to `timeout` for its task
    /// to exit. Returns whether no task is alive at the end.
    pub async fn stop_sync_and_wait(&self, source_id: Uuid, timeout: Duration) -> bool {
        if self.stop_sync(source_id).await.is_err() {
            return !self.is_running(source_id);
        }
        let deadline = tokio::time::Instant::now() + timeout;
        while self.is_running(source_id) {
            if tokio::time::Instant::now() >= deadline {
                warn!("Sync task for source {} is still running {:?} after the stop request", source_id, timeout);
                return false;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        true
    }

    /// Force reset a source that may be stuck in syncing state
    /// This is used as a fail-safe mechanism for race conditions
    pub async fn force_reset_source(&self, source_id: Uuid) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Force resetting potentially stuck source {}", source_id);

        lock_registry(&self.running_syncs).remove(&source_id);
        self.state.sync_progress_tracker.unregister_sync(source_id);

        // Force reset database status to idle
        if let Err(e) = sqlx::query(
            "UPDATE sources SET status = 'idle', last_error = 'Force reset due to stuck sync', last_error_at = NOW(), updated_at = NOW() WHERE id = $1 AND status = 'syncing'"
        )
        .bind(source_id)
        .execute(self.state.db.get_pool())
        .await {
            error!("Failed to force reset source status: {}", e);
            return Err(e.into());
        }

        info!("Source {} force reset completed", source_id);
        Ok(())
    }

    /// Validates a source configuration and provides detailed error messages for debugging
    fn validate_source_config(&self, source: &crate::models::Source) -> Result<(), String> {
        use crate::models::{SourceType, WebDAVSourceConfig, S3SourceConfig, LocalFolderSourceConfig};
        
        match source.source_type {
            SourceType::WebDAV => {
                // Attempt to deserialize WebDAV config
                let config: WebDAVSourceConfig = serde_json::from_value(source.config.clone())
                    .map_err(|e| format!("Failed to parse WebDAV configuration JSON: {}", e))?;
                
                // Validate server URL format
                self.validate_webdav_url(&config.server_url, &source.name)?;
                
                // Additional WebDAV validations
                if config.username.trim().is_empty() {
                    return Err(format!("WebDAV username cannot be empty"));
                }
                
                if config.password.trim().is_empty() {
                    return Err(format!("WebDAV password cannot be empty"));
                }
                
                if config.watch_folders.is_empty() {
                    return Err(format!("WebDAV watch_folders cannot be empty"));
                }
                
                Ok(())
            }
            SourceType::S3 => {
                let _config: S3SourceConfig = serde_json::from_value(source.config.clone())
                    .map_err(|e| format!("Failed to parse S3 configuration JSON: {}", e))?;
                Ok(())
            }
            SourceType::LocalFolder => {
                let _config: LocalFolderSourceConfig = serde_json::from_value(source.config.clone())
                    .map_err(|e| format!("Failed to parse Local Folder configuration JSON: {}", e))?;
                Ok(())
            }
        }
    }

    /// Validates WebDAV server URL and provides specific error messages
    fn validate_webdav_url(&self, server_url: &str, source_name: &str) -> Result<(), String> {
        if server_url.trim().is_empty() {
            return Err(format!("WebDAV server_url is empty"));
        }
        
        // Normalize URL by adding protocol if missing (consistent with WebDAVConfig)
        let normalized_url = crate::services::webdav::config::WebDAVConfig::normalize_server_url(server_url);
        
        // Try to parse the normalized URL to catch other issues
        match reqwest::Url::parse(&normalized_url) {
            Ok(url) => {
                if url.scheme() != "http" && url.scheme() != "https" {
                    return Err(format!(
                        "WebDAV server_url has invalid scheme '{}'. Only 'http' and 'https' are supported. \
                         Current URL: '{}'", 
                        url.scheme(), normalized_url
                    ));
                }
                
                if url.host_str().is_none() {
                    return Err(format!(
                        "WebDAV server_url is missing hostname. \
                         Current URL: '{}'. \
                         Example: https://cloud.example.com", 
                        normalized_url
                    ));
                }
                
                crate::debug_log!("SOURCE_SCHEDULER", "✅ WebDAV URL validation passed for source '{}': {} (normalized to: {})", source_name, server_url, normalized_url);
                Ok(())
            }
            Err(e) => {
                Err(format!(
                    "WebDAV server_url is not a valid URL: {}. \
                     Current value: '{}' (normalized to: '{}'). \
                     The URL must be absolute and include the full domain. \
                     Examples: \
                     - https://cloud.example.com \
                     - http://192.168.1.100:8080/webdav \
                     - https://nextcloud.mydomain.com", 
                    e, server_url, normalized_url
                ))
            }
        }
    }

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
                    if validation_status == "healthy" { validation_status = "warning"; }
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
                    if validation_status == "healthy" { validation_status = "warning"; }
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
                    if validation_status == "healthy" { validation_status = "warning"; }
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
        let validation_issues_json = serde_json::to_string(&validation_issues)
            .unwrap_or_else(|_| "[]".to_string());

        if let Err(e) = sqlx::query(
            r#"
            UPDATE sources 
            SET validation_status = $1, 
                last_validation_at = NOW(), 
                validation_score = $2,
                validation_issues = $3,
                updated_at = NOW()
            WHERE id = $4
            "#
        )
        .bind(validation_status)
        .bind(validation_score)
        .bind(validation_issues_json)
        .bind(source.id)
        .execute(state.db.get_pool())
        .await {
            error!("Failed to update validation status: {}", e);
        }

        // Send notification if there are critical issues
        if validation_status == "critical" || validation_score < 50 {
            let notification = crate::models::CreateNotification {
                notification_type: if validation_status == "critical" { "error" } else { "warning" }.to_string(),
                title: format!("Source Validation {}", if validation_status == "critical" { "Failed" } else { "Warning" }),
                message: format!("Source {} has validation issues (score: {})", source.name, validation_score),
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

            if let Err(e) = state.db.create_notification(source.user_id, &notification).await {
                error!("Failed to create validation notification: {}", e);
            }
        }

        info!("✅ Validation completed for {}: {} (score: {})", source.name, validation_status, validation_score);
        Ok(())
    }

    fn validate_source_config_detailed(source: &crate::models::Source) -> Result<(), String> {
        // Reuse existing validation logic but return more detailed errors
        Self::validate_source_config_static(source)
    }

    fn validate_source_config_static(source: &crate::models::Source) -> Result<(), String> {
        use crate::models::{SourceType, WebDAVSourceConfig, S3SourceConfig, LocalFolderSourceConfig};
        
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
                let _config: LocalFolderSourceConfig = serde_json::from_value(source.config.clone())
                    .map_err(|e| format!("Failed to parse Local Folder configuration: {}", e))?;
                Ok(())
            }
        }
    }

    async fn analyze_sync_patterns(
        source: &crate::models::Source,
        state: &Arc<AppState>
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
            "#
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

            if total_syncs < 2 && chrono::Utc::now().signed_duration_since(source.created_at).num_days() > 1 {
                score_penalty += 10;
                issues.push(serde_json::json!({
                    "type": "sync_pattern",
                    "severity": "info",
                    "message": "Very few syncs performed since source creation",
                    "recommendation": "Consider enabling auto-sync or manually triggering sync to ensure content is up to date"
                }));
            }
        }

        Ok(SyncHealthAnalysis { score_penalty, issues })
    }


    async fn analyze_error_patterns(
        source: &crate::models::Source,
        _state: &Arc<AppState>
    ) -> Result<ErrorAnalysis, Box<dyn std::error::Error + Send + Sync>> {
        let mut score_penalty = 0;
        let mut issues = Vec::new();

        // Check if source has recent errors
        if let Some(last_error_at) = source.last_error_at {
            let hours_since_error = chrono::Utc::now().signed_duration_since(last_error_at).num_hours();
            
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

        Ok(ErrorAnalysis { score_penalty, issues })
    }

    async fn run_periodic_validations(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Get all enabled sources
        let sources = self.state.db.get_sources_for_sync().await?;
        
        for source in sources {
            // Only validate if it's been more than 30 minutes since last validation
            let should_validate = if let Some(last_validation) = source.last_validation_at {
                chrono::Utc::now().signed_duration_since(last_validation).num_minutes() > 30
            } else {
                true // Never validated before
            };
            
            if should_validate && source.enabled {
                info!("Running periodic validation for source: {}", source.name);
                
                // Run validation in background to avoid blocking
                let source_clone = source.clone();
                let state_clone = self.state.clone();
                tokio::spawn(async move {
                    if let Err(e) = Self::validate_source_health(&source_clone, &state_clone).await {
                        error!("Periodic validation failed for source {}: {}", source_clone.name, e);
                    }
                });
            }
        }
        
        Ok(())
    }
}