use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Json, Redirect, Response},
    routing::{get, post},
    Router,
};
use chrono::Duration;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, OnceLock};
use uuid::Uuid;

use crate::{
    auth::{create_jwt_with_ttl, sha256_hex, AuthUser},
    models::{
        ChangePasswordRequest, CreateUser, LoginRequest, LoginResponse, RegisterRequest, User,
        UserResponse, UserRole,
    },
    oidc::{OidcUserInfo, PendingLogin},
    utils::{
        client_ip::ClientIp,
        security::{constant_time_eq, validate_account_username, validate_email, validate_password},
    },
    AppState,
};

const OIDC_STATE_COOKIE: &str = "readur_oidc_state";
const DEFAULT_OIDC_COOKIE_PATH: &str = "/api/auth/oidc";
const EPHEMERAL_OIDC_STATE: &str = "oidc_state";
const EPHEMERAL_OIDC_HANDOFF: &str = "oidc_handoff";

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/password", post(change_password))
        .route("/me", get(me))
        .route("/config", get(get_auth_config))
        .route("/oidc/login", get(oidc_login))
        .route("/oidc/callback", get(oidc_callback))
        .route("/oidc/exchange", post(oidc_exchange))
        .nest("/keys", crate::routes::api_keys::router())
}

fn json_error(status: StatusCode, message: &str) -> Response {
    (status, Json(serde_json::json!({ "error": message }))).into_response()
}

fn rate_limited(retry_after: u64) -> Response {
    let mut response = json_error(StatusCode::TOO_MANY_REQUESTS, "Too many attempts, try again later");
    if let Ok(value) = HeaderValue::from_str(&retry_after.to_string()) {
        response.headers_mut().insert(header::RETRY_AFTER, value);
    }
    response
}

fn issue_session(state: &AppState, user: User) -> Response {
    match create_jwt_with_ttl(&user, &state.config.jwt_secret, state.config.security.jwt_ttl_hours) {
        Ok(token) => Json(LoginResponse { token, user: user.into() }).into_response(),
        Err(e) => {
            tracing::error!("Failed to create session token: {}", e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
        }
    }
}

#[derive(Serialize, utoipa::ToSchema)]
struct AuthConfig {
    allow_local_auth: bool,
    allow_registration: bool,
    oidc_enabled: bool,
    enable_per_user_watch: bool,
}

#[utoipa::path(
    post,
    path = "/api/auth/register",
    tag = "auth",
    request_body = RegisterRequest,
    responses(
        (status = 200, description = "Account created; it may require administrator approval before it can sign in", body = UserResponse),
        (status = 400, description = "Invalid data, or the account could not be created"),
        (status = 403, description = "Self-registration is disabled"),
        (status = 429, description = "Too many registrations from this client")
    )
)]
async fn register(
    State(state): State<Arc<AppState>>,
    client_ip: ClientIp,
    Json(request): Json<RegisterRequest>,
) -> Response {
    if !state.config.allow_local_auth.unwrap_or(true) || !state.config.security.allow_registration {
        return json_error(StatusCode::FORBIDDEN, "Self-registration is disabled");
    }

    if let Some(ip) = client_ip.0 {
        if let Err(retry_after) = state.rate_limiters.registration_by_ip.check(&ip).await {
            return rate_limited(retry_after);
        }
    }

    if let Err(message) = validate_account_username(&request.username)
        .and_then(|_| validate_email(&request.email))
        .and_then(|_| validate_password(&request.password))
    {
        return json_error(StatusCode::BAD_REQUEST, message);
    }

    let new_user = CreateUser {
        username: request.username,
        email: request.email,
        password: request.password,
        role: Some(UserRole::User),
    };
    let active = !state.config.security.registration_requires_approval;

    match state.db.create_user_with_status(new_user, active).await {
        Ok(user) => {
            if !active {
                tracing::info!(user_id = %user.id, "New account registered; awaiting administrator approval");
            }
            (StatusCode::OK, Json(UserResponse::from(user))).into_response()
        }
        Err(e) => {
            tracing::warn!("User registration failed: {}", e);
            json_error(
                StatusCode::BAD_REQUEST,
                "Could not create an account with the supplied username or email",
            )
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/auth/config",
    tag = "auth",
    responses(
        (status = 200, description = "Authentication configuration", body = AuthConfig),
    )
)]
async fn get_auth_config(State(state): State<Arc<AppState>>) -> Json<AuthConfig> {
    let allow_local_auth = state.config.allow_local_auth.unwrap_or(true);
    Json(AuthConfig {
        allow_local_auth,
        allow_registration: allow_local_auth && state.config.security.allow_registration,
        oidc_enabled: state.oidc_client.is_some(),
        enable_per_user_watch: state.config.enable_per_user_watch,
    })
}

/// Address used to key per-client login limits. Requests without connection
/// information (only in-process test harnesses) share a single placeholder
/// address, so the limits still apply to them as one client.
fn rate_limit_ip(client_ip: ClientIp) -> std::net::IpAddr {
    if client_ip.0.is_none() {
        tracing::debug!("Client address unknown; using placeholder address for rate limiting");
    }
    client_ip.or_unspecified()
}

/// A valid bcrypt hash of a random string, verified against when the user
/// doesn't exist so response time does not reveal which usernames exist.
fn dummy_password_hash() -> &'static str {
    static HASH: OnceLock<String> = OnceLock::new();
    HASH.get_or_init(|| {
        bcrypt::hash(crate::oidc::random_urlsafe(24), crate::db::users::BCRYPT_COST)
            .expect("bcrypt hash of random string")
    })
}

