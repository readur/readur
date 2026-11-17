//! The WebDAV sync of a user's settings-level WebDAV account. Started by the
//! REST trigger (`routes::webdav`) and the background
//! `scheduling::webdav_scheduler`, so it lives below both.
//!
//! One sync per user at a time: [`start_webdav_sync`] claims the user before
//! anything runs, in two steps that must both succeed. An in-process slot is
//! held until the sync task has actually exited (a cancelled task keeps it
//! while it winds down, so a new run never starts underneath it), and the
//! `webdav_sync_state` row is claimed with a single conditional write (so a
//! second server instance, or anything else marking the row running, is
//! respected too). The slot and the row are released on every exit path,
//! including a panicking sync task.

use std::collections::HashSet;
use std::future::Future;
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};
use std::time::Instant;

use chrono::Utc;
use futures::stream::{FuturesUnordered, StreamExt};
use tracing::{error, info, warn};
use uuid::Uuid;

use super::webdav_file_processing::{FileProcessor, StopSignal, CONCURRENT_DOWNLOADS};
use crate::{
    db::Database,
    models::{source::UpdateWebDAVSyncState, FileIngestionInfo},
    services::webdav::{
        PendingDirectoryCommit, SmartSyncResult, SmartSyncService, SyncPhase, SyncProgress,
        WebDAVConfig, WebDAVService,
    },
    utils::{extension_filter::ExtensionFilter, outbound::validate_outbound_url},
    AppState,
};

/// How a sync run ended: the number of files ingested, or why it failed.
pub type WebDavSyncResult = Result<usize, Box<dyn std::error::Error + Send + Sync>>;

/// The error a run stopped through the cancel endpoint ends with.
const CANCELLED_MESSAGE: &str = "Sync cancelled by user";

/// Whether a request to start a user's WebDAV sync started one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebDavSyncStart {
    Started,
    /// A sync for the user is running, or a cancelled one is still winding down.
    AlreadyRunning,
}

/// What a settings-level WebDAV sync needs to run.
pub struct WebDavSyncRequest {
    pub user_id: Uuid,
    pub webdav_service: WebDAVService,
    pub config: WebDAVConfig,
    pub enable_background_ocr: bool,
}

/// Users with a settings-level WebDAV sync task alive in this process.
static LIVE_RUNS: LazyLock<Mutex<HashSet<Uuid>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

fn live_runs() -> MutexGuard<'static, HashSet<Uuid>> {
    LIVE_RUNS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// A user's in-process slot; dropping it frees the slot, however the owner
/// ended (return, error or unwinding).
struct LiveRunSlot {
    user_id: Uuid,
}

impl LiveRunSlot {
    fn reserve(user_id: Uuid) -> Option<Self> {
        // The guard is released before a slot exists: a refused reservation
        // must not build (and so drop) a slot, whose `Drop` would free the
        // holder's entry -- and, under the still-held lock, deadlock.
        let inserted = live_runs().insert(user_id);
        inserted.then(|| Self { user_id })
    }
}

impl Drop for LiveRunSlot {
    fn drop(&mut self) {
        live_runs().remove(&self.user_id);
    }
}

/// Whether a settings-level WebDAV sync task for the user is alive in this
/// process (running, or winding down after a cancel).
pub fn is_webdav_sync_live(user_id: Uuid) -> bool {
    live_runs().contains(&user_id)
}

/// Claims the user and runs their WebDAV sync in the background; `on_finished`
/// gets the result once the run has ended and its end has been recorded.
/// Returns [`WebDavSyncStart::AlreadyRunning`] without starting anything while
/// another sync of the user holds the claim.
pub async fn start_webdav_sync<F, Fut>(
    state: Arc<AppState>,
    request: WebDavSyncRequest,
    on_finished: F,
) -> anyhow::Result<WebDavSyncStart>
where
    F: FnOnce(WebDavSyncResult) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let user_id = request.user_id;
    let run = perform_claimed_webdav_sync(state.clone(), request);
    run_claimed_webdav_sync(state, user_id, run, on_finished).await
}

