use anyhow::{anyhow, Result};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::{
    db::sources::{SyncCompletion, SYNC_CANCELLED_MESSAGE},
    models::{FileIngestionInfo, Source, SourceStatus, SourceType, User, UserRole},
    services::webdav::{PendingDirectoryCommit, SyncPhase, SyncProgress},
    utils::sync_cancelled::cancelled,
    AppState,
};

#[path = "source_sync_backends.rs"]
mod backends;
#[path = "source_sync_pipeline.rs"]
mod pipeline;
pub use pipeline::PipelineRun;

/// Status message recorded on a source whose owner cannot sync it.
pub const OWNER_INACTIVE_MESSAGE: &str = "Sync skipped: the source owner's account is disabled";

/// Sources sync on behalf of their owner, so only an active owner's sources sync.
fn owner_may_sync(owner: Option<&User>) -> bool {
    owner.is_some_and(|owner| owner.is_active)
}

/// Whether the owner may read local folders outside the configured allowlist
/// (an active admin, and only when no allowlist is configured).
fn owner_is_active_admin(owner: Option<&User>) -> bool {
    owner.is_some_and(|owner| owner.is_active && owner.role == UserRole::Admin)
}

/// What one sync run did, counted per file after the extension filter.
/// `discovered = processed + skipped + failed` once the run has finished; a
/// cancelled run stops early and leaves the remainder pending.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SyncOutcome {
    /// Files that passed the extension filter, including those skipped for
    /// their size.
    pub discovered: usize,
    /// Files ingested as new documents.
    pub processed: usize,
    /// Files not ingested on purpose: already present, duplicate content, or
    /// over the size limit. Includes `unchanged` and `oversized`.
    pub skipped: usize,
    /// Files skipped before download because the stored document has the
    /// same path, size and modification time.
    pub unchanged: usize,
    /// Files skipped before download because they are over `max_file_size_mb`.
    pub oversized: usize,
    /// Files whose download or ingestion failed; they are retried next run.
    pub failed: usize,
    /// Watch folders the run tried to list.
    pub folders: usize,
    /// Watch folders whose listing failed.
    pub folders_failed: usize,
    /// Subdirectories inside an otherwise listed watch folder whose own
    /// listing failed; their files are missing from this run and the
    /// directory is rescanned next time.
    pub directories_failed: usize,
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
        self.failed == 0 && self.folders_failed == 0 && self.directories_failed == 0
    }

    /// Files discovered this run that were neither ingested nor skipped.
    pub fn pending(&self) -> i64 {
        self.discovered
            .saturating_sub(self.processed + self.skipped) as i64
    }

    /// A one-line account of what failed, for the source's `last_error`;
    /// `None` when the run was clean.
    pub fn failure_summary(&self) -> Option<String> {
        if self.is_clean() {
            return None;
        }
        let mut parts = Vec::new();
        if self.folders_failed > 0 {
            parts.push(format!(
                "{} of {} folders could not be listed",
                self.folders_failed, self.folders
            ));
        }
        if self.directories_failed > 0 {
            parts.push(format!(
                "{} subdirectories could not be listed",
                self.directories_failed
            ));
        }
        if self.failed > 0 {
            parts.push(format!(
                "{} of {} files failed",
                self.failed, self.discovered
            ));
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

    /// Counts a listed file that is over the size limit: discovered, and
    /// skipped without being handed to the processor.
    fn skip_oversized(&mut self) {
        self.discovered += 1;
        self.skipped += 1;
        self.oversized += 1;
    }
}

/// What listing one watch folder produced.
///
/// `failed_directories` names subdirectories the source could not list this
/// run (the WebDAV scanner reports them; S3 and local folders never do).
/// Their files are absent from `files`, and the run is reported as not clean
/// so the operator sees it, while the files that were listed still sync.
///
/// `directory_commit` is the directory state a WebDAV scan wants recorded;
/// the pipeline commits it after the folder's files are done, minus what the
/// failed files invalidate, and not at all when the run was cancelled.
#[derive(Debug, Default, Clone)]
pub struct FolderDiscovery {
    pub files: Vec<FileIngestionInfo>,
    pub failed_directories: Vec<String>,
    pub directory_commit: Option<PendingDirectoryCommit>,
}

impl From<Vec<FileIngestionInfo>> for FolderDiscovery {
    fn from(files: Vec<FileIngestionInfo>) -> Self {
        Self {
            files,
            failed_directories: Vec::new(),
            directory_commit: None,
        }
    }
}

/// What processing one discovered file produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileOutcome {
    Ingested,
    Skipped,
    /// Skipped before download: already synced and unchanged.
    Unchanged,
}

