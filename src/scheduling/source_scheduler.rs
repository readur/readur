use std::sync::Arc;
use std::time::Duration;
use std::collections::HashMap;
use tokio::time::interval;
use tokio::sync::{Mutex, RwLock};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use chrono::Utc;
use uuid::Uuid;

use crate::{
    AppState,
    models::{SourceType, LocalFolderSourceConfig, S3SourceConfig, WebDAVSourceConfig},
};
use super::source_sync::SourceSyncService;

pub struct SourceScheduler {
    state: Arc<AppState>,
    sync_service: SourceSyncService,
    check_interval: Duration,
    // Track running sync tasks and their cancellation tokens
    running_syncs: Arc<RwLock<HashMap<Uuid, CancellationToken>>>,
}

impl SourceScheduler {
    pub fn new(state: Arc<AppState>) -> Self {
        let sync_service = SourceSyncService::new(state.clone());
        
        Self {
            state,
            sync_service,
            check_interval: Duration::from_secs(60), // Check every minute for due syncs
            running_syncs: Arc::new(RwLock::new(HashMap::new())),
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
        }
    }

    async fn resume_interrupted_syncs(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Checking for interrupted source syncs to resume");
        
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
            
            // Check if this source was likely interrupted during sync
            // This is a simplified check - you might want to add specific interrupted tracking
            if source.status.to_string() == "syncing" {
                info!("Found potentially interrupted sync for source {}, will resume", source.name);
                
                // Reset status and trigger new sync
                if let Err(e) = sqlx::query(
                    r#"UPDATE sources SET status = 'idle', updated_at = NOW() WHERE id = $1"#
                )
                .bind(source.id)
                .execute(self.state.db.get_pool())
                .await {
                    error!("Failed to reset interrupted source status: {}", e);
                    continue;
                }
                
                // Always resume interrupted syncs regardless of auto_sync setting
                // This ensures that manually triggered syncs that were interrupted by server restart
                // will continue downloading files instead of just starting OCR on existing files
                let should_resume = true;
                
                if should_resume {
                    info!("Resuming interrupted sync for source {}", source.name);
                    
                    let sync_service = self.sync_service.clone();
                    let source_clone = source.clone();
                    let state_clone = self.state.clone();
                    
                    tokio::spawn(async move {
                        // Get user's OCR setting - simplified, you might want to store this in source config
                        let enable_background_ocr = true; // Default to true, could be made configurable per source
                        
                        match sync_service.sync_source(&source_clone, enable_background_ocr).await {
                            Ok(files_processed) => {
                                info!("Resumed sync completed for source {}: {} files processed", 
                                      source_clone.name, files_processed);
                                
                                // Create notification for successful resume
                                let notification = crate::models::CreateNotification {
                                    notification_type: "success".to_string(),
                                    title: "Source Sync Resumed".to_string(),
                                    message: format!("Resumed sync for {} after server restart. Processed {} files", 
                                                   source_clone.name, files_processed),
                                    action_url: Some("/sources".to_string()),
                                    metadata: Some(serde_json::json!({
                                        "source_type": source_clone.source_type.to_string(),
                                        "source_id": source_clone.id,
                                        "files_processed": files_processed
                                    })),
                                };
                                
                                if let Err(e) = state_clone.db.create_notification(source_clone.user_id, &notification).await {
                                    error!("Failed to create resume notification: {}", e);
                                }
                            }
                            Err(e) => {
                                error!("Resumed sync failed for source {}: {}", source_clone.name, e);
                            }
                        }
                    });
                }
            }
        }
        
