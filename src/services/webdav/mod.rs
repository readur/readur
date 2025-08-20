// Simplified WebDAV service modules - consolidated architecture

pub mod config;
pub mod service; 
pub mod smart_sync;
pub mod progress_shim; // Backward compatibility shim for simplified progress tracking
pub mod error_classifier; // WebDAV error classification for generic error tracking
pub mod metrics_integration; // WebDAV metrics collection integration

// Re-export main types for convenience
pub use config::{WebDAVConfig, RetryConfig, ConcurrencyConfig};
pub use service::{
    WebDAVService, WebDAVDiscoveryResult, WebDAVDownloadResult, ServerCapabilities, HealthStatus, test_webdav_connection,
    ValidationReport, ValidationIssue, ValidationIssueType, ValidationSeverity, 
    ValidationRecommendation, ValidationAction, ValidationSummary
};
pub use smart_sync::{SmartSyncService, SmartSyncDecision, SmartSyncStrategy, SmartSyncResult};
pub use metrics_integration::{WebDAVServiceWithMetrics, SyncWithMetrics};

// Backward compatibility exports for progress tracking (simplified)
pub use progress_shim::{SyncProgress, SyncPhase, ProgressStats};

// Test modules
#[cfg(test)]
mod url_construction_tests;
#[cfg(test)]
mod subdirectory_edge_cases_tests;
#[cfg(test)]
mod protocol_detection_tests;
#[cfg(test)]
mod tests;