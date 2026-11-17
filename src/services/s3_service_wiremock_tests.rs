//! `S3Service` behaviour against a wiremock endpoint. Included from
//! `s3_service.rs` as `wiremock_tests` (only with the `s3` feature).

use super::S3Service;
use crate::services::s3_listing::mock_s3::*;
use crate::storage::StorageBackend;
use wiremock::{
    matchers::{method, path, path_regex, query_param},
    Mock, MockServer, ResponseTemplate,
};

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
    let err = service
        .discover_files_in_folder("docs")
        .await
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("AccessDenied"),
        "error text should carry the S3 error code, got: {err}"
    );
    assert!(
        err.contains("Access Denied"),
        "error text should carry the S3 message, got: {err}"
    );
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
    Mock::given(method("GET"))
        .and(path("/b/docs/ok.pdf"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"hello".to_vec()))
        .mount(&server)
        .await;

    let service = S3Service::new(config_for(&server)).await.unwrap();
    assert_eq!(
        service.download_file("docs/ok.pdf").await.unwrap(),
        b"hello"
    );
}

#[tokio::test]
async fn download_with_body_shorter_than_content_length_is_an_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/b/docs/short.pdf"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Length", "100")
                .set_body_bytes(b"hello".to_vec()),
        )
        .mount(&server)
        .await;

    let service = S3Service::new(config_for(&server)).await.unwrap();
    let err = service
        .download_file("docs/short.pdf")
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("docs/short.pdf"), "{err}");
    // Every attempt (initial + retries) hit the server; none returned truncated bytes as success.
    assert!(
        server.received_requests().await.unwrap().len() >= 2,
        "download is retried"
    );
}

#[tokio::test]
async fn cancelled_service_makes_no_requests() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"hello".to_vec()))
        .mount(&server)
        .await;

    let token = tokio_util::sync::CancellationToken::new();
    token.cancel();
    let service = S3Service::new(config_for(&server))
        .await
        .unwrap()
        .with_cancellation_token(token);

    assert!(service.discover_files_in_folder("docs").await.is_err());
    assert!(service.download_file("docs/a.pdf").await.is_err());
    assert_eq!(server.received_requests().await.unwrap().len(), 0);
}

#[tokio::test]
async fn file_exists_distinguishes_missing_from_failure() {
    let server = MockServer::start().await;
    Mock::given(method("HEAD"))
        .and(path("/b/missing.pdf"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    Mock::given(method("HEAD"))
        .and(path("/b/present.pdf"))
        .respond_with(ResponseTemplate::new(200).insert_header("Content-Length", "3"))
        .mount(&server)
        .await;
    Mock::given(method("HEAD"))
        .and(path("/b/forbidden.pdf"))
        .respond_with(ResponseTemplate::new(403))
        .mount(&server)
        .await;

    let service = S3Service::new(config_for(&server)).await.unwrap();
    assert!(!StorageBackend::file_exists(&service, "s3://missing.pdf")
        .await
        .unwrap());
    assert!(StorageBackend::file_exists(&service, "s3://present.pdf")
        .await
        .unwrap());
    assert!(StorageBackend::file_exists(&service, "forbidden.pdf")
        .await
        .is_err());
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

    let files = service
        .discover_files_in_folder("/documents")
        .await
        .unwrap();
    assert_eq!(files.len(), 1);
    // The stored path is the full object key, which is what download_file takes.
    assert_eq!(files[0].relative_path, "clients/acme/documents/a.pdf");
    assert_eq!(files[0].full_path, "s3://b/clients/acme/documents/a.pdf");
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

/// A stop request during the wait between two attempts ends the download
/// right away instead of waiting out the backoff.
#[tokio::test]
async fn cancellation_during_a_retry_wait_returns_promptly() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/b/docs/a.pdf"))
        .respond_with(ResponseTemplate::new(403).set_body_raw(ACCESS_DENIED_XML, "application/xml"))
        .mount(&server)
        .await;
    let token = tokio_util::sync::CancellationToken::new();
    let service = S3Service::new(config_for(&server))
        .await
        .unwrap()
        .with_cancellation_token(token.clone())
        .with_retry_base_delay(std::time::Duration::from_secs(60));
    let canceller = token.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        canceller.cancel();
    });

    let started = std::time::Instant::now();
    let err = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        service.download_file("docs/a.pdf"),
    )
    .await
    .expect("the download must not wait out the retry delay")
    .unwrap_err();
    assert!(crate::utils::sync_cancelled::is_cancelled(&err), "{err}");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(1),
        "took {:?}",
        started.elapsed()
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

