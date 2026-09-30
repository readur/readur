use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use std::sync::Arc;

use crate::{
    auth::AuthUser,
    errors::search::SearchError,
    models::{
        EnhancedDocumentResponse, SearchFacetsResponse, SearchMode, SearchRequest, SearchResponse,
    },
    AppState,
};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(search_documents))
        .route("/enhanced", get(enhanced_search_documents))
        .route("/facets", get(get_search_facets))
}

#[utoipa::path(
    get,
    path = "/api/search",
    tag = "search",
    description = "Search documents with basic relevance ranking and OCR text matching",
    security(
        ("bearer_auth" = [])
    ),
    params(
        SearchRequest
    ),
    responses(
        (status = 200, description = "Enhanced search results with relevance ranking, text snippets, and OCR-extracted content matching", body = SearchResponse),
        (status = 400, description = "Invalid query, sort or filter parameter"),
        (status = 401, description = "Unauthorized - valid authentication required"),
        (status = 500, description = "Internal server error")
    )
)]
async fn search_documents(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Query(search_request): Query<SearchRequest>,
) -> Result<Json<SearchResponse>, SearchError> {
    // Validate filters, then query length (allow empty query if filters are present)
    let filters = search_request.filters();
    filters.validate().map_err(SearchError::invalid_syntax)?;
    if search_request.query.len() < 2 && !filters.is_active() {
        return Err(SearchError::query_too_short(search_request.query.len(), 2));
    }
    if search_request.query.len() > 1000 {
        return Err(SearchError::query_too_long(
            search_request.query.len(),
            1000,
        ));
    }

    // Validate pagination
    let limit = search_request.limit.unwrap_or(25);
    let offset = search_request.offset.unwrap_or(0);
    if limit > 1000 || offset < 0 || limit <= 0 {
        return Err(SearchError::invalid_pagination(offset, limit));
    }

    // Get total count (without pagination) for proper pagination support. The
    // list below always matches in simple mode, so the count does too.
    let total = state
        .db
        .count_search_documents_in_mode(
            auth_user.user.id,
            auth_user.user.role.clone(),
            &search_request,
            &SearchMode::Simple,
        )
        .await
        .map_err(|e| SearchError::index_unavailable(format!("Count failed: {}", e)))?;

    // Check if too many results
    if total > 10000 {
        return Err(SearchError::too_many_results(total, 10000));
    }

    let documents = state
        .db
        .search_documents_with_role(auth_user.user.id, auth_user.user.role, &search_request)
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

    let response = SearchResponse {
        documents,
        total,
        query_time_ms: 0,
        suggestions: Vec::new(),
    };

    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/api/search/enhanced",
    tag = "search",
    description = "Enhanced search with improved ranking, text snippets, and query suggestions",
    security(
        ("bearer_auth" = [])
    ),
    params(
        SearchRequest
    ),
    responses(
        (status = 200, description = "Enhanced search results with snippets and suggestions", body = SearchResponse),
        (status = 400, description = "Invalid query, sort or filter parameter"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
async fn enhanced_search_documents(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Query(search_request): Query<SearchRequest>,
) -> Result<Json<SearchResponse>, StatusCode> {
    // Validate filters, then query length (allow empty query if filters are present)
    let filters = search_request.filters();
    if filters.validate().is_err() {
        return Err(StatusCode::BAD_REQUEST);
    }
    if search_request.query.len() < 2 && !filters.is_active() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Generate suggestions before moving search_request
    let suggestions = generate_search_suggestions(&search_request.query);

    let start_time = std::time::Instant::now();

    // Get total count (without pagination) for proper pagination support
    let total = state
        .db
        .count_search_documents(
            auth_user.user.id,
            auth_user.user.role.clone(),
            &search_request,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let documents = state
        .db
        .enhanced_search_documents_with_role(
            auth_user.user.id,
            auth_user.user.role,
            &search_request,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let query_time = start_time.elapsed().as_millis() as u64;

    let response = SearchResponse {
        documents,
        total,
        query_time_ms: query_time,
        suggestions,
    };

    Ok(Json(response))
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

#[utoipa::path(
    get,
    path = "/api/search/facets",
    tag = "search",
    description = "Get available search facets (MIME types, tags) with document counts for filtering",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Search facets with counts", body = SearchFacetsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_search_facets(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> Result<Json<SearchFacetsResponse>, StatusCode> {
    let user_id = auth_user.user.id;
    let user_role = auth_user.user.role;

    // Get MIME type facets
    let mime_type_facets = state
        .db
        .get_mime_type_facets(user_id, user_role.clone())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get tag facets
    let tag_facets = state
        .db
        .get_tag_facets(user_id, user_role)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = SearchFacetsResponse {
        mime_types: mime_type_facets,
        tags: tag_facets,
    };

    Ok(Json(response))
}
