use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use std::time::Duration;
use serde::{Serialize, Deserialize};

use crate::services::webdav::{SyncProgress, ProgressStats, SyncPhase};

/// Global service for tracking active sync operations
#[derive(Debug, Clone)]
pub struct SyncProgressTracker {
    inner: Arc<Mutex<SyncProgressTrackerInner>>,
}

#[derive(Debug)]
struct SyncProgressTrackerInner {
    /// Maps source_id to active sync progress
    active_syncs: HashMap<Uuid, Arc<SyncProgress>>,
    /// Maps source_id to last known progress stats (for recently completed syncs)
    recent_stats: HashMap<Uuid, ProgressStats>,
}

/// Serializable progress information for API responses
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SyncProgressInfo {
    pub source_id: Uuid,
    pub phase: String,
    pub phase_description: String,
    pub elapsed_time_secs: u64,
    pub directories_found: usize,
    pub directories_processed: usize,
    pub files_found: usize,
    pub files_processed: usize,
    pub bytes_processed: u64,
    pub processing_rate_files_per_sec: f64,
    pub files_progress_percent: f64,
    pub estimated_time_remaining_secs: Option<u64>,
    pub current_directory: String,
    pub current_file: Option<String>,
    pub errors: usize,
    pub warnings: usize,
    pub is_active: bool,
}

impl SyncProgressTracker {
    /// Create a new progress tracker
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(SyncProgressTrackerInner {
                active_syncs: HashMap::new(),
                recent_stats: HashMap::new(),
            })),
        }
    }

    /// Register a new active sync
    pub fn register_sync(&self, source_id: Uuid, progress: Arc<SyncProgress>) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.active_syncs.insert(source_id, progress);
            // Remove from recent stats since it's now active
            inner.recent_stats.remove(&source_id);
        }
    }

    /// Unregister a completed sync and store final stats
    pub fn unregister_sync(&self, source_id: Uuid) {
        if let Ok(mut inner) = self.inner.lock() {
            if let Some(progress) = inner.active_syncs.remove(&source_id) {
                // Store final stats for recent access
                if let Some(stats) = progress.get_stats() {
                    inner.recent_stats.insert(source_id, stats);
                }
            }
        }
    }

    /// Get progress information for a specific source
    pub fn get_progress(&self, source_id: Uuid) -> Option<SyncProgressInfo> {
        if let Ok(inner) = self.inner.lock() {
            // Check if there's an active sync
            if let Some(progress) = inner.active_syncs.get(&source_id) {
                if let Some(stats) = progress.get_stats() {
                    return Some(Self::stats_to_info(source_id, stats, true));
                }
            }
            
            // Check recent stats for completed syncs
            if let Some(stats) = inner.recent_stats.get(&source_id) {
                return Some(Self::stats_to_info(source_id, stats.clone(), false));
            }
        }
        None
    }

    /// Get progress information for all active syncs
    pub fn get_all_active_progress(&self) -> Vec<SyncProgressInfo> {
        if let Ok(inner) = self.inner.lock() {
            inner.active_syncs
                .iter()
                .filter_map(|(source_id, progress)| {
                    progress.get_stats()
                        .map(|stats| Self::stats_to_info(*source_id, stats, true))
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Check if a source is currently syncing
    pub fn is_syncing(&self, source_id: Uuid) -> bool {
        if let Ok(inner) = self.inner.lock() {
            inner.active_syncs.contains_key(&source_id)
        } else {
            false
        }
    }

    /// Get list of all source IDs with active syncs
    pub fn get_active_source_ids(&self) -> Vec<Uuid> {
        if let Ok(inner) = self.inner.lock() {
            inner.active_syncs.keys().copied().collect()
        } else {
            Vec::new()
        }
    }

    /// Convert ProgressStats to SyncProgressInfo
    fn stats_to_info(source_id: Uuid, stats: ProgressStats, is_active: bool) -> SyncProgressInfo {
        let (phase_name, phase_description) = Self::phase_to_strings(&stats.phase);
        
        SyncProgressInfo {
            source_id,
            phase: phase_name,
            phase_description,
            elapsed_time_secs: stats.elapsed_time.as_secs(),
            directories_found: stats.directories_found,
            directories_processed: stats.directories_processed,
            files_found: stats.files_found,
            files_processed: stats.files_processed,
            bytes_processed: stats.bytes_processed,
            processing_rate_files_per_sec: stats.processing_rate,
            files_progress_percent: stats.files_progress_percent(),
            estimated_time_remaining_secs: stats.estimated_time_remaining()
                .map(|d| d.as_secs()),
            current_directory: stats.current_directory,
            current_file: stats.current_file,
            errors: stats.errors.len(),
            warnings: stats.warnings,
            is_active,
        }
    }

    /// Convert SyncPhase to human-readable strings
    fn phase_to_strings(phase: &SyncPhase) -> (String, String) {
        match phase {
            SyncPhase::Initializing => (
                "initializing".to_string(),
                "Initializing sync operation".to_string(),
            ),
            SyncPhase::Evaluating => (
                "evaluating".to_string(),
                "Evaluating what needs to be synced".to_string(),
            ),
            SyncPhase::DiscoveringDirectories => (
                "discovering_directories".to_string(),
                "Discovering directories and folder structure".to_string(),
            ),
            SyncPhase::DiscoveringFiles => (
                "discovering_files".to_string(),
                "Discovering files to sync".to_string(),
            ),
            SyncPhase::ProcessingFiles => (
                "processing_files".to_string(),
                "Downloading and processing files".to_string(),
            ),
            SyncPhase::SavingMetadata => (
                "saving_metadata".to_string(),
                "Saving metadata and finalizing sync".to_string(),
            ),
            SyncPhase::Completed => (
                "completed".to_string(),
                "Sync completed successfully".to_string(),
            ),
            SyncPhase::Failed(error) => (
                "failed".to_string(),
                format!("Sync failed: {}", error),
            ),
            SyncPhase::Retrying { attempt, category, delay_ms } => (
                "retrying".to_string(),
                format!("Retry attempt {} for {:?} (delay: {}ms)", attempt, category, delay_ms),
            ),
        }
    }
}

impl Default for SyncProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}