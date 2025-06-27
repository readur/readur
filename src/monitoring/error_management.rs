use tracing::{debug, info, warn, error};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Error classification system for better user experience
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ErrorSeverity {
    /// Critical errors that prevent core functionality - show to user
    Critical,
    /// Important errors that affect specific features - log and possibly notify
    Important, 
    /// Minor issues that don't impact functionality - log for debugging only
    Minor,
    /// Expected errors in normal operation - suppress unless debugging
    Expected,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ErrorCategory {
    /// PDF processing issues (font encoding, corruption, etc.)
    PdfProcessing,
    /// OCR processing issues
    OcrProcessing,
    /// Database constraints and data integrity
    Database,
    /// Network and external service issues
    Network,
    /// File system and storage issues
    FileSystem,
    /// Authentication and authorization
    Auth,
    /// Configuration and setup issues
    Config,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedError {
    pub category: ErrorCategory,
    pub severity: ErrorSeverity,
    pub code: String,
    pub user_message: String,
    pub technical_details: String,
    pub suggested_action: Option<String>,
    pub suppression_key: Option<String>, // For suppressing repeated errors
}

/// Error management service with intelligent logging and user experience
pub struct ErrorManager {
    error_suppressions: Arc<RwLock<HashMap<String, ErrorSuppressionState>>>,
}

#[derive(Debug, Clone)]
struct ErrorSuppressionState {
    count: usize,
    last_occurrence: chrono::DateTime<chrono::Utc>,
    suppressed_until: Option<chrono::DateTime<chrono::Utc>>,
}

impl ErrorManager {
    pub fn new() -> Self {
        Self {
            error_suppressions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Handle an error with intelligent logging and suppression
    pub async fn handle_error(&self, error: ManagedError) {
        // Check if this error should be suppressed
        if let Some(suppression_key) = &error.suppression_key {
            if self.should_suppress_error(suppression_key).await {
                debug!(
                    category = ?error.category,
                    code = error.code,
                    "Suppressed repeated error: {}", error.technical_details
                );
                return;
            }
            self.record_error_occurrence(suppression_key).await;
        }

        // Log based on severity and category
        match error.severity {
            ErrorSeverity::Critical => {
                error!(
                    category = ?error.category,
                    code = error.code,
                    user_message = error.user_message,
                    "Critical error: {}",
                    error.technical_details
                );
            }
            ErrorSeverity::Important => {
                warn!(
                    category = ?error.category,
                    code = error.code,
                    "Important error: {} | User: {}",
                    error.technical_details,
                    error.user_message
                );
            }
            ErrorSeverity::Minor => {
                info!(
                    category = ?error.category,
                    code = error.code,
                    "Minor issue: {}",
                    error.technical_details
                );
            }
            ErrorSeverity::Expected => {
                debug!(
                    category = ?error.category,
                    code = error.code,
                    "Expected error: {}",
                    error.technical_details
                );
            }
        }
    }

    /// Check if an error should be suppressed based on recent occurrences
    async fn should_suppress_error(&self, suppression_key: &str) -> bool {
        let suppressions = self.error_suppressions.read().await;
        if let Some(state) = suppressions.get(suppression_key) {
            // Suppress if we've seen this error more than 3 times in the last 5 minutes
            if state.count > 3 {
                let five_minutes_ago = chrono::Utc::now() - chrono::Duration::minutes(5);
                return state.last_occurrence > five_minutes_ago;
            }
        }
        false
    }

    /// Record that an error occurred for suppression tracking
    async fn record_error_occurrence(&self, suppression_key: &str) {
        let mut suppressions = self.error_suppressions.write().await;
        let now = chrono::Utc::now();
        
        let state = suppressions.entry(suppression_key.to_string()).or_insert(ErrorSuppressionState {
            count: 0,
            last_occurrence: now,
            suppressed_until: None,
        });
        
        state.count += 1;
        state.last_occurrence = now;
        
        // Reset count if last error was more than 10 minutes ago
        let ten_minutes_ago = now - chrono::Duration::minutes(10);
        if state.last_occurrence < ten_minutes_ago {
            state.count = 1;
        }
    }
}

/// PDF-specific error handling utilities
pub struct PdfErrorHandler;

impl PdfErrorHandler {
    /// Create a managed error for PDF font encoding issues
    pub fn font_encoding_error(filename: &str, file_size: u64, technical_error: &str) -> ManagedError {
        ManagedError {
            category: ErrorCategory::PdfProcessing,
            severity: ErrorSeverity::Expected, // These are common and not actionable by users
            code: "PDF_FONT_ENCODING".to_string(),
            user_message: format!("Processing '{}' using image-based OCR due to PDF encoding", filename),
            technical_details: format!(
                "PDF font encoding issue in {} ({} bytes): {}", 
                filename, file_size, technical_error
            ),
            suggested_action: Some("File will be processed using image-based OCR instead".to_string()),
            suppression_key: Some(format!("pdf_font_encoding_{}", filename)),
        }
    }

    /// Create a managed error for PDF corruption
    pub fn corruption_error(filename: &str, file_size: u64, technical_error: &str) -> ManagedError {
        ManagedError {
            category: ErrorCategory::PdfProcessing,
            severity: ErrorSeverity::Minor,
            code: "PDF_CORRUPTION".to_string(),
            user_message: format!("'{}' may be corrupted, attempting image-based OCR", filename),
            technical_details: format!(
                "PDF corruption detected in {} ({} bytes): {}", 
                filename, file_size, technical_error
            ),
            suggested_action: Some("Consider re-uploading the PDF if OCR results are poor".to_string()),
            suppression_key: Some(format!("pdf_corruption_{}", filename)),
        }
    }
}

/// OCR-specific error handling utilities  
pub struct OcrErrorHandler;

impl OcrErrorHandler {
    /// Create a managed error for OCR database constraint violations
    pub fn database_constraint_error(document_id: &str, constraint: &str, attempted_status: &str) -> ManagedError {
        ManagedError {
            category: ErrorCategory::Database,
            severity: ErrorSeverity::Critical,
            code: "OCR_STATUS_CONSTRAINT".to_string(),
            user_message: "Document processing encountered a system error".to_string(),
            technical_details: format!(
                "Invalid OCR status '{}' for document {} violates constraint {}", 
                attempted_status, document_id, constraint
            ),
            suggested_action: Some("Contact support if this persists".to_string()),
            suppression_key: None, // Don't suppress database issues
        }
    }

    /// Create a managed error for OCR processing timeouts
    pub fn timeout_error(filename: &str, timeout_seconds: u64) -> ManagedError {
        ManagedError {
            category: ErrorCategory::OcrProcessing,
            severity: ErrorSeverity::Important,
            code: "OCR_TIMEOUT".to_string(),
            user_message: format!("'{}' is taking longer than expected to process", filename),
            technical_details: format!(
                "OCR processing timeout after {} seconds for {}", 
                timeout_seconds, filename
            ),
            suggested_action: Some("Large or complex files may take additional time".to_string()),
            suppression_key: Some(format!("ocr_timeout_{}", filename)),
        }
    }
}

/// Macro for easy error handling throughout the codebase
#[macro_export]
macro_rules! handle_managed_error {
    ($error_manager:expr, $error:expr) => {
        $error_manager.handle_error($error).await;
    };
}

/// Global error manager instance (use with lazy_static or similar)
static ERROR_MANAGER: std::sync::OnceLock<ErrorManager> = std::sync::OnceLock::new();

pub fn get_error_manager() -> &'static ErrorManager {
    ERROR_MANAGER.get_or_init(|| ErrorManager::new())
}