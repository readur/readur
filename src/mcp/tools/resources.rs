//! MCP resources: original document files at `readur://documents/{id}`.

use rmcp::{
    model::{ListResourceTemplatesResult, ReadResourceRequestParams, ReadResourceResponse},
    ErrorData as McpError,
};

use crate::{mcp::ReadurMcp, models::User};

pub fn resource_templates() -> ListResourceTemplatesResult {
    ListResourceTemplatesResult::default()
}

pub async fn read_resource(
    _server: &ReadurMcp,
    _user: &User,
    request: ReadResourceRequestParams,
) -> Result<ReadResourceResponse, McpError> {
    Err(McpError::resource_not_found(
        format!("unknown resource: {}", request.uri),
        None,
    ))
}
