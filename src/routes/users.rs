use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use ts_rs::TS;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::{AdminUser, AuthUser},
    db::unique_violation::{
        violated_unique_constraint_in, USERS_EMAIL_UNIQUE, USERS_USERNAME_UNIQUE,
    },
    errors::user::UserError,
    models::{CreateUser, UpdateUser, UserResponse, UserRole},
    utils::security::{validate_account_username, validate_email, validate_password},
    AppState,
};

#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct UserWatchDirectoryResponse {
    pub user_id: Uuid,
    pub username: String,
    pub watch_directory_path: String,
    pub exists: bool,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export, optional_fields)]
pub struct CreateUserWatchDirectoryRequest {
    pub ensure_created: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct UserWatchDirectoryOperationResponse {
    pub success: bool,
    pub message: String,
    pub watch_directory_path: Option<String>,
}

/// Validate the account fields supplied to an admin create/update request.
fn validate_account_fields(
    username: Option<&str>,
    email: Option<&str>,
    password: Option<&str>,
) -> Result<(), UserError> {
    if let Some(username) = username {
        validate_account_username(username)
            .map_err(|reason| UserError::invalid_username(username, reason))?;
    }
    if let Some(email) = email {
        validate_email(email).map_err(|_| UserError::invalid_email(email))?;
    }
    if let Some(password) = password {
        validate_password(password).map_err(UserError::invalid_password)?;
    }
    Ok(())
}

/// Map a failed user insert/update to the REST error. A username or email clash is recognised
/// by SQLSTATE 23505 and the name of the unique constraint it hit, not by the error text.
fn map_user_write_error(
    e: anyhow::Error,
    id: Option<Uuid>,
    username: &str,
    email: &str,
) -> UserError {
    match violated_unique_constraint_in(&e) {
        Some(USERS_USERNAME_UNIQUE) => return UserError::duplicate_username(username),
        Some(USERS_EMAIL_UNIQUE) => return UserError::duplicate_email(email),
        _ => {}
    }
    match id {
        // `Database::update_user` reports a user deleted since the handler looked it up this way.
        Some(id) if e.to_string().contains("not found") => UserError::not_found_by_id(id),
        _ => UserError::internal_server_error(format!("Failed to save user: {}", e)),
    }
}

fn can_access_user_data(auth_user: &AuthUser, target_user_id: Uuid) -> Result<(), UserError> {
    // Admin can access any user's data, users can only access their own
    if auth_user.user.role == UserRole::Admin || auth_user.user.id == target_user_id {
        Ok(())
    } else {
        Err(UserError::permission_denied(
            "Cannot access other user's data",
        ))
    }
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_users).post(create_user))
        .route("/{id}", get(get_user).put(update_user).delete(delete_user))
        .route(
            "/{id}/watch/directory",
            get(get_user_watch_directory)
                .post(create_user_watch_directory)
                .delete(delete_user_watch_directory),
        )
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
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<UserResponse>>, UserError> {
    let users =
        state.db.get_all_users().await.map_err(|e| {
            UserError::internal_server_error(format!("Failed to fetch users: {}", e))
        })?;

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
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<UserResponse>, UserError> {
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
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Json(user_data): Json<CreateUser>,
) -> Result<Json<UserResponse>, UserError> {
    validate_account_fields(
        Some(&user_data.username),
        Some(&user_data.email),
        Some(&user_data.password),
    )?;

    let (username, email) = (user_data.username.clone(), user_data.email.clone());
    let user = state
        .db
        .create_user(user_data)
        .await
        .map_err(|e| map_user_write_error(e, None, &username, &email))?;

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
    admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(update_data): Json<UpdateUser>,
) -> Result<Json<UserResponse>, UserError> {
    let existing = state
        .db
        .get_user_by_id(id)
        .await
        .map_err(|e| UserError::internal_server_error(format!("Failed to fetch user: {}", e)))?
        .ok_or_else(|| UserError::not_found_by_id(id))?;

    // Only fields that actually change are validated, so accounts whose
    // username or email predates the current rules stay editable.
    validate_account_fields(
        update_data
            .username
            .as_deref()
            .filter(|u| *u != existing.username),
        update_data
            .email
            .as_deref()
            .filter(|e| *e != existing.email),
        update_data.password.as_deref(),
    )?;

    // An admin disabling their own account would lock themselves out (and
    // possibly leave the instance without an active admin).
    if admin.user.id == id && update_data.is_active == Some(false) {
        return Err(UserError::permission_denied(
            "Cannot deactivate your own account",
        ));
    }

    let username = update_data
        .username
        .clone()
        .unwrap_or_else(|| existing.username.clone());
    let email = update_data
        .email
        .clone()
        .unwrap_or_else(|| existing.email.clone());
    let user = state
        .db
        .update_user(
            id,
            update_data.username,
            update_data.email,
            update_data.password,
            update_data.is_active,
        )
        .await
        .map_err(|e| map_user_write_error(e, Some(id), &username, &email))?;

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
    admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, UserError> {
    // Prevent users from deleting themselves
    if admin.user.id == id {
        return Err(UserError::delete_restricted(
            id,
            "Cannot delete your own account",
        ));
    }

    state.db.delete_user(id).await.map_err(|e| {
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
    path = "/api/users/{id}/watch/directory",
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
        return Err(UserError::internal_server_error(
            "Per-user watch directories are not enabled".to_string(),
        ));
    }

    // Get the user
    let user = state
        .db
        .get_user_by_id(id)
        .await
        .map_err(|e| UserError::internal_server_error(format!("Failed to fetch user: {}", e)))?
        .ok_or_else(|| UserError::not_found_by_id(id))?;

    // Get the user watch service
    let user_watch_service = state.user_watch_service.as_ref().ok_or_else(|| {
        UserError::internal_server_error("User watch service not initialized".to_string())
    })?;

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
    path = "/api/users/{id}/watch/directory",
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
        return Err(UserError::internal_server_error(
            "Per-user watch directories are not enabled".to_string(),
        ));
    }

    // Get the user
    let user = state
        .db
        .get_user_by_id(id)
        .await
        .map_err(|e| UserError::internal_server_error(format!("Failed to fetch user: {}", e)))?
        .ok_or_else(|| UserError::not_found_by_id(id))?;

    // Get the user watch service
    let user_watch_service = state.user_watch_service.as_ref().ok_or_else(|| {
        UserError::internal_server_error("User watch service not initialized".to_string())
    })?;

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
    path = "/api/users/{id}/watch/directory",
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
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<UserWatchDirectoryOperationResponse>, UserError> {
    // Check if per-user watch is enabled
    if !state.config.enable_per_user_watch {
        return Err(UserError::internal_server_error(
            "Per-user watch directories are not enabled".to_string(),
        ));
    }

    // Get the user
    let user = state
        .db
        .get_user_by_id(id)
        .await
        .map_err(|e| UserError::internal_server_error(format!("Failed to fetch user: {}", e)))?
        .ok_or_else(|| UserError::not_found_by_id(id))?;

    // Get the user watch service
    let user_watch_service = state.user_watch_service.as_ref().ok_or_else(|| {
        UserError::internal_server_error("User watch service not initialized".to_string())
    })?;

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
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::unique_violation::fake::{db_error, unique_violation};
    use crate::errors::AppError;

    fn map(error: sqlx::Error, id: Option<Uuid>) -> UserError {
        map_user_write_error(anyhow::Error::from(error), id, "alice", "alice@example.com")
    }

    #[test]
    fn username_clash_is_classified_by_constraint_not_text() {
        // No "username" or "unique" anywhere in the message: a German-locale server.
        let err = map(
            unique_violation(
                "doppelter Schlüsselwert verletzt »users_username_key«",
                USERS_USERNAME_UNIQUE,
            ),
            None,
        );
        assert!(
            matches!(err, UserError::DuplicateUsername { ref username } if username == "alice")
        );
        assert_eq!(err.status_code(), StatusCode::CONFLICT);
        assert_eq!(err.user_message(), "Username already exists");
        assert_eq!(err.error_code(), "USER_DUPLICATE_USERNAME");
    }

    #[test]
    fn email_clash_is_classified_by_constraint_not_text() {
        let err = map(
            unique_violation("Schlüssel existiert bereits", USERS_EMAIL_UNIQUE),
            Some(Uuid::new_v4()),
        );
        assert!(
            matches!(err, UserError::DuplicateEmail { ref email } if email == "alice@example.com")
        );
        assert_eq!(err.status_code(), StatusCode::CONFLICT);
        assert_eq!(err.user_message(), "Email already exists");
        assert_eq!(err.error_code(), "USER_DUPLICATE_EMAIL");
    }

    #[test]
    fn matching_text_without_the_structured_fields_is_not_a_clash() {
        // The old substring match would have called each of these a duplicate.
        let text = "duplicate key value violates unique constraint \"users_username_key\" (email)";
        let not_unique = map(
            db_error(text, Some("23503"), Some(USERS_USERNAME_UNIQUE)),
            None,
        );
        assert!(matches!(not_unique, UserError::InternalServerError { .. }));
        let other_constraint = map(unique_violation(text, "some_other_key"), None);
        assert!(matches!(
            other_constraint,
            UserError::InternalServerError { .. }
        ));
        let plain = map_user_write_error(anyhow::anyhow!(text), None, "alice", "a@example.com");
        assert!(matches!(plain, UserError::InternalServerError { .. }));
    }

    #[test]
    fn user_gone_during_update_is_not_found() {
        let id = Uuid::new_v4();
        let err = map_user_write_error(
            anyhow::anyhow!("User not found"),
            Some(id),
            "a",
            "a@example.com",
        );
        assert!(matches!(err, UserError::NotFoundById { id: got } if got == id));
    }
}
