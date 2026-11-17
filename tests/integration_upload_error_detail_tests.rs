//! Upload failures reach REST clients with their status and `error_code`
//! but never the raw database or storage error behind them.

use axum::http::StatusCode;
use serde_json::{json, Value};

use readur::test_utils::{TestConfigBuilder, TestContext};

#[tokio::test]
async fn rest_upload_storage_failure_hides_the_raw_error() {
    // A storage failure used to reach the client verbatim ("Failed to ingest
    // document: File exists (os error 17)"), and database ones with
    // constraint names. The response keeps its status and error_code and
    // gets a generic message.
    let uploads = tempfile::tempdir().unwrap();
    let upload_path = uploads.path().to_str().unwrap().to_string();
    let ctx =
        TestContext::with_config(TestConfigBuilder::default().with_upload_path(&upload_path)).await;
    let auth = ctx.auth_helper();
    let user = auth.create_test_user().await;
    let token = auth.login_user(&user.username, &user.password).await;
    let alice = user.user_response.id;
    // Documents are stored under <upload path>/documents; make that a file.
    let documents = uploads.path().join("documents");
    let _ = std::fs::remove_dir_all(&documents);
    std::fs::write(&documents, b"not a directory").unwrap();

    let boundary = "readur-storage-failure-boundary";
    let body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"note.txt\"\r\n\
         Content-Type: text/plain\r\n\r\ncannot be stored\r\n--{boundary}--\r\n"
    );
    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/api/documents")
        .header("Authorization", format!("Bearer {token}"))
        .header(
            "Content-Type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(axum::body::Body::from(body))
        .unwrap();
    let response = tower::ServiceExt::oneshot(ctx.app().clone(), request)
        .await
        .unwrap();

    let status = response.status();
    let body: Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{body}");
    assert_eq!(body["error_code"], json!("UPLOAD_INTERNAL_ERROR"));
    assert_eq!(body["status"], json!(500));
    assert_eq!(body["error"], json!("Failed to ingest document"));
    let message = body["error"].as_str().unwrap();
    for leak in ["os error", "exists", &upload_path, "documents/"] {
        assert!(!message.contains(leak), "{leak:?} leaked: {body}");
    }

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM documents WHERE user_id = $1")
        .bind(alice)
        .fetch_one(&ctx.state().db.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}
