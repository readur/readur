//! Checks applied to user-supplied URLs the server will connect to (WebDAV
//! servers, S3-compatible endpoints).
//!
//! Private and LAN addresses stay allowed because self-hosted storage on the
//! local network is a primary use case. Link-local ranges (which include
//! cloud instance metadata services), well-known metadata addresses and the
//! unspecified address are refused.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutboundUrlError {
    Invalid,
    UnsupportedScheme,
    /// The host resolves to an address that may not be contacted.
    BlockedDestination,
    /// The host could not be resolved right now.
    Unresolvable,
}

impl std::fmt::Display for OutboundUrlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            Self::Invalid => "Invalid URL",
            Self::UnsupportedScheme => "Only http and https URLs are supported",
            Self::BlockedDestination => "The server address is not allowed",
            Self::Unresolvable => "The server host could not be resolved",
        };
        f.write_str(msg)
    }
}

/// Validate a URL the server is about to connect to: http(s) only, and every
/// address the host resolves to must be permitted by [`is_blocked_ip`].
pub async fn validate_outbound_url(raw: &str) -> Result<(), OutboundUrlError> {
    let url = url::Url::parse(raw.trim()).map_err(|_| OutboundUrlError::Invalid)?;
    if url.scheme() != "http" && url.scheme() != "https" {
        return Err(OutboundUrlError::UnsupportedScheme);
    }
    let port = url.port_or_known_default().ok_or(OutboundUrlError::Invalid)?;

    let addrs: Vec<IpAddr> = match url.host().ok_or(OutboundUrlError::Invalid)? {
        url::Host::Ipv4(ip) => vec![IpAddr::V4(ip)],
        url::Host::Ipv6(ip) => vec![IpAddr::V6(ip)],
        url::Host::Domain(domain) => tokio::net::lookup_host((domain, port))
            .await
            .map_err(|_| OutboundUrlError::Unresolvable)?
            .map(|sa| sa.ip())
            .collect(),
    };

    if addrs.is_empty() {
        return Err(OutboundUrlError::Unresolvable);
    }
    if addrs.iter().any(is_blocked_ip) {
        return Err(OutboundUrlError::BlockedDestination);
    }
    Ok(())
}

/// Like [`validate_outbound_url`], but tolerates a host that does not
/// resolve right now. Used when saving configuration, where a temporarily
/// unreachable server should not prevent editing a source.
pub async fn validate_outbound_url_for_config(raw: &str) -> Result<(), OutboundUrlError> {
    match validate_outbound_url(raw).await {
        Err(OutboundUrlError::Unresolvable) => Ok(()),
        other => other,
    }
}

/// Synchronous part of the destination check: the scheme must be http(s) and
/// a host given as an IP literal must be permitted. Domain names are checked
/// when they are resolved (see [`FilteringResolver`]).
pub fn check_url_without_dns(url: &url::Url) -> Result<(), OutboundUrlError> {
    if url.scheme() != "http" && url.scheme() != "https" {
        return Err(OutboundUrlError::UnsupportedScheme);
    }
    let blocked = match url.host() {
        Some(url::Host::Ipv4(ip)) => is_blocked_ip(&IpAddr::V4(ip)),
        Some(url::Host::Ipv6(ip)) => is_blocked_ip(&IpAddr::V6(ip)),
        Some(url::Host::Domain(_)) => false,
        None => return Err(OutboundUrlError::Invalid),
    };
    if blocked {
        Err(OutboundUrlError::BlockedDestination)
    } else {
        Ok(())
    }
}

/// DNS resolver for clients that connect to user-configured servers. Blocked
/// addresses are dropped from every lookup, so a host name that later
/// resolves to a refused address cannot be reached even though it passed
/// validation when the configuration was saved.
#[derive(Debug, Default, Clone, Copy)]
pub struct FilteringResolver;

impl reqwest::dns::Resolve for FilteringResolver {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let host = name.as_str().to_string();
        Box::pin(async move {
            let addrs: Vec<std::net::SocketAddr> = tokio::net::lookup_host((host.as_str(), 0))
                .await?
                .filter(|addr| !is_blocked_ip(&addr.ip()))
                .collect();
            if addrs.is_empty() {
                let err: Box<dyn std::error::Error + Send + Sync> =
                    Box::new(std::io::Error::new(std::io::ErrorKind::PermissionDenied, OutboundUrlError::BlockedDestination.to_string()));
                return Err(err);
            }
            let iter: reqwest::dns::Addrs = Box::new(addrs.into_iter());
            Ok(iter)
        })
    }
}

/// Maximum number of redirects followed by [`redirect_policy`].
const MAX_REDIRECTS: usize = 10;

/// Redirect policy for clients that connect to user-configured servers:
/// each hop must be http(s) and must not target a refused IP literal.
/// Host names are filtered by [`FilteringResolver`] when connecting.
pub fn redirect_policy() -> reqwest::redirect::Policy {
    reqwest::redirect::Policy::custom(|attempt| {
        if attempt.previous().len() >= MAX_REDIRECTS {
            return attempt.error("too many redirects");
        }
        match check_url_without_dns(attempt.url()) {
            Ok(()) => attempt.follow(),
            Err(e) => attempt.error(e.to_string()),
        }
    })
}

/// HTTP client builder for connections to user-configured servers, with the
/// destination checks applied to DNS results and redirects.
pub fn client_builder() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .dns_resolver(std::sync::Arc::new(FilteringResolver))
        .redirect(redirect_policy())
}

impl std::error::Error for OutboundUrlError {}

