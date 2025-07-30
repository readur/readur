// Simplified progress tracking shim for backward compatibility
// This provides basic types that do nothing but maintain API compatibility

use std::time::Duration;

/// Simplified progress tracker that just logs
#[derive(Debug, Clone)]
pub struct SyncProgress {
    // Empty struct - all progress tracking is now just logging
}

/// Simplified sync phases for basic logging
#[derive(Debug, Clone, PartialEq)]
pub enum SyncPhase {
    Initializing,
    Evaluating,
    DiscoveringDirectories,
    DiscoveringFiles,
    ProcessingFiles,
    SavingMetadata,
    Completed,
    Failed(String),
    Retrying { attempt: u32, category: String, delay_ms: u64 },
}

/// Empty progress stats for compatibility
#[derive(Debug, Clone)]
pub struct ProgressStats {
    pub phase: SyncPhase,
    pub elapsed_time: Duration,
    pub directories_found: usize,
    pub directories_processed: usize,
    pub files_found: usize,
    pub files_processed: usize,
    pub bytes_processed: u64,
    pub processing_rate: f64,
    pub current_directory: String,
    pub current_file: Option<String>,
    pub errors: Vec<String>,
    pub warnings: usize,
}

impl SyncProgress {
    pub fn new() -> Self {
        Self {}
    }

    pub fn set_phase(&self, _phase: SyncPhase) {
        // Do nothing - progress tracking simplified to basic logging
    }

    pub fn set_current_directory(&self, _directory: &str) {
        // Do nothing - progress tracking simplified to basic logging
    }

    pub fn set_current_file(&self, _file: Option<&str>) {
        // Do nothing - progress tracking simplified to basic logging
    }

    pub fn add_directories_found(&self, _count: usize) {
        // Do nothing - progress tracking simplified to basic logging
    }

    pub fn add_files_found(&self, _count: usize) {
        // Do nothing - progress tracking simplified to basic logging
    }

    pub fn add_files_processed(&self, _count: usize, _bytes: u64) {
        // Do nothing - progress tracking simplified to basic logging
    }

    pub fn add_error(&self, _error: &str) {
        // Do nothing - progress tracking simplified to basic logging
    }

    pub fn add_warning(&self) {
        // Do nothing - progress tracking simplified to basic logging
    }

    pub fn update_files_processed(&self, _count: usize) {
        // Do nothing - progress tracking simplified to basic logging
    }

    pub fn update_files_found(&self, _count: usize) {
        // Do nothing - progress tracking simplified to basic logging
    }

    pub fn get_stats(&self) -> Option<ProgressStats> {
        // Return dummy stats for compatibility
        Some(ProgressStats {
            phase: SyncPhase::Completed,
            elapsed_time: Duration::from_secs(0),
            directories_found: 0,
            directories_processed: 0,
            files_found: 0,
            files_processed: 0,
            bytes_processed: 0,
            processing_rate: 0.0,
            current_directory: String::new(),
            current_file: None,
            errors: Vec::new(),
            warnings: 0,
        })
    }
}

impl ProgressStats {
    pub fn files_progress_percent(&self) -> f64 {
        0.0 // Simplified - no real progress tracking
    }

    pub fn estimated_time_remaining(&self) -> Option<Duration> {
        None // Simplified - no real progress tracking
    }
}

impl Default for SyncProgress {
    fn default() -> Self {
        Self::new()
    }
}