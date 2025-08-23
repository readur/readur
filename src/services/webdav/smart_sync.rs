use std::sync::Arc;
use std::collections::HashMap;
use std::time::Instant;
use anyhow::Result;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::{AppState, models::{FileIngestionInfo}};
use crate::models::source::{CreateWebDAVDirectory};
use crate::models::source_error::{ErrorSourceType, ErrorContext};
use crate::webdav_xml_parser::compare_etags;
use crate::services::source_error_tracker::SourceErrorTracker;
use super::{WebDAVService, SyncProgress};

/// Smart sync service that provides intelligent WebDAV synchronization
/// by comparing directory ETags to avoid unnecessary scans
#[derive(Clone)]
pub struct SmartSyncService {
    state: Arc<AppState>,
    error_tracker: SourceErrorTracker,
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

/// Complete result from smart sync operation
#[derive(Debug, Clone)]
pub struct SmartSyncResult {
    pub files: Vec<FileIngestionInfo>,
    pub directories: Vec<FileIngestionInfo>,
    pub strategy_used: SmartSyncStrategy,
    pub directories_scanned: usize,
    pub directories_skipped: usize,
}

impl SmartSyncService {
    pub fn new(state: Arc<AppState>) -> Self {
        let error_tracker = SourceErrorTracker::new(state.db.clone());
        Self { state, error_tracker }
    }

    /// Get access to the application state (primarily for testing)
    pub fn state(&self) -> &Arc<AppState> {
        &self.state
    }

