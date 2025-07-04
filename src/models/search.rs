use serde::{Deserialize, Serialize};
use utoipa::{ToSchema, IntoParams};

use super::responses::EnhancedDocumentResponse;

#[derive(Debug, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct SearchRequest {
    /// Search query text (searches both document content and OCR-extracted text)
    pub query: String,
    /// Filter by specific tags
    pub tags: Option<Vec<String>>,
    /// Filter by MIME types (e.g., "application/pdf", "image/png")
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
    /// List of matching documents with enhanced metadata and snippets
    pub documents: Vec<EnhancedDocumentResponse>,
    /// Total number of documents matching the search criteria
    pub total: i64,
    /// Time taken to execute the search in milliseconds
    pub query_time_ms: u64,
    /// Search suggestions for query improvement
    pub suggestions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FacetItem {
    /// The facet value (e.g., mime type or tag)
    pub value: String,
    /// Number of documents with this value
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SearchFacetsResponse {
    /// MIME type facets with counts
    pub mime_types: Vec<FacetItem>,
    /// Tag facets with counts
    pub tags: Vec<FacetItem>,
}