/// The claim protocol of [`start_webdav_sync`] around an arbitrary `run`.
/// `run` must record its own end in `webdav_sync_state`; when it panics or is
/// aborted, the supervisor records the run as failed instead. Only once that
/// is written is the slot released and `on_finished` called, so a new run
/// can never be overwritten by the previous run's final write.
pub async fn run_claimed_webdav_sync<R, F, Fut>(
    state: Arc<AppState>,
    user_id: Uuid,
    run: R,
    on_finished: F,
) -> anyhow::Result<WebDavSyncStart>
where
    R: Future<Output = WebDavSyncResult> + Send + 'static,
    F: FnOnce(WebDavSyncResult) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let Some(slot) = LiveRunSlot::reserve(user_id) else {
        return Ok(WebDavSyncStart::AlreadyRunning);
    };
    if !state.db.claim_webdav_sync(user_id).await? {
        return Ok(WebDavSyncStart::AlreadyRunning);
    }

    let task = tokio::spawn(run);
    let db = state.db.clone();
    tokio::spawn(async move {
        let result = match task.await {
            Ok(result) => result,
            Err(join_error) => {
                let reason = if join_error.is_panic() {
                    "sync task panicked"
                } else {
                    "sync task was aborted"
                };
                error!("WebDAV {} for user {}: {}", reason, user_id, join_error);
                let message = format!("Sync failed: {}: {}", reason, join_error);
                record_run_end(&db, user_id, vec![message.clone()], 0).await;
                Err(message.into())
            }
        };
        drop(slot);
        on_finished(result).await;
    });
    Ok(WebDavSyncStart::Started)
}

/// Writes the final state of a run: not running, with its errors and count.
async fn record_run_end(db: &Database, user_id: Uuid, errors: Vec<String>, files_processed: usize) {
    let final_state = UpdateWebDAVSyncState {
        last_sync_at: Some(Utc::now()),
        sync_cursor: None,
        is_running: false,
        files_processed: files_processed as i64,
        files_remaining: 0,
        current_folder: None,
        errors,
    };
    if let Err(e) = db.update_webdav_sync_state(user_id, &final_state).await {
        error!(
            "Failed to record the end of the WebDAV sync of user {}: {}",
            user_id, e
        );
    }
}

/// The sync itself, for a user whose claim the caller holds. Its end is
/// recorded (awaited, not spawned) before it returns.
async fn perform_claimed_webdav_sync(
    state: Arc<AppState>,
    request: WebDavSyncRequest,
) -> WebDavSyncResult {
    let started = Instant::now();
    let request_id = Uuid::new_v4();
    let user_id = request.user_id;
    info!(
        "[{}] Starting WebDAV sync for user {} from {}: folders {:?}, extensions {:?}",
        request_id,
        user_id,
        request.config.server_url,
        request.config.watch_folders,
        request.config.file_extensions
    );

    // The server address is re-checked at connect time: DNS may have changed
    // since the settings were saved.
    let server_url = WebDAVConfig::normalize_server_url(&request.config.server_url);
    if let Err(e) = validate_outbound_url(&server_url).await {
        warn!(
            "[{}] WebDAV sync for user {} not started: {}",
            request_id, user_id, e
        );
        record_run_end(&state.db, user_id, vec![e.to_string()], 0).await;
        return Err(Box::new(e));
    }

    let result = UserSyncRun::new(state.clone(), request, request_id)
        .run()
        .await;
    record_result(&state.db, user_id, request_id, &result, started).await;
    result
}

/// Logs how the run ended and records its end.
async fn record_result(
    db: &Database,
    user_id: Uuid,
    request_id: Uuid,
    result: &WebDavSyncResult,
    started: Instant,
) {
    let elapsed = started.elapsed().as_secs_f64();
    let (errors, files) = match result {
        Ok(files) => (Vec::new(), *files),
        Err(e) => (vec![format!("Sync failed: {}", e)], 0),
    };
    info!(
        "[{}] WebDAV sync of user {} ended after {:.2}s: {} files processed, errors: {:?}",
        request_id, user_id, elapsed, files, errors
    );
    record_run_end(db, user_id, errors, files).await;
}

/// The state of one settings-level sync run across its watch folders.
struct UserSyncRun {
    state: Arc<AppState>,
    user_id: Uuid,
    request_id: Uuid,
    webdav_service: WebDAVService,
    watch_folders: Vec<String>,
    extensions: ExtensionFilter,
    processor: FileProcessor,
    progress: Arc<SyncProgress>,
    files_processed: usize,
    directories_scanned: usize,
    directories_skipped: usize,
    errors: Vec<String>,
}

