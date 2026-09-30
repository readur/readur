//! Client IP resolution that only honours forwarding headers from trusted
//! reverse proxies (`TRUSTED_PROXIES`). Anything else could be set by the
//! client itself and must not drive rate limiting or audit logs.

use axum::{
    extract::{ConnectInfo, FromRequestParts},
    http::{request::Parts, HeaderMap, HeaderValue},
};
use ipnet::IpNet;
use std::convert::Infallible;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;

use crate::AppState;

/// Best-known client address. `None` when the server was not started with
/// connection info (e.g. in-process tests); callers decide how to treat that.
#[derive(Debug, Clone, Copy)]
pub struct ClientIp(pub Option<IpAddr>);

impl ClientIp {
    /// Address to key per-client limits on; unknown clients share one bucket.
    pub fn or_unspecified(self) -> IpAddr {
        self.0.unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST))
    }
}

impl FromRequestParts<Arc<AppState>> for ClientIp {
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &Arc<AppState>) -> Result<Self, Self::Rejection> {
        let peer = parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map(|ConnectInfo(addr)| addr.ip());
        Ok(ClientIp(peer.map(|peer| {
            resolve_client_ip(peer, &parts.headers, &state.config.security.trusted_proxies)
        })))
    }
}

fn is_trusted(ip: &IpAddr, trusted: &[IpNet]) -> bool {
    trusted.iter().any(|net| net.contains(ip))
}

/// Resolve the originating client given the TCP peer and headers.
///
/// If the peer is a trusted proxy, walk `X-Forwarded-For` from the right,
/// skipping further trusted hops; the first untrusted address is the client.
/// An entry that cannot be parsed ends the walk and the peer is used, since
/// nothing to its left can be attributed to a trusted hop. A header that is
/// not valid text is treated the same way. `X-Real-IP` is only consulted when
/// no `X-Forwarded-For` header is present. Otherwise the peer itself is the
/// client.
pub fn resolve_client_ip(peer: IpAddr, headers: &HeaderMap, trusted: &[IpNet]) -> IpAddr {
    if !is_trusted(&peer, trusted) {
        return peer;
    }

    let values: Vec<&HeaderValue> = headers.get_all("x-forwarded-for").iter().collect();
    if values.is_empty() {
        return headers
            .get("x-real-ip")
            .and_then(|v| v.to_str().ok())
            .and_then(parse_forwarded_ip)
            .unwrap_or(peer);
    }

    let mut entries: Vec<&str> = Vec::new();
    for value in values {
        match value.to_str() {
            Ok(text) => entries.extend(text.split(',')),
            Err(_) => return peer,
        }
    }

    let mut leftmost = None;
    for entry in entries.iter().rev() {
        let Some(ip) = parse_forwarded_ip(entry) else {
            return peer;
        };
        if !is_trusted(&ip, trusted) {
            return ip;
        }
        leftmost = Some(ip);
    }
    // Every hop is a trusted proxy: the leftmost one is the best known client.
    leftmost.unwrap_or(peer)
}

/// Parse one forwarding entry: a bare address, `ipv4:port`, or `[ipv6]:port`.
fn parse_forwarded_ip(entry: &str) -> Option<IpAddr> {
    let entry = entry.trim();
    if let Ok(ip) = entry.parse::<IpAddr>() {
        return Some(ip);
    }
    if let Some(rest) = entry.strip_prefix('[') {
        let (addr, after) = rest.split_once(']')?;
        let port_ok = after.is_empty() || after.strip_prefix(':').is_some_and(is_port);
        return if port_ok { addr.parse::<Ipv6Addr>().ok().map(IpAddr::V6) } else { None };
    }
    let (addr, port) = entry.rsplit_once(':')?;
    if !is_port(port) {
        return None;
    }
    addr.parse::<Ipv4Addr>().ok().map(IpAddr::V4)
}

