//! Search tools. They build the same `SearchRequest` the REST search
//! endpoints receive and run it through the shared functions in
//! `services::search`, so results and validation match the web UI.

use http::request::Parts;
use rmcp::{
    handler::server::{tool::Extension, wrapper::Parameters},
    model::CallToolResult,
    tool, tool_router, ErrorData as McpError,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    errors::{search::SearchError, AppError},
    mcp::{format, ReadurMcp},
    models::{
        FacetItem, SearchFacetsResponse, SearchMode, SearchRequest, SearchResponse, UserRole,
    },
    services::search::{load_search_facets, run_basic_search, run_enhanced_search},
};

const DEFAULT_LIMIT: u32 = 25;
const MAX_LIMIT: u32 = 100;
const DEFAULT_SNIPPET_LENGTH: u32 = 200;
const MAX_SNIPPET_LENGTH: u32 = 1000;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchDocumentsParams {
    /// Text to search for in document content and OCR-extracted text (at
    /// least 2 characters, unless a filter is given).
    pub query: String,
    /// Maximum number of results (default 25, max 100).
    pub limit: Option<u32>,
    /// Number of results to skip, for pagination (default 0).
    pub offset: Option<u32>,
    /// Only documents with one of these MIME types, e.g. ["application/pdf"].
    pub mime_types: Option<Vec<String>>,
    /// Only documents carrying a label with one of these names.
    pub tags: Option<Vec<String>>,
    /// Only documents carrying one of these label IDs.
    pub label_ids: Option<Vec<Uuid>>,
    /// Only documents with this OCR status: pending, processing, completed or failed.
    pub ocr_status: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum McpSearchMode {
    /// Word matching (default).
    Simple,
    /// Exact phrase matching.
    Phrase,
    /// Finds words spelled close to the query (typos, OCR slips) in documents
    /// of any length, ranked by closeness; the cutoff is the user's fuzzy
    /// search threshold setting.
    Fuzzy,
    /// Operators (keywords in any case): `AND`/`&` or adjacency, `OR`/`|`,
    /// `NOT`/`!`/leading `-`, `( )` grouping, "quoted phrases", `word:*` or
    /// `word*` prefix, `<->`/`<N>` followed-by. AND binds tighter than OR.
    Boolean,
}

impl From<McpSearchMode> for SearchMode {
    fn from(mode: McpSearchMode) -> Self {
        match mode {
            McpSearchMode::Simple => SearchMode::Simple,
            McpSearchMode::Phrase => SearchMode::Phrase,
            McpSearchMode::Fuzzy => SearchMode::Fuzzy,
            McpSearchMode::Boolean => SearchMode::Boolean,
        }
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct EnhancedSearchParams {
    /// Text to search for in document content and OCR-extracted text (at
    /// least 2 characters, unless a filter is given).
    pub query: String,
    /// How to match the query: simple (default), phrase, fuzzy or boolean.
    pub search_mode: Option<McpSearchMode>,
    /// Include text snippets around each match (default true).
    pub include_snippets: Option<bool>,
    /// Snippet length in characters (default 200, max 1000).
    pub snippet_length: Option<u32>,
    /// Maximum number of results (default 25, max 100).
    pub limit: Option<u32>,
    /// Number of results to skip, for pagination (default 0).
    pub offset: Option<u32>,
    /// Only documents with one of these MIME types, e.g. ["application/pdf"].
    pub mime_types: Option<Vec<String>>,
    /// Only documents carrying a label with one of these names.
    pub tags: Option<Vec<String>>,
    /// Only documents carrying one of these label IDs.
    pub label_ids: Option<Vec<Uuid>>,
    /// Only documents with this OCR status: pending, processing, completed or failed.
    pub ocr_status: Option<String>,
}

/// Empty filter lists mean "no filter", as an omitted REST query parameter does.
fn non_empty<T>(values: Option<Vec<T>>) -> Option<Vec<T>> {
    values.filter(|v| !v.is_empty())
}

fn page(limit: Option<u32>, offset: Option<u32>) -> (i64, i64) {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    (limit as i64, offset.unwrap_or(0) as i64)
}

impl From<SearchDocumentsParams> for SearchRequest {
    fn from(p: SearchDocumentsParams) -> Self {
        let (limit, offset) = page(p.limit, p.offset);
        SearchRequest {
            query: p.query,
            limit: Some(limit),
            offset: Some(offset),
            mime_types: non_empty(p.mime_types),
            tags: non_empty(p.tags),
            label_ids: non_empty(p.label_ids),
            ocr_status: p.ocr_status,
            ..Default::default()
        }
    }
}

impl From<EnhancedSearchParams> for SearchRequest {
    fn from(p: EnhancedSearchParams) -> Self {
        let (limit, offset) = page(p.limit, p.offset);
        let snippet_length = p
            .snippet_length
            .unwrap_or(DEFAULT_SNIPPET_LENGTH)
            .clamp(1, MAX_SNIPPET_LENGTH);
        SearchRequest {
            query: p.query,
            limit: Some(limit),
            offset: Some(offset),
            search_mode: p.search_mode.map(Into::into),
            include_snippets: p.include_snippets,
            snippet_length: Some(snippet_length as i32),
            mime_types: non_empty(p.mime_types),
            tags: non_empty(p.tags),
            label_ids: non_empty(p.label_ids),
            ocr_status: p.ocr_status,
            ..Default::default()
        }
    }
}

/// Invalid input becomes a tool error the model can correct; anything else
/// is logged and reported without internal detail.
fn search_failure(tool: &str, error: SearchError) -> Result<CallToolResult, McpError> {
    if error.status_code().is_client_error() {
        let message = match error {
            SearchError::InvalidSyntax { details } => details,
            SearchError::QueryTooShort { .. } => format!(
                "{}, or give a filter such as mime_types to list matching documents.",
                error.user_message()
            ),
            other => other.user_message(),
        };
        Ok(format::tool_error(message))
    } else {
        Err(format::internal(tool, error))
    }
}

/// One line per hit (filename, id, score), then the snippets indented below.
fn format_results(heading: &str, request: &SearchRequest, response: &SearchResponse) -> String {
    let offset = request.offset.unwrap_or(0);
    let mut lines = vec![if response.documents.is_empty() {
        format!(
            "{heading} for \"{}\": no documents matched (total: {}).",
            request.query, response.total
        )
    } else {
        format!(
            "{heading} for \"{}\": showing {}-{} of {}.",
            request.query,
            offset + 1,
            offset + response.documents.len() as i64,
            response.total
        )
    }];
    for doc in &response.documents {
        let mut line = format!("- {} [{}]", doc.original_filename, doc.id);
        if let Some(rank) = doc.search_rank {
            line.push_str(&format!(" score {rank:.2}"));
        }
        line.push_str(&format!(
            " | {} | OCR {}",
            doc.mime_type,
            format::ocr_status(doc.ocr_status.as_deref())
        ));
        lines.push(line);
        for snippet in &doc.snippets {
            lines.push(format!("    \u{2026}{}\u{2026}", snippet.text.trim()));
        }
    }
    let shown = offset + response.documents.len() as i64;
    if shown < response.total {
        lines.push(format!("More results available: use offset {shown}."));
    }
    if !response.suggestions.is_empty() {
        lines.push(format!("Suggestions: {}", response.suggestions.join(", ")));
    }
    lines.join("\n")
}

#[tool_router(router = search_router, vis = "pub(crate)")]
impl ReadurMcp {
    #[tool(
        description = "Search documents by text in their content and OCR-extracted text, newest first. \
Optionally filter by MIME type, label name (tags), label ID or OCR status. Returns matching documents \
with IDs you can pass to get_document or get_document_ocr_text. Use enhanced_search for relevance \
ranking, match snippets and phrase/fuzzy/boolean matching.",
        annotations(
            title = "Search Documents",
            read_only_hint = true,
            open_world_hint = false
        )
    )]
    async fn search_documents(
        &self,
        Parameters(params): Parameters<SearchDocumentsParams>,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, McpError> {
        let user = Self::caller(&parts)?;
        let request = SearchRequest::from(params);
        match run_basic_search(&self.state, user, &request).await {
            Ok(response) => format::text_and_json(
                format_results("Search results", &request, &response),
                &response,
            ),
            Err(e) => search_failure("search_documents", e),
        }
    }

    #[tool(
        description = "Search documents ranked by relevance, with text snippets showing where the query \
matched and suggestions for refining the query. search_mode selects simple word matching (default), \
exact phrase, fuzzy (words spelled close to the query, within the user's fuzzy search threshold setting) \
or boolean matching. Boolean syntax: AND/& or adjacency, OR/|, \
NOT/!/leading -, parentheses for grouping, \"quoted phrases\", inv:* or inv* for prefixes, <-> or <N> for \
followed-by; AND binds tighter than OR (e.g. `(invoice OR receipt) 2024 -\"draft copy\"`). Supports the same \
filters as search_documents.",
        annotations(
            title = "Enhanced Search",
            read_only_hint = true,
            open_world_hint = false
        )
    )]
    async fn enhanced_search(
        &self,
        Parameters(params): Parameters<EnhancedSearchParams>,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, McpError> {
        let user = Self::caller(&parts)?;
        let request = SearchRequest::from(params);
        match run_enhanced_search(&self.state, user, &request).await {
            Ok(response) => format::text_and_json(
                format_results("Enhanced search", &request, &response),
                &response,
            ),
            Err(e) => search_failure("enhanced_search", e),
        }
    }

    #[tool(
        description = "List filter values for searching: the MIME types of the documents you can \
search (with counts), and every label you own with how many of those documents carry it. \
Labels with no documents are listed separately; filtering on them finds nothing.",
        annotations(
            title = "Get Search Facets",
            read_only_hint = true,
            open_world_hint = false
        )
    )]
    async fn get_search_facets(
        &self,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, McpError> {
        let user = Self::caller(&parts)?;
        let facets = load_search_facets(&self.state, user)
            .await
            .map_err(|e| format::internal("load search facets", e))?;
        format::text_and_json(facets_text(&facets, user.role == UserRole::Admin), &facets)
    }
}

