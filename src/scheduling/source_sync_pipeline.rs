//! The generic sync pipeline every backend runs: discovery, filtering and
//! bounded-concurrency download and ingestion of each file.

use anyhow::{anyhow, Context, Result};
use futures::stream::{FuturesUnordered, StreamExt};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{
    ingestion::document_ingestion::{DocumentIngestionService, IngestionResult},
    models::{FileIngestionInfo, UserRole},
    services::webdav::{PendingDirectoryCommit, SyncPhase, SyncProgress},
    utils::extension_filter::ExtensionFilter,
    utils::sync_cancelled::{cancelled, is_cancelled},
    AppState,
};

use super::{FileOutcome, FolderDiscovery, SourceSyncService, SyncOutcome};

/// How many files one sync downloads and ingests at the same time.
const CONCURRENT_FILES: usize = 5;

/// What one [`SourceSyncService::run_pipeline`] call syncs: whose source,
/// which folders, which file types, and the run's cancellation and progress
/// handles.
pub struct PipelineRun<'a> {
    pub user_id: Uuid,
    pub source_id: Uuid,
    pub watch_folders: &'a [String],
    pub file_extensions: &'a [String],
    pub enable_background_ocr: bool,
    pub cancellation_token: CancellationToken,
    pub progress: &'a Arc<SyncProgress>,
}

/// Everything one discovered file's processing shares with the rest of its
/// run; cloned into each file's future.
#[derive(Clone)]
struct FileContext {
    state: Arc<AppState>,
    user_id: Uuid,
    source_id: Uuid,
    enable_background_ocr: bool,
    semaphore: Arc<Semaphore>,
    cancellation_token: CancellationToken,
}

