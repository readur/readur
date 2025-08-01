use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Json, Response},
    body::Body,
};
use std::sync::Arc;
use tracing::{debug, error};
use std::collections::HashMap;
use sqlx::Row;

use crate::{
    auth::AuthUser,
    models::UserRole,
    services::file_service::FileService,
    AppState,
};
use super::types::FailedDocumentsQuery;

/// Get failed documents with filtering and pagination
#[utoipa::path(
    get,
    path = "/api/documents/failed",
    tag = "documents",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("limit" = Option<i64>, Query, description = "Number of documents to return"),
        ("offset" = Option<i64>, Query, description = "Number of documents to skip"),
        ("stage" = Option<String>, Query, description = "Filter by failure stage (ocr, ingestion, validation, etc.)"),
        ("reason" = Option<String>, Query, description = "Filter by failure reason")
    ),
    responses(
        (status = 200, description = "Failed documents list", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_failed_documents(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Query(params): Query<FailedDocumentsQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let limit = params.limit.unwrap_or(25);
    let offset = params.offset.unwrap_or(0);
    
    // Query the unified failed_documents table
    let mut query_builder = sqlx::QueryBuilder::new(
        r#"
        SELECT id, filename, original_filename, file_path, file_size, mime_type,
               content, tags, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms,
               failure_reason, failure_stage, error_message, existing_document_id,
               ingestion_source, retry_count, last_retry_at, created_at, updated_at
        FROM failed_documents
        WHERE ($1::uuid IS NULL OR user_id = $1)
        "#
    );
    
    let mut bind_count = 1;
    
    // Add stage filter if specified
    if let Some(stage) = &params.stage {
        bind_count += 1;
        query_builder.push(&format!(" AND failure_stage = ${}", bind_count));
    }
    
    // Add reason filter if specified  
    if let Some(reason) = &params.reason {
        bind_count += 1;
        query_builder.push(&format!(" AND failure_reason = ${}", bind_count));
    }
    
    query_builder.push(" ORDER BY created_at DESC");
    query_builder.push(&format!(" LIMIT ${} OFFSET ${}", bind_count + 1, bind_count + 2));
    
    let mut query = query_builder.build();
    
    // Bind parameters in order
    query = query.bind(if auth_user.user.role == UserRole::Admin { 
        None 
    } else { 
        Some(auth_user.user.id) 
    });
    
    if let Some(stage) = &params.stage {
        query = query.bind(stage);
    }
    
    if let Some(reason) = &params.reason {
        query = query.bind(reason);
    }
    
    query = query.bind(limit).bind(offset);
    
    let failed_docs = query
        .fetch_all(state.db.get_pool())
        .await
        .map_err(|e| {
            error!("Failed to fetch failed documents: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    // Count total for pagination
    let mut count_query_builder = sqlx::QueryBuilder::new(
        "SELECT COUNT(*) FROM failed_documents WHERE ($1::uuid IS NULL OR user_id = $1)"
    );
    
    let mut count_bind_count = 1;
    
    if let Some(stage) = &params.stage {
        count_bind_count += 1;
        count_query_builder.push(&format!(" AND failure_stage = ${}", count_bind_count));
    }
    
    if let Some(reason) = &params.reason {
        count_bind_count += 1;
        count_query_builder.push(&format!(" AND failure_reason = ${}", count_bind_count));
    }
    
    let mut count_query = count_query_builder.build_query_scalar::<i64>();
    
    count_query = count_query.bind(if auth_user.user.role == UserRole::Admin { 
        None 
    } else { 
        Some(auth_user.user.id) 
    });
    
    if let Some(stage) = &params.stage {
        count_query = count_query.bind(stage);
    }
    
    if let Some(reason) = &params.reason {
        count_query = count_query.bind(reason);
    }
    
    let total_count = count_query
        .fetch_one(state.db.get_pool())
        .await
        .unwrap_or(0);
    
    // Convert to JSON response format
    let documents: Vec<serde_json::Value> = failed_docs.iter().map(|row| {
        serde_json::json!({
            "id": row.get::<uuid::Uuid, _>("id"),
            "filename": row.get::<String, _>("filename"),
            "original_filename": row.get::<Option<String>, _>("original_filename"),
            "file_path": row.get::<Option<String>, _>("file_path"),
            "file_size": row.get::<Option<i64>, _>("file_size"),
            "mime_type": row.get::<Option<String>, _>("mime_type"),
            "content": row.get::<Option<String>, _>("content"),
            "tags": row.get::<Option<Vec<String>>, _>("tags").unwrap_or_default(),
            "ocr_text": row.get::<Option<String>, _>("ocr_text"),
            "ocr_confidence": row.get::<Option<f32>, _>("ocr_confidence"),
            "ocr_word_count": row.get::<Option<i32>, _>("ocr_word_count"),
            "ocr_processing_time_ms": row.get::<Option<i32>, _>("ocr_processing_time_ms"),
            "failure_reason": row.get::<String, _>("failure_reason"),
            "failure_stage": row.get::<String, _>("failure_stage"),
            "error_message": row.get::<Option<String>, _>("error_message"),
            "existing_document_id": row.get::<Option<uuid::Uuid>, _>("existing_document_id"),
            "ingestion_source": row.get::<String, _>("ingestion_source"),
            "retry_count": row.get::<Option<i32>, _>("retry_count"),
            "last_retry_at": row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("last_retry_at"),
            "created_at": row.get::<chrono::DateTime<chrono::Utc>, _>("created_at"),
            "updated_at": row.get::<chrono::DateTime<chrono::Utc>, _>("updated_at"),
            
            // Computed fields for backward compatibility
            "failure_category": categorize_failure_reason(
                Some(&row.get::<String, _>("failure_reason")),
                row.get::<Option<String>, _>("error_message").as_deref()
            ),
            "source": match row.get::<String, _>("failure_stage").as_str() {
                "ocr" => "OCR Processing",
                "ingestion" => "Document Ingestion", 
                "validation" => "Document Validation",
                "storage" => "File Storage",
                "processing" => "Document Processing",
                "sync" => "Source Synchronization",
                _ => "Unknown"
            }
        })
    }).collect();
    
    // Calculate statistics for the response
    let mut stage_stats = HashMap::new();
    let mut reason_stats = HashMap::new();
    
    for doc in &documents {
        let stage = doc["failure_stage"].as_str().unwrap_or("unknown");
        let reason = doc["failure_reason"].as_str().unwrap_or("unknown");
        
        *stage_stats.entry(stage).or_insert(0) += 1;
        *reason_stats.entry(reason).or_insert(0) += 1;
    }
    
    let response = serde_json::json!({
        "documents": documents,
        "pagination": {
            "limit": limit,
            "offset": offset,
            "total": total_count,
            "total_pages": (total_count as f64 / limit as f64).ceil() as i64
        },
        "statistics": {
            "total_failed": total_count,
            "by_stage": stage_stats,
            "by_reason": reason_stats
        },
        "filters": {
            "stage": params.stage,
            "reason": params.reason
        }
    });
    
    Ok(Json(response))
}

/// Get failed OCR documents with detailed information
#[utoipa::path(
    get,
    path = "/api/documents/failed-ocr",
    tag = "documents",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("limit" = Option<i64>, Query, description = "Number of documents to return"),
        ("offset" = Option<i64>, Query, description = "Number of documents to skip")
    ),
    responses(
        (status = 200, description = "Failed OCR documents list", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_failed_ocr_documents(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Query(pagination): Query<super::types::PaginationQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let limit = pagination.limit.unwrap_or(50);
    let offset = pagination.offset.unwrap_or(0);
    
    // Get failed OCR documents with additional failure details
    let failed_docs = sqlx::query(
        r#"
        SELECT d.id, d.filename, d.original_filename, d.file_path, d.file_size, 
               d.mime_type, d.created_at, d.updated_at, d.user_id,
               d.ocr_status, d.ocr_error, d.ocr_failure_reason,
               d.ocr_completed_at, d.tags,
               -- Count retry attempts from OCR queue
               COALESCE(q.retry_count, 0) as retry_count,
               q.last_attempt_at
        FROM documents d
        LEFT JOIN (
            SELECT document_id, 
                   COUNT(*) as retry_count,
                   MAX(created_at) as last_attempt_at
            FROM ocr_queue 
            WHERE status IN ('failed', 'completed')
            GROUP BY document_id
        ) q ON d.id = q.document_id
        WHERE d.ocr_status = 'failed'
          AND ($1::uuid IS NULL OR d.user_id = $1)  -- Admin can see all, users see only their own
        ORDER BY d.updated_at DESC
        LIMIT $2 OFFSET $3
        "#
    )
    .bind(if auth_user.user.role == UserRole::Admin { 
        None 
    } else { 
        Some(auth_user.user.id) 
    })
    .bind(limit)
    .bind(offset)
    .fetch_all(state.db.get_pool())
    .await
    .map_err(|e| {
        error!("Failed to fetch failed OCR documents: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    // Count total failed documents
    let total_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM documents 
        WHERE ocr_status = 'failed'
          AND ($1::uuid IS NULL OR user_id = $1)
        "#
    )
    .bind(if auth_user.user.role == UserRole::Admin { 
        None 
    } else { 
        Some(auth_user.user.id) 
    })
    .fetch_one(state.db.get_pool())
    .await
    .map_err(|e| {
        error!("Failed to count failed OCR documents: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    let failed_documents: Vec<serde_json::Value> = failed_docs
        .into_iter()
        .map(|row| {
            let tags: Vec<String> = row.get::<Option<Vec<String>>, _>("tags").unwrap_or_default();
            
            serde_json::json!({
                "id": row.get::<uuid::Uuid, _>("id"),
                "filename": row.get::<String, _>("filename"),
                "original_filename": row.get::<String, _>("original_filename"),
                "file_size": row.get::<i64, _>("file_size"),
                "mime_type": row.get::<String, _>("mime_type"),
                "created_at": row.get::<chrono::DateTime<chrono::Utc>, _>("created_at"),
                "updated_at": row.get::<chrono::DateTime<chrono::Utc>, _>("updated_at"),
                "tags": tags,
                "ocr_status": row.get::<Option<String>, _>("ocr_status"),
                "ocr_error": row.get::<Option<String>, _>("ocr_error"),
                "ocr_failure_reason": row.get::<Option<String>, _>("ocr_failure_reason"),
                "ocr_completed_at": row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("ocr_completed_at"),
                "retry_count": row.get::<Option<i64>, _>("retry_count").unwrap_or(0),
                "last_attempt_at": row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("last_attempt_at"),
                "can_retry": true,
                "failure_category": categorize_failure_reason(
                    row.get::<Option<String>, _>("ocr_failure_reason").as_deref(),
                    row.get::<Option<String>, _>("ocr_error").as_deref()
                )
            })
        })
        .collect();
    
    let response = serde_json::json!({
        "documents": failed_documents,
        "pagination": {
            "total": total_count,
            "limit": limit,
            "offset": offset,
            "has_more": offset + limit < total_count
        },
        "statistics": {
            "total_failed": total_count,
            "failure_categories": get_failure_statistics(&state, auth_user.user.id, auth_user.user.role.clone()).await?
        }
    });
    
    Ok(Json(response))
}

/// View a failed document file
#[utoipa::path(
    get,
    path = "/api/documents/failed/{id}/view",
    tag = "documents",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = uuid::Uuid, Path, description = "Failed Document ID")
    ),
    responses(
        (status = 200, description = "Failed document content for viewing in browser"),
        (status = 404, description = "Failed document not found or file deleted"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn view_failed_document(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(failed_document_id): Path<uuid::Uuid>,
) -> Result<Response, StatusCode> {
    // Get failed document from database
    let row = sqlx::query(
        r#"
        SELECT file_path, filename, mime_type, user_id
        FROM failed_documents 
        WHERE id = $1 AND ($2::uuid IS NULL OR user_id = $2)
        "#
    )
    .bind(failed_document_id)
    .bind(if auth_user.user.role == UserRole::Admin { 
        None 
    } else { 
        Some(auth_user.user.id) 
    })
    .fetch_optional(&state.db.pool)
    .await
    .map_err(|e| {
        error!("Failed to fetch failed document: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .ok_or(StatusCode::NOT_FOUND)?;
    
    let file_path: Option<String> = row.get("file_path");
    let filename: String = row.get("filename");
    let mime_type: Option<String> = row.get("mime_type");
    
    // Check if file_path exists (some failed documents might not have been saved)
    let file_path = file_path.ok_or(StatusCode::NOT_FOUND)?;
    
    let file_service = &state.file_service;
    let file_data = file_service
        .read_file(&file_path)
        .await
        .map_err(|e| {
            error!("Failed to read failed document file: {}", e);
            StatusCode::NOT_FOUND
        })?;
    
    // Determine content type from mime_type or file extension
    let content_type = mime_type
        .unwrap_or_else(|| {
            mime_guess::from_path(&filename)
                .first_or_octet_stream()
                .to_string()
        });
    
    let response = Response::builder()
        .header("Content-Type", content_type)
        .header("Content-Length", file_data.len())
        .header("Content-Disposition", format!("inline; filename=\"{}\"", filename))
        .body(Body::from(file_data))
        .map_err(|e| {
            error!("Failed to build response: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    debug!("Failed document viewed: {}", failed_document_id);
    Ok(response)
}

/// Helper function to categorize failure reasons
fn categorize_failure_reason(failure_reason: Option<&str>, error_message: Option<&str>) -> &'static str {
    match failure_reason {
        Some("pdf_font_encoding") => "PDF Font Issues",
        Some("pdf_corruption") => "PDF Corruption", 
        Some("processing_timeout") => "Timeout",
        Some("memory_limit") => "Memory Limit",
        Some("pdf_parsing_panic") => "PDF Parsing Error",
        Some("low_ocr_confidence") => "Low OCR Confidence",
        Some("unknown") | None => {
            // Try to categorize based on error message
            if let Some(error) = error_message {
                let error_lower = error.to_lowercase();
                if error_lower.contains("timeout") {
                    "Timeout"
                } else if error_lower.contains("memory") {
                    "Memory Limit" 
                } else if error_lower.contains("font") || error_lower.contains("encoding") {
                    "PDF Font Issues"
                } else if error_lower.contains("corrupt") {
                    "PDF Corruption"
                } else if error_lower.contains("quality below threshold") || error_lower.contains("confidence") {
                    "Low OCR Confidence"
                } else {
                    "Unknown Error"
                }
            } else {
                "Unknown Error"
            }
        }
        _ => "Other"
    }
}

/// Helper function to get failure statistics
async fn get_failure_statistics(
    state: &Arc<AppState>, 
    user_id: uuid::Uuid, 
    user_role: UserRole
) -> Result<serde_json::Value, StatusCode> {
    let stats = sqlx::query(
        r#"
        SELECT 
            ocr_failure_reason,
            COUNT(*) as count
        FROM documents 
        WHERE ocr_status = 'failed'
          AND ($1::uuid IS NULL OR user_id = $1)
        GROUP BY ocr_failure_reason
        ORDER BY count DESC
        "#
    )
    .bind(if user_role == UserRole::Admin { 
        None 
    } else { 
        Some(user_id) 
    })
    .fetch_all(state.db.get_pool())
    .await
    .map_err(|e| {
        error!("Failed to get failure statistics: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    let categories: Vec<serde_json::Value> = stats
        .into_iter()
        .map(|row| {
            let reason = row.get::<Option<String>, _>("ocr_failure_reason");
            let count = row.get::<i64, _>("count");
            
            serde_json::json!({
                "reason": reason.clone().unwrap_or_else(|| "unknown".to_string()),
                "display_name": categorize_failure_reason(reason.as_deref(), None),
                "count": count
            })
        })
        .collect();
    
    Ok(serde_json::json!(categories))
}

/// Helper function to calculate estimated wait time for retries
pub async fn calculate_estimated_wait_time(priority: i32) -> i64 {
    // Simple estimation based on priority - in a real implementation,
    // this would check actual queue depth and processing times
    match priority {
        15.. => 1,      // High priority retry: ~1 minute
        10..14 => 3,    // Medium priority: ~3 minutes  
        5..9 => 10,     // Low priority: ~10 minutes
        _ => 30,        // Very low priority: ~30 minutes
    }
}