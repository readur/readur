use anyhow::Result;
use std::collections::HashMap;
use std::time::Duration;

use crate::models::{
    ErrorSourceType, SourceErrorType, SourceErrorSeverity, SourceErrorClassifier,
    ErrorContext, ErrorClassification, SourceScanFailure, RetryStrategy,
};
use crate::models::source::{
    WebDAVScanFailureType, WebDAVScanFailureSeverity,
};

/// WebDAV-specific error classifier that maps WebDAV errors to the generic system
pub struct WebDAVErrorClassifier;

impl WebDAVErrorClassifier {
    pub fn new() -> Self {
        Self
    }

    /// Map WebDAV-specific error types to generic error types
    fn map_webdav_error_type(webdav_type: &WebDAVScanFailureType) -> SourceErrorType {
        match webdav_type {
            WebDAVScanFailureType::Timeout => SourceErrorType::Timeout,
            WebDAVScanFailureType::PathTooLong => SourceErrorType::PathTooLong,
            WebDAVScanFailureType::PermissionDenied => SourceErrorType::PermissionDenied,
            WebDAVScanFailureType::InvalidCharacters => SourceErrorType::InvalidCharacters,
            WebDAVScanFailureType::NetworkError => SourceErrorType::NetworkError,
            WebDAVScanFailureType::ServerError => SourceErrorType::ServerError,
            WebDAVScanFailureType::XmlParseError => SourceErrorType::XmlParseError,
            WebDAVScanFailureType::TooManyItems => SourceErrorType::TooManyItems,
            WebDAVScanFailureType::DepthLimit => SourceErrorType::DepthLimit,
            WebDAVScanFailureType::SizeLimit => SourceErrorType::SizeLimit,
            WebDAVScanFailureType::Unknown => SourceErrorType::Unknown,
        }
    }

    /// Map WebDAV-specific severity to generic severity
    fn map_webdav_severity(webdav_severity: &WebDAVScanFailureSeverity) -> SourceErrorSeverity {
        match webdav_severity {
            WebDAVScanFailureSeverity::Low => SourceErrorSeverity::Low,
            WebDAVScanFailureSeverity::Medium => SourceErrorSeverity::Medium,
            WebDAVScanFailureSeverity::High => SourceErrorSeverity::High,
            WebDAVScanFailureSeverity::Critical => SourceErrorSeverity::Critical,
        }
    }

