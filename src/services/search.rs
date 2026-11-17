//! Document search shared by the REST search routes and the MCP search tools:
//! request validation, basic and enhanced search, and facets. Both callers
//! go through these functions, so results and validation match.

use crate::{
    errors::search::SearchError,
    models::{
        EnhancedDocumentResponse, FuzzyThreshold, MonthCount, SearchFacetsResponse, SearchMode,
        SearchRequest, SearchResponse, User,
    },
    AppState,
};

/// Basic (simple-mode) search for `user`: validation, total count, the page
/// of results and their labels. Backs `GET /api/search` and the MCP
/// `search_documents` tool.
pub async fn run_basic_search(
    state: &AppState,
    user: &User,
    search_request: &SearchRequest,
) -> Result<SearchResponse, SearchError> {
    validate_search_request(search_request)?;

    // Get total count (without pagination) for proper pagination support. The
    // list below always matches in simple mode, so the count does too.
    let total = state
        .db
        .count_basic_search_documents(user.id, user.role, search_request)
        .await
        .map_err(|e| SearchError::index_unavailable(format!("Count failed: {}", e)))?;

    // Check if too many results
    if total > 10000 {
        return Err(SearchError::too_many_results(total, 10000));
    }

    let documents = state
        .db
        .search_documents_with_role(user.id, user.role, search_request)
        .await
        .map_err(|e| SearchError::index_unavailable(format!("Search failed: {}", e)))?;

    let mut documents: Vec<EnhancedDocumentResponse> = documents
        .into_iter()
        .map(|item| {
            EnhancedDocumentResponse::from_document(
                item.document,
                None,
                Vec::new(),
                item.ocr_progress_current,
                item.ocr_progress_total,
            )
        })
        .collect();

    state
        .db
        .attach_labels_to_search_results(&mut documents)
        .await
        .map_err(|e| SearchError::index_unavailable(format!("Loading labels failed: {}", e)))?;

    Ok(SearchResponse {
        documents,
        total,
        query_time_ms: 0,
        suggestions: Vec::new(),
    })
}

/// Longest accepted query, in bytes.
const MAX_QUERY_LEN: usize = 1000;
/// Largest page and snippet window a search may ask for.
const MAX_LIMIT: i64 = 1000;
const MAX_SNIPPET_LENGTH: i32 = 2000;

/// Request checks shared by basic and enhanced search, so neither can send
/// an unbounded query, page or snippet window to the database. An empty
/// query is allowed when a filter narrows the search.
pub fn validate_search_request(request: &SearchRequest) -> Result<(), SearchError> {
    let filters = request.filters();
    filters.validate().map_err(SearchError::invalid_syntax)?;
    if request.query.len() < 2 && !filters.is_active() {
        return Err(SearchError::query_too_short(request.query.len(), 2));
    }
    if request.query.len() > MAX_QUERY_LEN {
        return Err(SearchError::query_too_long(
            request.query.len(),
            MAX_QUERY_LEN,
        ));
    }
    let limit = request.limit.unwrap_or(25);
    let offset = request.offset.unwrap_or(0);
    if !(1..=MAX_LIMIT).contains(&limit) || offset < 0 {
        return Err(SearchError::invalid_pagination(offset, limit));
    }
    if let Some(length) = request.snippet_length {
        if !(1..=MAX_SNIPPET_LENGTH).contains(&length) {
            return Err(SearchError::invalid_syntax(format!(
                "snippet_length must be between 1 and {MAX_SNIPPET_LENGTH}, got {length}"
            )));
        }
    }
    Ok(())
}

/// Enhanced search for `user` (ranking, snippets, suggestions). Shared by
/// `GET /api/search/enhanced` and the MCP `enhanced_search` tool.
pub async fn run_enhanced_search(
    state: &AppState,
    user: &User,
    search_request: &SearchRequest,
) -> Result<SearchResponse, SearchError> {
    validate_search_request(search_request)?;

    let suggestions = generate_search_suggestions(&search_request.query);

    let start_time = std::time::Instant::now();

    let fuzzy = fuzzy_threshold_for(state, user, search_request)
        .await
        .map_err(|e| SearchError::index_unavailable(format!("Loading settings failed: {}", e)))?;

    // Get total count (without pagination) for proper pagination support.
    // Same request and cutoff as the list, so the two agree.
    let total = state
        .db
        .count_search_documents(user.id, user.role, search_request, fuzzy)
        .await
        .map_err(|e| SearchError::index_unavailable(format!("Count failed: {}", e)))?;

    let documents = state
        .db
        .enhanced_search_documents_with_role(user.id, user.role, search_request, fuzzy)
        .await
        .map_err(|e| SearchError::index_unavailable(format!("Search failed: {}", e)))?;

    let query_time = start_time.elapsed().as_millis() as u64;

    Ok(SearchResponse {
        documents,
        total,
        query_time_ms: query_time,
        suggestions,
    })
}

