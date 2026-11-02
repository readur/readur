use anyhow::Result;
use axum::{
    extract::FromRequestParts,
    http::{request::Parts, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    models::{User, UserRole},
    AppState,
};

/// Prefix that identifies a personal access token. Any `Authorization: Bearer`
/// value starting with this string is treated as an API key instead of a JWT.
/// The prefix doubles as a secret-scanning signal (e.g. for GitHub's scanner).
pub const API_KEY_PREFIX: &str = "readur_pat_";

/// Number of characters of the plaintext stored as `key_prefix` in the DB so
/// the UI can display an identifying fragment without revealing the full key.
/// Matches `readur_pat_` + 1 random character.
pub const API_KEY_DISPLAY_PREFIX_LEN: usize = 12;

/// `iss` claim of every session token issued by this server.
pub const JWT_ISSUER: &str = "readur";
/// `aud` claim of every session token issued by this server.
pub const JWT_AUDIENCE: &str = "readur-api";
/// Session lifetime used when no explicit TTL is configured.
pub const DEFAULT_JWT_TTL_HOURS: i64 = 12;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub username: String,
    pub exp: usize,
    pub iat: usize,
    pub iss: String,
    pub aud: String,
    /// The user's `token_version` at issue time. Tokens whose version no
    /// longer matches the database are rejected.
    pub ver: i32,
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
        let token = extract_token_from_headers(&parts.headers)
            .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Missing authorization header").into_response())?;

        let user = authenticate_token(&token, state).await?;
        Ok(AuthUser { user })
    }
}

/// An authenticated user holding the admin role. Rejects everyone else with
/// 403 (or 401 when unauthenticated).
pub struct AdminUser {
    pub user: User,
}

impl FromRequestParts<Arc<AppState>> for AdminUser {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let AuthUser { user } = AuthUser::from_request_parts(parts, state).await?;
        if user.role != UserRole::Admin {
            return Err((StatusCode::FORBIDDEN, "Admin access required").into_response());
        }
        Ok(AdminUser { user })
    }
}

impl From<AdminUser> for AuthUser {
    fn from(admin: AdminUser) -> Self {
        AuthUser { user: admin.user }
    }
}

/// Authenticate a bearer credential (session JWT or `readur_pat_` API key)
/// and return the user it belongs to. Every failure mode maps to the same 401
/// so callers cannot distinguish them.
pub async fn authenticate_token(token: &str, state: &Arc<AppState>) -> Result<User, Response> {
    if token.starts_with(API_KEY_PREFIX) {
        return authenticate_api_key(token, state).await;
    }

    let unauthorized = || (StatusCode::UNAUTHORIZED, "Invalid token").into_response();

    let claims = verify_jwt(token, &state.config.jwt_secret).map_err(|_| unauthorized())?;

    let user = state
        .db
        .get_user_by_id(claims.sub)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response())?
        .ok_or_else(unauthorized)?;

    if !user.is_active || claims.ver != user.token_version {
        return Err(unauthorized());
    }

    Ok(user)
}

/// Authenticate a request presenting a `readur_pat_` API key. Returns 401 for
/// every failure mode (invalid, revoked, expired, user gone or deactivated)
/// so the caller cannot distinguish between them — prevents enumeration.
async fn authenticate_api_key(token: &str, state: &Arc<AppState>) -> Result<User, Response> {
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

    if !user.is_active {
        return Err(unauthorized());
    }

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

    Ok(user)
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

pub(crate) fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

/// Issue a session token with the default lifetime.
pub fn create_jwt(user: &User, secret: &str) -> Result<String> {
    create_jwt_with_ttl(user, secret, DEFAULT_JWT_TTL_HOURS)
}

/// Issue a session token valid for `ttl_hours`.
pub fn create_jwt_with_ttl(user: &User, secret: &str, ttl_hours: i64) -> Result<String> {
    let now = Utc::now();
    let expiration = now
        .checked_add_signed(Duration::hours(ttl_hours))
        .expect("valid timestamp")
        .timestamp();

    let claims = Claims {
        sub: user.id,
        username: user.username.clone(),
        exp: expiration as usize,
        iat: now.timestamp() as usize,
        iss: JWT_ISSUER.to_string(),
        aud: JWT_AUDIENCE.to_string(),
        ver: user.token_version,
    };

    let token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;

    Ok(token)
}

/// Verify signature, algorithm, expiry, issuer and audience of a session
/// token. Revocation (`ver`) and account state are checked by
/// [`authenticate_token`], which has database access.
pub fn verify_jwt(token: &str, secret: &str) -> Result<Claims> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.leeway = 30;
    validation.set_issuer(&[JWT_ISSUER]);
    validation.set_audience(&[JWT_AUDIENCE]);
    validation.set_required_spec_claims(&["exp", "iat", "iss", "aud", "sub"]);

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )?;

    Ok(token_data.claims)
}

