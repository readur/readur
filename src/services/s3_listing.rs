//! `ListObjectsV2` pagination for S3 sources.
//!
//! The listing loop lives here, separate from [`crate::services::s3_service::S3Service`],
//! so it can be exercised against a mock endpoint without the rest of the
//! service and so the pagination rules are in one place:
//!
//! * a page with `IsTruncated=true` and a `NextContinuationToken` is followed;
//! * a page with `IsTruncated=true` and **no** token is an error (re-requesting
//!   the first page would loop forever and grow memory);
//! * anything else ends the listing.

use anyhow::{anyhow, Result};

#[cfg(feature = "s3")]
use crate::models::FileIngestionInfo;
#[cfg(feature = "s3")]
use crate::services::s3_client::describe_error;
#[cfg(feature = "s3")]
use crate::utils::extension_filter::ExtensionFilter;
#[cfg(feature = "s3")]
use aws_sdk_s3::Client;
#[cfg(feature = "s3")]
use chrono::DateTime;
#[cfg(feature = "s3")]
use tokio_util::sync::CancellationToken;
#[cfg(feature = "s3")]
use tracing::debug;

/// Upper bound on pages followed in a single listing. At the default 1000
/// keys per page this allows 100 million objects under one prefix; beyond
/// that the listing is treated as runaway and aborted.
pub const MAX_LISTING_PAGES: usize = 100_000;

/// Build the key prefix to list for a watch folder.
///
/// The source-level `prefix` (if any) is joined with `folder`; leading
/// slashes are dropped (S3 keys have none), empty and duplicate separators
/// are collapsed, and a non-empty result always ends in `/` so that
/// `documents` does not also match `documents-archive/...`. An empty result
/// lists the whole bucket.
pub fn listing_prefix(prefix: Option<&str>, folder: &str) -> String {
    let segments: Vec<&str> = prefix
        .unwrap_or("")
        .split('/')
        .chain(folder.split('/'))
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .collect();
    if segments.is_empty() {
        String::new()
    } else {
        format!("{}/", segments.join("/"))
    }
}

/// Decide how to proceed after a `ListObjectsV2` page.
///
/// Returns `Ok(Some(token))` to request the next page, `Ok(None)` when the
/// listing is complete, and an error when the page claims to be truncated but
/// carries no continuation token.
pub fn next_continuation(is_truncated: Option<bool>, next_token: Option<String>) -> Result<Option<String>> {
    match (is_truncated, next_token) {
        (Some(true), Some(token)) => Ok(Some(token)),
        (Some(true), None) => Err(anyhow!("truncated listing without continuation token")),
        _ => Ok(None),
    }
}

/// Everything a listing needs besides the prefix.
#[cfg(feature = "s3")]
pub(crate) struct ListingContext<'a> {
    pub client: &'a Client,
    pub bucket: &'a str,
    pub region: &'a str,
    pub filter: &'a ExtensionFilter,
    pub cancellation: Option<&'a CancellationToken>,
    pub max_pages: usize,
}

/// List every object under `prefix`, following continuation tokens.
#[cfg(feature = "s3")]
pub(crate) async fn list_objects(ctx: &ListingContext<'_>, prefix: &str) -> Result<Vec<FileIngestionInfo>> {
    let mut files = Vec::new();
    let mut continuation_token: Option<String> = None;
    let mut pages = 0usize;

    loop {
        if ctx.cancellation.is_some_and(|token| token.is_cancelled()) {
            return Err(anyhow!("Listing of S3 prefix '{}' cancelled after {} pages", prefix, pages));
        }
        if pages >= ctx.max_pages {
            return Err(anyhow!(
                "Listing of S3 prefix '{}' exceeded {} pages; aborting",
                prefix, ctx.max_pages
            ));
        }
        pages += 1;

        let mut request = ctx.client.list_objects_v2().bucket(ctx.bucket).prefix(prefix);
        if let Some(token) = &continuation_token {
            request = request.continuation_token(token);
        }

        let response = request
            .send()
            .await
            .map_err(|e| anyhow!("Failed to list S3 objects: {}", describe_error(&e)))?;

        for object in response.contents.unwrap_or_default() {
            if let Some(info) = object_to_file_info(object, ctx) {
                files.push(info);
            }
        }

        match next_continuation(response.is_truncated, response.next_continuation_token)? {
            Some(token) => continuation_token = Some(token),
            None => break,
        }
    }

    Ok(files)
}

