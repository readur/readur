//! Construction of the S3 SDK client for a source: addressing-style
//! selection and outbound destination filtering.
//!
//! Every client uses [`FilteringResolver`] for DNS, so each host the SDK
//! connects to (the configured endpoint, or `<bucket>.<endpoint>` with
//! virtual-hosted addressing) is checked when the connection is made, not
//! only when the configuration was validated.

use crate::models::S3SourceConfig;

#[cfg(feature = "s3")]
use crate::utils::outbound::FilteringResolver;
#[cfg(feature = "s3")]
use aws_credential_types::Credentials;
#[cfg(feature = "s3")]
use aws_sdk_s3::Client;
#[cfg(feature = "s3")]
use aws_types::region::Region as AwsRegion;
#[cfg(feature = "s3")]
use tracing::{debug, info, warn};

fn custom_endpoint(config: &S3SourceConfig) -> Option<&str> {
    config.endpoint_url.as_deref().map(str::trim).filter(|u| !u.is_empty())
}

/// Which S3 addressing styles to try, in priority order.
/// true = path-style (http://endpoint/bucket/key), false = virtual-hosted
/// (http://bucket.endpoint/key).
pub(crate) fn addressing_styles_to_try(config: &S3SourceConfig) -> Vec<bool> {
    match config.force_path_style {
        Some(style) => vec![style],
        // Custom endpoints are S3-compatible services (MinIO, RustFS, ...)
        // which almost always require path-style; probe it first, then fall
        // back to virtual-hosted for services that only support that.
        None if custom_endpoint(config).is_some() => vec![true, false],
        // AWS proper: keep the SDK's virtual-hosted default, no probing.
        None => vec![false],
    }
}

/// URLs to check with the outbound destination checks before connecting.
/// Empty when the default AWS endpoints are used. With explicitly
/// virtual-hosted addressing on a custom endpoint the bucket name becomes
/// part of the host name, so that host is included as well. In auto-detect
/// mode the virtual-hosted host is only contacted if path-style fails, and is
/// covered by the connect-time filtering of the client's resolver.
pub fn outbound_urls(config: &S3SourceConfig) -> Vec<String> {
    let Some(endpoint) = custom_endpoint(config) else {
        return Vec::new();
    };
    let mut urls = vec![endpoint.to_string()];
    if config.force_path_style == Some(false) {
        if let Ok(mut url) = url::Url::parse(endpoint) {
            if let Some(url::Host::Domain(host)) = url.host() {
                let virtual_host = format!("{}.{}", config.bucket_name, host);
                if url.set_host(Some(&virtual_host)).is_ok() {
                    urls.push(url.to_string());
                }
            }
        }
    }
    urls
}

/// HTTP client for the SDK whose DNS lookups drop refused addresses.
#[cfg(feature = "s3")]
fn filtering_http_client() -> aws_sdk_s3::config::SharedHttpClient {
    use aws_smithy_http_client::tls::{rustls_provider::CryptoMode, Provider};
    aws_smithy_http_client::Builder::new()
        .tls_provider(Provider::Rustls(CryptoMode::AwsLc))
        .build_with_resolver(FilteringResolver)
}

#[cfg(feature = "s3")]
fn client_for_style(config: &S3SourceConfig, force_path_style: bool) -> Client {
    let credentials = Credentials::new(
        &config.access_key_id,
        &config.secret_access_key,
        None, // session token
        None, // expiry
        "readur-s3-source",
    );
    let region = if config.region.is_empty() { "us-east-1".to_string() } else { config.region.clone() };

    let mut builder = aws_sdk_s3::config::Builder::new()
        .region(AwsRegion::new(region))
        .credentials_provider(credentials)
        .behavior_version_latest()
        .http_client(filtering_http_client())
        .force_path_style(force_path_style);
    if let Some(endpoint_url) = custom_endpoint(config) {
        builder = builder.endpoint_url(endpoint_url);
    }
    Client::from_conf(builder.build())
}

/// Cheap request used to check whether an addressing style works.
#[cfg(feature = "s3")]
async fn probe_style(client: &Client, bucket: &str, path_style: bool) -> bool {
    let probe = client.list_objects_v2().bucket(bucket).max_keys(1).send();
    match tokio::time::timeout(std::time::Duration::from_secs(3), probe).await {
        Ok(Ok(_)) => true,
        Ok(Err(e)) => {
            debug!("S3 addressing probe (path_style={}) failed: {}", path_style, e);
            false
        }
        Err(_) => {
            debug!("S3 addressing probe (path_style={}) timed out after 3s", path_style);
            false
        }
    }
}

