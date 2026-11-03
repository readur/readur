use std::sync::Arc;
use anyhow::{anyhow, Result};
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use futures::stream::{FuturesUnordered, StreamExt};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{
    AppState,
    models::{FileIngestionInfo, Source, SourceType, SourceStatus, LocalFolderSourceConfig, S3SourceConfig, User, UserRole, WebDAVSourceConfig},
    ingestion::document_ingestion::{DocumentIngestionService, IngestionResult},
    services::local_folder_service::{authorize_local_folder_paths, LocalFolderService},
    services::s3_service::S3Service,
    services::webdav::{WebDAVService, WebDAVConfig, SyncProgress, SyncPhase},
    utils::extension_filter::ExtensionFilter,
    utils::outbound::{validate_outbound_url, validate_outbound_url_for_config},
};

/// Status message recorded on a source whose owner cannot sync it.
pub const OWNER_INACTIVE_MESSAGE: &str = "Sync skipped: the source owner's account is disabled";

/// How many files one sync downloads and ingests at the same time.
const CONCURRENT_FILES: usize = 5;

/// Sources sync on behalf of their owner, so only an active owner's sources sync.
fn owner_may_sync(owner: Option<&User>) -> bool {
    owner.is_some_and(|owner| owner.is_active)
}

/// Whether the owner may read local folders outside the configured allowlist
/// (an active admin, and only when no allowlist is configured).
fn owner_is_active_admin(owner: Option<&User>) -> bool {
    owner.is_some_and(|owner| owner.is_active && owner.role == UserRole::Admin)
}

/// What one sync run did, counted per file after the extension and size
/// filters. `discovered = processed + skipped + failed` once the run has
/// finished; a cancelled run stops early and leaves the remainder pending.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SyncOutcome {
    /// Files that passed the filters and were handed to the processor.
    pub discovered: usize,
    /// Files ingested as new documents.
    pub processed: usize,
    /// Files not ingested on purpose: already present, duplicate content, or
    /// over the size limit.
    pub skipped: usize,
    /// Files whose download or ingestion failed; they are retried next run.
    pub failed: usize,
    /// Watch folders the run tried to list.
    pub folders: usize,
    /// Watch folders whose listing failed.
    pub folders_failed: usize,
    /// The first folder or file error, for the source's `last_error`.
    pub first_error: Option<String>,
    /// Total size of the discovered files.
    pub total_size_bytes: i64,
    /// The run stopped because its cancellation token fired.
    pub cancelled: bool,
}

impl SyncOutcome {
    /// Nothing failed (a cancelled run can still be clean).
    pub fn is_clean(&self) -> bool {
        self.failed == 0 && self.folders_failed == 0
    }

    /// Files discovered this run that were neither ingested nor skipped.
    pub fn pending(&self) -> i64 {
        self.discovered.saturating_sub(self.processed + self.skipped) as i64
    }

    /// A one-line account of what failed, for the source's `last_error`;
    /// `None` when the run was clean.
    pub fn failure_summary(&self) -> Option<String> {
        if self.is_clean() {
            return None;
        }
        let mut parts = Vec::new();
        if self.folders_failed > 0 {
            parts.push(format!("{} of {} folders could not be listed", self.folders_failed, self.folders));
        }
        if self.failed > 0 {
            parts.push(format!("{} of {} files failed", self.failed, self.discovered));
        }
        let mut summary = parts.join("; ");
        if let Some(first) = &self.first_error {
            summary.push_str("; first: ");
            summary.push_str(first);
        }
        Some(summary)
    }

    fn record_error(&mut self, message: String) {
        if self.first_error.is_none() {
            self.first_error = Some(message);
        }
    }
}

/// What processing one discovered file produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileOutcome {
    Ingested,
    Skipped,
}

#[derive(Clone)]
pub struct SourceSyncService {
    state: Arc<AppState>,
}