#[utoipa::path(
    post,
    path = "/api/auth/login",
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = LoginResponse),
        (status = 401, description = "Invalid credentials"),
        (status = 403, description = "Local login disabled, or account disabled / awaiting approval"),
        (status = 429, description = "Too many failed attempts")
    )
)]
async fn login(
    State(state): State<Arc<AppState>>,
    client_ip: ClientIp,
    Json(login_data): Json<LoginRequest>,
) -> Response {
    if !state.config.allow_local_auth.unwrap_or(true) {
        return json_error(StatusCode::FORBIDDEN, "Local login is disabled");
    }

    let attempt = match state
        .rate_limiters
        .reserve_password_attempt(&login_data.username, rate_limit_ip(client_ip), true)
        .await
    {
        Ok(attempt) => attempt,
        Err(retry_after) => return rate_limited(retry_after),
    };

    let user = match state.db.get_user_by_username(&login_data.username).await {
        Ok(user) => user,
        Err(e) => {
            tracing::error!("Database error during login: {}", e);
            attempt.succeeded(&state.rate_limiters).await;
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error");
        }
    };

    // Always run exactly one bcrypt verification (OIDC-only accounts have no
    // password hash and can never log in locally).
    let stored_hash = user.as_ref().and_then(|u| u.password_hash.as_deref());
    let password_ok = bcrypt::verify(&login_data.password, stored_hash.unwrap_or(dummy_password_hash()))
        .unwrap_or(false)
        && stored_hash.is_some();

    let user = match (user, password_ok) {
        (Some(user), true) => user,
        // A failed attempt stays counted.
        _ => return json_error(StatusCode::UNAUTHORIZED, "Invalid username or password"),
    };
    attempt.succeeded(&state.rate_limiters).await;

    if !user.is_active {
        return json_error(
            StatusCode::FORBIDDEN,
            "This account is disabled or awaiting administrator approval",
        );
    }

    issue_session(&state, user)
}

