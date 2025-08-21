use anyhow::Result;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::db::Database;
use crate::models::{
    CreateSourceScanFailure, SourceScanFailure, SourceScanFailureResponse,
    SourceScanFailureStats, ErrorSourceType, SourceErrorType, SourceErrorSeverity,
    SourceErrorClassifier, ErrorContext, ErrorClassification,
    ListFailuresQuery, RetryFailureRequest, ExcludeResourceRequest,
};

/// Pre-compiled regex patterns for performance
static HTTP_STATUS_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b([4-5]\d{2})\b").expect("HTTP status regex should be valid")
});

static ERROR_CODE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)error[:\s]+([A-Z0-9_]+)").expect("Error code regex should be valid")
});

static OS_ERROR_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)os error (\d+)").expect("OS error regex should be valid")
});

/// Detailed information about why a resource should or shouldn't be skipped
#[derive(Debug, Clone)]
pub struct SkipDecision {
    pub should_skip: bool,
    pub reason: String,
    pub failure_count: i32,
    pub time_since_last_failure_minutes: i64,
    pub cooldown_remaining_minutes: Option<i64>,
}

/// Generic error tracking service for all source types
#[derive(Clone)]
pub struct SourceErrorTracker {
    db: Database,
    classifiers: HashMap<ErrorSourceType, Arc<dyn SourceErrorClassifier>>,
}

