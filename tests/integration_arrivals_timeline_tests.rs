//! Integration tests for `GET /api/sources/arrivals` (per-lane daily arrival
//! counts) and `GET /api/search/timeline` (search matches per month).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use serde_json::Value;
use tower::ServiceExt;
use uuid::Uuid;

use readur::models::Document;
use readur::test_utils::TestContext;

async fn get(ctx: &TestContext, uri: &str, token: Option<&str>) -> (StatusCode, Value) {
    let mut builder = Request::builder().method("GET").uri(uri);
    if let Some(token) = token {
        builder = builder.header("Authorization", format!("Bearer {}", token));
    }
    let response = ctx
        .app
        .clone()
        .oneshot(builder.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

async fn create_source(ctx: &TestContext, user_id: Uuid, name: &str, source_type: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO sources (user_id, name, source_type, config) VALUES ($1, $2, $3, '{}'::jsonb) RETURNING id",
    )
    .bind(user_id)
    .bind(name)
    .bind(source_type)
    .fetch_one(&ctx.state.db.pool)
    .await
    .unwrap()
}

struct Doc {
    user_id: Uuid,
    created_at: DateTime<Utc>,
    source_type: Option<&'static str>,
    source_id: Option<Uuid>,
    mime_type: &'static str,
    content: &'static str,
}

impl Doc {
    fn new(user_id: Uuid, created_at: DateTime<Utc>) -> Self {
        Doc {
            user_id,
            created_at,
            source_type: Some("web_upload"),
            source_id: None,
            mime_type: "application/pdf",
            content: "plain text",
        }
    }

    fn source(mut self, source_type: Option<&'static str>, source_id: Option<Uuid>) -> Self {
        self.source_type = source_type;
        self.source_id = source_id;
        self
    }

    fn text(mut self, mime_type: &'static str, content: &'static str) -> Self {
        self.mime_type = mime_type;
        self.content = content;
        self
    }
}

async fn insert(ctx: &TestContext, doc: Doc) -> Uuid {
    let id = Uuid::new_v4();
    let document = Document {
        id,
        filename: format!("{}.pdf", id),
        original_filename: format!("{}.pdf", id),
        file_path: format!("/tmp/arrivals/{}", id),
        file_size: 100,
        mime_type: doc.mime_type.to_string(),
        content: Some(doc.content.to_string()),
        ocr_text: None,
        ocr_confidence: None,
        ocr_word_count: None,
        ocr_processing_time_ms: None,
        ocr_status: Some("completed".to_string()),
        ocr_error: None,
        ocr_completed_at: None,
        ocr_retry_count: None,
        ocr_failure_reason: None,
        tags: vec![],
        created_at: doc.created_at,
        updated_at: doc.created_at,
        user_id: doc.user_id,
        file_hash: Some(format!("{:x}", Uuid::new_v4().as_u128())),
        original_created_at: None,
        original_modified_at: None,
        source_path: None,
        source_type: doc.source_type.map(str::to_string),
        source_id: doc.source_id,
        file_permissions: None,
        file_owner: None,
        file_group: None,
        source_metadata: None,
    };
    ctx.state.db.create_document(document).await.unwrap();
    id
}

fn lane<'a>(lanes: &'a Value, key: &str) -> &'a Value {
    lanes
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["key"] == key)
        .unwrap_or_else(|| panic!("no lane {} in {}", key, lanes))
}

fn counts(lane: &Value) -> Vec<i64> {
    lane["days"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["count"].as_i64().unwrap())
        .collect()
}

fn keys(lanes: &Value) -> Vec<String> {
    lanes
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["key"].as_str().unwrap().to_string())
        .collect()
}

fn parse_time(value: &Value) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value.as_str().unwrap())
        .unwrap()
        .with_timezone(&Utc)
}

