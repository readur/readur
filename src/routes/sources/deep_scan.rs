//! A manual full rescan of a WebDAV source, run under the same claim as
//! every other sync of the source.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    db::sources::{SyncCompletion, SYNC_CANCELLED_MESSAGE},
    models::SourceStatus,
    scheduling::source_scheduler::SyncStartError,
    services::webdav::{SyncPhase, SyncProgress},
    utils::outbound::validate_outbound_url,
    AppState,
};

/// Trigger a deep scan for a source
#[utoipa::path(
    post,
    path = "/api/sources/{id}/scan/deep",
    tag = "sources",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = Uuid, Path, description = "Source ID")
    ),
    responses(
        (status = 200, description = "Deep scan started successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Source not found"),
        (status = 409, description = "Source is already syncing"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn trigger_deep_scan(
    auth_user: AuthUser,
    Path(source_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!(
        "Starting deep scan for source {} by user {}",
        source_id, auth_user.user.username
    );

    let source = state
        .db
        .get_source(auth_user.user.id, source_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let Some(scheduler) = state.source_scheduler.as_ref() else {
        error!(
            "Deep scan for source {} not started: no source scheduler is running",
            source_id
        );
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    if source.source_type != crate::models::SourceType::WebDAV {
        error!(
            "Deep scan not supported for source type: {:?}",
            source.source_type
        );
        return Ok(Json(serde_json::json!({
            "success": false,
            "error": "unsupported_source_type",
            "message": "Deep scan is currently only supported for WebDAV sources"
        })));
    }

    let config: crate::models::WebDAVSourceConfig =
        serde_json::from_value(source.config).map_err(|e| {
            error!(
                "Failed to parse WebDAV config for source {}: {}",
                source_id, e
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Re-check the destination at connect time; DNS may have changed
    // since the source was saved.
    let server_url =
        crate::services::webdav::WebDAVConfig::normalize_server_url(&config.server_url);
    if let Err(e) = validate_outbound_url(&server_url).await {
        error!("Deep scan for source {} not started: {}", source_id, e);
        if let Err(db_err) = state
            .db
            .update_source_status(source_id, SourceStatus::Error, Some(e.to_string()))
            .await
        {
            error!("Failed to update source status: {}", db_err);
        }
        return Err(StatusCode::BAD_REQUEST);
    }

    let webdav_config = crate::services::webdav::WebDAVConfig {
        server_url: config.server_url.clone(),
        username: config.username.clone(),
        password: config.password.clone(),
        watch_folders: config.watch_folders.clone(),
        file_extensions: config.file_extensions.clone(),
        timeout_seconds: 600, // 10 minutes for deep scan
        server_type: config.server_type.clone(),
    };

    let webdav_service =
        crate::services::webdav::WebDAVService::new(webdav_config).map_err(|e| {
            error!("Failed to create WebDAV service for deep scan: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // The service holds the watch folders decoded and normalised.
    let watch_folders = webdav_service.get_config().watch_folders.clone();
    let scan = DeepScan {
        state: state.clone(),
        user_id: auth_user.user.id,
        source_id,
        source_name: source.name.clone(),
        webdav_service,
        watch_folders,
        extensions: crate::utils::extension_filter::ExtensionFilter::new(&config.file_extensions),
    };

    // The same claim as every other sync: refused while any sync task for
    // the source is alive, released when the scan task ends.
    match scheduler
        .run_claimed(source_id, move |token, progress| scan.run(token, progress))
        .await
    {
        Ok(()) => Ok(Json(serde_json::json!({
            "success": true,
            "message": format!("Deep scan started for source '{}'. This will perform a complete rescan of all configured folders.", source.name)
        }))),
        Err(
            SyncStartError::AlreadyRunning | SyncStartError::Stopping | SyncStartError::NotClaimed,
        ) => Ok(Json(serde_json::json!({
            "success": false,
            "error": "source_already_syncing",
            "message": "Source is already syncing. Please wait for the current sync to complete before starting a deep scan."
        }))),
        Err(e) => {
            error!(
                "Deep scan for source {} could not be started: {}",
                source_id, e
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// A manual WebDAV deep scan: every watch folder is rescanned in full, the
/// files found are ingested, and the run is recorded like any other sync.
struct DeepScan {
    state: Arc<AppState>,
    user_id: Uuid,
    source_id: Uuid,
    source_name: String,
    webdav_service: crate::services::webdav::WebDAVService,
    watch_folders: Vec<String>,
    extensions: crate::utils::extension_filter::ExtensionFilter,
}

impl DeepScan {
    /// Runs the scan to completion, or until `token` is cancelled; a
    /// cancelled run is recorded like a stopped sync, with the files it had
    /// already ingested.
    async fn run(self, token: tokio_util::sync::CancellationToken, progress: Arc<SyncProgress>) {
        let start_time = chrono::Utc::now();
        progress.set_phase(SyncPhase::Initializing);
        info!(
            "🚀 Starting manual deep scan for source '{}'",
            self.source_name
        );
        let scan = DeepScan {
            webdav_service: self.webdav_service.with_cancellation_token(token.clone()),
            ..self
        };
        let this = &scan;

        let (files, directories_tracked, pending) = this.discover(&token, &progress).await;
        if token.is_cancelled() {
            this.finish_cancelled(0).await;
            return;
        }

        if files.is_empty() {
            crate::services::webdav::PendingDirectoryCommit::commit_all(
                &this.state.db,
                pending,
                &[],
            )
            .await;
            info!(
                "Deep scan found no files but tracked {} directories for source {}",
                directories_tracked, this.source_id
            );
            progress.set_phase(SyncPhase::Completed);
            let message = format!(
                "Smart deep scan completed: {} directories tracked, no files found",
                directories_tracked
            );
            this.finish(SourceStatus::Idle, &message, 0, Some(0)).await;
            return;
        }

        info!(
            "Deep scan will process {} files from {} directories for source {}",
            files.len(),
            directories_tracked,
            this.source_id
        );
        match crate::services::webdav_file_processing::process_files_for_deep_scan(
            this.state.clone(),
            this.user_id,
            &this.webdav_service,
            &files,
            true, // enable background OCR
            Some(this.source_id),
            &token,
        )
        .await
        {
            Ok(processed) if token.is_cancelled() => {
                this.finish_cancelled(processed.ingested as i64).await;
            }
            Ok(processed) => {
                // The directory state is recorded only now that the files are
                // done, minus the directories of files that failed.
                crate::services::webdav::PendingDirectoryCommit::commit_all(
                    &this.state.db,
                    pending,
                    &processed.failed_files,
                )
                .await;
                progress.set_phase(SyncPhase::Completed);
                this.finish_processed(
                    &processed,
                    files.len(),
                    directories_tracked,
                    chrono::Utc::now() - start_time,
                    &progress,
                )
                .await;
            }
            Err(e) => {
                error!(
                    "Deep scan file processing failed for source {}: {}",
                    this.source_id, e
                );
                progress.set_phase(SyncPhase::Failed(e.to_string()));
                progress.add_error(&format!("File processing failed: {}", e));
                this.finish(
                    SourceStatus::Error,
                    &format!("Deep scan failed: {}", e),
                    0,
                    Some(0),
                )
                .await;
                this.notify(
                    "error",
                    "Deep Scan Failed",
                    format!("Deep scan of {} failed: {}", this.source_name, e),
                    serde_json::json!({
                        "source_id": this.source_id,
                        "scan_type": "deep_scan",
                        "error": e.to_string()
                    }),
                )
                .await;
            }
        }
    }

    /// Rescans every watch folder; a folder that fails is logged and skipped.
    /// Stops between folders once `token` is cancelled. The directory rows
    /// each folder wants recorded are returned for committing after the files
    /// have been processed.
    async fn discover(
        &self,
        token: &tokio_util::sync::CancellationToken,
        progress: &Arc<SyncProgress>,
    ) -> (
        Vec<crate::models::FileIngestionInfo>,
        usize,
        Vec<crate::services::webdav::PendingDirectoryCommit>,
    ) {
        let smart_sync_service = crate::services::webdav::SmartSyncService::new(self.state.clone())
            .with_cancellation_token(token.clone());
        let mut files = Vec::new();
        let mut directories_tracked = 0;
        let mut pending = Vec::new();
        for watch_folder in &self.watch_folders {
            if token.is_cancelled() {
                break;
            }
            info!("🔍 Deep scan processing watch folder: {}", watch_folder);
            progress.set_current_directory(watch_folder);
            match smart_sync_service
                .perform_smart_sync(
                    self.user_id,
                    Some(self.source_id),
                    &self.webdav_service,
                    watch_folder,
                    crate::services::webdav::SmartSyncStrategy::FullDeepScan, // Force deep scan for directory reset
                    Some(progress),
                )
                .await
            {
                Ok(result) => {
                    info!(
                        "Deep scan found {} files and {} directories in {}",
                        result.files.len(),
                        result.directories.len(),
                        watch_folder
                    );
                    files.extend(
                        result
                            .files
                            .into_iter()
                            .filter(|f| self.extensions.matches_name(&f.name)),
                    );
                    directories_tracked += result.directories.len();
                    pending.push(result.directory_commit);
                }
                // A scan interrupted by the stop request is not a folder failure.
                Err(_) if token.is_cancelled() => break,
                Err(e) => {
                    let error_msg =
                        format!("Deep scan failed for watch folder {}: {}", watch_folder, e);
                    error!("{}", error_msg);
                    progress.add_error(&error_msg);
                }
            }
        }
        (files, directories_tracked, pending)
    }

    /// Records a finished scan the way a scheduled sync is recorded: idle
    /// with a success notification when every file went through, otherwise
    /// the error status with a summary and the failed files counted as
    /// pending.
    async fn finish_processed(
        &self,
        processed: &crate::services::webdav_file_processing::ProcessedFiles,
        files_total: usize,
        directories_tracked: usize,
        duration: chrono::Duration,
        progress: &Arc<SyncProgress>,
    ) {
        let files_processed = processed.ingested;
        let metadata = serde_json::json!({
            "source_id": self.source_id,
            "scan_type": "deep_scan",
            "files_processed": files_processed,
            "files_failed": processed.failed_files.len(),
            "duration_seconds": duration.num_seconds()
        });
        if let Some(first_failed) = processed.failed_files.first() {
            let summary = format!(
                "{} of {} files failed; first: {}",
                processed.failed_files.len(),
                files_total,
                first_failed
            );
            warn!(
                "Deep scan finished with failures for source {}: {}",
                self.source_id, summary
            );
            progress.add_error(&summary);
            self.finish(
                SourceStatus::Error,
                &summary,
                files_processed as i64,
                Some(processed.failed_files.len() as i64),
            )
            .await;
            self.notify(
                "warning",
                "Deep Scan Finished With Errors",
                format!("Deep scan of {}: {}", self.source_name, summary),
                metadata,
            )
            .await;
            return;
        }
        info!(
            "Deep scan completed for source {}: {} files processed in {:?}",
            self.source_id, files_processed, duration
        );
        self.finish(
            SourceStatus::Idle,
            &format!("Deep scan completed: {} files processed", files_processed),
            files_processed as i64,
            Some(0),
        )
        .await;
        self.notify("success", "Deep Scan Completed", format!(
            "Smart deep scan of {} completed successfully. {} files processed, {} directories tracked in {:.1} minutes.",
            self.source_name, files_processed, directories_tracked, duration.num_seconds() as f64 / 60.0
        ), metadata).await;
    }

    /// Records a stopped scan: idle with the cancelled message, the files
    /// ingested so far added to the total, and the pending count left alone
    /// because the scan did not get to count what remained.
    async fn finish_cancelled(&self, files_synced: i64) {
        info!(
            "Deep scan for source {} was cancelled after {} files",
            self.source_id, files_synced
        );
        self.finish(
            SourceStatus::Idle,
            SYNC_CANCELLED_MESSAGE,
            files_synced,
            None,
        )
        .await;
    }

    async fn finish(
        &self,
        status: SourceStatus,
        message: &str,
        files_synced: i64,
        files_pending: Option<i64>,
    ) {
        let completion = SyncCompletion {
            status,
            error: Some(message),
            files_synced,
            files_pending,
        };
        match self.state.db.finish_sync(self.source_id, &completion).await {
            Ok(true) => {}
            Ok(false) => warn!("Deep scan completion for source {} not recorded: the source was no longer marked as syncing", self.source_id),
            Err(e) => error!("Failed to record deep scan completion for source {}: {}", self.source_id, e),
        }
    }

    async fn notify(&self, kind: &str, title: &str, message: String, metadata: serde_json::Value) {
        let notification = crate::models::CreateNotification {
            notification_type: kind.to_string(),
            title: title.to_string(),
            message,
            action_url: Some(
                if kind == "success" {
                    "/documents"
                } else {
                    "/sources"
                }
                .to_string(),
            ),
            metadata: Some(metadata),
        };
        if let Err(e) = self
            .state
            .db
            .create_notification(self.user_id, &notification)
            .await
        {
            error!("Failed to create deep scan notification: {}", e);
        }
    }
}