#[utoipa::path(
    post,
    path = "/api/auth/logout",
    tag = "auth",
    security(("bearer_auth" = [])),
    responses(
        (status = 204, description = "All of the user's sessions were revoked"),
        (status = 401, description = "Not authenticated")
    )
)]
async fn logout(State(state): State<Arc<AppState>>, auth_user: AuthUser) -> Response {
    match state.db.revoke_user_sessions(auth_user.user.id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            tracing::error!("Failed to revoke sessions: {}", e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/auth/password",
    tag = "auth",
    security(("bearer_auth" = [])),
    request_body = ChangePasswordRequest,
    responses(
        (status = 200, description = "Password changed; other sessions revoked; a fresh session is returned", body = LoginResponse),
        (status = 400, description = "New password rejected, or account has no local password"),
        (status = 401, description = "Current password incorrect"),
        (status = 429, description = "Too many attempts")
    )
)]
async fn change_password(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    client_ip: ClientIp,
    Json(request): Json<ChangePasswordRequest>,
) -> Response {
    let user = auth_user.user;
    let Some(current_hash) = user.password_hash.as_deref() else {
        return json_error(StatusCode::BAD_REQUEST, "This account does not use a local password");
    };

    let attempt = match state
        .rate_limiters
        .reserve_password_attempt(&user.username, rate_limit_ip(client_ip), false)
        .await
    {
        Ok(attempt) => attempt,
        Err(retry_after) => return rate_limited(retry_after),
    };
    if !bcrypt::verify(&request.current_password, current_hash).unwrap_or(false) {
        return json_error(StatusCode::UNAUTHORIZED, "Current password is incorrect");
    }
    attempt.succeeded(&state.rate_limiters).await;

    if let Err(message) = validate_password(&request.new_password) {
        return json_error(StatusCode::BAD_REQUEST, message);
    }

    match state.db.set_user_password(user.id, &request.new_password).await {
        Ok(updated) => issue_session(&state, updated),
        Err(e) => {
            tracing::error!("Failed to change password: {}", e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/auth/me",
    tag = "auth",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Current user information", body = UserResponse),
        (status = 401, description = "Unauthorized - invalid or missing token"),
        (status = 500, description = "Internal server error")
    )
)]
async fn me(auth_user: AuthUser) -> Json<UserResponse> {
    Json(auth_user.user.into())
}

// ---------------------------------------------------------------------------
// OIDC
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct OidcCallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

#[derive(Deserialize, utoipa::ToSchema)]
struct OidcExchangeRequest {
    code: String,
}

#[derive(Serialize, Deserialize)]
struct OidcHandoff {
    user_id: Uuid,
    token_version: i32,
}

/// Base URL of the web UI. Derived from configuration only — never from
/// request headers, which an attacker can influence.
fn frontend_base_url(state: &AppState) -> Option<String> {
    if let Some(public_url) = &state.config.public_url {
        return Some(public_url.trim_end_matches('/').to_string());
    }
    let redirect = url::Url::parse(state.config.oidc_redirect_uri.as_deref()?).ok()?;
    Some(redirect.origin().ascii_serialization())
}

fn oidc_cookie_is_secure(state: &AppState) -> bool {
    state
        .config
        .oidc_redirect_uri
        .as_deref()
        .map(|u| u.starts_with("https://"))
        .unwrap_or(true)
}

/// Path scope of the OIDC state cookie: the directory of `OIDC_REDIRECT_URI`
/// (e.g. `/readur/api/auth/oidc` when served under `/readur`), so the cookie
/// reaches the callback behind a path prefix. Falls back to `/api/auth/oidc`.
fn oidc_cookie_path(redirect_uri: Option<&str>) -> String {
    let parent = redirect_uri
        .and_then(|uri| url::Url::parse(uri).ok())
        .and_then(|url| url.path().rsplit_once('/').map(|(parent, _)| parent.to_string()));
    match parent {
        Some(path)
            if path.starts_with('/') && path.bytes().all(|b| b.is_ascii_graphic() && b != b';' && b != b',') =>
        {
            path
        }
        _ => DEFAULT_OIDC_COOKIE_PATH.to_string(),
    }
}

fn oidc_state_cookie(state: &AppState, value: &str, max_age_secs: i64) -> String {
    format!(
        "{}={}; Path={}; Max-Age={}; HttpOnly; SameSite=Lax{}",
        OIDC_STATE_COOKIE,
        value,
        oidc_cookie_path(state.config.oidc_redirect_uri.as_deref()),
        max_age_secs,
        if oidc_cookie_is_secure(state) { "; Secure" } else { "" }
    )
}

fn read_cookie<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(k, _)| *k == name)
        .map(|(_, v)| v)
}

