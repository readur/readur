//! MCP tools, one module per area. Each module contributes a named
//! `#[tool_router]` that `ReadurMcp::new` combines.

pub mod documents;
pub mod labels;
pub mod resources;
pub mod search;
pub mod sources;
pub mod status;
