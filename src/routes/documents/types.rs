use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{ToSchema, IntoParams};
use uuid::Uuid;
use ts_rs::TS;

use crate::models::search::{deserialize_comma_separated, deserialize_comma_separated_uuids};
use crate::models::{DocumentFilters, DocumentSortField, SortOrder};

#[derive(Debug, Deserialize, ToSchema, TS, IntoParams)]
#[ts(export, optional_fields)]
pub struct PaginationQuery {
    /// Maximum number of documents to return (default: 25)
    pub limit: Option<i64>,
    /// Number of documents to skip (default: 0)
    pub offset: Option<i64>,
    /// Column to sort by (default: created_at)
    pub sort_by: Option<DocumentSortField>,
    /// Sort direction (default: desc)
    pub sort_order: Option<SortOrder>,
    /// Filter by OCR status: pending, processing, completed, failed
    pub ocr_status: Option<String>,
    /// Filter by MIME types (comma-separated, matches any)
    #[serde(default, deserialize_with = "deserialize_comma_separated")]
    #[ts(optional = nullable, type = "string")]
    pub mime_types: Option<Vec<String>>,
    /// Filter by label IDs (comma-separated UUIDs, matches any)
    #[serde(default, deserialize_with = "deserialize_comma_separated_uuids")]
    #[ts(optional = nullable, type = "string")]
    pub label_ids: Option<Vec<Uuid>>,
    /// Filter by label names (comma-separated, matches any)
    #[serde(default, deserialize_with = "deserialize_comma_separated")]
    #[ts(optional = nullable, type = "string")]
    pub tags: Option<Vec<String>>,
    /// Filter by source IDs (comma-separated UUIDs, matches any)
    #[serde(default, deserialize_with = "deserialize_comma_separated_uuids")]
    #[ts(optional = nullable, type = "string")]
    pub source_ids: Option<Vec<Uuid>>,
    /// Filter by source types (comma-separated); `direct_upload` also matches documents without a source
    #[serde(default, deserialize_with = "deserialize_comma_separated")]
    #[ts(optional = nullable, type = "string")]
    pub source_types: Option<Vec<String>>,
    /// Only documents created at or after this RFC 3339 timestamp
    pub created_from: Option<DateTime<Utc>>,
    /// Only documents created at or before this RFC 3339 timestamp
    pub created_to: Option<DateTime<Utc>>,
}

impl PaginationQuery {
    /// Collects the filter parameters of this query.
    pub fn filters(&self) -> DocumentFilters {
        DocumentFilters {
            ocr_status: self.ocr_status.clone(),
            mime_types: self.mime_types.clone(),
            label_ids: self.label_ids.clone(),
            tags: self.tags.clone(),
            source_ids: self.source_ids.clone(),
            source_types: self.source_types.clone(),
            created_from: self.created_from,
            created_to: self.created_to,
        }
    }
}

#[derive(Deserialize, ToSchema, TS, IntoParams)]
#[ts(export, optional_fields)]
pub struct FailedDocumentsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub stage: Option<String>,  // 'ocr', 'ingestion', 'validation', etc.
    pub reason: Option<String>, // 'duplicate_content', 'low_ocr_confidence', etc.
}

#[derive(Deserialize, Serialize, ToSchema, TS)]
#[ts(export, optional_fields)]
pub struct BulkDeleteRequest {
    pub document_ids: Vec<uuid::Uuid>,
}

#[derive(Deserialize, Serialize, ToSchema, TS)]
#[ts(export, optional_fields)]
pub struct DeleteLowConfidenceRequest {
    pub max_confidence: f32,
    pub preview_only: Option<bool>,
}

#[derive(Deserialize, ToSchema, TS)]
#[ts(export, optional_fields)]
pub struct RetryOcrRequest {
    pub language: Option<String>,
    pub languages: Option<Vec<String>>,
}

#[derive(Deserialize, Serialize, ToSchema, TS)]
#[ts(export)]
pub struct DocumentUploadResponse {
    pub id: uuid::Uuid,
    pub filename: String,
    pub file_size: i64,
    pub mime_type: String,
    pub status: String,
    pub message: String,
}

#[derive(Serialize, ToSchema, TS)]
#[ts(export)]
pub struct BulkDeleteResponse {
    pub deleted_count: i64,
    pub failed_count: i64,
    pub deleted_documents: Vec<uuid::Uuid>,
    pub failed_documents: Vec<uuid::Uuid>,
    pub total_files_deleted: i64,
    pub total_files_failed: i64,
}

#[derive(Serialize, ToSchema, TS)]
#[ts(export)]
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
    pub user_settings: Option<crate::models::SettingsResponse>,
}

#[derive(Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct DocumentPaginationInfo {
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
    pub has_more: bool,
}

#[derive(Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct PaginatedDocumentsResponse {
    pub documents: Vec<crate::models::DocumentResponse>,
    pub pagination: DocumentPaginationInfo,
}

impl Default for PaginationQuery {
    fn default() -> Self {
        Self {
            limit: Some(25),
            offset: Some(0),
            sort_by: None,
            sort_order: None,
            ocr_status: None,
            mime_types: None,
            label_ids: None,
            tags: None,
            source_ids: None,
            source_types: None,
            created_from: None,
            created_to: None,
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::Query;
    use axum::http::Uri;

    fn parse(qs: &str) -> Result<PaginationQuery, String> {
        let uri: Uri = format!("/api/documents?{}", qs).parse().unwrap();
        Query::<PaginationQuery>::try_from_uri(&uri)
            .map(|q| q.0)
            .map_err(|e| e.to_string())
    }

    #[test]
    fn parses_sort_and_filters() {
        let label = Uuid::new_v4();
        let q = parse(&format!(
            "limit=2&offset=4&sort_by=filename&sort_order=asc&label_ids={}&source_types=direct_upload&ocr_status=failed",
            label
        ))
        .unwrap();
        assert_eq!(q.limit, Some(2));
        assert_eq!(q.offset, Some(4));
        assert_eq!(q.sort_by, Some(DocumentSortField::Filename));
        assert_eq!(q.sort_order, Some(SortOrder::Asc));
        let filters = q.filters();
        assert_eq!(filters.label_ids, Some(vec![label]));
        assert_eq!(filters.source_types, Some(vec!["direct_upload".to_string()]));
        assert!(filters.validate().is_ok());
    }

    #[test]
    fn rejects_unknown_sort_values_and_bad_uuids() {
        assert!(parse("sort_by=file_path").is_err());
        assert!(parse("sort_by=id").is_err());
        assert!(parse("sort_order=up").is_err());
        assert!(parse("label_ids=nope").is_err());
        assert!(parse("source_ids=00000000-0000-0000-0000-00000000000g").is_err());
    }

    #[test]
    fn plain_pagination_still_parses() {
        let q = parse("limit=10&offset=0").unwrap();
        assert!(!q.filters().is_active());
        assert_eq!(q.sort_by, None);
    }
}