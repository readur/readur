use serde::{Deserialize, Serialize};
use utoipa::{ToSchema, IntoParams};

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct PaginationQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub ocr_status: Option<String>,
}

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct FailedDocumentsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub stage: Option<String>,  // 'ocr', 'ingestion', 'validation', etc.
    pub reason: Option<String>, // 'duplicate_content', 'low_ocr_confidence', etc.
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct BulkDeleteRequest {
    pub document_ids: Vec<uuid::Uuid>,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct DeleteLowConfidenceRequest {
    pub max_confidence: f32,
    pub preview_only: Option<bool>,
}

#[derive(Deserialize, ToSchema)]
pub struct RetryOcrRequest {
    pub language: Option<String>,
    pub languages: Option<Vec<String>>,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct DocumentUploadResponse {
    pub id: uuid::Uuid,
    pub filename: String,
    pub file_size: i64,
    pub mime_type: String,
    pub status: String,
    pub message: String,
}

#[derive(Serialize, ToSchema)]
pub struct BulkDeleteResponse {
    pub deleted_count: i64,
    pub failed_count: i64,
    pub deleted_documents: Vec<uuid::Uuid>,
    pub failed_documents: Vec<uuid::Uuid>,
    pub total_files_deleted: i64,
    pub total_files_failed: i64,
}

#[derive(Serialize, ToSchema)]
pub struct DocumentDebugInfo {
    pub document_id: uuid::Uuid,
    pub filename: String,
    pub file_path: String,
    pub file_size: i64,
    pub mime_type: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub ocr_status: Option<String>,
    pub ocr_confidence: Option<f32>,
    pub ocr_word_count: Option<i32>,
    pub processing_steps: Vec<String>,
    pub file_exists: bool,
    pub readable: bool,
    pub permissions: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct DocumentPaginationInfo {
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
    pub has_more: bool,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct PaginatedDocumentsResponse {
    pub documents: Vec<crate::models::DocumentResponse>,
    pub pagination: DocumentPaginationInfo,
}

impl Default for PaginationQuery {
    fn default() -> Self {
        Self {
            limit: Some(25),
            offset: Some(0),
            ocr_status: None,
        }
    }
}

impl Default for FailedDocumentsQuery {
    fn default() -> Self {
        Self {
            limit: Some(25),
            offset: Some(0),
            stage: None,
            reason: None,
        }
    }
}