    /// Classify WebDAV error using the original logic from error_tracking.rs
    fn classify_webdav_error_type(&self, error: &anyhow::Error) -> WebDAVScanFailureType {
        let error_str = error.to_string().to_lowercase();
        
        // Check for specific error patterns (from original WebDAV error tracking)
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

    /// Classify WebDAV error severity using original logic
    fn classify_webdav_severity(&self, 
        webdav_type: &WebDAVScanFailureType,
        http_status: Option<i32>,
        failure_count: i32,
    ) -> WebDAVScanFailureSeverity {
        match webdav_type {
            WebDAVScanFailureType::PathTooLong | 
            WebDAVScanFailureType::InvalidCharacters => WebDAVScanFailureSeverity::Critical,
            
            WebDAVScanFailureType::PermissionDenied |
            WebDAVScanFailureType::XmlParseError |
            WebDAVScanFailureType::TooManyItems |
            WebDAVScanFailureType::DepthLimit |
            WebDAVScanFailureType::SizeLimit => WebDAVScanFailureSeverity::High,
            
            WebDAVScanFailureType::Timeout |
            WebDAVScanFailureType::ServerError => {
                if let Some(code) = http_status {
                    if code == 404 {
                        WebDAVScanFailureSeverity::Critical
                    } else if code >= 500 {
                        WebDAVScanFailureSeverity::Medium
                    } else {
                        WebDAVScanFailureSeverity::Medium
                    }
                } else {
                    WebDAVScanFailureSeverity::Medium
                }
            },
            
            WebDAVScanFailureType::NetworkError => WebDAVScanFailureSeverity::Low,
            
            WebDAVScanFailureType::Unknown => {
                // Escalate severity based on failure count for unknown errors
                if failure_count > 5 {
                    WebDAVScanFailureSeverity::High
                } else {
                    WebDAVScanFailureSeverity::Medium
                }
            }
        }
    }

    /// Extract HTTP status code from error (from original WebDAV error tracking)
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

    /// Extract error code if present (from original WebDAV error tracking)
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

    /// Try to estimate item count from error message (from original WebDAV error tracking)
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

    /// Build WebDAV-specific diagnostic data
    fn build_webdav_diagnostics(&self, error: &anyhow::Error, context: &ErrorContext) -> serde_json::Value {
        let mut diagnostics = serde_json::json!({
            "error_chain": format!("{:?}", error),
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "webdav_specific": true,
        });

        // Add stack trace if available
        let backtrace = error.backtrace().to_string();
        if !backtrace.is_empty() && backtrace != "disabled backtrace" {
            diagnostics["backtrace"] = serde_json::json!(backtrace);
        }

        // Add WebDAV-specific context
        if let Some(server_type) = &context.server_type {
            diagnostics["server_type"] = serde_json::json!(server_type);
        }
        if let Some(server_version) = &context.server_version {
            diagnostics["server_version"] = serde_json::json!(server_version);
        }

        // Add estimated item count if available
        if let Some(item_count) = self.estimate_item_count_from_error(error) {
            diagnostics["estimated_item_count"] = serde_json::json!(item_count);
        }

        // Add path analysis
        let path_depth = context.resource_path.matches('/').count();
        diagnostics["path_length"] = serde_json::json!(context.resource_path.len());
        diagnostics["path_depth"] = serde_json::json!(path_depth);

        // Add response metrics
        if let Some(response_time) = context.response_time {
            diagnostics["response_time_ms"] = serde_json::json!(response_time.as_millis());
        }
        if let Some(response_size) = context.response_size {
            diagnostics["response_size_bytes"] = serde_json::json!(response_size);
        }

        // Add any additional context
        for (key, value) in &context.additional_context {
            diagnostics[key] = value.clone();
        }

        diagnostics
    }
}

impl SourceErrorClassifier for WebDAVErrorClassifier {
    fn classify_error(&self, error: &anyhow::Error, context: &ErrorContext) -> ErrorClassification {
        // Use original WebDAV classification logic
        let webdav_type = self.classify_webdav_error_type(error);
        let http_status = self.extract_http_status(error);
        let webdav_severity = self.classify_webdav_severity(&webdav_type, http_status, 1);

        // Map to generic types
        let error_type = Self::map_webdav_error_type(&webdav_type);
        let severity = Self::map_webdav_severity(&webdav_severity);

        // Determine retry strategy based on error type
        let retry_strategy = match webdav_type {
            WebDAVScanFailureType::NetworkError => RetryStrategy::Exponential,
            WebDAVScanFailureType::Timeout => RetryStrategy::Exponential,
            WebDAVScanFailureType::ServerError => RetryStrategy::Exponential,
            WebDAVScanFailureType::XmlParseError => RetryStrategy::Linear,
            _ => RetryStrategy::Exponential,
        };

        // Set retry delay based on error type
        let retry_delay_seconds = match webdav_type {
            WebDAVScanFailureType::NetworkError => 60,  // 1 minute
            WebDAVScanFailureType::Timeout => 900,      // 15 minutes
            WebDAVScanFailureType::ServerError => 300,  // 5 minutes
            WebDAVScanFailureType::XmlParseError => 600, // 10 minutes
            _ => 300, // 5 minutes default
        };

        // Set max retries based on severity
        let max_retries = match webdav_severity {
            WebDAVScanFailureSeverity::Critical => 1,
            WebDAVScanFailureSeverity::High => 3,
            WebDAVScanFailureSeverity::Medium => 5,
            WebDAVScanFailureSeverity::Low => 10,
        };

        // Build user-friendly message
        let user_friendly_message = self.build_webdav_user_message(&webdav_type, &context.resource_path, http_status);
        let recommended_action = self.build_webdav_recommended_action(&webdav_type, &webdav_severity);

        // Build diagnostic data
        let diagnostic_data = self.build_webdav_diagnostics(error, context);

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
        self.build_webdav_diagnostics(error, context)
    }