impl SourceSyncService {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    /// Perform sync for any source type
    pub async fn sync_source(&self, source: &Source, enable_background_ocr: bool) -> Result<SyncOutcome> {
        // Call the cancellable version with no cancellation token
        self.sync_source_with_cancellation(source, enable_background_ocr, CancellationToken::new()).await
    }

    /// Perform sync for any source type with cancellation support
    pub async fn sync_source_with_cancellation(&self, source: &Source, enable_background_ocr: bool, cancellation_token: CancellationToken) -> Result<SyncOutcome> {
        info!("Starting sync for source {} ({})", source.name, source.source_type);

        // Check for cancellation before starting
        if cancellation_token.is_cancelled() {
            info!("Sync for source {} was cancelled before starting", source.name);
            return Err(anyhow!("Sync cancelled"));
        }

        let owner = self.state.db.get_user_by_id(source.user_id).await?;
        if !owner_may_sync(owner.as_ref()) {
            warn!("Skipping sync for source {} ({}): owner account is disabled", source.name, source.id);
            if let Err(e) = self.update_source_status(source.id, SourceStatus::Error, Some(OWNER_INACTIVE_MESSAGE)).await {
                error!("Failed to update source status: {}", e);
            }
            return Err(anyhow!(OWNER_INACTIVE_MESSAGE));
        }

        // Update source status to syncing
        if let Err(e) = self.update_source_status(source.id, SourceStatus::Syncing, None).await {
            error!("Failed to update source status: {}", e);
        }

        let progress = Arc::new(SyncProgress::new());
        progress.set_phase(SyncPhase::Initializing);
        self.state.sync_progress_tracker.register_sync(source.id, progress.clone());

        let sync_result = match source.source_type {
            SourceType::WebDAV => self.sync_webdav_source(source, enable_background_ocr, cancellation_token.clone(), &progress).await,
            SourceType::LocalFolder => self.sync_local_folder_source(source, enable_background_ocr, cancellation_token.clone()).await,
            SourceType::S3 => self.sync_s3_source(source, enable_background_ocr, cancellation_token.clone()).await,
        };

        match &sync_result {
            Ok(_) => progress.set_phase(SyncPhase::Completed),
            Err(e) => progress.set_phase(SyncPhase::Failed(e.to_string())),
        }
        self.state.sync_progress_tracker.unregister_sync(source.id);

        let cancelled = cancellation_token.is_cancelled() || sync_result.as_ref().is_ok_and(|o| o.cancelled);
        match &sync_result {
            _ if cancelled => {
                info!("Sync for source {} was cancelled", source.name);
                // Don't overwrite status if it's already been set to cancelled by stop_sync
                if let Err(e) = self.update_source_status_if_not_cancelled(source.id, SourceStatus::Idle, Some("Sync cancelled by user")).await {
                    error!("Failed to update source status after cancellation: {}", e);
                }
            }
            Ok(outcome) => match outcome.failure_summary() {
                None => {
                    info!("Sync completed for source {}: {} files processed", source.name, outcome.processed);
                    if let Err(e) = self.update_source_status_if_not_cancelled(source.id, SourceStatus::Idle, None).await {
                        error!("Failed to update source status after successful sync: {}", e);
                    }
                }
                Some(summary) => {
                    warn!("Sync finished with failures for source {}: {}", source.name, summary);
                    if let Err(e) = self.update_source_status_if_not_cancelled(source.id, SourceStatus::Error, Some(&summary)).await {
                        error!("Failed to update source status after partial sync: {}", e);
                    }
                }
            },
            Err(e) => {
                error!("Sync failed for source {}: {}", source.name, e);
                let error_msg = format!("Sync failed: {}", e);
                if let Err(e) = self.update_source_status_if_not_cancelled(source.id, SourceStatus::Error, Some(&error_msg)).await {
                    error!("Failed to update source status after error: {}", e);
                }
            }
        }

        sync_result
    }

