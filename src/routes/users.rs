use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, delete},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use utoipa::ToSchema;

use crate::{
    auth::AuthUser,
    errors::user::UserError,
    models::{CreateUser, UpdateUser, UserResponse, UserRole},
    AppState,
};

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UserWatchDirectoryResponse {
    pub user_id: Uuid,
    pub username: String,
    pub watch_directory_path: String,
    pub exists: bool,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateUserWatchDirectoryRequest {
    pub ensure_created: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UserWatchDirectoryOperationResponse {
    pub success: bool,
    pub message: String,
    pub watch_directory_path: Option<String>,
}

fn require_admin(auth_user: &AuthUser) -> Result<(), UserError> {
    if auth_user.user.role != UserRole::Admin {
        Err(UserError::permission_denied("Admin access required"))
    } else {
        Ok(())
    }
}

fn can_access_user_data(auth_user: &AuthUser, target_user_id: Uuid) -> Result<(), UserError> {
    // Admin can access any user's data, users can only access their own
    if auth_user.user.role == UserRole::Admin || auth_user.user.id == target_user_id {
        Ok(())
    } else {
        Err(UserError::permission_denied("Cannot access other user's data"))
    }
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_users).post(create_user))
        .route("/{id}", get(get_user).put(update_user).delete(delete_user))
        .route("/{id}/watch-directory", get(get_user_watch_directory).post(create_user_watch_directory).delete(delete_user_watch_directory))
}

#[utoipa::path(
    get,
    path = "/api/users",
    tag = "users",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "List of all users", body = Vec<UserResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - Admin access required"),
        (status = 500, description = "Internal server error")
    )
)]
async fn list_users(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<UserResponse>>, UserError> {
    require_admin(&auth_user)?;
    let users = state
        .db
        .get_all_users()
        .await
        .map_err(|e| UserError::internal_server_error(format!("Failed to fetch users: {}", e)))?;

    let user_responses: Vec<UserResponse> = users.into_iter().map(|u| u.into()).collect();
    Ok(Json(user_responses))
}

#[utoipa::path(
    get,
    path = "/api/users/{id}",
    tag = "users",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = Uuid, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "User information", body = UserResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - Admin access required"),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_user(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<UserResponse>, UserError> {
    require_admin(&auth_user)?;
    let user = state
        .db
        .get_user_by_id(id)
        .await
        .map_err(|e| UserError::internal_server_error(format!("Failed to fetch user: {}", e)))?
        .ok_or_else(|| UserError::not_found_by_id(id))?;

    Ok(Json(user.into()))
}

#[utoipa::path(
    post,
    path = "/api/users",
    tag = "users",
    security(
        ("bearer_auth" = [])
    ),
    request_body = CreateUser,
    responses(
        (status = 200, description = "User created successfully", body = UserResponse),
        (status = 400, description = "Bad request - invalid user data"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - Admin access required"),
        (status = 500, description = "Internal server error")
    )
)]
async fn create_user(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
    Json(user_data): Json<CreateUser>,
) -> Result<Json<UserResponse>, UserError> {
    require_admin(&auth_user)?;
    
    let user = state
        .db
        .create_user(user_data)
        .await
        .map_err(|e| {
            let error_msg = e.to_string();
            if error_msg.contains("username") && error_msg.contains("unique") {
                UserError::duplicate_username(&error_msg)
            } else if error_msg.contains("email") && error_msg.contains("unique") {
                UserError::duplicate_email(&error_msg)
            } else {
                UserError::internal_server_error(format!("Failed to create user: {}", e))
            }
        })?;

    Ok(Json(user.into()))
}

#[utoipa::path(
    put,
    path = "/api/users/{id}",
    tag = "users",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = Uuid, Path, description = "User ID")
    ),
    request_body = UpdateUser,
    responses(
        (status = 200, description = "User updated successfully", body = UserResponse),
        (status = 400, description = "Bad request - invalid user data"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - Admin access required"),
        (status = 500, description = "Internal server error")
    )
)]
async fn update_user(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(update_data): Json<UpdateUser>,
) -> Result<Json<UserResponse>, UserError> {
    require_admin(&auth_user)?;
    
    let user = state
        .db
        .update_user(id, update_data.username, update_data.email, update_data.password)
        .await
        .map_err(|e| {
            let error_msg = e.to_string();
            if error_msg.contains("username") && error_msg.contains("unique") {
                UserError::duplicate_username(&error_msg)
            } else if error_msg.contains("email") && error_msg.contains("unique") {
                UserError::duplicate_email(&error_msg)
            } else if error_msg.contains("not found") {
                UserError::not_found_by_id(id)
            } else {
                UserError::internal_server_error(format!("Failed to update user: {}", e))
            }
        })?;

    Ok(Json(user.into()))
}

