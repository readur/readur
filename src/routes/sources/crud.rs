use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;
use uuid::Uuid;
use tracing::{error, info};

use crate::{
    auth::AuthUser,
    errors::source::SourceError,
    models::{CreateSource, SourceResponse, SourceWithStats, UpdateSource, SourceType},
    AppState,
};

/// List all sources for the authenticated user
#[utoipa::path(
    get,
    path = "/api/sources",
    tag = "sources",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "List of user sources", body = Vec<SourceResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn list_sources(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<SourceResponse>>, SourceError> {
    let sources = state
        .db
        .get_sources(auth_user.user.id)
        .await
        .map_err(|e| SourceError::connection_failed(format!("Failed to retrieve sources: {}", e)))?;

    // Get source IDs for batch counting
    let source_ids: Vec<Uuid> = sources.iter().map(|s| s.id).collect();
    
    // Get document counts for all sources in one query
    let counts = state
        .db
        .count_documents_for_sources(auth_user.user.id, &source_ids)
        .await
        .map_err(|e| SourceError::connection_failed(format!("Failed to count documents: {}", e)))?;
    
    // Create a map for quick lookup
    let count_map: std::collections::HashMap<Uuid, (i64, i64)> = counts
        .into_iter()
        .map(|(id, total, ocr)| (id, (total, ocr)))
        .collect();

    let responses: Vec<SourceResponse> = sources
        .into_iter()
        .map(|s| {
            let (total_docs, total_ocr) = count_map.get(&s.id).copied().unwrap_or((0, 0));
            let mut response: SourceResponse = s.into();
            response.total_documents = total_docs;
            response.total_documents_ocr = total_ocr;
            response
        })
        .collect();
    
    Ok(Json(responses))
}

/// Create a new source
#[utoipa::path(
    post,
    path = "/api/sources",
    tag = "sources",
    security(
        ("bearer_auth" = [])
    ),
    request_body = CreateSource,
    responses(
        (status = 201, description = "Source created successfully", body = SourceResponse),
        (status = 400, description = "Bad request - invalid source data"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn create_source(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
    Json(source_data): Json<CreateSource>,
) -> Result<Json<SourceResponse>, SourceError> {
    // Validate source configuration based on type
    if let Err(validation_error) = validate_source_config(&source_data) {
        error!("Source validation failed: {}", validation_error);
        error!("Invalid source data received: {:?}", source_data);
        return Err(SourceError::configuration_invalid(validation_error));
    }

    let source = state
        .db
        .create_source(auth_user.user.id, &source_data)
        .await
        .map_err(|e| {
            error!("Failed to create source in database: {}", e);
            let error_msg = e.to_string();
            if error_msg.contains("name") && error_msg.contains("unique") {
                SourceError::duplicate_name(&source_data.name)
            } else {
                SourceError::connection_failed(format!("Database error: {}", e))
            }
        })?;

    let mut response: SourceResponse = source.into();
    // New sources have no documents yet
    response.total_documents = 0;
    response.total_documents_ocr = 0;

    Ok(Json(response))
}

/// Get a specific source by ID with detailed stats
#[utoipa::path(
    get,
    path = "/api/sources/{id}",
    tag = "sources",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = Uuid, Path, description = "Source ID")
    ),
    responses(
        (status = 200, description = "Source details with stats", body = SourceWithStats),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Source not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_source(
    auth_user: AuthUser,
    Path(source_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<SourceWithStats>, StatusCode> {
    let source = state
        .db
        .get_source(auth_user.user.id, source_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Get recent documents for this source
    let recent_documents = state
        .db
        .get_recent_documents_for_source(auth_user.user.id, source_id, 10)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get document counts
    let (total_documents, total_documents_ocr) = state
        .db
        .count_documents_for_source(auth_user.user.id, source_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Calculate sync progress
    let sync_progress = if source.total_files_pending > 0 {
        Some(
            (source.total_files_synced as f32
                / (source.total_files_synced + source.total_files_pending) as f32)
                * 100.0,
        )
    } else {
        None
    };

    let mut source_response: SourceResponse = source.into();
    source_response.total_documents = total_documents;
    source_response.total_documents_ocr = total_documents_ocr;

    let response = SourceWithStats {
        source: source_response,
        recent_documents: recent_documents.into_iter().map(|d| d.into()).collect(),
        sync_progress,
    };

    Ok(Json(response))
}

/// Update a source
#[utoipa::path(
    put,
    path = "/api/sources/{id}",
    tag = "sources",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = Uuid, Path, description = "Source ID")
    ),
    request_body = UpdateSource,
    responses(
        (status = 200, description = "Source updated successfully", body = SourceResponse),
        (status = 400, description = "Bad request - invalid update data"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Source not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn update_source(
    auth_user: AuthUser,
    Path(source_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
    Json(update_data): Json<UpdateSource>,
) -> Result<Json<SourceResponse>, StatusCode> {
    info!("Updating source {} with data: {:?}", source_id, update_data);
    
    // Check if source exists
    let existing = state
        .db
        .get_source(auth_user.user.id, source_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Validate config if provided
    if let Some(config) = &update_data.config {
        if let Err(validation_error) = validate_config_for_type(&existing.source_type, config) {
            error!("Config validation failed for source {}: {}", source_id, validation_error);
            error!("Invalid config received: {:?}", config);
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    let source = state
        .db
        .update_source(auth_user.user.id, source_id, &update_data)
        .await
        .map_err(|e| {
            error!("Failed to update source {} in database: {}", source_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Get document counts
    let (total_documents, total_documents_ocr) = state
        .db
        .count_documents_for_source(auth_user.user.id, source_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut response: SourceResponse = source.into();
    response.total_documents = total_documents;
    response.total_documents_ocr = total_documents_ocr;

    info!("Successfully updated source {}: {}", source_id, response.name);
    Ok(Json(response))
}

/// Delete a source
#[utoipa::path(
    delete,
    path = "/api/sources/{id}",
    tag = "sources",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = Uuid, Path, description = "Source ID")
    ),
    responses(
        (status = 204, description = "Source deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Source not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn delete_source(
    auth_user: AuthUser,
    Path(source_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, StatusCode> {
    let deleted = state
        .db
        .delete_source(auth_user.user.id, source_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Validate source configuration based on type
fn validate_source_config(source: &CreateSource) -> Result<(), &'static str> {
    validate_config_for_type(&source.source_type, &source.config)
}

/// Validate configuration for a specific source type
pub fn validate_config_for_type(
    source_type: &SourceType,
    config: &serde_json::Value,
) -> Result<(), &'static str> {
    match source_type {
        SourceType::WebDAV => {
            let _: crate::models::WebDAVSourceConfig =
                serde_json::from_value(config.clone()).map_err(|_| "Invalid WebDAV configuration")?;
            Ok(())
        }
        SourceType::LocalFolder => {
            let _: crate::models::LocalFolderSourceConfig =
                serde_json::from_value(config.clone()).map_err(|_| "Invalid Local Folder configuration")?;
            Ok(())
        }
        SourceType::S3 => {
            let _: crate::models::S3SourceConfig =
                serde_json::from_value(config.clone()).map_err(|_| "Invalid S3 configuration")?;
            Ok(())
        }
    }
}