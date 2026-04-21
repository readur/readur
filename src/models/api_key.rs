use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// A personal API key for programmatic authentication.
///
/// The full plaintext token is only ever known at creation time and is never
/// stored. `key_hash` is the SHA-256 (hex) digest of the full `readur_pat_<...>`
/// string and is what the auth extractor compares against on each request.
/// `key_prefix` is the first 12 characters of the plaintext (`readur_pat_X`),
/// stored separately so the UI can identify keys without exposing them.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ApiKey {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    #[serde(skip_serializing)]
    pub key_hash: String,
    pub key_prefix: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl ApiKey {
    pub fn is_expired(&self) -> bool {
        self.expires_at.map_or(false, |exp| exp < Utc::now())
    }

    pub fn is_revoked(&self) -> bool {
        self.revoked_at.is_some()
    }
}

/// Payload for creating a new API key. `expires_in_days` is validated server-side
/// to fall within `1..=365`. `None` means "no expiration".
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub expires_in_days: Option<u32>,
}

/// Metadata-only view of an API key. Never contains the plaintext or hash.
#[derive(Debug, Serialize, ToSchema)]
pub struct ApiKeyResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub key_prefix: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub is_expired: bool,
    pub created_at: DateTime<Utc>,
}

impl From<ApiKey> for ApiKeyResponse {
    fn from(k: ApiKey) -> Self {
        let is_expired = k.is_expired();
        Self {
            id: k.id,
            user_id: k.user_id,
            name: k.name,
            key_prefix: k.key_prefix,
            expires_at: k.expires_at,
            last_used_at: k.last_used_at,
            revoked_at: k.revoked_at,
            is_expired,
            created_at: k.created_at,
        }
    }
}

/// Response body returned only from the create endpoint. `plaintext` is the
/// full `readur_pat_<...>` value and is the single chance the caller has to
/// capture it — the server does not retain it.
#[derive(Debug, Serialize, ToSchema)]
pub struct CreateApiKeyResponse {
    pub api_key: ApiKeyResponse,
    pub plaintext: String,
}
