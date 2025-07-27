use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{info, warn};

/// Thread-safe progress tracking for WebDAV sync operations
#[derive(Debug, Clone)]
pub struct SyncProgress {
    inner: Arc<Mutex<SyncProgressInner>>,
}

#[derive(Debug)]
struct SyncProgressInner {
    start_time: Instant,
    last_update: Instant,
    last_status_report: Instant,
    
    // Discovery phase
    directories_found: usize,
    files_found: usize,
    
    // Processing phase
    directories_processed: usize,
    files_processed: usize,
    bytes_processed: u64,
    
    // Current state
    current_directory: String,
    current_file: Option<String>,
    current_phase: SyncPhase,
    
    // Performance tracking
    processing_rate_files_per_sec: f64,
    
    // Error tracking
    errors: Vec<String>,
    warnings: usize,
    
    // Configuration
    update_interval: Duration,
    status_report_interval: Duration,
}

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
}

impl SyncProgress {
    /// Create a new progress tracker
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            inner: Arc::new(Mutex::new(SyncProgressInner {
                start_time: now,
                last_update: now,
                last_status_report: now,
                directories_found: 0,
                files_found: 0,
                directories_processed: 0,
                files_processed: 0,
                bytes_processed: 0,
                current_directory: String::new(),
                current_file: None,
                current_phase: SyncPhase::Initializing,
                processing_rate_files_per_sec: 0.0,
                errors: Vec::new(),
                warnings: 0,
                update_interval: Duration::from_secs(10),
                status_report_interval: Duration::from_secs(60),
            })),
        }
    }

    /// Set the current sync phase
    pub fn set_phase(&self, phase: SyncPhase) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.current_phase = phase.clone();
            match phase {
                SyncPhase::Evaluating => {
                    info!("🧠 Smart sync: Evaluating directory changes...");
                }
                SyncPhase::DiscoveringDirectories => {
                    info!("🔍 Discovering directories...");
                }
                SyncPhase::DiscoveringFiles => {
                    info!("🔍 Discovering files...");
                }
                SyncPhase::ProcessingFiles => {
                    info!("📁 Processing files...");
                }
                SyncPhase::SavingMetadata => {
                    info!("💾 Saving directory metadata...");
                }
                SyncPhase::Completed => {
                    self.log_completion_summary();
                }
                SyncPhase::Failed(ref error) => {
                    warn!("❌ Sync failed: {}", error);
                }
                _ => {}
            }
        }
    }

    /// Set the current directory being processed
    pub fn set_current_directory(&self, directory: &str) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.current_directory = directory.to_string();
            inner.current_file = None;
            
            // Check if we should log an update
            self.maybe_log_progress(&mut inner);
        }
    }

    /// Set the current file being processed
    pub fn set_current_file(&self, file: Option<&str>) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.current_file = file.map(|f| f.to_string());
            
            // Check if we should log an update
            self.maybe_log_progress(&mut inner);
        }
    }

    /// Increment directory count (discovered or processed)
    pub fn add_directories_found(&self, count: usize) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.directories_found += count;
            self.maybe_log_progress(&mut inner);
        }
    }

    /// Increment processed directory count
    pub fn add_directories_processed(&self, count: usize) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.directories_processed += count;
            self.maybe_log_progress(&mut inner);
        }
    }

    /// Increment file count (discovered or processed)
    pub fn add_files_found(&self, count: usize) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.files_found += count;
            self.maybe_log_progress(&mut inner);
        }
    }

    /// Increment processed file count
    pub fn add_files_processed(&self, count: usize, bytes: u64) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.files_processed += count;
            inner.bytes_processed += bytes;
            
            // Update processing rate
            let elapsed = inner.start_time.elapsed().as_secs_f64();
            if elapsed > 0.0 {
                inner.processing_rate_files_per_sec = inner.files_processed as f64 / elapsed;
            }
            
            self.maybe_log_progress(&mut inner);
        }
    }

    /// Add an error message
    pub fn add_error(&self, error: &str) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.errors.push(error.to_string());
            warn!("🚨 Sync error: {}", error);
        }
    }

    /// Add a warning
    pub fn add_warning(&self, warning: &str) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.warnings += 1;
            warn!("⚠️ Sync warning: {}", warning);
        }
    }

    /// Force a progress update (useful for important milestones)
    pub fn force_update(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            self.log_progress_now(&mut inner);
        }
    }

    /// Force a status report (detailed progress summary)
    pub fn force_status_report(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            self.log_status_report(&mut inner);
        }
    }

    /// Get current progress statistics
    pub fn get_stats(&self) -> Option<ProgressStats> {
        self.inner.lock().ok().map(|inner| ProgressStats {
            elapsed_time: inner.start_time.elapsed(),
            phase: inner.current_phase.clone(),
            directories_found: inner.directories_found,
            directories_processed: inner.directories_processed,
            files_found: inner.files_found,
            files_processed: inner.files_processed,
            bytes_processed: inner.bytes_processed,
            processing_rate: inner.processing_rate_files_per_sec,
            errors: inner.errors.len(),
            warnings: inner.warnings,
            current_directory: inner.current_directory.clone(),
            current_file: inner.current_file.clone(),
        })
    }

    /// Check if we should log progress and do it if needed
    fn maybe_log_progress(&self, inner: &mut SyncProgressInner) {
        let now = Instant::now();
        
        // Regular progress updates
        if now.duration_since(inner.last_update) >= inner.update_interval {
            self.log_progress_now(inner);
        }
        
        // Status reports (more detailed)
        if now.duration_since(inner.last_status_report) >= inner.status_report_interval {
            self.log_status_report(inner);
        }
    }

    /// Log progress immediately
    fn log_progress_now(&self, inner: &mut SyncProgressInner) {
        let elapsed = inner.start_time.elapsed();
        let elapsed_secs = elapsed.as_secs();
        
        match inner.current_phase {
            SyncPhase::DiscoveringDirectories | SyncPhase::DiscoveringFiles => {
                if !inner.current_directory.is_empty() {
                    info!(
                        "📊 Discovery Progress: {} dirs, {} files found | 📁 Current: {} | ⏱️ {}m {}s",
                        inner.directories_found,
                        inner.files_found,
                        inner.current_directory,
                        elapsed_secs / 60,
                        elapsed_secs % 60
                    );
                }
            }
            SyncPhase::ProcessingFiles => {
                let progress_pct = if inner.files_found > 0 {
                    (inner.files_processed as f64 / inner.files_found as f64 * 100.0) as u32
                } else {
                    0
                };
                
                let rate_str = if inner.processing_rate_files_per_sec > 0.0 {
                    format!(" | 🔄 {:.1} files/sec", inner.processing_rate_files_per_sec)
                } else {
                    String::new()
                };
                
                let current_file_str = inner.current_file
                    .as_ref()
                    .map(|f| format!(" | 📄 {}", f))
                    .unwrap_or_default();
                
                info!(
                    "📊 Processing: {}/{} files ({}%){}{} | ⏱️ {}m {}s",
                    inner.files_processed,
                    inner.files_found,
                    progress_pct,
                    rate_str,
                    current_file_str,
                    elapsed_secs / 60,
                    elapsed_secs % 60
                );
            }
            _ => {
                if !inner.current_directory.is_empty() {
                    info!(
                        "📊 Sync Progress | 📁 Current: {} | ⏱️ {}m {}s",
                        inner.current_directory,
                        elapsed_secs / 60,
                        elapsed_secs % 60
                    );
                }
            }
        }
        
        inner.last_update = Instant::now();
    }

    /// Log detailed status report
    fn log_status_report(&self, inner: &mut SyncProgressInner) {
        let elapsed = inner.start_time.elapsed();
        let elapsed_secs = elapsed.as_secs();
        
        let rate_str = if inner.processing_rate_files_per_sec > 0.0 {
            format!(" | Rate: {:.1} files/sec", inner.processing_rate_files_per_sec)
        } else {
            String::new()
        };
        
        let size_mb = inner.bytes_processed as f64 / (1024.0 * 1024.0);
        
        let eta_str = if inner.processing_rate_files_per_sec > 0.0 && inner.files_found > inner.files_processed {
            let remaining_files = inner.files_found - inner.files_processed;
            let eta_secs = (remaining_files as f64 / inner.processing_rate_files_per_sec) as u64;
            format!(" | Est. remaining: {}m {}s", eta_secs / 60, eta_secs % 60)
        } else {
            String::new()
        };
        
        info!(
            "📊 Status Report ({}m {}s elapsed):\n\
             📁 Directories: {} found, {} processed\n\
             📄 Files: {} found, {} processed\n\
             💾 Data: {:.1} MB processed{}{}\n\
             ⚠️ Issues: {} errors, {} warnings",
            elapsed_secs / 60,
            elapsed_secs % 60,
            inner.directories_found,
            inner.directories_processed,
            inner.files_found,
            inner.files_processed,
            size_mb,
            rate_str,
            eta_str,
            inner.errors.len(),
            inner.warnings
        );
        
        inner.last_status_report = Instant::now();
    }

    /// Log completion summary
    fn log_completion_summary(&self) {
        if let Ok(inner) = self.inner.lock() {
            let elapsed = inner.start_time.elapsed();
            let elapsed_secs = elapsed.as_secs();
            let size_mb = inner.bytes_processed as f64 / (1024.0 * 1024.0);
            
            let avg_rate = if elapsed.as_secs_f64() > 0.0 {
                inner.files_processed as f64 / elapsed.as_secs_f64()
            } else {
                0.0
            };
            
            info!(
                "✅ Sync Complete!\n\
                 📊 Summary:\n\
                 📁 Directories: {} processed\n\
                 📄 Files: {} processed\n\
                 💾 Data: {:.1} MB\n\
                 ⏱️ Duration: {}m {}s\n\
                 🔄 Avg rate: {:.1} files/sec\n\
                 ⚠️ Issues: {} errors, {} warnings",
                inner.directories_processed,
                inner.files_processed,
                size_mb,
                elapsed_secs / 60,
                elapsed_secs % 60,
                avg_rate,
                inner.errors.len(),
                inner.warnings
            );
            
            if !inner.errors.is_empty() {
                warn!("🚨 Errors encountered during sync:");
                for (i, error) in inner.errors.iter().enumerate() {
                    warn!("  {}. {}", i + 1, error);
                }
            }
        }
    }
}