fn is_port(s: &str) -> bool {
    !s.is_empty() && s.len() <= 5 && s.bytes().all(|b| b.is_ascii_digit()) && s.parse::<u16>().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(xff: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert("x-forwarded-for", xff.parse().unwrap());
        h
    }

    #[test]
    fn untrusted_peer_ignores_headers() {
        let peer: IpAddr = "203.0.113.9".parse().unwrap();
        let got = resolve_client_ip(peer, &headers("1.2.3.4"), &[]);
        assert_eq!(got, peer);
    }

    #[test]
    fn trusted_peer_uses_rightmost_untrusted_hop() {
        let trusted: Vec<IpNet> = vec!["10.0.0.0/8".parse().unwrap()];
        let peer: IpAddr = "10.0.0.2".parse().unwrap();
        // Client spoofed "1.1.1.1"; the proxy appended the real address.
        let got = resolve_client_ip(peer, &headers("1.1.1.1, 198.51.100.7, 10.0.0.5"), &trusted);
        assert_eq!(got, "198.51.100.7".parse::<IpAddr>().unwrap());
    }

    fn trusted() -> Vec<IpNet> {
        vec!["10.0.0.0/8".parse().unwrap()]
    }

    fn peer() -> IpAddr {
        "10.0.0.2".parse().unwrap()
    }

    #[test]
    fn unparseable_entry_ends_the_walk_at_the_peer() {
        // "garbage" sits between the client-supplied part and the proxy hops.
        let got = resolve_client_ip(peer(), &headers("198.51.100.7, garbage, 10.0.0.5"), &trusted());
        assert_eq!(got, peer());
        let got = resolve_client_ip(peer(), &headers("198.51.100.7, "), &trusted());
        assert_eq!(got, peer());
    }

    #[test]
    fn entries_with_ports_are_accepted() {
        let got = resolve_client_ip(peer(), &headers("198.51.100.7:51234, 10.0.0.5:443"), &trusted());
        assert_eq!(got, "198.51.100.7".parse::<IpAddr>().unwrap());
        let got = resolve_client_ip(peer(), &headers("[2001:db8::1]:8080"), &trusted());
        assert_eq!(got, "2001:db8::1".parse::<IpAddr>().unwrap());
        let got = resolve_client_ip(peer(), &headers("2001:db8::2"), &trusted());
        assert_eq!(got, "2001:db8::2".parse::<IpAddr>().unwrap());
        assert_eq!(parse_forwarded_ip("[2001:db8::1]:99999"), None);
        assert_eq!(parse_forwarded_ip("1.2.3.4:"), None);
        assert_eq!(parse_forwarded_ip("unknown"), None);
    }

    #[test]
    fn multiple_headers_are_combined_in_order() {
        let mut h = HeaderMap::new();
        h.append("x-forwarded-for", "1.1.1.1".parse().unwrap());
        h.append("x-forwarded-for", "198.51.100.7, 10.0.0.5".parse().unwrap());
        assert_eq!(resolve_client_ip(peer(), &h, &trusted()), "198.51.100.7".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn non_text_forwarded_header_falls_back_to_peer_not_real_ip() {
        let mut h = HeaderMap::new();
        h.insert("x-forwarded-for", HeaderValue::from_bytes(b"198.51.100.7\xff").unwrap());
        h.insert("x-real-ip", "203.0.113.50".parse().unwrap());
        assert_eq!(resolve_client_ip(peer(), &h, &trusted()), peer());
    }

    #[test]
    fn real_ip_is_used_only_without_forwarded_for() {
        let mut h = HeaderMap::new();
        h.insert("x-real-ip", "203.0.113.50".parse().unwrap());
        assert_eq!(resolve_client_ip(peer(), &h, &trusted()), "203.0.113.50".parse::<IpAddr>().unwrap());
        h.insert("x-forwarded-for", "198.51.100.7".parse().unwrap());
        assert_eq!(resolve_client_ip(peer(), &h, &trusted()), "198.51.100.7".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn all_trusted_hops_resolve_to_the_leftmost() {
        let got = resolve_client_ip(peer(), &headers("10.1.1.1, 10.0.0.5"), &trusted());
        assert_eq!(got, "10.1.1.1".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn trusted_peer_without_header_is_client() {
        let trusted: Vec<IpNet> = vec!["10.0.0.0/8".parse().unwrap()];
        let peer: IpAddr = "10.0.0.2".parse().unwrap();
        assert_eq!(resolve_client_ip(peer, &HeaderMap::new(), &trusted), peer);
    }
}