/// Readable facets. MIME types follow the search's role scoping (an admin
/// searches every user's documents) while labels are always the caller's
/// own, so the text says which is which; unused labels are split out.
fn facets_text(facets: &SearchFacetsResponse, is_admin: bool) -> String {
    let line = |facet: &FacetItem| {
        let noun = if facet.count == 1 {
            "document"
        } else {
            "documents"
        };
        format!("  - {} ({} {})", facet.value, facet.count, noun)
    };
    let scope = if is_admin {
        "all users' documents"
    } else {
        "your documents"
    };
    let mut lines = vec![format!("MIME types ({scope}):")];
    if facets.mime_types.is_empty() {
        lines.push("  (none)".to_string());
    }
    lines.extend(facets.mime_types.iter().map(line));

    let (used, unused): (Vec<_>, Vec<_>) = facets.tags.iter().partition(|tag| tag.count > 0);
    lines.push("Your labels:".to_string());
    if used.is_empty() {
        lines.push("  (none on any document)".to_string());
    }
    lines.extend(used.into_iter().map(line));
    if !unused.is_empty() {
        let names: Vec<&str> = unused.iter().map(|tag| tag.value.as_str()).collect();
        lines.push(format!(
            "Labels not on any document yet: {}",
            names.join(", ")
        ));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(value: &str, count: i64) -> FacetItem {
        FacetItem {
            value: value.to_string(),
            count,
        }
    }

    #[test]
    fn unused_labels_are_listed_separately() {
        let facets = SearchFacetsResponse {
            mime_types: vec![item("application/pdf", 3)],
            tags: vec![item("Invoices", 2), item("Empty", 0)],
        };
        let text = facets_text(&facets, false);
        assert!(text.contains("  - Invoices (2 documents)"), "{text}");
        assert!(
            !text.contains("Empty (0"),
            "unused labels must not look like filter values: {text}"
        );
        assert!(
            text.contains("Labels not on any document yet: Empty"),
            "{text}"
        );
        assert!(text.contains("MIME types (your documents)"), "{text}");
    }

    #[test]
    fn admins_are_told_mime_types_cover_all_users() {
        let facets = SearchFacetsResponse {
            mime_types: vec![],
            tags: vec![item("Old", 0)],
        };
        let text = facets_text(&facets, true);
        assert!(text.contains("MIME types (all users' documents)"), "{text}");
        assert!(text.contains("(none on any document)"), "{text}");
    }
}
