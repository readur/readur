use axum::{extract::State, http::StatusCode, response::Json, routing::get, Router};
use std::sync::Arc;

use crate::{
    auth::AuthUser, models::AvailableLanguagesResponse,
    services::ocr_languages::available_languages_for_user, AppState,
};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/health", get(crate::ocr::api::health_check))
        .route("/languages", get(get_available_languages))
}

#[utoipa::path(
    get,
    path = "/api/ocr/languages",
    tag = "ocr",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Available OCR languages and user's current language", body = AvailableLanguagesResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_available_languages(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> Result<Json<AvailableLanguagesResponse>, StatusCode> {
    available_languages_for_user(&state.db, auth_user.user.id)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