    fn build_user_friendly_message(&self, failure: &SourceScanFailure) -> String {
        // Convert generic failure back to WebDAV-specific types for message building
        let webdav_type = match failure.error_type {
            SourceErrorType::Timeout => WebDAVScanFailureType::Timeout,
            SourceErrorType::PathTooLong => WebDAVScanFailureType::PathTooLong,
            SourceErrorType::PermissionDenied => WebDAVScanFailureType::PermissionDenied,
            SourceErrorType::InvalidCharacters => WebDAVScanFailureType::InvalidCharacters,
            SourceErrorType::NetworkError => WebDAVScanFailureType::NetworkError,
            SourceErrorType::ServerError => WebDAVScanFailureType::ServerError,
            SourceErrorType::XmlParseError => WebDAVScanFailureType::XmlParseError,
            SourceErrorType::TooManyItems => WebDAVScanFailureType::TooManyItems,
            SourceErrorType::DepthLimit => WebDAVScanFailureType::DepthLimit,
            SourceErrorType::SizeLimit => WebDAVScanFailureType::SizeLimit,
            _ => WebDAVScanFailureType::Unknown,
        };

        self.build_webdav_user_message(&webdav_type, &failure.resource_path, failure.http_status_code)
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
        ErrorSourceType::WebDAV
    }
}

impl WebDAVErrorClassifier {
    /// Build WebDAV-specific user message (from original error tracking logic)
    fn build_webdav_user_message(&self, 
        failure_type: &WebDAVScanFailureType, 
        directory_path: &str,
        http_status: Option<i32>,
    ) -> String {
        match failure_type {
            WebDAVScanFailureType::Timeout => {
                format!(
                    "The WebDAV directory '{}' is taking too long to scan. This might be due to a large number of files or slow server response.",
                    directory_path
                )
            }
            WebDAVScanFailureType::PathTooLong => {
                format!(
                    "The WebDAV path '{}' exceeds system limits. Consider shortening directory names.",
                    directory_path
                )
            }
            WebDAVScanFailureType::PermissionDenied => {
                format!(
                    "Access denied to WebDAV directory '{}'. Please check your WebDAV permissions.",
                    directory_path
                )
            }
            WebDAVScanFailureType::TooManyItems => {
                format!(
                    "WebDAV directory '{}' contains too many items. Consider organizing into subdirectories.",
                    directory_path
                )
            }
            WebDAVScanFailureType::ServerError if http_status == Some(404) => {
                format!(
                    "WebDAV directory '{}' was not found on the server. It may have been deleted or moved.",
                    directory_path
                )
            }
            WebDAVScanFailureType::XmlParseError => {
                format!(
                    "Malformed XML response from WebDAV server for directory '{}'. Server may be incompatible.",
                    directory_path
                )
            }
            WebDAVScanFailureType::NetworkError => {
                format!(
                    "Network error accessing WebDAV directory '{}'. Check your connection.",
                    directory_path
                )
            }
            _ => {
                format!(
                    "Failed to scan WebDAV directory '{}'. Error will be retried automatically.",
                    directory_path
                )
            }
        }
    }

    /// Build WebDAV-specific recommended action
    fn build_webdav_recommended_action(&self, 
        failure_type: &WebDAVScanFailureType,
        severity: &WebDAVScanFailureSeverity,
    ) -> String {
        match (failure_type, severity) {
            (WebDAVScanFailureType::PathTooLong, _) => {
                "Shorten directory names or reorganize the directory structure.".to_string()
            }
            (WebDAVScanFailureType::InvalidCharacters, _) => {
                "Remove or rename directories with invalid characters.".to_string()
            }
            (WebDAVScanFailureType::PermissionDenied, _) => {
                "Check WebDAV server permissions and authentication credentials.".to_string()
            }
            (WebDAVScanFailureType::TooManyItems, _) => {
                "Split large directories into smaller subdirectories.".to_string()
            }
            (WebDAVScanFailureType::XmlParseError, _) => {
                "Check WebDAV server compatibility or contact server administrator.".to_string()
            }
            (WebDAVScanFailureType::Timeout, WebDAVScanFailureSeverity::High) => {
                "Consider excluding this directory from scanning due to repeated timeouts.".to_string()
            }
            (WebDAVScanFailureType::NetworkError, _) => {
                "Check network connectivity to WebDAV server.".to_string()
            }
            (_, WebDAVScanFailureSeverity::Critical) => {
                "Manual intervention required. This error cannot be resolved automatically.".to_string()
            }
            _ => {
                "The system will retry this operation automatically with increasing delays.".to_string()
            }
        }
    }
}