use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post},
    Router,
};
use base64ct::Encoding;
use rand::Rng;
use serde::Deserialize;
use std::sync::Arc;
use tracing::{debug, error, warn};
use uuid::Uuid;

use crate::{
    auth::{sha256_hex, AuthUser, API_KEY_DISPLAY_PREFIX_LEN, API_KEY_PREFIX},
    errors::api_key::ApiKeyError,
    models::{
        api_key::{ApiKey, ApiKeyResponse, CreateApiKeyRequest, CreateApiKeyResponse},
        UserRole,
    },
    AppState,
};

/// Cap on how many active (non-revoked) API keys one user can have at a time.
/// Protects against runaway key creation, accidental or malicious.
const MAX_ACTIVE_KEYS_PER_USER: i64 = 20;

/// Maximum allowed `expires_in_days`. Bounds the blast radius of a leaked key
/// and aligns with industry norms (GitHub PATs = 1 year max).
const MAX_EXPIRES_IN_DAYS: u32 = 365;

/// Maximum characters allowed in a user-supplied key name. Prevents pathological
/// payloads from taking up space / hurting the UI.
const MAX_NAME_LEN: usize = 100;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(create_api_key))
        .route("/", get(list_api_keys))
        .route("/{id}", delete(revoke_api_key))
}

/// Generate a fresh plaintext key of the form `readur_pat_<43 chars of base64url>`.
/// 32 bytes of OS randomness = 256 bits of entropy.
fn generate_plaintext_key() -> String {
    let bytes: [u8; 32] = rand::rng().random();
    let tail = base64ct::Base64UrlUnpadded::encode_string(&bytes);
    format!("{}{}", API_KEY_PREFIX, tail)
}

#[derive(Deserialize)]
pub struct ListQuery {
    /// Admin-only: when true, returns every user's keys. Ignored for non-admins.
    #[serde(default)]
    pub all: bool,
}

/// POST /api/auth/api-keys — create a new API key and return the plaintext
/// exactly once. The caller must save it immediately; the server never exposes
/// it again.
pub async fn create_api_key(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Json(payload): Json<CreateApiKeyRequest>,
) -> Result<Json<CreateApiKeyResponse>, ApiKeyError> {
    let user_id = auth_user.user.id;

    // Rate-limit creation per user before doing any work.
    if let Err(retry_after) = state.rate_limiters.api_key_creation.check(&user_id).await {
        warn!(user_id = %user_id, "Rate limited API key creation");
        return Err(ApiKeyError::RateLimited { retry_after_secs: retry_after });
    }

    let name = payload.name.trim();
    if name.is_empty() {
        return Err(ApiKeyError::InvalidRequest { reason: "Name is required".into() });
    }
    if name.chars().count() > MAX_NAME_LEN {
        return Err(ApiKeyError::InvalidRequest {
            reason: format!("Name must be {} characters or fewer", MAX_NAME_LEN),
        });
    }

    let expires_at = match payload.expires_in_days {
        Some(0) => {
            return Err(ApiKeyError::InvalidRequest {
                reason: "expires_in_days must be at least 1".into(),
            });
        }
        Some(days) if days > MAX_EXPIRES_IN_DAYS => {
            return Err(ApiKeyError::InvalidRequest {
                reason: format!("expires_in_days cannot exceed {}", MAX_EXPIRES_IN_DAYS),
            });
        }
        Some(days) => Some(chrono::Utc::now() + chrono::Duration::days(days as i64)),
        None => None,
    };

    // Enforce per-user cap to avoid unbounded key accumulation.
    let active_count = state
        .db
        .count_active_api_keys_for_user(user_id)
        .await
        .map_err(|e| {
            error!("Failed to count active API keys: {}", e);
            ApiKeyError::InternalError { message: "Failed to create API key".into() }
        })?;
    if active_count >= MAX_ACTIVE_KEYS_PER_USER {
        return Err(ApiKeyError::MaxKeysReached { limit: MAX_ACTIVE_KEYS_PER_USER });
    }

    let plaintext = generate_plaintext_key();
    let key_hash = sha256_hex(&plaintext);
    let key_prefix: String = plaintext.chars().take(API_KEY_DISPLAY_PREFIX_LEN).collect();

    let api_key = state
        .db
        .create_api_key(user_id, name, &key_hash, &key_prefix, expires_at)
        .await
        .map_err(|e| {
            // Never log the plaintext. Hash is also sensitive enough that we
            // avoid logging it on the success path.
            error!("Failed to persist API key: {}", e);
            ApiKeyError::InternalError { message: "Failed to create API key".into() }
        })?;

    debug!(api_key_id = %api_key.id, user_id = %user_id, "Created API key");

    Ok(Json(CreateApiKeyResponse {
        api_key: ApiKeyResponse::from(api_key),
        plaintext,
    }))
}

/// GET /api/auth/api-keys — list the caller's keys. Admins may pass `?all=true`
/// to see every user's keys for incident response.
pub async fn list_api_keys(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<ApiKeyResponse>>, ApiKeyError> {
    let is_admin = auth_user.user.role == UserRole::Admin;

    let keys: Vec<ApiKey> = if query.all && is_admin {
        state.db.list_all_api_keys().await
    } else {
        state.db.list_api_keys_for_user(auth_user.user.id).await
    }
    .map_err(|e| {
        error!("Failed to list API keys: {}", e);
        ApiKeyError::InternalError { message: "Failed to list API keys".into() }
    })?;

    Ok(Json(keys.into_iter().map(ApiKeyResponse::from).collect()))
}

/// DELETE /api/auth/api-keys/:id — revoke a key. Regular users may only revoke
/// their own keys; admins may revoke anyone's.
pub async fn revoke_api_key(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiKeyError> {
    let revoked = if auth_user.user.role == UserRole::Admin {
        state.db.admin_revoke_api_key(id).await
    } else {
        state.db.revoke_api_key(id, auth_user.user.id).await
    }
    .map_err(|e| {
        error!("Failed to revoke API key: {}", e);
        ApiKeyError::InternalError { message: "Failed to revoke API key".into() }
    })?;

    if revoked {
        debug!(api_key_id = %id, revoked_by = %auth_user.user.id, "Revoked API key");
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiKeyError::NotFound)
    }
}
