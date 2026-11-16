use http::request::Parts;
use rmcp::{
    handler::server::tool::Extension, model::CallToolResult, tool, tool_router,
    ErrorData as McpError,
};

use crate::{
    mcp::{format, ReadurMcp},
    models::UserResponse,
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
}