#[utoipa::path(
    get,
    path = "/api/auth/oidc/login",
    tag = "auth",
    responses(
        (status = 302, description = "Redirect to OIDC provider"),
        (status = 400, description = "OIDC not configured"),
        (status = 429, description = "Too many login attempts from this client"),
        (status = 500, description = "Internal server error")
    )
)]
async fn oidc_login(State(state): State<Arc<AppState>>, client_ip: ClientIp) -> Response {
    let Some(oidc_client) = state.oidc_client.as_ref() else {
        return json_error(StatusCode::BAD_REQUEST, "OIDC is not configured");
    };

    // Each login start stores pending state server-side.
    if let Err(retry_after) = state.rate_limiters.auth_misc_by_ip.check(&rate_limit_ip(client_ip)).await {
        return rate_limited(retry_after);
    }

    let start = match oidc_client.begin_login() {
        Ok(start) => start,
        Err(e) => {
            tracing::error!("Failed to build OIDC authorization request: {}", e);
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error");
        }
    };

    let payload = match serde_json::to_value(&start.pending) {
        Ok(v) => v,
        Err(_) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error"),
    };
    if let Err(e) = state
        .db
        .put_auth_ephemeral(&start.state, EPHEMERAL_OIDC_STATE, &payload, Duration::minutes(10))
        .await
    {
        tracing::error!("Failed to persist OIDC login state: {}", e);
        return json_error(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error");
    }

    let cookie = oidc_state_cookie(&state, &start.state, 600);
    (
        [(header::SET_COOKIE, cookie)],
        Redirect::to(start.authorization_url.as_str()),
    )
        .into_response()
}

/// Reasons an OIDC callback can fail. Only these fixed codes are passed to
/// the web UI; details are logged server-side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OidcCallbackError {
    ProviderError,
    InvalidState,
    AuthFailed,
    NoAccount,
    AccountDisabled,
    ServerError,
}

impl OidcCallbackError {
    fn code(self) -> &'static str {
        match self {
            Self::ProviderError => "provider_error",
            Self::InvalidState => "invalid_state",
            Self::AuthFailed => "auth_failed",
            Self::NoAccount => "no_account",
            Self::AccountDisabled => "account_disabled",
            Self::ServerError => "server_error",
        }
    }

    fn status(self) -> StatusCode {
        match self {
            Self::ProviderError | Self::AuthFailed => StatusCode::UNAUTHORIZED,
            Self::InvalidState => StatusCode::BAD_REQUEST,
            Self::NoAccount | Self::AccountDisabled => StatusCode::FORBIDDEN,
            Self::ServerError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::ProviderError | Self::AuthFailed => "Authentication failed",
            Self::InvalidState => "Invalid or expired login state",
            Self::NoAccount => "No account is available for this identity. Contact your administrator.",
            Self::AccountDisabled => "This account is disabled or awaiting administrator approval",
            Self::ServerError => "Internal server error",
        }
    }
}

/// Send the browser back to the web UI with a fixed error code in the URL
/// fragment. Falls back to a JSON error when the UI address is unknown.
fn oidc_callback_failure(state: &AppState, error: OidcCallbackError) -> Response {
    let clear_cookie = [(header::SET_COOKIE, oidc_state_cookie(state, "", 0))];
    match frontend_base_url(state) {
        Some(base) => {
            let target = format!("{}/auth/callback#error={}", base, error.code());
            (clear_cookie, Redirect::to(&target)).into_response()
        }
        None => (clear_cookie, json_error(error.status(), error.message())).into_response(),
    }
}

#[utoipa::path(
    get,
    path = "/api/auth/oidc/callback",
    tag = "auth",
    responses(
        (status = 303, description = "Redirect to the web UI: `/auth/callback#code=<one-time code>` on success, or `/auth/callback#error=<code>` where code is one of provider_error, invalid_state, auth_failed, no_account, account_disabled, server_error"),
        (status = 400, description = "Missing or mismatched state / code (only when the web UI address is unknown)"),
        (status = 401, description = "Authentication with the provider failed (only when the web UI address is unknown)"),
        (status = 403, description = "No permitted local account for this identity (only when the web UI address is unknown)"),
        (status = 500, description = "Internal server error")
    )
)]
async fn oidc_callback(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<OidcCallbackQuery>,
) -> Response {
    match complete_oidc_callback(&state, &headers, params).await {
        Ok(response) => response,
        Err(error) => oidc_callback_failure(&state, error),
    }
}

