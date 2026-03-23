use axum::{
    extract::{Path, State},
    http::{StatusCode, header::CONTENT_TYPE},
    response::{Json, Response},
    body::Body,
    routing::{get, post, delete},
    Router,
};
use serde::Deserialize;
use std::sync::Arc;
use tracing::{debug, error};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    errors::shared_link::SharedLinkError,
    models::shared_link::{
        CreateSharedLinkRequest, SharedDocumentMetadata, SharedLinkPasswordRequest,
        SharedLinkResponse,
    },
    AppState,
};

/// Router for authenticated shared link management
pub fn authenticated_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(create_shared_link))
        .route("/", get(list_shared_links))
        .route("/document/{document_id}", get(list_shared_links_for_document))
        .route("/{id}", delete(revoke_shared_link))
}

/// Router for public (unauthenticated) shared link access
pub fn public_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/{token}", get(get_shared_document_metadata))
        .route("/{token}/verify", post(verify_shared_link_password))
        .route("/{token}/download", get(download_shared_document))
        .route("/{token}/view", get(view_shared_document))
}

fn generate_token() -> String {
    use rand::Rng;
    use base64ct::Encoding;
    let bytes: [u8; 32] = rand::rng().random();
    // URL-safe base64 without padding
    base64ct::Base64UrlUnpadded::encode_string(&bytes)
}

fn get_base_url(state: &AppState) -> String {
    // Use the configured public URL or fall back to server address
    state.config.public_url.clone().unwrap_or_else(|| {
        format!("http://{}", state.config.server_address)
    })
}

/// Validate that a shared link is currently accessible (not expired, not revoked, views remaining)
fn validate_shared_link(link: &crate::models::shared_link::SharedLink) -> Result<(), SharedLinkError> {
    if link.is_revoked {
        return Err(SharedLinkError::Revoked);
    }
    if let Some(expires_at) = link.expires_at {
        if expires_at < chrono::Utc::now() {
            return Err(SharedLinkError::Expired);
        }
    }
    if let Some(max_views) = link.max_views {
        if link.view_count >= max_views {
            return Err(SharedLinkError::MaxViewsReached);
        }
    }
    Ok(())
}

// ─── Authenticated Handlers ────────────────────────────────────────────────

pub async fn create_shared_link(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Json(payload): Json<CreateSharedLinkRequest>,
) -> Result<Json<SharedLinkResponse>, SharedLinkError> {
    let user_id = auth_user.user.id;

    // Verify the user owns this document (or is admin)
    let document = state
        .db
        .get_document_by_id(payload.document_id, user_id, auth_user.user.role)
        .await
        .map_err(|e| {
            error!("Failed to fetch document: {}", e);
            SharedLinkError::InternalError { message: "Failed to verify document access".into() }
        })?
        .ok_or(SharedLinkError::DocumentNotFound)?;

    let token = generate_token();

    let password_hash = match &payload.password {
        Some(password) => {
            let hash = bcrypt::hash(password, bcrypt::DEFAULT_COST).map_err(|e| {
                error!("Failed to hash password: {}", e);
                SharedLinkError::InternalError { message: "Failed to create shared link".into() }
            })?;
            Some(hash)
        }
        None => None,
    };

    let link = state
        .db
        .create_shared_link(
            payload.document_id,
            user_id,
            &token,
            password_hash.as_deref(),
            payload.expires_at,
            payload.max_views,
        )
        .await
        .map_err(|e| {
            error!("Failed to create shared link: {}", e);
            SharedLinkError::InternalError { message: "Failed to create shared link".into() }
        })?;

    let base_url = get_base_url(&state);
    debug!("Created shared link for document {}: {}", document.id, token);

    Ok(Json(SharedLinkResponse::from_shared_link(link, &base_url)))
}

