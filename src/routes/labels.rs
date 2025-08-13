use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put, delete},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::{ToSchema, IntoParams};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, Row};

use crate::{auth::AuthUser, AppState};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Label {
    pub id: Uuid,
    pub user_id: Option<Uuid>, // nullable for system labels
    pub name: String,
    pub description: Option<String>,
    pub color: String,
    pub background_color: Option<String>,
    pub icon: Option<String>,
    pub is_system: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub document_count: i64,
    #[serde(default)]
    pub source_count: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateLabel {
    pub name: String,
    pub description: Option<String>,
    #[serde(default = "default_color")]
    pub color: String,
    pub background_color: Option<String>,
    pub icon: Option<String>,
}

fn default_color() -> String {
    "#0969da".to_string()
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateLabel {
    pub name: Option<String>,
    pub description: Option<String>,
    pub color: Option<String>,
    pub background_color: Option<String>,
    pub icon: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LabelAssignment {
    pub label_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct LabelQuery {
    #[serde(default)]
    pub include_counts: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct BulkUpdateRequest {
    pub document_ids: Vec<Uuid>,
    pub label_ids: Vec<Uuid>,
    #[serde(default = "default_bulk_mode")]
    pub mode: String, // "replace", "add", or "remove"
}

fn default_bulk_mode() -> String {
    "replace".to_string()
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_labels))
        .route("/", post(create_label))
        .route("/{id}", get(get_label))
        .route("/{id}", put(update_label))
        .route("/{id}", delete(delete_label))
        .route("/documents/{document_id}", get(get_document_labels))
        .route("/documents/{document_id}", put(update_document_labels))
        .route("/documents/{document_id}/labels/{label_id}", post(add_document_label))
        .route("/documents/{document_id}/labels/{label_id}", delete(remove_document_label))
        .route("/bulk/documents", post(bulk_update_document_labels))
}

#[utoipa::path(
    get,
    path = "/api/labels",
    tag = "labels",
    security(("bearer_auth" = [])),
    params(LabelQuery),
    responses(
        (status = 200, description = "List of labels", body = Vec<Label>)
    )
)]
pub async fn get_labels(
    Query(query): Query<LabelQuery>,
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> Result<Json<Vec<Label>>, StatusCode> {
    let user_id = auth_user.user.id;

    let labels = if query.include_counts {
        sqlx::query_as::<_, Label>(
            r#"
            SELECT 
                l.id, l.user_id, l.name, l.description, l.color, 
                l.background_color, l.icon, l.is_system, l.created_at, l.updated_at,
                COUNT(DISTINCT dl.document_id) as document_count,
                COUNT(DISTINCT sl.source_id) as source_count
            FROM labels l
            LEFT JOIN document_labels dl ON l.id = dl.label_id
            LEFT JOIN source_labels sl ON l.id = sl.label_id
            WHERE (l.user_id = $1 OR l.is_system = TRUE)
            GROUP BY l.id, l.user_id, l.name, l.description, l.color, 
                     l.background_color, l.icon, l.is_system, l.created_at, l.updated_at
            ORDER BY l.name
            "#
        )
        .bind(user_id)
    } else {
        sqlx::query_as::<_, Label>(
            r#"
            SELECT 
                id, user_id, name, description, color, 
                background_color, icon, is_system, created_at, updated_at,
                0::bigint as document_count, 0::bigint as source_count
            FROM labels
            WHERE (user_id = $1 OR is_system = TRUE)
            ORDER BY name
            "#
        )
        .bind(user_id)
    }
    .fetch_all(state.db.get_pool())
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch labels: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(labels))
}

#[utoipa::path(
    post,
    path = "/api/labels",
    tag = "labels",
    security(("bearer_auth" = [])),
    request_body = CreateLabel,
    responses(
        (status = 201, description = "Label created successfully", body = Label),
        (status = 400, description = "Invalid input or label already exists"),
    )
)]
pub async fn create_label(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Json(payload): Json<CreateLabel>,
) -> Result<Json<Label>, StatusCode> {
    let user_id = auth_user.user.id;

    // Validate name is not empty
    if payload.name.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Validate color format
    if !payload.color.starts_with('#') || payload.color.len() != 7 {
        return Err(StatusCode::BAD_REQUEST);
    }

    if let Some(ref bg_color) = payload.background_color {
        if !bg_color.starts_with('#') || bg_color.len() != 7 {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    let label = sqlx::query_as::<_, Label>(
        r#"
        INSERT INTO labels (user_id, name, description, color, background_color, icon)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING 
            id, user_id, name, description, color, background_color, icon, 
            is_system, created_at, updated_at,
            0::bigint as document_count, 0::bigint as source_count
        "#
    )
    .bind(user_id)
    .bind(payload.name)
    .bind(payload.description)
    .bind(payload.color)
    .bind(payload.background_color)
    .bind(payload.icon)
    .fetch_one(state.db.get_pool())
    .await
    .map_err(|e| {
        tracing::error!("Failed to create label: {}", e);
        if e.to_string().contains("duplicate key") {
            StatusCode::CONFLICT
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    Ok(Json(label))
}

#[utoipa::path(
    get,
    path = "/api/labels/{id}",
    tag = "labels",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Label ID")
    ),
    responses(
        (status = 200, description = "Label details", body = Label),
        (status = 404, description = "Label not found"),
    )
)]
pub async fn get_label(
    Path(label_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> Result<Json<Label>, StatusCode> {
    let user_id = auth_user.user.id;

    let label = sqlx::query_as::<_, Label>(
        r#"
        SELECT 
            l.id, l.user_id, l.name, l.description, l.color, 
            l.background_color, l.icon, l.is_system, l.created_at, l.updated_at,
            COUNT(DISTINCT dl.document_id) as document_count,
            COUNT(DISTINCT sl.source_id) as source_count
        FROM labels l
        LEFT JOIN document_labels dl ON l.id = dl.label_id
        LEFT JOIN source_labels sl ON l.id = sl.label_id
        WHERE l.id = $1 AND (l.user_id = $2 OR l.is_system = TRUE)
        GROUP BY l.id, l.user_id, l.name, l.description, l.color, 
                 l.background_color, l.icon, l.is_system, l.created_at, l.updated_at
        "#
    )
    .bind(label_id)
    .bind(user_id)
    .fetch_optional(state.db.get_pool())
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch label: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    match label {
        Some(label) => Ok(Json(label)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

#[utoipa::path(
    put,
    path = "/api/labels/{id}",
    tag = "labels",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Label ID")
    ),
    request_body = UpdateLabel,
    responses(
        (status = 200, description = "Label updated successfully", body = Label),
        (status = 404, description = "Label not found"),
        (status = 400, description = "Invalid input"),
    )
)]
pub async fn update_label(
    Path(label_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Json(payload): Json<UpdateLabel>,
) -> Result<Json<Label>, StatusCode> {
    let user_id = auth_user.user.id;

    // Validate color formats if provided
    if let Some(ref color) = payload.color {
        if !color.starts_with('#') || color.len() != 7 {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    if let Some(ref bg_color) = payload.background_color.as_ref() {
        if !bg_color.starts_with('#') || bg_color.len() != 7 {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    // Check if label exists and user has permission
    let existing = sqlx::query(
        "SELECT id FROM labels WHERE id = $1 AND user_id = $2 AND is_system = FALSE"
    )
    .bind(label_id)
    .bind(user_id)
    .fetch_optional(state.db.get_pool())
    .await
    .map_err(|e| {
        tracing::error!("Failed to check label existence: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if existing.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    // Use COALESCE to update only provided fields
    let label = sqlx::query_as::<_, Label>(
        r#"
        UPDATE labels 
        SET 
            name = COALESCE($2, name),
            description = COALESCE($3, description),
            color = COALESCE($4, color),
            background_color = COALESCE($5, background_color),
            icon = COALESCE($6, icon),
            updated_at = CURRENT_TIMESTAMP
        WHERE id = $1
        RETURNING 
            id, user_id, name, description, color, background_color, icon, 
            is_system, created_at, updated_at,
            0::bigint as document_count, 0::bigint as source_count
        "#
    )
    .bind(label_id)
    .bind(payload.name)
    .bind(payload.description)
    .bind(payload.color)
    .bind(payload.background_color)
    .bind(payload.icon)
    .fetch_one(state.db.get_pool())
    .await
    .map_err(|e| {
        tracing::error!("Failed to update label: {}", e);
        if e.to_string().contains("duplicate key") {
            StatusCode::CONFLICT
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    Ok(Json(label))
}

#[utoipa::path(
    delete,
    path = "/api/labels/{id}",
    tag = "labels",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Label ID")
    ),
    responses(
        (status = 204, description = "Label deleted successfully"),
        (status = 404, description = "Label not found"),
    )
)]
pub async fn delete_label(
    Path(label_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> Result<StatusCode, StatusCode> {
    let user_id = auth_user.user.id;

    let result = sqlx::query(
        "DELETE FROM labels WHERE id = $1 AND user_id = $2 AND is_system = FALSE"
    )
    .bind(label_id)
    .bind(user_id)
    .execute(state.db.get_pool())
    .await
    .map_err(|e| {
        tracing::error!("Failed to delete label: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if result.rows_affected() == 0 {
        Err(StatusCode::NOT_FOUND)
    } else {
        Ok(StatusCode::NO_CONTENT)
    }
}

#[utoipa::path(
    get,
    path = "/api/labels/documents/{document_id}",
    tag = "labels",
    security(("bearer_auth" = [])),
    params(
        ("document_id" = Uuid, Path, description = "Document ID")
    ),
    responses(
        (status = 200, description = "Document labels", body = Vec<Label>),
        (status = 404, description = "Document not found"),
    )
)]
pub async fn get_document_labels(
    Path(document_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> Result<Json<Vec<Label>>, StatusCode> {
    let user_id = auth_user.user.id;

    // Verify document ownership
    let doc = sqlx::query(
        "SELECT id FROM documents WHERE id = $1 AND user_id = $2"
    )
    .bind(document_id)
    .bind(user_id)
    .fetch_optional(state.db.get_pool())
    .await
    .map_err(|e| {
        tracing::error!("Failed to verify document ownership: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if doc.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    let labels = sqlx::query_as::<_, Label>(
        r#"
        SELECT 
            l.id, l.user_id, l.name, l.description, l.color, 
            l.background_color, l.icon, l.is_system, l.created_at, l.updated_at,
            0::bigint as document_count, 0::bigint as source_count
        FROM labels l
        INNER JOIN document_labels dl ON l.id = dl.label_id
        WHERE dl.document_id = $1
        ORDER BY l.name
        "#
    )
    .bind(document_id)
    .fetch_all(state.db.get_pool())
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch document labels: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(labels))
}

#[utoipa::path(
    put,
    path = "/api/labels/documents/{document_id}",
    tag = "labels",
    security(("bearer_auth" = [])),
    params(
        ("document_id" = Uuid, Path, description = "Document ID")
    ),
    request_body = LabelAssignment,
    responses(
        (status = 200, description = "Document labels updated", body = Vec<Label>),
        (status = 404, description = "Document not found"),
        (status = 400, description = "One or more labels not found"),
    )
)]
pub async fn update_document_labels(
    Path(document_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Json(payload): Json<LabelAssignment>,
) -> Result<Json<Vec<Label>>, StatusCode> {
    let user_id = auth_user.user.id;

    // Verify document ownership
    let doc = sqlx::query(
        "SELECT id FROM documents WHERE id = $1 AND user_id = $2"
    )
    .bind(document_id)
    .bind(user_id)
    .fetch_optional(state.db.get_pool())
    .await
    .map_err(|e| {
        tracing::error!("Failed to verify document ownership: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if doc.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    // Verify all labels exist and are accessible
    if !payload.label_ids.is_empty() {
        let label_count = sqlx::query(
            "SELECT COUNT(*) as count FROM labels WHERE id = ANY($1) AND (user_id = $2 OR is_system = TRUE)"
        )
        .bind(&payload.label_ids)
        .bind(user_id)
        .fetch_one(state.db.get_pool())
        .await
        .map_err(|e| {
            tracing::error!("Failed to verify labels: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        let count: i64 = label_count.try_get("count").unwrap_or(0);
        if count as usize != payload.label_ids.len() {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    // Begin transaction
    let mut tx = state.db.get_pool().begin().await.map_err(|e| {
        tracing::error!("Failed to begin transaction: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Remove existing labels
    sqlx::query(
        "DELETE FROM document_labels WHERE document_id = $1"
    )
    .bind(document_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!("Failed to remove existing labels: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Add new labels
    for label_id in &payload.label_ids {
        sqlx::query(
            "INSERT INTO document_labels (document_id, label_id, assigned_by) VALUES ($1, $2, $3)"
        )
        .bind(document_id)
        .bind(label_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("Failed to add label: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    }

    tx.commit().await.map_err(|e| {
        tracing::error!("Failed to commit transaction: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Return updated labels
    get_document_labels(Path(document_id), State(state), auth_user).await
}

#[utoipa::path(
    post,
    path = "/api/labels/documents/{document_id}/labels/{label_id}",
    tag = "labels",
    security(("bearer_auth" = [])),
    params(
        ("document_id" = Uuid, Path, description = "Document ID"),
        ("label_id" = Uuid, Path, description = "Label ID")
    ),
    responses(
        (status = 201, description = "Label added to document"),
        (status = 404, description = "Document or label not found"),
        (status = 409, description = "Label already assigned"),
    )
)]
pub async fn add_document_label(
    Path((document_id, label_id)): Path<(Uuid, Uuid)>,
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> Result<StatusCode, StatusCode> {
    let user_id = auth_user.user.id;

    // Verify document ownership
    let doc = sqlx::query(
        "SELECT id FROM documents WHERE id = $1 AND user_id = $2"
    )
    .bind(document_id)
    .bind(user_id)
    .fetch_optional(state.db.get_pool())
    .await
    .map_err(|e| {
        tracing::error!("Failed to verify document ownership: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if doc.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    // Verify label exists and is accessible
    let label = sqlx::query(
        "SELECT id FROM labels WHERE id = $1 AND (user_id = $2 OR is_system = TRUE)"
    )
    .bind(label_id)
    .bind(user_id)
    .fetch_optional(state.db.get_pool())
    .await
    .map_err(|e| {
        tracing::error!("Failed to verify label: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if label.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    let result = sqlx::query(
        "INSERT INTO document_labels (document_id, label_id, assigned_by) VALUES ($1, $2, $3)"
    )
    .bind(document_id)
    .bind(label_id)
    .bind(user_id)
    .execute(state.db.get_pool())
    .await;

    match result {
        Ok(_) => Ok(StatusCode::CREATED),
        Err(e) if e.to_string().contains("duplicate key") => Ok(StatusCode::OK), // Already assigned
        Err(e) => {
            tracing::error!("Failed to add document label: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[utoipa::path(
    delete,
    path = "/api/labels/documents/{document_id}/labels/{label_id}",
    tag = "labels",
    security(("bearer_auth" = [])),
    params(
        ("document_id" = Uuid, Path, description = "Document ID"),
        ("label_id" = Uuid, Path, description = "Label ID")
    ),
    responses(
        (status = 204, description = "Label removed from document"),
        (status = 404, description = "Document not found or label not assigned"),
    )
)]
pub async fn remove_document_label(
    Path((document_id, label_id)): Path<(Uuid, Uuid)>,
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> Result<StatusCode, StatusCode> {
    let user_id = auth_user.user.id;

    // Verify document ownership
    let doc = sqlx::query(
        "SELECT id FROM documents WHERE id = $1 AND user_id = $2"
    )
    .bind(document_id)
    .bind(user_id)
    .fetch_optional(state.db.get_pool())
    .await
    .map_err(|e| {
        tracing::error!("Failed to verify document ownership: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if doc.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    let result = sqlx::query(
        "DELETE FROM document_labels WHERE document_id = $1 AND label_id = $2"
    )
    .bind(document_id)
    .bind(label_id)
    .execute(state.db.get_pool())
    .await
    .map_err(|e| {
        tracing::error!("Failed to remove document label: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if result.rows_affected() == 0 {
        Err(StatusCode::NOT_FOUND)
    } else {
        Ok(StatusCode::NO_CONTENT)
    }
}

#[utoipa::path(
    post,
    path = "/api/labels/bulk/documents",
    tag = "labels",
    security(("bearer_auth" = [])),
    request_body = BulkUpdateRequest,
    responses(
        (status = 200, description = "Bulk operation completed"),
        (status = 400, description = "Invalid input"),
    )
)]
pub async fn bulk_update_document_labels(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Json(payload): Json<BulkUpdateRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let user_id = auth_user.user.id;

    // Validate mode
    if !["replace", "add", "remove"].contains(&payload.mode.as_str()) {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Verify document ownership
    let doc_count = sqlx::query(
        "SELECT COUNT(*) as count FROM documents WHERE id = ANY($1) AND user_id = $2"
    )
    .bind(&payload.document_ids)
    .bind(user_id)
    .fetch_one(state.db.get_pool())
    .await
    .map_err(|e| {
        tracing::error!("Failed to verify document ownership: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let count: i64 = doc_count.try_get("count").unwrap_or(0);
    if count as usize != payload.document_ids.len() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Verify labels exist and are accessible (if any labels provided)
    if !payload.label_ids.is_empty() {
        let label_count = sqlx::query(
            "SELECT COUNT(*) as count FROM labels WHERE id = ANY($1) AND (user_id = $2 OR is_system = TRUE)"
        )
        .bind(&payload.label_ids)
        .bind(user_id)
        .fetch_one(state.db.get_pool())
        .await
        .map_err(|e| {
            tracing::error!("Failed to verify labels: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        let count: i64 = label_count.try_get("count").unwrap_or(0);
        if count as usize != payload.label_ids.len() {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    // Begin transaction
    let mut tx = state.db.get_pool().begin().await.map_err(|e| {
        tracing::error!("Failed to begin transaction: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    match payload.mode.as_str() {
        "replace" => {
            // Remove all existing labels for these documents
            sqlx::query(
                "DELETE FROM document_labels WHERE document_id = ANY($1)"
            )
            .bind(&payload.document_ids)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                tracing::error!("Failed to remove existing labels: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

            // Add new labels
            if !payload.label_ids.is_empty() {
                for document_id in &payload.document_ids {
                    for label_id in &payload.label_ids {
                        sqlx::query(
                            "INSERT INTO document_labels (document_id, label_id, assigned_by) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING"
                        )
                        .bind(document_id)
                        .bind(label_id)
                        .bind(user_id)
                        .execute(&mut *tx)
                        .await
                        .map_err(|e| {
                            tracing::error!("Failed to add label: {}", e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?;
                    }
                }
            }
        },
        "add" => {
            if !payload.label_ids.is_empty() {
                for document_id in &payload.document_ids {
                    for label_id in &payload.label_ids {
                        sqlx::query(
                            "INSERT INTO document_labels (document_id, label_id, assigned_by) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING"
                        )
                        .bind(document_id)
                        .bind(label_id)
                        .bind(user_id)
                        .execute(&mut *tx)
                        .await
                        .map_err(|e| {
                            tracing::error!("Failed to add label: {}", e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?;
                    }
                }
            }
        },
        "remove" => {
            if !payload.label_ids.is_empty() {
                sqlx::query(
                    "DELETE FROM document_labels WHERE document_id = ANY($1) AND label_id = ANY($2)"
                )
                .bind(&payload.document_ids)
                .bind(&payload.label_ids)
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to remove labels: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
            }
        },
        _ => return Err(StatusCode::BAD_REQUEST)
    }

    tx.commit().await.map_err(|e| {
        tracing::error!("Failed to commit transaction: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(serde_json::json!({
        "message": format!("Labels {}d successfully", payload.mode),
        "documents_updated": payload.document_ids.len()
    })))
}