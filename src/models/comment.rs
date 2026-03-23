use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct DocumentComment {
    pub id: Uuid,
    pub document_id: Uuid,
    pub user_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub content: String,
    pub is_edited: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct CommentWithAuthor {
    pub id: Uuid,
    pub document_id: Uuid,
    pub user_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub content: String,
    pub is_edited: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub username: String,
    pub user_role: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CommentThread {
    #[serde(flatten)]
    pub comment: CommentWithAuthor,
    pub reply_count: i64,
    pub replies: Vec<CommentWithAuthor>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateCommentRequest {
    pub content: String,
    pub parent_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateCommentRequest {
    pub content: String,
}
