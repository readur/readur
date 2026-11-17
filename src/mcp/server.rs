use std::sync::{Arc, OnceLock};

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
    tool_router: Arc<ToolRouter<Self>>,
}

/// The tool set depends only on `MCP_ALLOW_DELETE`, so each variant (tool
/// metadata and input schemas included) is built once per process rather
/// than once per request.
static TOOLS_WITHOUT_DELETE: OnceLock<Arc<ToolRouter<ReadurMcp>>> = OnceLock::new();
static TOOLS_WITH_DELETE: OnceLock<Arc<ToolRouter<ReadurMcp>>> = OnceLock::new();

impl ReadurMcp {
    pub fn new(state: Arc<AppState>) -> Self {
        let tool_router = Self::tools(state.config.mcp.allow_delete);
        Self { state, tool_router }
    }

    fn tools(allow_delete: bool) -> Arc<ToolRouter<Self>> {
        let cache = if allow_delete {
            &TOOLS_WITH_DELETE
        } else {
            &TOOLS_WITHOUT_DELETE
        };
        cache
            .get_or_init(|| {
                let mut router = Self::status_router()
                    + Self::documents_router()
                    + Self::search_router()
                    + Self::labels_router()
                    + Self::sources_router();
                if !allow_delete {
                    router.remove_route("delete_document");
                }
                Arc::new(router)
            })
            .clone()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn names(router: &ToolRouter<ReadurMcp>) -> Vec<String> {
        router
            .list_all()
            .into_iter()
            .map(|tool| tool.name.to_string())
            .collect()
    }

    #[test]
    fn tool_routers_are_built_once_per_variant() {
        assert!(Arc::ptr_eq(
            &ReadurMcp::tools(false),
            &ReadurMcp::tools(false)
        ));
        assert!(Arc::ptr_eq(
            &ReadurMcp::tools(true),
            &ReadurMcp::tools(true)
        ));
        assert!(!Arc::ptr_eq(
            &ReadurMcp::tools(false),
            &ReadurMcp::tools(true)
        ));
    }

    /// Clients may retry tools annotated idempotent without asking, so the
    /// claim must be true: repeating the call must succeed with the same
    /// end state. Changing this list means checking that for the tool.
    #[test]
    fn idempotent_annotations_are_deliberate() {
        let mut idempotent: Vec<String> = ReadurMcp::tools(true)
            .list_all()
            .into_iter()
            .filter(|tool| tool.annotations.as_ref().and_then(|a| a.idempotent_hint) == Some(true))
            .map(|tool| tool.name.to_string())
            .collect();
        idempotent.sort();
        assert_eq!(
            idempotent,
            [
                "add_document_label",
                "remove_document_label",
                "update_label"
            ]
        );
    }

    /// Every tool that changes nothing must say so, and every one that
    /// changes something must say whether it destroys data.
    #[test]
    fn every_tool_declares_its_effects() {
        for tool in ReadurMcp::tools(true).list_all() {
            let annotations = tool
                .annotations
                .as_ref()
                .unwrap_or_else(|| panic!("{} has no annotations", tool.name));
            let read_only = annotations
                .read_only_hint
                .unwrap_or_else(|| panic!("{} lacks read_only_hint", tool.name));
            if !read_only {
                assert!(
                    annotations.destructive_hint.is_some(),
                    "{} lacks destructive_hint",
                    tool.name
                );
            }
            assert_eq!(annotations.open_world_hint, Some(false), "{}", tool.name);
        }
    }

    #[test]
    fn delete_tool_only_exists_when_allowed() {
        let without = names(&ReadurMcp::tools(false));
        let with = names(&ReadurMcp::tools(true));
        assert!(!without.contains(&"delete_document".to_string()));
        assert!(with.contains(&"delete_document".to_string()));
        assert_eq!(with.len(), without.len() + 1);
    }
}