#[derive(Clone)]
pub struct SourceSyncService {
    state: Arc<AppState>,
}

impl SourceSyncService {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    /// A complete sync run: claims the source, syncs it and records the
    /// completion. Fails without running when the source is already syncing.
    pub async fn sync_source(
        &self,
        source: &Source,
        enable_background_ocr: bool,
    ) -> Result<SyncOutcome> {
        if !self.state.db.start_sync_atomic(source.id).await? {
            return Err(anyhow!("Sync already running for this source"));
        }
        let progress = Arc::new(SyncProgress::new());
        self.state
            .sync_progress_tracker
            .register_sync(source.id, progress.clone());
        let result = self
            .sync_claimed_source(
                source,
                enable_background_ocr,
                CancellationToken::new(),
                &progress,
            )
            .await;
        self.state.sync_progress_tracker.unregister_sync(source.id);
        result
    }

    /// Syncs a source the caller has already claimed with `start_sync_atomic`
    /// and records how the run ended through `finish_sync`, on every path, so
    /// `last_sync_at` always moves and a failing source is not retried on the
    /// next scheduler tick. The caller owns the registration of `progress`.
    pub async fn sync_claimed_source(
        &self,
        source: &Source,
        enable_background_ocr: bool,
        cancellation_token: CancellationToken,
        progress: &Arc<SyncProgress>,
    ) -> Result<SyncOutcome> {
        info!(
            "Starting sync for source {} ({})",
            source.name, source.source_type
        );
        progress.set_phase(SyncPhase::Initializing);

        let result = self
            .run_source(
                source,
                enable_background_ocr,
                cancellation_token.clone(),
                progress,
            )
            .await;

        match &result {
            Ok(_) => progress.set_phase(SyncPhase::Completed),
            Err(e) => progress.set_phase(SyncPhase::Failed(e.to_string())),
        }
        let cancelled =
            cancellation_token.is_cancelled() || result.as_ref().is_ok_and(|o| o.cancelled);
        self.record_completion(source, &result, cancelled).await;
        result
    }

    async fn run_source(
        &self,
        source: &Source,
        enable_background_ocr: bool,
        cancellation_token: CancellationToken,
        progress: &Arc<SyncProgress>,
    ) -> Result<SyncOutcome> {
        if cancellation_token.is_cancelled() {
            return Err(cancelled("Sync"));
        }

        let owner = self.state.db.get_user_by_id(source.user_id).await?;
        if !owner_may_sync(owner.as_ref()) {
            warn!(
                "Skipping sync for source {} ({}): owner account is disabled",
                source.name, source.id
            );
            return Err(anyhow!(OWNER_INACTIVE_MESSAGE));
        }

        match source.source_type {
            SourceType::WebDAV => {
                self.sync_webdav_source(source, enable_background_ocr, cancellation_token, progress)
                    .await
            }
            SourceType::LocalFolder => {
                self.sync_local_folder_source(
                    source,
                    enable_background_ocr,
                    cancellation_token,
                    progress,
                )
                .await
            }
            SourceType::S3 => {
                self.sync_s3_source(source, enable_background_ocr, cancellation_token, progress)
                    .await
            }
        }
    }