// ---- Deleting a document's objects ----

const NO_SUCH_KEY_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Error><Code>NoSuchKey</Code><Message>The specified key does not exist.</Message><RequestId>r1</RequestId><HostId>h1</HostId></Error>"#;
const NO_SUCH_BUCKET_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Error><Code>NoSuchBucket</Code><Message>The specified bucket does not exist</Message><RequestId>r1</RequestId><HostId>h1</HostId></Error>"#;

fn user_and_document() -> (uuid::Uuid, uuid::Uuid) {
    (uuid::Uuid::new_v4(), uuid::Uuid::new_v4())
}

/// Paths of every DELETE the server received, sorted.
async fn deleted_paths(server: &MockServer) -> Vec<String> {
    let mut paths: Vec<String> = server
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .filter(|r| r.method.as_str() == "DELETE")
        .map(|r| r.url.path().to_string())
        .collect();
    paths.sort();
    paths
}

fn sorted(mut paths: Vec<String>) -> Vec<String> {
    paths.sort();
    paths
}

async fn mount_delete_ok(server: &MockServer) {
    Mock::given(method("DELETE"))
        .respond_with(ResponseTemplate::new(204))
        .mount(server)
        .await;
}

/// The key of a document uploaded in an earlier month cannot be recomputed
/// from "now"; deletion must use the key recorded at upload.
#[tokio::test]
async fn deleting_a_document_from_a_previous_month_deletes_its_recorded_key() {
    let server = MockServer::start().await;
    mount_delete_ok(&server).await;
    let service = S3Service::new(config_for(&server)).await.unwrap();
    let (user, doc) = user_and_document();
    let stored = format!("s3://documents/{user}/2019/02/{doc}.pdf");

    StorageBackend::delete_document_files(&service, user, doc, &stored)
        .await
        .unwrap();

    assert_eq!(
        deleted_paths(&server).await,
        sorted(vec![
            format!("/b/documents/{user}/2019/02/{doc}.pdf"),
            format!("/b/thumbnails/{user}/{doc}_thumb.jpg"),
            format!("/b/processed_images/{user}/{doc}_processed.png"),
        ])
    );
}

/// A file path that is not an S3 storage path (a local file from before S3
/// storage was enabled) names no object; only derived objects are deleted.
#[tokio::test]
async fn deleting_a_document_with_a_local_path_deletes_no_document_object() {
    let server = MockServer::start().await;
    mount_delete_ok(&server).await;
    let service = S3Service::new(config_for(&server)).await.unwrap();
    let (user, doc) = user_and_document();

    StorageBackend::delete_document_files(&service, user, doc, "./uploads/documents/x.pdf")
        .await
        .unwrap();

    assert_eq!(
        deleted_paths(&server).await,
        sorted(vec![
            format!("/b/thumbnails/{user}/{doc}_thumb.jpg"),
            format!("/b/processed_images/{user}/{doc}_processed.png"),
        ])
    );
}

#[tokio::test]
async fn missing_objects_are_classified_from_the_typed_error() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/b/no-such-key"))
        .respond_with(ResponseTemplate::new(404).set_body_raw(NO_SUCH_KEY_XML, "application/xml"))
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/b/bare-404"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/b/present"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/b/no-such-bucket"))
        .respond_with(
            ResponseTemplate::new(404).set_body_raw(NO_SUCH_BUCKET_XML, "application/xml"),
        )
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/b/denied"))
        .respond_with(ResponseTemplate::new(403).set_body_raw(ACCESS_DENIED_XML, "application/xml"))
        .mount(&server)
        .await;
    let service = S3Service::new(config_for(&server))
        .await
        .unwrap()
        .with_retry_base_delay(std::time::Duration::from_millis(1));

    assert_eq!(
        service.delete_object("no-such-key").await.unwrap(),
        super::ObjectDeletion::AlreadyGone
    );
    assert_eq!(
        service.delete_object("bare-404").await.unwrap(),
        super::ObjectDeletion::AlreadyGone
    );
    assert_eq!(
        service.delete_object("present").await.unwrap(),
        super::ObjectDeletion::Deleted
    );
    // A 404 for the bucket is a misconfiguration, not a deleted object.
    let err = service
        .delete_object("no-such-bucket")
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("NoSuchBucket"), "{err}");
    let err = service
        .delete_object("denied")
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("AccessDenied"), "{err}");
    // delete_file treats an absent key as success.
    service.delete_file("no-such-key").await.unwrap();
}

