use chrono::Utc;
use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use uuid::Uuid;

use super::source_sync::{SourceSyncService, SyncOutcome};
use crate::{
    db::sources::{SyncCompletion, SYNC_CANCELLED_MESSAGE},
    models::{
        LocalFolderSourceConfig, S3SourceConfig, Source, SourceStatus, SourceType,
        WebDAVSourceConfig,
    },
    services::sync_progress_tracker::SyncProgressTracker,
    services::webdav::SyncProgress,
    AppState,
};

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
    registry
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
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

/// Why a stop request did nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum StopSyncError {
    /// No sync task for the source is alive in this scheduler.
    #[error("No running sync found for this source")]
    NotRunning,
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
        if registry
            .get(&self.source_id)
            .is_some_and(|run| run.run_id == self.run_id)
        {
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

    async fn resume_interrupted_syncs(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Checking for interrupted source syncs to resume");

        // First, reset any sources that are stuck in syncing state
        match self.state.db.reset_stuck_syncing_sources().await {
            Ok(reset_count) => {
                if reset_count > 0 {
                    info!(
                        "Reset {} sources that were stuck in syncing state from previous session",
                        reset_count
                    );
                }
            }
            Err(e) => {
                error!("Failed to reset stuck syncing sources: {}", e);
            }
        }

        // Get all enabled sources that might have been interrupted
        let sources = match self.state.db.get_sources_for_sync().await {
            Ok(sources) => {
                info!(
                    "Successfully loaded {} sources from database for sync check",
                    sources.len()
                );
                sources
            }
            Err(e) => {
                error!("Failed to load sources from database during startup: {}", e);
                return Err(e.into());
            }
        };

        for source in sources {
            info!(
                "Processing source during startup check: ID={}, Name='{}', Type={}, Status={}",
                source.id,
                source.name,
                source.source_type.to_string(),
                source.status.to_string()
            );

            // Validate source configuration before attempting any operations
            if let Err(e) = self.validate_source_config(&source) {
                error!(
                    "❌ CONFIGURATION ERROR for source '{}' (ID: {}): {}",
                    source.name, source.id, e
                );
                error!(
                    "Source config JSON: {}",
                    serde_json::to_string_pretty(&source.config)
                        .unwrap_or_else(|_| "Invalid JSON".to_string())
                );
                continue;
            }

            // Sources are already reset to idle by reset_stuck_syncing_sources
            // We could add logic here to resume specific sources if needed
            info!("Source {} is now ready for normal scheduling", source.name);
        }

        Ok(())
    }

    /// One scheduler tick: starts the background sync of every source that
    /// is due and whose configuration is valid.
    pub async fn check_and_sync_sources(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Get all sources that might need syncing
        let sources = self.state.db.get_sources_for_sync().await?;

        for source in sources {
            // The configuration itself decides, not the text of an earlier
            // error: a source whose configuration is invalid is never synced.
            if let Err(e) = self.validate_source_config(&source) {
                self.record_configuration_error(&source, &e).await;
                continue;
            }

            if self.is_sync_due(&source).await? {
                self.launch_due_sync(source).await;
            }
        }

        Ok(())
    }

    async fn launch_due_sync(&self, source: Source) {
        info!(
            "Starting background sync for source: {} ({})",
            source.name, source.source_type
        );
        match self.launch_sync(source).await {
            Ok(()) => {}
            Err(
                e @ (SyncStartError::AlreadyRunning
                | SyncStartError::Stopping
                | SyncStartError::NotClaimed),
            ) => info!("Background sync not started: {}", e),
            Err(e) => error!("Background sync could not be started: {}", e),
        }
    }

    /// Marks a source whose configuration is invalid as failed. A source
    /// already in the error state with an error recorded within the last hour
    /// is left alone, so the check that runs every minute neither rewrites
    /// nor re-logs it.
    async fn record_configuration_error(&self, source: &Source, error: &str) {
        let recorded_recently = source.status == SourceStatus::Error
            && source
                .last_error_at
                .is_some_and(|at| Utc::now() - at <= chrono::Duration::hours(1));
        if recorded_recently {
            return;
        }
        error!(
            "Configuration error for source '{}' ({}): {}",
            source.name, source.id, error
        );
        let message = format!("Configuration error: {}", error);
        if let Err(e) = sqlx::query(
            "UPDATE sources SET status = 'error', last_error = $1, last_error_at = NOW(), updated_at = NOW() WHERE id = $2",
        )
        .bind(crate::db::sanitize::DbText(&message))
        .bind(source.id)
        .execute(self.state.db.get_pool())
        .await
        {
            error!("Failed to update source error status: {}", e);
        }
    }

    async fn is_sync_due(
        &self,
        source: &crate::models::Source,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        // Get sync interval from source config
        let sync_interval_minutes = match source.source_type {
            SourceType::WebDAV => {
                let config: WebDAVSourceConfig = serde_json::from_value(source.config.clone())?;
                if !config.auto_sync {
                    return Ok(false);
                }
                config.sync_interval_minutes
            }
            SourceType::LocalFolder => {
                let config: LocalFolderSourceConfig =
                    serde_json::from_value(source.config.clone())?;
                if !config.auto_sync {
                    return Ok(false);
                }
                config.sync_interval_minutes
            }
            SourceType::S3 => {
                let config: S3SourceConfig = serde_json::from_value(source.config.clone())?;
                if !config.auto_sync {
                    return Ok(false);
                }
                config.sync_interval_minutes
            }
        };

        if sync_interval_minutes <= 0 {
            warn!(
                "Invalid sync interval for source {}: {} minutes",
                source.name, sync_interval_minutes
            );
            return Ok(false);
        }

        // Check if a sync is already running
        if source.status == SourceStatus::Syncing {
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

            info!(
                "Sync is due for source {} (last sync {} minutes ago, interval {} minutes)",
                source.name, elapsed_minutes, sync_interval_minutes
            );
        } else {
            info!(
                "No previous sync found for source {}, sync is due",
                source.name
            );
        }

        // Sync is due
        Ok(true)
    }

    pub async fn trigger_sync(&self, source_id: uuid::Uuid) -> Result<(), SyncStartError> {
        info!("Triggering manual sync for source {}", source_id);

        let source = self
            .state
            .db
            .get_source_by_id(source_id)
            .await?
            .ok_or(SyncStartError::NotFound)?;
        self.validate_source_config(&source)
            .map_err(SyncStartError::Configuration)?;
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
            let result = sync_service
                .sync_claimed_source(&source, enable_background_ocr, token, &progress)
                .await;
            Self::report_sync_result(&state, &source, &result).await;
        })
        .await
    }

    /// Post-run follow-up: a health check and a notification. A cancelled run
    /// is only logged; a run with failures gets a warning, a clean run with
    /// new files a success.
    async fn report_sync_result(
        state: &Arc<AppState>,
        source: &Source,
        result: &anyhow::Result<SyncOutcome>,
    ) {
        let notification = match result {
            Ok(outcome) if outcome.cancelled => {
                info!(
                    "Sync for source {} was cancelled after {} files",
                    source.name, outcome.processed
                );
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
                        message: format!(
                            "Successfully processed {} files from {}",
                            outcome.processed, source.name
                        ),
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
            if let Err(e) = state
                .db
                .create_notification(source.user_id, &notification)
                .await
            {
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
                return Err(if existing.stopping {
                    SyncStartError::Stopping
                } else {
                    SyncStartError::AlreadyRunning
                });
            }
            registry.insert(
                source_id,
                RunningSync {
                    run_id,
                    token: token.clone(),
                    stopping: false,
                },
            );
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
        self.state
            .sync_progress_tracker
            .register_sync(source_id, progress.clone());
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
                let reason = if join_error.is_panic() {
                    "sync task panicked"
                } else {
                    "sync task was aborted"
                };
                error!("{} for source {}: {}", reason, source_id, join_error);
                let message = format!("{}: {}", reason, join_error);
                let completion = SyncCompletion {
                    status: SourceStatus::Error,
                    error: Some(&message),
                    files_synced: 0,
                    files_pending: None,
                };
                match db.finish_sync(source_id, &completion).await {
                    Ok(true) => {}
                    Ok(false) => warn!(
                        "Source {} was no longer marked as syncing after its task ended abnormally",
                        source_id
                    ),
                    Err(e) => error!(
                        "Failed to record the abnormal end of sync for source {}: {}",
                        source_id, e
                    ),
                }
            }
        });

        Ok(())
    }

    fn release_run(&self, source_id: Uuid, run_id: Uuid) {
        let mut registry = lock_registry(&self.running_syncs);
        if registry
            .get(&source_id)
            .is_some_and(|run| run.run_id == run_id)
        {
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
    pub async fn stop_sync(&self, source_id: Uuid) -> Result<(), StopSyncError> {
        info!("Stopping sync for source {}", source_id);

        let token = {
            let mut registry = lock_registry(&self.running_syncs);
            registry.get_mut(&source_id).map(|run| {
                run.stopping = true;
                run.token.clone()
            })
        };
        let Some(token) = token else {
            return Err(StopSyncError::NotRunning);
        };

        token.cancel();
        info!("Cancellation signal sent for source {}", source_id);

        if let Err(e) = self
            .state
            .db
            .update_source_status_atomic(
                source_id,
                Some(SourceStatus::Syncing),
                SourceStatus::Idle,
                Some(SYNC_CANCELLED_MESSAGE),
            )
            .await
        {
            error!("Failed to update source status after cancellation: {}", e);
        }
        self.state.sync_progress_tracker.unregister_sync(source_id);

        Ok(())
    }

    /// Stops the running sync, if any, and waits up to `timeout` for its task
    /// to exit. Returns whether no task is alive at the end.
    pub async fn stop_sync_and_wait(&self, source_id: Uuid, timeout: Duration) -> bool {
        if let Err(StopSyncError::NotRunning) = self.stop_sync(source_id).await {
            return !self.is_running(source_id);
        }
        let deadline = tokio::time::Instant::now() + timeout;
        while self.is_running(source_id) {
            if tokio::time::Instant::now() >= deadline {
                warn!(
                    "Sync task for source {} is still running {:?} after the stop request",
                    source_id, timeout
                );
                return false;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        true
    }

    /// Force reset a source that may be stuck in syncing state
    /// This is used as a fail-safe mechanism for race conditions
    pub async fn force_reset_source(
        &self,
        source_id: Uuid,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
        use crate::models::{
            LocalFolderSourceConfig, S3SourceConfig, SourceType, WebDAVSourceConfig,
        };

        match source.source_type {
            SourceType::WebDAV => {
                // Attempt to deserialize WebDAV config
                let config: WebDAVSourceConfig = serde_json::from_value(source.config.clone())
                    .map_err(|e| format!("Failed to parse WebDAV configuration JSON: {}", e))?;

                // Validate server URL format
                self.validate_webdav_url(&config.server_url, &source.name)?;

                // Additional WebDAV validations
                if config.username.trim().is_empty() {
                    return Err("WebDAV username cannot be empty".to_string());
                }

                if config.password.trim().is_empty() {
                    return Err("WebDAV password cannot be empty".to_string());
                }

                if config.watch_folders.is_empty() {
                    return Err("WebDAV watch_folders cannot be empty".to_string());
                }

                Ok(())
            }
            SourceType::S3 => {
                let _config: S3SourceConfig = serde_json::from_value(source.config.clone())
                    .map_err(|e| format!("Failed to parse S3 configuration JSON: {}", e))?;
                Ok(())
            }
            SourceType::LocalFolder => {
                let _config: LocalFolderSourceConfig =
                    serde_json::from_value(source.config.clone()).map_err(|e| {
                        format!("Failed to parse Local Folder configuration JSON: {}", e)
                    })?;
                Ok(())
            }
        }
    }

    /// Validates WebDAV server URL and provides specific error messages
    fn validate_webdav_url(&self, server_url: &str, source_name: &str) -> Result<(), String> {
        if server_url.trim().is_empty() {
            return Err("WebDAV server_url is empty".to_string());
        }

        // Normalize URL by adding protocol if missing (consistent with WebDAVConfig)
        let normalized_url =
            crate::services::webdav::config::WebDAVConfig::normalize_server_url(server_url);

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

                crate::debug_log!(
                    "SOURCE_SCHEDULER",
                    "✅ WebDAV URL validation passed for source '{}': {} (normalized to: {})",
                    source_name,
                    server_url,
                    normalized_url
                );
                Ok(())
            }
            Err(e) => Err(format!(
                "WebDAV server_url is not a valid URL: {}. \
                     Current value: '{}' (normalized to: '{}'). \
                     The URL must be absolute and include the full domain. \
                     Examples: \
                     - https://cloud.example.com \
                     - http://192.168.1.100:8080/webdav \
                     - https://nextcloud.mydomain.com",
                e, server_url, normalized_url
            )),
        }
    }

    async fn run_periodic_validations(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Get all enabled sources
        let sources = self.state.db.get_sources_for_sync().await?;

        for source in sources {
            // Only validate if it's been more than 30 minutes since last validation
            let should_validate = if let Some(last_validation) = source.last_validation_at {
                chrono::Utc::now()
                    .signed_duration_since(last_validation)
                    .num_minutes()
                    > 30
            } else {
                true // Never validated before
            };

            if should_validate && source.enabled {
                info!("Running periodic validation for source: {}", source.name);

                // Run validation in background to avoid blocking
                let source_clone = source.clone();
                let state_clone = self.state.clone();
                tokio::spawn(async move {
                    if let Err(e) = Self::validate_source_health(&source_clone, &state_clone).await
                    {
                        error!(
                            "Periodic validation failed for source {}: {}",
                            source_clone.name, e
                        );
                    }
                });
            }
        }

        Ok(())
    }
}
