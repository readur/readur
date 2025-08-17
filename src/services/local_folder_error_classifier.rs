use anyhow::Result;
use std::collections::HashMap;

use crate::models::{
    ErrorSourceType, SourceErrorType, SourceErrorSeverity, SourceErrorClassifier,
    ErrorContext, ErrorClassification, SourceScanFailure, RetryStrategy,
};

/// Local filesystem error classifier for generic error tracking system
pub struct LocalFolderErrorClassifier;

impl LocalFolderErrorClassifier {
    pub fn new() -> Self {
        Self
    }

    /// Classify local filesystem errors based on standard OS error patterns
    fn classify_local_error_type(&self, error: &anyhow::Error) -> SourceErrorType {
        let error_str = error.to_string().to_lowercase();
        
        // Standard filesystem error patterns
        if error_str.contains("permission denied") || error_str.contains("access denied") {
            SourceErrorType::PermissionDenied
        } else if error_str.contains("no such file") || error_str.contains("not found") || error_str.contains("does not exist") {
            SourceErrorType::NotFound
        } else if error_str.contains("file name too long") || error_str.contains("path too long") || error_str.contains("name too long") {
            SourceErrorType::PathTooLong
        } else if error_str.contains("invalid filename") || error_str.contains("invalid characters") || error_str.contains("illegal character") {
            SourceErrorType::InvalidCharacters
        } else if error_str.contains("too many files") || error_str.contains("too many entries") {
            SourceErrorType::TooManyItems
        } else if error_str.contains("directory not empty") || error_str.contains("file exists") {
            SourceErrorType::Conflict
        } else if error_str.contains("no space") || error_str.contains("disk full") || error_str.contains("quota exceeded") {
            SourceErrorType::QuotaExceeded
        } else if error_str.contains("file too large") || error_str.contains("size limit") {
            SourceErrorType::SizeLimit
        } else if error_str.contains("too many links") || error_str.contains("link count") {
            SourceErrorType::DepthLimit
        } else if error_str.contains("device busy") || error_str.contains("resource busy") {
            SourceErrorType::Conflict
        } else if error_str.contains("operation not supported") || error_str.contains("function not implemented") {
            SourceErrorType::UnsupportedOperation
        } else if error_str.contains("timeout") || error_str.contains("timed out") {
            SourceErrorType::Timeout
        } else if error_str.contains("network") || error_str.contains("connection") {
            SourceErrorType::NetworkError // For network filesystems
        } else {
            SourceErrorType::Unknown
        }
    }

    /// Determine appropriate severity for local filesystem errors
    fn classify_local_severity(&self, error_type: &SourceErrorType, path: &str) -> SourceErrorSeverity {
        match error_type {
            SourceErrorType::PathTooLong | 
            SourceErrorType::InvalidCharacters => SourceErrorSeverity::Critical,
            
            SourceErrorType::PermissionDenied => {
                // System directories are more critical than user directories
                if path.starts_with("/etc/") || path.starts_with("/sys/") || path.starts_with("/proc/") {
                    SourceErrorSeverity::Critical
                } else {
                    SourceErrorSeverity::High
                }
            }
            
            SourceErrorType::NotFound => {
                // Root directories not found is critical
                if path.len() < 10 && path.matches('/').count() <= 2 {
                    SourceErrorSeverity::Critical
                } else {
                    SourceErrorSeverity::Medium
                }
            }
            
            SourceErrorType::QuotaExceeded | 
            SourceErrorType::TooManyItems => SourceErrorSeverity::High,
            
            SourceErrorType::SizeLimit | 
            SourceErrorType::DepthLimit => SourceErrorSeverity::High,
            
            SourceErrorType::UnsupportedOperation => SourceErrorSeverity::Critical,
            
            SourceErrorType::Conflict => SourceErrorSeverity::Medium,
            
            SourceErrorType::Timeout |
            SourceErrorType::NetworkError => SourceErrorSeverity::Medium,
            
            _ => SourceErrorSeverity::Low,
        }
    }

    /// Extract OS error code from error message
    fn extract_os_error_code(&self, error: &anyhow::Error) -> Option<String> {
        let error_str = error.to_string();
        
        // Look for OS error codes
        if let Some(caps) = regex::Regex::new(r"(?i)os error (\d+)")
            .ok()
            .and_then(|re| re.captures(&error_str))
        {
            return caps.get(1).map(|m| format!("OS_{}", m.as_str()));
        }
        
        // Look for errno patterns
        if let Some(caps) = regex::Regex::new(r"(?i)errno[:\s]+(\d+)")
            .ok()
            .and_then(|re| re.captures(&error_str))
        {
            return caps.get(1).map(|m| format!("ERRNO_{}", m.as_str()));
        }
        
        // Look for Windows error codes
        if let Some(caps) = regex::Regex::new(r"(?i)error[:\s]+(\d+)")
            .ok()
            .and_then(|re| re.captures(&error_str))
        {
            return caps.get(1).map(|m| format!("WIN_{}", m.as_str()));
        }
        
        None
    }