/// NoSuchKey for every object is "already deleted"; any other failure is
/// reported, and the remaining objects are still deleted.
#[tokio::test]
async fn document_deletion_tolerates_missing_objects_and_reports_other_errors() {
    let server = MockServer::start().await;
    let (user, doc) = user_and_document();
    Mock::given(method("DELETE"))
        .and(path(format!("/b/thumbnails/{user}/{doc}_thumb.jpg")))
        .respond_with(ResponseTemplate::new(403).set_body_raw(ACCESS_DENIED_XML, "application/xml"))
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .respond_with(ResponseTemplate::new(404).set_body_raw(NO_SUCH_KEY_XML, "application/xml"))
        .mount(&server)
        .await;
    let service = S3Service::new(config_for(&server))
        .await
        .unwrap()
        .with_retry_base_delay(std::time::Duration::from_millis(1));
    let stored = format!("s3://documents/{user}/2020/07/{doc}.pdf");

    let err = StorageBackend::delete_document_files(&service, user, doc, &stored)
        .await
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("Thumbnail") && err.contains("AccessDenied"),
        "{err}"
    );
    assert!(
        !err.contains("Document:") && !err.contains("Processed image"),
        "{err}"
    );
    let paths = deleted_paths(&server).await;
    assert!(
        paths.contains(&format!("/b/processed_images/{user}/{doc}_processed.png")),
        "{paths:?}"
    );

    // With only NoSuchKey answers, deletion succeeds.
    let (user2, doc2) = user_and_document();
    StorageBackend::delete_document_files(
        &service,
        user2,
        doc2,
        &format!("s3://documents/{user2}/2020/07/{doc2}.pdf"),
    )
    .await
    .unwrap();
}

// ---- S3_PREFIX ----

fn prefixed_service_config(server: &MockServer) -> crate::models::S3SourceConfig {
    let mut cfg = config_for(server);
    cfg.prefix = Some("/tenant/a/".to_string());
    cfg
}

#[tokio::test]
async fn prefix_is_applied_to_every_written_key_and_reads_use_the_recorded_key() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"bytes".to_vec()))
        .mount(&server)
        .await;
    let service = S3Service::new(prefixed_service_config(&server))
        .await
        .unwrap();
    let (user, doc) = user_and_document();

    let stored = StorageBackend::store_document(&service, user, doc, "scan.pdf", b"pdf")
        .await
        .unwrap();
    let thumb = StorageBackend::store_thumbnail(&service, user, doc, b"jpg")
        .await
        .unwrap();
    let processed = StorageBackend::store_processed_image(&service, user, doc, b"png")
        .await
        .unwrap();
    assert!(
        stored.starts_with(&format!("s3://tenant/a/documents/{user}/")),
        "{stored}"
    );
    assert!(stored.ends_with(&format!("/{doc}.pdf")), "{stored}");
    assert_eq!(
        thumb,
        format!("s3://tenant/a/thumbnails/{user}/{doc}_thumb.jpg")
    );
    assert_eq!(
        processed,
        format!("s3://tenant/a/processed_images/{user}/{doc}_processed.png")
    );

    let puts: Vec<String> = server
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .filter(|r| r.method.as_str() == "PUT")
        .map(|r| r.url.path().to_string())
        .collect();
    assert_eq!(puts.len(), 3);
    assert!(
        puts.iter().all(|p| p.starts_with("/b/tenant/a/")),
        "{puts:?}"
    );

    // A document stored with the prefix and one stored before it was set are
    // both read by their recorded key.
    StorageBackend::retrieve_file(&service, &stored)
        .await
        .unwrap();
    StorageBackend::retrieve_file(
        &service,
        &format!("s3://documents/{user}/2019/02/{doc}.pdf"),
    )
    .await
    .unwrap();
    let gets: Vec<String> = server
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .filter(|r| r.method.as_str() == "GET")
        .map(|r| r.url.path().to_string())
        .collect();
    assert_eq!(
        gets,
        vec![
            format!("/b/{}", stored.trim_start_matches("s3://")),
            format!("/b/documents/{user}/2019/02/{doc}.pdf")
        ]
    );
}