/// How a folder's files went.
struct FolderFiles {
    processed: usize,
    failed_files: Vec<String>,
    cancelled: bool,
}

impl UserSyncRun {
    fn new(state: Arc<AppState>, request: WebDavSyncRequest, request_id: Uuid) -> Self {
        // The service holds the watch folders decoded and normalised.
        let watch_folders = request.webdav_service.get_config().watch_folders.clone();
        let processor = FileProcessor::new(
            state.clone(),
            request.user_id,
            request.webdav_service.clone(),
            request.enable_background_ocr,
            None,
            StopSignal::UserSyncState,
        );
        Self {
            state,
            user_id: request.user_id,
            request_id,
            webdav_service: request.webdav_service,
            watch_folders,
            extensions: ExtensionFilter::new(&request.config.file_extensions),
            processor,
            progress: Arc::new(SyncProgress::new()),
            files_processed: 0,
            directories_scanned: 0,
            directories_skipped: 0,
            errors: Vec::new(),
        }
    }

    async fn run(mut self) -> WebDavSyncResult {
        let started = Instant::now();
        self.progress.set_phase(SyncPhase::Initializing);
        let folders = std::mem::take(&mut self.watch_folders);
        for (index, folder) in folders.iter().enumerate() {
            if self.processor.stop_requested().await {
                break;
            }
            info!(
                "[{}] Processing folder {}/{}: '{}'",
                self.request_id,
                index + 1,
                folders.len(),
                folder
            );
            self.write_progress(Some(folder), self.files_processed, 0)
                .await;
            self.sync_folder(folder).await;
        }
        // Checked once more after the last folder: a run cancelled at any
        // point ends as cancelled, never as completed.
        if self.processor.stop_requested().await {
            info!(
                "[{}] WebDAV sync cancelled after {} files",
                self.request_id, self.files_processed
            );
            return Err(CANCELLED_MESSAGE.into());
        }
        self.progress.set_phase(SyncPhase::Completed);
        self.log_summary(started.elapsed().as_secs_f64());
        Ok(self.files_processed)
    }

    async fn sync_folder(&mut self, folder: &str) {
        let started = Instant::now();
        let smart_sync = SmartSyncService::new(self.state.clone());
        let outcome = smart_sync
            .evaluate_and_sync(
                self.user_id,
                None,
                &self.webdav_service,
                folder,
                Some(&self.progress),
            )
            .await;
        let elapsed = started.elapsed().as_secs_f64();
        match outcome {
            Ok(Some(result)) => self.process_folder(folder, result, elapsed).await,
            Ok(None) => info!(
                "[{}] No changes detected for '{}' ({:.2}s)",
                self.request_id, folder, elapsed
            ),
            Err(e) => {
                let message = format!(
                    "Smart sync failed for folder '{}' after {:.2}s: {}",
                    folder, elapsed, e
                );
                error!("[{}] {}", self.request_id, message);
                self.progress.add_error(&message);
                self.errors.push(message);
            }
        }
    }

    async fn process_folder(&mut self, folder: &str, result: SmartSyncResult, scan_seconds: f64) {
        self.directories_scanned += result.directories_scanned;
        self.directories_skipped += result.directories_skipped;
        info!(
            "[{}] Smart sync of '{}': {} files, {} dirs scanned, {} skipped, {:?} in {:.2}s",
            self.request_id,
            folder,
            result.files.len(),
            result.directories_scanned,
            result.directories_skipped,
            result.strategy_used,
            scan_seconds
        );

        // Directories were already handled by the smart sync service.
        let files: Vec<FileIngestionInfo> = result
            .files
            .into_iter()
            .filter(|file| self.extensions.matches_name(&file.name))
            .collect();
        let started = Instant::now();
        let outcome = self.process_files(folder, &files).await;
        self.files_processed += outcome.processed;

        // The folder's directory state is recorded only now that its files
        // are done, and not at all when the run was stopped.
        if outcome.cancelled {
            info!(
                "[{}] Directory state of '{}' not recorded: sync was cancelled",
                self.request_id, folder
            );
        } else {
            PendingDirectoryCommit::commit_all(
                &self.state.db,
                vec![result.directory_commit],
                &outcome.failed_files,
            )
            .await;
        }
        let elapsed = started.elapsed().as_secs_f64();
        info!(
            "[{}] Folder '{}': {} files processed in {:.2}s",
            self.request_id, folder, outcome.processed, elapsed
        );
    }

