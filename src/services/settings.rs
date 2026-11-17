//! User settings as the API reports them.

use crate::models::SettingsResponse;

/// A user's settings as the API reports them, or the defaults for a user who
/// has never saved any. Backs `GET /api/settings` and the MCP
/// `get_settings` tool.
pub async fn settings_response_for_user(
    db: &crate::db::Database,
    user_id: uuid::Uuid,
) -> anyhow::Result<SettingsResponse> {
    Ok(db
        .get_user_settings(user_id)
        .await?
        .unwrap_or_default()
        .into())
}
