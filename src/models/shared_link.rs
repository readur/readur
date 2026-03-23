use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct SharedLink {
    pub id: Uuid,
    pub document_id: Uuid,
    pub created_by: Uuid,
    pub token: String,
    #[serde(skip_serializing)]
    pub password_hash: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub max_views: Option<i32>,
    pub view_count: i32,
    pub is_revoked: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateSharedLinkRequest {
    pub document_id: Uuid,
    pub password: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub max_views: Option<i32>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SharedLinkResponse {
    pub id: Uuid,
    pub document_id: Uuid,
    pub token: String,
    pub url: String,
    pub has_password: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub max_views: Option<i32>,
    pub view_count: i32,
    pub is_expired: bool,
    pub is_revoked: bool,
    pub created_at: DateTime<Utc>,
}

impl SharedLinkResponse {
    pub fn from_shared_link(link: SharedLink, base_url: &str) -> Self {
        let is_expired = link.expires_at.map_or(false, |exp| exp < Utc::now())
            || link.max_views.map_or(false, |max| link.view_count >= max);

        Self {
            id: link.id,
            document_id: link.document_id,
            token: link.token.clone(),
            url: format!("{}/shared/{}", base_url, link.token),
            has_password: link.password_hash.is_some(),
            expires_at: link.expires_at,
            max_views: link.max_views,
            view_count: link.view_count,
            is_expired,
            is_revoked: link.is_revoked,
            created_at: link.created_at,
        }
    }
}

/// Request body for accessing a password-protected shared link
#[derive(Debug, Deserialize, ToSchema)]
pub struct SharedLinkPasswordRequest {
    pub password: String,
}

/// Metadata response for public shared link access
#[derive(Debug, Serialize, ToSchema)]
pub struct SharedDocumentMetadata {
    pub filename: String,
    pub original_filename: String,
    pub file_size: i64,
    pub mime_type: String,
    pub requires_password: bool,
    pub created_at: DateTime<Utc>,
}
