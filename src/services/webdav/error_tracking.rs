use anyhow::{anyhow, Result};
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::db::Database;
use crate::models::{
    CreateWebDAVScanFailure, WebDAVScanFailureType, WebDAVScanFailure,
    WebDAVScanFailureResponse, WebDAVFailureDiagnostics,
};

/// Helper for tracking and analyzing WebDAV scan failures
pub struct WebDAVErrorTracker {
    db: Database,
}

impl WebDAVErrorTracker {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
    
    /// Analyze an error and record it as a scan failure
    pub async fn track_scan_error(
        &self,
        user_id: Uuid,
        directory_path: &str,
        error: &anyhow::Error,
        response_time: Option<Duration>,
        response_size: Option<usize>,
        server_type: Option<&str>,
    ) -> Result<()> {
        let failure_type = self.classify_error_type(error);
        let http_status = self.extract_http_status(error);
        
        // Build diagnostic data
        let mut diagnostic_data = serde_json::json!({
            "error_chain": format!("{:?}", error),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        
        // Add stack trace if available
        if let Some(backtrace) = error.backtrace().to_string().as_str() {
            if !backtrace.is_empty() {
                diagnostic_data["backtrace"] = serde_json::json!(backtrace);
            }
        }
        
        // Estimate item count from error message if possible
        let estimated_items = self.estimate_item_count_from_error(error);
        
        let failure = CreateWebDAVScanFailure {
            user_id,
            directory_path: directory_path.to_string(),
            failure_type,
            error_message: error.to_string(),
            error_code: self.extract_error_code(error),
            http_status_code: http_status,
            response_time_ms: response_time.map(|d| d.as_millis() as i32),
            response_size_bytes: response_size.map(|s| s as i64),
            diagnostic_data: Some(diagnostic_data),
            server_type: server_type.map(|s| s.to_string()),
            server_version: None, // Could be extracted from headers if available
            estimated_item_count: estimated_items,
        };
        
        match self.db.record_scan_failure(&failure).await {
            Ok(failure_id) => {
                warn!(
                    "📝 Recorded scan failure for directory '{}': {} (ID: {})",
                    directory_path, error, failure_id
                );
            }
            Err(e) => {
                error!(
                    "Failed to record scan failure for directory '{}': {}",
                    directory_path, e
                );
            }
        }
        
        Ok(())
    }
    
    /// Check if a directory should be skipped due to previous failures
    pub async fn should_skip_directory(
        &self,
        user_id: Uuid,
        directory_path: &str,
    ) -> Result<bool> {
        match self.db.is_known_failure(user_id, directory_path).await {
            Ok(should_skip) => {
                if should_skip {
                    debug!(
                        "⏭️ Skipping directory '{}' due to previous failures",
                        directory_path
                    );
                }
                Ok(should_skip)
            }
            Err(e) => {
                // If we can't check, err on the side of trying to scan
                warn!(
                    "Failed to check failure status for directory '{}': {}",
                    directory_path, e
                );
                Ok(false)
            }
        }
    }
    
    /// Mark a directory scan as successful (resolves any previous failures)
    pub async fn mark_scan_successful(
        &self,
        user_id: Uuid,
        directory_path: &str,
    ) -> Result<()> {
        match self.db.resolve_scan_failure(user_id, directory_path, "successful_scan").await {
            Ok(resolved) => {
                if resolved {
                    info!(
                        "✅ Resolved previous scan failures for directory '{}'",
                        directory_path
                    );
                }
            }
            Err(e) => {
                debug!(
                    "Failed to mark scan as successful for directory '{}': {}",
                    directory_path, e
                );
            }
        }
        Ok(())
    }
    
    /// Get directories that are ready for retry
    pub async fn get_retry_candidates(&self, user_id: Uuid) -> Result<Vec<String>> {
        self.db.get_directories_ready_for_retry(user_id).await
    }
    
    /// Classify the type of error based on error message and context
    fn classify_error_type(&self, error: &anyhow::Error) -> WebDAVScanFailureType {
        let error_str = error.to_string().to_lowercase();
        
        // Check for specific error patterns
        if error_str.contains("timeout") || error_str.contains("timed out") {
            WebDAVScanFailureType::Timeout
        } else if error_str.contains("name too long") || error_str.contains("path too long") {
            WebDAVScanFailureType::PathTooLong
        } else if error_str.contains("permission denied") || error_str.contains("forbidden") || error_str.contains("401") || error_str.contains("403") {
            WebDAVScanFailureType::PermissionDenied
        } else if error_str.contains("invalid character") || error_str.contains("illegal character") {
            WebDAVScanFailureType::InvalidCharacters
        } else if error_str.contains("connection refused") || error_str.contains("network") || error_str.contains("dns") {
            WebDAVScanFailureType::NetworkError
        } else if error_str.contains("500") || error_str.contains("502") || error_str.contains("503") || error_str.contains("504") {
            WebDAVScanFailureType::ServerError
        } else if error_str.contains("xml") || error_str.contains("parse") || error_str.contains("malformed") {
            WebDAVScanFailureType::XmlParseError
        } else if error_str.contains("too many") || error_str.contains("limit exceeded") {
            WebDAVScanFailureType::TooManyItems
        } else if error_str.contains("depth") || error_str.contains("nested") {
            WebDAVScanFailureType::DepthLimit
        } else if error_str.contains("size") || error_str.contains("too large") {
            WebDAVScanFailureType::SizeLimit
        } else if error_str.contains("404") || error_str.contains("not found") {
            WebDAVScanFailureType::ServerError // Will be further classified by HTTP status
        } else {
            WebDAVScanFailureType::Unknown
        }
    }
    
    /// Extract HTTP status code from error if present
    fn extract_http_status(&self, error: &anyhow::Error) -> Option<i32> {
        let error_str = error.to_string();
        
        // Look for common HTTP status code patterns
        if error_str.contains("404") {
            Some(404)
        } else if error_str.contains("401") {
            Some(401)
        } else if error_str.contains("403") {
            Some(403)
        } else if error_str.contains("500") {
            Some(500)
        } else if error_str.contains("502") {
            Some(502)
        } else if error_str.contains("503") {
            Some(503)
        } else if error_str.contains("504") {
            Some(504)
        } else if error_str.contains("405") {
            Some(405)
        } else {
            // Try to extract any 3-digit number that looks like an HTTP status
            let re = regex::Regex::new(r"\b([4-5]\d{2})\b").ok()?;
            re.captures(&error_str)
                .and_then(|cap| cap.get(1))
                .and_then(|m| m.as_str().parse::<i32>().ok())
        }
    }
    
    /// Extract error code if present (e.g., system error codes)
    fn extract_error_code(&self, error: &anyhow::Error) -> Option<String> {
        let error_str = error.to_string();
        
        // Look for common error code patterns
        if let Some(caps) = regex::Regex::new(r"(?i)error[:\s]+([A-Z0-9_]+)")
            .ok()
            .and_then(|re| re.captures(&error_str))
        {
            return caps.get(1).map(|m| m.as_str().to_string());
        }
        
        // Look for OS error codes
        if let Some(caps) = regex::Regex::new(r"(?i)os error (\d+)")
            .ok()
            .and_then(|re| re.captures(&error_str))
        {
            return caps.get(1).map(|m| format!("OS_{}", m.as_str()));
        }
        
        None
    }
    
    /// Try to estimate item count from error message
    fn estimate_item_count_from_error(&self, error: &anyhow::Error) -> Option<i32> {
        let error_str = error.to_string();
        
        // Look for patterns like "1000 items", "contains 500 files", etc.
        if let Some(caps) = regex::Regex::new(r"(\d+)\s*(?:items?|files?|directories|folders?|entries)")
            .ok()
            .and_then(|re| re.captures(&error_str))
        {
            return caps.get(1)
                .and_then(|m| m.as_str().parse::<i32>().ok());
        }
        
        None
    }
    
    /// Build a user-friendly error message with recommendations
    pub fn build_user_friendly_error_message(
        &self,
        failure: &WebDAVScanFailure,
    ) -> String {
        use crate::models::WebDAVScanFailureType;
        
        let base_message = match &failure.failure_type {
            WebDAVScanFailureType::Timeout => {
                format!(
                    "The directory '{}' is taking too long to scan. This might be due to a large number of files or slow server response.",
                    failure.directory_path
                )
            }
            WebDAVScanFailureType::PathTooLong => {
                format!(
                    "The path '{}' exceeds system limits ({}+ characters). Consider shortening directory names.",
                    failure.directory_path,
                    failure.path_length.unwrap_or(0)
                )
            }
            WebDAVScanFailureType::PermissionDenied => {
                format!(
                    "Access denied to '{}'. Please check your WebDAV permissions.",
                    failure.directory_path
                )
            }
            WebDAVScanFailureType::TooManyItems => {
                format!(
                    "Directory '{}' contains too many items (estimated: {}). Consider organizing into subdirectories.",
                    failure.directory_path,
                    failure.estimated_item_count.unwrap_or(0)
                )
            }
            WebDAVScanFailureType::ServerError if failure.http_status_code == Some(404) => {
                format!(
                    "Directory '{}' was not found on the server. It may have been deleted or moved.",
                    failure.directory_path
                )
            }
            _ => {
                format!(
                    "Failed to scan directory '{}': {}",
                    failure.directory_path,
                    failure.error_message.as_ref().unwrap_or(&"Unknown error".to_string())
                )
            }
        };
        
        // Add retry information if applicable
        let retry_info = if failure.consecutive_failures > 1 {
            format!(
                " This has failed {} times.",
                failure.consecutive_failures
            )
        } else {
            String::new()
        };
        
        // Add next retry time if scheduled
        let next_retry = if let Some(next_retry_at) = failure.next_retry_at {
            if !failure.user_excluded && !failure.resolved {
                let duration = next_retry_at.signed_duration_since(chrono::Utc::now());
                if duration.num_seconds() > 0 {
                    format!(
                        " Will retry in {} minutes.",
                        duration.num_minutes().max(1)
                    )
                } else {
                    " Ready for retry.".to_string()
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        
        format!("{}{}{}", base_message, retry_info, next_retry)
    }
}

/// Extension trait for WebDAV service to add error tracking capabilities
pub trait WebDAVServiceErrorTracking {
    /// Track an error that occurred during scanning
    async fn track_scan_error(
        &self,
        user_id: Uuid,
        directory_path: &str,
        error: anyhow::Error,
        scan_duration: Duration,
    ) -> Result<()>;
    
    /// Check if directory should be skipped
    async fn should_skip_for_failures(
        &self,
        user_id: Uuid,
        directory_path: &str,
    ) -> Result<bool>;
    
    /// Mark directory scan as successful
    async fn mark_scan_success(
        &self,
        user_id: Uuid,
        directory_path: &str,
    ) -> Result<()>;
}