use chrono::{DateTime, Utc};
use serde::{de::Error as _, Deserialize, Deserializer, Serialize};
use ts_rs::TS;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use super::responses::EnhancedDocumentResponse;

/// Maximum length for comma-separated query parameters (DoS protection)
const MAX_COMMA_SEPARATED_LENGTH: usize = 2000;
/// Maximum number of items in a comma-separated list (DoS protection)
const MAX_COMMA_SEPARATED_ITEMS: usize = 50;

/// Deserializes a comma-separated string into Vec<String>.
///
/// Handles: "a,b,c" -> vec!["a", "b", "c"]
/// - Trims whitespace from each value
/// - Filters empty values
/// - Returns None if result is empty or input exceeds limits
pub(crate) fn deserialize_comma_separated<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    Ok(opt.and_then(|s| {
        // DoS protection: reject overly long inputs
        if s.len() > MAX_COMMA_SEPARATED_LENGTH {
            return None;
        }

        let vec: Vec<String> = s
            .split(',')
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty())
            .take(MAX_COMMA_SEPARATED_ITEMS)
            .collect();

        if vec.is_empty() {
            None
        } else {
            Some(vec)
        }
    }))
}

/// Deserializes a comma-separated string into Vec<Uuid>.
///
/// Unlike `deserialize_comma_separated`, invalid input is an error rather than
/// being silently dropped: an unparseable UUID, an overly long value or too
/// many items all fail deserialization (surfaced as HTTP 400 by the extractor).
pub(crate) fn deserialize_comma_separated_uuids<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<Uuid>>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    let Some(s) = opt else {
        return Ok(None);
    };
    if s.len() > MAX_COMMA_SEPARATED_LENGTH {
        return Err(D::Error::custom("UUID list parameter is too long"));
    }

    let mut ids = Vec::new();
    for part in s.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        if ids.len() >= MAX_COMMA_SEPARATED_ITEMS {
            return Err(D::Error::custom("UUID list parameter has too many items"));
        }
        let id = Uuid::parse_str(part)
            .map_err(|_| D::Error::custom("UUID list parameter contains an invalid UUID"))?;
        ids.push(id);
    }

    Ok(if ids.is_empty() { None } else { Some(ids) })
}

/// Whitelisted columns that document lists and searches can be sorted by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum DocumentSortField {
    CreatedAt,
    UpdatedAt,
    Filename,
    FileSize,
    OcrStatus,
    MimeType,
    OcrConfidence,
}

impl DocumentSortField {
    /// Every sortable field, in declaration order.
    pub const ALL: [DocumentSortField; 7] = [
        DocumentSortField::CreatedAt,
        DocumentSortField::UpdatedAt,
        DocumentSortField::Filename,
        DocumentSortField::FileSize,
        DocumentSortField::OcrStatus,
        DocumentSortField::MimeType,
        DocumentSortField::OcrConfidence,
    ];

    /// The `documents` column this field sorts by. Only these static strings
    /// ever reach the ORDER BY clause.
    pub fn sql_column(&self) -> &'static str {
        match self {
            DocumentSortField::CreatedAt => "created_at",
            DocumentSortField::UpdatedAt => "updated_at",
            DocumentSortField::Filename => "original_filename",
            DocumentSortField::FileSize => "file_size",
            DocumentSortField::OcrStatus => "ocr_status",
            DocumentSortField::MimeType => "mime_type",
            DocumentSortField::OcrConfidence => "ocr_confidence",
        }
    }
}

/// Sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    Asc,
    Desc,
}

impl SortOrder {
    pub fn sql(&self) -> &'static str {
        match self {
            SortOrder::Asc => "ASC",
            SortOrder::Desc => "DESC",
        }
    }
}

/// OCR statuses accepted by the `ocr_status` filter.
pub const FILTERABLE_OCR_STATUSES: [&str; 4] = ["pending", "processing", "completed", "failed"];

/// Filters shared by the document list and search endpoints.
///
/// All values are passed to SQL as bind parameters only.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct DocumentFilters {
    /// OCR status: pending (also matches documents with no status), processing, completed, failed
    pub ocr_status: Option<String>,
    /// Match any of these MIME types
    pub mime_types: Option<Vec<String>>,
    /// Match documents carrying any of these label IDs
    pub label_ids: Option<Vec<Uuid>>,
    /// Match documents carrying any label with one of these names (legacy `tags` filter)
    pub tags: Option<Vec<String>>,
    /// Match documents ingested from any of these sources
    pub source_ids: Option<Vec<Uuid>>,
    /// Match any of these source types; `direct_upload` also matches documents without a source
    pub source_types: Option<Vec<String>>,
    /// Only documents created at or after this instant
    pub created_from: Option<DateTime<Utc>>,
    /// Only documents created at or before this instant
    pub created_to: Option<DateTime<Utc>>,
}

