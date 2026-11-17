//! Every error response of every API route has the one documented shape,
//! `readur::errors::ApiErrorBody`: `{"error": "...", "code": "...", "status": N}`
//! with `status` equal to the HTTP status.
//!
//! Default-deny style: the routes come from the OpenAPI document plus the
//! hand-maintained undocumented ones (tests/support/api_routes.rs), and each
//! is probed without credentials, with invalid credentials, and as a signed-in
//! user and admin sending a body that is not JSON. Any 4xx/5xx answer whose
//! body does not parse as the shared shape fails the test, so a new route
//! cannot opt out.

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    Router,
};
use readur::errors::ApiErrorBody;
use readur::test_utils::TestContext;
use tower::ServiceExt;
use utoipa::OpenApi;

#[path = "support/api_routes.rs"]
mod api_routes;
use api_routes::{all_routes, documented_routes};

/// Requests that would end the session the sweep is using.
const SESSION_ENDING: &[(&str, &str)] =
    &[("POST", "/api/auth/logout"), ("POST", "/api/auth/password")];

/// Replace `{param}` segments with a random UUID, or a dummy token.
fn concrete_path(template: &str) -> String {
    template
        .split('/')
        .map(|segment| {
            if segment.starts_with('{') && segment.ends_with('}') {
                if segment.contains("token") {
                    "not-a-real-token".to_string()
                } else {
                    uuid::Uuid::new_v4().to_string()
                }
            } else {
                segment.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// Sends the request; returns a failure description when it is an error
/// response without the shared shape, and `None` otherwise.
async fn check(
    app: &Router,
    method: &str,
    uri: &str,
    auth: Option<&str>,
    body: &'static str,
) -> Option<String> {
    let mut request = Request::builder()
        .method(Method::from_bytes(method.as_bytes()).unwrap())
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(auth) = auth {
        request = request.header("authorization", auth);
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::from(body)).unwrap())
        .await
        .unwrap();
    let status = response.status();
    if !(status.is_client_error() || status.is_server_error()) {
        return None;
    }
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let raw = String::from_utf8_lossy(&bytes);
    match serde_json::from_slice::<ApiErrorBody>(&bytes) {
        Ok(parsed)
            if parsed.status == status.as_u16()
                && !parsed.code.is_empty()
                && !parsed.error.is_empty() =>
        {
            if content_type.starts_with("application/json") {
                None
            } else {
                Some(format!(
                    "{method} {uri} -> {status}: content-type {content_type:?}"
                ))
            }
        }
        _ => Some(format!("{method} {uri} -> {status}: {raw}")),
    }
}

#[tokio::test]
async fn every_error_response_has_the_shared_shape() {
    let ctx = TestContext::new().await;
    let app = ctx.app().clone();
    let auth = ctx.auth_helper();
    let user = auth.create_test_user().await;
    let user_token = auth.login_user(&user.username, &user.password).await;
    let admin = auth.create_admin_user().await;
    let admin_token = auth.login_user(&admin.username, &admin.password).await;
    let user_auth = format!("Bearer {user_token}");
    let admin_auth = format!("Bearer {admin_token}");

    let mut failures = Vec::new();
    let mut errors_checked = 0;
    for (method, template) in all_routes() {
        let uri = concrete_path(&template);
        let mut probes: Vec<(Option<&str>, &'static str)> = vec![
            (None, "{}"),
            (Some("Bearer not-a-jwt"), "{}"),
            (Some("Bearer readur_pat_notarealkey"), "{}"),
        ];
        if !SESSION_ENDING.contains(&(method.as_str(), template.as_str())) {
            probes.push((Some(user_auth.as_str()), "this is not json"));
            probes.push((Some(admin_auth.as_str()), "this is not json"));
            probes.push((Some(admin_auth.as_str()), "{}"));
        }
        for (header, body) in probes {
            if let Some(failure) = check(&app, &method, &uri, header, body).await {
                failures.push(failure);
            }
            errors_checked += 1;
        }
    }

    let _ = ctx.cleanup_and_close().await;
    assert!(
        errors_checked > 600,
        "only {errors_checked} requests were checked"
    );
    assert!(
        failures.is_empty(),
        "Error responses without the shared {{error, code, status}} shape:\n{}",
        failures.join("\n")
    );
}

/// Specific errors keep their specific code; generic ones get the status's.
#[tokio::test]
async fn error_codes_are_specific_where_the_handler_knows_more() {
    let ctx = TestContext::new().await;
    let app = ctx.app().clone();
    let auth = ctx.auth_helper();
    let admin = auth.create_admin_user().await;
    let token = auth.login_user(&admin.username, &admin.password).await;

    let cases = [
        // UserError
        (
            "GET",
            format!("/api/users/{}", uuid::Uuid::new_v4()),
            Some(&token),
            StatusCode::NOT_FOUND,
            "USER_NOT_FOUND_BY_ID",
        ),
        // The authentication extractor
        (
            "GET",
            "/api/auth/me".to_string(),
            None,
            StatusCode::UNAUTHORIZED,
            "UNAUTHORIZED",
        ),
        // A handler that answers with a bare status code
        (
            "GET",
            format!("/api/labels/{}", uuid::Uuid::new_v4()),
            Some(&token),
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
        ),
    ];
    for (method, uri, token, status, code) in cases {
        let mut request = Request::builder().method(method).uri(&uri);
        if let Some(token) = token {
            request = request.header("authorization", format!("Bearer {token}"));
        }
        let response = app
            .clone()
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), status, "{method} {uri}");
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: ApiErrorBody = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body.code, code, "{method} {uri}");
        assert_eq!(body.status, status.as_u16(), "{method} {uri}");
    }

    let _ = ctx.cleanup_and_close().await;
}

