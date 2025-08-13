// Simplified WebDAV service modules - consolidated architecture

pub mod config;
pub mod service; 
pub mod smart_sync;
pub mod progress_shim; // Backward compatibility shim for simplified progress tracking

// New enhanced WebDAV features
pub mod locking;
pub mod partial_content;
pub mod directory_ops;
pub mod status_codes;

// Re-export main types for convenience
pub use config::{WebDAVConfig, RetryConfig, ConcurrencyConfig};
pub use service::{
    WebDAVService, WebDAVDiscoveryResult, ServerCapabilities, HealthStatus, test_webdav_connection,
    ValidationReport, ValidationIssue, ValidationIssueType, ValidationSeverity, 
    ValidationRecommendation, ValidationAction, ValidationSummary, WebDAVDownloadResult
};
pub use smart_sync::{SmartSyncService, SmartSyncDecision, SmartSyncStrategy, SmartSyncResult};

// Export new feature types
pub use locking::{LockManager, LockInfo, LockScope, LockType, LockDepth, LockRequest};
pub use partial_content::{PartialContentManager, ByteRange, DownloadProgress};
pub use directory_ops::{CreateDirectoryOptions, DirectoryCreationResult};
pub use status_codes::{WebDAVStatusCode, WebDAVError, StatusCodeHandler};

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
#[cfg(test)]
mod locking_tests;
#[cfg(test)]
mod partial_content_tests;
#[cfg(test)]
mod directory_ops_tests;