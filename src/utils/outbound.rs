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
    fn categorizes_errors() {
        assert_eq!(categorize_connection_error("status: 401 Unauthorized <html>"), "Authentication failed - check the credentials");
        assert_eq!(categorize_connection_error("invalid peer certificate"), "TLS/certificate error");
        assert_eq!(categorize_connection_error("NoSuchBucket"), "The requested resource was not found on the server");
        assert_eq!(categorize_connection_error("connection refused"), "Server is unreachable or returned an unexpected response");
    }
}