    async fn sync_webdav_source(&self, source: &Source, enable_background_ocr: bool, cancellation_token: CancellationToken, progress: &Arc<SyncProgress>) -> Result<SyncOutcome> {
        let config: WebDAVSourceConfig = serde_json::from_value(source.config.clone())
            .map_err(|e| anyhow!("Invalid WebDAV config: {}", e))?;

        info!("WebDAV source sync config: server_url={}, username={}, watch_folders={:?}, file_extensions={:?}, server_type={:?}",
            config.server_url, config.username, config.watch_folders, config.file_extensions, config.server_type);

        // Re-check the destination at connect time; DNS may have changed
        // since the source was saved. A host that does not resolve right now
        // is left to the WebDAV client: its resolver applies the same address
        // checks, and its retry and error handling report the failure through
        // the sync progress like any other connection problem.
        validate_outbound_url_for_config(&WebDAVConfig::normalize_server_url(&config.server_url))
            .await
            .map_err(|e| anyhow!("{}", e))?;

        // Requests to list files in a Nextcloud folder might take > 2 minutes
        // Set timeout to 3 minutes to accommodate large folder structures
        let webdav_config = WebDAVConfig {
            server_url: config.server_url,
            username: config.username,
            password: config.password,
            watch_folders: config.watch_folders,
            file_extensions: config.file_extensions,
            timeout_seconds: 180, // 3 minutes for discover_files_in_folder operations
            server_type: config.server_type,
        };

        let webdav_service = WebDAVService::new(webdav_config.clone())
            .map_err(|e| anyhow!("Failed to create WebDAV service: {}", e))?;

        info!("WebDAV service created successfully, starting sync with {} folders", webdav_config.watch_folders.len());

        self.run_pipeline(
            source.user_id,
            source.id,
            &webdav_config.watch_folders,
            &webdav_config.file_extensions,
            enable_background_ocr,
            cancellation_token,
            |folder_path| {
                let service = webdav_service.clone();
                let state_clone = self.state.clone();
                let user_id = source.user_id;  // Capture user_id from source
                let progress = progress.clone(); // Clone progress for the async closure
                async move {
                    info!("🧠 Using smart sync for scheduled sync: {}", folder_path);
                    progress.set_current_directory(&folder_path);

                    // Use smart sync service for intelligent discovery
                    let smart_sync_service = crate::services::webdav::SmartSyncService::new(state_clone);

                    match smart_sync_service.evaluate_and_sync(user_id, Some(source.id), &service, &folder_path, Some(&progress)).await {
                        Ok(Some(sync_result)) => {
                            info!("✅ Smart sync completed for {}: {} files found using {:?}",
                                  folder_path, sync_result.files.len(), sync_result.strategy_used);
                            Ok(sync_result.files)
                        },
                        Ok(None) => {
                            info!("🔍 Smart sync: No changes detected for {}, skipping", folder_path);
                            Ok(Vec::new()) // No files to process
                        },
                        Err(e) => {
                            error!("Smart sync failed for scheduled sync {}: {}", folder_path, e);
                            Err(e)
                        }
                    }
                }
            },
            |file_path| {
                let service = webdav_service.clone();
                let progress = progress.clone(); // Clone progress for the async closure
                async move {
                    debug!("WebDAV download_file called for: {}", file_path);
                    progress.set_current_file(Some(&file_path));
                    let result = service.download_file(&file_path).await;
                    match &result {
                        Ok(data) => debug!("WebDAV downloaded {} bytes for file: {}", data.len(), file_path),
                        Err(e) => error!("WebDAV download failed for file {}: {}", file_path, e),
                    }
                    result
                }
            }
        ).await
    }

