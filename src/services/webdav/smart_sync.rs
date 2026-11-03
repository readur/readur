use std::sync::Arc;
use std::collections::HashMap;
use std::time::Instant;
use anyhow::Result;
use tracing::{debug, info, warn};
use uuid::Uuid;
use tokio_util::sync::CancellationToken;

use crate::{AppState, models::{FileIngestionInfo}};
use crate::models::source::{CreateWebDAVDirectory};
use crate::models::source_error::{ErrorSourceType, ErrorContext};
use crate::webdav_xml_parser::compare_etags;
use crate::services::source_error_tracker::SourceErrorTracker;
use super::{WebDAVService, SyncProgress};
use super::paths::normalize_watch_folder;
use super::service::{DiscoveryTracking, WebDAVScanResult};

/// Normalize a path by removing trailing slashes for consistent comparison
/// This handles inconsistencies where WebDAV servers may return paths with or without trailing slashes
fn normalize_path(path: &str) -> String {
    path.trim_end_matches('/').to_string()
}

/// Row key under which a watch folder's own ETag is stored: the watch folder
/// in the same form as the href-derived directory paths.
fn root_row_path(folder_path: &str) -> String {
    normalize_watch_folder(folder_path)
}

/// True when `path` is one of `failed` or lies anywhere beneath one of them.
fn is_within_failed(path: &str, failed: &[String]) -> bool {
    let path = normalize_path(path);
    failed.iter().any(|f| {
        let f = normalize_path(f);
        path == f || path.starts_with(&format!("{}/", f))
    })
}

/// Check if a path is an immediate child of a parent path
/// This is used to filter directory comparisons to only check immediate children,
/// not deeply nested directories during shallow discovery
fn is_immediate_child(parent_path: &str, child_path: &str) -> bool {
    let parent = normalize_path(parent_path);
    let child = normalize_path(child_path);

    // Handle root path special case: "/" normalizes to ""
    // For root, immediate children are paths like "/Documents" (one segment after root)
    if parent.is_empty() {
        // Child should start with "/" and have exactly one path segment
        if !child.starts_with('/') {
            return false;
        }
        // Remove the leading slash and check for no more slashes
        let remaining = &child[1..];
        return !remaining.is_empty() && !remaining.contains('/');
    }

    // Child path must be longer than parent
    if child.len() <= parent.len() {
        return false;
    }

    // Child must start with parent followed by a path separator
    // This prevents "/Documents" from matching "/DocumentsBackup"
    let expected_prefix = format!("{}/", parent);

    if !child.starts_with(&expected_prefix) {
        return false;
    }

    // Get the remaining path after parent (excluding the separator)
    let remaining = &child[expected_prefix.len()..];

    // If remaining is empty, this is not a valid child
    if remaining.is_empty() {
        return false;
    }

    // Immediate child has no additional slashes in the remaining path
    !remaining.contains('/')
}

/// Smart sync service that provides intelligent WebDAV synchronization
/// by comparing directory ETags to avoid unnecessary scans
#[derive(Clone)]
pub struct SmartSyncService {
    state: Arc<AppState>,
    error_tracker: SourceErrorTracker,
    cancellation: Option<CancellationToken>,
}

/// Result of smart sync evaluation
#[derive(Debug, Clone)]
pub enum SmartSyncDecision {
    /// No changes detected, sync can be skipped entirely
    SkipSync,
    /// Smart sync detected changes, need to perform discovery
    RequiresSync(SmartSyncStrategy),
}

/// Strategy for performing sync after smart evaluation
#[derive(Debug, Clone)]
pub enum SmartSyncStrategy {
    /// Full deep scan needed (first time, too many changes, or fallback)
    FullDeepScan,
    /// Targeted scan of specific changed directories
    TargetedScan(Vec<String>),
}

/// Outcome of evaluating a folder, keeping the shallow root listing when the
/// folder itself changed so the files directly inside it can be returned.
struct Evaluation {
    decision: SmartSyncDecision,
    root_listing: Option<WebDAVScanResult>,
    root_changed: bool,
}

impl Evaluation {
    fn deep_scan() -> Self {
        Self { decision: SmartSyncDecision::RequiresSync(SmartSyncStrategy::FullDeepScan), root_listing: None, root_changed: false }
    }
}

