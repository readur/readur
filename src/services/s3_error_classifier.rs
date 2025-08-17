use anyhow::Result;
use std::collections::HashMap;

use crate::models::{
    ErrorSourceType, SourceErrorType, SourceErrorSeverity, SourceErrorClassifier,
    ErrorContext, ErrorClassification, SourceScanFailure, RetryStrategy,
};

/// S3-specific error classifier for generic error tracking system
pub struct S3ErrorClassifier;

impl S3ErrorClassifier {
    pub fn new() -> Self {
        Self
    }

    /// Classify S3-specific errors based on AWS SDK error patterns
    fn classify_s3_error_type(&self, error: &anyhow::Error) -> SourceErrorType {
        let error_str = error.to_string().to_lowercase();
        
        // AWS S3 specific error patterns
        if error_str.contains("nosuchbucket") || error_str.contains("no such bucket") {
            SourceErrorType::NotFound
        } else if error_str.contains("nosuchkey") || error_str.contains("no such key") {
            SourceErrorType::NotFound
        } else if error_str.contains("accessdenied") || error_str.contains("access denied") {
            SourceErrorType::PermissionDenied
        } else if error_str.contains("invalidbucketname") || error_str.contains("invalid bucket name") {
            SourceErrorType::InvalidCharacters
        } else if error_str.contains("requesttimeout") || error_str.contains("timeout") {
            SourceErrorType::Timeout
        } else if error_str.contains("slowdown") || error_str.contains("throttling") {
            SourceErrorType::RateLimited
        } else if error_str.contains("serviceunavailable") || error_str.contains("service unavailable") {
            SourceErrorType::ServerError
        } else if error_str.contains("internalerror") || error_str.contains("internal error") {
            SourceErrorType::ServerError
        } else if error_str.contains("invalidsecurity") || error_str.contains("signaturemismatch") {
            SourceErrorType::PermissionDenied
        } else if error_str.contains("quotaexceeded") || error_str.contains("quota exceeded") {
            SourceErrorType::QuotaExceeded
        } else if error_str.contains("entitytoolarge") || error_str.contains("too large") {
            SourceErrorType::SizeLimit
        } else if error_str.contains("network") || error_str.contains("connection") {
            SourceErrorType::NetworkError
        } else if error_str.contains("json") || error_str.contains("xml") || error_str.contains("parse") {
            SourceErrorType::JsonParseError
        } else if error_str.contains("conflict") {
            SourceErrorType::Conflict
        } else if error_str.contains("unsupported") || error_str.contains("not implemented") {
            SourceErrorType::UnsupportedOperation
        } else {
            SourceErrorType::Unknown
        }
    }

    /// Determine appropriate severity for S3 errors
    fn classify_s3_severity(&self, error_type: &SourceErrorType, error_str: &str) -> SourceErrorSeverity {
        match error_type {
            SourceErrorType::NotFound => {
                // Check if it's a bucket vs object not found
                if error_str.contains("bucket") {
                    SourceErrorSeverity::Critical // Bucket not found is critical
                } else {
                    SourceErrorSeverity::Medium // Object not found might be temporary
                }
            }
            SourceErrorType::PermissionDenied => SourceErrorSeverity::High,
            SourceErrorType::InvalidCharacters => SourceErrorSeverity::Critical,
            SourceErrorType::QuotaExceeded => SourceErrorSeverity::High,
            SourceErrorType::SizeLimit => SourceErrorSeverity::High,
            SourceErrorType::UnsupportedOperation => SourceErrorSeverity::Critical,
            SourceErrorType::Timeout => SourceErrorSeverity::Medium,
            SourceErrorType::RateLimited => SourceErrorSeverity::Low,
            SourceErrorType::NetworkError => SourceErrorSeverity::Low,
            SourceErrorType::ServerError => SourceErrorSeverity::Medium,
            SourceErrorType::JsonParseError => SourceErrorSeverity::Medium,
            SourceErrorType::Conflict => SourceErrorSeverity::Medium,
            _ => SourceErrorSeverity::Medium,
        }
    }