impl Default for SyncProgress {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of progress statistics
#[derive(Debug, Clone)]
pub struct ProgressStats {
    pub elapsed_time: Duration,
    pub phase: SyncPhase,
    pub directories_found: usize,
    pub directories_processed: usize,
    pub files_found: usize,
    pub files_processed: usize,
    pub bytes_processed: u64,
    pub processing_rate: f64,
    pub errors: usize,
    pub warnings: usize,
    pub current_directory: String,
    pub current_file: Option<String>,
}

impl ProgressStats {
    /// Get progress percentage for files (0-100)
    pub fn files_progress_percent(&self) -> f64 {
        if self.files_found > 0 {
            (self.files_processed as f64 / self.files_found as f64) * 100.0
        } else {
            0.0
        }
    }
    
    /// Get estimated time remaining in seconds
    pub fn estimated_time_remaining(&self) -> Option<Duration> {
        if self.processing_rate > 0.0 && self.files_found > self.files_processed {
            let remaining_files = self.files_found - self.files_processed;
            let eta_secs = (remaining_files as f64 / self.processing_rate) as u64;
            Some(Duration::from_secs(eta_secs))
        } else {
            None
        }
    }
    
    /// Get human-readable data size processed
    pub fn data_size_mb(&self) -> f64 {
        self.bytes_processed as f64 / (1024.0 * 1024.0)
    }
}