/// Complete result from smart sync operation
#[derive(Debug, Clone)]
pub struct SmartSyncResult {
    pub files: Vec<FileIngestionInfo>,
    pub directories: Vec<FileIngestionInfo>,
    pub strategy_used: SmartSyncStrategy,
    pub directories_scanned: usize,
    pub directories_skipped: usize,
    /// Directories whose listing failed; their subtrees were not scanned and
    /// their stored state was left untouched.
    pub failed_directories: Vec<String>,
}

/// What a folder sync produced, including whether any part of it was skipped
/// because a directory could not be listed. `strategy` is `None` when the
/// evaluation found nothing to do.
#[derive(Debug, Clone)]
pub struct SmartSyncOutcome {
    pub files: Vec<FileIngestionInfo>,
    pub failed_directories: Vec<String>,
    pub strategy: Option<SmartSyncStrategy>,
}

impl SmartSyncService {
    pub fn new(state: Arc<AppState>) -> Self {
        let error_tracker = SourceErrorTracker::new(state.db.clone());
        Self { state, error_tracker, cancellation: None }
    }

    /// Makes evaluation and scanning stop (with an error) once `token` is cancelled.
    pub fn with_cancellation_token(mut self, token: CancellationToken) -> Self {
        self.cancellation = Some(token);
        self
    }

    fn check_cancelled(&self) -> Result<()> {
        match &self.cancellation {
            Some(token) if token.is_cancelled() => Err(anyhow::anyhow!("Smart sync cancelled")),
            _ => Ok(()),
        }
    }

    /// Get access to the application state (primarily for testing)
    pub fn state(&self) -> &Arc<AppState> {
        &self.state
    }
    
    /// Get sync metrics (simplified version without loop detection)
    pub async fn get_sync_metrics(&self, _webdav_service: &WebDAVService) -> Result<serde_json::Value> {
        Ok(serde_json::json!({
            "loop_detection_removed": true,
            "message": "Loop detection has been replaced with database-based failure tracking"
        }))
    }

    /// Evaluates whether sync is needed and determines the best strategy
    pub async fn evaluate_sync_need(
        &self,
        user_id: Uuid,
        webdav_service: &WebDAVService,
        folder_path: &str,
        _progress: Option<&SyncProgress>, // Simplified: no complex progress tracking
    ) -> Result<SmartSyncDecision> {
        Ok(self.evaluate(user_id, webdav_service, folder_path).await?.decision)
    }

