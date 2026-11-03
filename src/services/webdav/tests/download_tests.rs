//! Download integrity: the body must be complete and must be file content, not
//! an HTML page served in place of it.

use crate::models::FileIngestionInfo;
use crate::services::webdav::{RetryConfig, WebDAVConfig, WebDAVService};
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

fn service_for(server: &MockServer) -> WebDAVService {
    let config = WebDAVConfig {
        server_url: server.uri(),
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        watch_folders: vec!["/Documents".to_string()],
        file_extensions: vec![],
        timeout_seconds: 5,
        server_type: Some("generic".to_string()),
    };
    let retry = RetryConfig { max_retries: 0, initial_delay_ms: 1, max_delay_ms: 1, rate_limit_backoff_ms: 1, ..RetryConfig::default() };
    WebDAVService::new_with_retry(config, retry).expect("service")
}

fn file_info(relative_path: &str, name: &str) -> FileIngestionInfo {
    FileIngestionInfo {
        relative_path: relative_path.to_string(),
        full_path: relative_path.to_string(),
        #[allow(deprecated)]
        path: relative_path.to_string(),
        name: name.to_string(),
        size: 0,
        mime_type: "application/pdf".to_string(),
        last_modified: None,
        etag: "e".to_string(),
        is_directory: false,
        created_at: None,
        permissions: None,
        owner: None,
        group: None,
        metadata: None,
    }
}

async fn serve(server: &MockServer, at: &str, response: ResponseTemplate) {
    Mock::given(method("GET")).and(path(at)).respond_with(response).mount(server).await;
}

#[tokio::test]
async fn complete_body_is_returned() {
    let server = MockServer::start().await;
    serve(&server, "/Documents/report.pdf",
          ResponseTemplate::new(200).set_body_raw(b"%PDF-1.4 content".to_vec(), "application/pdf")).await;
    let service = service_for(&server);

    let bytes = service.download_file("/Documents/report.pdf").await.expect("download");
    assert_eq!(bytes, b"%PDF-1.4 content");

    let bytes = service.download_file_info(&file_info("/Documents/report.pdf", "report.pdf")).await.expect("download");
    assert_eq!(bytes, b"%PDF-1.4 content");

    let result = service.download_file_with_mime_detection(&file_info("/Documents/report.pdf", "report.pdf")).await.expect("download");
    assert_eq!(result.content, b"%PDF-1.4 content");
}

#[tokio::test]
async fn html_page_in_place_of_pdf_is_rejected() {
    let server = MockServer::start().await;
    serve(&server, "/Documents/report.pdf",
          ResponseTemplate::new(200)
              .set_body_raw("<!DOCTYPE html><html><body>Please sign in</body></html>", "text/html; charset=utf-8")).await;
    let service = service_for(&server);

    let err = service.download_file("/Documents/report.pdf").await.unwrap_err();
    assert!(err.to_string().contains("HTML page"), "{err}");
    let err = service.download_file_with_mime_detection(&file_info("/Documents/report.pdf", "report.pdf")).await.unwrap_err();
    assert!(err.to_string().contains("HTML page"), "{err}");
}

#[tokio::test]
async fn untyped_html_body_for_pdf_is_rejected_by_sniffing() {
    let server = MockServer::start().await;
    serve(&server, "/Documents/report.pdf",
          ResponseTemplate::new(200)
              .set_body_raw("\n<html><head><title>Login</title></head></html>", "application/octet-stream")).await;
    let service = service_for(&server);

    let err = service.download_file("/Documents/report.pdf").await.unwrap_err();
    assert!(err.to_string().contains("HTML page"), "{err}");
}

#[tokio::test]
async fn html_file_may_be_html() {
    let server = MockServer::start().await;
    serve(&server, "/Documents/page.html",
          ResponseTemplate::new(200).set_body_raw("<html></html>", "text/html")).await;
    let service = service_for(&server);
    assert_eq!(service.download_file("/Documents/page.html").await.expect("download"), b"<html></html>");
}

#[tokio::test]
async fn truncated_body_is_rejected() {
    // The server declares more bytes than it sends; the transfer must not be
    // accepted as a complete file whichever way the client surfaces it.
    let server = MockServer::start().await;
    serve(&server, "/Documents/report.pdf",
          ResponseTemplate::new(200).insert_header("Content-Length", "4096")
              .set_body_raw(b"%PDF".to_vec(), "application/pdf")).await;
    let service = service_for(&server);

    let result = tokio::time::timeout(std::time::Duration::from_secs(20), service.download_file("/Documents/report.pdf")).await;
    match result {
        Ok(Ok(bytes)) => panic!("truncated body accepted: {} bytes", bytes.len()),
        Ok(Err(_)) => {}
        Err(_) => panic!("download of a truncated body hung"),
    }
}