    /// Get filesystem type from path patterns
    fn detect_filesystem_type(&self, path: &str) -> Option<String> {
        if path.starts_with("/proc/") {
            Some("procfs".to_string())
        } else if path.starts_with("/sys/") {
            Some("sysfs".to_string())
        } else if path.starts_with("/dev/") {
            Some("devfs".to_string())
        } else if path.starts_with("/tmp/") || path.starts_with("/var/tmp/") {
            Some("tmpfs".to_string())
        } else if cfg!(windows) && path.len() >= 3 && path.chars().nth(1) == Some(':') {
            Some("ntfs".to_string())
        } else if cfg!(unix) {
            Some("unix".to_string())
        } else {
            None
        }
    }

    /// Build local filesystem diagnostic data
    fn build_local_diagnostics(&self, error: &anyhow::Error, context: &ErrorContext) -> serde_json::Value {
        let mut diagnostics = serde_json::json!({
            "error_chain": format!("{:?}", error),
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "local_filesystem": true,
            "operation": context.operation,
        });

        // Add path analysis
        let path = &context.resource_path;
        let path_depth = path.matches(std::path::MAIN_SEPARATOR).count();
        let path_length = path.len();
        
        diagnostics["path_length"] = serde_json::json!(path_length);
        diagnostics["path_depth"] = serde_json::json!(path_depth);
        diagnostics["path_components"] = serde_json::json!(path.split(std::path::MAIN_SEPARATOR).count());

        // Detect filesystem type
        if let Some(fs_type) = self.detect_filesystem_type(path) {
            diagnostics["filesystem_type"] = serde_json::json!(fs_type);
        }

        // Add OS information
        diagnostics["os_type"] = if cfg!(windows) {
            serde_json::json!("windows")
        } else if cfg!(unix) {
            serde_json::json!("unix")
        } else {
            serde_json::json!("unknown")
        };

        // Try to get file/directory metadata if accessible
        if let Ok(metadata) = std::fs::metadata(path) {
            diagnostics["is_directory"] = serde_json::json!(metadata.is_dir());
            diagnostics["is_file"] = serde_json::json!(metadata.is_file());
            diagnostics["size_bytes"] = serde_json::json!(metadata.len());
            
            // Add permissions on Unix systems
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = metadata.permissions().mode();
                diagnostics["unix_permissions"] = serde_json::json!(format!("{:o}", mode));
            }
        }

        // Add performance metrics
        if let Some(response_time) = context.response_time {
            diagnostics["response_time_ms"] = serde_json::json!(response_time.as_millis());
        }

        // Add any additional context
        for (key, value) in &context.additional_context {
            diagnostics[key] = value.clone();
        }

