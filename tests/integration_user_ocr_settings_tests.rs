//! Every per-user setting the API stores changes behaviour, and the ones that
//! could not (the OCR worker pool size, CPU priority, retention, ...) are gone
//! from the API: the upload size limit and background OCR switch apply to the
//! uploading user only, out-of-range engine options are refused, and the
//! dropped settings are neither reported nor stored.

use axum::http::StatusCode;
use base64ct::{Base64, Encoding};
use serde_json::{json, Value};
use tower::util::ServiceExt;
use uuid::Uuid;

use readur::services::documents::BACKGROUND_OCR_OFF_WARNING;
use readur::test_utils::{TestConfigBuilder, TestContext};

/// Per-user settings that were stored and shown but had no effect.
const DROPPED: [&str; 11] = [
    "concurrent_ocr_jobs",
    "cpu_priority",
    "memory_limit_mb",
    "auto_detect_language_combination",
    "search_results_per_page",
    "search_snippet_length",
    "retention_days",
    "enable_auto_cleanup",
    "enable_compression",
    "allowed_file_types",
    "office_extraction_enable_detailed_logging",
];

async fn login(ctx: &TestContext) -> (Uuid, String) {
    let auth = ctx.auth_helper();
    let user = auth.create_test_user().await;
    let token = auth.login_user(&user.username, &user.password).await;
    (user.user_response.id, token)
}