/// Extract the credential from an `Authorization: Bearer <token>` header.
pub fn extract_token_from_headers(headers: &HeaderMap) -> Option<String> {
    let auth_header = headers.get("authorization")?;
    let auth_str = auth_header.to_str().ok()?;

    let (scheme, token) = auth_str.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("bearer") {
        return None;
    }
    let token = token.trim();
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::AuthProvider;

    fn user(version: i32) -> User {
        User {
            id: Uuid::new_v4(),
            username: "u".into(),
            email: "u@example.com".into(),
            password_hash: None,
            role: UserRole::User,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            oidc_subject: None,
            oidc_issuer: None,
            oidc_email: None,
            auth_provider: AuthProvider::Local,
            token_version: version,
            is_active: true,
        }
    }

    const SECRET: &str = "0123456789abcdef0123456789abcdef";

    #[test]
    fn roundtrip_carries_version() {
        let token = create_jwt(&user(7), SECRET).unwrap();
        let claims = verify_jwt(&token, SECRET).unwrap();
        assert_eq!(claims.ver, 7);
        assert_eq!(claims.iss, JWT_ISSUER);
        assert_eq!(claims.aud, JWT_AUDIENCE);
    }

    #[test]
    fn rejects_foreign_issuer_and_audience() {
        let now = Utc::now().timestamp() as usize;
        let mk = |iss: &str, aud: &str| {
            let claims = Claims {
                sub: Uuid::new_v4(),
                username: "u".into(),
                exp: now + 3600,
                iat: now,
                iss: iss.into(),
                aud: aud.into(),
                ver: 0,
            };
            encode(&Header::default(), &claims, &EncodingKey::from_secret(SECRET.as_bytes())).unwrap()
        };
        assert!(verify_jwt(&mk("other", JWT_AUDIENCE), SECRET).is_err());
        assert!(verify_jwt(&mk(JWT_ISSUER, "other"), SECRET).is_err());
        assert!(verify_jwt(&mk(JWT_ISSUER, JWT_AUDIENCE), SECRET).is_ok());
    }

    #[test]
    fn rejects_legacy_tokens_without_new_claims() {
        #[derive(Serialize)]
        struct Legacy {
            sub: Uuid,
            username: String,
            exp: usize,
        }
        let legacy = Legacy {
            sub: Uuid::new_v4(),
            username: "u".into(),
            exp: Utc::now().timestamp() as usize + 3600,
        };
        let token = encode(&Header::default(), &legacy, &EncodingKey::from_secret(SECRET.as_bytes())).unwrap();
        assert!(verify_jwt(&token, SECRET).is_err());
    }

    #[test]
    fn bearer_parsing() {
        let mut h = HeaderMap::new();
        h.insert("authorization", "Bearer abc".parse().unwrap());
        assert_eq!(extract_token_from_headers(&h).as_deref(), Some("abc"));
        h.insert("authorization", "bearer abc".parse().unwrap());
        assert_eq!(extract_token_from_headers(&h).as_deref(), Some("abc"));
        h.insert("authorization", "Basic abc".parse().unwrap());
        assert!(extract_token_from_headers(&h).is_none());
        h.insert("authorization", "Bearer ".parse().unwrap());
        assert!(extract_token_from_headers(&h).is_none());
    }
}