impl SourceErrorTracker {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            classifiers: HashMap::new(),
        }
    }

    /// Register a source-specific error classifier
    pub fn register_classifier(&mut self, classifier: Arc<dyn SourceErrorClassifier>) {
        let source_type = classifier.source_type();
        self.classifiers.insert(source_type, classifier);
        info!("Registered error classifier for source type: {:?}", source_type);
    }

    /// Track an error for any source type
    pub async fn track_error(
        &self,
        user_id: Uuid,
        source_type: ErrorSourceType,
        source_id: Option<Uuid>,
        resource_path: &str,
        error: &anyhow::Error,
        context: ErrorContext,
    ) -> Result<Uuid> {
        let classification = if let Some(classifier) = self.classifiers.get(&source_type) {
            classifier.classify_error(error, &context)
        } else {
            // Fallback to generic classification
            self.classify_error_generic(error, &context)
        };

        let create_failure = CreateSourceScanFailure {
            user_id,
            source_type,
            source_id,
            resource_path: resource_path.to_string(),
            error_type: classification.error_type,
            error_message: error.to_string(),
            error_code: self.extract_error_code(error),
            http_status_code: self.extract_http_status(error),
            response_time_ms: context.response_time.map(|d| d.as_millis() as i32),
            response_size_bytes: context.response_size.map(|s| s as i64),
            resource_size_bytes: None, // Will be filled by specific classifiers
            diagnostic_data: Some(classification.diagnostic_data),
        };

        match self.db.record_source_scan_failure(&create_failure).await {
            Ok(failure_id) => {
                warn!(
                    "📝 Recorded scan failure for {} resource '{}': {} (ID: {})",
                    source_type, resource_path, error, failure_id
                );
                Ok(failure_id)
            }
            Err(e) => {
                error!(
                    "Failed to record scan failure for {} resource '{}': {}",
                    source_type, resource_path, e
                );
                Err(e)
            }
        }
    }

    /// Check if a resource should be skipped due to previous failures
    pub async fn should_skip_resource(
        &self,
        user_id: Uuid,
        source_type: ErrorSourceType,
        source_id: Option<Uuid>,
        resource_path: &str,
    ) -> Result<bool> {
        self.should_skip_resource_with_details(user_id, source_type, source_id, resource_path).await
            .map(|result| result.should_skip)
    }
    
    /// Check if a resource should be skipped with detailed information about why
    pub async fn should_skip_resource_with_details(
        &self,
        user_id: Uuid,
        source_type: ErrorSourceType,
        source_id: Option<Uuid>,
        resource_path: &str,
    ) -> Result<SkipDecision> {
        // Check if there are any failures for this resource path
        match self.db.is_source_known_failure(user_id, source_type, source_id, resource_path).await {
            Ok(true) => {
                // There is a known failure, but we need more details
                // For now, implement a simple cooldown based on the fact that there's a failure
                // In a real implementation, we'd need a method that returns failure details
                let skip_reason = format!("Resource has previous failures recorded in system");
                
                info!(
                    "⏭️ Skipping {} resource '{}' due to error tracking: {}",
                    source_type, resource_path, skip_reason
                );
                
                Ok(SkipDecision {
                    should_skip: true,
                    reason: skip_reason,
                    failure_count: 1, // We don't have exact count, use 1 as placeholder
                    time_since_last_failure_minutes: 0, // We don't have exact time
                    cooldown_remaining_minutes: Some(60), // Default 1 hour cooldown
                })
            }
            Ok(false) => {
                debug!(
                    "✅ No previous failures for {} resource '{}', proceeding with scan",
                    source_type, resource_path
                );
                Ok(SkipDecision {
                    should_skip: false,
                    reason: "No previous failures recorded".to_string(),
                    failure_count: 0,
                    time_since_last_failure_minutes: 0,
                    cooldown_remaining_minutes: None,
                })
            }
            Err(e) => {
                warn!(
                    "Failed to check failure status for {} resource '{}': {}",
                    source_type, resource_path, e
                );
                // If we can't check, err on the side of trying to scan
                Ok(SkipDecision {
                    should_skip: false,
                    reason: format!("Error checking failure status: {}", e),
                    failure_count: 0,
                    time_since_last_failure_minutes: 0,
                    cooldown_remaining_minutes: None,
                })
            }
        }
    }

    /// Mark a resource scan as successful (resolves any previous failures)
    pub async fn mark_success(
        &self,
        user_id: Uuid,
        source_type: ErrorSourceType,
        source_id: Option<Uuid>,
        resource_path: &str,
    ) -> Result<()> {
        match self.db.resolve_source_scan_failure(
            user_id, 
            source_type, 
            source_id, 
            resource_path, 
            "successful_scan"
        ).await {
            Ok(resolved) => {
                if resolved {
                    info!(
                        "✅ Resolved previous scan failures for {} resource '{}'",
                        source_type, resource_path
                    );
                }
                Ok(())
            }
            Err(e) => {
                debug!(
                    "Failed to mark scan as successful for {} resource '{}': {}",
                    source_type, resource_path, e
                );
                Ok(()) // Don't fail the entire operation for this
            }
        }
    }

    /// Get resources ready for retry
    pub async fn get_retry_candidates(
        &self,
        user_id: Uuid,
        source_type: Option<ErrorSourceType>,
        limit: Option<i32>,
    ) -> Result<Vec<SourceScanFailure>> {
        self.db.get_source_retry_candidates(user_id, source_type, limit.unwrap_or(10)).await
    }

    /// Get scan failures with optional filtering
    pub async fn list_failures(
        &self,
        user_id: Uuid,
        query: ListFailuresQuery,
    ) -> Result<Vec<SourceScanFailureResponse>> {
        let failures = self.db.list_source_scan_failures(user_id, &query).await?;
        
        let mut responses = Vec::new();
        for failure in failures {
            let diagnostic_summary = if let Some(classifier) = self.classifiers.get(&failure.source_type) {
                self.build_diagnostics_with_classifier(&failure, classifier.as_ref())
            } else {
                self.build_diagnostics_generic(&failure)
            };

            responses.push(SourceScanFailureResponse {
                id: failure.id,
                source_type: failure.source_type,
                source_name: None, // Will be filled by joined query in future enhancement
                resource_path: failure.resource_path,
                error_type: failure.error_type,
                error_severity: failure.error_severity,
                failure_count: failure.failure_count,
                consecutive_failures: failure.consecutive_failures,
                first_failure_at: failure.first_failure_at,
                last_failure_at: failure.last_failure_at,
                next_retry_at: failure.next_retry_at,
                error_message: failure.error_message,
                http_status_code: failure.http_status_code,
                user_excluded: failure.user_excluded,
                user_notes: failure.user_notes,
                resolved: failure.resolved,
                diagnostic_summary,
            });
        }

        Ok(responses)
    }

    /// Get detailed failure information
    pub async fn get_failure_details(
        &self,
        user_id: Uuid,
        failure_id: Uuid,
    ) -> Result<Option<SourceScanFailureResponse>> {
        if let Some(failure) = self.db.get_source_scan_failure(user_id, failure_id).await? {
            let diagnostic_summary = if let Some(classifier) = self.classifiers.get(&failure.source_type) {
                self.build_diagnostics_with_classifier(&failure, classifier.as_ref())
            } else {
                self.build_diagnostics_generic(&failure)
            };

            Ok(Some(SourceScanFailureResponse {
                id: failure.id,
                source_type: failure.source_type,
                source_name: None, // Will be filled by joined query
                resource_path: failure.resource_path,
                error_type: failure.error_type,
                error_severity: failure.error_severity,
                failure_count: failure.failure_count,
                consecutive_failures: failure.consecutive_failures,
                first_failure_at: failure.first_failure_at,
                last_failure_at: failure.last_failure_at,
                next_retry_at: failure.next_retry_at,
                error_message: failure.error_message,
                http_status_code: failure.http_status_code,
                user_excluded: failure.user_excluded,
                user_notes: failure.user_notes,
                resolved: failure.resolved,
                diagnostic_summary,
            }))
        } else {
            Ok(None)
        }
    }

    /// Retry a failed resource
    pub async fn retry_failure(
        &self,
        user_id: Uuid,
        failure_id: Uuid,
        request: RetryFailureRequest,
    ) -> Result<bool> {
        if let Some(failure) = self.db.get_source_scan_failure(user_id, failure_id).await? {
            let success = self.db.reset_source_scan_failure(
                user_id,
                failure.source_type,
                failure.source_id,
                &failure.resource_path,
            ).await?;

            if success {
                info!(
                    "🔄 Reset failure for {} resource '{}' for retry",
                    failure.source_type, failure.resource_path
                );
            }

            Ok(success)
        } else {
            Ok(false)
        }
    }

    /// Exclude a resource from scanning
    pub async fn exclude_resource(
        &self,
        user_id: Uuid,
        failure_id: Uuid,
        request: ExcludeResourceRequest,
    ) -> Result<bool> {
        if let Some(failure) = self.db.get_source_scan_failure(user_id, failure_id).await? {
            let success = self.db.exclude_source_from_scan(
                user_id,
                failure.source_type,
                failure.source_id,
                &failure.resource_path,
                Some(&request.reason),
            ).await?;

            if success {
                info!(
                    "🚫 Excluded {} resource '{}' from scanning: {}",
                    failure.source_type, failure.resource_path, request.reason
                );
            }

            Ok(success)
        } else {
            Ok(false)
        }
    }

    /// Get failure statistics
    pub async fn get_stats(&self, user_id: Uuid, source_type: Option<ErrorSourceType>) -> Result<SourceScanFailureStats> {
        self.db.get_source_scan_failure_stats(user_id, source_type).await
    }

    /// Build user-friendly error message using source-specific classifier
    pub fn build_user_friendly_message(&self, failure: &SourceScanFailure) -> String {
        if let Some(classifier) = self.classifiers.get(&failure.source_type) {
            classifier.build_user_friendly_message(failure)
        } else {
            self.build_user_friendly_message_generic(failure)
        }
    }

    /// Generic error classification fallback
    fn classify_error_generic(&self, error: &anyhow::Error, context: &ErrorContext) -> ErrorClassification {
        let error_str = error.to_string().to_lowercase();
        
        let error_type = if error_str.contains("timeout") || error_str.contains("timed out") {
            SourceErrorType::Timeout
        } else if error_str.contains("permission denied") || error_str.contains("forbidden") || error_str.contains("401") || error_str.contains("403") {
            SourceErrorType::PermissionDenied
        } else if error_str.contains("not found") || error_str.contains("404") {
            SourceErrorType::NotFound
        } else if error_str.contains("connection refused") || error_str.contains("network") || error_str.contains("dns") {
            SourceErrorType::NetworkError
        } else if error_str.contains("500") || error_str.contains("502") || error_str.contains("503") || error_str.contains("504") {
            SourceErrorType::ServerError
        } else if error_str.contains("too many") || error_str.contains("rate limit") {
            SourceErrorType::RateLimited
        } else {
            SourceErrorType::Unknown
        };

        let severity = match error_type {
            SourceErrorType::NotFound => SourceErrorSeverity::Critical,
            SourceErrorType::PermissionDenied => SourceErrorSeverity::High,
            SourceErrorType::Timeout | SourceErrorType::ServerError => SourceErrorSeverity::Medium,
            SourceErrorType::NetworkError | SourceErrorType::RateLimited => SourceErrorSeverity::Low,
            _ => SourceErrorSeverity::Medium,
        };

        let retry_strategy = match error_type {
            SourceErrorType::RateLimited => crate::models::RetryStrategy::Linear,
            SourceErrorType::NetworkError => crate::models::RetryStrategy::Exponential,
            _ => crate::models::RetryStrategy::Exponential,
        };

        let retry_delay = match error_type {
            SourceErrorType::RateLimited => 600, // 10 minutes for rate limits
            SourceErrorType::NetworkError => 60, // 1 minute for network issues
            SourceErrorType::Timeout => 900,     // 15 minutes for timeouts
            _ => 300,                             // 5 minutes default
        };

        ErrorClassification {
            error_type,
            severity,
            retry_strategy,
            retry_delay_seconds: retry_delay,
            max_retries: 5,
            user_friendly_message: format!("Error accessing resource: {}", error),
            recommended_action: "The system will retry this operation automatically.".to_string(),
            diagnostic_data: serde_json::json!({
                "error_message": error.to_string(),
                "context": {
                    "operation": context.operation,
                    "response_time_ms": context.response_time.map(|d| d.as_millis()),
                    "response_size": context.response_size,
                }
            }),
        }
    }

    /// Generic diagnostics builder
    fn build_diagnostics_generic(&self, failure: &SourceScanFailure) -> crate::models::SourceFailureDiagnostics {
        let resource_size_mb = failure.resource_size_bytes.map(|b| b as f64 / 1_048_576.0);
        let response_size_mb = failure.response_size_bytes.map(|b| b as f64 / 1_048_576.0);

        let (recommended_action, can_retry, user_action_required) = match (&failure.error_type, &failure.error_severity) {
            (SourceErrorType::NotFound, _) => (
                "Resource not found. It may have been deleted or moved.".to_string(),
                false,
                false,
            ),
            (SourceErrorType::PermissionDenied, _) => (
                "Access denied. Check permissions for this resource.".to_string(),
                false,
                true,
            ),
            (SourceErrorType::Timeout, _) if failure.failure_count > 3 => (
                "Repeated timeouts. Resource may be too large or source is slow.".to_string(),
                true,
                false,
            ),
            (SourceErrorType::NetworkError, _) => (
                "Network error. Will retry automatically.".to_string(),
                true,
                false,
            ),
            (SourceErrorType::RateLimited, _) => (
                "Rate limited. Will retry with longer delays.".to_string(),
                true,
                false,
            ),
            _ if failure.error_severity == SourceErrorSeverity::Critical => (
                "Critical error that requires manual intervention.".to_string(),
                false,
                true,
            ),
            _ => (
                "Temporary error. Will retry automatically.".to_string(),
                true,
                false,
            ),
        };

        crate::models::SourceFailureDiagnostics {
            resource_depth: failure.resource_depth,
            estimated_item_count: failure.estimated_item_count,
            response_time_ms: failure.response_time_ms,
            response_size_mb,
            resource_size_mb,
            recommended_action,
            can_retry,
            user_action_required,
            source_specific_info: HashMap::new(),
        }
    }

    /// Build diagnostics using source-specific classifier
    fn build_diagnostics_with_classifier(
        &self,
        failure: &SourceScanFailure,
        classifier: &dyn SourceErrorClassifier,
    ) -> crate::models::SourceFailureDiagnostics {
        // Start with generic diagnostics
        let mut diagnostics = self.build_diagnostics_generic(failure);
        
        // Enhance with source-specific information
        let user_message = classifier.build_user_friendly_message(failure);
        if !user_message.is_empty() {
            diagnostics.recommended_action = user_message;
        }
        
        // Extract source-specific diagnostic info from the diagnostic_data JSON
        if let Some(data) = failure.diagnostic_data.as_object() {
            for (key, value) in data {
                diagnostics.source_specific_info.insert(key.clone(), value.clone());
            }
        }
        
        diagnostics
    }

    /// Generic user-friendly message builder
    fn build_user_friendly_message_generic(&self, failure: &SourceScanFailure) -> String {
        let base_message = match &failure.error_type {
            SourceErrorType::Timeout => {
                format!(
                    "The {} resource '{}' is taking too long to access. This might be due to a large size or slow connection.",
                    failure.source_type, failure.resource_path
                )
            }
            SourceErrorType::PermissionDenied => {
                format!(
                    "Access denied to {} resource '{}'. Please check your permissions.",
                    failure.source_type, failure.resource_path
                )
            }
            SourceErrorType::NotFound => {
                format!(
                    "{} resource '{}' was not found. It may have been deleted or moved.",
                    failure.source_type, failure.resource_path
                )
            }
            SourceErrorType::NetworkError => {
                format!(
                    "Network error accessing {} resource '{}'. Will retry automatically.",
                    failure.source_type, failure.resource_path
                )
            }
            _ => {
                format!(
                    "Error accessing {} resource '{}': {}",
                    failure.source_type,
                    failure.resource_path,
                    failure.error_message.as_ref().unwrap_or(&"Unknown error".to_string())
                )
            }
        };

        // Add retry information if applicable
        let retry_info = if failure.consecutive_failures > 1 {
            format!(" This has failed {} times.", failure.consecutive_failures)
        } else {
            String::new()
        };

        // Add next retry time if scheduled
        let next_retry = if let Some(next_retry_at) = failure.next_retry_at {
            if !failure.user_excluded && !failure.resolved {
                let duration = next_retry_at.signed_duration_since(chrono::Utc::now());
                if duration.num_seconds() > 0 {
                    format!(" Will retry in {} minutes.", duration.num_minutes().max(1))
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

    /// Extract HTTP status code from error if present
    fn extract_http_status(&self, error: &anyhow::Error) -> Option<i32> {
        let error_str = error.to_string();
        
        // Look for common HTTP status code patterns first (fast path)
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
        } else {
            // Use pre-compiled regex for any other 4xx/5xx status codes
            HTTP_STATUS_REGEX.captures(&error_str)
                .and_then(|cap| cap.get(1))
                .and_then(|m| m.as_str().parse::<i32>().ok())
        }
    }

    /// Extract error code if present (e.g., system error codes)
    fn extract_error_code(&self, error: &anyhow::Error) -> Option<String> {
        let error_str = error.to_string();
        
        // Look for common error code patterns using pre-compiled regex
        if let Some(caps) = ERROR_CODE_REGEX.captures(&error_str) {
            return caps.get(1).map(|m| m.as_str().to_string());
        }
        
        // Look for OS error codes using pre-compiled regex
        if let Some(caps) = OS_ERROR_REGEX.captures(&error_str) {
            return caps.get(1).map(|m| format!("OS_{}", m.as_str()));
        }
        
        None
    }
}

/// Extension trait for services to add error tracking capabilities
pub trait SourceServiceErrorTracking {
    /// Track an error that occurred during operation
    async fn track_error(
        &self,
        user_id: Uuid,
        resource_path: &str,
        error: anyhow::Error,
        operation_duration: Duration,
    ) -> Result<()>;
    
    /// Check if resource should be skipped
    async fn should_skip_for_failures(
        &self,
        user_id: Uuid,
        resource_path: &str,
    ) -> Result<bool>;
    
    /// Mark resource operation as successful
    async fn mark_operation_success(
        &self,
        user_id: Uuid,
        resource_path: &str,
    ) -> Result<()>;
}