        diagnostics
    }

    /// Build local filesystem user-friendly messages
    fn build_local_user_message(&self, 
        error_type: &SourceErrorType, 
        resource_path: &str,
        error_message: &str,
    ) -> String {
        match error_type {
            SourceErrorType::NotFound => {
                format!("Local path '{}' does not exist. It may have been deleted or moved.", resource_path)
            }
            SourceErrorType::PermissionDenied => {
                format!("Access denied to local path '{}'. Check file/directory permissions.", resource_path)
            }
            SourceErrorType::PathTooLong => {
                format!("Local path '{}' exceeds filesystem limits. Consider shortening the path.", resource_path)
            }
            SourceErrorType::InvalidCharacters => {
                format!("Local path '{}' contains invalid characters for this filesystem.", resource_path)
            }
            SourceErrorType::TooManyItems => {
                format!("Directory '{}' contains too many files for efficient processing.", resource_path)
            }
            SourceErrorType::QuotaExceeded => {
                format!("Disk quota exceeded for path '{}'. Free up space or increase quota.", resource_path)
            }
            SourceErrorType::SizeLimit => {
                format!("File '{}' exceeds size limits for processing.", resource_path)
            }
            SourceErrorType::Conflict => {
                format!("File or directory conflict at '{}'. Resource may be in use.", resource_path)
            }
            SourceErrorType::UnsupportedOperation => {
                format!("Operation not supported on filesystem for path '{}'.", resource_path)
            }
            SourceErrorType::Timeout => {
                format!("Filesystem operation timed out for path '{}'. This may indicate slow storage.", resource_path)
            }
            SourceErrorType::NetworkError => {
                format!("Network filesystem error for path '{}'. Check network connectivity.", resource_path)
            }
            _ => {
                format!("Error accessing local path '{}': {}", resource_path, error_message)
            }
        }
    }

    /// Build local filesystem recommended actions
    fn build_local_recommended_action(&self, error_type: &SourceErrorType, severity: &SourceErrorSeverity) -> String {
        match (error_type, severity) {
            (SourceErrorType::NotFound, SourceErrorSeverity::Critical) => {
                "Verify the base directory path exists and is accessible.".to_string()
            }
            (SourceErrorType::PermissionDenied, _) => {
                "Check file/directory permissions and user access rights. Consider running with elevated privileges if appropriate.".to_string()
            }
            (SourceErrorType::PathTooLong, _) => {
                "Shorten the path by reorganizing directory structure or using shorter names.".to_string()
            }
            (SourceErrorType::InvalidCharacters, _) => {
                "Rename files/directories to remove invalid characters for this filesystem.".to_string()
            }
            (SourceErrorType::TooManyItems, _) => {
                "Consider organizing files into subdirectories or excluding this directory from processing.".to_string()
            }
            (SourceErrorType::QuotaExceeded, _) => {
                "Free up disk space or contact administrator to increase quota limits.".to_string()
            }
            (SourceErrorType::SizeLimit, _) => {
                "Consider excluding large files from processing or splitting them if possible.".to_string()
            }
            (SourceErrorType::UnsupportedOperation, _) => {
                "This filesystem type does not support the required operation.".to_string()
            }
            (SourceErrorType::NetworkError, _) => {
                "Check network connection for network filesystem mounts.".to_string()
            }
            (_, SourceErrorSeverity::Critical) => {
                "Manual intervention required. This filesystem error cannot be resolved automatically.".to_string()
            }
            _ => {
                "Filesystem operations will be retried automatically after a brief delay.".to_string()
            }
        }
    }
}

impl SourceErrorClassifier for LocalFolderErrorClassifier {
    fn classify_error(&self, error: &anyhow::Error, context: &ErrorContext) -> ErrorClassification {
        let error_type = self.classify_local_error_type(error);
        let severity = self.classify_local_severity(&error_type, &context.resource_path);

        // Determine retry strategy - local filesystem errors usually don't benefit from exponential backoff
        let retry_strategy = match error_type {
            SourceErrorType::NetworkError => RetryStrategy::Exponential, // For network filesystems
            SourceErrorType::Timeout => RetryStrategy::Linear,
            SourceErrorType::Conflict => RetryStrategy::Linear, // Resource might become available
            _ => RetryStrategy::Fixed, // Most filesystem errors are immediate
        };

        // Set retry delay based on error type
        let retry_delay_seconds = match error_type {
            SourceErrorType::NetworkError => 60,   // 1 minute for network issues
            SourceErrorType::Timeout => 30,        // 30 seconds for timeouts
            SourceErrorType::Conflict => 10,       // 10 seconds for conflicts
            SourceErrorType::QuotaExceeded => 300, // 5 minutes for quota issues
            _ => 5, // 5 seconds for most filesystem errors
        };

        // Set max retries based on severity
        let max_retries = match severity {
            SourceErrorSeverity::Critical => 1,
            SourceErrorSeverity::High => 3,
            SourceErrorSeverity::Medium => 5,
            SourceErrorSeverity::Low => 10,
        };

        // Build user-friendly message and recommended action
        let error_str = error.to_string();
        let user_friendly_message = self.build_local_user_message(&error_type, &context.resource_path, &error_str);
        let recommended_action = self.build_local_recommended_action(&error_type, &severity);

        // Build diagnostic data
        let diagnostic_data = self.build_local_diagnostics(error, context);

        ErrorClassification {
            error_type,
            severity,
            retry_strategy,
            retry_delay_seconds,
            max_retries,
            user_friendly_message,
            recommended_action,
            diagnostic_data,
        }
    }

    fn extract_diagnostics(&self, error: &anyhow::Error, context: &ErrorContext) -> serde_json::Value {
        self.build_local_diagnostics(error, context)
    }

    fn build_user_friendly_message(&self, failure: &SourceScanFailure) -> String {
        let binding = String::new();
        let error_message = failure.error_message.as_ref().unwrap_or(&binding);
        self.build_local_user_message(&failure.error_type, &failure.resource_path, error_message)
    }

    fn should_retry(&self, failure: &SourceScanFailure) -> bool {
        match failure.error_severity {
            SourceErrorSeverity::Critical => false,
            SourceErrorSeverity::High => failure.failure_count < 3,
            SourceErrorSeverity::Medium => failure.failure_count < 5,
            SourceErrorSeverity::Low => failure.failure_count < 10,
        }
    }

    fn source_type(&self) -> ErrorSourceType {
        ErrorSourceType::Local
    }
}