use http::request::Parts;
use rmcp::{
    handler::server::tool::Extension, model::CallToolResult, tool, tool_router,
    ErrorData as McpError,
};

use crate::{
    mcp::{format, ReadurMcp},
    models::{UserResponse, UserRole},
    routes::{
        ocr::available_languages_for_user, queue::QueueStatsResponse,
        settings::settings_response_for_user,
    },
};

#[tool_router(router = status_router, vis = "pub(crate)")]
impl ReadurMcp {
    #[tool(
        description = "Get information about the authenticated Readur user: ID, username, email, and role.",
        annotations(title = "Who Am I", read_only_hint = true, open_world_hint = false)
    )]
    async fn whoami(&self, Extension(parts): Extension<Parts>) -> Result<CallToolResult, McpError> {
        let user = UserResponse::from(Self::caller(&parts)?.clone());
        let text = format!(
            "Username: {}\nEmail: {}\nRole: {:?}\nUser ID: {}",
            user.username, user.email, user.role, user.id
        );
        format::text_and_json(text, &user)
    }

    #[tool(
        description = "Get the authenticated user's Readur settings (OCR language and tuning, search, \
                       file limits, WebDAV, ...) as key-value pairs. Stored passwords are never returned; \
                       only whether one is set.",
        annotations(title = "Get Settings", read_only_hint = true, open_world_hint = false)
    )]
    async fn get_settings(
        &self,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, McpError> {
        let user = Self::caller(&parts)?;
        let settings = settings_response_for_user(&self.state.db, user.id)
            .await
            .map_err(|e| format::internal("load settings", e))?;

        let value = serde_json::to_value(&settings)
            .map_err(|e| format::internal("serialize settings", e))?;
        let mut lines = vec!["Settings:".to_string()];
        if let Some(fields) = value.as_object() {
            for (key, value) in fields {
                let shown = match value {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Null => "(not set)".to_string(),
                    other => other.to_string(),
                };
                lines.push(format!("- {key}: {shown}"));
            }
        }
        format::text_and_json(lines.join("\n"), &settings)
    }

    #[tool(
        description = "Get OCR processing queue statistics: pending, processing, and failed jobs, jobs \
                       completed today, and average/oldest wait times. The queue is shared by all users, \
                       so this requires an administrator account.",
        annotations(
            title = "Get Queue Stats",
            read_only_hint = true,
            open_world_hint = false
        )
    )]
    async fn get_queue_stats(
        &self,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, McpError> {
        // Mirrors `/api/queue/stats`, which is admin-only.
        if Self::caller(&parts)?.role != UserRole::Admin {
            return Ok(format::tool_error(
                "Queue statistics are only available to administrators.",
            ));
        }
        let stats = self
            .state
            .queue_service
            .get_stats()
            .await
            .map_err(|e| format::internal("load queue statistics", e))?;
        let stats = QueueStatsResponse::from(stats);

        let minutes = |m: Option<f64>| m.map_or("n/a".to_string(), |m| format!("{m:.1} min"));
        let text = format!(
            "OCR queue:\n- Pending: {}\n- Processing: {}\n- Failed: {}\n- Completed today: {}\n\
             - Average wait: {}\n- Oldest pending: {}",
            stats.pending,
            stats.processing,
            stats.failed,
            stats.completed_today,
            minutes(stats.avg_wait_time_minutes),
            minutes(stats.oldest_pending_minutes),
        );
        format::text_and_json(text, &stats)
    }

    #[tool(
        description = "List the OCR languages installed on the Readur server (code and name) and the \
                       authenticated user's current OCR language.",
        annotations(
            title = "Get OCR Languages",
            read_only_hint = true,
            open_world_hint = false
        )
    )]
    async fn get_ocr_languages(
        &self,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, McpError> {
        let user = Self::caller(&parts)?;
        let languages = available_languages_for_user(&self.state.db, user.id)
            .await
            .map_err(|e| format::internal("load OCR languages", e))?;

        let mut lines = vec![format!(
            "Current OCR language: {}\nInstalled languages ({}):",
            languages.current_user_language,
            languages.available_languages.len()
        )];
        lines.extend(
            languages
                .available_languages
                .iter()
                .map(|lang| format!("- {} ({})", lang.name, lang.code)),
        );
        format::text_and_json(lines.join("\n"), &languages)
    }
}