    /// Extract AWS error code from error message
    fn extract_aws_error_code(&self, error: &anyhow::Error) -> Option<String> {
        let error_str = error.to_string();
        
        // Look for AWS error code patterns
        // Example: "Error: NoSuchBucket (S3ResponseError)"
        if let Some(caps) = regex::Regex::new(r"(?i)(NoSuchBucket|NoSuchKey|AccessDenied|InvalidBucketName|RequestTimeout|Throttling|SlowDown|ServiceUnavailable|InternalError|InvalidSecurity|SignatureMismatch|QuotaExceeded|EntityTooLarge)")
            .ok()
            .and_then(|re| re.captures(&error_str))
        {
            return caps.get(1).map(|m| m.as_str().to_string());
        }
        
        // Look for HTTP status in AWS responses
        if let Some(caps) = regex::Regex::new(r"(?i)status[:\s]+(\d{3})")
            .ok()
            .and_then(|re| re.captures(&error_str))
        {
            return caps.get(1).map(|m| format!("HTTP_{}", m.as_str()));
        }
        
        None
    }

    /// Build S3-specific diagnostic data
    fn build_s3_diagnostics(&self, error: &anyhow::Error, context: &ErrorContext) -> serde_json::Value {
        let mut diagnostics = serde_json::json!({
            "error_chain": format!("{:?}", error),
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "s3_specific": true,
            "operation": context.operation,
        });

        // Extract S3-specific information from error
        let error_str = error.to_string();
        
        // Try to extract bucket name
        if let Some(caps) = regex::Regex::new(r"bucket[:\s]+([a-z0-9.-]+)")
            .ok()
            .and_then(|re| re.captures(&error_str))
        {
            diagnostics["bucket_name"] = serde_json::json!(caps.get(1).unwrap().as_str());
        }
        
        // Try to extract region information
        if let Some(caps) = regex::Regex::new(r"region[:\s]+([a-z0-9-]+)")
            .ok()
            .and_then(|re| re.captures(&error_str))
        {
            diagnostics["aws_region"] = serde_json::json!(caps.get(1).unwrap().as_str());
        }

        // Add path analysis for S3 keys
        let key_depth = context.resource_path.matches('/').count();
        diagnostics["key_length"] = serde_json::json!(context.resource_path.len());
        diagnostics["key_depth"] = serde_json::json!(key_depth);
        
        // Add performance metrics
        if let Some(response_time) = context.response_time {
            diagnostics["response_time_ms"] = serde_json::json!(response_time.as_millis());
        }
        if let Some(response_size) = context.response_size {
            diagnostics["response_size_bytes"] = serde_json::json!(response_size);
        }

        // Add any additional S3-specific context
        for (key, value) in &context.additional_context {
            diagnostics[key] = value.clone();
        }

        diagnostics
    }

    /// Build S3-specific user-friendly messages
    fn build_s3_user_message(&self, 
        error_type: &SourceErrorType, 
        resource_path: &str,
        error_message: &str,
    ) -> String {
        match error_type {
            SourceErrorType::NotFound => {
                if error_message.to_lowercase().contains("bucket") {
                    format!("S3 bucket for path '{}' does not exist or is not accessible.", resource_path)
                } else {
                    format!("S3 object '{}' was not found. It may have been deleted or moved.", resource_path)
                }
            }
            SourceErrorType::PermissionDenied => {
                format!("Access denied to S3 resource '{}'. Check your AWS credentials and bucket permissions.", resource_path)
            }
            SourceErrorType::InvalidCharacters => {
                format!("S3 path '{}' contains invalid characters. Please use valid S3 key naming conventions.", resource_path)
            }
            SourceErrorType::QuotaExceeded => {
                format!("AWS quota exceeded for S3 operations. Please check your AWS service limits.")
            }
            SourceErrorType::RateLimited => {
                format!("S3 requests are being rate limited. Operations will be retried with exponential backoff.")
            }
            SourceErrorType::Timeout => {
                format!("S3 request for '{}' timed out. This may be due to large object size or network issues.", resource_path)
            }
            SourceErrorType::NetworkError => {
                format!("Network error accessing S3 resource '{}'. Check your internet connection.", resource_path)
            }
            SourceErrorType::ServerError => {
                format!("AWS S3 service error for resource '{}'. This is usually temporary.", resource_path)
            }
            SourceErrorType::SizeLimit => {
                format!("S3 object '{}' exceeds size limits for processing.", resource_path)
            }
            _ => {
                format!("Error accessing S3 resource '{}': {}", resource_path, error_message)
            }
        }
    }

