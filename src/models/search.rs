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
    /// Approximate length of text snippets in bytes, 1 to 2000 (default: 200)
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

#[derive(Debug, Default, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub enum SearchMode {
    /// Simple text search with basic word matching
    #[default]
    #[serde(rename = "simple")]
    Simple,
    /// Exact phrase matching
    #[serde(rename = "phrase")]
    Phrase,
    /// Word-level fuzzy matching: finds documents containing a word (or run
    /// of words) spelled like the query, ranked by how close it is. The
    /// cutoff is the user's fuzzy search threshold; see `FuzzyThreshold`.
    #[serde(rename = "fuzzy")]
    Fuzzy,
    /// Boolean search: `AND` / `&` / adjacency, `OR` / `|`, `NOT` / `!` /
    /// leading `-` (keywords in any case), `( )` grouping, "quoted phrases",
    /// `word:*` / `word*` prefixes and `<->` / `<N>` followed-by. Parsed by
    /// `db::documents::boolean_to_tsquery`; malformed input never errors.
    #[serde(rename = "boolean")]
    Boolean,
}

/// The cutoff of fuzzy search: the minimum pg_trgm `word_similarity` between
/// the query and the closest stretch of a document's text. 1.0 only accepts
/// the query's exact spelling; lower values let more spelling differences
/// through. Comes from the user's `fuzzy_search_threshold` setting.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FuzzyThreshold(f32);

impl FuzzyThreshold {
    /// Used when the user has no setting (or an unusable one). Admits a
    /// one-letter slip in a medium-length word ("invoce" scores about 0.57
    /// against "invoice") while keeping unrelated text out (a long English
    /// document scores about 0.3 against an absent word by chance).
    pub const DEFAULT: f32 = 0.5;
    /// The lowest cutoff applied. Below this nearly every long document
    /// shares enough letter triples with any query to match, so smaller
    /// settings are raised to it.
    pub const MIN: f32 = 0.1;
    /// The highest cutoff: the query's exact spelling.
    pub const MAX: f32 = 1.0;

    /// The cutoff for a stored setting. A missing, non-finite or out-of-range
    /// value (the settings API only accepts 0.0..=1.0) falls back to
    /// `DEFAULT`; values below `MIN` are raised to it.
    pub fn from_setting(setting: Option<f32>) -> Self {
        match setting {
            Some(value) if value.is_finite() && (0.0..=Self::MAX).contains(&value) => {
                Self(value.max(Self::MIN))
            }
            _ => Self(Self::DEFAULT),
        }
    }

    pub fn value(self) -> f32 {
        self.0
    }

    /// The text `pg_trgm.word_similarity_threshold` is set to.
    pub fn setting(self) -> String {
        self.0.to_string()
    }

    /// The cutoff as Postgres holds it once `setting` is applied: a double
    /// parsed from that text (not the widened `f32`, which differs in the
    /// low digits, e.g. 0.57).
    pub fn cutoff(self) -> f64 {
        self.setting().parse().unwrap_or(f64::from(self.0))
    }
}

impl Default for FuzzyThreshold {
    fn default() -> Self {
        Self(Self::DEFAULT)
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

#[cfg(test)]
mod fuzzy_threshold_tests {
    use super::FuzzyThreshold;

    #[test]
    fn missing_or_unusable_settings_fall_back_to_the_default() {
        for setting in [
            None,
            Some(f32::NAN),
            Some(f32::INFINITY),
            Some(-0.1),
            Some(1.01),
        ] {
            assert_eq!(
                FuzzyThreshold::from_setting(setting).value(),
                FuzzyThreshold::DEFAULT,
                "{setting:?}"
            );
        }
        assert_eq!(FuzzyThreshold::default().value(), FuzzyThreshold::DEFAULT);
    }

    #[test]
    fn valid_settings_are_kept_and_tiny_ones_raised_to_the_floor() {
        assert_eq!(FuzzyThreshold::from_setting(Some(0.7)).value(), 0.7);
        assert_eq!(FuzzyThreshold::from_setting(Some(1.0)).value(), 1.0);
        assert_eq!(
            FuzzyThreshold::from_setting(Some(FuzzyThreshold::MIN)).value(),
            FuzzyThreshold::MIN
        );
        for tiny in [0.0, 0.05] {
            assert_eq!(
                FuzzyThreshold::from_setting(Some(tiny)).value(),
                FuzzyThreshold::MIN,
                "{tiny}"
            );
        }
    }
}
