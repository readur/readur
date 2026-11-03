//! `S3Service` behaviour against a wiremock endpoint. Included from
//! `s3_service.rs` as `wiremock_tests` (only with the `s3` feature).

use super::S3Service;
use crate::services::s3_listing::mock_s3::*;
use crate::storage::StorageBackend;
use wiremock::{matchers::{method, path, path_regex, query_param}, Mock, MockServer, ResponseTemplate};

const ACCESS_DENIED_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Error><Code>AccessDenied</Code><Message>Access Denied</Message><RequestId>r1</RequestId><HostId>h1</HostId></Error>"#;

#[tokio::test]
async fn listing_error_reports_the_service_error_code() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex("^/b/?$"))
        .respond_with(ResponseTemplate::new(403).set_body_raw(ACCESS_DENIED_XML, "application/xml"))
        .mount(&server)
        .await;

    let service = S3Service::new(config_for(&server)).await.unwrap();
    let err = service.discover_files_in_folder("docs").await.unwrap_err().to_string();
    assert!(err.contains("AccessDenied"), "error text should carry the S3 error code, got: {err}");
    assert!(err.contains("Access Denied"), "error text should carry the S3 message, got: {err}");
}

#[tokio::test]
async fn test_connection_error_is_categorised_as_authentication() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex("^/b/?$"))
        .respond_with(ResponseTemplate::new(403).set_body_raw(ACCESS_DENIED_XML, "application/xml"))
        .mount(&server)
        .await;

    let service = S3Service::new(config_for(&server)).await.unwrap();
    let err = service.test_connection().await.unwrap_err().to_string();
    assert_eq!(
        crate::utils::outbound::categorize_connection_error(&err),
        "Authentication failed - check the credentials"
    );
}

#[tokio::test]
async fn download_returns_body_and_checks_length() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/b/docs/ok.pdf"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"hello".to_vec()))
        .mount(&server)
        .await;

    let service = S3Service::new(config_for(&server)).await.unwrap();
    assert_eq!(service.download_file("docs/ok.pdf").await.unwrap(), b"hello");
}

#[tokio::test]
async fn download_with_body_shorter_than_content_length_is_an_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/b/docs/short.pdf"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Length", "100")
                .set_body_bytes(b"hello".to_vec()),
        )
        .mount(&server)
        .await;

    let service = S3Service::new(config_for(&server)).await.unwrap();
    let err = service.download_file("docs/short.pdf").await.unwrap_err().to_string();
    assert!(err.contains("docs/short.pdf"), "{err}");
    // Every attempt (initial + retries) hit the server; none returned truncated bytes as success.
    assert!(server.received_requests().await.unwrap().len() >= 2, "download is retried");
}

#[tokio::test]
async fn file_exists_distinguishes_missing_from_failure() {
    let server = MockServer::start().await;
    Mock::given(method("HEAD")).and(path("/b/missing.pdf"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    Mock::given(method("HEAD")).and(path("/b/present.pdf"))
        .respond_with(ResponseTemplate::new(200).insert_header("Content-Length", "3"))
        .mount(&server)
        .await;
    Mock::given(method("HEAD")).and(path("/b/forbidden.pdf"))
        .respond_with(ResponseTemplate::new(403))
        .mount(&server)
        .await;

    let service = S3Service::new(config_for(&server)).await.unwrap();
    assert!(!StorageBackend::file_exists(&service, "s3://missing.pdf").await.unwrap());
    assert!(StorageBackend::file_exists(&service, "s3://present.pdf").await.unwrap());
    assert!(StorageBackend::file_exists(&service, "forbidden.pdf").await.is_err());
}

#[tokio::test]
async fn discovery_lists_under_joined_prefix() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex("^/b/?$"))
        .and(query_param("list-type", "2"))
        .and(query_param("prefix", "clients/acme/documents/"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            list_page(&["clients/acme/documents/a.pdf"], None, false),
            "application/xml",
        ))
        .mount(&server)
        .await;

    let mut cfg = config_for(&server);
    cfg.prefix = Some("clients/acme".to_string());
    let service = S3Service::new(cfg).await.unwrap();

    let files = service.discover_files_in_folder("/documents").await.unwrap();
    assert_eq!(files.len(), 1);
    // The stored path is the full object key, which is what download_file takes.
    assert_eq!(files[0].relative_path, "clients/acme/documents/a.pdf");
    assert_eq!(files[0].full_path, "s3://b/clients/acme/documents/a.pdf");
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}
