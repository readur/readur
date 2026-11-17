//! Shared output helpers. Tools return human-readable text for the model
//! plus the same data as `structured_content` for programmatic clients.

use chrono::{DateTime, Utc};
use rmcp::{
    model::{CallToolResult, ContentBlock},
    ErrorData as McpError,
};
use serde::Serialize;

/// A successful result carrying `text` and `data` as structured content.
pub fn text_and_json(
    text: impl Into<String>,
    data: &impl Serialize,
) -> Result<CallToolResult, McpError> {
    let value = serde_json::to_value(data)
        .map_err(|e| McpError::internal_error(format!("failed to serialize result: {e}"), None))?;
    let mut result = CallToolResult::success(vec![ContentBlock::text(text.into())]);
    result.structured_content = Some(value);
    Ok(result)
}

/// A tool-level failure the model should see and can act on (bad input,
/// not found, ...). Protocol and server faults use `McpError` instead.
pub fn tool_error(message: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(message.into())])
}

/// A server fault: log `error` in full, give the client only `failed to
/// {action}` so internal details never reach the model.
pub fn internal(action: &str, error: impl std::fmt::Display) -> McpError {
    tracing::error!("MCP failed to {action}: {error}");
    McpError::internal_error(format!("failed to {action}"), None)
}

/// Binary-unit file size, e.g. `1.5 MiB`.
pub fn file_size(bytes: i64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

/// `2026-11-16 14:03:22 UTC`
pub fn date(timestamp: DateTime<Utc>) -> String {
    timestamp.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_errors_hide_the_underlying_error() {
        let err = internal(
            "load settings",
            "password authentication failed for user \"readur\"",
        );
        assert_eq!(err.message, "failed to load settings");
    }

    #[test]
    fn file_size_uses_binary_units() {
        assert_eq!(file_size(0), "0 B");
        assert_eq!(file_size(1023), "1023 B");
        assert_eq!(file_size(1536), "1.5 KiB");
        assert_eq!(file_size(5 * 1024 * 1024), "5.0 MiB");
    }
}
