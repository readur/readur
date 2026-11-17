//! Handlers answer with a success status their OpenAPI annotation declares.
//!
//! Each case is a basic happy-path request; its status must be 2xx and one of
//! the 2xx statuses the OpenAPI document lists for that operation. Covers
//! every create endpoint (which answer 201 Created) plus the reads, updates
//! and deletes around them, so an annotation and its handler cannot drift
//! apart again (labels and sources used to answer 200 while declaring 201).

use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use readur::test_utils::{document_helpers, test_password, TestContext};
use serde_json::{json, Value};
use tower::ServiceExt;
use utoipa::OpenApi;
use uuid::Uuid;

/// The 2xx statuses the OpenAPI document declares for `method path`.
fn declared_success(method: &str, path: &str) -> Vec<u16> {
    let api = readur::swagger::ApiDoc::openapi();
    let item = api
        .paths
        .paths
        .get(path)
        .unwrap_or_else(|| panic!("{path} is not in the OpenAPI document"));
    let operation = match method {
        "GET" => &item.get,
        "POST" => &item.post,
        "PUT" => &item.put,
        "DELETE" => &item.delete,
        "PATCH" => &item.patch,
        _ => panic!("unexpected method {method}"),
    }
    .as_ref()
    .unwrap_or_else(|| panic!("{method} {path} is not in the OpenAPI document"));
    operation
        .responses
        .responses
        .keys()
        .filter_map(|code| code.parse::<u16>().ok())
        .filter(|code| (200..300).contains(code))
        .collect()
}

struct Client {
    app: Router,
    token: String,
}

