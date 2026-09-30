//! HTTP-level hardening: CORS policy and default security response headers.

use axum::http::{header, HeaderName, HeaderValue, Method};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::set_header::SetResponseHeaderLayer;

use crate::config::Config;

/// Policy applied to every response that doesn't set its own. Tuned for the
/// bundled SPA: MUI injects inline styles, fonts come from Google Fonts, and
/// documents are previewed via `blob:` iframes/images.
const DEFAULT_CSP: &str = "default-src 'self'; \
    script-src 'self'; \
    style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; \
    font-src 'self' data: https://fonts.gstatic.com; \
    img-src 'self' data: blob:; \
    connect-src 'self' ws: wss:; \
    frame-src 'self' blob:; \
    worker-src 'self' blob:; \
    base-uri 'self'; \
    form-action 'self'; \
    frame-ancestors 'self'";

/// Restrictive policy for user-uploaded content: no scripts and an opaque
/// origin, even if a browser decides to render it as a document.
const USER_CONTENT_CSP: &str =
    "sandbox; default-src 'none'; img-src 'self' data: blob:; media-src 'self'; style-src 'unsafe-inline'";

/// Response headers for serving a user-uploaded file.
///
/// Active content types (HTML, SVG, XML, scripts, unknown) are always sent
/// as attachments. Everything except PDF additionally gets a sandboxing CSP
/// (Chromium's built-in PDF viewer refuses to run in sandboxed documents;
/// PDFs are covered by `nosniff` plus the fixed content type).
pub fn user_content_headers(
    mime_type: &str,
    original_filename: &str,
    prefer_inline: bool,
) -> Vec<(HeaderName, HeaderValue)> {
    let filename = crate::utils::security::content_disposition_filename(original_filename);
    let active = crate::utils::security::is_active_content_mime(mime_type);
    let disposition = if prefer_inline && !active { "inline" } else { "attachment" };

    let content_type = HeaderValue::from_str(mime_type)
        .ok()
        .filter(|_| !mime_type.trim().is_empty())
        .unwrap_or_else(|| HeaderValue::from_static("application/octet-stream"));

    let mut headers = vec![
        (header::CONTENT_TYPE, content_type),
        (
            header::CONTENT_DISPOSITION,
            HeaderValue::from_str(&format!("{}; filename=\"{}\"", disposition, filename))
                .unwrap_or_else(|_| HeaderValue::from_static("attachment")),
        ),
        (header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff")),
    ];

    let is_pdf = is_pdf_media_type(mime_type);
    if !is_pdf {
        headers.push((header::CONTENT_SECURITY_POLICY, HeaderValue::from_static(USER_CONTENT_CSP)));
    }
    headers
}

/// True when the essence of `mime_type` (ignoring parameters) is exactly
/// `application/pdf`.
fn is_pdf_media_type(mime_type: &str) -> bool {
    mime_type
        .split(';')
        .next()
        .map(|essence| essence.trim().eq_ignore_ascii_case("application/pdf"))
        .unwrap_or(false)
}

/// A 200 response carrying a user-uploaded file with [`user_content_headers`].
pub fn user_content_response(
    mime_type: &str,
    original_filename: &str,
    prefer_inline: bool,
    data: Vec<u8>,
) -> Result<axum::response::Response, axum::http::Error> {
    let mut builder = axum::response::Response::builder().status(axum::http::StatusCode::OK);
    for (name, value) in user_content_headers(mime_type, original_filename, prefer_inline) {
        builder = builder.header(name, value);
    }
    builder
        .header(header::CONTENT_LENGTH, data.len())
        .body(axum::body::Body::from(data))
}

/// Cross-origin requests are denied unless origins are listed in
/// `CORS_ALLOWED_ORIGINS`. The bundled frontend is same-origin and needs none.
/// Origins are normalized and validated when the configuration is loaded.
pub fn cors_layer(config: &Config) -> CorsLayer {
    let origins: Vec<HeaderValue> = config
        .security
        .cors_allowed_origins
        .iter()
        .filter_map(|o| HeaderValue::from_str(o).ok())
        .collect();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE, header::ACCEPT])
}

/// Default security headers. Each is only added if the handler did not set
/// the header itself.
pub fn security_header_layers(config: &Config) -> Vec<SetResponseHeaderLayer<HeaderValue>> {
    let mut layers = vec![
        SetResponseHeaderLayer::if_not_present(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ),
        SetResponseHeaderLayer::if_not_present(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static("SAMEORIGIN"),
        ),
        SetResponseHeaderLayer::if_not_present(
            header::REFERRER_POLICY,
            HeaderValue::from_static("no-referrer"),
        ),
        SetResponseHeaderLayer::if_not_present(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(DEFAULT_CSP),
        ),
        SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("cross-origin-opener-policy"),
            HeaderValue::from_static("same-origin"),
        ),
    ];

    let https = config
        .public_url
        .as_deref()
        .map(|u| u.starts_with("https://"))
        .unwrap_or(false);
    if https {
        layers.push(SetResponseHeaderLayer::if_not_present(
            header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=31536000"),
        ));
    }

    layers
}

#[cfg(test)]
mod tests {
    use super::*;

    fn has_csp(mime: &str) -> bool {
        user_content_headers(mime, "file", true)
            .iter()
            .any(|(name, _)| name == header::CONTENT_SECURITY_POLICY)
    }

    #[test]
    fn only_exact_pdf_media_type_skips_the_sandbox() {
        assert!(!has_csp("application/pdf"));
        assert!(!has_csp("Application/PDF; charset=binary"));
        assert!(has_csp("application/pdfx"));
        assert!(has_csp("application/pdf+xml"));
        assert!(has_csp("text/html"));
        assert!(has_csp(""));
    }
}