    /// Writes the final status, message and counters for a run. A cancelled
    /// run ends idle with the cancellation message; a run with failures ends
    /// in the error status with the failure summary and its failed files
    /// still counted as pending.
    async fn record_completion(
        &self,
        source: &Source,
        result: &Result<SyncOutcome>,
        cancelled: bool,
    ) {
        let (status, error, files_synced, files_pending) = match result {
            Ok(outcome) if cancelled => {
                info!(
                    "Sync for source {} was cancelled after {} files",
                    source.name, outcome.processed
                );
                (
                    SourceStatus::Idle,
                    Some(SYNC_CANCELLED_MESSAGE.to_string()),
                    outcome.processed,
                    Some(outcome.pending()),
                )
            }
            Err(e) if cancelled => {
                info!("Sync for source {} was cancelled: {}", source.name, e);
                (
                    SourceStatus::Idle,
                    Some(SYNC_CANCELLED_MESSAGE.to_string()),
                    0,
                    None,
                )
            }
            Ok(outcome) => match outcome.failure_summary() {
                None => {
                    info!(
                        "Sync completed for source {}: {} files processed",
                        source.name, outcome.processed
                    );
                    (
                        SourceStatus::Idle,
                        None,
                        outcome.processed,
                        Some(outcome.pending()),
                    )
                }
                Some(summary) => {
                    warn!(
                        "Sync finished with failures for source {}: {}",
                        source.name, summary
                    );
                    (
                        SourceStatus::Error,
                        Some(summary),
                        outcome.processed,
                        Some(outcome.pending()),
                    )
                }
            },
            Err(e) => {
                error!("Sync failed for source {}: {}", source.name, e);
                (SourceStatus::Error, Some(e.to_string()), 0, None)
            }
        };

        let completion = SyncCompletion {
            status,
            error: error.as_deref(),
            files_synced: files_synced as i64,
            files_pending,
        };
        match self.state.db.finish_sync(source.id, &completion).await {
            Ok(true) => {}
            Ok(false) => warn!("Completion of sync for source {} not recorded: the source was no longer marked as syncing", source.id),
            Err(e) => error!("Failed to record sync completion for source {}: {}", source.id, e),
        }
    }
}

#[cfg(test)]
mod owner_tests {
    use super::*;
    use crate::models::AuthProvider;
    use uuid::Uuid;

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
        let outcome = SyncOutcome {
            discovered: 10,
            processed: 4,
            skipped: 3,
            failed: 2,
            ..Default::default()
        };
        assert_eq!(outcome.pending(), 3);
        assert!(!outcome.is_clean());
        assert!(SyncOutcome {
            discovered: 2,
            processed: 2,
            ..Default::default()
        }
        .is_clean());
    }

    /// An oversized file is discovered and skipped, so it does not hide a
    /// pending or failed file behind it.
    #[test]
    fn oversized_files_count_as_discovered_and_skipped() {
        let mut outcome = SyncOutcome::default();
        outcome.skip_oversized();
        outcome.discovered += 2; // two files handed to the processor
        outcome.processed += 1;
        outcome.failed += 1;
        assert_eq!(outcome.oversized, 1);
        assert_eq!(outcome.skipped, 1);
        assert_eq!(
            outcome.discovered,
            outcome.processed + outcome.skipped + outcome.failed
        );
        assert_eq!(outcome.pending(), 1);
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
        assert_eq!(
            SyncOutcome {
                discovered: 2,
                processed: 2,
                ..Default::default()
            }
            .failure_summary(),
            None
        );
        let outcome = SyncOutcome {
            discovered: 120,
            processed: 117,
            failed: 3,
            folders: 2,
            folders_failed: 1,
            first_error: Some("folder /b: 500".into()),
            ..Default::default()
        };
        assert_eq!(
            outcome.failure_summary().as_deref(),
            Some(
                "1 of 2 folders could not be listed; 3 of 120 files failed; first: folder /b: 500"
            )
        );
    }
}