async fn send(
    ctx: &TestContext,
    method: &str,
    uri: &str,
    token: &str,
    content_type: Option<String>,
    body: Vec<u8>,
) -> (StatusCode, Value) {
    let mut request = axum::http::Request::builder()
        .method(method)
        .uri(uri)
        .header("Authorization", format!("Bearer {token}"));
    if let Some(content_type) = content_type {
        request = request.header("Content-Type", content_type);
    }
    let response = ctx
        .app
        .clone()
        .oneshot(request.body(axum::body::Body::from(body)).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn get_settings(ctx: &TestContext, token: &str) -> Value {
    let (status, body) = send(ctx, "GET", "/api/settings", token, None, vec![]).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

async fn put_settings(ctx: &TestContext, token: &str, update: Value) -> (StatusCode, Value) {
    send(
        ctx,
        "PUT",
        "/api/settings",
        token,
        Some("application/json".into()),
        update.to_string().into_bytes(),
    )
    .await
}

async fn upload(
    ctx: &TestContext,
    token: &str,
    filename: &str,
    content: &[u8],
) -> (StatusCode, Value) {
    let boundary = format!("----boundary{}", Uuid::new_v4());
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(b"Content-Type: text/plain\r\n\r\n");
    body.extend_from_slice(content);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    send(
        ctx,
        "POST",
        "/api/documents",
        token,
        Some(format!("multipart/form-data; boundary={boundary}")),
        body,
    )
    .await
}

async fn queued(ctx: &TestContext, document_id: Uuid) -> bool {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM ocr_queue WHERE document_id = $1")
        .bind(document_id)
        .fetch_one(&ctx.state.db.pool)
        .await
        .unwrap()
        > 0
}

fn document_id(body: &Value) -> Uuid {
    body["id"].as_str().expect("document id").parse().unwrap()
}

/// Distinct content per call, so deduplication never interferes.
fn text_of(len: usize) -> Vec<u8> {
    let mut data = format!("{}\n", Uuid::new_v4()).into_bytes();
    data.resize(len, b'x');
    data
}

#[tokio::test]
async fn dropped_settings_are_neither_reported_nor_stored() {
    let ctx = TestContext::new().await;
    let (_, token) = login(&ctx).await;

    let settings = get_settings(&ctx, &token).await;
    for name in DROPPED {
        assert!(
            settings.get(name).is_none(),
            "GET /api/settings still reports {name}"
        );
    }

    // Old clients may still send them: they are ignored, not an error
    let mut update = json!({ "ocr_timeout_seconds": 120 });
    for name in DROPPED {
        update[name] = json!(1);
    }
    update["cpu_priority"] = json!("high");
    update["allowed_file_types"] = json!(["pdf"]);
    let (status, body) = put_settings(&ctx, &token, update).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["ocr_timeout_seconds"], 120);
    for name in DROPPED {
        assert!(body.get(name).is_none(), "PUT /api/settings reports {name}");
    }

    // ...and the columns no longer exist to hold them
    let columns: Vec<String> = sqlx::query_scalar(
        "SELECT column_name::text FROM information_schema.columns WHERE table_name = 'settings'",
    )
    .fetch_all(&ctx.state.db.pool)
    .await
    .unwrap();
    for name in DROPPED {
        assert!(
            !columns.iter().any(|c| c == name),
            "settings.{name} still exists"
        );
    }

    // The MCP get_settings tool reports the same shape
    let result = ctx.mcp_call_tool(&token, "get_settings", json!({})).await;
    let reported = result.to_string();
    for name in DROPPED {
        assert!(!reported.contains(name), "MCP get_settings reports {name}");
    }
}

#[tokio::test]
async fn upload_limit_is_the_uploaders_own() {
    let ctx = TestContext::with_config(TestConfigBuilder::default().with_max_file_size_mb(5)).await;
    let (_, strict) = login(&ctx).await;
    let (_, relaxed) = login(&ctx).await;

    let (status, body) = put_settings(&ctx, &strict, json!({ "max_file_size_mb": 1 })).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["max_file_size_mb"], 1);
    // A user who never set a limit has the server's
    assert_eq!(
        get_settings(&ctx, &relaxed).await["max_file_size_mb"],
        Value::Null
    );

    let two_mb = 2 * 1024 * 1024;
    let (status, body) = upload(&ctx, &strict, "big.txt", &text_of(two_mb)).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE, "{body}");
    assert!(body.to_string().contains("1MB"), "{body}");

    let (status, body) = upload(&ctx, &relaxed, "big.txt", &text_of(two_mb)).await;
    assert!(
        status.is_success(),
        "the other user is not limited: {status} {body}"
    );

    // Under their own limit the strict user uploads normally
    let (status, body) = upload(&ctx, &strict, "small.txt", &text_of(512 * 1024)).await;
    assert!(status.is_success(), "{status} {body}");

    // A limit above the server's cannot lift the server's cap
    let (status, _) = put_settings(&ctx, &strict, json!({ "max_file_size_mb": 400 })).await;
    assert_eq!(status, StatusCode::OK);
    // (the request body limit in front of the route cuts it off first)
    let (status, body) = upload(&ctx, &strict, "huge.txt", &text_of(6 * 1024 * 1024)).await;
    assert!(status.is_client_error(), "{status} {body}");

    // null returns the user to the server's limit
    let (status, body) = put_settings(&ctx, &strict, json!({ "max_file_size_mb": null })).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["max_file_size_mb"], Value::Null);
    let (status, body) = upload(&ctx, &strict, "big-again.txt", &text_of(two_mb)).await;
    assert!(status.is_success(), "{status} {body}");
}

#[tokio::test]
async fn mcp_uploads_honour_the_uploaders_limit() {
    let ctx = TestContext::with_config(TestConfigBuilder::default().with_max_file_size_mb(5)).await;
    let (_, token) = login(&ctx).await;
    let (status, _) = put_settings(&ctx, &token, json!({ "max_file_size_mb": 1 })).await;
    assert_eq!(status, StatusCode::OK);

    let result = ctx
        .mcp_call_tool(
            &token,
            "upload_document",
            json!({
                "filename": "big.txt",
                "content_base64": Base64::encode_string(&text_of(2 * 1024 * 1024)),
            }),
        )
        .await;
    assert_eq!(result["isError"], json!(true), "{result}");
    assert!(
        result.to_string().contains("exceeds maximum allowed size"),
        "{result}"
    );
}

