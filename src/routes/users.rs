use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    models::{CreateUser, UpdateUser, UserResponse, UserRole},
    AppState,
};

fn require_admin(auth_user: &AuthUser) -> Result<(), StatusCode> {
    if auth_user.user.role != UserRole::Admin {
        Err(StatusCode::FORBIDDEN)
    } else {
        Ok(())
    }
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_users).post(create_user))
        .route("/{id}", get(get_user).put(update_user).delete(delete_user))
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
) -> Result<Json<Vec<UserResponse>>, StatusCode> {
    require_admin(&auth_user)?;
    let users = state
        .db
        .get_all_users()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
) -> Result<Json<UserResponse>, StatusCode> {
    require_admin(&auth_user)?;
    let user = state
        .db
        .get_user_by_id(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

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
) -> Result<Json<UserResponse>, StatusCode> {
    require_admin(&auth_user)?;
    let user = state
        .db
        .create_user(user_data)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

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
) -> Result<Json<UserResponse>, StatusCode> {
    require_admin(&auth_user)?;
    let user = state
        .db
        .update_user(id, update_data.username, update_data.email, update_data.password)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

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
) -> Result<StatusCode, StatusCode> {
    require_admin(&auth_user)?;
    
    // Prevent users from deleting themselves
    if auth_user.user.id == id {
        return Err(StatusCode::FORBIDDEN);
    }

    state
        .db
        .delete_user(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}