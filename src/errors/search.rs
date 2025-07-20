use axum::http::StatusCode;
use thiserror::Error;

use super::{AppError, ErrorCategory, ErrorSeverity, impl_into_response};

/// Errors related to search operations
#[derive(Error, Debug)]
pub enum SearchError {
    #[error("Search query is too short: {length} characters (minimum: {min_length})")]
    QueryTooShort { length: usize, min_length: usize },
    
    #[error("Search query is too long: {length} characters (maximum: {max_length})")]
    QueryTooLong { length: usize, max_length: usize },
    
    #[error("Search index is unavailable: {reason}")]
    IndexUnavailable { reason: String },
    
    #[error("Invalid search syntax: {details}")]
    InvalidSyntax { details: String },
    
    #[error("Too many search results: {result_count} (maximum: {max_results})")]
    TooManyResults { result_count: i64, max_results: i64 },
    
    #[error("Search timeout after {timeout_seconds} seconds")]
    SearchTimeout { timeout_seconds: u64 },
    
    #[error("Invalid search mode '{mode}'. Valid modes: simple, phrase, fuzzy, boolean")]
    InvalidSearchMode { mode: String },
    
    #[error("Invalid MIME type filter '{mime_type}'")]
    InvalidMimeType { mime_type: String },
    
    #[error("Invalid pagination parameters: offset {offset}, limit {limit}")]
    InvalidPagination { offset: i64, limit: i64 },
    
    #[error("Boolean search syntax error: {details}")]
    BooleanSyntaxError { details: String },
    
    #[error("Fuzzy search threshold {threshold} is invalid. Valid range: 0.0 - 1.0")]
    InvalidFuzzyThreshold { threshold: f32 },
    
    #[error("Search index is rebuilding, try again in a few minutes")]
    IndexRebuilding,
    
    #[error("Search operation cancelled by user")]
    SearchCancelled,
    
    #[error("No search results found")]
    NoResults,
    
    #[error("Invalid snippet length {length}. Valid range: {min_length} - {max_length}")]
    InvalidSnippetLength { length: i32, min_length: i32, max_length: i32 },
    
    #[error("Search quota exceeded: {queries_today} queries today (limit: {daily_limit})")]
    QuotaExceeded { queries_today: i64, daily_limit: i64 },
    
    #[error("Invalid tag filter '{tag}'")]
    InvalidTagFilter { tag: String },
    
    #[error("Search index corruption detected: {details}")]
    IndexCorruption { details: String },
    
    #[error("Permission denied: cannot search documents belonging to other users")]
    PermissionDenied,
    
    #[error("Search feature is disabled")]
    SearchDisabled,
}