/// Build the client for a source. Without an explicit addressing style on a
/// custom endpoint, each candidate style is probed and the first that works
/// is kept. A failed probe does not fail construction: connection errors
/// surface later via initialize()/test_connection().
#[cfg(feature = "s3")]
pub(crate) async fn build_client(config: &S3SourceConfig) -> Client {
    if let Some(endpoint_url) = custom_endpoint(config) {
        info!("Using custom S3 endpoint: {}", endpoint_url);
    }
    let styles = addressing_styles_to_try(config);
    let first = client_for_style(config, styles[0]);
    if styles.len() == 1 || config.bucket_name.is_empty() {
        return first;
    }
    if let Some(detected) = detect_style(config, &styles, &first).await {
        return detected;
    }
    warn!(
        "Could not auto-detect S3 addressing style for bucket '{}'; defaulting to path-style. \
         Set S3_FORCE_PATH_STYLE=true|false to override.",
        config.bucket_name
    );
    first
}

/// Probe `styles` in order and return a client for the first that works.
/// `first` is the already-built client for `styles[0]`.
#[cfg(feature = "s3")]
async fn detect_style(config: &S3SourceConfig, styles: &[bool], first: &Client) -> Option<Client> {
    for &style in styles {
        let candidate = if style == styles[0] { first.clone() } else { client_for_style(config, style) };
        if probe_style(&candidate, &config.bucket_name, style).await {
            info!("Auto-detected S3 addressing style: {}", if style { "path-style" } else { "virtual-hosted" });
            return Some(candidate);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_config() -> S3SourceConfig {
        S3SourceConfig {
            bucket_name: "b".to_string(),
            region: "us-east-1".to_string(),
            access_key_id: "k".to_string(),
            secret_access_key: "s".to_string(),
            endpoint_url: None,
            force_path_style: None,
            prefix: None,
            watch_folders: vec![],
            file_extensions: vec![],
            auto_sync: false,
            sync_interval_minutes: 0,
        }
    }

    #[test]
    fn addressing_style_explicit_wins() {
        let mut cfg = base_config();
        cfg.force_path_style = Some(true);
        assert_eq!(addressing_styles_to_try(&cfg), vec![true]);
        cfg.force_path_style = Some(false);
        cfg.endpoint_url = Some("http://minio:9000".to_string());
        assert_eq!(addressing_styles_to_try(&cfg), vec![false]);
    }

    #[test]
    fn addressing_style_auto_detects_with_custom_endpoint() {
        let mut cfg = base_config();
        cfg.endpoint_url = Some("http://minio:9000".to_string());
        // path-style first: S3-compatible services almost always need it
        assert_eq!(addressing_styles_to_try(&cfg), vec![true, false]);
    }

    #[test]
    fn addressing_style_aws_default_without_endpoint() {
        let cfg = base_config();
        assert_eq!(addressing_styles_to_try(&cfg), vec![false]);
        let mut cfg2 = base_config();
        cfg2.endpoint_url = Some("  ".to_string()); // blank = unset
        assert_eq!(addressing_styles_to_try(&cfg2), vec![false]);
    }

    #[test]
    fn outbound_urls_cover_contacted_hosts() {
        let mut cfg = base_config();
        assert!(outbound_urls(&cfg).is_empty());

        cfg.endpoint_url = Some("http://minio.lan:9000".to_string());
        assert_eq!(outbound_urls(&cfg), vec!["http://minio.lan:9000".to_string()]);

        cfg.force_path_style = Some(false);
        let urls = outbound_urls(&cfg);
        assert_eq!(urls.len(), 2);
        assert_eq!(urls[1], format!("http://{}.minio.lan:9000/", cfg.bucket_name));
    }

    #[cfg(feature = "s3")]
    #[tokio::test]
    async fn clients_use_the_filtering_http_client() {
        let mut cfg = base_config();
        cfg.endpoint_url = Some("http://minio.lan:9000".to_string());
        cfg.force_path_style = Some(true);
        let client = build_client(&cfg).await;
        assert!(client.config().http_client().is_some());
    }

    /// Requests go through the filtering HTTP client to a host name that
    /// resolves to a permitted address, and auto-detection keeps path-style
    /// when the endpoint accepts it.
    #[cfg(feature = "s3")]
    #[tokio::test]
    async fn auto_detected_client_reaches_permitted_endpoint() {
        use wiremock::{matchers::{method, path_regex}, Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        let body = r#"<?xml version="1.0" encoding="UTF-8"?>
<ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/"><Name>b</Name><KeyCount>0</KeyCount><MaxKeys>1</MaxKeys><IsTruncated>false</IsTruncated></ListBucketResult>"#;
        Mock::given(method("GET"))
            .and(path_regex("^/b/?$"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(body, "application/xml"))
            .mount(&server)
            .await;

        let mut cfg = base_config();
        cfg.endpoint_url = Some(format!("http://localhost:{}", server.address().port()));
        let client = build_client(&cfg).await;
        let listed = client.list_objects_v2().bucket("b").max_keys(1).send().await;
        assert!(listed.is_ok(), "{:?}", listed.err());
        assert!(server.received_requests().await.unwrap().len() >= 2, "probe and listing reach the endpoint");
    }
}