pub fn is_blocked_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_blocked_ipv4(v4),
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_blocked_ipv4(&v4);
            }
            v6.is_unspecified()
                // fe80::/10 link-local
                || (v6.segments()[0] & 0xffc0) == 0xfe80
                // AWS instance metadata over IPv6
                || *v6 == Ipv6Addr::new(0xfd00, 0x0ec2, 0, 0, 0, 0, 0, 0x0254)
        }
    }
}

fn is_blocked_ipv4(ip: &Ipv4Addr) -> bool {
    ip.is_unspecified()
        || ip.is_link_local() // 169.254.0.0/16
        || ip.is_broadcast()
        // Alibaba Cloud metadata service
        || *ip == Ipv4Addr::new(100, 100, 100, 200)
}

/// Coarse category for a failed connection test. Upstream error bodies and
/// detailed messages are logged server-side, not returned to the client.
pub fn categorize_connection_error(detail: &str) -> &'static str {
    let d = detail.to_ascii_lowercase();
    if ["401", "403", "unauthorized", "forbidden", "invalidaccesskeyid", "signaturedoesnotmatch", "accessdenied", "authentication"]
        .iter()
        .any(|p| d.contains(p))
    {
        "Authentication failed - check the credentials"
    } else if ["certificate", "tls", "ssl"].iter().any(|p| d.contains(p)) {
        "TLS/certificate error"
    } else if ["404", "not found", "nosuchbucket"].iter().any(|p| d.contains(p)) {
        "The requested resource was not found on the server"
    } else if ["timed out", "timeout"].iter().any(|p| d.contains(p)) {
        "Connection timed out"
    } else {
        "Server is unreachable or returned an unexpected response"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rejects_metadata_and_link_local() {
        for url in [
            "http://169.254.169.254/latest/meta-data/",
            "http://[fe80::1]/",
            "http://[fd00:ec2::254]/",
            "http://100.100.100.200/",
            "http://0.0.0.0/",
            "http://[::]/",
            "http://[::ffff:169.254.169.254]/",
        ] {
            assert_eq!(validate_outbound_url(url).await, Err(OutboundUrlError::BlockedDestination), "{}", url);
        }
    }

    #[tokio::test]
    async fn allows_lan_and_loopback_literals() {
        for url in ["http://192.168.1.10:8080/remote.php/dav", "https://10.0.0.5/", "http://127.0.0.1:9000", "http://[fd12::1]/"] {
            assert_eq!(validate_outbound_url(url).await, Ok(()), "{}", url);
        }
    }

    #[tokio::test]
    async fn rejects_other_schemes() {
        assert_eq!(validate_outbound_url("file:///etc/passwd").await, Err(OutboundUrlError::UnsupportedScheme));
        assert_eq!(validate_outbound_url("ftp://example.com").await, Err(OutboundUrlError::UnsupportedScheme));
        assert_eq!(validate_outbound_url("not a url").await, Err(OutboundUrlError::Invalid));
    }

    #[tokio::test]
    async fn config_check_tolerates_unresolvable_hosts() {
        assert_eq!(validate_outbound_url_for_config("https://does-not-exist.invalid/").await, Ok(()));
    }

    #[test]
    fn url_check_without_dns() {
        let check = |u: &str| check_url_without_dns(&url::Url::parse(u).unwrap());
        assert_eq!(check("http://169.254.169.254/"), Err(OutboundUrlError::BlockedDestination));
        assert_eq!(check("http://[fe80::1]/"), Err(OutboundUrlError::BlockedDestination));
        assert_eq!(check("file:///etc/passwd"), Err(OutboundUrlError::UnsupportedScheme));
        assert_eq!(check("https://nextcloud.example/remote.php/dav"), Ok(()));
        assert_eq!(check("http://192.168.1.10:8080/"), Ok(()));
    }

    #[tokio::test]
    async fn resolver_keeps_permitted_addresses() {
        use reqwest::dns::Resolve;
        let name: reqwest::dns::Name = "localhost".parse().unwrap();
        let addrs: Vec<_> = FilteringResolver.resolve(name).await.unwrap().collect();
        assert!(!addrs.is_empty());
        assert!(addrs.iter().all(|a| !is_blocked_ip(&a.ip())));
    }

    #[tokio::test]
    async fn redirects_to_refused_destinations_are_not_followed() {
        use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(path("/start"))
            .respond_with(ResponseTemplate::new(302).insert_header("location", "http://169.254.169.254/latest/meta-data/"))
            .mount(&server)
            .await;
        Mock::given(path("/hop"))
            .respond_with(ResponseTemplate::new(302).insert_header("location", "/done"))
            .mount(&server)
            .await;
        Mock::given(path("/done")).respond_with(ResponseTemplate::new(200)).mount(&server).await;

        let client = client_builder().build().unwrap();
        let err = client.get(format!("{}/start", server.uri())).send().await.unwrap_err();
        assert!(err.is_redirect(), "{err}");

        let ok = client.get(format!("{}/hop", server.uri())).send().await.unwrap();
        assert_eq!(ok.status(), 200);
    }

    #[test]
    fn categorizes_errors() {
        assert_eq!(categorize_connection_error("status: 401 Unauthorized <html>"), "Authentication failed - check the credentials");
        assert_eq!(categorize_connection_error("invalid peer certificate"), "TLS/certificate error");
        assert_eq!(categorize_connection_error("NoSuchBucket"), "The requested resource was not found on the server");
        assert_eq!(categorize_connection_error("connection refused"), "Server is unreachable or returned an unexpected response");
    }
}
