use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response, Redirect},
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    auth::{create_jwt, AuthUser},
    models::{CreateUser, LoginRequest, LoginResponse, UserResponse, UserRole},
    AppState,
};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/me", get(me))
        .route("/oidc/login", get(oidc_login))
        .route("/oidc/callback", get(oidc_callback))
}

#[utoipa::path(
    post,
    path = "/api/auth/register",
    tag = "auth",
    request_body = CreateUser,
    responses(
        (status = 200, description = "User registered successfully", body = UserResponse),
        (status = 400, description = "Bad request - username/email already exists or invalid data"),
        (status = 500, description = "Internal server error")
    )
)]
async fn register(
    State(state): State<Arc<AppState>>,
    Json(user_data): Json<CreateUser>,
) -> Response {
    match state.db.create_user(user_data).await {
        Ok(user) => {
            let user_response: UserResponse = user.into();
            (StatusCode::OK, Json(user_response)).into_response()
        }
        Err(e) => {
            tracing::error!("User registration failed: {}", e);
            
            // Check for specific database constraint violations
            let error_message = if e.to_string().contains("users_username_key") {
                "Username already exists"
            } else if e.to_string().contains("users_email_key") {
                "Email already exists"
            } else if e.to_string().contains("duplicate key") {
                "User with this username or email already exists"
            } else {
                "Registration failed due to invalid data"
            };
            
            (
                StatusCode::BAD_REQUEST, 
                Json(serde_json::json!({
                    "error": error_message,
                    "details": e.to_string()
                }))
            ).into_response()
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/auth/login",
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = LoginResponse),
        (status = 401, description = "Unauthorized - invalid credentials"),
        (status = 500, description = "Internal server error")
    )
)]
async fn login(
    State(state): State<Arc<AppState>>,
    Json(login_data): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    let user = state
        .db
        .get_user_by_username(&login_data.username)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let password_hash = user.password_hash
        .as_ref()
        .ok_or(StatusCode::UNAUTHORIZED)?; // OIDC users don't have passwords
        
    let is_valid = bcrypt::verify(&login_data.password, password_hash)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !is_valid {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = create_jwt(&user, &state.config.jwt_secret)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(LoginResponse {
        token,
        user: user.into(),
    }))
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

#[derive(Deserialize)]
struct OidcCallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/auth/oidc/login",
    tag = "auth",
    responses(
        (status = 302, description = "Redirect to OIDC provider"),
        (status = 400, description = "OIDC not configured"),
        (status = 500, description = "Internal server error")
    )
)]
async fn oidc_login(State(state): State<Arc<AppState>>) -> Result<Redirect, StatusCode> {
    let oidc_client = state
        .oidc_client
        .as_ref()
        .ok_or(StatusCode::BAD_REQUEST)?;

    let (auth_url, _csrf_token) = oidc_client.get_authorization_url();
    
    Ok(Redirect::to(auth_url.as_str()))
}

#[utoipa::path(
    get,
    path = "/api/auth/oidc/callback",
    tag = "auth",
    responses(
        (status = 200, description = "OIDC authentication successful", body = LoginResponse),
        (status = 400, description = "Bad request - missing or invalid parameters"),
        (status = 401, description = "Authentication failed"),
        (status = 500, description = "Internal server error")
    )
)]
async fn oidc_callback(
    State(state): State<Arc<AppState>>,
    Query(params): Query<OidcCallbackQuery>,
) -> Result<Json<LoginResponse>, StatusCode> {
    if let Some(error) = params.error {
        tracing::error!("OIDC callback error: {}", error);
        return Err(StatusCode::UNAUTHORIZED);
    }

    let code = params.code.ok_or(StatusCode::BAD_REQUEST)?;
    
    let oidc_client = state
        .oidc_client
        .as_ref()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    // Exchange authorization code for access token
    let access_token = oidc_client
        .exchange_code(&code)
        .await
        .map_err(|e| {
            tracing::error!("Failed to exchange code: {}", e);
            StatusCode::UNAUTHORIZED
        })?;

    // Get user info from OIDC provider
    let user_info = oidc_client
        .get_user_info(&access_token)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get user info: {}", e);
            StatusCode::UNAUTHORIZED
        })?;

    // Find or create user in database
    let user = match state.db.get_user_by_oidc_subject(&user_info.sub, &state.config.oidc_issuer_url.as_ref().unwrap()).await {
        Ok(Some(existing_user)) => existing_user,
        Ok(None) => {
            // Create new user
            let username = user_info.preferred_username
                .or_else(|| user_info.email.clone())
                .unwrap_or_else(|| format!("oidc_user_{}", &user_info.sub[..8]));
            
            let email = user_info.email.unwrap_or_else(|| format!("{}@oidc.local", username));
            
            let create_user = CreateUser {
                username,
                email: email.clone(),
                password: "".to_string(), // Not used for OIDC users
                role: Some(UserRole::User),
            };
            
            state.db.create_oidc_user(
                create_user,
                &user_info.sub,
                &state.config.oidc_issuer_url.as_ref().unwrap(),
                &email,
            ).await.map_err(|e| {
                tracing::error!("Failed to create OIDC user: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
        }
        Err(e) => {
            tracing::error!("Database error during OIDC lookup: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Create JWT token
    let token = create_jwt(&user, &state.config.jwt_secret)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(LoginResponse {
        token,
        user: user.into(),
    }))
}