    /// Compares the folder's own ETag and its immediate subdirectories' ETags
    /// with the stored ones to decide what, if anything, needs scanning.
    async fn evaluate(
        &self,
        user_id: Uuid,
        webdav_service: &WebDAVService,
        folder_path: &str,
    ) -> Result<Evaluation> {
        self.check_cancelled()?;
        let folder_path = &normalize_watch_folder(folder_path);
        let evaluation_start = Instant::now();
        let eval_request_id = Uuid::new_v4();
        
        info!("[{}] 🧠 Evaluating smart sync for folder: '{}'", eval_request_id, folder_path);
        
        // Get all known directory ETags from database in bulk
        let known_directories = self.state.db.list_webdav_directories(user_id).await
            .map_err(|e| anyhow::anyhow!("Failed to fetch known directories: {}", e))?;
        
        // Filter to only directories under the current folder path
        // Use normalized paths (without trailing slashes) for consistent comparison
        let normalized_folder_path = normalize_path(folder_path);
        let relevant_dirs: HashMap<String, String> = known_directories
            .into_iter()
            .filter(|dir| normalize_path(&dir.directory_path).starts_with(&normalized_folder_path))
            .map(|dir| (normalize_path(&dir.directory_path), dir.directory_etag))
            .collect();
        
        if relevant_dirs.is_empty() {
            let eval_elapsed = evaluation_start.elapsed();
            info!("[{}] No known directories for '{}', requires full deep scan (evaluated in {:.2}s)", 
                  eval_request_id, folder_path, eval_elapsed.as_secs_f64());
            return Ok(Evaluation::deep_scan());
        }
        
        info!("[{}] Found {} known directories for smart sync comparison", 
              eval_request_id, relevant_dirs.len());
        
        // Do a shallow discovery of the root folder to check immediate changes
        match webdav_service.discover_tree(folder_path, false, None).await {
            Ok(root_discovery) => {
                let mut changed_directories = Vec::new();
                let mut new_directories = Vec::new();

                // The folder's own ETag covers the files that live directly in it.
                let root_changed = match (relevant_dirs.get(&normalized_folder_path), &root_discovery.root) {
                    (Some(known_etag), Some(root)) => !compare_etags(known_etag, &root.etag),
                    (None, Some(_)) => true,
                    (_, None) => false,
                };
                if root_changed {
                    info!("[{}] 🔄 Folder '{}' itself changed; files directly inside it will be listed",
                          eval_request_id, folder_path);
                }
                
                // Check if any immediate subdirectories have changed ETags
                for directory in &root_discovery.directories {
                    // Use normalized path for HashMap lookup to handle trailing slash differences
                    let normalized_dir_path = normalize_path(&directory.relative_path);
                    match relevant_dirs.get(&normalized_dir_path) {
                        Some(known_etag) => {
                            // Use proper ETag comparison that handles weak/strong semantics
                            if !compare_etags(known_etag, &directory.etag) {
                                info!("[{}] 🔄 Directory changed: '{}' (old: {}, new: {})",
                                      eval_request_id, directory.relative_path, known_etag, directory.etag);
                                changed_directories.push(normalized_dir_path);
                            } else {
                                debug!("[{}] ✅ Directory unchanged: '{}' (ETag: {})",
                                       eval_request_id, directory.relative_path, directory.etag);
                            }
                        }
                        None => {
                            info!("[{}] ✨ New directory discovered: '{}'",
                                  eval_request_id, directory.relative_path);
                            new_directories.push(normalized_dir_path);
                        }
                    }
                }

                // Check for deleted directories (directories that were known but not discovered)
                // IMPORTANT: Only check immediate children of folder_path, not nested directories
                // Shallow discovery only returns immediate children, so checking nested directories
                // would falsely flag them as deleted
                let discovered_paths: std::collections::HashSet<String> = root_discovery.directories
                    .iter()
                    .map(|d| normalize_path(&d.relative_path))
                    .collect();

                // Count immediate children for logging
                let immediate_children_count = relevant_dirs
                    .keys()
                    .filter(|p| is_immediate_child(&normalized_folder_path, p))
                    .count();

                debug!("[{}] Checking {} immediate children for deletions (filtered from {} total known directories)",
                       eval_request_id, immediate_children_count, relevant_dirs.len());

                let mut deleted_directories = Vec::new();
                for (known_path, _) in &relevant_dirs {
                    // Only check deletion for immediate children of the folder being scanned
                    // Nested directories (e.g., /folder/sub/deep/) won't appear in shallow discovery
                    // and should not be flagged as deleted
                    if is_immediate_child(&normalized_folder_path, known_path) {
                        if !discovered_paths.contains(known_path.as_str()) {
                            info!("[{}] 🗑️ Directory deleted: '{}'", eval_request_id, known_path);
                            deleted_directories.push(known_path.clone());
                        }
                    }
                }

                // If directories were deleted, we need to clean them up
                if !deleted_directories.is_empty() {
                    info!("[{}] Found {} deleted directories that need cleanup: {:?}",
                          eval_request_id, deleted_directories.len(),
                          deleted_directories.iter().take(3).collect::<Vec<_>>());
                    // We'll handle deletion in the sync operation itself
                }
                
                // If no changes detected and no deletions, we can skip
                if changed_directories.is_empty() && new_directories.is_empty() && deleted_directories.is_empty() && !root_changed {
                    let eval_elapsed = evaluation_start.elapsed();
                    info!("[{}] ✅ Smart sync: No directory changes detected for '{}', sync can be skipped (evaluated in {:.2}s)", 
                          eval_request_id, folder_path, eval_elapsed.as_secs_f64());
                    return Ok(Evaluation { decision: SmartSyncDecision::SkipSync, root_listing: None, root_changed: false });
                }
                
                // Determine strategy based on scope of changes
                let total_changes = changed_directories.len() + new_directories.len() + deleted_directories.len();
                let total_known = relevant_dirs.len();
                let change_ratio = total_changes as f64 / total_known.max(1) as f64;
                
                let eval_elapsed = evaluation_start.elapsed();
                if change_ratio > 0.3 || new_directories.len() > 5 || !deleted_directories.is_empty() {
                    // Too many changes or deletions detected, do full deep scan for efficiency
                    info!("[{}] 📁 Smart sync: Large changes detected for '{}' ({} changed, {} new, {} deleted, {:.1}% change ratio), using full deep scan (evaluated in {:.2}s)", 
                          eval_request_id, folder_path, changed_directories.len(), new_directories.len(), 
                          deleted_directories.len(), change_ratio * 100.0, eval_elapsed.as_secs_f64());
                    return Ok(Evaluation::deep_scan());
                } else {
                    // Targeted scan of changed directories
                    let mut targets = changed_directories;
                    targets.extend(new_directories);
                    info!("[{}] 🎯 Smart sync: Targeted changes detected for '{}', scanning {} directories: {:?} (evaluated in {:.2}s)", 
                          eval_request_id, folder_path, targets.len(), 
                          targets.iter().take(3).collect::<Vec<_>>(), eval_elapsed.as_secs_f64());
                    return Ok(Evaluation {
                        decision: SmartSyncDecision::RequiresSync(SmartSyncStrategy::TargetedScan(targets)),
                        root_listing: Some(root_discovery),
                        root_changed,
                    });
                }
            }
            Err(e) => {
                let eval_elapsed = evaluation_start.elapsed();
                warn!("[{}] Smart sync evaluation failed for '{}' after {:.2}s, falling back to deep scan: {}", 
                      eval_request_id, folder_path, eval_elapsed.as_secs_f64(), e);
                
                // Track the error using the generic error tracker
                let context = ErrorContext {
                    resource_path: folder_path.to_string(),
                    source_id: None,
                    operation: "evaluate_sync_need".to_string(),
                    response_time: None,
                    response_size: None,
                    server_type: None,
                    server_version: None,
                    additional_context: std::collections::HashMap::new(),
                };
                
                if let Err(track_error) = self.error_tracker.track_error(
                    user_id,
                    ErrorSourceType::WebDAV,
                    None, // source_id - we don't have a specific source ID for this operation
                    folder_path,
                    &e,
                    context,
                ).await {
                    warn!("Failed to track sync evaluation error: {}", track_error);
                }
                
                let final_elapsed = evaluation_start.elapsed();
                info!("[{}] Fallback decision: Full deep scan for '{}' (total evaluation time: {:.2}s)", 
                      eval_request_id, folder_path, final_elapsed.as_secs_f64());
                return Ok(Evaluation::deep_scan());
            }
        }
    }

