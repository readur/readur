use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tracing::{error, info, warn};

use crate::{
    db::Database,
    ocr_queue::OcrQueueService,
    file_service::FileService,
    AppState,
};
use crate::webdav_service::{WebDAVConfig, WebDAVService};
use crate::routes::webdav::webdav_sync::perform_webdav_sync_with_tracking;

pub struct WebDAVScheduler {
    db: Database,
    state: Arc<AppState>,
    check_interval: Duration,
}

impl WebDAVScheduler {
    pub fn new(state: Arc<AppState>) -> Self {
        Self {
            db: state.db.clone(),
            state,
            check_interval: Duration::from_secs(60), // Check every minute for due syncs
        }
    }

    pub async fn start(&self) {
        info!("Starting WebDAV background sync scheduler");
        
        let mut interval_timer = interval(self.check_interval);
        
        loop {
            interval_timer.tick().await;
            
            if let Err(e) = self.check_and_sync_users().await {
                error!("Error in WebDAV sync scheduler: {}", e);
            }
        }
    }

    async fn check_and_sync_users(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Get all users with WebDAV auto-sync enabled
        let users_with_settings = self.db.get_all_user_settings().await?;
        
        for user_settings in users_with_settings {
            // Skip if WebDAV auto-sync is not enabled
            if !user_settings.webdav_auto_sync || !user_settings.webdav_enabled {
                continue;
            }

            // Check if sync is due for this user
            if self.is_sync_due(&user_settings).await? {
                info!("Starting background WebDAV sync for user {}", user_settings.user_id);
                
                // Get WebDAV configuration
                let webdav_config = self.build_webdav_config(&user_settings)?;
                
                // Create WebDAV service
                match WebDAVService::new(webdav_config.clone()) {
                    Ok(webdav_service) => {
                        // Start sync in background task for this user
                        let state_clone = self.state.clone();
                        let user_id = user_settings.user_id;
                        let enable_background_ocr = user_settings.enable_background_ocr;
                        
                        tokio::spawn(async move {
                            match perform_webdav_sync_with_tracking(state_clone.clone(), user_id, webdav_service, webdav_config, enable_background_ocr).await {
                                Ok(files_processed) => {
                                    info!("Background WebDAV sync completed for user {}: {} files processed", user_id, files_processed);
                                    
                                    // Send success notification if files were processed
                                    if files_processed > 0 {
                                        let notification = crate::models::CreateNotification {
                                            notification_type: "success".to_string(),
                                            title: "WebDAV Sync Completed".to_string(),
                                            message: format!("Successfully processed {} files from WebDAV sync", files_processed),
                                            action_url: Some("/documents".to_string()),
                                            metadata: Some(serde_json::json!({
                                                "sync_type": "webdav",
                                                "files_processed": files_processed
                                            })),
                                        };
                                        
                                        if let Err(e) = state_clone.db.create_notification(user_id, &notification).await {
                                            error!("Failed to create success notification: {}", e);
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("Background WebDAV sync failed for user {}: {}", user_id, e);
                                    
                                    // Send error notification
                                    let notification = crate::models::CreateNotification {
                                        notification_type: "error".to_string(),
                                        title: "WebDAV Sync Failed".to_string(),
                                        message: format!("WebDAV sync encountered an error: {}", e),
                                        action_url: Some("/settings".to_string()),
                                        metadata: Some(serde_json::json!({
                                            "sync_type": "webdav",
                                            "error": e.to_string()
                                        })),
                                    };
                                    
                                    if let Err(e) = state_clone.db.create_notification(user_id, &notification).await {
                                        error!("Failed to create error notification: {}", e);
                                    }
                                }
                            }
                        });
                    }
                    Err(e) => {
                        error!("Failed to create WebDAV service for user {}: {}", user_settings.user_id, e);
                    }
                }
            }
        }

        Ok(())
    }

    async fn is_sync_due(&self, user_settings: &crate::models::Settings) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        // Get the sync interval in minutes
        let sync_interval_minutes = user_settings.webdav_sync_interval_minutes;
        
        if sync_interval_minutes <= 0 {
            warn!("Invalid sync interval for user {}: {} minutes", user_settings.user_id, sync_interval_minutes);
            return Ok(false);
        }

        // Check if a sync is already running
        if let Ok(Some(sync_state)) = self.db.get_webdav_sync_state(user_settings.user_id).await {
            if sync_state.is_running {
                info!("Sync already running for user {}", user_settings.user_id);
                return Ok(false);
            }

            // Check last sync time
            if let Some(last_sync) = sync_state.last_sync_at {
                let elapsed = chrono::Utc::now() - last_sync;
                let elapsed_minutes = elapsed.num_minutes();
                
                if elapsed_minutes < sync_interval_minutes as i64 {
                    // Only log this occasionally to avoid spam
                    if elapsed_minutes % 10 == 0 {
                        info!("Sync not due for user {} (last sync {} minutes ago, interval {} minutes)", 
                            user_settings.user_id, elapsed_minutes, sync_interval_minutes);
                    }
                    return Ok(false);
                }
                
                info!("Sync is due for user {} (last sync {} minutes ago, interval {} minutes)", 
                    user_settings.user_id, elapsed_minutes, sync_interval_minutes);
            } else {
                info!("No previous sync found for user {}, sync is due", user_settings.user_id);
            }
        } else {
            info!("No sync state found for user {}, sync is due", user_settings.user_id);
        }

        // Sync is due
        Ok(true)
    }

    fn build_webdav_config(&self, settings: &crate::models::Settings) -> Result<WebDAVConfig, Box<dyn std::error::Error + Send + Sync>> {
        let server_url = settings.webdav_server_url.as_ref()
            .ok_or("WebDAV server URL not configured")?
            .clone();
        let username = settings.webdav_username.as_ref()
            .ok_or("WebDAV username not configured")?
            .clone();
        let password = settings.webdav_password.as_ref()
            .unwrap_or(&String::new())
            .clone();

        if server_url.is_empty() || username.is_empty() {
            return Err("WebDAV configuration incomplete".into());
        }

        Ok(WebDAVConfig {
            server_url,
            username,
            password,
            watch_folders: settings.webdav_watch_folders.clone(),
            file_extensions: settings.webdav_file_extensions.clone(),
            timeout_seconds: 30,
            server_type: Some("nextcloud".to_string()),
        })
    }
}