/// Turning the prefix on must not strand objects written without it.
#[tokio::test]
async fn prefix_deletion_covers_prefixed_and_legacy_unprefixed_objects() {
    let server = MockServer::start().await;
    mount_delete_ok(&server).await;
    let service = S3Service::new(prefixed_service_config(&server))
        .await
        .unwrap();
    let (user, doc) = user_and_document();

    // Stored before S3_PREFIX was set: the recorded key has no prefix.
    StorageBackend::delete_document_files(
        &service,
        user,
        doc,
        &format!("s3://documents/{user}/2019/02/{doc}.pdf"),
    )
    .await
    .unwrap();
    assert_eq!(
        deleted_paths(&server).await,
        sorted(vec![
            format!("/b/documents/{user}/2019/02/{doc}.pdf"),
            format!("/b/thumbnails/{user}/{doc}_thumb.jpg"),
            format!("/b/tenant/a/thumbnails/{user}/{doc}_thumb.jpg"),
            format!("/b/processed_images/{user}/{doc}_processed.png"),
            format!("/b/tenant/a/processed_images/{user}/{doc}_processed.png"),
        ])
    );

    // Stored with the prefix: the recorded, prefixed key is deleted.
    server.reset().await;
    mount_delete_ok(&server).await;
    let (user, doc) = user_and_document();
    StorageBackend::delete_document_files(
        &service,
        user,
        doc,
        &format!("s3://tenant/a/documents/{user}/2024/05/{doc}.pdf"),
    )
    .await
    .unwrap();
    assert!(deleted_paths(&server)
        .await
        .contains(&format!("/b/tenant/a/documents/{user}/2024/05/{doc}.pdf")));
}

/// With an S3 backend, FileService deletes the recorded object and also the
/// thumbnail / OCR review images cached on local disk.
#[tokio::test]
async fn file_service_with_s3_backend_deletes_bucket_objects_and_local_caches() {
    let server = MockServer::start().await;
    mount_delete_ok(&server).await;
    let upload = tempfile::tempdir().unwrap();
    let service = std::sync::Arc::new(S3Service::new(config_for(&server)).await.unwrap());
    let file_service = crate::services::file_service::FileService::with_storage(
        upload.path().to_string_lossy().to_string(),
        service,
    );
    let (user, doc) = user_and_document();
    let thumbs = upload.path().join("thumbnails");
    let processed = upload.path().join("processed_images");
    std::fs::create_dir_all(&thumbs).unwrap();
    std::fs::create_dir_all(&processed).unwrap();
    let thumb = thumbs.join(format!("{doc}_thumb.jpg"));
    let review = processed.join(format!("{doc}_processed_1700000000.png"));
    let unrelated = processed.join(format!("{}_processed_1700000000.png", uuid::Uuid::new_v4()));
    for f in [&thumb, &review, &unrelated] {
        std::fs::write(f, b"x").unwrap();
    }

    let mut document = file_service.create_document_with_id(
        doc,
        "scan.pdf",
        "scan.pdf",
        "",
        3,
        "application/pdf",
        user,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    );
    document.file_path = format!("s3://documents/{user}/2018/11/{doc}.pdf");
    file_service.delete_document_files(&document).await.unwrap();

    assert!(deleted_paths(&server)
        .await
        .contains(&format!("/b/documents/{user}/2018/11/{doc}.pdf")));
    assert!(
        !thumb.exists() && !review.exists(),
        "local caches of the document are removed"
    );
    assert!(
        unrelated.exists(),
        "another document's review image is untouched"
    );
}