    /// Performs smart sync based on the strategy determined by evaluation
    pub async fn perform_smart_sync(
        &self,
        user_id: Uuid,
        source_id: Option<Uuid>,
        webdav_service: &WebDAVService,
        folder_path: &str,
        strategy: SmartSyncStrategy,
        _progress: Option<&SyncProgress>, // Simplified: no complex progress tracking
    ) -> Result<SmartSyncResult> {
        self.check_cancelled()?;
        let folder_path = &normalize_watch_folder(folder_path);
        let sync_request_id = Uuid::new_v4();
        match strategy {
            SmartSyncStrategy::FullDeepScan => {
                info!("[{}] 🔍 Performing full deep scan for: '{}'", sync_request_id, folder_path);
                self.perform_full_deep_scan(user_id, source_id, webdav_service, folder_path, _progress, sync_request_id).await
            }
            SmartSyncStrategy::TargetedScan(target_dirs) => {
                info!("[{}] 🎯 Performing targeted scan of {} directories: {:?}", 
                      sync_request_id, target_dirs.len(), 
                      target_dirs.iter().take(3).collect::<Vec<_>>());
                self.perform_targeted_scan(user_id, source_id, webdav_service, target_dirs, _progress, sync_request_id).await
            }
        }
    }

    /// Combined evaluation and execution for convenience
    pub async fn evaluate_and_sync(
        &self,
        user_id: Uuid,
        source_id: Option<Uuid>,
        webdav_service: &WebDAVService,
        folder_path: &str,
        _progress: Option<&SyncProgress>, // Simplified: no complex progress tracking
    ) -> Result<Option<SmartSyncResult>> {
        let folder_path = &normalize_watch_folder(folder_path);
        let eval_and_sync_start = Instant::now();
        let eval_sync_request_id = Uuid::new_v4();

        let evaluation = self.evaluate(user_id, webdav_service, folder_path).await?;
        match evaluation.decision {
            SmartSyncDecision::SkipSync => {
                let total_elapsed = eval_and_sync_start.elapsed();
                info!("[{}] ✅ Smart sync: Skipping sync for '{}' - no changes detected (completed in {:.2}s)",
                      eval_sync_request_id, folder_path, total_elapsed.as_secs_f64());
                Ok(None)
            }
            SmartSyncDecision::RequiresSync(strategy) => {
                let mut result = self.perform_smart_sync(user_id, source_id, webdav_service, folder_path, strategy, _progress).await?;
                if evaluation.root_changed {
                    if let Some(listing) = evaluation.root_listing {
                        self.absorb_root_listing(user_id, folder_path, listing, &mut result).await;
                    }
                }
                let total_elapsed = eval_and_sync_start.elapsed();
                info!("[{}] ✅ Smart sync completed for '{}' - {} files found, {} dirs scanned, {} dirs failed in {:.2}s",
                      eval_sync_request_id, folder_path, result.files.len(),
                      result.directories_scanned, result.failed_directories.len(), total_elapsed.as_secs_f64());
                Ok(Some(result))
            }
        }
    }

