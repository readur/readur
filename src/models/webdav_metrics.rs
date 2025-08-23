use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::time::Duration;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use utoipa::{ToSchema, IntoParams};

/// WebDAV operation types for categorizing different kinds of operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WebDAVOperationType {
    Discovery,
    Download,
    MetadataFetch,
    ConnectionTest,
    Validation,
    FullSync,
}

impl std::fmt::Display for WebDAVOperationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Discovery => write!(f, "discovery"),
            Self::Download => write!(f, "download"),
            Self::MetadataFetch => write!(f, "metadata_fetch"),
            Self::ConnectionTest => write!(f, "connection_test"),
            Self::Validation => write!(f, "validation"),
            Self::FullSync => write!(f, "full_sync"),
        }
    }
}

/// WebDAV request types (HTTP methods)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum WebDAVRequestType {
    #[serde(rename = "PROPFIND")]
    PropFind,
    #[serde(rename = "GET")]
    Get,
    #[serde(rename = "HEAD")]
    Head,
    #[serde(rename = "OPTIONS")]
    Options,
    #[serde(rename = "POST")]
    Post,
    #[serde(rename = "PUT")]
    Put,
    #[serde(rename = "DELETE")]
    Delete,
}

impl std::fmt::Display for WebDAVRequestType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PropFind => write!(f, "PROPFIND"),
            Self::Get => write!(f, "GET"),
            Self::Head => write!(f, "HEAD"),
            Self::Options => write!(f, "OPTIONS"),
            Self::Post => write!(f, "POST"),
            Self::Put => write!(f, "PUT"),
            Self::Delete => write!(f, "DELETE"),
        }
    }
}

