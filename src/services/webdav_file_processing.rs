//! Downloading and ingesting the files a WebDAV listing produced: shared by
//! the settings-level WebDAV sync (`services::webdav_sync`) and the deep scan
//! of WebDAV sources.

use futures::stream::{FuturesUnordered, StreamExt};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{
    ingestion::document_ingestion::{
        DeduplicationPolicy, DocumentIngestionService, IngestionResult,
    },
    models::{source::CreateWebDAVFile, Document, FileIngestionInfo, UserRole},
    services::webdav::WebDAVService,
    AppState,
};

/// Downloads at most this many files at a time.
pub(crate) const CONCURRENT_DOWNLOADS: usize = 5;

/// How a run in progress is told to stop.
#[derive(Clone)]
pub(crate) enum StopSignal {
    /// A sync started by the source scheduler: the run's cancellation token.
    Token(CancellationToken),
    /// The settings-level WebDAV sync of the user: cancelled when the
    /// `webdav_sync_state` row no longer marks it running.
    UserSyncState,
}

/// How a listed file compares with its tracking record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Change {
    /// Same ETag as when it was last synced: skipped.
    Unchanged,
    EtagChanged,
    New,
    /// The tracking record could not be read; the file is processed.
    TrackingUnknown,
}

/// What processing a batch of discovered files produced.
#[derive(Debug, Default)]
pub struct ProcessedFiles {
    /// Files that were downloaded and ingested.
    pub ingested: usize,
    /// Paths of files that failed, so their directories are not recorded as done.
    pub failed_files: Vec<String>,
}

/// Processes one file at a time on behalf of a run; cheap to clone, so each
/// concurrent file gets its own handle sharing the download limit.
#[derive(Clone)]
pub(crate) struct FileProcessor {
    state: Arc<AppState>,
    user_id: Uuid,
    webdav_service: WebDAVService,
    enable_background_ocr: bool,
    source_id: Option<Uuid>,
    stop: StopSignal,
    downloads: Arc<Semaphore>,
}

impl FileProcessor {
    pub(crate) fn new(
        state: Arc<AppState>,
        user_id: Uuid,
        webdav_service: WebDAVService,
        enable_background_ocr: bool,
        source_id: Option<Uuid>,
        stop: StopSignal,
    ) -> Self {
        Self {
            state,
            user_id,
            webdav_service,
            enable_background_ocr,
            source_id,
            stop,
            downloads: Arc::new(Semaphore::new(CONCURRENT_DOWNLOADS)),
        }
    }

    /// Whether the run this processor works for has been told to stop.
    pub(crate) async fn stop_requested(&self) -> bool {
        match &self.stop {
            StopSignal::Token(token) => token.is_cancelled(),
            StopSignal::UserSyncState => matches!(
                self.state.db.get_webdav_sync_state(self.user_id).await,
                Ok(Some(sync_state)) if !sync_state.is_running
            ),
        }
    }

    /// Downloads and ingests one file. `Ok(true)` when it was ingested,
    /// `Ok(false)` when it was unchanged since the last sync or the run was
    /// stopped before the file was started.
    ///
    /// WebDAV files are tracked by `relative_path`: the WebDAV service sets it
    /// on every listed file (and sets the deprecated `path` to the same value),
    /// so the `webdav_files.webdav_path` key stays what it always was.
    pub(crate) async fn process(&self, file_info: &FileIngestionInfo) -> Result<bool, String> {
        let started = Instant::now();
        let request_id = Uuid::new_v4();
        let path = file_info.relative_path.as_str();

        let _permit = self
            .downloads
            .acquire()
            .await
            .map_err(|e| format!("Semaphore error: {}", e))?;
        // Checked after the permit, so files queued behind the concurrency
        // limit are not started once the run has been stopped.
        if self.stop_requested().await {
            return Ok(false);
        }

        let change = self.change(request_id, file_info).await;
        debug!("[{}] WebDAV file '{}': {:?}", request_id, path, change);
        if change == Change::Unchanged {
            return Ok(false);
        }
        let file_data = self
            .webdav_service
            .download_file(path)
            .await
            .map_err(|e| format!("Failed to download {}: {}", path, e))?;
        let size = file_data.len();

        let Some((document, queue_ocr, sync_status)) =
            self.ingest(request_id, file_info, file_data).await?
        else {
            // On the ignored-files list. Not tracked, so the file is looked
            // at again once the user takes it off the list.
            return Ok(false);
        };
        self.record_tracking(request_id, file_info, &document, sync_status)
            .await;
        if self.enable_background_ocr && queue_ocr {
            self.queue_ocr(request_id, &document, file_info.size).await;
        }

        let elapsed = started.elapsed().as_secs_f64();
        debug!(
            "[{}] '{}' ({} bytes) stored as document {} ({}) in {:.2}s",
            request_id, path, size, document.id, sync_status, elapsed
        );
        Ok(true)
    }

    /// How the file compares with its tracking record.
    async fn change(&self, request_id: Uuid, file_info: &FileIngestionInfo) -> Change {
        let path = file_info.relative_path.as_str();
        let tracked = self
            .state
            .db
            .get_webdav_file_by_path(self.user_id, path)
            .await;
        match tracked {
            Ok(Some(existing)) if existing.etag == file_info.etag => Change::Unchanged,
            Ok(Some(_)) => Change::EtagChanged,
            Ok(None) => Change::New,
            Err(e) => {
                warn!(
                    "[{}] Error checking existing WebDAV file '{}': {}",
                    request_id, path, e
                );
                Change::TrackingUnknown
            }
        }
    }