    /// Like `evaluate_and_sync`, but also reports the directories whose listing
    /// failed so the caller can treat the sync as incomplete rather than successful.
    pub async fn evaluate_and_sync_detailed(
        &self,
        user_id: Uuid,
        source_id: Option<Uuid>,
        webdav_service: &WebDAVService,
        folder_path: &str,
        progress: Option<&SyncProgress>,
    ) -> Result<SmartSyncOutcome> {
        Ok(match self.evaluate_and_sync(user_id, source_id, webdav_service, folder_path, progress).await? {
            None => SmartSyncOutcome { files: Vec::new(), failed_directories: Vec::new(), strategy: None },
            Some(result) => SmartSyncOutcome {
                files: result.files,
                failed_directories: result.failed_directories,
                strategy: Some(result.strategy_used),
            },
        })
    }

    /// Adds the files listed directly in the folder to `result` and records the
    /// folder's own ETag so they are not listed again until it changes.
    async fn absorb_root_listing(&self, user_id: Uuid, folder_path: &str, listing: WebDAVScanResult, result: &mut SmartSyncResult) {
        info!("Folder '{}' changed: adding {} files listed directly in it", folder_path, listing.files.len());
        result.files.extend(listing.files);
        if let Some(root) = listing.root {
            let row = Self::root_row(user_id, folder_path, &root.etag);
            if let Err(e) = self.state.db.create_or_update_webdav_directory(&row).await {
                warn!("Failed to save ETag for folder '{}': {}", folder_path, e);
            }
        }
    }

    fn root_row(user_id: Uuid, folder_path: &str, etag: &str) -> CreateWebDAVDirectory {
        CreateWebDAVDirectory {
            user_id,
            directory_path: root_row_path(folder_path),
            directory_etag: etag.to_string(),
            file_count: 0,
            total_size_bytes: 0,
        }
    }

    fn tracking(&self, user_id: Uuid, source_id: Option<Uuid>) -> DiscoveryTracking {
        DiscoveryTracking { user_id, source_id, tracker: self.error_tracker.clone() }
    }

    /// Directory rows to persist from a scan: every listed directory except those
    /// inside a subtree that could not be fully listed, whose stored state must
    /// stay as it was so they are rescanned next time.
    fn directories_to_persist(user_id: Uuid, scan: &WebDAVScanResult) -> Vec<CreateWebDAVDirectory> {
        scan.directories
            .iter()
            .filter(|d| !is_within_failed(&d.relative_path, &scan.failed_directories))
            .map(|d| CreateWebDAVDirectory {
                user_id,
                directory_path: d.relative_path.clone(),
                directory_etag: d.etag.clone(),
                file_count: 0, // Will be updated by stats
                total_size_bytes: 0, // Will be updated by stats
            })
            .collect()
    }

    async fn save_directories_individually(&self, directories: &[CreateWebDAVDirectory]) {
        let mut saved = 0;
        for directory in directories {
            match self.state.db.create_or_update_webdav_directory(directory).await {
                Ok(_) => saved += 1,
                Err(e) => warn!("Failed to save directory ETag for {}: {}", directory.directory_path, e),
            }
        }
        info!("Fallback: Saved ETags for {}/{} directories", saved, directories.len());
    }

