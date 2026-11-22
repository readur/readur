//! The sync-progress WebSocket's Origin check, over real TCP connections to a served app so the
//! handshake is a genuine HTTP/1.1 upgrade and the peer address (`ConnectInfo`) is real.
//!
//! The handler checks the Origin before the token, so a handshake without a token answers 403
//! when its Origin is rejected and 401 when it is accepted. With a valid token the accepted
//! handshake completes (101).
//!
//! The headers mirror what the Vite dev server's `/api` proxy sends (frontend/vite.config.ts):
//! `changeOrigin` rewrites Host to the backend's address, the browser's Origin stays the dev
//! server's, and the proxy reports the dev server's host in X-Forwarded-Host. The server may
//! believe that only from a `TRUSTED_PROXIES` peer.

use std::net::SocketAddr;

use readur::models::{CreateSource, SourceType};
use readur::test_utils::{TestConfigBuilder, TestContext};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

const DEV_SERVER_ORIGIN: &str = "http://localhost:5174";
const DEV_SERVER_HOST: &str = "localhost:5174";

/// Serve the context's app on an ephemeral loopback port, with peer addresses.
async fn serve(ctx: &TestContext) -> SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = ctx.app().clone();
    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });
    addr
}

/// Send a WebSocket upgrade for a source's progress stream (`Host` is the backend's own address,
/// as after `changeOrigin`) with the extra headers, and return the response status.
async fn handshake_status(addr: SocketAddr, source: Uuid, extra: &[(&str, &str)]) -> u16 {
    let mut request = format!(
        "GET /api/sources/{source}/sync/progress/ws HTTP/1.1\r\n\
         Host: {addr}\r\n\
         Connection: Upgrade\r\n\
         Upgrade: websocket\r\n\
         Sec-WebSocket-Version: 13\r\n\
         Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n"
    );
    for (name, value) in extra {
        request.push_str(&format!("{name}: {value}\r\n"));
    }
    request.push_str("\r\n");

    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut response = Vec::new();
    let mut buf = [0u8; 1024];
    while !response.windows(2).any(|w| w == b"\r\n") {
        let n = stream.read(&mut buf).await.unwrap();
        assert!(n > 0, "connection closed before a status line");
        response.extend_from_slice(&buf[..n]);
    }
    let status_line = String::from_utf8_lossy(&response);
    status_line
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("no status in {status_line:?}"))
}

fn vite_proxy_headers<'a>(origin: &'a str, forwarded_host: &'a str) -> Vec<(&'a str, &'a str)> {
    vec![
        ("Origin", origin),
        ("X-Forwarded-Host", forwarded_host),
        ("X-Forwarded-Proto", "http"),
    ]
}

#[tokio::test]
async fn trusted_proxy_forwarded_host_admits_the_dev_server_origin() {
    let ctx =
        TestContext::with_config(TestConfigBuilder::default().with_trusted_proxies("127.0.0.1"))
            .await;
    let addr = serve(&ctx).await;
    let source = Uuid::new_v4();

    // Accepted: reaches the token check.
    let proxied = vite_proxy_headers(DEV_SERVER_ORIGIN, DEV_SERVER_HOST);
    assert_eq!(handshake_status(addr, source, &proxied).await, 401);

    // A foreign page proxied the same way: Origin and X-Forwarded-Host disagree.
    let foreign = vite_proxy_headers("http://evil.example", DEV_SERVER_HOST);
    assert_eq!(handshake_status(addr, source, &foreign).await, 403);

    // The scheme the proxy reports must match the Origin's.
    let wrong_scheme = [
        ("Origin", DEV_SERVER_ORIGIN),
        ("X-Forwarded-Host", DEV_SERVER_HOST),
        ("X-Forwarded-Proto", "https"),
    ];
    assert_eq!(handshake_status(addr, source, &wrong_scheme).await, 403);

    // Without forwarding headers the dev server's Origin does not match Host.
    let bare = [("Origin", DEV_SERVER_ORIGIN)];
    assert_eq!(handshake_status(addr, source, &bare).await, 403);
}

#[tokio::test]
async fn forwarded_host_from_an_untrusted_peer_is_ignored() {
    // No TRUSTED_PROXIES: the loopback peer's X-Forwarded-Host proves nothing.
    let ctx = TestContext::with_config(TestConfigBuilder::default()).await;
    let addr = serve(&ctx).await;
    let proxied = vite_proxy_headers(DEV_SERVER_ORIGIN, DEV_SERVER_HOST);
    assert_eq!(handshake_status(addr, Uuid::new_v4(), &proxied).await, 403);
}

#[tokio::test]
async fn cors_allowed_origins_admit_a_proxied_origin_without_trusting_the_peer() {
    let ctx = TestContext::with_config(
        TestConfigBuilder::default().with_cors_allowed_origins(&[DEV_SERVER_ORIGIN]),
    )
    .await;
    let addr = serve(&ctx).await;
    let source = Uuid::new_v4();
    assert_eq!(
        handshake_status(addr, source, &[("Origin", DEV_SERVER_ORIGIN)]).await,
        401
    );
    assert_eq!(
        handshake_status(addr, source, &[("Origin", "http://localhost:9999")]).await,
        403
    );
}

#[tokio::test]
async fn proxied_handshake_with_a_token_upgrades() {
    let ctx =
        TestContext::with_config(TestConfigBuilder::default().with_trusted_proxies("127.0.0.1"))
            .await;
    let auth = ctx.auth_helper();
    let mut user = auth.create_test_user().await;
    let token = user.login(&auth).await.unwrap().to_string();
    let watch = tempfile::tempdir().unwrap();
    let source = ctx
        .state()
        .db
        .create_source(
            user.user_response.id,
            &CreateSource {
                name: "origin".into(),
                source_type: SourceType::LocalFolder,
                enabled: Some(true),
                config: json!({
                    "watch_folders": [watch.path().to_str().unwrap()],
                    "file_extensions": ["txt"],
                    "auto_sync": false,
                    "sync_interval_minutes": 60,
                    "recursive": true,
                    "follow_symlinks": false
                }),
            },
        )
        .await
        .unwrap()
        .id;
    let addr = serve(&ctx).await;

    let protocol = format!("bearer.{token}");
    let mut headers = vite_proxy_headers(DEV_SERVER_ORIGIN, DEV_SERVER_HOST);
    headers.push(("Sec-WebSocket-Protocol", &protocol));
    assert_eq!(handshake_status(addr, source, &headers).await, 101);

    // The same authenticated handshake from a foreign page is still refused.
    let mut foreign = vite_proxy_headers("http://evil.example", DEV_SERVER_HOST);
    foreign.push(("Sec-WebSocket-Protocol", &protocol));
    assert_eq!(handshake_status(addr, source, &foreign).await, 403);
}
