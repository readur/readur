use std::sync::Arc;

use http::request::Parts;
use rmcp::{
    handler::server::router::tool::ToolRouter,
    model::{
        Implementation, ListResourceTemplatesResult, PaginatedRequestParams,
        ReadResourceRequestParams, ReadResourceResponse, ServerCapabilities, ServerConfig,
    },
    service::RequestContext,
    tool_handler, ErrorData as McpError, RoleServer, ServerHandler,
};

use super::{tools, McpCaller};
use crate::{models::User, AppState};

const INSTRUCTIONS: &str = "Readur is a document management system with OCR. Use search_documents \
or enhanced_search to find documents by content, get_document_ocr_text to read a document's \
extracted text, and labels to organise documents. All results are limited to what the \
authenticated user can access.";

/// One MCP server instance. A fresh instance is built per request (the
/// transport is stateless), so it only holds cheap shared handles.
#[derive(Clone)]
pub struct ReadurMcp {
    pub(crate) state: Arc<AppState>,
    tool_router: ToolRouter<Self>,
}

impl ReadurMcp {
    pub fn new(state: Arc<AppState>) -> Self {
        let mut tool_router = Self::status_router()
            + Self::documents_router()
            + Self::search_router()
            + Self::labels_router()
            + Self::sources_router();
        if !state.config.mcp.allow_delete {
            tool_router.remove_route("delete_document");
        }
        Self { state, tool_router }
    }

    /// The user the request authenticated as. The HTTP layer rejects
    /// unauthenticated requests before rmcp runs, so a missing caller is an
    /// internal error.
    pub(crate) fn caller(parts: &Parts) -> Result<&User, McpError> {
        parts
            .extensions
            .get::<McpCaller>()
            .map(|caller| &caller.0)
            .ok_or_else(|| McpError::internal_error("request is not authenticated", None))
    }

    pub(crate) fn caller_from_context(
        context: &RequestContext<RoleServer>,
    ) -> Result<&User, McpError> {
        let parts = context
            .extensions
            .get::<Parts>()
            .ok_or_else(|| McpError::internal_error("missing HTTP request context", None))?;
        Self::caller(parts)
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for ReadurMcp {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
        .with_server_info(
            Implementation::new("readur", env!("CARGO_PKG_VERSION")).with_title("Readur"),
        )
        .with_instructions(INSTRUCTIONS)
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        Ok(tools::resources::resource_templates())
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, McpError> {
        let user = Self::caller_from_context(&context)?;
        tools::resources::read_resource(self, user, request).await
    }
}
