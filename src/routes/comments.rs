use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put, delete},
    Router,
};
use serde::Deserialize;
use std::sync::Arc;
use tracing::{debug, error, warn};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    errors::comment::CommentError,
    models::comment::{
        CommentThread, CommentWithAuthor, CreateCommentRequest, UpdateCommentRequest,
    },
    models::UserRole,
    AppState,
};

const MAX_COMMENT_LENGTH: usize = 10_000;
const MAX_PAGINATION_LIMIT: i64 = 100;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/documents/{document_id}/comments", get(list_comments))
        .route("/documents/{document_id}/comments", post(create_comment))
        .route("/documents/{document_id}/comments/{comment_id}/replies", get(list_replies))
        .route("/documents/{document_id}/comments/{comment_id}", put(update_comment))
        .route("/documents/{document_id}/comments/{comment_id}", delete(delete_comment))
        .route("/documents/{document_id}/comments/count", get(get_comment_count))
}

#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    50
}

/// Verify the authenticated user can access the document
async fn verify_document_access(
    state: &AppState,
    document_id: Uuid,
    user_id: Uuid,
    role: UserRole,
) -> Result<(), CommentError> {
    state
        .db
        .get_document_by_id(document_id, user_id, role)
        .await
        .map_err(|e| {
            error!("Failed to verify document access: {}", e);
            CommentError::InternalError { message: "Failed to verify document access".into() }
        })?
        .ok_or(CommentError::DocumentNotFound)?;
    Ok(())
}