/// Convert one listed object into a [`FileIngestionInfo`], or `None` when it
/// is a directory marker or its extension is not configured.
#[cfg(feature = "s3")]
fn object_to_file_info(object: aws_sdk_s3::types::Object, ctx: &ListingContext<'_>) -> Option<FileIngestionInfo> {
    let key = object.key?;
    // Skip "directories" (keys ending with /)
    if key.ends_with('/') {
        return None;
    }

    let extension = std::path::Path::new(&key)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !ctx.filter.matches_extension(&extension) {
        debug!("Skipping S3 object with unsupported extension: {}", key);
        return None;
    }

    let file_name = std::path::Path::new(&key)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&key)
        .to_string();

    let size = object.size.unwrap_or(0);
    let last_modified = object
        .last_modified
        .and_then(|dt| DateTime::from_timestamp(dt.secs(), 0));

    // Fallback ETag when none is provided; quotes are stripped either way.
    let etag = object
        .e_tag
        .unwrap_or_else(|| format!("fallback-{}", key.chars().take(16).collect::<String>()))
        .trim_matches('"')
        .to_string();

    let mime_type = crate::services::s3_service::S3Service::get_mime_type(&extension);

    let mut metadata_map = serde_json::Map::new();
    if let Some(storage_class) = &object.storage_class {
        metadata_map.insert("storage_class".to_string(), serde_json::Value::String(storage_class.as_str().to_string()));
    }
    if let Some(owner) = &object.owner {
        if let Some(display_name) = &owner.display_name {
            metadata_map.insert("owner_display_name".to_string(), serde_json::Value::String(display_name.clone()));
        }
        if let Some(id) = &owner.id {
            metadata_map.insert("owner_id".to_string(), serde_json::Value::String(id.clone()));
        }
    }
    metadata_map.insert("s3_key".to_string(), serde_json::Value::String(key.clone()));
    metadata_map.insert("s3_bucket".to_string(), serde_json::Value::String(ctx.bucket.to_string()));
    metadata_map.insert("s3_region".to_string(), serde_json::Value::String(ctx.region.to_string()));

    Some(FileIngestionInfo {
        relative_path: key.clone(),
        full_path: format!("s3://{}/{}", ctx.bucket, key),
        #[allow(deprecated)]
        path: key.clone(),
        name: file_name,
        size,
        mime_type,
        last_modified,
        etag,
        is_directory: false,
        created_at: None,  // S3 only exposes last-modified
        permissions: None, // S3 uses ACLs/policies, not mode bits
        owner: object.owner.as_ref().and_then(|o| o.display_name.clone()),
        group: None,
        metadata: Some(serde_json::Value::Object(metadata_map)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listing_prefix_joins_and_normalises() {
        assert_eq!(listing_prefix(None, "documents"), "documents/");
        assert_eq!(listing_prefix(None, "/documents/"), "documents/");
        assert_eq!(listing_prefix(Some("clients/acme"), "documents"), "clients/acme/documents/");
        assert_eq!(listing_prefix(Some("/clients//acme/"), "/documents"), "clients/acme/documents/");
        assert_eq!(listing_prefix(Some("clients/acme"), ""), "clients/acme/");
        assert_eq!(listing_prefix(Some("  "), " docs "), "docs/");
    }

    #[test]
    fn listing_prefix_empty_means_whole_bucket() {
        assert_eq!(listing_prefix(None, ""), "");
        assert_eq!(listing_prefix(Some(""), "/"), "");
        assert_eq!(listing_prefix(Some("//"), "  "), "");
    }

    #[test]
    fn continuation_rules() {
        assert_eq!(next_continuation(Some(true), Some("t".into())).unwrap(), Some("t".to_string()));
        assert_eq!(next_continuation(Some(false), Some("t".into())).unwrap(), None);
        assert_eq!(next_continuation(None, None).unwrap(), None);
        let err = next_continuation(Some(true), None).unwrap_err();
        assert!(err.to_string().contains("continuation token"), "{err}");
    }
}

#[cfg(all(test, feature = "s3"))]
pub(crate) mod mock_s3 {
    //! Shared helpers for tests against a wiremock S3 endpoint.
    use crate::models::S3SourceConfig;
    use wiremock::{matchers::{method, path_regex, query_param, query_param_is_missing}, Mock, MockServer, ResponseTemplate};

    pub fn config_for(server: &MockServer) -> S3SourceConfig {
        S3SourceConfig {
            bucket_name: "b".to_string(),
            region: "us-east-1".to_string(),
            access_key_id: "k".to_string(),
            secret_access_key: "s".to_string(),
            endpoint_url: Some(format!("http://localhost:{}", server.address().port())),
            force_path_style: Some(true),
            prefix: None,
            watch_folders: vec!["docs/".to_string()],
            file_extensions: vec!["pdf".to_string()],
            auto_sync: false,
            sync_interval_minutes: 0,
        }
    }

    pub fn list_page(keys: &[&str], next_token: Option<&str>, truncated: bool) -> String {
        let contents: String = keys
            .iter()
            .map(|k| format!(
                "<Contents><Key>{k}</Key><LastModified>2024-01-01T00:00:00.000Z</LastModified><ETag>\"etag-{k}\"</ETag><Size>10</Size><StorageClass>STANDARD</StorageClass></Contents>"
            ))
            .collect();
        let token = next_token
            .map(|t| format!("<NextContinuationToken>{t}</NextContinuationToken>"))
            .unwrap_or_default();
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?><ListBucketResult xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\"><Name>b</Name><KeyCount>{}</KeyCount><MaxKeys>1000</MaxKeys><IsTruncated>{truncated}</IsTruncated>{token}{contents}</ListBucketResult>",
            keys.len()
        )
    }

    /// Mount a first page (request carries no continuation token).
    pub async fn mount_first_page(server: &MockServer, body: String) {
        Mock::given(method("GET"))
            .and(path_regex("^/b/?$"))
            .and(query_param("list-type", "2"))
            .and(query_param_is_missing("continuation-token"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(body, "application/xml"))
            .mount(server)
            .await;
    }

    /// Mount the page served for a given continuation token.
    pub async fn mount_page_for_token(server: &MockServer, token: &str, body: String) {
        Mock::given(method("GET"))
            .and(path_regex("^/b/?$"))
            .and(query_param("continuation-token", token))
            .respond_with(ResponseTemplate::new(200).set_body_raw(body, "application/xml"))
            .mount(server)
            .await;
    }
}

#[cfg(all(test, feature = "s3"))]
mod listing_tests {
    use super::mock_s3::*;
    use super::*;
    use wiremock::MockServer;

    async fn list_with(server: &MockServer, max_pages: usize) -> Result<Vec<FileIngestionInfo>> {
        list_with_token(server, max_pages, None, &["pdf"]).await
    }

    async fn list_with_token(
        server: &MockServer,
        max_pages: usize,
        cancellation: Option<&CancellationToken>,
        extensions: &[&str],
    ) -> Result<Vec<FileIngestionInfo>> {
        let cfg = config_for(server);
        let client = crate::services::s3_client::build_client(&cfg).await;
        let filter = ExtensionFilter::new(extensions);
        let ctx = ListingContext {
            client: &client,
            bucket: &cfg.bucket_name,
            region: &cfg.region,
            filter: &filter,
            cancellation,
            max_pages,
        };
        list_objects(&ctx, "docs/").await
    }

    async fn listed_keys(server: &MockServer, extensions: &[&str]) -> Vec<String> {
        list_with_token(server, MAX_LISTING_PAGES, None, extensions)
            .await
            .unwrap()
            .into_iter()
            .map(|f| f.relative_path)
            .collect()
    }

    #[tokio::test]
    async fn extension_filter_is_case_and_dot_insensitive_and_empty_allows_all() {
        let server = MockServer::start().await;
        mount_first_page(&server, list_page(&["docs/a.pdf", "docs/B.PDF", "docs/c.zip", "docs/README"], None, false)).await;

        assert_eq!(listed_keys(&server, &[".PDF"]).await, vec!["docs/a.pdf", "docs/B.PDF"]);
        assert_eq!(listed_keys(&server, &["Zip"]).await, vec!["docs/c.zip"]);
        assert_eq!(
            listed_keys(&server, &[]).await,
            vec!["docs/a.pdf", "docs/B.PDF", "docs/c.zip", "docs/README"],
            "an empty list allows every object"
        );
    }

    #[tokio::test]
    async fn cancelled_token_stops_listing_before_any_request() {
        let server = MockServer::start().await;
        mount_first_page(&server, list_page(&["docs/a.pdf"], None, false)).await;

        let token = CancellationToken::new();
        token.cancel();
        let err = list_with_token(&server, MAX_LISTING_PAGES, Some(&token), &["pdf"]).await.unwrap_err();
        assert!(err.to_string().contains("cancelled"), "{err}");
        assert_eq!(server.received_requests().await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn follows_continuation_tokens_across_pages() {
        let server = MockServer::start().await;
        mount_first_page(&server, list_page(&["docs/a.pdf"], Some("t1"), true)).await;
        mount_page_for_token(&server, "t1", list_page(&["docs/b.pdf", "docs/skip.zip"], None, false)).await;

        let files = list_with(&server, MAX_LISTING_PAGES).await.unwrap();
        let keys: Vec<&str> = files.iter().map(|f| f.relative_path.as_str()).collect();
        assert_eq!(keys, vec!["docs/a.pdf", "docs/b.pdf"]);
        assert_eq!(files[0].etag, "etag-docs/a.pdf");
        assert_eq!(server.received_requests().await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn truncated_page_without_token_is_an_error() {
        let server = MockServer::start().await;
        mount_first_page(&server, list_page(&["docs/a.pdf"], None, true)).await;

        let err = list_with(&server, MAX_LISTING_PAGES).await.unwrap_err();
        assert!(err.to_string().contains("continuation token"), "{err}");
        assert_eq!(server.received_requests().await.unwrap().len(), 1, "page 1 must not be re-listed");
    }

    #[tokio::test]
    async fn page_limit_aborts_runaway_listing() {
        let server = MockServer::start().await;
        mount_first_page(&server, list_page(&["docs/a.pdf"], Some("t1"), true)).await;
        mount_page_for_token(&server, "t1", list_page(&["docs/b.pdf"], None, false)).await;

        let err = list_with(&server, 1).await.unwrap_err();
        assert!(err.to_string().contains("exceeded 1 pages"), "{err}");
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }
}
