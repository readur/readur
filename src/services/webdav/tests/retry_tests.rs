//! 429/503 responses are retried a bounded number of times, honouring Retry-After.

use crate::services::webdav::{RetryConfig, WebDAVConfig, WebDAVService};
use reqwest::Method;
use std::time::{Duration, Instant};
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

fn service_for(server: &MockServer, max_retries: u32) -> WebDAVService {
    let config = WebDAVConfig {
        server_url: server.uri(),
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        watch_folders: vec!["/Documents".to_string()],
        file_extensions: vec![],
        timeout_seconds: 5,
        server_type: Some("generic".to_string()),
    };
    let retry = RetryConfig {
        max_retries,
        initial_delay_ms: 1,
        max_delay_ms: 5,
        backoff_multiplier: 1.0,
        timeout_seconds: 5,
        rate_limit_backoff_ms: 1,
    };
    WebDAVService::new_with_retry(config, retry).expect("service")
}

async fn get(service: &WebDAVService, server: &MockServer) -> anyhow::Result<reqwest::Response> {
    service.authenticated_request(Method::GET, &format!("{}/Documents/x.txt", server.uri()), None, None).await
}

#[tokio::test]
async fn rate_limited_then_ok_succeeds() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/Documents/x.txt"))
        .respond_with(ResponseTemplate::new(429))
        .up_to_n_times(2)
        .expect(2)
        .mount(&server).await;
    Mock::given(method("GET")).and(path("/Documents/x.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_raw("ok", "text/plain"))
        .expect(1)
        .mount(&server).await;

    let service = service_for(&server, 3);
    let response = get(&service, &server).await.expect("request should succeed after retries");
    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn persistent_rate_limit_fails_after_max_retries() {
    let server = MockServer::start().await;
    // max_retries = 2 means one initial request plus two retries.
    Mock::given(method("GET")).and(path("/Documents/x.txt"))
        .respond_with(ResponseTemplate::new(429))
        .expect(3)
        .mount(&server).await;

    let service = service_for(&server, 2);
    let err = tokio::time::timeout(Duration::from_secs(10), get(&service, &server))
        .await
        .expect("must not loop forever")
        .unwrap_err();
    assert!(err.to_string().contains("429"), "{err}");
}

#[tokio::test]
async fn retry_after_header_is_honoured() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/Documents/x.txt"))
        .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "1"))
        .up_to_n_times(1)
        .mount(&server).await;
    Mock::given(method("GET")).and(path("/Documents/x.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_raw("ok", "text/plain"))
        .mount(&server).await;

    let service = service_for(&server, 3);
    let started = Instant::now();
    get(&service, &server).await.expect("request should succeed");
    assert!(started.elapsed() >= Duration::from_millis(900), "Retry-After was not honoured: {:?}", started.elapsed());
}

#[tokio::test]
async fn service_unavailable_is_retried_with_a_bound() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/Documents/x.txt"))
        .respond_with(ResponseTemplate::new(503).insert_header("Retry-After", "0"))
        .expect(2)
        .mount(&server).await;

    let service = service_for(&server, 1);
    let err = tokio::time::timeout(Duration::from_secs(10), get(&service, &server))
        .await
        .expect("must not loop forever")
        .unwrap_err();
    assert!(err.to_string().contains("503"), "{err}");
}