    /// Build S3-specific recommended actions
    fn build_s3_recommended_action(&self, error_type: &SourceErrorType, severity: &SourceErrorSeverity) -> String {
        match (error_type, severity) {
            (SourceErrorType::NotFound, SourceErrorSeverity::Critical) => {
                "Verify the S3 bucket name and region configuration.".to_string()
            }
            (SourceErrorType::PermissionDenied, _) => {
                "Check AWS IAM permissions and S3 bucket policies. Ensure read access is granted.".to_string()
            }
            (SourceErrorType::InvalidCharacters, _) => {
                "Rename S3 objects to use valid characters and naming conventions.".to_string()
            }
            (SourceErrorType::QuotaExceeded, _) => {
                "Contact AWS support to increase service limits or reduce usage.".to_string()
            }
            (SourceErrorType::RateLimited, _) => {
                "Reduce request rate or enable request throttling. Will retry automatically.".to_string()
            }
            (SourceErrorType::SizeLimit, _) => {
                "Consider splitting large objects or excluding them from processing.".to_string()
            }
            (SourceErrorType::NetworkError, _) => {
                "Check network connectivity to AWS S3 endpoints.".to_string()
            }
            (_, SourceErrorSeverity::Critical) => {
                "Manual intervention required. This S3 error cannot be resolved automatically.".to_string()
            }
            _ => {
                "S3 operations will be retried automatically with appropriate delays.".to_string()
            }
        }
    }
}

impl SourceErrorClassifier for S3ErrorClassifier {
    fn classify_error(&self, error: &anyhow::Error, context: &ErrorContext) -> ErrorClassification {
        let error_type = self.classify_s3_error_type(error);
        let error_str = error.to_string();
        let severity = self.classify_s3_severity(&error_type, &error_str);

        // Determine retry strategy based on error type
        let retry_strategy = match error_type {
            SourceErrorType::RateLimited => RetryStrategy::Exponential, // AWS recommends exponential backoff
            SourceErrorType::NetworkError => RetryStrategy::Linear,
            SourceErrorType::Timeout => RetryStrategy::Exponential,
            SourceErrorType::ServerError => RetryStrategy::Exponential,
            _ => RetryStrategy::Exponential,
        };

        // Set retry delay based on error type
        let retry_delay_seconds = match error_type {
            SourceErrorType::RateLimited => 1200, // 20 minutes for rate limiting
            SourceErrorType::NetworkError => 30,   // 30 seconds for network
            SourceErrorType::Timeout => 300,       // 5 minutes for timeouts
            SourceErrorType::ServerError => 180,   // 3 minutes for server errors
            _ => 300, // 5 minutes default
        };

        // Set max retries based on severity
        let max_retries = match severity {
            SourceErrorSeverity::Critical => 1,
            SourceErrorSeverity::High => 2,
            SourceErrorSeverity::Medium => 5,
            SourceErrorSeverity::Low => 10,
        };

        // Build user-friendly message and recommended action
        let user_friendly_message = self.build_s3_user_message(&error_type, &context.resource_path, &error_str);
        let recommended_action = self.build_s3_recommended_action(&error_type, &severity);

        // Build diagnostic data
        let diagnostic_data = self.build_s3_diagnostics(error, context);

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
        self.build_s3_diagnostics(error, context)
    }

    fn build_user_friendly_message(&self, failure: &SourceScanFailure) -> String {
        let binding = String::new();
        let error_message = failure.error_message.as_ref().unwrap_or(&binding);
        self.build_s3_user_message(&failure.error_type, &failure.resource_path, error_message)
    }

    fn should_retry(&self, failure: &SourceScanFailure) -> bool {
        match failure.error_severity {
            SourceErrorSeverity::Critical => false,
            SourceErrorSeverity::High => failure.failure_count < 2,
            SourceErrorSeverity::Medium => failure.failure_count < 5,
            SourceErrorSeverity::Low => failure.failure_count < 10,
        }
    }

    fn source_type(&self) -> ErrorSourceType {
        ErrorSourceType::S3
    }
}