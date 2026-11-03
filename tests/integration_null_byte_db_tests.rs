//! #901: U+0000 in user-, file- or remote-derived text must never reach
//! PostgreSQL, and recording a failed document must never itself fail.

use anyhow::Result;
use chrono::Utc;
use readur::db::sanitize::{DbJson, DbText};
use readur::models::{CreateUser, CreateWebDAVDirectory, FailedDocument, UserRole};
use readur::test_utils::{document_helpers::create_test_document, TestContext};
use serde_json::json;
use uuid::Uuid;

async fn create_user(ctx: &TestContext) -> Uuid {
    let suffix = &Uuid::new_v4().simple().to_string()[..8];
    ctx.state
        .db
        .create_user(CreateUser {
            username: format!("nul_{suffix}"),
            email: format!("nul_{suffix}@example.com"),
            password: "password123".to_string(),
            role: Some(UserRole::User),
        })
        .await
        .unwrap()
        .id
}

/// A PDF whose Info dictionary stores CreationDate as a UTF-16BE literal, as
/// many scanners and bank exports do: every other byte of the string is 0x00.
fn pdf_with_utf16_creation_date() -> Vec<u8> {
    let mut pdf = b"%PDF-1.4\n1 0 obj\n<< /CreationDate (\xFE\xFF".to_vec();
    for c in "D:20251201".chars() {
        pdf.extend_from_slice(&[0, c as u8]);
    }
    pdf.extend_from_slice(b") >>\nendobj\n%%EOF\n");
    pdf
}

async fn run(ctx: TestContext, test: impl std::future::Future<Output = Result<()>>) {
    let result = test.await;
    if let Err(e) = ctx.cleanup_and_close().await {
        eprintln!("Warning: test cleanup failed: {e}");
    }
    result.unwrap();
}

#[tokio::test]
async fn raw_binds_hit_the_issue_errors_and_sanitized_binds_do_not() {
    let ctx = TestContext::new().await;
    let pool = ctx.state.db.get_pool().clone();
    run(ctx, async move {
        let text_err = sqlx::query("SELECT $1::text")
            .bind("a\0b")
            .execute(&pool)
            .await
            .unwrap_err();
        assert!(
            text_err.to_string().contains("invalid byte sequence"),
            "{text_err}"
        );
        let json_err = sqlx::query("SELECT $1::jsonb")
            .bind(sqlx::types::Json(json!({"t": "a\0b"})))
            .execute(&pool)
            .await
            .unwrap_err();
        assert!(
            json_err
                .to_string()
                .contains("unsupported Unicode escape sequence"),
            "{json_err}"
        );

        let (text, jsonb): (String, serde_json::Value) =
            sqlx::query_as("SELECT $1::text, $2::jsonb")
                .bind(DbText("a\0b"))
                .bind(DbJson(&json!({"t": "a\0b"})))
                .fetch_one(&pool)
                .await?;
        assert_eq!(text, "ab");
        assert_eq!(jsonb, json!({"t": "ab"}));
        Ok(())
    })
    .await;
}

/// The reported failure: "Failed to create document record ... unsupported
/// Unicode escape sequence", from PDF metadata in the JSONB source_metadata.
#[tokio::test]
async fn document_with_nul_in_pdf_metadata_and_text_fields_is_created() {
    let ctx = TestContext::new().await;
    let user_id = create_user(&ctx).await;
    let db = ctx.state.db.clone();
    run(ctx, async move {
        let pdf = pdf_with_utf16_creation_date();
        let metadata = readur::metadata_extraction::extract_content_metadata(
            &pdf,
            "application/pdf",
            "Invoice 511038.pdf",
        )
        .await?
        .expect("PDF metadata");
        assert!(
            metadata.to_string().contains("\\u0000"),
            "fixture should reproduce the NUL: {metadata}"
        );

        let mut doc = create_test_document(user_id);
        doc.filename = "Invoice\0 511038.pdf".to_string();
        doc.original_filename = doc.filename.clone();
        doc.ocr_text = Some("Total\0 due".to_string());
        doc.tags = vec!["bi\0lls".to_string()];
        doc.source_path = Some("/Bills/Invoice\0 511038.pdf".to_string());
        doc.file_owner = Some("own\0er".to_string());
        doc.source_metadata = Some(metadata);

        let saved = db.create_document(doc).await?;

        assert_eq!(saved.filename, "Invoice 511038.pdf");
        assert_eq!(saved.ocr_text.as_deref(), Some("Total due"));
        assert_eq!(saved.tags, ["bills"]);
        assert_eq!(
            saved.source_path.as_deref(),
            Some("/Bills/Invoice 511038.pdf")
        );
        assert_eq!(saved.file_owner.as_deref(), Some("owner"));
        let stored = saved.source_metadata.expect("source_metadata");
        assert!(
            stored["pdf_creation_date"]
                .as_str()
                .unwrap()
                .contains("D:20251201"),
            "{stored}"
        );

        db.update_document_ocr(
            saved.id,
            Some("re\0run".into()),
            Some(90.0),
            Some(1),
            Some(5),
            Some("completed".into()),
        )
        .await?;
        Ok(())
    })
    .await;
}

