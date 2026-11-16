use chrono::{DateTime, Utc};
use http::request::Parts;
use rmcp::{
    handler::server::{tool::Extension, wrapper::Parameters},
    model::CallToolResult,
    tool, tool_router, ErrorData as McpError,
};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    mcp::{format, ReadurMcp},
    models::{SourceResponse, SourceStatus, SourceType},
    routes::sources::source_responses_for_user,
    services::sync_progress_tracker::SyncProgressInfo,
};

#[derive(Debug, serde::Deserialize, rmcp::schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct SourceSyncStatusParams {
    /// ID (UUID) of the source, as returned by `list_sources`.
    #[schemars(with = "String", extend("format" = "uuid"))]
    pub source_id: Uuid,
}

#[derive(Serialize)]
struct SourceList {
    sources: Vec<SourceResponse>,
}

/// Sync state of one source. Leaves out the source configuration entirely.
#[derive(Serialize)]
struct SourceSyncStatus {
    source_id: Uuid,
    name: String,
    source_type: SourceType,
    enabled: bool,
    status: SourceStatus,
    last_sync_at: Option<DateTime<Utc>>,
    last_error: Option<String>,
    last_error_at: Option<DateTime<Utc>>,
    total_files_synced: i64,
    total_files_pending: i64,
    sync_in_progress: bool,
    /// Live progress of the running sync, if any.
    progress: Option<SyncProgressInfo>,
}

fn last_sync(at: Option<DateTime<Utc>>) -> String {
    at.map_or("never".to_string(), format::date)
}

#[tool_router(router = sources_router, vis = "pub(crate)")]
impl ReadurMcp {
    #[tool(
        description = "List the authenticated user's document sources (WebDAV servers, local folders, \
                       S3 buckets) with their status, last sync time, last error, and document counts. \
                       Connection settings are included but credentials are never returned; only whether \
                       one is set (e.g. `has_password`).",
        annotations(title = "List Sources", read_only_hint = true, open_world_hint = false)
    )]
    async fn list_sources(
        &self,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, McpError> {
        let user = Self::caller(&parts)?;
        let sources = source_responses_for_user(&self.state.db, user.id)
            .await
            .map_err(|e| {
                tracing::error!("MCP list_sources failed for user {}: {}", user.id, e);
                McpError::internal_error("failed to load sources", None)
            })?;

        let text = if sources.is_empty() {
            "No sources configured.".to_string()
        } else {
            let mut lines = vec![format!("{} source(s):", sources.len())];
            for s in &sources {
                lines.push(format!(
                    "- {} ({}, id {}): {}{}, {} documents ({} with OCR), last sync {}",
                    s.name,
                    s.source_type,
                    s.id,
                    s.status,
                    if s.enabled { "" } else { ", disabled" },
                    s.total_documents,
                    s.total_documents_ocr,
                    last_sync(s.last_sync_at),
                ));
                if let Some(error) = &s.last_error {
                    lines.push(format!("  Last error: {error}"));
                }
            }
            lines.join("\n")
        };
        format::text_and_json(text, &SourceList { sources })
    }

    #[tool(
        description = "Get the sync status of one of the authenticated user's sources: idle/syncing/error, \
                       last sync time and error, file totals, and live progress (phase, files processed, \
                       estimated time remaining) when a sync is running.",
        annotations(
            title = "Get Source Sync Status",
            read_only_hint = true,
            open_world_hint = false
        )
    )]
    async fn get_source_sync_status(
        &self,
        Parameters(p): Parameters<SourceSyncStatusParams>,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, McpError> {
        let user = Self::caller(&parts)?;
        // Same ownership check as `/api/sources/{id}/sync/status`.
        let source = self
            .state
            .db
            .get_source(user.id, p.source_id)
            .await
            .map_err(|e| {
                tracing::error!(
                    "MCP get_source_sync_status failed for source {}: {}",
                    p.source_id,
                    e
                );
                McpError::internal_error("failed to load source", None)
            })?;
        let Some(source) = source else {
            return Ok(format::tool_error(format!(
                "Source {} not found.",
                p.source_id
            )));
        };
        let progress = self.state.sync_progress_tracker.get_progress(source.id);

        let status = SourceSyncStatus {
            source_id: source.id,
            name: source.name,
            source_type: source.source_type,
            enabled: source.enabled,
            status: source.status,
            last_sync_at: source.last_sync_at,
            last_error: source.last_error,
            last_error_at: source.last_error_at,
            total_files_synced: source.total_files_synced,
            total_files_pending: source.total_files_pending,
            sync_in_progress: progress.as_ref().is_some_and(|p| p.is_active),
            progress,
        };

        let mut lines = vec![
            format!(
                "Source: {} ({}, id {})",
                status.name, status.source_type, status.source_id
            ),
            format!(
                "Status: {}{}",
                status.status,
                if status.enabled { "" } else { " (disabled)" }
            ),
            format!("Last sync: {}", last_sync(status.last_sync_at)),
            format!(
                "Files synced: {}, pending: {}",
                status.total_files_synced, status.total_files_pending
            ),
        ];
        if let Some(error) = &status.last_error {
            lines.push(format!("Last error: {error}"));
        }
        match &status.progress {
            Some(progress) if progress.is_active => {
                lines.push(format!(
                    "Sync in progress: {} ({}), {}/{} files ({:.0}%)",
                    progress.phase,
                    progress.phase_description,
                    progress.files_processed,
                    progress.files_found,
                    progress.files_progress_percent,
                ));
                if let Some(remaining) = progress.estimated_time_remaining_secs {
                    lines.push(format!("Estimated time remaining: {remaining}s"));
                }
            }
            Some(progress) => lines.push(format!(
                "Most recent sync finished: {} ({})",
                progress.phase, progress.phase_description
            )),
            None => lines.push("No sync in progress.".to_string()),
        }
        format::text_and_json(lines.join("\n"), &status)
    }
}
