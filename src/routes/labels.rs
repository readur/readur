use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    db::labels::{
        is_label_name_taken, LabelAssignOutcome, LabelRemoveOutcome, LabelReplaceOutcome,
    },
    errors::label::LabelError,
    AppState,
};

// The label API's request and response types live in `models::label`; re-exported here because
// they are this endpoint group's wire format.
pub use crate::models::label::{
    BulkUpdateRequest, CreateLabel, Label, LabelAssignment, LabelQuery, UpdateLabel,
};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_labels))
        .route("/", post(create_label))
        .route("/{id}", get(get_label))
        .route("/{id}", put(update_label))
        .route("/{id}", delete(delete_label))
        .route("/documents/{document_id}", get(get_document_labels))
        .route("/documents/{document_id}", put(update_document_labels))
        .route(
            "/documents/{document_id}/labels/{label_id}",
            post(add_document_label),
        )
        .route(
            "/documents/{document_id}/labels/{label_id}",
            delete(remove_document_label),
        )
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
    state
        .db
        .list_labels(auth_user.user.id, query.include_counts)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to fetch labels: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
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
) -> Result<Json<Label>, LabelError> {
    payload.validate()?;

    let label = state
        .db
        .create_label(auth_user.user.id, &payload)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create label: {}", e);
            if is_label_name_taken(&e) {
                LabelError::duplicate_name(payload.name.clone())
            } else {
                LabelError::invalid_name(payload.name.clone(), e.to_string())
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
    let label = state
        .db
        .get_label_for_user(label_id, auth_user.user.id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch label: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    label.map(Json).ok_or(StatusCode::NOT_FOUND)
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
) -> Result<Json<Label>, LabelError> {
    payload.validate()?;

    let label = state
        .db
        .update_label(label_id, auth_user.user.id, &payload)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update label: {}", e);
            let name = payload.name.clone().unwrap_or_default();
            if is_label_name_taken(&e) {
                LabelError::duplicate_name(name)
            } else {
                LabelError::invalid_name(name, e.to_string())
            }
        })?;

    label.map(Json).ok_or(LabelError::NotFound)
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

    let result = sqlx::query("DELETE FROM labels WHERE id = $1 AND user_id = $2")
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
    let labels = state
        .db
        .get_document_labels_for_user(document_id, auth_user.user.id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch document labels: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    labels.map(Json).ok_or(StatusCode::NOT_FOUND)
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
    let outcome = state
        .db
        .replace_document_labels_for_user(document_id, &payload.label_ids, auth_user.user.id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update document labels: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    match outcome {
        LabelReplaceOutcome::Replaced(labels) => Ok(Json(labels)),
        LabelReplaceOutcome::DocumentNotFound => Err(StatusCode::NOT_FOUND),
        LabelReplaceOutcome::LabelNotFound => Err(StatusCode::BAD_REQUEST),
    }
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
    let outcome = state
        .db
        .add_document_label_for_user(document_id, label_id, auth_user.user.id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to add document label: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    match outcome {
        LabelAssignOutcome::Added => Ok(StatusCode::CREATED),
        LabelAssignOutcome::AlreadyAssigned => Ok(StatusCode::OK),
        LabelAssignOutcome::DocumentNotFound | LabelAssignOutcome::LabelNotFound => {
            Err(StatusCode::NOT_FOUND)
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
    let outcome = state
        .db
        .remove_document_label_for_user(document_id, label_id, auth_user.user.id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to remove document label: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    match outcome {
        LabelRemoveOutcome::Removed => Ok(StatusCode::NO_CONTENT),
        LabelRemoveOutcome::NotAssigned | LabelRemoveOutcome::DocumentNotFound => {
            Err(StatusCode::NOT_FOUND)
        }
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
    let doc_count =
        sqlx::query("SELECT COUNT(*) as count FROM documents WHERE id = ANY($1) AND user_id = $2")
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
        let label_count =
            sqlx::query("SELECT COUNT(*) as count FROM labels WHERE id = ANY($1) AND user_id = $2")
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
            sqlx::query("DELETE FROM document_labels WHERE document_id = ANY($1)")
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
        }
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
        }
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
        }
        _ => return Err(StatusCode::BAD_REQUEST),
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
