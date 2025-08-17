// Re-export all model types for backward compatibility and ease of use

pub mod user;
pub mod document;
pub mod search;
pub mod settings;
pub mod source;
pub mod source_error;
pub mod responses;

// Re-export commonly used types - being explicit to avoid naming conflicts
pub use user::*;
pub use document::*;
pub use search::*;
pub use settings::*;

// Re-export source types with explicit naming to avoid conflicts
pub use source::{
    Source, SourceStatus, CreateSource, UpdateSource, 
    SourceResponse, SourceWithStats, WebDAVSourceConfig, 
    LocalFolderSourceConfig, S3SourceConfig, Notification, 
    NotificationSummary, CreateNotification, WebDAVFolderInfo
};

// Use fully qualified path for source::SourceType to distinguish from source_error::MonitoredSourceType
pub use source::SourceType;

// Re-export source_error types with full qualification
pub use source_error::*;

pub use responses::*;