#[utoipa::path(
    delete,
    path = "/api/users/{id}",
    tag = "users",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = Uuid, Path, description = "User ID")
    ),
    responses(
        (status = 204, description = "User deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - Admin access required or cannot delete yourself"),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    )
)]
async fn delete_user(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, UserError> {
    require_admin(&auth_user)?;
    
    // Prevent users from deleting themselves
    if auth_user.user.id == id {
        return Err(UserError::delete_restricted(id, "Cannot delete your own account"));
    }

    state
        .db
        .delete_user(id)
        .await
        .map_err(|e| {
            let error_msg = e.to_string();
            if error_msg.contains("not found") {
                UserError::not_found_by_id(id)
            } else {
                UserError::internal_server_error(format!("Failed to delete user: {}", e))
            }
        })?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/api/users/{id}/watch-directory",
    tag = "users",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = Uuid, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "User watch directory information", body = UserWatchDirectoryResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - Admin access required or not your user account"),
        (status = 404, description = "User not found"),
        (status = 501, description = "Per-user watch directories are disabled"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_user_watch_directory(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<UserWatchDirectoryResponse>, UserError> {
    can_access_user_data(&auth_user, id)?;
    
    // Check if per-user watch is enabled
    if !state.config.enable_per_user_watch {
        return Err(UserError::internal_server_error("Per-user watch directories are not enabled".to_string()));
    }
    
    // Get the user
    let user = state
        .db
        .get_user_by_id(id)
        .await
        .map_err(|e| UserError::internal_server_error(format!("Failed to fetch user: {}", e)))?
        .ok_or_else(|| UserError::not_found_by_id(id))?;
    
    // Get the user watch service
    let user_watch_service = state
        .user_watch_service
        .as_ref()
        .ok_or_else(|| UserError::internal_server_error("User watch service not initialized".to_string()))?;
    
    // Get the watch directory path 
    let watch_directory_path = match user_watch_service.get_user_directory(user.id).await {
        Some(path) => path.to_string_lossy().to_string(),
        None => {
            // Try to construct the path manually if not cached
            let base_dir = std::path::Path::new(&state.config.user_watch_base_dir);
            base_dir.join(&user.username).to_string_lossy().to_string()
        }
    };
    
    // Check if directory exists
    let exists = tokio::fs::metadata(&watch_directory_path).await.is_ok();
    
    let response = UserWatchDirectoryResponse {
        user_id: user.id,
        username: user.username,
        watch_directory_path,
        exists,
        enabled: state.config.enable_per_user_watch,
    };
    
    Ok(Json(response))
}

#[utoipa::path(
    post,
    path = "/api/users/{id}/watch-directory",
    tag = "users",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = Uuid, Path, description = "User ID")
    ),
    request_body = CreateUserWatchDirectoryRequest,
    responses(
        (status = 200, description = "User watch directory created successfully", body = UserWatchDirectoryOperationResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - Admin access required or not your user account"),
        (status = 404, description = "User not found"),
        (status = 501, description = "Per-user watch directories are disabled"),
        (status = 500, description = "Internal server error")
    )
)]
async fn create_user_watch_directory(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(request): Json<CreateUserWatchDirectoryRequest>,
) -> Result<Json<UserWatchDirectoryOperationResponse>, UserError> {
    can_access_user_data(&auth_user, id)?;
    
    // Check if per-user watch is enabled
    if !state.config.enable_per_user_watch {
        return Err(UserError::internal_server_error("Per-user watch directories are not enabled".to_string()));
    }
    
    // Get the user
    let user = state
        .db
        .get_user_by_id(id)
        .await
        .map_err(|e| UserError::internal_server_error(format!("Failed to fetch user: {}", e)))?
        .ok_or_else(|| UserError::not_found_by_id(id))?;
    
    // Get the user watch service
    let user_watch_service = state
        .user_watch_service
        .as_ref()
        .ok_or_else(|| UserError::internal_server_error("User watch service not initialized".to_string()))?;
    
    // Create or ensure the directory exists
    let ensure_created = request.ensure_created.unwrap_or(true);
    
    let result = if ensure_created {
        user_watch_service.ensure_user_directory(&user).await
    } else {
        match user_watch_service.get_user_directory(user.id).await {
            Some(path) => Ok(path),
            None => {
                let base_dir = std::path::Path::new(&state.config.user_watch_base_dir);
                Ok(base_dir.join(&user.username))
            }
        }
    };
    
    match result {
        Ok(watch_directory_path) => {
            let response = UserWatchDirectoryOperationResponse {
                success: true,
                message: format!("Watch directory ready for user '{}'", user.username),
                watch_directory_path: Some(watch_directory_path.to_string_lossy().to_string()),
            };
            Ok(Json(response))
        }
        Err(e) => {
            let response = UserWatchDirectoryOperationResponse {
                success: false,
                message: format!("Failed to create watch directory: {}", e),
                watch_directory_path: None,
            };
            Ok(Json(response))
        }
    }
}

#[utoipa::path(
    delete,
    path = "/api/users/{id}/watch-directory",
    tag = "users",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = Uuid, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "User watch directory removed successfully", body = UserWatchDirectoryOperationResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - Admin access required"),
        (status = 404, description = "User not found"),
        (status = 501, description = "Per-user watch directories are disabled"),
        (status = 500, description = "Internal server error")
    )
)]
async fn delete_user_watch_directory(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<UserWatchDirectoryOperationResponse>, UserError> {
    require_admin(&auth_user)?; // Only admins can delete watch directories
    
    // Check if per-user watch is enabled
    if !state.config.enable_per_user_watch {
        return Err(UserError::internal_server_error("Per-user watch directories are not enabled".to_string()));
    }
    
    // Get the user
    let user = state
        .db
        .get_user_by_id(id)
        .await
        .map_err(|e| UserError::internal_server_error(format!("Failed to fetch user: {}", e)))?
        .ok_or_else(|| UserError::not_found_by_id(id))?;
    
    // Get the user watch service
    let user_watch_service = state
        .user_watch_service
        .as_ref()
        .ok_or_else(|| UserError::internal_server_error("User watch service not initialized".to_string()))?;
    
    // Remove the user's watch directory
    match user_watch_service.remove_user_directory(&user).await {
        Ok(_) => {
            let response = UserWatchDirectoryOperationResponse {
                success: true,
                message: format!("Watch directory removed for user '{}'", user.username),
                watch_directory_path: None,
            };
            Ok(Json(response))
        }
        Err(e) => {
            let response = UserWatchDirectoryOperationResponse {
                success: false,
                message: format!("Failed to remove watch directory: {}", e),
                watch_directory_path: None,
            };
            Ok(Json(response))
        }
    }
}