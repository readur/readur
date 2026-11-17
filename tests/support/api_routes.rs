//! The complete list of API routes, shared by the tests that need it: every operation of the
//! OpenAPI document plus the mounted routes that have no OpenAPI annotation.
//!
//! Included with `#[path = "support/api_routes.rs"] mod api_routes;`. The default-deny test
//! (tests/integration_default_deny_tests.rs) probes every route listed here, so a route listed
//! here that the router does not serve fails that test.

use utoipa::OpenApi;

/// Mounted routes without an OpenAPI annotation.
pub const UNDOCUMENTED_ROUTES: &[(&str, &str)] = &[
    ("GET", "/api/mcp"),
    ("POST", "/api/mcp"),
    ("DELETE", "/api/mcp"),
    ("POST", "/api/auth/keys"),
    ("GET", "/api/auth/keys"),
    ("DELETE", "/api/auth/keys/{id}"),
    ("GET", "/api/documents/ocr/stats"),
    ("POST", "/api/documents/{id}/ocr/stop"),
    ("GET", "/api/documents/{id}/validate"),
    ("GET", "/api/comments/documents/{document_id}/comments"),
    ("POST", "/api/comments/documents/{document_id}/comments"),
    (
        "GET",
        "/api/comments/documents/{document_id}/comments/{comment_id}/replies",
    ),
    (
        "PUT",
        "/api/comments/documents/{document_id}/comments/{comment_id}",
    ),
    (
        "DELETE",
        "/api/comments/documents/{document_id}/comments/{comment_id}",
    ),
    (
        "GET",
        "/api/comments/documents/{document_id}/comments/count",
    ),
    ("POST", "/api/shared/links"),
    ("GET", "/api/shared/links"),
    ("GET", "/api/shared/links/document/{document_id}"),
    ("DELETE", "/api/shared/links/{id}"),
    ("GET", "/api/public/shared/{token}"),
    ("POST", "/api/public/shared/{token}/verify"),
    ("POST", "/api/public/shared/{token}/download"),
    ("POST", "/api/public/shared/{token}/view"),
];

/// `(METHOD, path template)` of every operation in the OpenAPI document.
pub fn documented_routes() -> Vec<(String, String)> {
    let api = readur::swagger::ApiDoc::openapi();
    let mut routes = Vec::new();
    for (path, item) in api.paths.paths.iter() {
        let ops = [
            ("GET", item.get.is_some()),
            ("POST", item.post.is_some()),
            ("PUT", item.put.is_some()),
            ("DELETE", item.delete.is_some()),
            ("PATCH", item.patch.is_some()),
        ];
        for (method, present) in ops {
            if present {
                routes.push((method.to_string(), path.clone()));
            }
        }
    }
    routes
}

/// Every route, sorted and without duplicates.
pub fn all_routes() -> Vec<(String, String)> {
    let mut routes = documented_routes();
    routes.extend(
        UNDOCUMENTED_ROUTES
            .iter()
            .map(|(m, p)| (m.to_string(), p.to_string())),
    );
    routes.sort();
    routes.dedup();
    routes
}