    /// Processes the folder's files concurrently, checking for a cancel after
    /// each one and recording progress as files complete.
    async fn process_files(&mut self, folder: &str, files: &[FileIngestionInfo]) -> FolderFiles {
        info!(
            "[{}] Processing {} files from '{}' ({} at a time)",
            self.request_id,
            files.len(),
            folder,
            CONCURRENT_DOWNLOADS
        );
        self.progress.set_phase(SyncPhase::ProcessingFiles);
        self.progress.add_files_found(files.len());
        let mut file_futures: FuturesUnordered<_> = files
            .iter()
            .map(|file_info| {
                let processor = self.processor.clone();
                async move {
                    (
                        file_info.relative_path.clone(),
                        processor.process(file_info).await,
                    )
                }
            })
            .collect();

        let mut outcome = FolderFiles {
            processed: 0,
            failed_files: Vec::new(),
            cancelled: false,
        };
        while let Some((file_path, result)) = file_futures.next().await {
            if self.processor.stop_requested().await {
                self.errors
                    .push(format!("{} during file processing", CANCELLED_MESSAGE));
                outcome.cancelled = true;
                break;
            }
            match result {
                Ok(true) => {
                    outcome.processed += 1;
                    self.progress.add_files_processed(1, 0);
                }
                Ok(false) => {}
                Err(error) => {
                    self.file_failed(folder, &error);
                    outcome.failed_files.push(file_path);
                }
            }
            let remaining = files.len() - outcome.processed;
            self.write_progress(
                Some(folder),
                self.files_processed + outcome.processed,
                remaining,
            )
            .await;
        }
        outcome
    }

    fn file_failed(&mut self, folder: &str, error: &str) {
        error!(
            "[{}] File processing error in folder '{}': {}",
            self.request_id, folder, error
        );
        self.progress.add_error(error);
        self.errors.push(format!("Folder '{}': {}", folder, error));
    }

    async fn write_progress(
        &self,
        folder: Option<&str>,
        files_processed: usize,
        files_remaining: usize,
    ) {
        let update = UpdateWebDAVSyncState {
            last_sync_at: Some(Utc::now()),
            sync_cursor: None,
            is_running: true,
            files_processed: files_processed as i64,
            files_remaining: files_remaining as i64,
            current_folder: folder.map(str::to_string),
            errors: self.errors.clone(),
        };
        // Conditional on the row still being marked running: a cancel that
        // landed in between is never undone by a progress write.
        let written = self
            .state
            .db
            .update_running_webdav_sync_progress(self.user_id, &update)
            .await;
        if let Err(e) = written {
            warn!("Failed to update WebDAV sync progress: {}", e);
        }
    }

    fn log_summary(&self, seconds: f64) {
        info!(
            "[{}] WebDAV sync of user {}: {} files, {} dirs scanned, {} skipped, {} errors in {:.2}s",
            self.request_id,
            self.user_id,
            self.files_processed,
            self.directories_scanned,
            self.directories_skipped,
            self.errors.len(),
            seconds
        );
        if !self.errors.is_empty() {
            let recent: Vec<&String> = self.errors.iter().take(3).collect();
            warn!(
                "[{}] Sync completed with {} errors. Recent errors: {:?}",
                self.request_id,
                self.errors.len(),
                recent
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The in-process slot is exclusive per user and freed when its holder is
    /// dropped, including by unwinding.
    #[test]
    fn live_run_slot_is_exclusive_and_released_on_drop_and_panic() {
        let user = Uuid::new_v4();
        let other = Uuid::new_v4();
        let slot = LiveRunSlot::reserve(user).expect("first reservation");
        assert!(
            LiveRunSlot::reserve(user).is_none(),
            "a second reservation must be refused"
        );
        let other_slot = LiveRunSlot::reserve(other).expect("another user is independent");
        assert!(is_webdav_sync_live(user));
        drop(slot);
        assert!(!is_webdav_sync_live(user));

        let unwound = std::panic::catch_unwind(|| {
            let _slot = LiveRunSlot::reserve(user).unwrap();
            panic!("sync exploded");
        });
        assert!(unwound.is_err());
        assert!(
            !is_webdav_sync_live(user),
            "the slot is freed while unwinding"
        );
        assert!(LiveRunSlot::reserve(user).is_some());
        drop(other_slot);
    }
}