    async fn sync_local_folder_source(&self, source: &Source, enable_background_ocr: bool, cancellation_token: CancellationToken) -> Result<SyncOutcome> {
        let config: LocalFolderSourceConfig = serde_json::from_value(source.config.clone())
            .map_err(|e| anyhow!("Invalid LocalFolder config: {}", e))?;

        // Re-check against the current configuration: the allowlist or the
        // owner's role may have changed since the source was saved.
        let owner = self.state.db.get_user_by_id(source.user_id).await?;
        let owner_is_admin = owner_is_active_admin(owner.as_ref());
        authorize_local_folder_paths(
            &config.watch_folders,
            owner_is_admin,
            &self.state.config.security.local_source_allowed_paths,
        )
        .map_err(|e| anyhow!("Local folder source is not permitted: {}", e))?;

        let local_service = LocalFolderService::new(config.clone())
            .map_err(|e| anyhow!("Failed to create LocalFolder service: {}", e))?;

        self.run_pipeline(
            source.user_id,
            source.id,
            &config.watch_folders,
            &config.file_extensions,
            enable_background_ocr,
            cancellation_token,
            |folder_path| {
                let service = local_service.clone();
                async move { service.discover_files_in_folder(&folder_path).await }
            },
            |file_path| {
                let service = local_service.clone();
                async move { service.read_file(&file_path).await }
            }
        ).await
    }

    async fn sync_s3_source(&self, source: &Source, enable_background_ocr: bool, cancellation_token: CancellationToken) -> Result<SyncOutcome> {
        let config: S3SourceConfig = serde_json::from_value(source.config.clone())
            .map_err(|e| anyhow!("Invalid S3 config: {}", e))?;

        for url in S3Service::outbound_urls(&config) {
            validate_outbound_url(&url).await.map_err(|e| anyhow!("{}", e))?;
        }

        let s3_service = S3Service::new(config.clone()).await
            .map_err(|e| anyhow!("Failed to create S3 service: {}", e))?;

        self.run_pipeline(
            source.user_id,
            source.id,
            &config.watch_folders,
            &config.file_extensions,
            enable_background_ocr,
            cancellation_token,
            |folder_path| {
                let service = s3_service.clone();
                async move { service.discover_files_in_folder(&folder_path).await }
            },
            |file_path| {
                let service = s3_service.clone();
                async move { service.download_file(&file_path).await }
            }
        ).await
    }