impl SourceSyncService {
    /// The generic sync: list every watch folder once with `discover_files`,
    /// keep the files that pass the extension and size filters, then download
    /// (`download_file`) and ingest them with bounded concurrency.
    ///
    /// Listing failures are counted per folder; the run only fails outright
    /// when no folder could be listed. File failures are counted and the run
    /// carries on, so one bad file does not block the rest. `progress` is fed
    /// the phase, the file counts and the errors as the run goes.
    pub async fn run_pipeline<F, D, Fut1, Fut2>(
        &self,
        run: PipelineRun<'_>,
        discover_files: F,
        download_file: D,
    ) -> Result<SyncOutcome>
    where
        F: Fn(String) -> Fut1,
        D: Fn(String) -> Fut2 + Clone,
        Fut1: std::future::Future<Output = Result<FolderDiscovery>>,
        Fut2: std::future::Future<Output = Result<Vec<u8>>>,
    {
        let PipelineRun {
            user_id,
            source_id,
            watch_folders,
            file_extensions,
            enable_background_ocr,
            cancellation_token,
            progress,
        } = run;
        let filter = ExtensionFilter::new(file_extensions);
        let max_file_bytes = i64::try_from(
            self.state
                .config
                .max_file_size_mb
                .saturating_mul(1024 * 1024),
        )
        .unwrap_or(i64::MAX);
        let mut outcome = SyncOutcome {
            folders: watch_folders.len(),
            ..Default::default()
        };
        let mut files_to_process: Vec<FileIngestionInfo> = Vec::new();
        let mut pending_commits: Vec<PendingDirectoryCommit> = Vec::new();
        let mut failed_files: Vec<String> = Vec::new();

        progress.set_phase(SyncPhase::DiscoveringFiles);
        for folder_path in watch_folders {
            if cancellation_token.is_cancelled() {
                info!("Sync cancelled during folder discovery");
                outcome.cancelled = true;
                return Ok(outcome);
            }

            match discover_files(folder_path.clone()).await {
                Ok(discovery) => {
                    if let Some(mut commit) = discovery.directory_commit {
                        commit.exclude_failures(&[], &discovery.failed_directories);
                        pending_commits.push(commit);
                    }
                    for directory in &discovery.failed_directories {
                        error!(
                            "Could not list directory {} under watch folder {}",
                            directory, folder_path
                        );
                        outcome.directories_failed += 1;
                        let message = format!("directory {}: could not be listed", directory);
                        progress.add_error(&message);
                        outcome.record_error(message);
                    }
                    let before = files_to_process.len();
                    for file_info in discovery.files {
                        if file_info.is_directory || !filter.matches_name(&file_info.name) {
                            continue;
                        }
                        if max_file_bytes > 0 && file_info.size > max_file_bytes {
                            outcome.skip_oversized();
                            continue;
                        }
                        outcome.discovered += 1;
                        outcome.total_size_bytes += file_info.size;
                        files_to_process.push(file_info);
                    }
                    info!(
                        "Found {} files to process in folder {}",
                        files_to_process.len() - before,
                        folder_path
                    );
                }
                // A listing interrupted by the stop request is not a folder failure.
                // The token is consulted too: a listing that fails for any reason
                // while the stop lands is simply listed again next run.
                Err(e) if is_cancelled(&e) || cancellation_token.is_cancelled() => {
                    info!("Sync cancelled while listing folder {}", folder_path);
                    outcome.cancelled = true;
                    return Ok(outcome);
                }
                Err(e) => {
                    error!("Failed to discover files in folder {}: {}", folder_path, e);
                    outcome.folders_failed += 1;
                    let message = format!("folder {}: {}", folder_path, e);
                    progress.add_error(&message);
                    outcome.record_error(message);
                }
            }
        }

        if outcome.oversized > 0 {
            warn!(
                "Skipped {} files larger than the {} MiB limit (max_file_size_mb) for source {}",
                outcome.oversized, self.state.config.max_file_size_mb, source_id
            );
        }
        if cancellation_token.is_cancelled() {
            info!("Sync cancelled after folder discovery");
            outcome.cancelled = true;
            return Ok(outcome);
        }
        if !watch_folders.is_empty() && outcome.folders_failed == watch_folders.len() {
            return Err(anyhow!(
                "Could not list any of the {} watch folders; first error: {}",
                watch_folders.len(),
                outcome.first_error.as_deref().unwrap_or("unknown")
            ));
        }

        progress.add_files_found(outcome.discovered);
        progress.set_phase(SyncPhase::ProcessingFiles);
        if let Err(e) = self
            .state
            .db
            .update_source_sync_stats(
                source_id,
                outcome.discovered as i64,
                outcome.total_size_bytes,
            )
            .await
        {
            error!("Failed to update initial sync stats: {}", e);
        }

        let file_context = FileContext {
            state: self.state.clone(),
            user_id,
            source_id,
            enable_background_ocr,
            semaphore: Arc::new(Semaphore::new(CONCURRENT_FILES)),
            cancellation_token: cancellation_token.clone(),
        };
        let mut file_futures = FuturesUnordered::new();
        for file_info in files_to_process {
            let context = file_context.clone();
            let download_file_clone = download_file.clone();
            file_futures.push(async move {
                let result = Self::process_single_file_with_cancellation(
                    context,
                    &file_info,
                    download_file_clone,
                )
                .await;
                (file_info, result)
            });
        }

        // Every future checks the token before doing any work, so after a
        // cancellation the remaining ones return promptly without downloading.
        while let Some((file_info, result)) = file_futures.next().await {
            // Progress counts every file the run is done with, so the percent
            // reaches 100 on a run that mostly skips; bytes count ingested ones.
            match &result {
                Ok(FileOutcome::Ingested) => {
                    progress.add_files_processed(1, file_info.size.max(0) as u64)
                }
                Ok(_) => progress.add_files_processed(1, 0),
                Err(_) => {}
            }
            match result {
                Ok(FileOutcome::Ingested) => {
                    outcome.processed += 1;
                    // Update statistics every 10 files processed or every file if under 10 total
                    if outcome.processed.is_multiple_of(10) || outcome.discovered <= 10 {
                        if let Err(e) = self
                            .state
                            .db
                            .update_source_sync_stats(
                                source_id,
                                outcome.pending(),
                                outcome.total_size_bytes,
                            )
                            .await
                        {
                            error!("Failed to update sync stats: {}", e);
                        }
                    }
                }
                Ok(FileOutcome::Skipped) => outcome.skipped += 1,
                Ok(FileOutcome::Unchanged) => {
                    outcome.skipped += 1;
                    outcome.unchanged += 1;
                }
                Err(e) if is_cancelled(&e) => {
                    debug!(
                        "File {} not processed: sync cancelled",
                        file_info.relative_path
                    );
                }
                Err(e) => {
                    error!(
                        "File processing error for {}: {:#}",
                        file_info.relative_path, e
                    );
                    outcome.failed += 1;
                    let message = format!("{}: {:#}", file_info.relative_path, e);
                    progress.add_error(&message);
                    outcome.record_error(message);
                    failed_files.push(file_info.relative_path.clone());
                }
            }
        }
        outcome.cancelled = cancellation_token.is_cancelled();
        if outcome.cancelled {
            info!("Sync cancelled: directory state of {} folders is not recorded, they are scanned again next run", pending_commits.len());
        } else {
            PendingDirectoryCommit::commit_all(&self.state.db, pending_commits, &failed_files)
                .await;
        }

        info!("Source sync finished for {}: {} discovered, {} processed, {} skipped, {} failed, {} folders failed{}",
              source_id, outcome.discovered, outcome.processed, outcome.skipped, outcome.failed, outcome.folders_failed,
              if outcome.cancelled { " (cancelled)" } else { "" });
        Ok(outcome)
    }

