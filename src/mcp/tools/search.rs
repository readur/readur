use rmcp::tool_router;

use crate::mcp::ReadurMcp;

#[tool_router(router = search_router, vis = "pub(crate)", allow_empty)]
impl ReadurMcp {}
