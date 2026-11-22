//! Settings for the built-in MCP endpoint at `/api/mcp` (`Config::mcp`).

use anyhow::Result;

use super::parsing::env_flag;

#[derive(Clone, Debug)]
pub struct McpConfig {
    /// Serve the MCP endpoint at all (`MCP_ENABLED`, default on). Every
    /// request still needs a session token or API key.
    pub enabled: bool,
    /// Expose the `delete_document` tool (`MCP_ALLOW_DELETE`, default off).
    pub allow_delete: bool,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            allow_delete: false,
        }
    }
}

impl McpConfig {
    pub fn from_env() -> Result<Self> {
        let defaults = Self::default();
        let enabled = env_flag("MCP_ENABLED", defaults.enabled)?;
        let allow_delete = env_flag("MCP_ALLOW_DELETE", defaults.allow_delete)?;
        println!(
            "🤖 MCP endpoint: {} (delete tool: {})",
            if enabled {
                "enabled at /api/mcp"
            } else {
                "disabled"
            },
            if allow_delete { "on" } else { "off" }
        );
        Ok(Self {
            enabled,
            allow_delete,
        })
    }
}
