use axum::{
    extract::{Query, State},
    http::{StatusCode, HeaderMap},
    response::{IntoResponse, Json, Response, Redirect},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
    auth::{create_jwt, AuthUser},
    models::{CreateUser, LoginRequest, LoginResponse, User, UserResponse, UserRole},
    oidc::OidcUserInfo,
    AppState,
};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/me", get(me))
        .route("/config", get(get_auth_config))
        .route("/oidc/login", get(oidc_login))
        .route("/oidc/callback", get(oidc_callback))
}

#[derive(Serialize, utoipa::ToSchema)]
struct AuthConfig {
    allow_local_auth: bool,
    oidc_enabled: bool,
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
    // Check if local authentication is enabled
    if !state.config.allow_local_auth.unwrap_or(true) {
        tracing::warn!("Local registration attempt rejected - local auth is disabled");
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": "Local registration is disabled",
                "details": "This instance only allows OIDC authentication. Please contact your administrator."
            }))
        ).into_response();
    }

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
    get,
    path = "/api/auth/config",
    tag = "auth",
    responses(
        (status = 200, description = "Authentication configuration", body = AuthConfig),
    )
)]
async fn get_auth_config(
    State(state): State<Arc<AppState>>,
) -> Json<AuthConfig> {
    let allow_local_auth = state.config.allow_local_auth.unwrap_or(true);
    let oidc_enabled = state.oidc_client.is_some();

    Json(AuthConfig {
        allow_local_auth,
        oidc_enabled,
    })
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
    // Check if local authentication is enabled
    if !state.config.allow_local_auth.unwrap_or(true) {
        tracing::warn!("Local authentication attempt rejected - local auth is disabled");
        return Err(StatusCode::FORBIDDEN);
    }

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
    headers: HeaderMap,
    Query(params): Query<OidcCallbackQuery>,
) -> Result<Redirect, StatusCode> {
    tracing::info!("OIDC callback called with params: code={:?}, state={:?}, error={:?}", 
        params.code, params.state, params.error);
    
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
        .exchange_code(&code, params.state.as_deref())
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

    // Find or create user in database with email-based syncing
    let issuer_url = state.config.oidc_issuer_url.as_ref().unwrap();
    tracing::debug!("Looking up user by OIDC subject: {} and issuer: {}", user_info.sub, issuer_url);

    let user = match state.db.get_user_by_oidc_subject(&user_info.sub, issuer_url).await {
        Ok(Some(existing_user)) => {
            tracing::debug!("Found existing OIDC user: {}", existing_user.username);
            existing_user
        },
        Ok(None) => {
            // No OIDC user found, check if there's an existing local user with this email
            let email = user_info.email.clone();

            if let Some(email_addr) = &email {
                tracing::debug!("Checking for existing local user with email: {}", email_addr);
                match state.db.get_user_by_email(email_addr).await {
                    Ok(Some(existing_local_user)) => {
                        // Found existing local user with matching email - link to OIDC
                        tracing::info!(
                            "Found existing local user '{}' with email '{}', linking to OIDC identity",
                            existing_local_user.username,
                            email_addr
                        );

                        match state.db.link_user_to_oidc(
                            existing_local_user.id,
                            &user_info.sub,
                            issuer_url,
                            email_addr,
                        ).await {
                            Ok(linked_user) => {
                                tracing::info!(
                                    "Successfully linked user '{}' to OIDC identity",
                                    linked_user.username
                                );
                                linked_user
                            },
                            Err(e) => {
                                tracing::error!("Failed to link existing user to OIDC: {}", e);
                                return Err(StatusCode::INTERNAL_SERVER_ERROR);
                            }
                        }
                    },
                    Ok(None) => {
                        // No existing user with this email
                        if state.config.oidc_auto_register.unwrap_or(false) {
                            // Auto-registration is enabled, create new OIDC user
                            tracing::debug!("No existing user with this email, creating new OIDC user (auto-registration enabled)");
                            create_new_oidc_user(
                                &state,
                                &user_info,
                                issuer_url,
                                email.as_deref(),
                            ).await?
                        } else {
                            // Auto-registration is disabled, reject login
                            tracing::warn!(
                                "OIDC login attempted for unregistered email '{}', but auto-registration is disabled",
                                email_addr
                            );
                            return Err(StatusCode::FORBIDDEN);
                        }
                    },
                    Err(e) => {
                        tracing::error!("Database error during email lookup: {}", e);
                        return Err(StatusCode::INTERNAL_SERVER_ERROR);
                    }
                }
            } else {
                // No email provided by OIDC provider
                if state.config.oidc_auto_register.unwrap_or(false) {
                    // Auto-registration is enabled, create new user without email sync
                    tracing::debug!("No email provided by OIDC, creating new user (auto-registration enabled)");
                    create_new_oidc_user(
                        &state,
                        &user_info,
                        issuer_url,
                        None,
                    ).await?
                } else {
                    // Auto-registration is disabled and no email to sync
                    tracing::warn!(
                        "OIDC login attempted without email claim, but auto-registration is disabled"
                    );
                    return Err(StatusCode::FORBIDDEN);
                }
            }
        }
        Err(e) => {
            tracing::error!("Database error during OIDC lookup: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Create JWT token
    let token = create_jwt(&user, &state.config.jwt_secret)
        .map_err(|e| {
            tracing::error!("Failed to create JWT token: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Redirect to frontend with token in URL fragment
    // The frontend should extract the token and store it
    // Use absolute URL to ensure hash fragment is handled correctly by the browser
    let host = headers
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("localhost:8000");

    // Check if behind a proxy (X-Forwarded-Proto header)
    let protocol = headers
        .get("x-forwarded-proto")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("https");

    let redirect_url = format!(
        "{}://{}/auth/callback?token={}",
        protocol,
        host,
        urlencoding::encode(&token)
    );
    tracing::info!("OIDC authentication successful for user: {}, redirecting to callback", user.username);

    Ok(Redirect::to(&redirect_url))
}

// Helper function to create a new OIDC user
async fn create_new_oidc_user(
    state: &Arc<AppState>,
    user_info: &OidcUserInfo,
    issuer_url: &str,
    email: Option<&str>,
) -> Result<User, StatusCode> {
    tracing::debug!("Creating new OIDC user");

    let username = user_info.preferred_username
        .clone()
        .or_else(|| email.map(|e| e.to_string()))
        .unwrap_or_else(|| format!("oidc_user_{}", &user_info.sub[..8]));

    let user_email = email
        .map(|e| e.to_string())
        .unwrap_or_else(|| format!("{}@oidc.local", username));

    tracing::debug!("New user details - username: {}, email: {}", username, user_email);

    let create_user = CreateUser {
        username,
        email: user_email.clone(),
        password: "".to_string(), // Not used for OIDC users
        role: Some(UserRole::User),
    };

    let result = state.db.create_oidc_user(
        create_user,
        &user_info.sub,
        issuer_url,
        &user_email,
    ).await;

    match result {
        Ok(user) => {
            tracing::info!("Successfully created OIDC user: {}", user.username);
            Ok(user)
        },
        Err(e) => {
            tracing::error!("Failed to create OIDC user: {} (full error: {:#})", e, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}