/// The second reported failure: `failure_reason = 'database_error'` violated
/// check_failure_reason, so the failed document was never recorded either.
#[tokio::test]
async fn failed_document_with_unknown_reason_and_nul_text_is_still_recorded() {
    let ctx = TestContext::new().await;
    let user_id = create_user(&ctx).await;
    let db = ctx.state.db.clone();
    run(ctx, async move {
        let failed = FailedDocument {
            id: Uuid::new_v4(),
            user_id,
            filename: "CREDITCARD\0_CHECKING_12-2025.pdf".to_string(),
            original_filename: Some("CREDITCARD\0_CHECKING_12-2025.pdf".to_string()),
            original_path: None,
            file_path: Some("/uploads/documents/x.pdf".to_string()),
            file_size: Some(1),
            file_hash: Some("df141794".to_string()),
            mime_type: Some("application/pdf".to_string()),
            content: None,
            tags: Vec::new(),
            ocr_text: Some("partial\0 text".to_string()),
            ocr_confidence: None,
            ocr_word_count: None,
            ocr_processing_time_ms: None,
            failure_reason: "database_error".to_string(),
            failure_stage: "not_a_stage".to_string(),
            existing_document_id: None,
            ingestion_source: "watch".to_string(),
            error_message: Some("unsupported Unicode escape sequence \0".to_string()),
            retry_count: Some(0),
            last_retry_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let stored = db.create_failed_document(failed).await?;

        assert_eq!(stored.failure_reason, "other");
        assert_eq!(stored.failure_stage, "processing");
        assert_eq!(stored.filename, "CREDITCARD_CHECKING_12-2025.pdf");
        assert_eq!(stored.ocr_text.as_deref(), Some("partial text"));
        let message = stored.error_message.unwrap();
        assert!(
            message.contains("failure_reason=database_error"),
            "{message}"
        );
        assert!(message.contains("failure_stage=not_a_stage"), "{message}");
        assert!(!message.contains('\0'));
        Ok(())
    })
    .await;
}

/// Keys are stored sanitized, so lookups by the same raw key must still match.
#[tokio::test]
async fn webdav_directory_with_nul_in_path_round_trips_by_raw_path() {
    let ctx = TestContext::new().await;
    let user_id = create_user(&ctx).await;
    let db = ctx.state.db.clone();
    run(ctx, async move {
        // urlencoding::decode turns a server's `%00` into a real U+0000.
        let raw_path = "/Documents/Bills\0/2025";
        db.create_or_update_webdav_directory(&CreateWebDAVDirectory {
            user_id,
            directory_path: raw_path.to_string(),
            directory_etag: "\"etag\0\"".to_string(),
            file_count: 1,
            total_size_bytes: 1,
        })
        .await?;

        let found = db
            .get_webdav_directory(user_id, raw_path)
            .await?
            .expect("directory by raw path");
        assert_eq!(found.directory_path, "/Documents/Bills/2025");
        assert_eq!(found.directory_etag, "\"etag\"");
        assert!(db.delete_webdav_directory(user_id, raw_path).await?);
        Ok(())
    })
    .await;
}