    /// Evaluates whether sync is needed and determines the best strategy
    pub async fn evaluate_sync_need(
        &self,
        user_id: Uuid,
        webdav_service: &WebDAVService,
        folder_path: &str,
        _progress: Option<&SyncProgress>, // Simplified: no complex progress tracking
    ) -> Result<SmartSyncDecision> {
        let evaluation_start = Instant::now();
        let eval_request_id = Uuid::new_v4();
        
        info!("[{}] 🧠 Evaluating smart sync for folder: '{}'", eval_request_id, folder_path);
        
        // Get all known directory ETags from database in bulk
        let known_directories = self.state.db.list_webdav_directories(user_id).await
            .map_err(|e| anyhow::anyhow!("Failed to fetch known directories: {}", e))?;
        
        // Filter to only directories under the current folder path
        let relevant_dirs: HashMap<String, String> = known_directories
            .into_iter()
            .filter(|dir| dir.directory_path.starts_with(folder_path))
            .map(|dir| (dir.directory_path, dir.directory_etag))
            .collect();
        
        if relevant_dirs.is_empty() {
            let eval_elapsed = evaluation_start.elapsed();
            info!("[{}] No known directories for '{}', requires full deep scan (evaluated in {:.2}s)", 
                  eval_request_id, folder_path, eval_elapsed.as_secs_f64());
            return Ok(SmartSyncDecision::RequiresSync(SmartSyncStrategy::FullDeepScan));
        }
        
        info!("[{}] Found {} known directories for smart sync comparison", 
              eval_request_id, relevant_dirs.len());
        
        // Do a shallow discovery of the root folder to check immediate changes
        match webdav_service.discover_files_and_directories(folder_path, false).await {
            Ok(root_discovery) => {
                let mut changed_directories = Vec::new();
                let mut new_directories = Vec::new();
                
                // Check if any immediate subdirectories have changed ETags
                for directory in &root_discovery.directories {
                    match relevant_dirs.get(&directory.relative_path) {
                        Some(known_etag) => {
                            // Use proper ETag comparison that handles weak/strong semantics
                            if !compare_etags(known_etag, &directory.etag) {
                                info!("[{}] 🔄 Directory changed: '{}' (old: {}, new: {})", 
                                      eval_request_id, directory.relative_path, known_etag, directory.etag);
                                changed_directories.push(directory.relative_path.clone());
                            } else {
                                debug!("[{}] ✅ Directory unchanged: '{}' (ETag: {})", 
                                       eval_request_id, directory.relative_path, directory.etag);
                            }
                        }
                        None => {
                            info!("[{}] ✨ New directory discovered: '{}'", 
                                  eval_request_id, directory.relative_path);
                            new_directories.push(directory.relative_path.clone());
                        }
                    }
                }

                // Check for deleted directories (directories that were known but not discovered)
                let discovered_paths: std::collections::HashSet<String> = root_discovery.directories
                    .iter()
                    .map(|d| d.relative_path.clone())
                    .collect();
                
                let mut deleted_directories = Vec::new();
                for (known_path, _) in &relevant_dirs {
                    if !discovered_paths.contains(known_path.as_str()) {
                        info!("[{}] 🗑️ Directory deleted: '{}'", eval_request_id, known_path);
                        deleted_directories.push(known_path.clone());
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
                if changed_directories.is_empty() && new_directories.is_empty() && deleted_directories.is_empty() {
                    let eval_elapsed = evaluation_start.elapsed();
                    info!("[{}] ✅ Smart sync: No directory changes detected for '{}', sync can be skipped (evaluated in {:.2}s)", 
                          eval_request_id, folder_path, eval_elapsed.as_secs_f64());
                    return Ok(SmartSyncDecision::SkipSync);
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
                    return Ok(SmartSyncDecision::RequiresSync(SmartSyncStrategy::FullDeepScan));
                } else {
                    // Targeted scan of changed directories
                    let mut targets = changed_directories;
                    targets.extend(new_directories);
                    info!("[{}] 🎯 Smart sync: Targeted changes detected for '{}', scanning {} directories: {:?} (evaluated in {:.2}s)", 
                          eval_request_id, folder_path, targets.len(), 
                          targets.iter().take(3).collect::<Vec<_>>(), eval_elapsed.as_secs_f64());
                    return Ok(SmartSyncDecision::RequiresSync(SmartSyncStrategy::TargetedScan(targets)));
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
                return Ok(SmartSyncDecision::RequiresSync(SmartSyncStrategy::FullDeepScan));
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
        let eval_and_sync_start = Instant::now();
        let eval_sync_request_id = Uuid::new_v4();
        
        match self.evaluate_sync_need(user_id, webdav_service, folder_path, _progress).await? {
            SmartSyncDecision::SkipSync => {
                let total_elapsed = eval_and_sync_start.elapsed();
                info!("[{}] ✅ Smart sync: Skipping sync for '{}' - no changes detected (completed in {:.2}s)", 
                      eval_sync_request_id, folder_path, total_elapsed.as_secs_f64());
                Ok(None)
            }
            SmartSyncDecision::RequiresSync(strategy) => {
                let result = self.perform_smart_sync(user_id, source_id, webdav_service, folder_path, strategy, _progress).await?;
                let total_elapsed = eval_and_sync_start.elapsed();
                info!("[{}] ✅ Smart sync completed for '{}' - {} files found, {} dirs scanned in {:.2}s", 
                      eval_sync_request_id, folder_path, result.files.len(), 
                      result.directories_scanned, total_elapsed.as_secs_f64());
                Ok(Some(result))
            }
        }
    }

    /// Performs a full deep scan and saves all directory ETags
    async fn perform_full_deep_scan(
        &self,
        user_id: Uuid,
        source_id: Option<Uuid>,
        webdav_service: &WebDAVService,
        folder_path: &str,
        _progress: Option<&SyncProgress>, // Simplified: no complex progress tracking
        request_id: Uuid,
    ) -> Result<SmartSyncResult> {
        // Use the enhanced discovery method with error tracking from WebDAVService
        let discovery_result = webdav_service.discover_files_and_directories_with_error_tracking(
            folder_path, 
            true, // recursive
            user_id, 
            &self.error_tracker, 
            source_id, // Pass the actual source_id
        ).await?;
        
        info!("Deep scan found {} files and {} directories in folder {}", 
              discovery_result.files.len(), discovery_result.directories.len(), folder_path);
        
        // Simplified: basic logging instead of complex progress tracking
        info!("Saving metadata for scan results");
        
        // Save all discovered directories atomically using bulk operations
        let directories_to_save: Vec<CreateWebDAVDirectory> = discovery_result.directories
            .iter()
            .map(|directory_info| CreateWebDAVDirectory {
                user_id,
                directory_path: directory_info.relative_path.clone(),
                directory_etag: directory_info.etag.clone(),
                file_count: 0, // Will be updated by stats
                total_size_bytes: 0, // Will be updated by stats
            })
            .collect();

        match self.state.db.sync_webdav_directories(user_id, &directories_to_save).await {
            Ok((saved_directories, deleted_count)) => {
                info!("✅ Atomic sync completed: {} directories updated/created, {} deleted", 
                      saved_directories.len(), deleted_count);
                
                if deleted_count > 0 {
                    info!("🗑️ Cleaned up {} orphaned directory records", deleted_count);
                }
            }
            Err(e) => {
                warn!("Failed to perform atomic directory sync: {}", e);
                // Fallback to individual saves if atomic operation fails
                let mut directories_saved = 0;
                for directory_info in &discovery_result.directories {
                    let webdav_directory = CreateWebDAVDirectory {
                        user_id,
                        directory_path: directory_info.relative_path.clone(),
                        directory_etag: directory_info.etag.clone(),
                        file_count: 0,
                        total_size_bytes: 0,
                    };
                    
                    if let Ok(_) = self.state.db.create_or_update_webdav_directory(&webdav_directory).await {
                        directories_saved += 1;
                    }
                }
                info!("Fallback: Saved ETags for {}/{} directories", directories_saved, discovery_result.directories.len());
            }
        }
        
        Ok(SmartSyncResult {
            files: discovery_result.files,
            directories: discovery_result.directories.clone(),
            strategy_used: SmartSyncStrategy::FullDeepScan,
            directories_scanned: discovery_result.directories.len(),
            directories_skipped: 0,
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
        let mut all_files = Vec::new();
        let mut all_directories = Vec::new();
        let mut directories_scanned = 0;

        // Scan each target directory recursively
        for target_dir in &target_directories {
            // Simplified: basic logging instead of complex progress tracking
            info!("Scanning target directory: {}", target_dir);
            
            // Use the enhanced discovery method with error tracking from WebDAVService
            match webdav_service.discover_files_and_directories_with_error_tracking(
                target_dir, 
                true, // recursive
                user_id, 
                &self.error_tracker, 
                source_id, // Pass the actual source_id
            ).await {
                Ok(discovery_result) => {
                    all_files.extend(discovery_result.files);
                    
                    // Collect directory info for bulk update later
                    let directories_to_save: Vec<CreateWebDAVDirectory> = discovery_result.directories
                        .iter()
                        .map(|directory_info| CreateWebDAVDirectory {
                            user_id,
                            directory_path: directory_info.relative_path.clone(),
                            directory_etag: directory_info.etag.clone(),
                            file_count: 0,
                            total_size_bytes: 0,
                        })
                        .collect();

                    // Save directories using bulk operation
                    if !directories_to_save.is_empty() {
                        match self.state.db.bulk_create_or_update_webdav_directories(&directories_to_save).await {
                            Ok(saved_directories) => {
                                debug!("Bulk updated {} directory ETags for target scan", saved_directories.len());
                            }
                            Err(e) => {
                                warn!("Failed bulk update for target scan, falling back to individual saves: {}", e);
                                // Fallback to individual saves
                                for directory_info in &discovery_result.directories {
                                    let webdav_directory = CreateWebDAVDirectory {
                                        user_id,
                                        directory_path: directory_info.relative_path.clone(),
                                        directory_etag: directory_info.etag.clone(),
                                        file_count: 0,
                                        total_size_bytes: 0,
                                    };
                                    
                                    if let Err(e) = self.state.db.create_or_update_webdav_directory(&webdav_directory).await {
                                        warn!("Failed to save directory ETag for {}: {}", directory_info.relative_path, e);
                                    }
                                }
                            }
                        }
                    }
                    
                    all_directories.extend(discovery_result.directories);
                    directories_scanned += 1;
                    
                    // Error tracking for success/failure is already handled by the enhanced discovery method
                }
                Err(e) => {
                    warn!("Failed to scan target directory {}: {} (error tracking handled by WebDAVService)", target_dir, e);
                    // Error tracking is already handled by the enhanced discovery method
                }
            }
        }

        info!("[{}] ✅ Targeted scan completed: {} directories scanned, {} files found", 
              request_id, directories_scanned, all_files.len());

        Ok(SmartSyncResult {
            files: all_files,
            directories: all_directories,
            strategy_used: SmartSyncStrategy::TargetedScan(target_directories),
            directories_scanned,
            directories_skipped: 0, // TODO: Could track this if needed
        })
    }
}