    async fn process_single_file_with_cancellation<D, Fut>(
        context: FileContext,
        file_info: &FileIngestionInfo,
        download_file: D,
    ) -> Result<FileOutcome>
    where
        D: Fn(String) -> Fut,
        Fut: std::future::Future<Output = Result<Vec<u8>>>,
    {
        let FileContext {
            state,
            user_id,
            source_id,
            enable_background_ocr,
            semaphore,
            cancellation_token,
        } = context;
        // Check for cancellation before starting file processing
        if cancellation_token.is_cancelled() {
            return Err(cancelled("Processing"));
        }

        let _permit = semaphore
            .acquire()
            .await
            .map_err(|e| anyhow!("Semaphore error: {}", e))?;

        debug!("Processing file: {}", file_info.relative_path);

        // Check for cancellation again after acquiring semaphore
        if cancellation_token.is_cancelled() {
            return Err(cancelled("Processing"));
        }

        // A document already stored for this path with the same size and
        // modification time needs no download. Any lookup failure or unknown
        // value falls through to the download and content check.
        match state
            .db
            .find_synced_document(user_id, source_id, &file_info.relative_path)
            .await
        {
            Ok(Some(stored)) if stored.matches(file_info.size, file_info.last_modified) => {
                debug!(
                    "Skipping unchanged file {} (document {})",
                    file_info.relative_path, stored.id
                );
                return Ok(FileOutcome::Unchanged);
            }
            Ok(_) => {}
            Err(e) => warn!(
                "Could not look up stored document for {}: {}",
                file_info.relative_path, e
            ),
        }

        // Download the file
        let file_data = download_file(file_info.relative_path.clone())
            .await
            .with_context(|| format!("Failed to download {}", file_info.relative_path))?;

        // Check for cancellation after download, before ingestion
        if cancellation_token.is_cancelled() {
            return Err(cancelled("Processing"));
        }

        debug!(
            "Downloaded file: {} ({} bytes)",
            file_info.name,
            file_data.len()
        );

        // A listing size of 0 means the source did not report one; otherwise
        // a different length means a truncated transfer or a file that was
        // still being written, so leave it for the next run.
        if file_info.size > 0 && file_data.len() as i64 != file_info.size {
            return Err(anyhow!(
                "size mismatch for {}: listed {} bytes, downloaded {}; file may have changed during sync",
                file_info.relative_path, file_info.size, file_data.len()
            ));
        }

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
            IngestionResult::Skipped {
                existing_document_id,
                reason,
            } => {
                info!(
                    "Skipped duplicate file {}: {} (existing: {})",
                    file_info.name, reason, existing_document_id
                );
                return Ok(FileOutcome::Skipped);
            }
            // Produced when another writer stored the same content first:
            // nothing new was ingested by this run.
            IngestionResult::ExistingDocument(doc) => {
                debug!("Found existing document for {}: {}", file_info.name, doc.id);
                return Ok(FileOutcome::Skipped);
            }
            IngestionResult::TrackedAsDuplicate {
                existing_document_id,
            } => {
                info!(
                    "Tracked {} as duplicate of existing document: {}",
                    file_info.name, existing_document_id
                );
                return Ok(FileOutcome::Skipped);
            }
        };

        // Queue for OCR if enabled and this is a new document (OCR continues even if sync is cancelled)
        if enable_background_ocr && should_queue_ocr {
            debug!(
                "Background OCR enabled, queueing document {} for processing",
                document.id
            );

            let priority = crate::ocr::priority::priority_for_file_size(file_info.size);

            if let Err(e) = state
                .queue_service
                .enqueue_document(document.id, priority, file_info.size)
                .await
            {
                // Without a queue row the document would stay pending forever and
                // the content-hash check would skip it on every later run, so
                // discard it and let the next run retry the file.
                Self::discard_document(&state, &document).await;
                return Err(anyhow!(
                    "Failed to enqueue {} for OCR: {}",
                    file_info.name,
                    e
                ));
            }
            debug!("Enqueued document {} for OCR processing", document.id);
        }

        Ok(FileOutcome::Ingested)
    }

    /// Removes a document this run created but could not finish setting up:
    /// the row first, and its stored files only once the row is gone. If the
    /// row cannot be deleted the files stay, so the surviving document remains
    /// openable; a lingering file after a successful row delete is only an
    /// orphan, which is logged.
    async fn discard_document(state: &AppState, document: &crate::models::Document) {
        match state
            .db
            .delete_document(document.id, document.user_id, UserRole::User)
            .await
        {
            Ok(true) => {}
            Ok(false) => warn!(
                "Document {} was already gone while discarding it",
                document.id
            ),
            Err(e) => {
                error!(
                    "Failed to delete document {} after a failed OCR enqueue, keeping its stored files: {}",
                    document.id, e
                );
                return;
            }
        }
        if let Err(e) = state.file_service.delete_document_files(document).await {
            error!(
                "Failed to delete stored files of discarded document {}: {}",
                document.id, e
            );
        }
    }
}
