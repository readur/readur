//! End-to-end MCP tests against a running readur server (`API_URL`, default
//! `http://localhost:8000`), the way real MCP clients talk to it: plain HTTP
//! over TCP with a real `Host` header, the 2025-06-18 handshake most
//! clients still use (`initialize`, then requests), and a `readur_pat_` API
//! key. The in-process suites (`integration_mcp_*_tests.rs`) cover each tool
//! in detail; these prove the whole stack, OCR worker included, works.

use std::time::Duration;

use base64ct::{Base64, Encoding};
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use uuid::Uuid;

use readur::models::UserRole;
use readur::test_utils::{create_live_server_user, test_password};

const PROTOCOL_VERSION: &str = "2025-06-18";

fn base_url() -> String {
    std::env::var("API_URL").unwrap_or_else(|_| "http://localhost:8000".to_string())
}

struct McpClient {
    http: Client,
    key: String,
}

impl McpClient {
    /// A fresh user on the live server, authenticated with a new API key.
    async fn new() -> Self {
        let http = Client::new();
        let suffix = Uuid::new_v4().simple().to_string();
        let username = format!("mcp_e2e_{suffix}");
        let password = test_password();
        create_live_server_user(
            &username,
            &format!("{username}@example.com"),
            &password,
            UserRole::User,
        )
        .await
        .expect("create live server user");

        let login: Value = http
            .post(format!("{}/api/auth/login", base_url()))
            .json(&json!({ "username": username, "password": password }))
            .send()
            .await
            .expect("login request")
            .json()
            .await
            .expect("login response");
        let jwt = login["token"].as_str().expect("session token").to_string();

        let created: Value = http
            .post(format!("{}/api/auth/keys", base_url()))
            .bearer_auth(&jwt)
            .json(&json!({ "name": "mcp-e2e" }))
            .send()
            .await
            .expect("create API key request")
            .json()
            .await
            .expect("create API key response");
        let key = created["plaintext"]
            .as_str()
            .expect("API key plaintext")
            .to_string();
        assert!(key.starts_with("readur_pat_"));
        Self { http, key }
    }

    async fn post(&self, token: Option<&str>, message: Value) -> (StatusCode, Value) {
        let mut request = self
            .http
            .post(format!("{}/api/mcp", base_url()))
            .header("accept", "application/json, text/event-stream")
            .header("mcp-protocol-version", PROTOCOL_VERSION)
            .json(&message);
        if let Some(token) = token {
            request = request.bearer_auth(token);
        }
        let response = request.send().await.expect("MCP request");
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        (
            status,
            serde_json::from_str(&body).unwrap_or(Value::String(body)),
        )
    }

    async fn request(&self, method: &str, params: Value) -> Value {
        let (status, body) = self
            .post(
                Some(&self.key),
                json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params }),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{method}: {body}");
        assert!(body.get("error").is_none(), "{method}: {body}");
        body["result"].clone()
    }

    async fn call(&self, tool: &str, arguments: Value) -> Value {
        let result = self
            .request(
                "tools/call",
                json!({ "name": tool, "arguments": arguments }),
            )
            .await;
        assert_eq!(result["isError"], json!(false), "{tool}: {result}");
        result
    }

    /// Poll until OCR has produced text for the document.
    async fn wait_for_ocr_text(&self, id: &str) -> Value {
        for _ in 0..90 {
            let result = self
                .call("get_document_ocr_text", json!({ "id": id }))
                .await;
            if result["structuredContent"]["has_ocr_text"] == json!(true) {
                return result;
            }
            assert_ne!(
                result["structuredContent"]["ocr_status"],
                json!("failed"),
                "OCR failed: {result}"
            );
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        panic!("OCR did not finish for document {id}");
    }
}

#[tokio::test]
async fn live_mcp_rejects_missing_and_bogus_credentials() {
    let client = McpClient::new().await;
    let list = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {} });
    let (status, _) = client.post(None, list.clone()).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = client
        .post(Some("readur_pat_bogus0000000000000000000000"), list)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn live_mcp_handshake_and_tool_list() {
    let client = McpClient::new().await;
    let init = client
        .request(
            "initialize",
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": { "name": "readur-e2e", "version": "1.0" }
            }),
        )
        .await;
    assert_eq!(init["serverInfo"]["name"], json!("readur"));
    assert!(init["capabilities"]["tools"].is_object());
    assert!(init["capabilities"]["resources"].is_object());

    let tools = client.request("tools/list", json!({})).await;
    let names: Vec<&str> = tools["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    for expected in [
        "whoami",
        "search_documents",
        "upload_document",
        "get_document_ocr_text",
        "list_labels",
    ] {
        assert!(
            names.contains(&expected),
            "{expected} missing from {names:?}"
        );
    }
    // Off unless MCP_ALLOW_DELETE=true, which the CI server does not set.
    assert!(!names.contains(&"delete_document"), "{names:?}");
}

#[tokio::test]
async fn live_mcp_upload_ocr_search_and_read_back() {
    let client = McpClient::new().await;
    let marker = format!("zebracorn{}", &Uuid::new_v4().simple().to_string()[..8]);
    let content = format!("Quarterly invoice {marker}. Total due: 4217 dollars.\n");
    let encoded = Base64::encode_string(content.as_bytes());

    let upload = client
        .call(
            "upload_document",
            json!({ "filename": "e2e-invoice.txt", "content_base64": encoded }),
        )
        .await;
    assert_eq!(
        upload["structuredContent"]["warnings"],
        json!([]),
        "{upload}"
    );
    let id = upload["structuredContent"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let ocr = client.wait_for_ocr_text(&id).await;
    assert!(
        ocr["structuredContent"]["text"]
            .as_str()
            .unwrap()
            .contains(&marker),
        "{ocr}"
    );

    let search = client
        .call("search_documents", json!({ "query": marker }))
        .await;
    assert!(
        search["content"][0]["text"].as_str().unwrap().contains(&id),
        "{search}"
    );

    let resource = client
        .request(
            "resources/read",
            json!({ "uri": format!("readur://documents/{id}") }),
        )
        .await;
    assert_eq!(
        resource["contents"][0]["text"],
        json!(content),
        "{resource}"
    );
}

#[tokio::test]
async fn live_mcp_retrying_ocr_twice_is_safe() {
    let client = McpClient::new().await;
    let encoded = Base64::encode_string(b"retry me twice\n");
    let upload = client
        .call(
            "upload_document",
            json!({ "filename": "e2e-retry.txt", "content_base64": encoded }),
        )
        .await;
    let id = upload["structuredContent"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    client.wait_for_ocr_text(&id).await;

    // Back to back, the second call races the real OCR worker: the first job
    // may still be queued, running, or (for a tiny file) already done. Every
    // outcome must be a successful call -- before the fix, a still-queued job
    // made the second retry fail on the queue's unique index. `call` asserts
    // there is no error; the exact no-op is pinned by the in-process tests.
    let first = client.call("retry_document_ocr", json!({ "id": id })).await;
    assert_eq!(first["structuredContent"]["queued"], json!(true), "{first}");
    let second = client.call("retry_document_ocr", json!({ "id": id })).await;
    assert!(
        second["structuredContent"]["queued"].is_boolean(),
        "{second}"
    );
}