#[tokio::test]
async fn arrivals_are_zero_filled_per_lane_and_scoped_like_documents() {
    let ctx = TestContext::new().await;
    let auth = ctx.auth_helper();
    let user = auth.create_test_user().await;
    let other = auth.create_test_user().await;
    let admin = auth.create_admin_user().await;
    let user_token = auth.login_user(&user.username, &user.password).await;
    let other_token = auth.login_user(&other.username, &other.password).await;
    let admin_token = auth.login_user(&admin.username, &admin.password).await;
    let user_id = user.user_response.id;
    let other_id = other.user_response.id;

    // Names sort case-insensitively: "nextcloud" < "Scans".
    let nextcloud = create_source(&ctx, user_id, "nextcloud", "webdav").await;
    let scans = create_source(&ctx, user_id, "Scans", "local_folder").await;
    let others = create_source(&ctx, other_id, "Other bucket", "s3").await;

    // Midnight UTC today: "today" documents land at or just after it, so the
    // test cannot straddle a day boundary.
    let today = Utc::now().date_naive();
    let midnight = today.and_hms_opt(0, 0, 0).unwrap().and_utc();
    let days_ago = |n: i64| midnight - Duration::days(n) + Duration::hours(3);

    for doc in [
        Doc::new(user_id, midnight).source(Some("source_sync"), Some(nextcloud)),
        Doc::new(user_id, midnight + Duration::seconds(1)).source(Some("webdav"), Some(nextcloud)),
        Doc::new(user_id, days_ago(1)).source(Some("source_sync"), Some(nextcloud)),
        Doc::new(user_id, days_ago(20)).source(Some("source_sync"), Some(nextcloud)),
        Doc::new(user_id, days_ago(3)).source(Some("watch_folder"), None),
        Doc::new(user_id, midnight).source(Some("web_upload"), None),
        Doc::new(user_id, midnight).source(Some("direct_upload"), None),
        Doc::new(user_id, days_ago(2)).source(None, None),
        Doc::new(user_id, days_ago(30)).source(Some("batch_ingest"), None),
        Doc::new(other_id, midnight).source(Some("web_upload"), None),
        Doc::new(other_id, midnight).source(Some("web_upload"), None),
        Doc::new(other_id, midnight).source(Some("source_sync"), Some(others)),
    ] {
        insert(&ctx, doc).await;
    }

    // Regular user: own sources (sorted by name), then watch, then upload.
    let (status, lanes) = get(&ctx, "/api/sources/arrivals", Some(&user_token)).await;
    assert_eq!(status, StatusCode::OK, "{}", lanes);
    assert_eq!(
        keys(&lanes),
        vec![
            nextcloud.to_string(),
            scans.to_string(),
            "watch".to_string(),
            "upload".to_string()
        ]
    );
    for l in lanes.as_array().unwrap() {
        let days = l["days"].as_array().unwrap();
        assert_eq!(days.len(), 14, "default window is 14 days");
        let dates: Vec<NaiveDate> = days
            .iter()
            .map(|d| NaiveDate::parse_from_str(d["date"].as_str().unwrap(), "%Y-%m-%d").unwrap())
            .collect();
        assert_eq!(dates[13], today, "the window ends today (UTC)");
        assert!(dates.windows(2).all(|w| w[1] - w[0] == Duration::days(1)));
        assert_eq!(l["today"], days[13]["count"]);
    }

    let nc = lane(&lanes, &nextcloud.to_string());
    assert_eq!(nc["kind"], "webdav");
    assert_eq!(nc["name"], "nextcloud");
    assert_eq!(nc["source_id"], nextcloud.to_string());
    assert_eq!(nc["enabled"], true);
    assert_eq!(nc["status"], "idle");
    assert_eq!(nc["today"], 2);
    assert_eq!(counts(nc)[12], 1);
    assert_eq!(
        counts(nc).iter().sum::<i64>(),
        3,
        "the 20-day-old document is outside the window"
    );
    assert_eq!(
        parse_time(&nc["last_arrival_at"]),
        midnight + Duration::seconds(1)
    );

    let sc = lane(&lanes, &scans.to_string());
    assert_eq!(sc["kind"], "local_folder");
    assert!(counts(sc).iter().all(|c| *c == 0));
    assert_eq!(sc["last_arrival_at"], Value::Null);

    let watch = lane(&lanes, "watch");
    assert_eq!(watch["kind"], "watch");
    assert_eq!(watch["name"], "Watch folder");
    assert_eq!(watch["source_id"], Value::Null);
    assert_eq!(watch["status"], Value::Null);
    assert_eq!(watch["enabled"], true);
    assert_eq!(counts(watch)[10], 1);
    assert_eq!(watch["today"], 0);

    let upload = lane(&lanes, "upload");
    assert_eq!(upload["kind"], "upload");
    assert_eq!(upload["name"], "Uploads");
    assert_eq!(upload["today"], 2, "web_upload and direct_upload");
    assert_eq!(
        counts(upload)[11],
        1,
        "a document without a source type is an upload"
    );
    assert_eq!(
        counts(upload).iter().sum::<i64>(),
        3,
        "batch_ingest 30 days ago is outside the window"
    );
    assert_eq!(parse_time(&upload["last_arrival_at"]), midnight);

    // The other user sees only their own source and documents.
    let (status, lanes) = get(&ctx, "/api/sources/arrivals", Some(&other_token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        keys(&lanes),
        vec![
            others.to_string(),
            "watch".to_string(),
            "upload".to_string()
        ]
    );
    assert_eq!(lane(&lanes, &others.to_string())["today"], 1);
    assert_eq!(lane(&lanes, "upload")["today"], 2);
    assert_eq!(lane(&lanes, "watch")["last_arrival_at"], Value::Null);

    // Admins see every source and every document, like GET /api/documents.
    let (status, lanes) = get(&ctx, "/api/sources/arrivals", Some(&admin_token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        keys(&lanes),
        vec![
            nextcloud.to_string(),
            others.to_string(),
            scans.to_string(),
            "watch".to_string(),
            "upload".to_string()
        ]
    );
    assert_eq!(lane(&lanes, "upload")["today"], 4);
    assert_eq!(lane(&lanes, &nextcloud.to_string())["today"], 2);
    assert_eq!(lane(&lanes, &others.to_string())["today"], 1);

    ctx.cleanup_and_close().await.ok();
}

#[tokio::test]
async fn arrivals_validates_days_and_requires_auth() {
    let ctx = TestContext::new().await;
    let auth = ctx.auth_helper();
    let user = auth.create_test_user().await;
    let token = auth.login_user(&user.username, &user.password).await;

    for (days, len) in [(1, 1), (7, 7), (60, 60)] {
        let (status, lanes) = get(
            &ctx,
            &format!("/api/sources/arrivals?days={}", days),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "days={}", days);
        assert_eq!(
            keys(&lanes),
            vec!["watch".to_string(), "upload".to_string()]
        );
        for l in lanes.as_array().unwrap() {
            assert_eq!(l["days"].as_array().unwrap().len(), len);
            assert_eq!(l["today"], 0);
        }
    }

    for bad in ["0", "61", "-3", "abc"] {
        let (status, _) = get(
            &ctx,
            &format!("/api/sources/arrivals?days={}", bad),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "days={}", bad);
    }

    let (status, _) = get(&ctx, "/api/sources/arrivals", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    ctx.cleanup_and_close().await.ok();
}

fn months(body: &Value) -> Vec<(String, i64)> {
    body.as_array()
        .unwrap()
        .iter()
        .map(|m| {
            (
                m["month"].as_str().unwrap().to_string(),
                m["count"].as_i64().unwrap(),
            )
        })
        .collect()
}

/// The timeline for `params` sums to the enhanced-search total for `params`.
async fn assert_parity(ctx: &TestContext, token: &str, params: &str) -> Vec<(String, i64)> {
    let (status, timeline) = get(
        ctx,
        &format!("/api/search/timeline?{}", params),
        Some(token),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "timeline {} -> {}",
        params,
        timeline
    );
    let (status, search) = get(
        ctx,
        &format!("/api/search/enhanced?{}&limit=1", params),
        Some(token),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "enhanced {} -> {}", params, search);
    let months = months(&timeline);
    let sum: i64 = months.iter().map(|(_, c)| c).sum();
    assert_eq!(
        sum,
        search["total"].as_i64().unwrap(),
        "parity for {}",
        params
    );
    assert!(
        months.windows(2).all(|w| w[0].0 < w[1].0),
        "ascending, no duplicates"
    );
    assert!(
        months.iter().all(|(_, c)| *c > 0),
        "zero months are omitted"
    );
    months
}

fn at(y: i32, m: u32, d: u32, h: u32, min: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(y, m, d, h, min, 0).unwrap()
}

fn month_list(items: &[(&str, i64)]) -> Vec<(String, i64)> {
    items.iter().map(|(m, c)| (m.to_string(), *c)).collect()
}

#[tokio::test]
async fn search_timeline_counts_every_match_per_month_with_filter_parity() {
    let ctx = TestContext::new().await;
    let auth = ctx.auth_helper();
    let user = auth.create_test_user().await;
    let other = auth.create_test_user().await;
    let admin = auth.create_admin_user().await;
    let user_token = auth.login_user(&user.username, &user.password).await;
    let admin_token = auth.login_user(&admin.username, &admin.password).await;
    let user_id = user.user_response.id;
    let other_id = other.user_response.id;
    let scans = create_source(&ctx, user_id, "Scans", "local_folder").await;

    let shoulder = "physio notes about the shoulder injury";
    let mri = insert(
        &ctx,
        Doc::new(user_id, at(2025, 11, 3, 9, 0)).text("image/png", "MRI of the injured shoulder"),
    )
    .await;
    for doc in [
        // 23:30 on 31 Jan is still January in UTC.
        Doc::new(user_id, at(2026, 1, 31, 23, 30)).text("application/pdf", shoulder),
        Doc::new(user_id, at(2026, 1, 2, 8, 0)).text("application/pdf", shoulder),
        Doc::new(user_id, at(2026, 3, 1, 0, 0))
            .text("application/pdf", shoulder)
            .source(Some("source_sync"), Some(scans)),
        Doc::new(user_id, at(2026, 2, 10, 12, 0)).text("application/pdf", "unrelated invoice"),
        Doc::new(other_id, at(2026, 2, 14, 12, 0)).text("application/pdf", shoulder),
    ] {
        insert(&ctx, doc).await;
    }
    let label: Uuid = sqlx::query_scalar(
        "INSERT INTO labels (user_id, name, color) VALUES ($1, 'medical', '#123456') RETURNING id",
    )
    .bind(user_id)
    .fetch_one(&ctx.state.db.pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO document_labels (document_id, label_id) VALUES ($1, $2)")
        .bind(mri)
        .bind(label)
        .execute(&ctx.state.db.pool)
        .await
        .unwrap();

    let all = assert_parity(&ctx, &user_token, "query=shoulder").await;
    assert_eq!(
        all,
        month_list(&[("2025-11", 1), ("2026-01", 2), ("2026-03", 1)])
    );

    // Every filter narrows the timeline exactly like the enhanced-search count.
    let pdf = assert_parity(
        &ctx,
        &user_token,
        "query=shoulder&mime_types=application/pdf",
    )
    .await;
    assert_eq!(pdf, month_list(&[("2026-01", 2), ("2026-03", 1)]));
    let labelled = assert_parity(
        &ctx,
        &user_token,
        &format!("query=shoulder&label_ids={}", label),
    )
    .await;
    assert_eq!(labelled, month_list(&[("2025-11", 1)]));
    let tagged = assert_parity(&ctx, &user_token, "query=shoulder&tags=medical").await;
    assert_eq!(tagged, labelled);
    let sourced = assert_parity(
        &ctx,
        &user_token,
        &format!("query=shoulder&source_ids={}", scans),
    )
    .await;
    assert_eq!(sourced, month_list(&[("2026-03", 1)]));
    let ranged = assert_parity(
        &ctx,
        &user_token,
        "query=shoulder&created_from=2026-01-01T00:00:00Z&created_to=2026-02-28T23:59:59Z",
    )
    .await;
    assert_eq!(ranged, month_list(&[("2026-01", 2)]));
    let phrase = assert_parity(
        &ctx,
        &user_token,
        "query=shoulder%20injury&search_mode=phrase",
    )
    .await;
    assert_eq!(phrase, month_list(&[("2026-01", 2), ("2026-03", 1)]));
    let none = assert_parity(&ctx, &user_token, "query=zebra").await;
    assert!(none.is_empty());

    // A filter without a query is accepted, as in enhanced search.
    let filter_only = assert_parity(&ctx, &user_token, "mime_types=application/pdf").await;
    assert_eq!(
        filter_only,
        month_list(&[("2026-01", 2), ("2026-02", 1), ("2026-03", 1)])
    );

    // Pagination and sort parameters don't change the histogram.
    let (_, paged) = get(
        &ctx,
        "/api/search/timeline?query=shoulder&limit=1&offset=2&sort_by=filename&sort_order=asc",
        Some(&user_token),
    )
    .await;
    assert_eq!(months(&paged), all);

    // Admins see every user's matches.
    let admin_months = assert_parity(&ctx, &admin_token, "query=shoulder").await;
    assert_eq!(
        admin_months,
        month_list(&[
            ("2025-11", 1),
            ("2026-01", 2),
            ("2026-02", 1),
            ("2026-03", 1)
        ])
    );

    ctx.cleanup_and_close().await.ok();
}

#[tokio::test]
async fn search_timeline_rejects_what_enhanced_search_rejects() {
    let ctx = TestContext::new().await;
    let auth = ctx.auth_helper();
    let user = auth.create_test_user().await;
    let token = auth.login_user(&user.username, &user.password).await;

    for params in [
        "query=a",
        "",
        "query=shoulder&ocr_status=done",
        "query=shoulder&created_from=2026-02-01T00:00:00Z&created_to=2026-01-01T00:00:00Z",
        "query=shoulder&label_ids=not-a-uuid",
    ] {
        let (timeline, _) = get(
            &ctx,
            &format!("/api/search/timeline?{}", params),
            Some(&token),
        )
        .await;
        let (enhanced, _) = get(
            &ctx,
            &format!("/api/search/enhanced?{}", params),
            Some(&token),
        )
        .await;
        assert_eq!(timeline, StatusCode::BAD_REQUEST, "timeline {}", params);
        assert_eq!(enhanced, StatusCode::BAD_REQUEST, "enhanced {}", params);
    }

    let (status, _) = get(&ctx, "/api/search/timeline?query=shoulder", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    ctx.cleanup_and_close().await.ok();
}