    /// Performs a full deep scan and saves all directory ETags
    async fn perform_full_deep_scan(
        &self,
        user_id: Uuid,
        source_id: Option<Uuid>,
        webdav_service: &WebDAVService,
        folder_path: &str,
        _progress: Option<&SyncProgress>, // Simplified: no complex progress tracking
        _request_id: Uuid,
    ) -> Result<SmartSyncResult> {
        let tracking = self.tracking(user_id, source_id);
        let scan = webdav_service.discover_tree(folder_path, true, Some(&tracking)).await?;
        info!("Deep scan found {} files and {} directories in folder {} ({} directories failed)",
              scan.files.len(), scan.directories.len(), folder_path, scan.failed_directories.len());

        let mut directories_to_save = Self::directories_to_persist(user_id, &scan);
        if let Some(root) = &scan.root {
            directories_to_save.push(Self::root_row(user_id, folder_path, &root.etag));
        }
        let complete = scan.failed_directories.is_empty();
        if !complete {
            warn!("{} directories under '{}' could not be listed; their stored state is kept and nothing is pruned: {:?}",
                  scan.failed_directories.len(), folder_path, scan.failed_directories.iter().take(5).collect::<Vec<_>>());
        }

        match self.state.db.sync_webdav_directories(user_id, folder_path, &directories_to_save, complete).await {
            Ok((saved_directories, deleted_count)) => {
                info!("✅ Atomic sync completed: {} directories updated/created, {} deleted",
                      saved_directories.len(), deleted_count);
            }
            Err(e) => {
                warn!("Failed to perform atomic directory sync: {}", e);
                self.save_directories_individually(&directories_to_save).await;
            }
        }

        let directories_scanned = scan.directories.len();
        Ok(SmartSyncResult {
            files: scan.files,
            directories: scan.directories,
            strategy_used: SmartSyncStrategy::FullDeepScan,
            directories_scanned,
            directories_skipped: 0,
            failed_directories: scan.failed_directories,
        })
    }