#[tokio::test]
async fn background_ocr_off_leaves_uploads_unqueued() {
    let ctx = TestContext::new().await;
    let (_, off) = login(&ctx).await;
    let (_, on) = login(&ctx).await;
    let (status, body) = put_settings(&ctx, &off, json!({ "enable_background_ocr": false })).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, body) = upload(&ctx, &off, "later.txt", &text_of(64)).await;
    assert!(status.is_success(), "{status} {body}");
    assert!(
        !queued(&ctx, document_id(&body)).await,
        "queued despite background OCR off"
    );
    assert_eq!(body["warnings"], json!([BACKGROUND_OCR_OFF_WARNING]));

    let (status, body) = upload(&ctx, &on, "now.txt", &text_of(64)).await;
    assert!(status.is_success(), "{status} {body}");
    assert!(
        queued(&ctx, document_id(&body)).await,
        "the other user's upload is queued"
    );
    assert_eq!(body["warnings"], json!([]));

    // Through MCP the model is told how to process it
    let result = ctx
        .mcp_call_tool(
            &off,
            "upload_document",
            json!({ "filename": "mcp.txt", "content_base64": Base64::encode_string(&text_of(64)) }),
        )
        .await;
    assert_eq!(result["isError"], json!(false), "{result}");
    assert!(
        result.to_string().contains("retry_document_ocr"),
        "{result}"
    );
}

#[tokio::test]
async fn engine_options_outside_what_tesseract_accepts_are_refused() {
    let ctx = TestContext::new().await;
    let (_, token) = login(&ctx).await;
    for update in [
        json!({ "ocr_dpi": 10 }),
        json!({ "ocr_dpi": 5000 }),
        json!({ "ocr_engine_mode": 4 }),
        json!({ "ocr_page_segmentation_mode": 14 }),
        json!({ "ocr_timeout_seconds": 5 }),
        json!({ "ocr_timeout_seconds": 601 }),
        json!({ "max_file_size_mb": 0 }),
        json!({ "ocr_upscale_factor": 0.0 }),
        json!({ "office_extraction_timeout_seconds": 601 }),
    ] {
        let (status, body) = put_settings(&ctx, &token, update.clone()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{update} -> {body}");
    }
    let (status, body) = put_settings(
        &ctx,
        &token,
        json!({ "ocr_dpi": 150, "ocr_engine_mode": 1, "ocr_page_segmentation_mode": 6 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        (
            body["ocr_dpi"].clone(),
            body["ocr_engine_mode"].clone(),
            body["ocr_page_segmentation_mode"].clone()
        ),
        (json!(150), json!(1), json!(6))
    );
}

/// The OCR worker processes a document with its owner's settings and the
/// server's timeout for an owner with none saved.
#[tokio::test]
async fn documents_are_processed_with_their_owners_settings() {
    let ctx = TestContext::new().await;
    let (user_id, token) = login(&ctx).await;
    let (status, _) = put_settings(
        &ctx,
        &token,
        json!({ "ocr_timeout_seconds": 45, "ocr_dpi": 200 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let stored = ctx.state.db.get_user_settings(user_id).await.unwrap();
    let settings = readur::ocr::user_settings::settings_for_processing(stored, 900);
    assert_eq!(
        readur::ocr::user_settings::ocr_timeout(&settings).as_secs(),
        45
    );
    assert_eq!(
        readur::ocr::tesseract_params::TesseractParams::from_settings(&settings).dpi,
        200
    );

    let (other, _) = login(&ctx).await;
    let none = ctx.state.db.get_user_settings(other).await.unwrap();
    let settings = readur::ocr::user_settings::settings_for_processing(none, 420);
    assert_eq!(
        readur::ocr::user_settings::ocr_timeout(&settings).as_secs(),
        420
    );
}
