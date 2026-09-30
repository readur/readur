//! Client IP resolution that only honours forwarding headers from trusted
//! reverse proxies (`TRUSTED_PROXIES`). Anything else could be set by the
//! client itself and must not drive rate limiting or audit logs.

use axum::{
    extract::{ConnectInfo, FromRequestParts},
    http::{request::Parts, HeaderMap},
};
use ipnet::IpNet;
use std::convert::Infallible;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
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
/// Otherwise the peer itself is the client.
pub fn resolve_client_ip(peer: IpAddr, headers: &HeaderMap, trusted: &[IpNet]) -> IpAddr {
    if !is_trusted(&peer, trusted) {
        return peer;
    }

    let forwarded: Vec<IpAddr> = headers
        .get_all("x-forwarded-for")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .filter_map(|s| s.trim().parse::<IpAddr>().ok())
        .collect();

    if let Some(client) = forwarded.iter().rev().find(|ip| !is_trusted(ip, trusted)) {
        return *client;
    }
    if let Some(first) = forwarded.first() {
        return *first;
    }

    headers
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.trim().parse::<IpAddr>().ok())
        .unwrap_or(peer)
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

    #[test]
    fn trusted_peer_without_header_is_client() {
        let trusted: Vec<IpNet> = vec!["10.0.0.0/8".parse().unwrap()];
        let peer: IpAddr = "10.0.0.2".parse().unwrap();
        assert_eq!(resolve_client_ip(peer, &HeaderMap::new(), &trusted), peer);
    }
}