    /// The generic sync: list every watch folder once with `discover_files`,
    /// keep the files that pass the extension and size filters, then download
    /// (`download_file`) and ingest them with bounded concurrency.
    ///
    /// Listing failures are counted per folder; the run only fails outright
    /// when no folder could be listed. File failures are counted and the run
    /// carries on, so one bad file does not block the rest.
    pub async fn run_pipeline<F, D, Fut1, Fut2>(
        &self,
        user_id: Uuid,
        source_id: Uuid,
        watch_folders: &[String],
        file_extensions: &[String],
        enable_background_ocr: bool,
        cancellation_token: CancellationToken,
        discover_files: F,
        download_file: D,
    ) -> Result<SyncOutcome>
    where
        F: Fn(String) -> Fut1,
        D: Fn(String) -> Fut2 + Clone,
        Fut1: std::future::Future<Output = Result<Vec<FileIngestionInfo>>>,
        Fut2: std::future::Future<Output = Result<Vec<u8>>>,
    {
        let filter = ExtensionFilter::new(file_extensions);
        let max_file_bytes = i64::try_from(self.state.config.max_file_size_mb.saturating_mul(1024 * 1024)).unwrap_or(i64::MAX);
        let mut outcome = SyncOutcome { folders: watch_folders.len(), ..Default::default() };
        let mut files_to_process: Vec<FileIngestionInfo> = Vec::new();
        let mut oversized = 0usize;

        for folder_path in watch_folders {
            if cancellation_token.is_cancelled() {
                info!("Sync cancelled during folder discovery");
                outcome.cancelled = true;
                return Ok(outcome);
            }

            match discover_files(folder_path.clone()).await {
                Ok(files) => {
                    let before = files_to_process.len();
                    for file_info in files {
                        if file_info.is_directory || !filter.matches_name(&file_info.name) {
                            continue;
                        }
                        if max_file_bytes > 0 && file_info.size > max_file_bytes {
                            oversized += 1;
                            continue;
                        }
                        outcome.total_size_bytes += file_info.size;
                        files_to_process.push(file_info);
                    }
                    info!("Found {} files to process in folder {}", files_to_process.len() - before, folder_path);
                }
                Err(e) => {
                    error!("Failed to discover files in folder {}: {}", folder_path, e);
                    outcome.folders_failed += 1;
                    outcome.record_error(format!("folder {}: {}", folder_path, e));
                }
            }
        }

        if oversized > 0 {
            warn!("Skipped {} files larger than the {} MiB limit (max_file_size_mb) for source {}",
                  oversized, self.state.config.max_file_size_mb, source_id);
            outcome.skipped += oversized;
        }
        if !watch_folders.is_empty() && outcome.folders_failed == watch_folders.len() {
            return Err(anyhow!(
                "Could not list any of the {} watch folders; first error: {}",
                watch_folders.len(),
                outcome.first_error.as_deref().unwrap_or("unknown")
            ));
        }

        outcome.discovered = files_to_process.len();
        if let Err(e) = self.state.db.update_source_sync_stats(
            source_id,
            0, // files_synced starts at 0
            outcome.discovered as i64,
            outcome.total_size_bytes,
        ).await {
            error!("Failed to update initial sync stats: {}", e);
        }

        let semaphore = Arc::new(Semaphore::new(CONCURRENT_FILES));
        let mut file_futures = FuturesUnordered::new();
        for file_info in files_to_process {
            let state_clone = self.state.clone();
            let semaphore_clone = semaphore.clone();
            let download_file_clone = download_file.clone();
            let cancellation_token_clone = cancellation_token.clone();
            file_futures.push(async move {
                let result = Self::process_single_file_with_cancellation(
                    state_clone,
                    user_id,
                    source_id,
                    &file_info,
                    enable_background_ocr,
                    semaphore_clone,
                    download_file_clone,
                    cancellation_token_clone,
                ).await;
                (file_info, result)
            });
        }

        // Every future checks the token before doing any work, so after a
        // cancellation the remaining ones return promptly without downloading.
        while let Some((file_info, result)) = file_futures.next().await {
            match result {
                Ok(FileOutcome::Ingested) => {
                    outcome.processed += 1;
                    // Update statistics every 10 files processed or every file if under 10 total
                    if outcome.processed % 10 == 0 || outcome.discovered <= 10 {
                        if let Err(e) = self.state.db.update_source_sync_stats(
                            source_id,
                            outcome.processed as i64,
                            outcome.pending(),
                            outcome.total_size_bytes,
                        ).await {
                            error!("Failed to update sync stats: {}", e);
                        }
                    }
                }
                Ok(FileOutcome::Skipped) => outcome.skipped += 1,
                Err(_) if cancellation_token.is_cancelled() => {
                    debug!("File {} not processed: sync cancelled", file_info.relative_path);
                }
                Err(e) => {
                    error!("File processing error for {}: {}", file_info.relative_path, e);
                    outcome.failed += 1;
                    outcome.record_error(format!("{}: {}", file_info.relative_path, e));
                }
            }
        }
        outcome.cancelled = cancellation_token.is_cancelled();

        if let Err(e) = self.state.db.update_source_sync_stats(
            source_id,
            outcome.processed as i64,
            outcome.pending(),
            outcome.total_size_bytes,
        ).await {
            error!("Failed to update final sync stats: {}", e);
        }

        info!("Source sync finished for {}: {} discovered, {} processed, {} skipped, {} failed, {} folders failed{}",
              source_id, outcome.discovered, outcome.processed, outcome.skipped, outcome.failed, outcome.folders_failed,
              if outcome.cancelled { " (cancelled)" } else { "" });
        Ok(outcome)
    }