impl Client {
    async fn send(&self, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
        let mut request = Request::builder()
            .method(method)
            .uri(uri)
            .header("authorization", format!("Bearer {}", self.token));
        let body = match body {
            Some(json) => {
                request = request.header("content-type", "application/json");
                Body::from(json.to_string())
            }
            None => Body::empty(),
        };
        let response = self
            .app
            .clone()
            .oneshot(request.body(body).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    /// Sends a happy-path request for the documented operation `method
    /// template` and checks its status against the annotation. Returns the
    /// body.
    async fn expect_declared(
        &self,
        failures: &mut Vec<String>,
        method: &str,
        template: &str,
        uri: &str,
        body: Option<Value>,
    ) -> Value {
        let (status, response) = self.send(method, uri, body).await;
        let declared = declared_success(method, template);
        if !status.is_success() || !declared.contains(&status.as_u16()) {
            failures.push(format!(
                "{method} {template}: answered {status}, OpenAPI declares {declared:?} ({response})"
            ));
        }
        response
    }
}

#[tokio::test]
async fn happy_paths_answer_a_declared_success_status() {
    let ctx = TestContext::new().await;
    let auth = ctx.auth_helper();
    let admin = auth.create_admin_user().await;
    let client = Client {
        app: ctx.app().clone(),
        token: auth.login_user(&admin.username, &admin.password).await,
    };
    let folder = tempfile::tempdir().unwrap();
    let mut failures = Vec::new();
    let f = &mut failures;

    // Labels
    let label = client
        .expect_declared(
            f,
            "POST",
            "/api/labels",
            "/api/labels",
            Some(json!({ "name": "status-contract" })),
        )
        .await;
    let label_id = label["id"].as_str().unwrap_or_default().to_string();
    client
        .expect_declared(f, "GET", "/api/labels", "/api/labels", None)
        .await;
    client
        .expect_declared(
            f,
            "GET",
            "/api/labels/{id}",
            &format!("/api/labels/{label_id}"),
            None,
        )
        .await;
    client
        .expect_declared(
            f,
            "PUT",
            "/api/labels/{id}",
            &format!("/api/labels/{label_id}"),
            Some(json!({ "name": "status-contract-renamed" })),
        )
        .await;

    // Labels on a document: first assignment creates, the second is a no-op.
    let mut document = document_helpers::create_test_document(admin.user_response.id);
    document.file_hash = Some(Uuid::new_v4().simple().to_string());
    let document = ctx.state().db.create_document(document).await.unwrap();
    let assign = format!("/api/labels/documents/{}/labels/{label_id}", document.id);
    let assign_template = "/api/labels/documents/{document_id}/labels/{label_id}";
    let (first, _) = client.send("POST", &assign, None).await;
    assert_eq!(first, StatusCode::CREATED, "first assignment");
    client
        .expect_declared(f, "POST", assign_template, &assign, None)
        .await;
    client
        .expect_declared(f, "DELETE", assign_template, &assign, None)
        .await;
    client
        .expect_declared(
            f,
            "DELETE",
            "/api/labels/{id}",
            &format!("/api/labels/{label_id}"),
            None,
        )
        .await;

    // Sources
    let source = client
        .expect_declared(
            f,
            "POST",
            "/api/sources",
            "/api/sources",
            Some(json!({
                "name": "status-contract",
                "source_type": "local_folder",
                "enabled": false,
                "config": {
                    "watch_folders": [folder.path().to_str().unwrap()],
                    "file_extensions": ["pdf"],
                    "auto_sync": false,
                    "sync_interval_minutes": 60,
                    "recursive": false,
                    "follow_symlinks": false
                }
            })),
        )
        .await;
    let source_id = source["id"].as_str().unwrap_or_default().to_string();
    client
        .expect_declared(f, "GET", "/api/sources", "/api/sources", None)
        .await;
    client
        .expect_declared(
            f,
            "GET",
            "/api/sources/{id}",
            &format!("/api/sources/{source_id}"),
            None,
        )
        .await;
    client
        .expect_declared(
            f,
            "DELETE",
            "/api/sources/{id}",
            &format!("/api/sources/{source_id}"),
            None,
        )
        .await;

    // Users
    let suffix = Uuid::new_v4().simple().to_string();
    let user = client
        .expect_declared(
            f,
            "POST",
            "/api/users",
            "/api/users",
            Some(json!({
                "username": format!("contract_{}", &suffix[..12]),
                "email": format!("contract_{}@example.com", &suffix[..12]),
                "password": test_password(),
            })),
        )
        .await;
    let user_id = user["id"].as_str().unwrap_or_default().to_string();
    client
        .expect_declared(f, "GET", "/api/users", "/api/users", None)
        .await;
    client
        .expect_declared(
            f,
            "GET",
            "/api/users/{id}",
            &format!("/api/users/{user_id}"),
            None,
        )
        .await;
    client
        .expect_declared(
            f,
            "DELETE",
            "/api/users/{id}",
            &format!("/api/users/{user_id}"),
            None,
        )
        .await;

    let _ = ctx.cleanup_and_close().await;
    assert!(
        failures.is_empty(),
        "Status does not match the OpenAPI annotation:\n{}",
        failures.join("\n")
    );
}

/// Every DELETE of a single resource answers 404 for an id that does not
/// exist (the API never treats deleting nothing as success), and declares it.
#[tokio::test]
async fn deleting_a_missing_resource_is_404() {
    let ctx = TestContext::new().await;
    let auth = ctx.auth_helper();
    let admin = auth.create_admin_user().await;
    let client = Client {
        app: ctx.app().clone(),
        token: auth.login_user(&admin.username, &admin.password).await,
    };

    let documented = [
        "/api/documents/{id}",
        "/api/labels/{id}",
        "/api/sources/{id}",
        "/api/users/{id}",
        "/api/ignored/files/{id}",
    ];
    let undocumented = ["/api/auth/keys/{id}", "/api/shared/links/{id}"];
    let mut failures = Vec::new();
    for template in documented.iter().chain(undocumented.iter()) {
        let uri = template.replace("{id}", &Uuid::new_v4().to_string());
        let (status, body) = client.send("DELETE", &uri, None).await;
        if status != StatusCode::NOT_FOUND {
            failures.push(format!("DELETE {template} -> {status} {body}"));
        }
        if documented.contains(template) {
            let api = readur::swagger::ApiDoc::openapi();
            let declares_404 = api.paths.paths[*template]
                .delete
                .as_ref()
                .is_some_and(|op| op.responses.responses.contains_key("404"));
            if !declares_404 {
                failures.push(format!("DELETE {template} does not declare 404"));
            }
        }
    }

    let _ = ctx.cleanup_and_close().await;
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The per-user watch-directory endpoints answer 501, as declared, while
/// the feature is switched off.
#[tokio::test]
async fn watch_directory_endpoints_are_501_when_disabled() {
    let ctx = TestContext::new().await;
    assert!(
        !ctx.state().config.enable_per_user_watch,
        "the test config disables per-user watch"
    );
    let auth = ctx.auth_helper();
    let admin = auth.create_admin_user().await;
    let client = Client {
        app: ctx.app().clone(),
        token: auth.login_user(&admin.username, &admin.password).await,
    };
    let uri = format!("/api/users/{}/watch/directory", admin.user_response.id);

    for (method, body) in [("GET", None), ("POST", Some(json!({}))), ("DELETE", None)] {
        let (status, body) = client.send(method, &uri, body).await;
        assert_eq!(status, StatusCode::NOT_IMPLEMENTED, "{method} {body}");
        assert_eq!(body["code"], "USER_WATCH_DISABLED", "{method} {body}");
        assert_eq!(body["status"], 501, "{method} {body}");
        assert_eq!(
            body["error"], "Per-user watch directories are not enabled on this server",
            "{method}"
        );
        let api = readur::swagger::ApiDoc::openapi();
        let item = &api.paths.paths["/api/users/{id}/watch/directory"];
        let op = match method {
            "GET" => &item.get,
            "POST" => &item.post,
            _ => &item.delete,
        };
        assert!(
            op.as_ref().unwrap().responses.responses.contains_key("501"),
            "{method}"
        );
    }

    let _ = ctx.cleanup_and_close().await;
}

/// The create endpoints the OpenAPI document leaves out answer 201 Created
/// too, like every documented one.
#[tokio::test]
async fn undocumented_create_endpoints_answer_201() {
    let ctx = TestContext::new().await;
    let auth = ctx.auth_helper();
    let user = auth.create_test_user().await;
    let client = Client {
        app: ctx.app().clone(),
        token: auth.login_user(&user.username, &user.password).await,
    };
    let mut document = document_helpers::create_test_document(user.user_response.id);
    document.file_hash = Some(Uuid::new_v4().simple().to_string());
    let document = ctx.state().db.create_document(document).await.unwrap();

    let cases = [
        (
            "/api/auth/keys".to_string(),
            json!({ "name": "status-contract" }),
        ),
        (
            "/api/shared/links".to_string(),
            json!({ "document_id": document.id }),
        ),
        (
            format!("/api/comments/documents/{}/comments", document.id),
            json!({ "content": "status contract" }),
        ),
    ];
    for (uri, body) in cases {
        let (status, response) = client.send("POST", &uri, Some(body)).await;
        assert_eq!(status, StatusCode::CREATED, "POST {uri}: {response}");
    }

    let _ = ctx.cleanup_and_close().await;
}