/// The fuzzy-mode cutoff for `user`: their `fuzzy_search_threshold` setting,
/// validated by `FuzzyThreshold::from_setting` (the default when unset).
/// Other modes ignore the cutoff, so their searches skip the settings lookup.
pub async fn fuzzy_threshold_for(
    state: &AppState,
    user: &User,
    search_request: &SearchRequest,
) -> anyhow::Result<FuzzyThreshold> {
    if !matches!(search_request.search_mode, Some(SearchMode::Fuzzy)) {
        return Ok(FuzzyThreshold::default());
    }
    let setting = state.db.get_fuzzy_search_threshold(user.id).await?;
    Ok(FuzzyThreshold::from_setting(setting))
}

/// Matches per UTC creation month for `user`, with the same predicate (and
/// fuzzy cutoff) as enhanced search's count. Backs `GET /api/search/timeline`.
pub async fn load_search_timeline(
    state: &AppState,
    user: &User,
    search_request: &SearchRequest,
) -> anyhow::Result<Vec<MonthCount>> {
    let fuzzy = fuzzy_threshold_for(state, user, search_request).await?;
    state
        .db
        .search_timeline(user.id, user.role, search_request, fuzzy)
        .await
}

fn generate_search_suggestions(query: &str) -> Vec<String> {
    // Simple suggestion generation - could be enhanced with a proper suggestion system
    let mut suggestions = Vec::new();

    if query.len() > 3 {
        // Common search variations
        suggestions.push(format!("\"{}\"", query)); // Exact phrase

        // Add wildcard suggestions
        if !query.contains('*') {
            suggestions.push(format!("{}*", query));
        }

        // Add similar terms (this would typically come from a thesaurus or ML model)
        if query.contains("document") {
            suggestions.push(query.replace("document", "file"));
            suggestions.push(query.replace("document", "paper"));
        }
    }

    suggestions.into_iter().take(3).collect()
}

/// MIME type and tag facets for the documents `user` can see. Shared by
/// `GET /api/search/facets` and the MCP `get_search_facets` tool.
pub async fn load_search_facets(
    state: &AppState,
    user: &User,
) -> anyhow::Result<SearchFacetsResponse> {
    let mime_types = state.db.get_mime_type_facets(user.id, user.role).await?;
    let tags = state.db.get_tag_facets(user.id, user.role).await?;
    Ok(SearchFacetsResponse { mime_types, tags })
}

#[cfg(test)]
mod validation_tests {
    use super::*;

    fn request(query: &str) -> SearchRequest {
        SearchRequest {
            query: query.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn query_length_is_bounded() {
        assert!(validate_search_request(&request("invoice")).is_ok());
        assert!(
            validate_search_request(&request("a")).is_err(),
            "too short without filters"
        );
        assert!(validate_search_request(&request(&"x".repeat(MAX_QUERY_LEN))).is_ok());
        assert!(validate_search_request(&request(&"x".repeat(MAX_QUERY_LEN + 1))).is_err());
    }

    #[test]
    fn pagination_is_bounded() {
        for (limit, offset, ok) in [
            (25, 0, true),
            (MAX_LIMIT, 10, true),
            (0, 0, false),
            (-5, 0, false),
            (MAX_LIMIT + 1, 0, false),
            (25, -1, false),
        ] {
            let req = SearchRequest {
                limit: Some(limit),
                offset: Some(offset),
                ..request("invoice")
            };
            assert_eq!(
                validate_search_request(&req).is_ok(),
                ok,
                "limit {limit} offset {offset}"
            );
        }
    }

    #[test]
    fn snippet_length_is_bounded() {
        for (length, ok) in [
            (1, true),
            (200, true),
            (MAX_SNIPPET_LENGTH, true),
            (0, false),
            (-1, false),
            (MAX_SNIPPET_LENGTH + 1, false),
        ] {
            let req = SearchRequest {
                snippet_length: Some(length),
                ..request("invoice")
            };
            assert_eq!(
                validate_search_request(&req).is_ok(),
                ok,
                "snippet_length {length}"
            );
        }
    }
}