async fn complete_oidc_callback(
    state: &Arc<AppState>,
    headers: &HeaderMap,
    params: OidcCallbackQuery,
) -> Result<Response, OidcCallbackError> {
    if let Some(error) = params.error {
        tracing::warn!("OIDC provider returned an error: {}", error);
        return Err(OidcCallbackError::ProviderError);
    }

    let (Some(code), Some(query_state)) = (params.code, params.state) else {
        return Err(OidcCallbackError::InvalidState);
    };

    // The state must match the value bound to this browser at login start
    // (prevents login CSRF), and must still be pending server-side.
    let cookie_matches = read_cookie(headers, OIDC_STATE_COOKIE)
        .map(|c| constant_time_eq(c.as_bytes(), query_state.as_bytes()))
        .unwrap_or(false);
    if !cookie_matches {
        tracing::warn!("OIDC callback state did not match the browser's state cookie");
        return Err(OidcCallbackError::InvalidState);
    }

    let pending: PendingLogin = match state.db.take_auth_ephemeral(&query_state, EPHEMERAL_OIDC_STATE).await {
        Ok(Some(value)) => serde_json::from_value(value).map_err(|_| OidcCallbackError::InvalidState)?,
        Ok(None) => return Err(OidcCallbackError::InvalidState),
        Err(e) => {
            tracing::error!("Failed to load OIDC login state: {}", e);
            return Err(OidcCallbackError::ServerError);
        }
    };

    let Some(oidc_client) = state.oidc_client.as_ref() else {
        return Err(OidcCallbackError::ServerError);
    };

    let user_info = oidc_client.complete_login(&code, &pending).await.map_err(|e| {
        tracing::warn!("OIDC login failed: {}", e);
        OidcCallbackError::AuthFailed
    })?;

    let user = resolve_oidc_user(state, &user_info).await?;

    if !user.is_active {
        return Err(OidcCallbackError::AccountDisabled);
    }

    // Hand the session to the SPA via a short-lived single-use code in the
    // URL fragment (never sent to servers or leaked via Referer). The SPA
    // redeems it at /oidc/exchange.
    let Some(base) = frontend_base_url(state) else {
        tracing::error!("Cannot determine frontend URL: set PUBLIC_URL or a valid OIDC_REDIRECT_URI");
        return Err(OidcCallbackError::ServerError);
    };

    let handoff_code = crate::oidc::random_urlsafe(32);
    let handoff = OidcHandoff { user_id: user.id, token_version: user.token_version };
    let stored = match serde_json::to_value(&handoff) {
        Ok(payload) => state
            .db
            .put_auth_ephemeral(&handoff_code, EPHEMERAL_OIDC_HANDOFF, &payload, Duration::seconds(60))
            .await
            .map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    };
    if let Err(e) = stored {
        tracing::error!("Failed to persist OIDC handoff: {}", e);
        return Err(OidcCallbackError::ServerError);
    }

    tracing::info!(user_id = %user.id, "OIDC authentication successful");
    let clear_cookie = [(header::SET_COOKIE, oidc_state_cookie(state, "", 0))];
    let redirect_url = format!("{}/auth/callback#code={}", base, handoff_code);
    Ok((clear_cookie, Redirect::to(&redirect_url)).into_response())
}

