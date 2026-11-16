//! MCP helpers for integration tests: JSON-RPC over `/api/mcp` and the
//! config switches for the endpoint.

use serde_json::json;
use tower::util::ServiceExt;

use super::{TestConfigBuilder, TestContext};

/// MCP protocol revision the test helpers speak (stateless, no `initialize`).
pub const MCP_TEST_PROTOCOL_VERSION: &str = "2026-07-28";

impl TestConfigBuilder {
    pub fn with_mcp_enabled(mut self, enabled: bool) -> Self {
        self.mcp.enabled = enabled;
        self
    }

    pub fn with_mcp_allow_delete(mut self, allow: bool) -> Self {
        self.mcp.allow_delete = allow;
        self
    }
}

impl TestContext {
    /// POST a raw JSON-RPC message to `/api/mcp` and return the HTTP status
    /// and the parsed body (a JSON string holding the raw text when the body
    /// is not JSON).
    pub async fn mcp_post(
        &self,
        token: Option<&str>,
        message: serde_json::Value,
    ) -> (axum::http::StatusCode, serde_json::Value) {
        let mut request = axum::http::Request::builder()
            .method("POST")
            .uri("/api/mcp")
            .header("content-type", "application/json")
            .header("accept", "application/json, text/event-stream")
            .header(
                "mcp-protocol-version",
                // `initialize` must announce the same revision it negotiates.
                message["params"]["protocolVersion"]
                    .as_str()
                    .unwrap_or(MCP_TEST_PROTOCOL_VERSION),
            );
        if let Some(method) = message["method"].as_str() {
            request = request.header("mcp-method", method);
        }
        // `Mcp-Name` carries the tool name, or the URI for `resources/read`.
        if let Some(name) = message["params"]["name"]
            .as_str()
            .or(message["params"]["uri"].as_str())
        {
            request = request.header("mcp-name", name);
        }
        if let Some(token) = token {
            request = request.header("authorization", format!("Bearer {}", token));
        }
        let request = request
            .body(axum::body::Body::from(message.to_string()))
            .unwrap();
        let response = self.app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = serde_json::from_slice(&bytes).unwrap_or_else(|_| {
            serde_json::Value::String(String::from_utf8_lossy(&bytes).into_owned())
        });
        (status, body)
    }

    /// Send one stateless MCP request (`method` + `params`) and return the
    /// JSON-RPC `result`. Panics on a non-200 status or a JSON-RPC error.
    pub async fn mcp_request(
        &self,
        token: &str,
        method: &str,
        params: serde_json::Value,
    ) -> serde_json::Value {
        let mut params = params;
        params["_meta"] = json!({
            "io.modelcontextprotocol/protocolVersion": MCP_TEST_PROTOCOL_VERSION,
            "io.modelcontextprotocol/clientInfo": { "name": "readur-tests", "version": "1.0" },
            "io.modelcontextprotocol/clientCapabilities": {}
        });
        let message = json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params });
        let (status, body) = self.mcp_post(Some(token), message).await;
        assert_eq!(
            status,
            axum::http::StatusCode::OK,
            "MCP {} failed: {}",
            method,
            body
        );
        assert!(
            body.get("error").is_none(),
            "MCP {} returned an error: {}",
            method,
            body
        );
        body["result"].clone()
    }

    /// Names of the tools `tools/list` returns for this caller.
    pub async fn mcp_tool_names(&self, token: &str) -> Vec<String> {
        let result = self.mcp_request(token, "tools/list", json!({})).await;
        result["tools"]
            .as_array()
            .expect("tools/list result has no tools array")
            .iter()
            .map(|tool| tool["name"].as_str().unwrap().to_string())
            .collect()
    }

    /// Call a tool and return the `CallToolResult` JSON (`content`,
    /// `structuredContent`, `isError`). Tool-level failures come back with
    /// `isError: true` rather than panicking, so tests can assert on them.
    pub async fn mcp_call_tool(
        &self,
        token: &str,
        name: &str,
        arguments: serde_json::Value,
    ) -> serde_json::Value {
        self.mcp_request(
            token,
            "tools/call",
            json!({ "name": name, "arguments": arguments }),
        )
        .await
    }

    /// Create a personal API key (`readur_pat_...`) for the user owning `token`.
    pub async fn create_api_key(&self, token: &str) -> String {
        let request = axum::http::Request::builder()
            .method("POST")
            .uri("/api/auth/keys")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {}", token))
            .body(axum::body::Body::from(
                json!({ "name": "mcp-test" }).to_string(),
            ))
            .unwrap();
        let response = self.app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or_default();
        assert!(
            status.is_success(),
            "creating API key failed: {} {}",
            status,
            body
        );
        body["plaintext"]
            .as_str()
            .expect("API key missing from response")
            .to_string()
    }
}
