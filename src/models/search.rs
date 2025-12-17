use serde::{Deserialize, Deserializer, Serialize};
use utoipa::{ToSchema, IntoParams};

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
fn deserialize_comma_separated<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
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

        if vec.is_empty() { None } else { Some(vec) }
    }))
}

#[derive(Debug, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct SearchRequest {
    /// Search query text (searches both document content and OCR-extracted text)
    #[serde(default)]
    pub query: String,
    /// Filter by specific tags (label names)
    #[serde(default, deserialize_with = "deserialize_comma_separated")]
    pub tags: Option<Vec<String>>,
    /// Filter by MIME types (e.g., "application/pdf", "image/png")
    #[serde(default, deserialize_with = "deserialize_comma_separated")]
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
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
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


#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SearchResponse {
    pub documents: Vec<EnhancedDocumentResponse>,
    pub total: i64,
    pub query_time_ms: u64,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FacetItem {
    pub value: String,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SearchFacetsResponse {
    pub mime_types: Vec<FacetItem>,
    pub tags: Vec<FacetItem>,
}