#[utoipa::path(
    post,
    path = "/api/auth/oidc/exchange",
    tag = "auth",
    request_body = OidcExchangeRequest,
    responses(
        (status = 200, description = "Session issued", body = LoginResponse),
        (status = 401, description = "Code invalid, expired or already used"),
        (status = 429, description = "Too many attempts")
    )
)]
async fn oidc_exchange(
    State(state): State<Arc<AppState>>,
    client_ip: ClientIp,
    Json(request): Json<OidcExchangeRequest>,
) -> Response {
    if let Some(ip) = client_ip.0 {
        if let Err(retry_after) = state.rate_limiters.auth_misc_by_ip.check(&ip).await {
            return rate_limited(retry_after);
        }
    }

    let invalid = || json_error(StatusCode::UNAUTHORIZED, "Invalid or expired code");

    let handoff: OidcHandoff = match state.db.take_auth_ephemeral(&request.code, EPHEMERAL_OIDC_HANDOFF).await {
        Ok(Some(value)) => match serde_json::from_value(value) {
            Ok(h) => h,
            Err(_) => return invalid(),
        },
        Ok(None) => return invalid(),
        Err(e) => {
            tracing::error!("Failed to load OIDC handoff: {}", e);
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error");
        }
    };

    match state.db.get_user_by_id(handoff.user_id).await {
        Ok(Some(user)) if user.is_active && user.token_version == handoff.token_version => {
            issue_session(&state, user)
        }
        Ok(_) => invalid(),
        Err(e) => {
            tracing::error!("Database error during OIDC exchange: {}", e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
        }
    }
}

/// Map a verified external identity to a local account.
///
/// 1. An account already bound to (issuer, subject) is used.
/// 2. Otherwise, an existing account with the same email is only linked when
///    `OIDC_LINK_EXISTING_BY_EMAIL=true` and the provider asserts the email
///    is verified, and that account is not bound to another identity.
/// 3. Otherwise a new account is created if `OIDC_AUTO_REGISTER=true`.
async fn resolve_oidc_user(state: &Arc<AppState>, info: &OidcUserInfo) -> Result<User, OidcCallbackError> {
    let internal = || OidcCallbackError::ServerError;
    let forbidden = || OidcCallbackError::NoAccount;

    let Some(issuer) = state.oidc_client.as_ref().map(|c| c.get_discovery().issuer.clone()) else {
        return Err(internal());
    };

    match state.db.get_user_by_oidc_subject(&info.sub, &issuer).await {
        Ok(Some(user)) => return Ok(user),
        Ok(None) => {}
        Err(e) => {
            tracing::error!("Database error during OIDC lookup: {}", e);
            return Err(internal());
        }
    }

    // Accounts created by earlier releases stored the issuer exactly as
    // configured (OIDC_ISSUER_URL) rather than the discovered issuer. Find
    // those and migrate them to the discovered value.
    let configured = state.config.oidc_issuer_url.as_deref().unwrap_or_default();
    for legacy_issuer in legacy_issuer_variants(configured, &issuer) {
        match state.db.get_user_by_oidc_subject(&info.sub, &legacy_issuer).await {
            Ok(Some(user)) => {
                return state
                    .db
                    .update_user_oidc_issuer(user.id, &issuer)
                    .await
                    .map(|user| {
                        tracing::info!(user_id = %user.id, "Updated stored OIDC issuer to the discovered value");
                        user
                    })
                    .map_err(|e| {
                        tracing::error!("Failed to update stored OIDC issuer: {}", e);
                        internal()
                    });
            }
            Ok(None) => {}
            Err(e) => {
                tracing::error!("Database error during OIDC lookup: {}", e);
                return Err(internal());
            }
        }
    }

    if let Some(email) = info.email.as_deref() {
        match state.db.get_user_by_email(email).await {
            Ok(Some(existing)) => {
                let may_link = state.config.security.oidc_link_existing_by_email
                    && info.email_verified == Some(true)
                    && existing.oidc_subject.is_none();
                if !may_link {
                    tracing::warn!(
                        user_id = %existing.id,
                        "OIDC identity matches an existing account's email but linking is not permitted"
                    );
                    return Err(forbidden());
                }
                return state
                    .db
                    .link_user_to_oidc(existing.id, &info.sub, &issuer, email)
                    .await
                    .map(|user| {
                        tracing::info!(user_id = %user.id, "Linked existing account to OIDC identity");
                        user
                    })
                    .map_err(|e| {
                        tracing::error!("Failed to link account to OIDC identity: {}", e);
                        internal()
                    });
            }
            Ok(None) => {}
            Err(e) => {
                tracing::error!("Database error during OIDC email lookup: {}", e);
                return Err(internal());
            }
        }
    }

    if !state.config.oidc_auto_register.unwrap_or(false) {
        tracing::warn!("OIDC login for unknown identity rejected: auto-registration is disabled");
        return Err(forbidden());
    }

    create_new_oidc_user(state, info, &issuer).await
}

/// Issuer strings under which earlier releases may have stored an identity:
/// the configured issuer URL with and without a trailing slash, excluding the
/// discovered issuer (which is looked up first).
fn legacy_issuer_variants(configured: &str, discovered: &str) -> Vec<String> {
    let configured = configured.trim();
    if configured.is_empty() {
        return Vec::new();
    }
    let base = configured.trim_end_matches('/');
    let mut variants: Vec<String> = Vec::new();
    for candidate in [configured.to_string(), base.to_string(), format!("{}/", base)] {
        if candidate != discovered && !variants.contains(&candidate) {
            variants.push(candidate);
        }
    }
    variants
}

/// Deterministic fallback username derived from the external identity.
fn fallback_oidc_username(issuer: &str, sub: &str) -> String {
    format!("oidc_{}", &sha256_hex(&format!("{}|{}", issuer, sub))[..12])
}

async fn create_new_oidc_user(
    state: &Arc<AppState>,
    info: &OidcUserInfo,
    issuer: &str,
) -> Result<User, OidcCallbackError> {
    let fallback = fallback_oidc_username(issuer, &info.sub);

    // Prefer the provider's username when it is valid and not already taken;
    // never attach to an existing account by username.
    let mut username = info
        .preferred_username
        .clone()
        .or_else(|| info.email.clone())
        .filter(|u| validate_account_username(u).is_ok())
        .unwrap_or_else(|| fallback.clone());
    match state.db.get_user_by_username(&username).await {
        Ok(Some(_)) => username = fallback,
        Ok(None) => {}
        Err(e) => {
            tracing::error!("Database error while checking OIDC username: {}", e);
            return Err(OidcCallbackError::ServerError);
        }
    }

    let user_email = info
        .email
        .clone()
        .unwrap_or_else(|| format!("{}@oidc.invalid", username));

    let create_user = CreateUser {
        username,
        email: user_email.clone(),
        password: String::new(), // Not used for OIDC users
        role: Some(UserRole::User),
    };

    match state.db.create_oidc_user(create_user, &info.sub, issuer, &user_email).await {
        Ok(user) => {
            tracing::info!(user_id = %user.id, "Created account for new OIDC identity");
            Ok(user)
        }
        Err(e) => {
            tracing::error!("Failed to create OIDC user: {:#}", e);
            Err(OidcCallbackError::NoAccount)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cookie_parsing() {
        let mut h = HeaderMap::new();
        h.insert(header::COOKIE, "a=1; readur_oidc_state=xyz; b=2".parse().unwrap());
        assert_eq!(read_cookie(&h, OIDC_STATE_COOKIE), Some("xyz"));
        assert_eq!(read_cookie(&h, "missing"), None);
    }

    #[test]
    fn oidc_cookie_path_follows_the_redirect_uri() {
        assert_eq!(oidc_cookie_path(Some("https://h/api/auth/oidc/callback")), "/api/auth/oidc");
        assert_eq!(oidc_cookie_path(Some("https://h/readur/api/auth/oidc/callback")), "/readur/api/auth/oidc");
        assert_eq!(oidc_cookie_path(Some("https://h/readur/api/auth/oidc/callback?x=1")), "/readur/api/auth/oidc");
        // Unusable values fall back to the default path.
        for uri in [None, Some("not a url"), Some("https://h/callback"), Some("https://h/a;b/callback")] {
            assert_eq!(oidc_cookie_path(uri), DEFAULT_OIDC_COOKIE_PATH, "{uri:?}");
        }
    }

    #[test]
    fn fallback_username_is_valid_and_stable() {
        let a = fallback_oidc_username("https://idp", "1");
        assert_eq!(a, fallback_oidc_username("https://idp", "1"));
        assert_ne!(a, fallback_oidc_username("https://idp", "2"));
        assert!(validate_account_username(&a).is_ok());
    }

    #[test]
    fn legacy_issuer_variants_cover_trailing_slash_forms() {
        let v = legacy_issuer_variants("https://idp.example/realms/x", "https://idp.example/realms/x");
        assert_eq!(v, vec!["https://idp.example/realms/x/".to_string()]);

        let v = legacy_issuer_variants("https://idp.example/", "https://idp.example");
        assert_eq!(v, vec!["https://idp.example/".to_string()]);

        assert!(legacy_issuer_variants("", "https://idp.example").is_empty());
    }
}