        Ok(())
    }

    async fn check_and_sync_sources(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Get all sources that might need syncing
        let sources = self.state.db.get_sources_for_sync().await?;
        
        for source in sources {
            // Validate source configuration before checking if sync is due
            if let Err(e) = self.validate_source_config(&source) {
                error!("❌ CONFIGURATION ERROR during background sync check for source '{}' (ID: {}): {}", 
                       source.name, source.id, e);
                error!("Source config JSON: {}", serde_json::to_string_pretty(&source.config).unwrap_or_else(|_| "Invalid JSON".to_string()));
                
                // Update source with error status
                if let Err(update_err) = sqlx::query(
                    r#"UPDATE sources SET status = 'error', last_error = $1, last_error_at = NOW(), updated_at = NOW() WHERE id = $2"#
                )
                .bind(format!("Configuration error: {}", e))
                .bind(source.id)
                .execute(self.state.db.get_pool())
                .await {
                    error!("Failed to update source error status: {}", update_err);
                }
                
                continue;
            }
            
            // Check if sync is due for this source
            if self.is_sync_due(&source).await? {
                info!("Starting background sync for source: {} ({})", source.name, source.source_type);
                
                let sync_service = self.sync_service.clone();
                let source_clone = source.clone();
                let state_clone = self.state.clone();
                let running_syncs_clone = self.running_syncs.clone();
                
                // Create cancellation token for this sync
                let cancellation_token = CancellationToken::new();
                
                // Register the sync task
                {
                    let mut running_syncs = running_syncs_clone.write().await;
                    running_syncs.insert(source.id, cancellation_token.clone());
                }
                
                // Start sync in background task
                let sync_handle = tokio::spawn(async move {
                    // Get user's OCR setting - simplified, you might want to store this in source config  
                    let enable_background_ocr = true; // Default to true, could be made configurable per source
                    
                    // Pass cancellation token to sync service
                    match sync_service.sync_source_with_cancellation(&source_clone, enable_background_ocr, cancellation_token.clone()).await {
                        Ok(files_processed) => {
                            info!("Background sync completed for source {}: {} files processed", 
                                  source_clone.name, files_processed);
                            
                            // Update last sync time
                            if let Err(e) = sqlx::query(
                                r#"UPDATE sources 
                                   SET last_sync_at = NOW(), 
                                       total_files_synced = total_files_synced + $2,
                                       updated_at = NOW()
                                   WHERE id = $1"#
                            )
                            .bind(source_clone.id)
                            .bind(files_processed as i64)
                            .execute(state_clone.db.get_pool())
                            .await {
                                error!("Failed to update source sync time: {}", e);
                            }
                            
                            // Send notification if files were processed
                            if files_processed > 0 {
                                let notification = crate::models::CreateNotification {
                                    notification_type: "success".to_string(),
                                    title: "Source Sync Completed".to_string(),
                                    message: format!("Successfully processed {} files from {}", 
                                                   files_processed, source_clone.name),
                                    action_url: Some("/documents".to_string()),
                                    metadata: Some(serde_json::json!({
                                        "source_type": source_clone.source_type.to_string(),
                                        "source_id": source_clone.id,
                                        "files_processed": files_processed
                                    })),
                                };
                                
                                if let Err(e) = state_clone.db.create_notification(source_clone.user_id, &notification).await {
                                    error!("Failed to create success notification: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            error!("Background sync failed for source {}: {}", source_clone.name, e);
                            
                            // Send error notification
                            let notification = crate::models::CreateNotification {
                                notification_type: "error".to_string(),
                                title: "Source Sync Failed".to_string(),
                                message: format!("Sync failed for {}: {}", source_clone.name, e),
                                action_url: Some("/sources".to_string()),
                                metadata: Some(serde_json::json!({
                                    "source_type": source_clone.source_type.to_string(),
                                    "source_id": source_clone.id,
                                    "error": e.to_string()
                                })),
                            };
                            
                            if let Err(e) = state_clone.db.create_notification(source_clone.user_id, &notification).await {
                                error!("Failed to create error notification: {}", e);
                            }
                        }
                    }
                    
                    // Cleanup: Remove the sync from running list
                    {
                        let mut running_syncs = running_syncs_clone.write().await;
                        running_syncs.remove(&source_clone.id);
                    }
                });
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
                    info!("Sync not due for source {} (last sync {} minutes ago, interval {} minutes)", 
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

    pub async fn trigger_sync(&self, source_id: uuid::Uuid) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Triggering manual sync for source {}", source_id);
        
        if let Some(source) = self.state.db.get_source_by_id(source_id).await? {
            let sync_service = self.sync_service.clone();
            let state_clone = self.state.clone();
            let running_syncs_clone = self.running_syncs.clone();
            
            // Create cancellation token for this sync
            let cancellation_token = CancellationToken::new();
            
            // Register the sync task
            {
                let mut running_syncs = running_syncs_clone.write().await;
                running_syncs.insert(source_id, cancellation_token.clone());
            }
            
            tokio::spawn(async move {
                let enable_background_ocr = true; // Could be made configurable
                
                match sync_service.sync_source_with_cancellation(&source, enable_background_ocr, cancellation_token).await {
                    Ok(files_processed) => {
                        info!("Manual sync completed for source {}: {} files processed", 
                              source.name, files_processed);
                        
                        // Update sync stats
                        if let Err(e) = sqlx::query(
                            r#"UPDATE sources 
                               SET last_sync_at = NOW(), 
                                   total_files_synced = total_files_synced + $2,
                                   updated_at = NOW()
                               WHERE id = $1"#
                        )
                        .bind(source.id)
                        .bind(files_processed as i64)
                        .execute(state_clone.db.get_pool())
                        .await {
                            error!("Failed to update source sync stats: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("Manual sync failed for source {}: {}", source.name, e);
                    }
                }
                
                // Cleanup: Remove the sync from running list
                {
                    let mut running_syncs = running_syncs_clone.write().await;
                    running_syncs.remove(&source.id);
                }
            });
            
            Ok(())
        } else {
            Err("Source not found".into())
        }
    }

    pub async fn stop_sync(&self, source_id: Uuid) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Stopping sync for source {}", source_id);
        
        // Get the cancellation token for this sync
        let cancellation_token = {
            let running_syncs = self.running_syncs.read().await;
            running_syncs.get(&source_id).cloned()
        };
        
        if let Some(token) = cancellation_token {
            // Cancel the sync operation
            token.cancel();
            info!("Cancellation signal sent for source {}", source_id);
            
            // Update source status to indicate cancellation
            if let Err(e) = sqlx::query(
                r#"UPDATE sources SET status = 'idle', last_error = 'Sync cancelled by user', last_error_at = NOW(), updated_at = NOW() WHERE id = $1"#
            )
            .bind(source_id)
            .execute(self.state.db.get_pool())
            .await {
                error!("Failed to update source status after cancellation: {}", e);
            }
            
            // Remove from running syncs list
            {
                let mut running_syncs = self.running_syncs.write().await;
                running_syncs.remove(&source_id);
            }
            
            Ok(())
        } else {
            Err("No running sync found for this source".into())
        }
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
        
        // Check if URL starts with a valid scheme
        if !server_url.starts_with("http://") && !server_url.starts_with("https://") {
            return Err(format!(
                "WebDAV server_url must start with 'http://' or 'https://'. \
                 Current value: '{}'. \
                 Examples of valid URLs: \
                 - https://cloud.example.com \
                 - http://192.168.1.100:8080 \
                 - https://nextcloud.mydomain.com:443", 
                server_url
            ));
        }
        
        // Try to parse as URL to catch other issues
        match reqwest::Url::parse(server_url) {
            Ok(url) => {
                if url.scheme() != "http" && url.scheme() != "https" {
                    return Err(format!(
                        "WebDAV server_url has invalid scheme '{}'. Only 'http' and 'https' are supported. \
                         Current URL: '{}'", 
                        url.scheme(), server_url
                    ));
                }
                
                if url.host_str().is_none() {
                    return Err(format!(
                        "WebDAV server_url is missing hostname. \
                         Current URL: '{}'. \
                         Example: https://cloud.example.com", 
                        server_url
                    ));
                }
                
                info!("✅ WebDAV URL validation passed for source '{}': {}", source_name, server_url);
                Ok(())
            }
            Err(e) => {
                Err(format!(
                    "WebDAV server_url is not a valid URL: {}. \
                     Current value: '{}'. \
                     The URL must be absolute and include the full domain. \
                     Examples: \
                     - https://cloud.example.com \
                     - http://192.168.1.100:8080/webdav \
                     - https://nextcloud.mydomain.com", 
                    e, server_url
                ))
            }
        }
    }
}