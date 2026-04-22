use anyhow::Result;
use axum::{
    extract::FromRequestParts,
    http::{request::Parts, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use uuid::Uuid;

use crate::{models::User, AppState};

/// Prefix that identifies a personal access token. Any `Authorization: Bearer`
/// value starting with this string is treated as an API key instead of a JWT.
/// The prefix doubles as a secret-scanning signal (e.g. for GitHub's scanner).
pub const API_KEY_PREFIX: &str = "readur_pat_";

/// Number of characters of the plaintext stored as `key_prefix` in the DB so
/// the UI can display an identifying fragment without revealing the full key.
/// Matches `readur_pat_` + 1 random character.
pub const API_KEY_DISPLAY_PREFIX_LEN: usize = 12;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub username: String,
    pub exp: usize,
}

pub struct AuthUser {
    pub user: User,
}

impl FromRequestParts<Arc<AppState>> for AuthUser {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let headers = &parts.headers;
        let token = extract_token_from_headers(headers)
            .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Missing authorization header").into_response())?;

        if token.starts_with(API_KEY_PREFIX) {
            return authenticate_api_key(&token, state).await;
        }

        let claims = verify_jwt(&token, &state.config.jwt_secret)
            .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid token").into_response())?;

        let user = state
            .db
            .get_user_by_id(claims.sub)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response())?
            .ok_or_else(|| (StatusCode::UNAUTHORIZED, "User not found").into_response())?;

        Ok(AuthUser { user })
    }
}

/// Authenticate a request presenting a `readur_pat_` API key. Returns 401 for
/// every failure mode (invalid, revoked, expired, user gone) so the caller
/// cannot distinguish between them — prevents enumeration.
async fn authenticate_api_key(token: &str, state: &Arc<AppState>) -> Result<AuthUser, Response> {
    let unauthorized = || (StatusCode::UNAUTHORIZED, "Invalid API key").into_response();

    let hash = sha256_hex(token);
    let api_key = state
        .db
        .get_api_key_by_hash(&hash)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response())?
        .ok_or_else(unauthorized)?;

    if api_key.is_expired() {
        return Err(unauthorized());
    }

    let user = state
        .db
        .get_user_by_id(api_key.user_id)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response())?
        .ok_or_else(unauthorized)?;

    // Fire-and-forget: the DB statement is throttled to at most one update per
    // key per 60 seconds, so this is safe even under heavy traffic.
    let db = state.db.clone();
    let key_id = api_key.id;
    tokio::spawn(async move {
        if let Err(e) = db.touch_api_key_last_used(key_id).await {
            tracing::warn!(api_key_id = %key_id, error = %e, "Failed to update api_key last_used_at");
        }
    });

    tracing::debug!(
        api_key_id = %api_key.id,
        user_id = %user.id,
        key_prefix = %api_key.key_prefix,
        "API key authenticated"
    );

    Ok(AuthUser { user })
}

/// Hex-encoded SHA-256 digest of `input`. Used to derive the `key_hash` column
/// value from an incoming plaintext API key. Hex output keeps the stored value
/// a stable 64-char string regardless of locale/encoding concerns.
pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let bytes = hasher.finalize();
    hex_encode(&bytes)
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

pub fn create_jwt(user: &User, secret: &str) -> Result<String> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::hours(24))
        .expect("valid timestamp")
        .timestamp();

    let claims = Claims {
        sub: user.id,
        username: user.username.clone(),
        exp: expiration as usize,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;

    Ok(token)
}

pub fn verify_jwt(token: &str, secret: &str) -> Result<Claims> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;

    Ok(token_data.claims)
}

fn extract_token_from_headers(headers: &HeaderMap) -> Option<String> {
    let auth_header = headers.get("authorization")?;
    let auth_str = auth_header.to_str().ok()?;

    if auth_str.starts_with("Bearer ") {
        Some(auth_str.trim_start_matches("Bearer ").to_string())
    } else {
        None
    }
}