impl AppError for SearchError {
    fn status_code(&self) -> StatusCode {
        match self {
            SearchError::QueryTooShort { .. } => StatusCode::BAD_REQUEST,
            SearchError::QueryTooLong { .. } => StatusCode::BAD_REQUEST,
            SearchError::IndexUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
            SearchError::InvalidSyntax { .. } => StatusCode::BAD_REQUEST,
            SearchError::TooManyResults { .. } => StatusCode::PAYLOAD_TOO_LARGE,
            SearchError::SearchTimeout { .. } => StatusCode::REQUEST_TIMEOUT,
            SearchError::InvalidSearchMode { .. } => StatusCode::BAD_REQUEST,
            SearchError::InvalidMimeType { .. } => StatusCode::BAD_REQUEST,
            SearchError::InvalidPagination { .. } => StatusCode::BAD_REQUEST,
            SearchError::BooleanSyntaxError { .. } => StatusCode::BAD_REQUEST,
            SearchError::InvalidFuzzyThreshold { .. } => StatusCode::BAD_REQUEST,
            SearchError::IndexRebuilding => StatusCode::SERVICE_UNAVAILABLE,
            SearchError::SearchCancelled => StatusCode::REQUEST_TIMEOUT,
            SearchError::NoResults => StatusCode::NOT_FOUND,
            SearchError::InvalidSnippetLength { .. } => StatusCode::BAD_REQUEST,
            SearchError::QuotaExceeded { .. } => StatusCode::TOO_MANY_REQUESTS,
            SearchError::InvalidTagFilter { .. } => StatusCode::BAD_REQUEST,
            SearchError::IndexCorruption { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            SearchError::PermissionDenied => StatusCode::FORBIDDEN,
            SearchError::SearchDisabled => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
    
    fn user_message(&self) -> String {
        match self {
            SearchError::QueryTooShort { min_length, .. } => format!("Search query must be at least {} characters", min_length),
            SearchError::QueryTooLong { max_length, .. } => format!("Search query must be less than {} characters", max_length),
            SearchError::IndexUnavailable { .. } => "Search is temporarily unavailable".to_string(),
            SearchError::InvalidSyntax { .. } => "Invalid search syntax".to_string(),
            SearchError::TooManyResults { max_results, .. } => format!("Too many results. Please refine your search (limit: {})", max_results),
            SearchError::SearchTimeout { .. } => "Search timed out. Please try a more specific query".to_string(),
            SearchError::InvalidSearchMode { .. } => "Invalid search mode. Use: simple, phrase, fuzzy, or boolean".to_string(),
            SearchError::InvalidMimeType { .. } => "Invalid file type filter".to_string(),
            SearchError::InvalidPagination { .. } => "Invalid pagination parameters".to_string(),
            SearchError::BooleanSyntaxError { details } => format!("Boolean search syntax error: {}", details),
            SearchError::InvalidFuzzyThreshold { .. } => "Fuzzy search threshold must be between 0.0 and 1.0".to_string(),
            SearchError::IndexRebuilding => "Search index is being rebuilt. Please try again in a few minutes".to_string(),
            SearchError::SearchCancelled => "Search was cancelled".to_string(),
            SearchError::NoResults => "No results found for your search".to_string(),
            SearchError::InvalidSnippetLength { min_length, max_length, .. } => format!("Snippet length must be between {} and {} characters", min_length, max_length),
            SearchError::QuotaExceeded { daily_limit, .. } => format!("Daily search limit of {} queries exceeded", daily_limit),
            SearchError::InvalidTagFilter { .. } => "Invalid tag filter specified".to_string(),
            SearchError::IndexCorruption { .. } => "Search index error. Please contact support".to_string(),
            SearchError::PermissionDenied => "Permission denied for search operation".to_string(),
            SearchError::SearchDisabled => "Search feature is currently disabled".to_string(),
        }
    }
    
    fn error_code(&self) -> &'static str {
        match self {
            SearchError::QueryTooShort { .. } => "SEARCH_QUERY_TOO_SHORT",
            SearchError::QueryTooLong { .. } => "SEARCH_QUERY_TOO_LONG",
            SearchError::IndexUnavailable { .. } => "SEARCH_INDEX_UNAVAILABLE",
            SearchError::InvalidSyntax { .. } => "SEARCH_INVALID_SYNTAX",
            SearchError::TooManyResults { .. } => "SEARCH_TOO_MANY_RESULTS",
            SearchError::SearchTimeout { .. } => "SEARCH_TIMEOUT",
            SearchError::InvalidSearchMode { .. } => "SEARCH_INVALID_MODE",
            SearchError::InvalidMimeType { .. } => "SEARCH_INVALID_MIME_TYPE",
            SearchError::InvalidPagination { .. } => "SEARCH_INVALID_PAGINATION",
            SearchError::BooleanSyntaxError { .. } => "SEARCH_BOOLEAN_SYNTAX_ERROR",
            SearchError::InvalidFuzzyThreshold { .. } => "SEARCH_INVALID_FUZZY_THRESHOLD",
            SearchError::IndexRebuilding => "SEARCH_INDEX_REBUILDING",
            SearchError::SearchCancelled => "SEARCH_CANCELLED",
            SearchError::NoResults => "SEARCH_NO_RESULTS",
            SearchError::InvalidSnippetLength { .. } => "SEARCH_INVALID_SNIPPET_LENGTH",
            SearchError::QuotaExceeded { .. } => "SEARCH_QUOTA_EXCEEDED",
            SearchError::InvalidTagFilter { .. } => "SEARCH_INVALID_TAG_FILTER",
            SearchError::IndexCorruption { .. } => "SEARCH_INDEX_CORRUPTION",
            SearchError::PermissionDenied => "SEARCH_PERMISSION_DENIED",
            SearchError::SearchDisabled => "SEARCH_DISABLED",
        }
    }
    
    fn error_category(&self) -> ErrorCategory {
        match self {
            SearchError::PermissionDenied => ErrorCategory::Auth,
            SearchError::IndexUnavailable { .. } 
            | SearchError::IndexRebuilding 
            | SearchError::IndexCorruption { .. } => ErrorCategory::Database,
            SearchError::SearchTimeout { .. } 
            | SearchError::SearchCancelled => ErrorCategory::Network,
            _ => ErrorCategory::Database, // Most search operations are database-related
        }
    }
    
    fn error_severity(&self) -> ErrorSeverity {
        match self {
            SearchError::IndexCorruption { .. } => ErrorSeverity::Critical,
            SearchError::IndexUnavailable { .. } 
            | SearchError::IndexRebuilding 
            | SearchError::SearchDisabled => ErrorSeverity::Important,
            SearchError::NoResults 
            | SearchError::QueryTooShort { .. } 
            | SearchError::QueryTooLong { .. } 
            | SearchError::InvalidSyntax { .. } => ErrorSeverity::Expected,
            _ => ErrorSeverity::Minor,
        }
    }
    
    fn suppression_key(&self) -> Option<String> {
        match self {
            SearchError::IndexUnavailable { .. } => Some("search_index_unavailable".to_string()),
            SearchError::IndexRebuilding => Some("search_index_rebuilding".to_string()),
            SearchError::SearchTimeout { .. } => Some("search_timeout".to_string()),
            SearchError::NoResults => Some("search_no_results".to_string()),
            _ => None,
        }
    }
    
    fn suggested_action(&self) -> Option<String> {
        match self {
            SearchError::QueryTooShort { min_length, .. } => Some(format!("Enter at least {} characters for your search", min_length)),
            SearchError::QueryTooLong { max_length, .. } => Some(format!("Shorten your search to less than {} characters", max_length)),
            SearchError::TooManyResults { .. } => Some("Use more specific search terms or apply filters".to_string()),
            SearchError::SearchTimeout { .. } => Some("Try a more specific search query".to_string()),
            SearchError::InvalidSearchMode { .. } => Some("Use one of: 'simple', 'phrase', 'fuzzy', or 'boolean'".to_string()),
            SearchError::BooleanSyntaxError { .. } => Some("Check boolean operators (AND, OR, NOT) and parentheses".to_string()),
            SearchError::InvalidFuzzyThreshold { .. } => Some("Set fuzzy threshold between 0.0 (loose) and 1.0 (exact)".to_string()),
            SearchError::IndexRebuilding => Some("Wait a few minutes for index rebuild to complete".to_string()),
            SearchError::NoResults => Some("Try different keywords or check spelling".to_string()),
            SearchError::InvalidSnippetLength { min_length, max_length, .. } => Some(format!("Set snippet length between {} and {}", min_length, max_length)),
            SearchError::QuotaExceeded { .. } => Some("Wait until tomorrow or contact administrator for limit increase".to_string()),
            SearchError::SearchDisabled => Some("Contact administrator to enable search functionality".to_string()),
            _ => None,
        }
    }
}

impl_into_response!(SearchError);

/// Convenience methods for creating common search errors
impl SearchError {
    pub fn query_too_short(length: usize, min_length: usize) -> Self {
        Self::QueryTooShort { length, min_length }
    }
    
    pub fn query_too_long(length: usize, max_length: usize) -> Self {
        Self::QueryTooLong { length, max_length }
    }
    
    pub fn index_unavailable<S: Into<String>>(reason: S) -> Self {
        Self::IndexUnavailable { reason: reason.into() }
    }
    
    pub fn invalid_syntax<S: Into<String>>(details: S) -> Self {
        Self::InvalidSyntax { details: details.into() }
    }
    
    pub fn too_many_results(result_count: i64, max_results: i64) -> Self {
        Self::TooManyResults { result_count, max_results }
    }
    
    pub fn search_timeout(timeout_seconds: u64) -> Self {
        Self::SearchTimeout { timeout_seconds }
    }
    
    pub fn invalid_search_mode<S: Into<String>>(mode: S) -> Self {
        Self::InvalidSearchMode { mode: mode.into() }
    }
    
    pub fn invalid_mime_type<S: Into<String>>(mime_type: S) -> Self {
        Self::InvalidMimeType { mime_type: mime_type.into() }
    }
    
    pub fn invalid_pagination(offset: i64, limit: i64) -> Self {
        Self::InvalidPagination { offset, limit }
    }
    
    pub fn boolean_syntax_error<S: Into<String>>(details: S) -> Self {
        Self::BooleanSyntaxError { details: details.into() }
    }
    
    pub fn invalid_fuzzy_threshold(threshold: f32) -> Self {
        Self::InvalidFuzzyThreshold { threshold }
    }
    
    pub fn invalid_snippet_length(length: i32, min_length: i32, max_length: i32) -> Self {
        Self::InvalidSnippetLength { length, min_length, max_length }
    }
    
    pub fn quota_exceeded(queries_today: i64, daily_limit: i64) -> Self {
        Self::QuotaExceeded { queries_today, daily_limit }
    }
    
    pub fn invalid_tag_filter<S: Into<String>>(tag: S) -> Self {
        Self::InvalidTagFilter { tag: tag.into() }
    }
    
    pub fn index_corruption<S: Into<String>>(details: S) -> Self {
        Self::IndexCorruption { details: details.into() }
    }
}