use chrono::{DateTime, Utc};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::services::webdav::{WebDAVConfig, WebDAVService};
use crate::services::webdav_sync::{start_webdav_sync, WebDavSyncRequest, WebDavSyncResult};
use crate::{
    db::{webdav::WEBDAV_SYNC_INTERRUPTED_MESSAGE, Database},
    models::{
        source::{UpdateWebDAVSyncState, WebDAVSyncState},
        CreateNotification, Settings,
    },
    AppState,
};

/// Why the scheduler starts a user's sync; decides the notification sent
/// when it ends.
#[derive(Debug, Clone, Copy)]
enum ScheduledSync {
    /// The sync was due by the user's interval.
    Due,
    /// The sync was interrupted by a server restart and is resumed.
    Resumed,
}

impl ScheduledSync {
    /// The notification for a finished run, if any: a resumed run always
    /// reports success, a due run only when it found files; failures of a due
    /// run are reported, of a resumed run only logged.
    fn notification(self, result: &WebDavSyncResult) -> Option<CreateNotification> {
        match (self, result) {
            (ScheduledSync::Resumed, Ok(files)) => Some(CreateNotification {
                notification_type: "success".to_string(),
                title: "WebDAV Sync Resumed".to_string(),
                message: format!(
                    "Resumed sync after server restart. Processed {} files",
                    files
                ),
                action_url: Some("/documents".to_string()),
                metadata: Some(
                    serde_json::json!({"sync_type": "webdav_resume", "files_processed": files}),
                ),
            }),
            (ScheduledSync::Due, Ok(files)) if *files > 0 => Some(CreateNotification {
                notification_type: "success".to_string(),
                title: "WebDAV Sync Completed".to_string(),
                message: format!("Successfully processed {} files from WebDAV sync", files),
                action_url: Some("/documents".to_string()),
                metadata: Some(
                    serde_json::json!({"sync_type": "webdav", "files_processed": files}),
                ),
            }),
            (ScheduledSync::Due, Ok(_)) | (ScheduledSync::Resumed, Err(_)) => None,
            (ScheduledSync::Due, Err(e)) => Some(CreateNotification {
                notification_type: "error".to_string(),
                title: "WebDAV Sync Failed".to_string(),
                message: format!("WebDAV sync encountered an error: {}", e),
                action_url: Some("/settings".to_string()),
                metadata: Some(serde_json::json!({"sync_type": "webdav", "error": e.to_string()})),
            }),
        }
    }
}

/// Logs how a scheduled run ended and sends its notification, if any.
async fn report_sync_end(
    state: Arc<AppState>,
    user_id: Uuid,
    kind: ScheduledSync,
    result: WebDavSyncResult,
) {
    info!(
        "{:?} WebDAV sync of user {} ended: {:?}",
        kind, user_id, result
    );
    if let Some(notification) = kind.notification(&result) {
        if let Err(e) = state.db.create_notification(user_id, &notification).await {
            error!("Failed to create WebDAV sync notification: {}", e);
        }
    }
}

/// Whether a user's scheduled sync should start now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SyncDue {
    Due,
    /// The last sync is more recent than the interval.
    NotYet,
    /// A sync is marked running.
    Running,
    /// The configured interval is not positive; never due.
    InvalidInterval,
}

