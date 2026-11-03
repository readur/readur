use axum::{
    extract::{Path, State, WebSocketUpgrade},
    extract::ws::{WebSocket, Message},
    http::{StatusCode, HeaderMap},
    response::{Json, Response},
};
use std::sync::Arc;
use uuid::Uuid;
use tracing::{error, info, warn};
use std::time::Duration;

use crate::{
    auth::AuthUser,
    db::sources::{SyncCompletion, SYNC_CANCELLED_MESSAGE},
    models::SourceStatus,
    scheduling::source_scheduler::SyncStartError,
    services::webdav::{SyncProgress, SyncPhase},
    utils::{client_ip::PeerIsTrustedProxy, outbound::validate_outbound_url},
    AppState,
};

// Removed WebSocketAuthQuery - using secure header-based authentication instead

/// Trigger a sync for a source
#[utoipa::path(
    post,
    path = "/api/sources/{id}/sync",
    tag = "sources",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = Uuid, Path, description = "Source ID")
    ),
    responses(
        (status = 200, description = "Sync triggered successfully"),
        (status = 400, description = "Source configuration is invalid"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Source not found"),
        (status = 409, description = "Source is already syncing"),
        (status = 500, description = "Internal server error"),
        (status = 501, description = "Not implemented - Source type not supported")
    )
)]
pub async fn trigger_sync(
    auth_user: AuthUser,
    Path(source_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, StatusCode> {
    let source = state
        .db
        .get_source(auth_user.user.id, source_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Trigger sync using the universal source scheduler
    // The scheduler will handle all status checks and atomic operations
    if let Some(scheduler) = &state.source_scheduler {
        match scheduler.trigger_sync(source_id).await {
            Ok(()) => {
                // Sync started successfully
            }
            Err(e) => {
                error!("Failed to trigger sync for source {}: {}", source_id, e);
                return Err(match e {
                    SyncStartError::AlreadyRunning | SyncStartError::Stopping | SyncStartError::NotClaimed => StatusCode::CONFLICT,
                    SyncStartError::NotFound => StatusCode::NOT_FOUND,
                    SyncStartError::Configuration(_) => StatusCode::BAD_REQUEST,
                    SyncStartError::Db(_) => StatusCode::INTERNAL_SERVER_ERROR,
                });
            }
        }
    } else {
        // Fallback to WebDAV scheduler for backward compatibility
        match source.source_type {
            crate::models::SourceType::WebDAV => {
                if let Some(webdav_scheduler) = &state.webdav_scheduler {
                    webdav_scheduler.trigger_sync(source_id).await;
                } else {
                    state
                        .db
                        .update_source_status(
                            source_id,
                            SourceStatus::Error,
                            Some("No scheduler available".to_string()),
                        )
                        .await
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                }
            }
            _ => {
                state
                    .db
                    .update_source_status(
                        source_id,
                        SourceStatus::Error,
                        Some("Source type not supported".to_string()),
                    )
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                return Err(StatusCode::NOT_IMPLEMENTED);
            }
        }
    }

    Ok(StatusCode::OK)
}

/// Stop sync for a source
#[utoipa::path(
    post,
    path = "/api/sources/{id}/sync/stop",
    tag = "sources",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = Uuid, Path, description = "Source ID")
    ),
    responses(
        (status = 200, description = "Sync stopped successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Source not found"),
        (status = 409, description = "Source is not currently syncing"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn stop_sync(
    auth_user: AuthUser,
    Path(source_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, StatusCode> {
    let source = state
        .db
        .get_source(auth_user.user.id, source_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Allow stopping sync regardless of current status to handle edge cases
    // where the database status might be out of sync with actual running tasks

    // Stop sync using the universal source scheduler
    if let Some(scheduler) = &state.source_scheduler {
        if let Err(e) = scheduler.stop_sync(source_id).await {
            let error_msg = e.to_string();
            // If no sync is running, treat it as success since the desired state is achieved
            if error_msg.contains("No running sync found") {
                info!("No sync was running for source {}, ensuring status is idle", source_id);
                // Use atomic operation to ensure status is idle if not already syncing
                let _ = state
                    .db
                    .update_source_status_atomic(
                        source_id,
                        None, // Don't check current status
                        SourceStatus::Idle,
                        Some("No sync was running")
                    )
                    .await;
            } else {
                error!("Failed to stop sync for source {}: {}", source_id, e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    } else {
        // Update status directly if no scheduler available (fallback)
        state
            .db
            .update_source_status(
                source_id,
                SourceStatus::Idle,
                Some("Sync cancelled by user".to_string()),
            )
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(StatusCode::OK)
}

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
    info!("Starting deep scan for source {} by user {}", source_id, auth_user.user.username);

    let source = state
        .db
        .get_source(auth_user.user.id, source_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let Some(scheduler) = state.source_scheduler.as_ref() else {
        error!("Deep scan for source {} not started: no source scheduler is running", source_id);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    if source.source_type != crate::models::SourceType::WebDAV {
        error!("Deep scan not supported for source type: {:?}", source.source_type);
        return Ok(Json(serde_json::json!({
            "success": false,
            "error": "unsupported_source_type",
            "message": "Deep scan is currently only supported for WebDAV sources"
        })));
    }

    let config: crate::models::WebDAVSourceConfig = serde_json::from_value(source.config)
        .map_err(|e| {
            error!("Failed to parse WebDAV config for source {}: {}", source_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Re-check the destination at connect time; DNS may have changed
    // since the source was saved.
    let server_url = crate::services::webdav::WebDAVConfig::normalize_server_url(&config.server_url);
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

    let webdav_service = crate::services::webdav::WebDAVService::new(webdav_config)
        .map_err(|e| {
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
    match scheduler.run_claimed(source_id, move |token, progress| scan.run(token, progress)).await {
        Ok(()) => Ok(Json(serde_json::json!({
            "success": true,
            "message": format!("Deep scan started for source '{}'. This will perform a complete rescan of all configured folders.", source.name)
        }))),
        Err(SyncStartError::AlreadyRunning | SyncStartError::Stopping | SyncStartError::NotClaimed) => {
            Ok(Json(serde_json::json!({
                "success": false,
                "error": "source_already_syncing",
                "message": "Source is already syncing. Please wait for the current sync to complete before starting a deep scan."
            })))
        }
        Err(e) => {
            error!("Deep scan for source {} could not be started: {}", source_id, e);
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
        info!("🚀 Starting manual deep scan for source '{}'", self.source_name);
        let scan = DeepScan { webdav_service: self.webdav_service.with_cancellation_token(token.clone()), ..self };
        let this = &scan;

        let (files, directories_tracked, pending) = this.discover(&token, &progress).await;
        if token.is_cancelled() {
            this.finish_cancelled(0).await;
            return;
        }

        if files.is_empty() {
            crate::services::webdav::PendingDirectoryCommit::commit_all(&this.state.db, pending, &[]).await;
            info!("Deep scan found no files but tracked {} directories for source {}", directories_tracked, this.source_id);
            progress.set_phase(SyncPhase::Completed);
            let message = format!("Smart deep scan completed: {} directories tracked, no files found", directories_tracked);
            this.finish(SourceStatus::Idle, &message, 0, Some(0)).await;
            return;
        }

        info!("Deep scan will process {} files from {} directories for source {}", files.len(), directories_tracked, this.source_id);
        match crate::routes::webdav::webdav_sync::process_files_for_deep_scan(
            this.state.clone(),
            this.user_id,
            &this.webdav_service,
            &files,
            true, // enable background OCR
            Some(this.source_id),
            &token,
        ).await {
            Ok(processed) if token.is_cancelled() => {
                this.finish_cancelled(processed.ingested as i64).await;
            }
            Ok(processed) => {
                // The directory state is recorded only now that the files are
                // done, minus the directories of files that failed.
                crate::services::webdav::PendingDirectoryCommit::commit_all(&this.state.db, pending, &processed.failed_files).await;
                let files_processed = processed.ingested;
                let duration = chrono::Utc::now() - start_time;
                info!("Deep scan completed for source {}: {} files processed in {:?}", this.source_id, files_processed, duration);
                progress.set_phase(SyncPhase::Completed);
                this.finish(SourceStatus::Idle, &format!("Deep scan completed: {} files processed", files_processed), files_processed as i64, Some(0)).await;
                this.notify("success", "Deep Scan Completed", format!(
                    "Smart deep scan of {} completed successfully. {} files processed, {} directories tracked in {:.1} minutes.",
                    this.source_name, files_processed, directories_tracked, duration.num_seconds() as f64 / 60.0
                ), serde_json::json!({
                    "source_id": this.source_id,
                    "scan_type": "deep_scan",
                    "files_processed": files_processed,
                    "duration_seconds": duration.num_seconds()
                })).await;
            }
            Err(e) => {
                error!("Deep scan file processing failed for source {}: {}", this.source_id, e);
                progress.set_phase(SyncPhase::Failed(e.to_string()));
                progress.add_error(&format!("File processing failed: {}", e));
                this.finish(SourceStatus::Error, &format!("Deep scan failed: {}", e), 0, Some(0)).await;
                this.notify("error", "Deep Scan Failed", format!("Deep scan of {} failed: {}", this.source_name, e), serde_json::json!({
                    "source_id": this.source_id,
                    "scan_type": "deep_scan",
                    "error": e.to_string()
                })).await;
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
    ) -> (Vec<crate::models::FileIngestionInfo>, usize, Vec<crate::services::webdav::PendingDirectoryCommit>) {
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
            match smart_sync_service.perform_smart_sync(
                self.user_id,
                Some(self.source_id),
                &self.webdav_service,
                watch_folder,
                crate::services::webdav::SmartSyncStrategy::FullDeepScan, // Force deep scan for directory reset
                Some(progress),
            ).await {
                Ok(result) => {
                    info!("Deep scan found {} files and {} directories in {}", result.files.len(), result.directories.len(), watch_folder);
                    files.extend(result.files.into_iter().filter(|f| self.extensions.matches_name(&f.name)));
                    directories_tracked += result.directories.len();
                    pending.push(result.directory_commit);
                }
                // A scan interrupted by the stop request is not a folder failure.
                Err(_) if token.is_cancelled() => break,
                Err(e) => {
                    let error_msg = format!("Deep scan failed for watch folder {}: {}", watch_folder, e);
                    error!("{}", error_msg);
                    progress.add_error(&error_msg);
                }
            }
        }
        (files, directories_tracked, pending)
    }

    /// Records a stopped scan: idle with the cancelled message, the files
    /// ingested so far added to the total, and the pending count left alone
    /// because the scan did not get to count what remained.
    async fn finish_cancelled(&self, files_synced: i64) {
        info!("Deep scan for source {} was cancelled after {} files", self.source_id, files_synced);
        self.finish(SourceStatus::Idle, SYNC_CANCELLED_MESSAGE, files_synced, None).await;
    }

    async fn finish(&self, status: SourceStatus, message: &str, files_synced: i64, files_pending: Option<i64>) {
        let completion = SyncCompletion { status, error: Some(message), files_synced, files_pending };
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
            action_url: Some(if kind == "success" { "/documents" } else { "/sources" }.to_string()),
            metadata: Some(metadata),
        };
        if let Err(e) = self.state.db.create_notification(self.user_id, &notification).await {
            error!("Failed to create deep scan notification: {}", e);
        }
    }
}

/// WebSocket endpoint for real-time sync progress updates
/// 
/// This endpoint provides real-time updates about source synchronization progress via WebSocket.
/// It sends progress messages every second during active sync operations and heartbeat messages
/// when no sync is running. This replaces the previous Server-Sent Events (SSE) implementation
/// with improved security by using query parameter authentication instead of exposing JWT tokens.
/// 
/// # Message Types
/// - `progress`: Real-time sync progress updates with detailed statistics
/// - `heartbeat`: Keep-alive messages when no sync is active
/// - `error`: Error messages for connection or sync issues
/// - `connection_confirmed`: Confirmation that the WebSocket connection is established
/// 
/// # Security
/// Authentication is handled via JWT token in the `Sec-WebSocket-Protocol` header during WebSocket handshake.
/// This secure approach prevents token exposure in logs, browser history, and referrer headers.
#[utoipa::path(
    get,
    path = "/api/sources/{id}/sync/progress/ws",
    tag = "sources",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = Uuid, Path, description = "Source ID to monitor for sync progress")
    ),
    responses(
        (status = 101, description = "WebSocket connection established - will stream real-time progress updates"),
        (status = 401, description = "Unauthorized - invalid or missing authentication token"),
        (status = 404, description = "Source not found or user does not have access"),
        (status = 500, description = "Internal server error during WebSocket upgrade")
    )
)]
pub async fn sync_progress_websocket(
    ws: WebSocketUpgrade,
    Path(source_id): Path<Uuid>,
    headers: HeaderMap,
    PeerIsTrustedProxy(via_trusted_proxy): PeerIsTrustedProxy,
    State(state): State<Arc<AppState>>,
) -> Result<Response, StatusCode> {
    // WebSocket handshakes are not subject to CORS, so the origin is
    // checked explicitly.
    if !websocket_origin_allowed(&headers, via_trusted_proxy, &state.config) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Token travels in Sec-WebSocket-Protocol (`bearer.<token>`) because
    // browsers cannot set an Authorization header on WebSocket requests.
    let (token, auth_protocol) = extract_websocket_token_and_protocol(&headers).ok_or(StatusCode::UNAUTHORIZED)?;

    // Same checks as every other endpoint: API keys, revocation, account state.
    let user = crate::auth::authenticate_token(&token, &state)
        .await
        .map_err(|resp| resp.status())?;

    // Verify the source exists and the user has access
    let _source = state
        .db
        .get_source(user.id, source_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Upgrade the connection to WebSocket with protocol acknowledgment
    Ok(ws
        .protocols([auth_protocol.clone()])
        .on_upgrade(move |socket| handle_websocket(socket, source_id, state)))
}

/// Handle WebSocket connection for sync progress updates
async fn handle_websocket(mut socket: WebSocket, source_id: Uuid, state: Arc<AppState>) {
    info!("WebSocket connection established for source {}", source_id);
    
    // Send connection confirmation
    let confirmation_msg = serde_json::json!({
        "type": "connection_confirmed",
        "data": {
            "source_id": source_id,
            "timestamp": chrono::Utc::now().timestamp()
        }
    });
    
    if let Err(e) = socket.send(Message::Text(confirmation_msg.to_string().into())).await {
        error!("Failed to send connection confirmation for source {}: {}", source_id, e);
        return;
    }
    
    let progress_tracker = state.sync_progress_tracker.clone();
    
    loop {
        // Check for progress update
        let progress_info = progress_tracker.get_progress(source_id);
        
        let message = match progress_info {
            Some(info) => {
                // Send current progress
                match serde_json::to_string(&serde_json::json!({
                    "type": "progress",
                    "data": info
                })) {
                    Ok(json) => Message::Text(json.into()),
                    Err(e) => {
                        error!("Failed to serialize progress info: {}", e);
                        let error_msg = serde_json::json!({
                            "type": "error",
                            "data": {
                                "message": format!("Failed to serialize progress: {}", e),
                                "error_type": "serialization_error"
                            }
                        });
                        Message::Text(error_msg.to_string().into())
                    }
                }
            }
            None => {
                // No active sync, send a heartbeat
                Message::Text(serde_json::json!({
                    "type": "heartbeat",
                    "data": {
                        "source_id": source_id,
                        "is_active": false,
                        "timestamp": chrono::Utc::now().timestamp()
                    }
                }).to_string().into())
            }
        };
        
        // Send the message to the client
        if let Err(e) = socket.send(message).await {
            error!("Failed to send WebSocket message for source {}: {}", source_id, e);
            
            // Try to send error notification to client before breaking
            let error_notification = serde_json::json!({
                "type": "error",
                "data": {
                    "message": "Connection error occurred, closing connection",
                    "error_type": "connection_error",
                    "details": e.to_string()
                }
            });
            
            // Attempt to send error message (ignore if this fails too)
            let _ = socket.send(Message::Text(error_notification.to_string().into())).await;
            break;
        }
        
        // Wait before next update
        tokio::time::sleep(Duration::from_secs(1)).await;
        
        // Check if the connection is still alive by trying to send a ping
        if let Err(e) = socket.send(Message::Ping(vec![].into())).await {
            info!("WebSocket connection closed for source {} (ping failed: {})", source_id, e);
            
            // Try to send graceful closure message
            let closure_msg = serde_json::json!({
                "type": "error",
                "data": {
                    "message": "Connection lost during ping check",
                    "error_type": "ping_failed",
                    "details": e.to_string()
                }
            });
            
            // Attempt to send closure message (ignore if this fails)
            let _ = socket.send(Message::Text(closure_msg.to_string().into())).await;
            break;
        }
    }
    
    // Send final close message if connection is still open
    let close_msg = serde_json::json!({
        "type": "connection_closing",
        "data": {
            "source_id": source_id,
            "message": "Server is closing connection",
            "timestamp": chrono::Utc::now().timestamp()
        }
    });
    
    // Try to send close notification (ignore failures)
    let _ = socket.send(Message::Text(close_msg.to_string().into())).await;
    
    info!("WebSocket connection terminated for source {}", source_id);
}

/// Decide whether a WebSocket handshake's `Origin` is acceptable.
///
/// Non-browser clients usually send no `Origin`; those are allowed because
/// they must still present a valid token. When an `Origin` is present it must
/// match the request's own `Host`, the configured `PUBLIC_URL`, or one of
/// `CORS_ALLOWED_ORIGINS` -- or, for requests relayed by a trusted proxy, the
/// host the proxy reports in `X-Forwarded-Host` (with `X-Forwarded-Proto`).
fn websocket_origin_allowed(headers: &HeaderMap, via_trusted_proxy: bool, config: &crate::config::Config) -> bool {
    let Some(origin) = headers.get(axum::http::header::ORIGIN) else {
        return true;
    };
    let Ok(origin) = origin.to_str() else {
        return false;
    };
    let origin = origin.trim_end_matches('/');

    // Compare the origin's host[:port] with the Host header.
    let origin_authority = origin.split_once("://").map(|(_, rest)| rest);
    let host = headers
        .get(axum::http::header::HOST)
        .and_then(|h| h.to_str().ok());
    if let (Some(authority), Some(host)) = (origin_authority, host) {
        if authority.eq_ignore_ascii_case(host) {
            return true;
        }
    }

    if via_trusted_proxy && matches_forwarded_host(origin, headers) {
        return true;
    }

    let public_origin = config.public_url.as_deref().and_then(url_origin);
    if public_origin.is_some_and(|p| p.eq_ignore_ascii_case(origin)) {
        return true;
    }

    config
        .security
        .cors_allowed_origins
        .iter()
        .any(|allowed| allowed.trim_end_matches('/').eq_ignore_ascii_case(origin))
}

/// Whether `origin` is the public address a proxy reports in
/// `X-Forwarded-Host`, with the scheme from `X-Forwarded-Proto` when given.
/// Only meaningful for requests from a trusted proxy.
fn matches_forwarded_host(origin: &str, headers: &HeaderMap) -> bool {
    let first_entry = |name: &str| {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(',').next())
            .map(str::trim)
            .filter(|v| !v.is_empty())
    };
    let Some(host) = first_entry("x-forwarded-host") else {
        return false;
    };
    match (first_entry("x-forwarded-proto"), origin.split_once("://")) {
        (Some(proto), _) => origin.eq_ignore_ascii_case(&format!("{}://{}", proto, host)),
        (None, Some((_, authority))) => authority.eq_ignore_ascii_case(host),
        (None, None) => false,
    }
}

/// `scheme://host[:port]` of a URL, without path.
fn url_origin(url: &str) -> Option<String> {
    let (scheme, rest) = url.split_once("://")?;
    let authority = rest.split('/').next()?;
    if authority.is_empty() {
        return None;
    }
    Some(format!("{}://{}", scheme, authority))
}

/// Extract JWT token from WebSocket headers securely
/// Uses Sec-WebSocket-Protocol header to avoid token exposure in logs/URLs
fn extract_websocket_token(headers: &HeaderMap) -> Option<String> {
    extract_websocket_token_and_protocol(headers).map(|(token, _)| token)
}

/// Extract JWT token and protocol string from WebSocket headers
/// Returns both the token and the original protocol for handshake acknowledgment
fn extract_websocket_token_and_protocol(headers: &HeaderMap) -> Option<(String, String)> {
    // Check for token in Sec-WebSocket-Protocol header (most secure)
    if let Some(protocol_header) = headers.get("sec-websocket-protocol") {
        if let Ok(protocols) = protocol_header.to_str() {
            // Format: "bearer.{token}" or "bearer, {token}"
            for protocol in protocols.split(',') {
                let protocol = protocol.trim();
                if protocol.starts_with("bearer.") {
                    let token = protocol.trim_start_matches("bearer.").to_string();
                    return Some((token, protocol.to_string()));
                }
                if protocol.starts_with("bearer ") {
                    let token = protocol.trim_start_matches("bearer ").to_string();
                    return Some((token, protocol.to_string()));
                }
            }
        }
    }
    
    // Fallback to Authorization header for backward compatibility
    if let Some(auth_header) = headers.get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") {
                let token = auth_str.trim_start_matches("Bearer ").to_string();
                // For Authorization header fallback, create a protocol format
                return Some((token.clone(), format!("bearer.{}", token)));
            }
        }
    }
    
    None
}

/// Get current sync progress (one-time API call)
#[utoipa::path(
    get,
    path = "/api/sources/{id}/sync/status",
    tag = "sources",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = Uuid, Path, description = "Source ID")
    ),
    responses(
        (status = 200, description = "Current sync progress", body = crate::services::sync_progress_tracker::SyncProgressInfo),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Source not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_sync_status(
    auth_user: AuthUser,
    Path(source_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Option<crate::services::sync_progress_tracker::SyncProgressInfo>>, StatusCode> {
    // Verify the source exists and the user has access
    let _source = state
        .db
        .get_source(auth_user.user.id, source_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Get current progress
    let progress_info = state.sync_progress_tracker.get_progress(source_id);

    Ok(Json(progress_info))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(origin: Option<&str>, host: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert("host", host.parse().unwrap());
        if let Some(origin) = origin {
            h.insert("origin", origin.parse().unwrap());
        }
        h
    }

    #[test]
    fn websocket_origin_checks() {
        let mut config = crate::test_helpers::create_test_config();
        config.public_url = Some("https://docs.example.com/app".to_string());
        config.security.cors_allowed_origins = vec!["http://localhost:5173".to_string()];

        assert!(websocket_origin_allowed(&headers(None, "internal:8000"), false, &config));
        assert!(websocket_origin_allowed(&headers(Some("http://internal:8000"), "internal:8000"), false, &config));
        assert!(websocket_origin_allowed(&headers(Some("https://docs.example.com"), "internal:8000"), false, &config));
        assert!(websocket_origin_allowed(&headers(Some("http://localhost:5173"), "internal:8000"), false, &config));
        assert!(!websocket_origin_allowed(&headers(Some("https://other.example"), "internal:8000"), false, &config));
        assert!(!websocket_origin_allowed(&headers(Some("null"), "internal:8000"), false, &config));
    }

    fn forwarded(origin: &str, host: &str, proto: Option<&str>) -> HeaderMap {
        let mut h = headers(Some(origin), "readur:8000");
        h.insert("x-forwarded-host", host.parse().unwrap());
        if let Some(proto) = proto {
            h.insert("x-forwarded-proto", proto.parse().unwrap());
        }
        h
    }

    #[test]
    fn websocket_origin_matches_forwarded_host_only_from_trusted_proxies() {
        let config = crate::test_helpers::create_test_config();
        let behind_proxy = forwarded("https://readur.example.com", "readur.example.com", Some("https"));
        assert!(websocket_origin_allowed(&behind_proxy, true, &config));
        // The same headers from a peer that is not a trusted proxy are ignored.
        assert!(!websocket_origin_allowed(&behind_proxy, false, &config));

        // Without X-Forwarded-Proto the host[:port] alone is compared.
        let no_proto = forwarded("https://readur.example.com:8443", "readur.example.com:8443", None);
        assert!(websocket_origin_allowed(&no_proto, true, &config));
        // The scheme must match when the proxy reports one.
        let wrong_scheme = forwarded("http://readur.example.com", "readur.example.com", Some("https"));
        assert!(!websocket_origin_allowed(&wrong_scheme, true, &config));
        // A different host is still refused.
        let other = forwarded("https://other.example", "readur.example.com", Some("https"));
        assert!(!websocket_origin_allowed(&other, true, &config));
        // The first entry of a proxy chain is the public address.
        let chain = forwarded("https://readur.example.com", "readur.example.com, internal:8000", Some("https, http"));
        assert!(websocket_origin_allowed(&chain, true, &config));
        // Without X-Forwarded-Host nothing extra is accepted.
        let mut no_host = headers(Some("https://readur.example.com"), "readur:8000");
        no_host.insert("x-forwarded-proto", "https".parse().unwrap());
        assert!(!websocket_origin_allowed(&no_host, true, &config));
    }
}