pub async fn list_shared_links(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> Result<Json<Vec<SharedLinkResponse>>, SharedLinkError> {
    let links = state
        .db
        .get_shared_links_by_user(auth_user.user.id)
        .await
        .map_err(|e| {
            error!("Failed to fetch shared links: {}", e);
            SharedLinkError::InternalError { message: "Failed to list shared links".into() }
        })?;

    let base_url = get_base_url(&state);
    let responses: Vec<SharedLinkResponse> = links
        .into_iter()
        .map(|l| SharedLinkResponse::from_shared_link(l, &base_url))
        .collect();

    Ok(Json(responses))
}

pub async fn list_shared_links_for_document(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(document_id): Path<Uuid>,
) -> Result<Json<Vec<SharedLinkResponse>>, SharedLinkError> {
    // Verify document access
    state
        .db
        .get_document_by_id(document_id, auth_user.user.id, auth_user.user.role)
        .await
        .map_err(|e| {
            error!("Failed to fetch document: {}", e);
            SharedLinkError::InternalError { message: "Failed to verify document access".into() }
        })?
        .ok_or(SharedLinkError::DocumentNotFound)?;

    let links = state
        .db
        .get_shared_links_by_document(document_id, auth_user.user.id)
        .await
        .map_err(|e| {
            error!("Failed to fetch shared links: {}", e);
            SharedLinkError::InternalError { message: "Failed to list shared links".into() }
        })?;

    let base_url = get_base_url(&state);
    let responses: Vec<SharedLinkResponse> = links
        .into_iter()
        .map(|l| SharedLinkResponse::from_shared_link(l, &base_url))
        .collect();

    Ok(Json(responses))
}

pub async fn revoke_shared_link(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, SharedLinkError> {
    let revoked = state
        .db
        .revoke_shared_link(id, auth_user.user.id)
        .await
        .map_err(|e| {
            error!("Failed to revoke shared link: {}", e);
            SharedLinkError::InternalError { message: "Failed to revoke shared link".into() }
        })?;

    if revoked {
        debug!("Revoked shared link {}", id);
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(SharedLinkError::NotFound)
    }
}

// ─── Public (Unauthenticated) Handlers ─────────────────────────────────────

pub async fn get_shared_document_metadata(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
) -> Result<Json<SharedDocumentMetadata>, SharedLinkError> {
    let link = state
        .db
        .get_shared_link_by_token(&token)
        .await
        .map_err(|e| {
            error!("Failed to fetch shared link: {}", e);
            SharedLinkError::InternalError { message: "Failed to access shared link".into() }
        })?
        .ok_or(SharedLinkError::NotFound)?;

    validate_shared_link(&link)?;

    // Fetch document without role-based filter (public access via token)
    let document = sqlx::query_as::<_, crate::models::Document>(
        &format!("SELECT {} FROM documents WHERE id = $1", "id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, ocr_retry_count, ocr_failure_reason, tags, created_at, updated_at, user_id, file_hash, original_created_at, original_modified_at, source_path, source_type, source_id, file_permissions, file_owner, file_group, source_metadata, has_ocr_text, ocr_progress_percent, ocr_current_page, ocr_total_pages")
    )
    .bind(link.document_id)
    .fetch_optional(state.db.get_pool())
    .await
    .map_err(|e| {
        error!("Failed to fetch document for shared link: {}", e);
        SharedLinkError::InternalError { message: "Failed to access shared document".into() }
    })?
    .ok_or(SharedLinkError::DocumentNotFound)?;

    Ok(Json(SharedDocumentMetadata {
        filename: document.filename.clone(),
        original_filename: document.original_filename.clone(),
        file_size: document.file_size,
        mime_type: document.mime_type.clone(),
        requires_password: link.password_hash.is_some(),
        created_at: document.created_at,
    }))
}

/// Optional password query param for download/view
#[derive(Debug, Deserialize)]
pub struct PasswordQuery {
    pub password: Option<String>,
}

pub async fn verify_shared_link_password(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
    Json(payload): Json<SharedLinkPasswordRequest>,
) -> Result<Json<serde_json::Value>, SharedLinkError> {
    let link = state
        .db
        .get_shared_link_by_token(&token)
        .await
        .map_err(|e| {
            error!("Failed to fetch shared link: {}", e);
            SharedLinkError::InternalError { message: "Failed to access shared link".into() }
        })?
        .ok_or(SharedLinkError::NotFound)?;

    validate_shared_link(&link)?;

    let password_hash = link.password_hash.as_ref().ok_or(SharedLinkError::NotFound)?;

    let valid = bcrypt::verify(&payload.password, password_hash).map_err(|e| {
        error!("Failed to verify password: {}", e);
        SharedLinkError::InternalError { message: "Failed to verify password".into() }
    })?;

    if !valid {
        return Err(SharedLinkError::InvalidPassword);
    }

    Ok(Json(serde_json::json!({ "valid": true })))
}

pub async fn download_shared_document(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
    axum::extract::Query(query): axum::extract::Query<PasswordQuery>,
) -> Result<Response<Body>, SharedLinkError> {
    let link = state
        .db
        .get_shared_link_by_token(&token)
        .await
        .map_err(|e| {
            error!("Failed to fetch shared link: {}", e);
            SharedLinkError::InternalError { message: "Failed to access shared link".into() }
        })?
        .ok_or(SharedLinkError::NotFound)?;

    validate_shared_link(&link)?;
    verify_password_if_required(&link, query.password.as_deref())?;

    // Increment view count
    let _ = state.db.increment_shared_link_view_count(link.id).await;

    // Fetch document
    let document = sqlx::query_as::<_, crate::models::Document>(
        "SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, ocr_retry_count, ocr_failure_reason, tags, created_at, updated_at, user_id, file_hash, original_created_at, original_modified_at, source_path, source_type, source_id, file_permissions, file_owner, file_group, source_metadata, has_ocr_text, ocr_progress_percent, ocr_current_page, ocr_total_pages FROM documents WHERE id = $1"
    )
    .bind(link.document_id)
    .fetch_optional(state.db.get_pool())
    .await
    .map_err(|e| {
        error!("Failed to fetch document: {}", e);
        SharedLinkError::InternalError { message: "Failed to access shared document".into() }
    })?
    .ok_or(SharedLinkError::DocumentNotFound)?;

    let file_data = state
        .file_service
        .read_file(&document.file_path)
        .await
        .map_err(|e| {
            error!("Failed to read document file: {}", e);
            SharedLinkError::InternalError { message: "Failed to read document file".into() }
        })?;

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, &document.mime_type)
        .header(
            "Content-Disposition",
            format!("attachment; filename=\"{}\"", document.original_filename),
        )
        .header("Content-Length", file_data.len().to_string())
        .body(Body::from(file_data))
        .map_err(|e| {
            error!("Failed to build response: {}", e);
            SharedLinkError::InternalError { message: "Failed to serve document".into() }
        })?;

    Ok(response)
}

pub async fn view_shared_document(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
    axum::extract::Query(query): axum::extract::Query<PasswordQuery>,
) -> Result<Response<Body>, SharedLinkError> {
    let link = state
        .db
        .get_shared_link_by_token(&token)
        .await
        .map_err(|e| {
            error!("Failed to fetch shared link: {}", e);
            SharedLinkError::InternalError { message: "Failed to access shared link".into() }
        })?
        .ok_or(SharedLinkError::NotFound)?;

    validate_shared_link(&link)?;
    verify_password_if_required(&link, query.password.as_deref())?;

    // Increment view count
    let _ = state.db.increment_shared_link_view_count(link.id).await;

    // Fetch document
    let document = sqlx::query_as::<_, crate::models::Document>(
        "SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, ocr_retry_count, ocr_failure_reason, tags, created_at, updated_at, user_id, file_hash, original_created_at, original_modified_at, source_path, source_type, source_id, file_permissions, file_owner, file_group, source_metadata, has_ocr_text, ocr_progress_percent, ocr_current_page, ocr_total_pages FROM documents WHERE id = $1"
    )
    .bind(link.document_id)
    .fetch_optional(state.db.get_pool())
    .await
    .map_err(|e| {
        error!("Failed to fetch document: {}", e);
        SharedLinkError::InternalError { message: "Failed to access shared document".into() }
    })?
    .ok_or(SharedLinkError::DocumentNotFound)?;

    let file_data = state
        .file_service
        .read_file(&document.file_path)
        .await
        .map_err(|e| {
            error!("Failed to read document file: {}", e);
            SharedLinkError::InternalError { message: "Failed to read document file".into() }
        })?;

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, &document.mime_type)
        .header(
            "Content-Disposition",
            format!("inline; filename=\"{}\"", document.original_filename),
        )
        .header("Content-Length", file_data.len().to_string())
        .body(Body::from(file_data))
        .map_err(|e| {
            error!("Failed to build response: {}", e);
            SharedLinkError::InternalError { message: "Failed to serve document".into() }
        })?;

    Ok(response)
}

fn verify_password_if_required(
    link: &crate::models::shared_link::SharedLink,
    password: Option<&str>,
) -> Result<(), SharedLinkError> {
    if let Some(hash) = &link.password_hash {
        let password = password.ok_or(SharedLinkError::PasswordRequired)?;
        let valid = bcrypt::verify(password, hash).map_err(|e| {
            error!("Failed to verify password: {}", e);
            SharedLinkError::InternalError { message: "Failed to verify password".into() }
        })?;
        if !valid {
            return Err(SharedLinkError::InvalidPassword);
        }
    }
    Ok(())
}