impl DocumentFilters {
    /// Validates filter values that have a closed set of allowed values.
    pub fn validate(&self) -> Result<(), String> {
        if let Some(status) = self.ocr_status.as_deref() {
            if !FILTERABLE_OCR_STATUSES.contains(&status) {
                return Err(format!(
                    "Invalid ocr_status filter; expected one of: {}",
                    FILTERABLE_OCR_STATUSES.join(", ")
                ));
            }
        }
        if let (Some(from), Some(to)) = (self.created_from, self.created_to) {
            if from > to {
                return Err("created_from must not be later than created_to".to_string());
            }
        }
        Ok(())
    }

    /// True when at least one filter is set.
    pub fn is_active(&self) -> bool {
        fn non_empty<T>(v: &Option<Vec<T>>) -> bool {
            v.as_ref().is_some_and(|v| !v.is_empty())
        }
        self.ocr_status.is_some()
            || non_empty(&self.mime_types)
            || non_empty(&self.label_ids)
            || non_empty(&self.tags)
            || non_empty(&self.source_ids)
            || non_empty(&self.source_types)
            || self.created_from.is_some()
            || self.created_to.is_some()
    }
}

#[derive(Debug, Default, Serialize, Deserialize, ToSchema, TS, IntoParams)]
#[ts(export, optional_fields)]
pub struct SearchRequest {
    /// Search query text (searches both document content and OCR-extracted text)
    #[serde(default)]
    #[ts(optional = nullable)]
    pub query: String,
    /// Filter by specific tags (label names)
    #[serde(default, deserialize_with = "deserialize_comma_separated")]
    #[ts(optional = nullable, type = "string")]
    pub tags: Option<Vec<String>>,
    /// Filter by MIME types (e.g., "application/pdf", "image/png")
    #[serde(default, deserialize_with = "deserialize_comma_separated")]
    #[ts(optional = nullable, type = "string")]
    pub mime_types: Option<Vec<String>>,
    /// Maximum number of results to return (default: 25)
    pub limit: Option<i64>,
    /// Number of results to skip for pagination (default: 0)
    pub offset: Option<i64>,
    /// Whether to include text snippets with search matches (default: true)
    pub include_snippets: Option<bool>,
    /// Length of text snippets in characters (default: 200)
    pub snippet_length: Option<i32>,
    /// Search algorithm to use (default: simple)
    pub search_mode: Option<SearchMode>,
    /// Column to sort by. When omitted, enhanced search orders by relevance and
    /// basic search orders newest first.
    pub sort_by: Option<DocumentSortField>,
    /// Sort direction (default: desc)
    pub sort_order: Option<SortOrder>,
    /// Filter by OCR status: pending, processing, completed, failed
    pub ocr_status: Option<String>,
    /// Filter by label IDs (comma-separated UUIDs, matches any)
    #[serde(default, deserialize_with = "deserialize_comma_separated_uuids")]
    #[ts(optional = nullable, type = "string")]
    pub label_ids: Option<Vec<Uuid>>,
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

impl SearchRequest {
    /// Collects the filter parameters of this request.
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

#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub enum SearchMode {
    /// Simple text search with basic word matching
    #[serde(rename = "simple")]
    Simple,
    /// Exact phrase matching
    #[serde(rename = "phrase")]
    Phrase,
    /// Fuzzy search using similarity matching (good for typos and partial matches)
    #[serde(rename = "fuzzy")]
    Fuzzy,
    /// Boolean search with AND, OR, NOT operators
    #[serde(rename = "boolean")]
    Boolean,
}

impl Default for SearchMode {
    fn default() -> Self {
        SearchMode::Simple
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct SearchResponse {
    pub documents: Vec<EnhancedDocumentResponse>,
    pub total: i64,
    pub query_time_ms: u64,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct FacetItem {
    pub value: String,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct SearchFacetsResponse {
    pub mime_types: Vec<FacetItem>,
    pub tags: Vec<FacetItem>,
}

/// Number of search matches created in one UTC calendar month
/// (`GET /api/search/timeline`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct MonthCount {
    /// UTC calendar month, `YYYY-MM`
    #[schema(example = "2026-09")]
    pub month: String,
    pub count: i64,
}

#[cfg(test)]
mod sort_filter_tests {
    use super::*;
    use axum::extract::Query;
    use axum::http::Uri;

    fn parse(qs: &str) -> Result<SearchRequest, String> {
        let uri: Uri = format!("/api/search?{}", qs).parse().unwrap();
        Query::<SearchRequest>::try_from_uri(&uri)
            .map(|q| q.0)
            .map_err(|e| e.to_string())
    }

    #[test]
    fn parses_every_sort_field_and_order() {
        let names = [
            ("created_at", DocumentSortField::CreatedAt),
            ("updated_at", DocumentSortField::UpdatedAt),
            ("filename", DocumentSortField::Filename),
            ("file_size", DocumentSortField::FileSize),
            ("ocr_status", DocumentSortField::OcrStatus),
            ("mime_type", DocumentSortField::MimeType),
            ("ocr_confidence", DocumentSortField::OcrConfidence),
        ];
        assert_eq!(names.len(), DocumentSortField::ALL.len());
        for (name, field) in names {
            for (dir, order) in [("asc", SortOrder::Asc), ("desc", SortOrder::Desc)] {
                let req = parse(&format!("query=ab&sort_by={}&sort_order={}", name, dir)).unwrap();
                assert_eq!(req.sort_by, Some(field));
                assert_eq!(req.sort_order, Some(order));
            }
        }
    }

    #[test]
    fn rejects_unknown_sort_by() {
        assert!(parse("query=ab&sort_by=file_path").is_err());
        assert!(parse("query=ab&sort_by=created_at%3B%20DROP%20TABLE%20documents").is_err());
        assert!(parse("query=ab&sort_by=CREATED_AT").is_err());
    }

    #[test]
    fn rejects_unknown_sort_order() {
        assert!(parse("query=ab&sort_order=sideways").is_err());
        assert!(parse("query=ab&sort_order=ASC").is_err());
    }

    #[test]
    fn parses_uuid_lists() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let req = parse(&format!("label_ids={},%20{}&source_ids={}", a, b, b)).unwrap();
        assert_eq!(req.label_ids, Some(vec![a, b]));
        assert_eq!(req.source_ids, Some(vec![b]));
        assert!(req.filters().is_active());
    }

    #[test]
    fn rejects_invalid_uuid_in_lists() {
        let a = Uuid::new_v4();
        assert!(parse(&format!("label_ids={},not-a-uuid", a)).is_err());
        assert!(parse("source_ids=123").is_err());
    }

    #[test]
    fn rejects_oversized_uuid_lists() {
        let many: Vec<String> = (0..MAX_COMMA_SEPARATED_ITEMS + 1)
            .map(|_| Uuid::new_v4().to_string())
            .collect();
        assert!(parse(&format!("label_ids={}", many.join(","))).is_err());
    }

    #[test]
    fn parses_filters_and_dates() {
        let req = parse(
            "source_types=direct_upload,webdav&mime_types=application/pdf&ocr_status=completed\
             &created_from=2026-01-01T00:00:00Z&created_to=2026-02-01T00:00:00Z",
        )
        .unwrap();
        let filters = req.filters();
        assert_eq!(
            filters.source_types,
            Some(vec!["direct_upload".to_string(), "webdav".to_string()])
        );
        assert_eq!(
            filters.mime_types,
            Some(vec!["application/pdf".to_string()])
        );
        assert_eq!(
            filters.created_from.unwrap().to_rfc3339(),
            "2026-01-01T00:00:00+00:00"
        );
        assert!(filters.validate().is_ok());
        assert!(parse("created_from=yesterday").is_err());
    }

    #[test]
    fn validate_rejects_unknown_ocr_status_and_inverted_range() {
        let bad_status = DocumentFilters {
            ocr_status: Some("done".to_string()),
            ..Default::default()
        };
        assert!(bad_status.validate().is_err());
        for status in FILTERABLE_OCR_STATUSES {
            let ok = DocumentFilters {
                ocr_status: Some(status.to_string()),
                ..Default::default()
            };
            assert!(ok.validate().is_ok());
        }
        let inverted = DocumentFilters {
            created_from: Some("2026-02-01T00:00:00Z".parse().unwrap()),
            created_to: Some("2026-01-01T00:00:00Z".parse().unwrap()),
            ..Default::default()
        };
        assert!(inverted.validate().is_err());
    }

    #[test]
    fn empty_request_has_no_filters() {
        let req = parse("query=hello").unwrap();
        assert!(!req.filters().is_active());
        assert_eq!(req.sort_by, None);
    }
}