    async fn process_single_file_with_cancellation<D, Fut>(
        state: Arc<AppState>,
        user_id: Uuid,
        source_id: Uuid,
        file_info: &FileIngestionInfo,
        enable_background_ocr: bool,
        semaphore: Arc<Semaphore>,
        download_file: D,
        cancellation_token: CancellationToken,
    ) -> Result<FileOutcome>
    where
        D: Fn(String) -> Fut,
        Fut: std::future::Future<Output = Result<Vec<u8>>>,
    {
        // Check for cancellation before starting file processing
        if cancellation_token.is_cancelled() {
            return Err(anyhow!("Processing cancelled"));
        }

        let _permit = semaphore.acquire().await
            .map_err(|e| anyhow!("Semaphore error: {}", e))?;

        debug!("Processing file: {}", file_info.relative_path);

        // Check for cancellation again after acquiring semaphore
        if cancellation_token.is_cancelled() {
            return Err(anyhow!("Processing cancelled"));
        }

        // Download the file
        let file_data = download_file(file_info.relative_path.clone()).await
            .map_err(|e| anyhow!("Failed to download {}: {}", file_info.relative_path, e))?;

        // Check for cancellation after download, before ingestion
        if cancellation_token.is_cancelled() {
            return Err(anyhow!("Processing cancelled"));
        }

        debug!("Downloaded file: {} ({} bytes)", file_info.name, file_data.len());

        // Use the unified ingestion service for consistent deduplication
        let file_service = (*state.file_service).clone();
        let ingestion_service = DocumentIngestionService::new(state.db.clone(), file_service);

        let result = ingestion_service
            .ingest_from_file_info(
                file_info,
                file_data,
                user_id,
                crate::ingestion::document_ingestion::DeduplicationPolicy::Skip,
                "source_sync",
                Some(source_id),
            )
            .await
            .map_err(|e| anyhow!("Document ingestion failed for {}: {}", file_info.name, e))?;

        let (document, should_queue_ocr) = match result {
            IngestionResult::Created(doc) => {
                debug!("Created new document for {}: {}", file_info.name, doc.id);
                (doc, true) // New document - queue for OCR
            }
            IngestionResult::Skipped { existing_document_id, reason } => {
                info!("Skipped duplicate file {}: {} (existing: {})", file_info.name, reason, existing_document_id);
                return Ok(FileOutcome::Skipped);
            }
            IngestionResult::ExistingDocument(doc) => {
                debug!("Found existing document for {}: {}", file_info.name, doc.id);
                (doc, false) // Existing document - don't re-queue OCR
            }
            IngestionResult::TrackedAsDuplicate { existing_document_id } => {
                info!("Tracked {} as duplicate of existing document: {}", file_info.name, existing_document_id);
                return Ok(FileOutcome::Skipped);
            }
        };

        // Queue for OCR if enabled and this is a new document (OCR continues even if sync is cancelled)
        if enable_background_ocr && should_queue_ocr {
            debug!("Background OCR enabled, queueing document {} for processing", document.id);

            let priority = if file_info.size <= 1024 * 1024 { 10 }
            else if file_info.size <= 5 * 1024 * 1024 { 8 }
            else if file_info.size <= 10 * 1024 * 1024 { 6 }
            else if file_info.size <= 50 * 1024 * 1024 { 4 }
            else { 2 };

            if let Err(e) = state.queue_service.enqueue_document(document.id, priority, file_info.size).await {
                error!("Failed to enqueue document for OCR: {}", e);
            } else {
                debug!("Enqueued document {} for OCR processing", document.id);
            }
        }

        Ok(FileOutcome::Ingested)
    }

    async fn update_source_status(&self, source_id: Uuid, status: SourceStatus, error_message: Option<&str>) -> Result<()> {
        let query = if let Some(error) = error_message {
            sqlx::query(
                r#"UPDATE sources
                   SET status = $2, last_error = $3, last_error_at = NOW(), updated_at = NOW()
                   WHERE id = $1"#
            )
            .bind(source_id)
            .bind(status.to_string())
            .bind(crate::db::sanitize::DbText(error))
        } else {
            sqlx::query(
                r#"UPDATE sources
                   SET status = $2, last_error = NULL, last_error_at = NULL, updated_at = NOW()
                   WHERE id = $1"#
            )
            .bind(source_id)
            .bind(status.to_string())
        };

        query.execute(self.state.db.get_pool()).await
            .map_err(|e| anyhow!("Failed to update source status: {}", e))?;

        Ok(())
    }

