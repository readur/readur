//! Every metric `/metrics` and `/api/metrics` export is measured, not a
//! stand-in: uptime grows between scrapes, the WebDAV metrics follow the
//! sources in the database, and the removed placeholders stay removed.

use std::collections::BTreeMap;

use axum::http::StatusCode;
use tower::util::ServiceExt;
use uuid::Uuid;

use readur::test_utils::TestContext;

/// Metrics that only ever reported a fixed number and were removed.
const REMOVED: [&str; 15] = [
    "readur_failed_logins_today",
    "readur_document_access_today",
    "readur_webdav_sessions_total",
    "readur_webdav_sessions_successful",
    "readur_webdav_sessions_failed",
    "readur_webdav_success_rate",
    "readur_webdav_files_processed",
    "readur_webdav_bytes_processed",
    "readur_webdav_avg_session_duration_seconds",
    "readur_webdav_avg_processing_rate",
    "readur_webdav_http_requests_total",
    "readur_webdav_request_success_rate",
    "readur_webdav_avg_request_duration_ms",
    "readur_webdav_sessions_active_last_hour",
    "readur_webdav_error_rate_last_hour",
];

async fn admin_token(ctx: &TestContext) -> String {
    let auth = ctx.auth_helper();
    let admin = auth.create_admin_user().await;
    auth.login_user(&admin.username, &admin.password).await
}

async fn get(ctx: &TestContext, uri: &str, token: &str) -> String {
    let response = ctx
        .app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri(uri)
                .header("Authorization", format!("Bearer {token}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

/// Unlabelled samples of one scrape: name -> value.
async fn scrape(ctx: &TestContext, token: &str) -> BTreeMap<String, f64> {
    get(ctx, "/metrics", token)
        .await
        .lines()
        .filter(|line| !line.starts_with('#') && !line.contains('{'))
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let name = parts.next()?.to_string();
            let value = parts.next()?.parse().ok()?;
            Some((name, value))
        })
        .collect()
}

async fn seed_webdav_source(ctx: &TestContext, owner: Uuid, status: &str, files: i64, bytes: i64) {
    sqlx::query(
        "INSERT INTO sources (user_id, name, source_type, enabled, config, status, \
         total_files_synced, total_files_pending, total_size_bytes, last_sync_at) \
         VALUES ($1, $2, 'webdav', true, '{}', $3, $4, 2, $5, NOW() - INTERVAL '10 minutes')",
    )
    .bind(owner)
    .bind(format!("dav-{}", Uuid::new_v4()))
    .bind(status)
    .bind(files)
    .bind(bytes)
    .execute(&ctx.state.db.pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn uptime_grows_between_scrapes() {
    let ctx = TestContext::new().await;
    let token = admin_token(&ctx).await;
    let first = scrape(&ctx, &token).await["readur_uptime_seconds"];
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    let second = scrape(&ctx, &token).await["readur_uptime_seconds"];
    assert!(second >= first + 1.0, "uptime {first} -> {second}");

    // /api/metrics reports the same clock
    let json: serde_json::Value =
        serde_json::from_str(&get(&ctx, "/api/metrics", &token).await).unwrap();
    let api_uptime = json["system"]["uptime_seconds"].as_f64().unwrap();
    assert!(
        api_uptime + 1.0 >= second && api_uptime <= second + 60.0,
        "{api_uptime} vs {second}"
    );
    assert!(json["database"].get("total_queries_today").is_none());
}

#[tokio::test]
async fn webdav_metrics_follow_the_sources_table() {
    let ctx = TestContext::new().await;
    let token = admin_token(&ctx).await;
    let owner = ctx.auth_helper().create_test_user().await.user_response.id;

    let before = scrape(&ctx, &token).await;
    seed_webdav_source(&ctx, owner, "idle", 7, 7000).await;
    seed_webdav_source(&ctx, owner, "error", 3, 300).await;
    let after = scrape(&ctx, &token).await;

    let delta = |name: &str| after[name] - before[name];
    assert_eq!(delta("readur_webdav_sources"), 2.0);
    assert_eq!(delta("readur_webdav_sources_enabled"), 2.0);
    assert_eq!(delta("readur_webdav_sources_error"), 1.0);
    assert_eq!(delta("readur_webdav_files_synced"), 10.0);
    assert_eq!(delta("readur_webdav_bytes_synced"), 7300.0);
    assert_eq!(delta("readur_webdav_files_pending"), 4.0);
    let age = after["readur_webdav_oldest_sync_age_seconds"];
    assert!(age >= 600.0, "oldest sync age {age}");
}

/// Scrape twice around a change in state: the counts that change follow it,
/// and nothing reports a removed placeholder.
#[tokio::test]
async fn no_metric_is_a_fixed_stand_in() {
    let ctx = TestContext::new().await;
    let token = admin_token(&ctx).await;
    let before = scrape(&ctx, &token).await;

    let user = ctx.auth_helper().create_test_user().await;
    seed_webdav_source(&ctx, user.user_response.id, "idle", 5, 50).await;
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    let after = scrape(&ctx, &token).await;

    for name in [
        "readur_uptime_seconds",
        "readur_users_total",
        "readur_users_registered_today",
        "readur_webdav_sources",
        "readur_webdav_files_synced",
    ] {
        assert!(
            after[name] > before[name],
            "{name}: {} -> {}",
            before[name],
            after[name]
        );
    }
    for name in REMOVED {
        assert!(!after.contains_key(name), "{name} is still exported");
    }
    // Measured process and storage figures, when reported, are real numbers
    if let Some(memory) = after.get("readur_memory_usage_bytes") {
        assert!(*memory > 1024.0 * 1024.0, "memory {memory}");
    }
    if let Some(usage) = after.get("readur_storage_usage_percent") {
        assert!(
            (0.0..=100.0).contains(usage) && *usage > 0.0,
            "storage {usage}"
        );
    }
    assert_ne!(after.get("readur_uptime_seconds"), Some(&3600.0));
}
