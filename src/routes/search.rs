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
    errors::{search::SearchError, AppError},
    models::{MonthCount, SearchFacetsResponse, SearchRequest, SearchResponse},
    services::search::{load_search_facets, run_basic_search, run_enhanced_search},
    AppState,
};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(search_documents))
        .route("/enhanced", get(enhanced_search_documents))
        .route("/facets", get(get_search_facets))
        .route("/timeline", get(search_timeline))
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
    run_basic_search(&state, &auth_user.user, &search_request)
        .await
        .map(Json)
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
    run_enhanced_search(&state, &auth_user.user, &search_request)
        .await
        .map(Json)
        .map_err(|e| {
            if e.status_code().is_client_error() {
                StatusCode::BAD_REQUEST
            } else {
                tracing::error!("Enhanced search failed: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })
}

#[utoipa::path(
    get,
    path = "/api/search/timeline",
    tag = "search",
    description = "Number of documents matching the search per UTC creation month, across all matches \
        (not one page). Takes the same query and filters as /api/search/enhanced; pagination, snippet \
        and sort parameters are ignored. Months without matches are omitted; ascending by month.",
    security(
        ("bearer_auth" = [])
    ),
    params(
        SearchRequest
    ),
    responses(
        (status = 200, description = "Match counts per month, oldest first", body = Vec<MonthCount>),
        (status = 400, description = "Invalid query or filter parameter"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
async fn search_timeline(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Query(search_request): Query<SearchRequest>,
) -> Result<Json<Vec<MonthCount>>, StatusCode> {
    // Same acceptance rules as enhanced search, so both calls agree.
    let filters = search_request.filters();
    if filters.validate().is_err() {
        return Err(StatusCode::BAD_REQUEST);
    }
    if search_request.query.len() < 2 && !filters.is_active() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let months = state
        .db
        .search_timeline(auth_user.user.id, auth_user.user.role, &search_request)
        .await
        .map_err(|e| {
            tracing::error!("Search timeline failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(months))
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
    load_search_facets(&state, &auth_user.user)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Loading search facets failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}