    /// Update source status only if it hasn't already been set to cancelled
    /// This prevents race conditions where stop_sync sets status to idle and sync task overwrites it
    async fn update_source_status_if_not_cancelled(&self, source_id: Uuid, status: SourceStatus, error_message: Option<&str>) -> Result<()> {
        let query = if let Some(error) = error_message {
            sqlx::query(
                r#"UPDATE sources
                   SET status = $2, last_error = $3, last_error_at = NOW(), updated_at = NOW()
                   WHERE id = $1 AND NOT (status = 'idle' AND last_error = 'Sync cancelled by user')"#
            )
            .bind(source_id)
            .bind(status.to_string())
            .bind(crate::db::sanitize::DbText(error))
        } else {
            sqlx::query(
                r#"UPDATE sources
                   SET status = $2, last_error = NULL, last_error_at = NULL, updated_at = NOW()
                   WHERE id = $1 AND NOT (status = 'idle' AND last_error = 'Sync cancelled by user')"#
            )
            .bind(source_id)
            .bind(status.to_string())
        };

        let result = query.execute(self.state.db.get_pool()).await
            .map_err(|e| anyhow!("Failed to update source status: {}", e))?;

        if result.rows_affected() == 0 {
            info!("Source {} status not updated - already cancelled by user", source_id);
        }

        Ok(())
    }

}

#[cfg(test)]
mod owner_tests {
    use super::*;
    use crate::models::AuthProvider;

    fn owner(role: UserRole, is_active: bool) -> User {
        User {
            id: Uuid::new_v4(),
            username: "owner".into(),
            email: "owner@example.com".into(),
            password_hash: None,
            role,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            oidc_subject: None,
            oidc_issuer: None,
            oidc_email: None,
            auth_provider: AuthProvider::Local,
            token_version: 0,
            is_active,
        }
    }

    #[test]
    fn only_active_owners_sync() {
        assert!(owner_may_sync(Some(&owner(UserRole::User, true))));
        assert!(!owner_may_sync(Some(&owner(UserRole::User, false))));
        assert!(!owner_may_sync(Some(&owner(UserRole::Admin, false))));
        assert!(!owner_may_sync(None));
    }

    #[test]
    fn local_folder_admin_rights_require_an_active_admin() {
        assert!(owner_is_active_admin(Some(&owner(UserRole::Admin, true))));
        assert!(!owner_is_active_admin(Some(&owner(UserRole::Admin, false))));
        assert!(!owner_is_active_admin(Some(&owner(UserRole::User, true))));
        assert!(!owner_is_active_admin(None));
    }
}

#[cfg(test)]
mod outcome_tests {
    use super::*;

    #[test]
    fn pending_is_what_was_neither_ingested_nor_skipped() {
        let outcome = SyncOutcome { discovered: 10, processed: 4, skipped: 3, failed: 2, ..Default::default() };
        assert_eq!(outcome.pending(), 3);
        assert!(!outcome.is_clean());
        assert!(SyncOutcome { discovered: 2, processed: 2, ..Default::default() }.is_clean());
    }

    #[test]
    fn only_the_first_error_is_kept() {
        let mut outcome = SyncOutcome::default();
        outcome.record_error("one".into());
        outcome.record_error("two".into());
        assert_eq!(outcome.first_error.as_deref(), Some("one"));
    }

    #[test]
    fn failure_summary_counts_folders_and_files() {
        assert_eq!(SyncOutcome { discovered: 2, processed: 2, ..Default::default() }.failure_summary(), None);
        let outcome = SyncOutcome {
            discovered: 120, processed: 117, failed: 3, folders: 2, folders_failed: 1,
            first_error: Some("folder /b: 500".into()), ..Default::default()
        };
        assert_eq!(
            outcome.failure_summary().as_deref(),
            Some("1 of 2 folders could not be listed; 3 of 120 files failed; first: folder /b: 500")
        );
    }
}