/// Each OpenAPI server URL joined with each documented path reaches a
/// mounted route, so Swagger UI's "Try it out" calls real endpoints. A TRACE
/// request is answered 405 by a mounted path (no route uses TRACE) and 404
/// by a path the router does not know.
#[tokio::test]
async fn openapi_server_urls_resolve_to_mounted_routes() {
    let ctx = TestContext::new().await;
    let app = ctx.app().clone();
    let api = readur::swagger::ApiDoc::openapi();
    let servers: Vec<String> = api
        .servers
        .as_ref()
        .map(|servers| servers.iter().map(|s| s.url.clone()).collect())
        .unwrap_or_else(|| vec!["/".to_string()]);
    assert!(!servers.is_empty());

    let mut unresolved = Vec::new();
    for server in &servers {
        assert!(
            server.starts_with('/'),
            "server URL {server:?} should be relative to the Readur host"
        );
        for (method, template) in documented_routes() {
            let uri = format!(
                "{}{}",
                server.trim_end_matches('/'),
                concrete_path(&template)
            );
            let probe = Request::builder()
                .method("TRACE")
                .uri(&uri)
                .body(Body::empty())
                .unwrap();
            let status = app.clone().oneshot(probe).await.unwrap().status();
            if status == StatusCode::NOT_FOUND {
                unresolved.push(format!(
                    "{method} {template} via server {server:?} -> {uri}"
                ));
            }
        }
    }

    let _ = ctx.cleanup_and_close().await;
    assert!(
        unresolved.is_empty(),
        "OpenAPI server + path does not reach a mounted route:\n{}",
        unresolved.join("\n")
    );
}

/// The shared shape is the documented schema for errors in the OpenAPI
/// document.
#[test]
fn error_body_schema_is_published() {
    let api = readur::swagger::ApiDoc::openapi();
    let schemas = &api.components.as_ref().unwrap().schemas;
    assert!(
        schemas.contains_key("ApiErrorBody"),
        "ApiErrorBody is not in the OpenAPI components"
    );
}
