//! `S3Service` behaviour against a wiremock endpoint. Included from
//! `s3_service.rs` as `wiremock_tests` (only with the `s3` feature).

use super::S3Service;
use crate::services::s3_listing::mock_s3::*;
use wiremock::{matchers::{method, path_regex, query_param}, Mock, MockServer, ResponseTemplate};

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
