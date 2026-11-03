//! Progress of one sync run, shared between the task that performs it and
//! the readers that report it (the progress tracker, the WebSocket feed).
//! Counters are atomics; the few strings sit behind a mutex that is never
//! held across an await.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// How many error messages a run keeps; the count keeps growing past this.
const MAX_RECORDED_ERRORS: usize = 100;

#[derive(Debug)]
pub struct SyncProgress {
    started: Instant,
    phase: Mutex<SyncPhase>,
    directories_found: AtomicUsize,
    directories_processed: AtomicUsize,
    files_found: AtomicUsize,
    files_processed: AtomicUsize,
    bytes_processed: AtomicU64,
    current_directory: Mutex<String>,
    current_file: Mutex<Option<String>>,
    errors: Mutex<Vec<String>>,
    error_count: AtomicUsize,
    warnings: AtomicUsize,
}

/// Sync phases
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

/// A snapshot of a run's progress.
#[derive(Debug, Clone)]
pub struct ProgressStats {
    pub phase: SyncPhase,
    pub elapsed_time: Duration,
    pub directories_found: usize,
    pub directories_processed: usize,
    pub files_found: usize,
    pub files_processed: usize,
    pub bytes_processed: u64,
    /// Files processed per second since the run started.
    pub processing_rate: f64,
    pub current_directory: String,
    pub current_file: Option<String>,
    /// The first [`MAX_RECORDED_ERRORS`] error messages.
    pub errors: Vec<String>,
    pub warnings: usize,
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl SyncProgress {
    pub fn new() -> Self {
        Self {
            started: Instant::now(),
            phase: Mutex::new(SyncPhase::Initializing),
            directories_found: AtomicUsize::new(0),
            directories_processed: AtomicUsize::new(0),
            files_found: AtomicUsize::new(0),
            files_processed: AtomicUsize::new(0),
            bytes_processed: AtomicU64::new(0),
            current_directory: Mutex::new(String::new()),
            current_file: Mutex::new(None),
            errors: Mutex::new(Vec::new()),
            error_count: AtomicUsize::new(0),
            warnings: AtomicUsize::new(0),
        }
    }

    pub fn set_phase(&self, phase: SyncPhase) {
        *lock(&self.phase) = phase;
    }

    pub fn set_current_directory(&self, directory: &str) {
        *lock(&self.current_directory) = directory.to_string();
        self.directories_processed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn set_current_file(&self, file: Option<&str>) {
        *lock(&self.current_file) = file.map(str::to_string);
    }

    pub fn add_directories_found(&self, count: usize) {
        self.directories_found.fetch_add(count, Ordering::Relaxed);
    }

    pub fn add_files_found(&self, count: usize) {
        self.files_found.fetch_add(count, Ordering::Relaxed);
    }

    pub fn add_files_processed(&self, count: usize, bytes: u64) {
        self.files_processed.fetch_add(count, Ordering::Relaxed);
        self.bytes_processed.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn add_error(&self, error: &str) {
        self.error_count.fetch_add(1, Ordering::Relaxed);
        let mut errors = lock(&self.errors);
        if errors.len() < MAX_RECORDED_ERRORS {
            errors.push(error.to_string());
        }
    }

    pub fn add_warning(&self) {
        self.warnings.fetch_add(1, Ordering::Relaxed);
    }

    /// Sets the processed count to an absolute value.
    pub fn update_files_processed(&self, count: usize) {
        self.files_processed.store(count, Ordering::Relaxed);
    }

    /// Sets the found count to an absolute value.
    pub fn update_files_found(&self, count: usize) {
        self.files_found.store(count, Ordering::Relaxed);
    }

    /// Number of errors recorded, including those past the kept messages.
    pub fn error_count(&self) -> usize {
        self.error_count.load(Ordering::Relaxed)
    }

    pub fn get_stats(&self) -> Option<ProgressStats> {
        let elapsed_time = self.started.elapsed();
        let files_processed = self.files_processed.load(Ordering::Relaxed);
        // Over whole seconds, so that readers taking snapshots a few
        // microseconds apart see the same figures.
        let processing_rate = match elapsed_time.as_secs() {
            0 => 0.0,
            secs => files_processed as f64 / secs as f64,
        };
        Some(ProgressStats {
            phase: lock(&self.phase).clone(),
            elapsed_time,
            directories_found: self.directories_found.load(Ordering::Relaxed),
            directories_processed: self.directories_processed.load(Ordering::Relaxed),
            files_found: self.files_found.load(Ordering::Relaxed),
            files_processed,
            bytes_processed: self.bytes_processed.load(Ordering::Relaxed),
            processing_rate,
            current_directory: lock(&self.current_directory).clone(),
            current_file: lock(&self.current_file).clone(),
            errors: lock(&self.errors).clone(),
            warnings: self.warnings.load(Ordering::Relaxed),
        })
    }
}

impl ProgressStats {
    /// Share of the found files that have been processed, 0 to 100.
    pub fn files_progress_percent(&self) -> f64 {
        if self.files_found == 0 {
            return 0.0;
        }
        (self.files_processed as f64 / self.files_found as f64 * 100.0).min(100.0)
    }

    /// Time to process the remaining files at the current rate, once at
    /// least one file has been processed.
    pub fn estimated_time_remaining(&self) -> Option<Duration> {
        if self.files_processed == 0 || self.processing_rate <= 0.0 {
            return None;
        }
        let remaining = self.files_found.saturating_sub(self.files_processed);
        Some(Duration::from_secs_f64(remaining as f64 / self.processing_rate))
    }
}

impl Default for SyncProgress {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_add_up_and_report_percent() {
        let progress = SyncProgress::new();
        progress.set_phase(SyncPhase::ProcessingFiles);
        progress.add_files_found(4);
        progress.add_files_processed(1, 10);
        progress.add_files_processed(2, 5);
        progress.add_error("one");
        progress.add_warning();
        progress.set_current_directory("/a");
        progress.set_current_file(Some("/a/b.txt"));

        let stats = progress.get_stats().unwrap();
        assert_eq!(stats.phase, SyncPhase::ProcessingFiles);
        assert_eq!(stats.files_found, 4);
        assert_eq!(stats.files_processed, 3);
        assert_eq!(stats.bytes_processed, 15);
        assert_eq!(stats.errors, vec!["one".to_string()]);
        assert_eq!(stats.warnings, 1);
        assert_eq!(stats.directories_processed, 1);
        assert_eq!(stats.current_directory, "/a");
        assert_eq!(stats.current_file.as_deref(), Some("/a/b.txt"));
        assert_eq!(stats.files_progress_percent(), 75.0);
    }

    #[test]
    fn remaining_time_follows_the_rate() {
        let stats = ProgressStats {
            phase: SyncPhase::ProcessingFiles,
            elapsed_time: Duration::from_secs(10),
            directories_found: 0,
            directories_processed: 0,
            files_found: 30,
            files_processed: 10,
            bytes_processed: 0,
            processing_rate: 1.0,
            current_directory: String::new(),
            current_file: None,
            errors: Vec::new(),
            warnings: 0,
        };
        assert_eq!(stats.estimated_time_remaining(), Some(Duration::from_secs(20)));
    }

    #[test]
    fn empty_run_reports_zero_without_dividing_by_zero() {
        let stats = SyncProgress::new().get_stats().unwrap();
        assert_eq!(stats.files_progress_percent(), 0.0);
        assert_eq!(stats.estimated_time_remaining(), None);
    }

    #[test]
    fn errors_past_the_cap_are_counted_but_not_kept() {
        let progress = SyncProgress::new();
        for i in 0..(MAX_RECORDED_ERRORS + 5) {
            progress.add_error(&format!("error {i}"));
        }
        assert_eq!(progress.error_count(), MAX_RECORDED_ERRORS + 5);
        assert_eq!(progress.get_stats().unwrap().errors.len(), MAX_RECORDED_ERRORS);
    }
}
