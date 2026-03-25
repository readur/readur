use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header::CONTENT_TYPE},
    response::{Json, Response},
    body::Body,
    routing::{get, post, delete},
    Router,
};
use serde::Deserialize;
use std::net::IpAddr;
use std::sync::Arc;
use tracing::{debug, error, warn};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    errors::shared_link::SharedLinkError,
    models::shared_link::{
        CreateSharedLinkRequest, SharedDocumentMetadata, SharedLinkPasswordRequest,
        SharedLinkResponse,
    },
    models::UserRole,
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
        .route("/{token}/download", post(download_shared_document))
        .route("/{token}/view", post(view_shared_document))
}

fn generate_token() -> String {
    use rand::Rng;
    use base64ct::Encoding;
    let bytes: [u8; 32] = rand::rng().random();
    // URL-safe base64 without padding
    base64ct::Base64UrlUnpadded::encode_string(&bytes)
}

/// Extract client IP from request headers, checking X-Forwarded-For first (for reverse proxies),
/// then X-Real-Ip, falling back to a default.
fn extract_client_ip(headers: &HeaderMap) -> IpAddr {
    if let Some(forwarded) = headers.get("x-forwarded-for") {
        if let Ok(val) = forwarded.to_str() {
            // X-Forwarded-For can contain multiple IPs; first one is the client
            if let Some(first_ip) = val.split(',').next() {
                if let Ok(ip) = first_ip.trim().parse::<IpAddr>() {
                    return ip;
                }
            }
        }
    }
    if let Some(real_ip) = headers.get("x-real-ip") {
        if let Ok(val) = real_ip.to_str() {
            if let Ok(ip) = val.trim().parse::<IpAddr>() {
                return ip;
            }
        }
    }
    // Fallback — treat as localhost if we can't determine IP
    IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)
}

/// Sanitize a filename for use in Content-Disposition headers.
/// Strips characters that could enable header injection or path traversal.
fn sanitize_filename(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | ' ') {
                c
            } else {
                '_'
            }
        })
        .collect();
    // Truncate to 255 characters and trim whitespace
    let truncated = if sanitized.len() > 255 {
        &sanitized[..255]
    } else {
        &sanitized
    };
    let result = truncated.trim().to_string();
    if result.is_empty() {
        "download".to_string()
    } else {
        result
    }
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

    // Rate limit shared link creation per user
    if let Err(retry_after) = state.rate_limiters.shared_link_creation.check(&user_id).await {
        warn!("Rate limited shared link creation for user {}", user_id);
        return Err(SharedLinkError::RateLimited { retry_after_secs: retry_after });
    }

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
    // Admins can see all shared links; regular users see only their own
    let links = if auth_user.user.role == UserRole::Admin {
        state.db.get_all_shared_links().await
    } else {
        state.db.get_shared_links_by_user(auth_user.user.id).await
    }
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

    // Admins see all links for the document; regular users see only their own
    let links = if auth_user.user.role == UserRole::Admin {
        state.db.get_all_shared_links_by_document(document_id).await
    } else {
        state.db.get_shared_links_by_document(document_id, auth_user.user.id).await
    }
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
    // Admins can revoke any link; regular users can only revoke their own
    let revoked = if auth_user.user.role == UserRole::Admin {
        state.db.admin_revoke_shared_link(id).await
    } else {
        state.db.revoke_shared_link(id, auth_user.user.id).await
    }
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
    headers: HeaderMap,
    Path(token): Path<String>,
) -> Result<Json<SharedDocumentMetadata>, SharedLinkError> {
    // Rate limit public access per IP
    let client_ip = extract_client_ip(&headers);
    if let Err(retry_after) = state.rate_limiters.shared_link_public.check(&client_ip).await {
        return Err(SharedLinkError::RateLimited { retry_after_secs: retry_after });
    }

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

    let document = state
        .db
        .get_document_by_id_unfiltered(link.document_id)
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

/// Password payload for download/view (POST body instead of query param to avoid logging secrets)
#[derive(Debug, Deserialize)]
pub struct PasswordPayload {
    pub password: Option<String>,
}

pub async fn verify_shared_link_password(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(token): Path<String>,
    Json(payload): Json<SharedLinkPasswordRequest>,
) -> Result<Json<serde_json::Value>, SharedLinkError> {
    // Rate limit password verification attempts per IP
    let client_ip = extract_client_ip(&headers);
    if let Err(retry_after) = state.rate_limiters.shared_link_password.check(&client_ip).await {
        warn!("Rate limited shared link password attempt from {}", client_ip);
        return Err(SharedLinkError::RateLimited { retry_after_secs: retry_after });
    }

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
    headers: HeaderMap,
    Path(token): Path<String>,
    Json(payload): Json<PasswordPayload>,
) -> Result<Response<Body>, SharedLinkError> {
    // Rate limit public access per IP (general limit + password-specific limit if password provided)
    let client_ip = extract_client_ip(&headers);
    if let Err(retry_after) = state.rate_limiters.shared_link_public.check(&client_ip).await {
        warn!("Rate limited shared link download from {}", client_ip);
        return Err(SharedLinkError::RateLimited { retry_after_secs: retry_after });
    }
    if payload.password.is_some() {
        if let Err(retry_after) = state.rate_limiters.shared_link_password.check(&client_ip).await {
            warn!("Rate limited shared link password attempt via download from {}", client_ip);
            return Err(SharedLinkError::RateLimited { retry_after_secs: retry_after });
        }
    }

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
    verify_password_if_required(&link, payload.password.as_deref())?;

    // Increment view count
    let _ = state.db.increment_shared_link_view_count(link.id).await;

    let document = state
        .db
        .get_document_by_id_unfiltered(link.document_id)
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
            format!("attachment; filename=\"{}\"", sanitize_filename(&document.original_filename)),
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
    headers: HeaderMap,
    Path(token): Path<String>,
    Json(payload): Json<PasswordPayload>,
) -> Result<Response<Body>, SharedLinkError> {
    // Rate limit public access per IP (general limit + password-specific limit if password provided)
    let client_ip = extract_client_ip(&headers);
    if let Err(retry_after) = state.rate_limiters.shared_link_public.check(&client_ip).await {
        warn!("Rate limited shared link view from {}", client_ip);
        return Err(SharedLinkError::RateLimited { retry_after_secs: retry_after });
    }
    if payload.password.is_some() {
        if let Err(retry_after) = state.rate_limiters.shared_link_password.check(&client_ip).await {
            warn!("Rate limited shared link password attempt via view from {}", client_ip);
            return Err(SharedLinkError::RateLimited { retry_after_secs: retry_after });
        }
    }

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
    verify_password_if_required(&link, payload.password.as_deref())?;

    // Increment view count
    let _ = state.db.increment_shared_link_view_count(link.id).await;

    let document = state
        .db
        .get_document_by_id_unfiltered(link.document_id)
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
            format!("inline; filename=\"{}\"", sanitize_filename(&document.original_filename)),
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