    /// Performs targeted scans of specific directories
    async fn perform_targeted_scan(
        &self,
        user_id: Uuid,
        source_id: Option<Uuid>,
        webdav_service: &WebDAVService,
        target_directories: Vec<String>,
        _progress: Option<&SyncProgress>, // Simplified: no complex progress tracking
        request_id: Uuid,
    ) -> Result<SmartSyncResult> {
        let tracking = self.tracking(user_id, source_id);
        let mut all_files = Vec::new();
        let mut all_directories = Vec::new();
        let mut failed_directories = Vec::new();
        let mut directories_scanned = 0;

        for target_dir in &target_directories {
            self.check_cancelled()?;
            info!("Scanning target directory: {}", target_dir);
            let scan = match webdav_service.discover_tree(target_dir, true, Some(&tracking)).await {
                Ok(scan) => scan,
                Err(e) => {
                    warn!("Failed to scan target directory {}: {}", target_dir, e);
                    failed_directories.push(target_dir.clone());
                    continue;
                }
            };

            let directories_to_save = Self::directories_to_persist(user_id, &scan);
            if !directories_to_save.is_empty() {
                match self.state.db.bulk_create_or_update_webdav_directories(&directories_to_save).await {
                    Ok(saved) => debug!("Bulk updated {} directory ETags for target scan", saved.len()),
                    Err(e) => {
                        warn!("Failed bulk update for target scan, falling back to individual saves: {}", e);
                        self.save_directories_individually(&directories_to_save).await;
                    }
                }
            }

            all_files.extend(scan.files);
            all_directories.extend(scan.directories);
            failed_directories.extend(scan.failed_directories);
            directories_scanned += 1;
        }

        info!("[{}] ✅ Targeted scan completed: {} directories scanned, {} files found, {} directories failed",
              request_id, directories_scanned, all_files.len(), failed_directories.len());

        Ok(SmartSyncResult {
            files: all_files,
            directories: all_directories,
            strategy_used: SmartSyncStrategy::TargetedScan(target_directories),
            directories_scanned,
            directories_skipped: 0, // TODO: Could track this if needed
            failed_directories,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path_removes_trailing_slash() {
        assert_eq!(normalize_path("/Documents/"), "/Documents");
        assert_eq!(normalize_path("/Documents"), "/Documents");
        assert_eq!(normalize_path("/"), "");
        assert_eq!(normalize_path(""), "");
    }

    #[test]
    fn test_normalize_path_handles_multiple_trailing_slashes() {
        assert_eq!(normalize_path("/Documents///"), "/Documents");
        assert_eq!(normalize_path("/path/to/folder//"), "/path/to/folder");
    }

    #[test]
    fn test_is_immediate_child_basic() {
        // Basic immediate children
        assert!(is_immediate_child("/Documents", "/Documents/Projects"));
        assert!(is_immediate_child("/Documents", "/Documents/Archive"));
        assert!(is_immediate_child("/", "/Documents"));
    }

    #[test]
    fn test_is_immediate_child_with_trailing_slashes() {
        // Should work regardless of trailing slashes
        assert!(is_immediate_child("/Documents/", "/Documents/Projects/"));
        assert!(is_immediate_child("/Documents", "/Documents/Projects/"));
        assert!(is_immediate_child("/Documents/", "/Documents/Projects"));
    }

    #[test]
    fn test_is_immediate_child_nested_not_immediate() {
        // Nested directories should NOT be immediate children
        assert!(!is_immediate_child("/Documents", "/Documents/Projects/Work"));
        assert!(!is_immediate_child("/Documents", "/Documents/Projects/Work/Deep"));
        assert!(!is_immediate_child("/", "/Documents/Projects"));
    }

    #[test]
    fn test_is_immediate_child_same_path() {
        // Same path is not a child of itself
        assert!(!is_immediate_child("/Documents", "/Documents"));
        assert!(!is_immediate_child("/Documents/", "/Documents/"));
        assert!(!is_immediate_child("/Documents", "/Documents/"));
    }

    #[test]
    fn test_is_immediate_child_unrelated_paths() {
        // Unrelated paths
        assert!(!is_immediate_child("/Documents", "/Photos/Album"));
        assert!(!is_immediate_child("/Documents", "/Other"));
    }

    #[test]
    fn test_is_immediate_child_similar_names() {
        // Similar but different paths (DocumentsBackup is not under Documents)
        assert!(!is_immediate_child("/Documents", "/DocumentsBackup"));
        assert!(!is_immediate_child("/Documents", "/DocumentsBackup/Folder"));
    }

    #[test]
    fn test_root_row_path_is_normalized() {
        assert_eq!(root_row_path("/Documents/"), "/Documents");
        assert_eq!(root_row_path("/Documents"), "/Documents");
        assert_eq!(root_row_path("Documents"), "/Documents");
        // Decoding happened when the service was built; rows are keyed by what it passes on.
        assert_eq!(root_row_path("/My Docs/"), "/My Docs");
        assert_eq!(root_row_path("/a%2520b"), "/a%2520b");
        assert_eq!(root_row_path("/"), "/");
        assert_eq!(root_row_path(""), "/");
    }

    #[test]
    fn test_is_within_failed_matches_self_and_descendants_only() {
        let failed = vec!["/Docs/Broken/".to_string()];
        assert!(is_within_failed("/Docs/Broken", &failed));
        assert!(is_within_failed("/Docs/Broken/", &failed));
        assert!(is_within_failed("/Docs/Broken/deep/er/", &failed));
        assert!(!is_within_failed("/Docs/BrokenToo", &failed));
        assert!(!is_within_failed("/Docs", &failed));
        assert!(!is_within_failed("/Docs/Ok/x", &failed));
        assert!(!is_within_failed("/Docs/Ok/x", &[]));
    }

    #[test]
    fn test_deletion_detection_scenario() {
        // Simulate the bug scenario:
        // - Parent folder: /Documents
        // - Known directories in DB: /Documents/Projects, /Documents/Projects/Work, /Documents/Archive
        // - Shallow discovery returns only: /Documents/Projects, /Documents/Archive
        //
        // The bug was: /Documents/Projects/Work was flagged as deleted
        // The fix: Only immediate children should be checked for deletion

        let parent = "/Documents";
        let known_dirs = vec![
            "/Documents/Projects",
            "/Documents/Projects/Work",        // nested - should NOT be checked
            "/Documents/Projects/Work/Deep",   // deeply nested - should NOT be checked
            "/Documents/Archive",
        ];

        // Only immediate children should be considered for deletion detection
        let immediate_children: Vec<_> = known_dirs
            .iter()
            .filter(|p| is_immediate_child(parent, p))
            .collect();

        assert_eq!(immediate_children.len(), 2);
        assert!(immediate_children.contains(&&"/Documents/Projects"));
        assert!(immediate_children.contains(&&"/Documents/Archive"));

        // Nested directories should NOT be in the list
        assert!(!immediate_children.contains(&&"/Documents/Projects/Work"));
        assert!(!immediate_children.contains(&&"/Documents/Projects/Work/Deep"));
    }
}