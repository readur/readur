//! WebDAV directory ETags are recorded only after the files a scan found have
//! been processed. A folder whose file could not be downloaded or ingested,
//! or whose subdirectory could not be listed, keeps its previous state (or
//! none), so the next run lists it again; a stopped run records nothing.

use anyhow::Result;
use readur::models::{CreateSource, CreateWebDAVDirectory, SourceType};
use readur::scheduling::source_sync::{SourceSyncService, SyncOutcome};
use readur::services::webdav::SyncProgress;
use readur::test_utils::TestContext;
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};


fn entry(href: &str, etag: &str, is_dir: bool) -> String {
    let resourcetype = if is_dir { "<d:resourcetype><d:collection/></d:resourcetype>" } else { "<d:resourcetype/>" };
    format!(
        r#"<d:response><d:href>{href}</d:href><d:propstat><d:prop>
            <d:getlastmodified>Tue, 29 Jul 2025 01:34:17 GMT</d:getlastmodified>
            <d:getetag>"{etag}"</d:getetag>{resourcetype}<d:getcontentlength>10</d:getcontentlength>
            <d:getcontenttype>text/plain</d:getcontenttype>
        </d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response>"#
    )
}

fn multistatus(entries: &[String]) -> String {
    format!(r#"<?xml version="1.0"?><d:multistatus xmlns:d="DAV:">{}</d:multistatus>"#, entries.join(""))
}

/// Mounts a PROPFIND listing under both spellings a scan may request.
async fn listing(server: &MockServer, dir: &str, response: ResponseTemplate) {
    for at in [dir.trim_end_matches('/').to_string(), format!("{}/", dir.trim_end_matches('/'))] {
        Mock::given(method("PROPFIND")).and(path(at)).respond_with(response.clone()).mount(server).await;
    }
}

async fn download(server: &MockServer, file: &str, response: ResponseTemplate) {
    Mock::given(method("GET")).and(path(file)).respond_with(response).mount(server).await;
}

fn ok_listing(entries: &[String]) -> ResponseTemplate {
    ResponseTemplate::new(207).set_body_string(multistatus(entries))
}

/// A 10-byte body (the listed getcontentlength) that differs per file so the
/// two files are not deduplicated by content.
fn ok_body(file: &str) -> ResponseTemplate {
    let body = format!("{:<10}", file.rsplit('/').next().unwrap_or(file));
    ResponseTemplate::new(200).set_body_raw(body.into_bytes(), "text/plain")
}

/// `/Docs` with `/Docs/A/x.txt` and `/Docs/Other/y.txt`; `/Docs/A` may hold a nested `/Docs/A/B`.
async fn mount_tree(server: &MockServer, nested_b: Option<ResponseTemplate>) {
    let mut a_entries = vec![entry("/Docs/A/", "a-v1", true), entry("/Docs/A/x.txt", "f-x", false)];
    if nested_b.is_some() {
        a_entries.push(entry("/Docs/A/B/", "b-v1", true));
    }
    listing(server, "/Docs", ok_listing(&[
        entry("/Docs/", "root-v1", true),
        entry("/Docs/A/", "a-v1", true),
        entry("/Docs/Other/", "other-v1", true),
    ])).await;
    listing(server, "/Docs/A", ok_listing(&a_entries)).await;
    listing(server, "/Docs/Other", ok_listing(&[
        entry("/Docs/Other/", "other-v1", true),
        entry("/Docs/Other/y.txt", "f-y", false),
    ])).await;
    if let Some(b) = nested_b {
        listing(server, "/Docs/A/B", b).await;
    }
}

async fn webdav_source(ctx: &TestContext, user_id: Uuid, server: &MockServer) -> readur::models::Source {
    ctx.state()
        .db
        .create_source(
            user_id,
            &CreateSource {
                name: "deferred-commit".into(),
                source_type: SourceType::WebDAV,
                enabled: Some(true),
                config: json!({
                    "server_url": server.uri(),
                    "username": "u",
                    "password": "p",
                    "watch_folders": ["/Docs"],
                    "file_extensions": ["txt"],
                    "auto_sync": false,
                    "sync_interval_minutes": 60,
                    "server_type": "generic"
                }),
            },
        )
        .await
        .unwrap()
}

/// Stored directory rows keyed by path without trailing slash.
async fn stored_rows(ctx: &TestContext, user_id: Uuid) -> BTreeMap<String, String> {
    ctx.state()
        .db
        .list_webdav_directories(user_id)
        .await
        .unwrap()
        .into_iter()
        .map(|d| (d.directory_path.trim_end_matches('/').to_string(), d.directory_etag))
        .collect()
}

async fn propfind_count(server: &MockServer, dir: &str) -> usize {
    let dir = dir.trim_end_matches('/');
    server
        .received_requests()
        .await
        .unwrap_or_default()
        .iter()
        .filter(|r| r.method.as_str() == "PROPFIND" && r.url.path().trim_end_matches('/') == dir)
        .count()
}

async fn sync(ctx: &TestContext, source: &readur::models::Source) -> SyncOutcome {
    SourceSyncService::new(ctx.state().clone()).sync_source(source, false).await.unwrap()
}

/// (i) A download failure under `/Docs/A` leaves `/Docs/A` and `/Docs`
/// unrecorded while the sibling is recorded; the next run lists `/Docs/A`
/// again and ingests the file.
#[tokio::test]
async fn download_failure_keeps_the_directory_and_its_ancestors_eligible() {
    let ctx = TestContext::new().await;
    let server = MockServer::start().await;
    let admin = ctx.auth_helper().create_admin_user().await;
    let user_id = admin.user_response.id;
    let source = webdav_source(&ctx, user_id, &server).await;

    mount_tree(&server, None).await;
    download(&server, "/Docs/A/x.txt", ResponseTemplate::new(500)).await;
    download(&server, "/Docs/Other/y.txt", ok_body("y.txt")).await;

    let first = sync(&ctx, &source).await;
    assert_eq!(first.failed, 1, "{first:?}");
    assert_eq!(first.processed, 1, "{first:?}");

    let rows = stored_rows(&ctx, user_id).await;
    assert_eq!(rows.get("/Docs/Other").map(String::as_str), Some("other-v1"), "{rows:?}");
    assert!(!rows.contains_key("/Docs/A"), "directory of the failed file must not be recorded: {rows:?}");
    assert!(!rows.contains_key("/Docs"), "ancestor of the failed file must not be recorded: {rows:?}");

    server.reset().await;
    mount_tree(&server, None).await;
    download(&server, "/Docs/A/x.txt", ok_body("x.txt")).await;
    download(&server, "/Docs/Other/y.txt", ok_body("y.txt")).await;

    let second = sync(&ctx, &source).await;
    assert_eq!(second.failed, 0, "{second:?}");
    assert_eq!(second.processed, 1, "the failed file is ingested on the next run: {second:?}");
    assert!(propfind_count(&server, "/Docs/A").await >= 1, "/Docs/A must be listed again");

    let rows = stored_rows(&ctx, user_id).await;
    assert_eq!(rows.get("/Docs").map(String::as_str), Some("root-v1"), "{rows:?}");
    assert_eq!(rows.get("/Docs/A").map(String::as_str), Some("a-v1"), "{rows:?}");

    let _ = ctx.cleanup_and_close().await;
}

/// (ii) A nested listing failure at `/Docs/A/B` leaves `/Docs/A` (and
/// `/Docs`) unrecorded, so the shallow check reaches `/Docs/A/B` next time.
#[tokio::test]
async fn nested_listing_failure_keeps_the_parent_unrecorded() {
    let ctx = TestContext::new().await;
    let server = MockServer::start().await;
    let admin = ctx.auth_helper().create_admin_user().await;
    let user_id = admin.user_response.id;
    let source = webdav_source(&ctx, user_id, &server).await;

    mount_tree(&server, Some(ResponseTemplate::new(500))).await;
    download(&server, "/Docs/A/x.txt", ok_body("x.txt")).await;
    download(&server, "/Docs/Other/y.txt", ok_body("y.txt")).await;

    let outcome = sync(&ctx, &source).await;
    assert_eq!(outcome.directories_failed, 1, "{outcome:?}");
    assert_eq!(outcome.processed, 2, "{outcome:?}");

    let rows = stored_rows(&ctx, user_id).await;
    assert_eq!(rows.get("/Docs/Other").map(String::as_str), Some("other-v1"), "{rows:?}");
    assert!(!rows.contains_key("/Docs/A/B"), "{rows:?}");
    assert!(!rows.contains_key("/Docs/A"), "parent of the failed directory must not be recorded: {rows:?}");
    assert!(!rows.contains_key("/Docs"), "{rows:?}");

    let _ = ctx.cleanup_and_close().await;
}

/// (iii) A clean run records every directory and prunes rows the listing no
/// longer contains.
#[tokio::test]
async fn clean_run_records_all_directories_and_prunes_stale_rows() {
    let ctx = TestContext::new().await;
    let server = MockServer::start().await;
    let admin = ctx.auth_helper().create_admin_user().await;
    let user_id = admin.user_response.id;
    let source = webdav_source(&ctx, user_id, &server).await;
    ctx.state()
        .db
        .create_or_update_webdav_directory(&CreateWebDAVDirectory {
            user_id,
            directory_path: "/Docs/Gone/".into(),
            directory_etag: "stale".into(),
            file_count: 0,
            total_size_bytes: 0,
        })
        .await
        .unwrap();

    mount_tree(&server, None).await;
    download(&server, "/Docs/A/x.txt", ok_body("x.txt")).await;
    download(&server, "/Docs/Other/y.txt", ok_body("y.txt")).await;

    let outcome = sync(&ctx, &source).await;
    assert_eq!(outcome.processed, 2, "{outcome:?}");
    assert!(outcome.is_clean(), "{outcome:?}");

    let rows = stored_rows(&ctx, user_id).await;
    let paths: Vec<&str> = rows.keys().map(String::as_str).collect();
    assert_eq!(paths, vec!["/Docs", "/Docs/A", "/Docs/Other"], "{rows:?}");

    let _ = ctx.cleanup_and_close().await;
}

/// (iv) A run stopped while its files were still downloading records nothing.
#[tokio::test]
async fn cancelled_run_records_no_directories() {
    let ctx = TestContext::new().await;
    let server = MockServer::start().await;
    let admin = ctx.auth_helper().create_admin_user().await;
    let user_id = admin.user_response.id;
    let source = webdav_source(&ctx, user_id, &server).await;

    mount_tree(&server, None).await;
    download(&server, "/Docs/A/x.txt", ok_body("x.txt").set_delay(Duration::from_secs(2))).await;
    download(&server, "/Docs/Other/y.txt", ok_body("y.txt").set_delay(Duration::from_secs(2))).await;

    let token = CancellationToken::new();
    let canceller = token.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;
        canceller.cancel();
    });
    assert!(ctx.state().db.start_sync_atomic(source.id).await.unwrap());
    let outcome = SourceSyncService::new(ctx.state().clone())
        .sync_claimed_source(&source, false, token, &Arc::new(SyncProgress::new()))
        .await
        .unwrap();
    assert!(outcome.cancelled, "{outcome:?}");

    let rows = stored_rows(&ctx, user_id).await;
    assert!(rows.is_empty(), "a cancelled run must record nothing: {rows:?}");

    let _ = ctx.cleanup_and_close().await;
}
