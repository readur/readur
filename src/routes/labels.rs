use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put, delete},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use sqlx::FromRow;

use crate::{auth::AuthUser, AppState};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Label {
    pub id: Uuid,
    pub user_id: Uuid,
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

#[derive(Debug, Deserialize, ToSchema)]
pub struct LabelQuery {
    #[serde(default)]
    pub include_counts: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct BulkUpdateMode {
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
        .route("/:id", get(get_label))
        .route("/:id", put(update_label))
        .route("/:id", delete(delete_label))
        .route("/documents/:document_id", get(get_document_labels))
        .route("/documents/:document_id", put(update_document_labels))
        .route("/documents/:document_id/labels/:label_id", post(add_document_label))
        .route("/documents/:document_id/labels/:label_id", delete(remove_document_label))
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
    let user_id = auth_user.id;

    let labels = if query.include_counts {
        sqlx::query_as!(
            Label,
            r#"
            SELECT 
                l.id, l.user_id, l.name, l.description, l.color, 
                l.background_color, l.icon, l.is_system, l.created_at, l.updated_at,
                COUNT(DISTINCT dl.document_id) as document_count,
                COUNT(DISTINCT sl.source_id) as source_count
            FROM labels l
            LEFT JOIN document_labels dl ON l.id = dl.label_id
            LEFT JOIN source_labels sl ON l.id = sl.label_id
            WHERE l.user_id = $1 OR l.is_system = TRUE
            GROUP BY l.id, l.user_id, l.name, l.description, l.color, 
                     l.background_color, l.icon, l.is_system, l.created_at, l.updated_at
            ORDER BY l.name
            "#,
            user_id
        )
    } else {
        sqlx::query_as!(
            Label,
            r#"
            SELECT 
                id, user_id, name, description, color, 
                background_color, icon, is_system, created_at, updated_at,
                0::bigint as document_count, 0::bigint as source_count
            FROM labels
            WHERE user_id = $1 OR is_system = TRUE
            ORDER BY name
            "#,
            user_id
        )
    }
    .fetch_all(&state.db)
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
    let user_id = auth_user.id;

    // Validate color format
    if !payload.color.starts_with('#') || payload.color.len() != 7 {
        return Err(StatusCode::BAD_REQUEST);
    }

    if let Some(ref bg_color) = payload.background_color {
        if !bg_color.starts_with('#') || bg_color.len() != 7 {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    let label = sqlx::query_as!(
        Label,
        r#"
        INSERT INTO labels (user_id, name, description, color, background_color, icon)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING 
            id, user_id, name, description, color, background_color, icon, 
            is_system, created_at, updated_at,
            0::bigint as document_count, 0::bigint as source_count
        "#,
        user_id,
        payload.name,
        payload.description,
        payload.color,
        payload.background_color,
        payload.icon
    )
    .fetch_one(&state.db)
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
    let user_id = auth_user.id;

    let label = sqlx::query_as!(
        Label,
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
        "#,
        label_id,
        user_id
    )
    .fetch_optional(&state.db)
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
    let user_id = auth_user.id;

    // Validate color formats if provided
    if let Some(ref color) = payload.color {
        if !color.starts_with('#') || color.len() != 7 {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    if let Some(Some(ref bg_color)) = payload.background_color.as_ref() {
        if !bg_color.starts_with('#') || bg_color.len() != 7 {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    // Check if label exists and user has permission
    let existing = sqlx::query!(
        "SELECT id FROM labels WHERE id = $1 AND user_id = $2 AND is_system = FALSE",
        label_id,
        user_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to check label existence: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if existing.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    // Build update query dynamically
    let mut query = "UPDATE labels SET updated_at = CURRENT_TIMESTAMP".to_string();
    let mut values: Vec<Box<dyn sqlx::Encode<'_, sqlx::Postgres> + Send + Sync>> = Vec::new();
    let mut param_index = 1;

    if let Some(name) = payload.name {
        query.push_str(&format!(", name = ${}", param_index));
        values.push(Box::new(name));
        param_index += 1;
    }

    if let Some(description) = payload.description {
        query.push_str(&format!(", description = ${}", param_index));
        values.push(Box::new(description));
        param_index += 1;
    }

    if let Some(color) = payload.color {
        query.push_str(&format!(", color = ${}", param_index));
        values.push(Box::new(color));
        param_index += 1;
    }

    if let Some(background_color) = payload.background_color {
        query.push_str(&format!(", background_color = ${}", param_index));
        values.push(Box::new(background_color));
        param_index += 1;
    }

    if let Some(icon) = payload.icon {
        query.push_str(&format!(", icon = ${}", param_index));
        values.push(Box::new(icon));
        param_index += 1;
    }

    query.push_str(&format!(" WHERE id = ${} RETURNING *", param_index));
    values.push(Box::new(label_id));

    // For simplicity, let's rebuild the query using individual fields
    let label = sqlx::query_as!(
        Label,
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
        "#,
        label_id,
        payload.name,
        payload.description,
        payload.color,
        payload.background_color,
        payload.icon
    )
    .fetch_one(&state.db)
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
    let user_id = auth_user.id;

    let result = sqlx::query!(
        "DELETE FROM labels WHERE id = $1 AND user_id = $2 AND is_system = FALSE",
        label_id,
        user_id
    )
    .execute(&state.db)
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
    let user_id = auth_user.id;

    // Verify document ownership
    let doc = sqlx::query!(
        "SELECT id FROM documents WHERE id = $1 AND user_id = $2",
        document_id,
        user_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to verify document ownership: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if doc.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    let labels = sqlx::query_as!(
        Label,
        r#"
        SELECT 
            l.id, l.user_id, l.name, l.description, l.color, 
            l.background_color, l.icon, l.is_system, l.created_at, l.updated_at,
            0::bigint as document_count, 0::bigint as source_count
        FROM labels l
        INNER JOIN document_labels dl ON l.id = dl.label_id
        WHERE dl.document_id = $1
        ORDER BY l.name
        "#,
        document_id
    )
    .fetch_all(&state.db)
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
    let user_id = auth_user.id;

    // Verify document ownership
    let doc = sqlx::query!(
        "SELECT id FROM documents WHERE id = $1 AND user_id = $2",
        document_id,
        user_id
    )
    .fetch_optional(&state.db)
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
        let label_count = sqlx::query!(
            "SELECT COUNT(*) as count FROM labels WHERE id = ANY($1) AND (user_id = $2 OR is_system = TRUE)",
            &payload.label_ids,
            user_id
        )
        .fetch_one(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to verify labels: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        if label_count.count.unwrap_or(0) as usize != payload.label_ids.len() {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    // Begin transaction
    let mut tx = state.db.begin().await.map_err(|e| {
        tracing::error!("Failed to begin transaction: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Remove existing labels
    sqlx::query!(
        "DELETE FROM document_labels WHERE document_id = $1",
        document_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!("Failed to remove existing labels: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Add new labels
    for label_id in &payload.label_ids {
        sqlx::query!(
            "INSERT INTO document_labels (document_id, label_id, assigned_by) VALUES ($1, $2, $3)",
            document_id,
            label_id,
            user_id
        )
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
    let user_id = auth_user.id;

    // Verify document ownership
    let doc = sqlx::query!(
        "SELECT id FROM documents WHERE id = $1 AND user_id = $2",
        document_id,
        user_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to verify document ownership: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if doc.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    // Verify label exists and is accessible
    let label = sqlx::query!(
        "SELECT id FROM labels WHERE id = $1 AND (user_id = $2 OR is_system = TRUE)",
        label_id,
        user_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to verify label: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if label.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    let result = sqlx::query!(
        "INSERT INTO document_labels (document_id, label_id, assigned_by) VALUES ($1, $2, $3)",
        document_id,
        label_id,
        user_id
    )
    .execute(&state.db)
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
    let user_id = auth_user.id;

    // Verify document ownership
    let doc = sqlx::query!(
        "SELECT id FROM documents WHERE id = $1 AND user_id = $2",
        document_id,
        user_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to verify document ownership: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if doc.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    let result = sqlx::query!(
        "DELETE FROM document_labels WHERE document_id = $1 AND label_id = $2",
        document_id,
        label_id
    )
    .execute(&state.db)
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
    request_body = LabelAssignment,
    params(
        ("mode" = String, Query, description = "Operation mode: replace, add, or remove")
    ),
    responses(
        (status = 200, description = "Bulk operation completed"),
        (status = 400, description = "Invalid input"),
    )
)]
pub async fn bulk_update_document_labels(
    Query(mode_query): Query<BulkUpdateMode>,
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Json(payload): Json<BulkUpdateMode>, // This should actually be a combined payload
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Note: This is a simplified implementation. In a real scenario, you'd want
    // a more complex payload structure that includes both document_ids and label_ids
    // For now, returning a placeholder response
    Ok(Json(serde_json::json!({
        "message": "Bulk update functionality placeholder",
        "mode": mode_query.mode
    })))
}