/// Status of a WebDAV sync session
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WebDAVSyncStatus {
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for WebDAVSyncStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InProgress => write!(f, "in_progress"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Request to create a new WebDAV sync session
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateWebDAVSyncSession {
    pub user_id: Uuid,
    pub source_id: Option<Uuid>,
    pub sync_type: String,
    pub root_path: String,
    pub max_depth: Option<i32>,
}

/// WebDAV sync session record
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct WebDAVSyncSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub source_id: Option<Uuid>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
    pub sync_type: String,
    pub root_path: String,
    pub max_depth: Option<i32>,
    pub directories_discovered: i32,
    pub directories_processed: i32,
    pub files_discovered: i32,
    pub files_processed: i32,
    pub total_bytes_discovered: i64,
    pub total_bytes_processed: i64,
    pub avg_file_size_bytes: Option<i64>,
    pub processing_rate_files_per_sec: Option<f64>,
    pub total_http_requests: i32,
    pub successful_requests: i32,
    pub failed_requests: i32,
    pub retry_attempts: i32,
    pub directories_skipped: i32,
    pub files_skipped: i32,
    pub skip_reasons: Option<serde_json::Value>,
    pub status: String,
    pub final_error_message: Option<String>,
    pub slowest_operation_ms: Option<i64>,
    pub slowest_operation_path: Option<String>,
    pub network_time_ms: Option<i64>,
    pub processing_time_ms: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to create a directory metrics record
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateWebDAVDirectoryMetric {
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub source_id: Option<Uuid>,
    pub directory_path: String,
    pub directory_depth: i32,
    pub parent_directory_path: Option<String>,
}

/// WebDAV directory scan metrics
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct WebDAVDirectoryMetric {
    pub id: Uuid,
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub source_id: Option<Uuid>,
    pub directory_path: String,
    pub directory_depth: i32,
    pub parent_directory_path: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub scan_duration_ms: Option<i64>,
    pub files_found: i32,
    pub subdirectories_found: i32,
    pub total_size_bytes: i64,
    pub files_processed: i32,
    pub files_skipped: i32,
    pub files_failed: i32,
    pub http_requests_made: i32,
    pub propfind_requests: i32,
    pub get_requests: i32,
    pub errors_encountered: i32,
    pub error_types: Option<serde_json::Value>,
    pub warnings_count: i32,
    pub avg_response_time_ms: Option<f64>,
    pub slowest_request_ms: Option<i64>,
    pub fastest_request_ms: Option<i64>,
    pub etag_matches: i32,
    pub etag_mismatches: i32,
    pub cache_hits: i32,
    pub cache_misses: i32,
    pub status: String,
    pub skip_reason: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Request to create an HTTP request metric record
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateWebDAVRequestMetric {
    pub session_id: Option<Uuid>,
    pub directory_metric_id: Option<Uuid>,
    pub user_id: Uuid,
    pub source_id: Option<Uuid>,
    pub request_type: WebDAVRequestType,
    pub operation_type: WebDAVOperationType,
    pub target_path: String,
    pub duration_ms: i64,
    pub request_size_bytes: Option<i64>,
    pub response_size_bytes: Option<i64>,
    pub http_status_code: Option<i32>,
    pub dns_lookup_ms: Option<i64>,
    pub tcp_connect_ms: Option<i64>,
    pub tls_handshake_ms: Option<i64>,
    pub time_to_first_byte_ms: Option<i64>,
    pub success: bool,
    pub retry_attempt: i32,
    pub error_type: Option<String>,
    pub error_message: Option<String>,
    pub server_header: Option<String>,
    pub dav_header: Option<String>,
    pub etag_value: Option<String>,
    pub last_modified: Option<DateTime<Utc>>,
    pub content_type: Option<String>,
    pub remote_ip: Option<String>,
    pub user_agent: Option<String>,
}

/// WebDAV HTTP request metrics
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct WebDAVRequestMetric {
    pub id: Uuid,
    pub session_id: Option<Uuid>,
    pub directory_metric_id: Option<Uuid>,
    pub user_id: Uuid,
    pub source_id: Option<Uuid>,
    pub request_type: String,
    pub operation_type: String,
    pub target_path: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: i64,
    pub request_size_bytes: Option<i64>,
    pub response_size_bytes: Option<i64>,
    pub http_status_code: Option<i32>,
    pub dns_lookup_ms: Option<i64>,
    pub tcp_connect_ms: Option<i64>,
    pub tls_handshake_ms: Option<i64>,
    pub time_to_first_byte_ms: Option<i64>,
    pub success: bool,
    pub retry_attempt: i32,
    pub error_type: Option<String>,
    pub error_message: Option<String>,
    pub server_header: Option<String>,
    pub dav_header: Option<String>,
    pub etag_value: Option<String>,
    pub last_modified: Option<DateTime<Utc>>,
    pub content_type: Option<String>,
    pub remote_ip: Option<String>,
    pub user_agent: Option<String>,
}

/// Summary metrics for WebDAV operations
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct WebDAVMetricsSummary {
    pub total_sessions: i32,
    pub successful_sessions: i32,
    pub failed_sessions: i32,
    pub total_files_processed: i64,
    pub total_bytes_processed: i64,
    pub avg_session_duration_sec: f64,
    pub avg_processing_rate: f64,
    pub total_http_requests: i64,
    pub request_success_rate: f64,
    pub avg_request_duration_ms: f64,
    pub common_error_types: serde_json::Value,
}

/// Request parameters for querying WebDAV metrics
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct WebDAVMetricsQuery {
    pub user_id: Option<Uuid>,
    pub source_id: Option<Uuid>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

/// Performance insights for WebDAV operations
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WebDAVPerformanceInsights {
    pub session_id: Uuid,
    pub avg_directory_scan_time_ms: f64,
    pub slowest_directories: Vec<SlowDirectoryInfo>,
    pub request_distribution: RequestTypeDistribution,
    pub error_analysis: ErrorAnalysis,
    pub performance_trends: PerformanceTrends,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SlowDirectoryInfo {
    pub path: String,
    pub scan_duration_ms: i64,
    pub files_count: i32,
    pub size_bytes: i64,
    pub error_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RequestTypeDistribution {
    pub propfind_count: i32,
    pub get_count: i32,
    pub head_count: i32,
    pub options_count: i32,
    pub total_count: i32,
    pub avg_propfind_duration_ms: f64,
    pub avg_get_duration_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorAnalysis {
    pub total_errors: i32,
    pub network_errors: i32,
    pub auth_errors: i32,
    pub timeout_errors: i32,
    pub server_errors: i32,
    pub most_problematic_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PerformanceTrends {
    pub requests_per_minute: Vec<f64>,
    pub avg_response_time_trend: Vec<f64>,
    pub error_rate_trend: Vec<f64>,
    pub throughput_mbps_trend: Vec<f64>,
}

/// Update request for WebDAV sync session
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateWebDAVSyncSession {
    pub directories_discovered: Option<i32>,
    pub directories_processed: Option<i32>,
    pub files_discovered: Option<i32>,
    pub files_processed: Option<i32>,
    pub total_bytes_discovered: Option<i64>,
    pub total_bytes_processed: Option<i64>,
    pub directories_skipped: Option<i32>,
    pub files_skipped: Option<i32>,
    pub skip_reasons: Option<serde_json::Value>,
    pub status: Option<WebDAVSyncStatus>,
    pub final_error_message: Option<String>,
}

/// Update request for WebDAV directory metric
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateWebDAVDirectoryMetric {
    pub files_found: Option<i32>,
    pub subdirectories_found: Option<i32>,
    pub total_size_bytes: Option<i64>,
    pub files_processed: Option<i32>,
    pub files_skipped: Option<i32>,
    pub files_failed: Option<i32>,
    pub http_requests_made: Option<i32>,
    pub propfind_requests: Option<i32>,
    pub get_requests: Option<i32>,
    pub errors_encountered: Option<i32>,
    pub error_types: Option<serde_json::Value>,
    pub warnings_count: Option<i32>,
    pub etag_matches: Option<i32>,
    pub etag_mismatches: Option<i32>,
    pub cache_hits: Option<i32>,
    pub cache_misses: Option<i32>,
    pub status: Option<String>,
    pub skip_reason: Option<String>,
    pub error_message: Option<String>,
}