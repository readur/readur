//! A cancelled token stops discovery between directory batches and stops
//! downloads before the request, without further requests to the server.

use crate::services::webdav::{ConcurrencyConfig, RetryConfig, WebDAVConfig, WebDAVService};
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

fn service_for(server: &MockServer, token: CancellationToken) -> WebDAVService {
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
    // One directory per batch so cancellation is observed between subdirectories.
    let concurrency = ConcurrencyConfig { max_concurrent_scans: 1, max_concurrent_downloads: 1, adaptive_rate_limiting: false };
    WebDAVService::new_with_configs(config, retry, concurrency)
        .expect("service")
        .with_cancellation_token(token)
}

fn entry(href: &str, is_dir: bool) -> String {
    let resourcetype = if is_dir { "<d:resourcetype><d:collection/></d:resourcetype>" } else { "<d:resourcetype/>" };
    format!(
        r#"<d:response><d:href>{href}</d:href><d:propstat><d:prop>
            <d:getetag>"e"</d:getetag>{resourcetype}
        </d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response>"#
    )
}

fn multistatus(entries: &[String]) -> String {
    format!(r#"<?xml version="1.0"?><d:multistatus xmlns:d="DAV:">{}</d:multistatus>"#, entries.join(""))
}

#[tokio::test]
async fn cancelled_before_discovery_makes_no_requests() {
    let server = MockServer::start().await;
    Mock::given(method("PROPFIND")).respond_with(ResponseTemplate::new(207)).expect(0).mount(&server).await;

    let token = CancellationToken::new();
    token.cancel();
    let service = service_for(&server, token);

    let started = Instant::now();
    let err = service.discover_tree("/Documents", true, None).await.unwrap_err();
    assert!(err.to_string().contains("cancelled"), "{err}");
    assert!(started.elapsed() < Duration::from_secs(1));

    let err = service.discover_files_and_directories("/Documents", false).await.unwrap_err();
    assert!(err.to_string().contains("cancelled"), "{err}");
}

#[tokio::test]
async fn cancelled_before_download_makes_no_requests() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).respond_with(ResponseTemplate::new(200)).expect(0).mount(&server).await;

    let token = CancellationToken::new();
    token.cancel();
    let service = service_for(&server, token);

    let err = service.download_file("/Documents/a.pdf").await.unwrap_err();
    assert!(err.to_string().contains("cancelled"), "{err}");
}

#[tokio::test]
async fn cancellation_during_recursive_discovery_stops_between_batches() {
    let server = MockServer::start().await;
    Mock::given(method("PROPFIND")).and(path("/Documents"))
        .respond_with(ResponseTemplate::new(207).set_body_raw(multistatus(&[
            entry("/Documents/", true),
            entry("/Documents/A/", true),
            entry("/Documents/B/", true),
            entry("/Documents/C/", true),
        ]), "application/xml"))
        .mount(&server).await;
    for sub in ["/Documents/A/", "/Documents/B/", "/Documents/C/"] {
        Mock::given(method("PROPFIND")).and(path(sub))
            .respond_with(ResponseTemplate::new(207)
                .set_delay(Duration::from_millis(300))
                .set_body_raw(multistatus(&[entry(sub, true)]), "application/xml"))
            .mount(&server).await;
    }

    let token = CancellationToken::new();
    let service = service_for(&server, token.clone());
    let canceller = token.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        canceller.cancel();
    });

    let started = Instant::now();
    let err = service.discover_tree("/Documents", true, None).await.unwrap_err();
    assert!(err.to_string().contains("cancelled"), "{err}");
    // Root plus at most the one in-flight subdirectory, not the whole tree.
    assert!(started.elapsed() < Duration::from_millis(900), "took {:?}", started.elapsed());
    let received = server.received_requests().await.unwrap_or_default().len();
    assert!(received <= 2, "expected the scan to stop after the in-flight batch, saw {received} requests");
}
