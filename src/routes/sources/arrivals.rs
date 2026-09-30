use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;
use tracing::error;

use crate::{
    auth::AuthUser,
    models::{ArrivalsQuery, SourceArrivals},
    AppState,
};

/// Daily document arrivals per ingestion lane
#[utoipa::path(
    get,
    path = "/api/sources/arrivals",
    tag = "sources",
    description = "Documents that arrived per UTC day for every visible source, plus the watch folder and uploads. \
        Every lane is returned, zero-filled, even without arrivals. Admins see every source and document; \
        users see their own.",
    security(
        ("bearer_auth" = [])
    ),
    params(ArrivalsQuery),
    responses(
        (status = 200, description = "One lane per source, then the watch folder and uploads", body = Vec<SourceArrivals>),
        (status = 400, description = "days is outside 1-60"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_source_arrivals(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
    Query(query): Query<ArrivalsQuery>,
) -> Result<Json<Vec<SourceArrivals>>, StatusCode> {
    let days = query.days().map_err(|_| StatusCode::BAD_REQUEST)?;
    let lanes = state
        .db
        .get_source_arrivals(auth_user.user.id, auth_user.user.role, days)
        .await
        .map_err(|e| {
            error!("Failed to load source arrivals: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(lanes))
}