pub async fn list_comments(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(document_id): Path<Uuid>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<Vec<CommentThread>>, CommentError> {
    verify_document_access(&state, document_id, auth_user.user.id, auth_user.user.role).await?;

    let limit = pagination.limit.min(MAX_PAGINATION_LIMIT).max(1);
    let comments = state
        .db
        .get_comments_by_document(document_id, limit, pagination.offset)
        .await
        .map_err(|e| {
            error!("Failed to fetch comments: {}", e);
            CommentError::InternalError { message: "Failed to fetch comments".into() }
        })?;

    // Build threads with reply counts and first few replies
    let mut threads = Vec::with_capacity(comments.len());
    for comment in comments {
        let reply_count = state
            .db
            .get_reply_count(comment.id)
            .await
            .unwrap_or(0);

        let replies = if reply_count > 0 {
            state
                .db
                .get_replies(comment.id, 3, 0) // First 3 replies inline
                .await
                .unwrap_or_default()
        } else {
            vec![]
        };

        threads.push(CommentThread {
            comment,
            reply_count,
            replies,
        });
    }

    Ok(Json(threads))
}

pub async fn list_replies(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path((document_id, comment_id)): Path<(Uuid, Uuid)>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<Vec<CommentWithAuthor>>, CommentError> {
    verify_document_access(&state, document_id, auth_user.user.id, auth_user.user.role).await?;

    // Verify the parent comment belongs to this document
    let parent = state
        .db
        .get_comment_by_id(comment_id)
        .await
        .map_err(|e| {
            error!("Failed to fetch parent comment: {}", e);
            CommentError::InternalError { message: "Failed to fetch comment".into() }
        })?
        .ok_or(CommentError::NotFound)?;

    if parent.document_id != document_id {
        return Err(CommentError::NotFound);
    }

    let limit = pagination.limit.min(MAX_PAGINATION_LIMIT).max(1);
    let replies = state
        .db
        .get_replies(comment_id, limit, pagination.offset)
        .await
        .map_err(|e| {
            error!("Failed to fetch replies: {}", e);
            CommentError::InternalError { message: "Failed to fetch replies".into() }
        })?;

    Ok(Json(replies))
}

pub async fn create_comment(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(document_id): Path<Uuid>,
    Json(payload): Json<CreateCommentRequest>,
) -> Result<(StatusCode, Json<CommentWithAuthor>), CommentError> {
    // Rate limit comment creation per user
    if let Err(retry_after) = state.rate_limiters.comment_creation.check(&auth_user.user.id).await {
        warn!("Rate limited comment creation for user {}", auth_user.user.id);
        return Err(CommentError::RateLimited { retry_after_secs: retry_after });
    }

    verify_document_access(&state, document_id, auth_user.user.id, auth_user.user.role).await?;

    let content = payload.content.trim();
    if content.is_empty() {
        return Err(CommentError::ContentEmpty);
    }
    if content.len() > MAX_COMMENT_LENGTH {
        return Err(CommentError::ContentTooLong {
            length: content.len(),
            max_length: MAX_COMMENT_LENGTH,
        });
    }

    // Enforce 1-level nesting
    if let Some(parent_id) = payload.parent_id {
        let parent = state
            .db
            .get_comment_by_id(parent_id)
            .await
            .map_err(|e| {
                error!("Failed to fetch parent comment: {}", e);
                CommentError::InternalError { message: "Failed to verify parent comment".into() }
            })?
            .ok_or(CommentError::ParentNotFound)?;

        if parent.parent_id.is_some() {
            return Err(CommentError::NestingTooDeep);
        }

        if parent.document_id != document_id {
            return Err(CommentError::ParentNotFound);
        }
    }

    let comment = state
        .db
        .create_comment(document_id, auth_user.user.id, payload.parent_id, content)
        .await
        .map_err(|e| {
            error!("Failed to create comment: {}", e);
            CommentError::InternalError { message: "Failed to create comment".into() }
        })?;

    // Return with author info
    let comment_with_author = CommentWithAuthor {
        id: comment.id,
        document_id: comment.document_id,
        user_id: comment.user_id,
        parent_id: comment.parent_id,
        content: comment.content,
        is_edited: comment.is_edited,
        created_at: comment.created_at,
        updated_at: comment.updated_at,
        username: auth_user.user.username.clone(),
        user_role: format!("{:?}", auth_user.user.role).to_lowercase(),
    };

    debug!("Comment created on document {} by user {}", document_id, auth_user.user.id);
    Ok((StatusCode::CREATED, Json(comment_with_author)))
}

pub async fn update_comment(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path((document_id, comment_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<UpdateCommentRequest>,
) -> Result<Json<CommentWithAuthor>, CommentError> {
    verify_document_access(&state, document_id, auth_user.user.id, auth_user.user.role).await?;

    let existing = state
        .db
        .get_comment_by_id(comment_id)
        .await
        .map_err(|e| {
            error!("Failed to fetch comment: {}", e);
            CommentError::InternalError { message: "Failed to fetch comment".into() }
        })?
        .ok_or(CommentError::NotFound)?;

    // Verify the comment belongs to the document in the URL path
    if existing.document_id != document_id {
        return Err(CommentError::NotFound);
    }

    // Only the author can edit
    if existing.user_id != auth_user.user.id {
        return Err(CommentError::PermissionDenied {
            reason: "You can only edit your own comments".into(),
        });
    }

    let content = payload.content.trim();
    if content.is_empty() {
        return Err(CommentError::ContentEmpty);
    }
    if content.len() > MAX_COMMENT_LENGTH {
        return Err(CommentError::ContentTooLong {
            length: content.len(),
            max_length: MAX_COMMENT_LENGTH,
        });
    }

    let updated = state
        .db
        .update_comment(comment_id, content)
        .await
        .map_err(|e| {
            error!("Failed to update comment: {}", e);
            CommentError::InternalError { message: "Failed to update comment".into() }
        })?;

    let comment_with_author = CommentWithAuthor {
        id: updated.id,
        document_id: updated.document_id,
        user_id: updated.user_id,
        parent_id: updated.parent_id,
        content: updated.content,
        is_edited: updated.is_edited,
        created_at: updated.created_at,
        updated_at: updated.updated_at,
        username: auth_user.user.username.clone(),
        user_role: format!("{:?}", auth_user.user.role).to_lowercase(),
    };

    Ok(Json(comment_with_author))
}

pub async fn delete_comment(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path((document_id, comment_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, CommentError> {
    verify_document_access(&state, document_id, auth_user.user.id, auth_user.user.role).await?;

    let existing = state
        .db
        .get_comment_by_id(comment_id)
        .await
        .map_err(|e| {
            error!("Failed to fetch comment: {}", e);
            CommentError::InternalError { message: "Failed to fetch comment".into() }
        })?
        .ok_or(CommentError::NotFound)?;

    // Verify the comment belongs to the document in the URL path
    if existing.document_id != document_id {
        return Err(CommentError::NotFound);
    }

    // Owner or admin can delete
    if existing.user_id != auth_user.user.id && auth_user.user.role != UserRole::Admin {
        return Err(CommentError::PermissionDenied {
            reason: "You can only delete your own comments".into(),
        });
    }

    state
        .db
        .delete_comment(comment_id)
        .await
        .map_err(|e| {
            error!("Failed to delete comment: {}", e);
            CommentError::InternalError { message: "Failed to delete comment".into() }
        })?;

    debug!("Comment {} deleted by user {}", comment_id, auth_user.user.id);
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_comment_count(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(document_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, CommentError> {
    verify_document_access(&state, document_id, auth_user.user.id, auth_user.user.role).await?;

    let count = state
        .db
        .get_comment_count(document_id)
        .await
        .map_err(|e| {
            error!("Failed to get comment count: {}", e);
            CommentError::InternalError { message: "Failed to get comment count".into() }
        })?;

    Ok(Json(serde_json::json!({ "count": count })))
}