/// Decides whether a user's sync is due from their interval and sync state:
/// due when there is no state or no previous sync, or when the last one
/// started at least `interval_minutes` ago.
fn sync_due(
    interval_minutes: i32,
    sync_state: Option<&WebDAVSyncState>,
    now: DateTime<Utc>,
) -> SyncDue {
    if interval_minutes <= 0 {
        return SyncDue::InvalidInterval;
    }
    let Some(sync_state) = sync_state else {
        return SyncDue::Due;
    };
    if sync_state.is_running {
        return SyncDue::Running;
    }
    match sync_state.last_sync_at {
        Some(last_sync) if (now - last_sync).num_minutes() < i64::from(interval_minutes) => {
            SyncDue::NotYet
        }
        _ => SyncDue::Due,
    }
}

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
        // First, check for any interrupted syncs that need to be resumed
        if let Err(e) = self.resume_interrupted_syncs().await {
            error!("Error resuming interrupted syncs: {}", e);
        }

        let mut interval_timer = interval(self.check_interval);
        loop {
            interval_timer.tick().await;
            if let Err(e) = self.check_and_sync_users().await {
                error!("Error in WebDAV sync scheduler: {}", e);
            }
        }
    }

    /// Resumes the auto-synced users whose last sync was marked as
    /// interrupted by a server restart, clearing that marker first.
    pub async fn resume_interrupted_syncs(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Checking for interrupted WebDAV syncs to resume");
        for user_settings in self.db.get_all_user_settings().await? {
            if !user_settings.webdav_enabled || !user_settings.webdav_auto_sync {
                continue;
            }
            let Ok(Some(sync_state)) = self.db.get_webdav_sync_state(user_settings.user_id).await
            else {
                continue;
            };
            // The marker the startup reset writes, compared exactly.
            if sync_state
                .errors
                .iter()
                .any(|e| e == WEBDAV_SYNC_INTERRUPTED_MESSAGE)
            {
                self.resume(&user_settings, sync_state).await;
            }
        }
        Ok(())
    }

    async fn resume(&self, user_settings: &Settings, sync_state: WebDAVSyncState) {
        info!(
            "Resuming the interrupted WebDAV sync of user {}",
            user_settings.user_id
        );
        let reset_state = UpdateWebDAVSyncState {
            last_sync_at: sync_state.last_sync_at,
            sync_cursor: sync_state.sync_cursor,
            is_running: false,
            files_processed: sync_state.files_processed,
            files_remaining: 0,
            current_folder: None,
            errors: sync_state
                .errors
                .into_iter()
                .filter(|e| e != WEBDAV_SYNC_INTERRUPTED_MESSAGE)
                .collect(),
        };
        match self
            .db
            .update_webdav_sync_state(user_settings.user_id, &reset_state)
            .await
        {
            Ok(()) => self.launch(user_settings, ScheduledSync::Resumed).await,
            Err(e) => error!("Failed to reset interrupted sync state: {}", e),
        }
    }

    /// One scheduler tick: starts the sync of every auto-synced user that is due.
    pub async fn check_and_sync_users(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for user_settings in self.db.get_all_user_settings().await? {
            if user_settings.webdav_auto_sync
                && user_settings.webdav_enabled
                && self.is_sync_due(&user_settings).await?
            {
                self.launch(&user_settings, ScheduledSync::Due).await;
            }
        }
        Ok(())
    }

    /// Starts the user's sync through the same claim as a manual start, so a
    /// sync that is already running (manual or scheduled) is never doubled.
    async fn launch(&self, user_settings: &Settings, kind: ScheduledSync) {
        let user_id = user_settings.user_id;
        let request = match Self::sync_request(user_settings) {
            Ok(request) => request,
            Err(e) => {
                error!("WebDAV sync for user {} not started: {}", user_id, e);
                return;
            }
        };
        let state = self.state.clone();
        let on_finished = move |result| report_sync_end(state, user_id, kind, result);
        match start_webdav_sync(self.state.clone(), request, on_finished).await {
            Ok(started) => info!("{:?} WebDAV sync of user {}: {:?}", kind, user_id, started),
            Err(e) => error!(
                "WebDAV sync of user {} could not be started: {}",
                user_id, e
            ),
        }
    }

    fn sync_request(
        user_settings: &Settings,
    ) -> Result<WebDavSyncRequest, Box<dyn std::error::Error + Send + Sync>> {
        let config = Self::build_webdav_config(user_settings)?;
        let webdav_service = WebDAVService::new(config.clone())?;
        Ok(WebDavSyncRequest {
            user_id: user_settings.user_id,
            webdav_service,
            config,
            enable_background_ocr: user_settings.enable_background_ocr,
        })
    }

    async fn is_sync_due(
        &self,
        user_settings: &Settings,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let user_id = user_settings.user_id;
        let interval_minutes = user_settings.webdav_sync_interval_minutes;
        // Only a pre-filter: the claim in `launch` is what prevents a double run.
        let sync_state = self.db.get_webdav_sync_state(user_id).await.ok().flatten();
        let due = sync_due(interval_minutes, sync_state.as_ref(), Utc::now());
        match due {
            SyncDue::InvalidInterval => warn!(
                "Invalid sync interval for user {}: {} minutes",
                user_id, interval_minutes
            ),
            SyncDue::Due | SyncDue::Running => info!("WebDAV sync of user {}: {:?}", user_id, due),
            SyncDue::NotYet => {}
        }
        Ok(due == SyncDue::Due)
    }

    fn build_webdav_config(
        settings: &Settings,
    ) -> Result<WebDAVConfig, Box<dyn std::error::Error + Send + Sync>> {
        let server_url = settings
            .webdav_server_url
            .clone()
            .ok_or("WebDAV server URL not configured")?;
        let username = settings
            .webdav_username
            .clone()
            .ok_or("WebDAV username not configured")?;
        let password = settings.webdav_password.clone().unwrap_or_default();

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

    pub async fn trigger_sync(&self, source_id: uuid::Uuid) {
        info!("Triggering manual sync for source {}", source_id);
        // TODO: Implement manual sync trigger for sources
        // For now, this is a placeholder that the routes can call
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(
        is_running: bool,
        last_sync_minutes_ago: Option<i64>,
        now: DateTime<Utc>,
    ) -> WebDAVSyncState {
        WebDAVSyncState {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            last_sync_at: last_sync_minutes_ago.map(|m| now - chrono::Duration::minutes(m)),
            sync_cursor: None,
            is_running,
            files_processed: 0,
            files_remaining: 0,
            current_folder: None,
            errors: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn sync_is_due_by_interval_and_never_while_running() {
        let now = Utc::now();
        assert_eq!(sync_due(60, None, now), SyncDue::Due);
        assert_eq!(
            sync_due(60, Some(&state(false, None, now)), now),
            SyncDue::Due
        );
        assert_eq!(
            sync_due(60, Some(&state(false, Some(59), now)), now),
            SyncDue::NotYet
        );
        assert_eq!(
            sync_due(60, Some(&state(false, Some(60), now)), now),
            SyncDue::Due
        );
        assert_eq!(
            sync_due(60, Some(&state(true, Some(600), now)), now),
            SyncDue::Running
        );
        assert_eq!(sync_due(0, None, now), SyncDue::InvalidInterval);
        assert_eq!(
            sync_due(-5, Some(&state(false, Some(600), now)), now),
            SyncDue::InvalidInterval
        );
    }

    #[test]
    fn scheduled_sync_notifications() {
        let resumed_ok = ScheduledSync::Resumed.notification(&Ok(0)).unwrap();
        assert_eq!(resumed_ok.title, "WebDAV Sync Resumed");
        assert!(ScheduledSync::Resumed
            .notification(&Err("boom".into()))
            .is_none());
        assert!(
            ScheduledSync::Due.notification(&Ok(0)).is_none(),
            "an empty due run is silent"
        );
        let due_ok = ScheduledSync::Due.notification(&Ok(3)).unwrap();
        assert_eq!(due_ok.notification_type, "success");
        assert!(due_ok.message.contains('3'));
        let due_err = ScheduledSync::Due
            .notification(&Err("boom".into()))
            .unwrap();
        assert_eq!(due_err.notification_type, "error");
        assert!(due_err.message.contains("boom"));
    }
}