    /// Ingests the downloaded bytes. Returns the document the file maps to,
    /// whether it is new (and so needs OCR), and the tracking status to
    /// record; `None` when the content is on the user's ignored-files list.
    async fn ingest(
        &self,
        request_id: Uuid,
        file_info: &FileIngestionInfo,
        file_data: Vec<u8>,
    ) -> Result<Option<(Document, bool, &'static str)>, String> {
        let ingestion = DocumentIngestionService::new(
            self.state.db.clone(),
            self.state.file_service.as_ref().clone(),
        );
        // Without a source the file is ingested like a generic WebDAV sync,
        // under a temporary id for tracking.
        let (policy, source_id) = match self.source_id {
            Some(source_id) => (DeduplicationPolicy::TrackAsDuplicate, source_id),
            None => (DeduplicationPolicy::Skip, Uuid::new_v4()),
        };
        let result = ingestion
            .ingest_from_file_info(
                file_info,
                file_data,
                self.user_id,
                policy,
                "webdav_sync",
                Some(source_id),
            )
            .await
            .map_err(|e| format!("Document ingestion failed for {}: {}", file_info.name, e))?;

        match result {
            IngestionResult::Created(document) => Ok(Some((document, true, "synced"))),
            IngestionResult::ExistingDocument(document) => {
                Ok(Some((document, false, "duplicate_content")))
            }
            IngestionResult::Ignored { ignored_file_id } => {
                info!(
                    "[{}] Skipped '{}': on the ignored-files list ({})",
                    request_id, file_info.relative_path, ignored_file_id
                );
                Ok(None)
            }
            IngestionResult::TrackedAsDuplicate {
                existing_document_id,
            }
            | IngestionResult::Skipped {
                existing_document_id,
                ..
            } => {
                // The duplicate is still tracked, against the existing document.
                let document = self
                    .existing_document(request_id, existing_document_id)
                    .await?;
                Ok(Some((document, false, "duplicate_content")))
            }
        }
    }

    async fn existing_document(
        &self,
        request_id: Uuid,
        document_id: Uuid,
    ) -> Result<Document, String> {
        let found = self
            .state
            .db
            .get_document_by_id(document_id, self.user_id, UserRole::User)
            .await;
        match found {
            Ok(Some(document)) => Ok(document),
            Ok(None) => {
                error!(
                    "[{}] Document {} not found for duplicate tracking",
                    request_id, document_id
                );
                Err("Document not found".to_string())
            }
            Err(e) => {
                error!(
                    "[{}] Failed to get existing document {}: {}",
                    request_id, document_id, e
                );
                Err(format!("Failed to get existing document: {}", e))
            }
        }
    }

    async fn record_tracking(
        &self,
        request_id: Uuid,
        file_info: &FileIngestionInfo,
        document: &Document,
        sync_status: &str,
    ) {
        let webdav_file = CreateWebDAVFile {
            user_id: self.user_id,
            webdav_path: file_info.relative_path.clone(),
            etag: file_info.etag.clone(),
            last_modified: file_info.last_modified,
            file_size: file_info.size,
            mime_type: file_info.mime_type.clone(),
            document_id: Some(document.id),
            sync_status: sync_status.to_string(),
            sync_error: None,
        };
        if let Err(e) = self
            .state
            .db
            .create_or_update_webdav_file(&webdav_file)
            .await
        {
            error!(
                "[{}] Failed to record WebDAV file '{}': {}",
                request_id, file_info.relative_path, e
            );
        }
    }

    async fn queue_ocr(&self, request_id: Uuid, document: &Document, file_size: i64) {
        let priority = crate::ocr::priority::priority_for_file_size(file_size);
        match self
            .state
            .queue_service
            .enqueue_document(document.id, priority, file_size)
            .await
        {
            Ok(_) => debug!(
                "[{}] Enqueued document {} for OCR (priority {})",
                request_id, document.id, priority
            ),
            Err(e) => error!(
                "[{}] Failed to enqueue document {} for OCR: {}",
                request_id, document.id, e
            ),
        }
    }
}

/// Process files for deep scan - similar to regular sync but forces processing.
/// Once `cancellation` is triggered no further file is started and the
/// count of files processed so far is returned; the caller decides how to
/// record the interrupted run.
pub async fn process_files_for_deep_scan(
    state: Arc<AppState>,
    user_id: Uuid,
    webdav_service: &WebDAVService,
    files_to_process: &[FileIngestionInfo],
    enable_background_ocr: bool,
    webdav_source_id: Option<Uuid>,
    cancellation: &CancellationToken,
) -> Result<ProcessedFiles, anyhow::Error> {
    let started = Instant::now();
    let request_id = Uuid::new_v4();

    let processor = FileProcessor::new(
        state,
        user_id,
        webdav_service.clone(),
        enable_background_ocr,
        webdav_source_id,
        StopSignal::Token(cancellation.clone()),
    );
    let mut file_futures: FuturesUnordered<_> = files_to_process
        .iter()
        .map(|file_info| {
            let processor = processor.clone();
            async move {
                (
                    file_info.relative_path.clone(),
                    processor.process(file_info).await,
                )
            }
        })
        .collect();

    let mut processed = ProcessedFiles::default();
    while let Some((file_path, result)) = file_futures.next().await {
        match result {
            Ok(true) => processed.ingested += 1,
            Ok(false) => {}
            Err(error) => {
                error!(
                    "[{}] Deep scan file processing error: {}",
                    request_id, error
                );
                processed.failed_files.push(file_path);
            }
        }
    }

    let elapsed = started.elapsed().as_secs_f64();
    info!(
        "[{}] Deep scan processing completed: {} of {} files ingested, {} failed in {:.2}s",
        request_id,
        processed.ingested,
        files_to_process.len(),
        processed.failed_files.len(),
        elapsed
    );
    Ok(processed)
}
