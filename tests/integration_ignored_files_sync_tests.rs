//! The ignored-files list end to end: deleting a document a source sync
//! imported puts its file on the owner's ignored list, the next sync does not
//! import it again, and taking it off the list lets the following sync import
//! it. Uploads, and deletions of uploaded documents, never touch the list.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use readur::ingestion::document_ingestion::{
    DeduplicationPolicy, DocumentIngestionService, IngestionResult, USER_UPLOAD_SOURCE_TYPE,
};
use readur::models::{CreateSource, FileIngestionInfo, Source, SourceType};
use readur::scheduling::source_sync::SourceSyncService;
use readur::test_utils::{TestConfigBuilder, TestContext};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

struct Fixture {
    ctx: TestContext,
    user_id: Uuid,
    token: String,
    source: Source,
    _folder: tempfile::TempDir,
    _uploads: tempfile::TempDir,
}

impl Fixture {
    /// An admin (allowed to sync local folders) with a local-folder source
    /// over a folder holding `keep.txt` and `drop.txt`.
    async fn new() -> Self {
        let uploads = tempfile::tempdir().unwrap();
        let ctx = TestContext::with_config(
            TestConfigBuilder::default().with_upload_path(uploads.path().to_str().unwrap()),
        )
        .await;
        let auth = ctx.auth_helper();
        let admin = auth.create_admin_user().await;
        let token = auth.login_user(&admin.username, &admin.password).await;
        let folder = tempfile::tempdir().unwrap();
        std::fs::write(folder.path().join("keep.txt"), b"a file the user keeps").unwrap();
        std::fs::write(folder.path().join("drop.txt"), b"a file the user deletes").unwrap();
        let source = ctx
            .state()
            .db
            .create_source(
                admin.user_response.id,
                &CreateSource {
                    name: "ignored-files".into(),
                    source_type: SourceType::LocalFolder,
                    enabled: Some(true),
                    config: json!({
                        "watch_folders": [folder.path().to_str().unwrap()],
                        "file_extensions": ["txt"],
                        "auto_sync": false,
                        "sync_interval_minutes": 60,
                        "recursive": true,
                        "follow_symlinks": false
                    }),
                },
            )
            .await
            .unwrap();
        Self {
            user_id: admin.user_response.id,
            ctx,
            token,
            source,
            _folder: folder,
            _uploads: uploads,
        }
    }

    /// Runs a sync and returns `(processed, skipped)`.
    async fn sync(&self) -> (usize, usize) {
        let outcome = SourceSyncService::new(self.ctx.state().clone())
            .sync_source(&self.source, false)
            .await
            .unwrap();
        assert_eq!(outcome.failed, 0, "{outcome:?}");
        (outcome.processed, outcome.skipped)
    }

    async fn send(&self, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
        let mut request = Request::builder()
            .method(method)
            .uri(uri)
            .header("Authorization", format!("Bearer {}", self.token));
        let body = match body {
            Some(json) => {
                request = request.header("Content-Type", "application/json");
                Body::from(json.to_string())
            }
            None => Body::empty(),
        };
        let response = self
            .ctx
            .app()
            .clone()
            .oneshot(request.body(body).unwrap())
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

    /// `(id, original_filename)` of the source's documents, by name.
    async fn documents(&self) -> Vec<(Uuid, String)> {
        sqlx::query_as(
            "SELECT id, original_filename FROM documents WHERE source_id = $1 ORDER BY original_filename",
        )
        .bind(self.source.id)
        .fetch_all(&self.ctx.state().db.pool)
        .await
        .unwrap()
    }

    async fn document_named(&self, name: &str) -> Uuid {
        self.documents()
            .await
            .into_iter()
            .find(|(_, n)| n == name)
            .unwrap_or_else(|| panic!("no document {name}"))
            .0
    }

    async fn ignored(&self) -> Vec<Value> {
        let (status, body) = self.send("GET", "/api/ignored/files", None).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body["ignored_files"].as_array().unwrap().clone()
    }

    async fn finish(self) {
        let _ = self.ctx.cleanup_and_close().await;
    }
}

#[tokio::test]
async fn deleted_synced_document_stays_out_until_unignored() {
    let f = Fixture::new().await;
    assert_eq!(f.sync().await.0, 2);

    // Deleting the synced document records it on the ignored list.
    let drop_id = f.document_named("drop.txt").await;
    let (status, body) = f
        .send("DELETE", &format!("/api/documents/{drop_id}"), None)
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT, "{body}");
    let ignored = f.ignored().await;
    assert_eq!(ignored.len(), 1, "{ignored:?}");
    let entry = &ignored[0];
    assert_eq!(entry["original_filename"], "drop.txt");
    assert_eq!(entry["source_type"], "local_folder");
    assert_eq!(entry["source_identifier"], f.source.id.to_string());
    assert_eq!(
        entry["reason"],
        readur::db::ignored_files::REASON_DELETED_BY_USER
    );
    assert!(
        entry["source_path"].as_str().unwrap().ends_with("drop.txt"),
        "{entry}"
    );

    // The next sync skips it.
    let (processed, _) = f.sync().await;
    assert_eq!(processed, 0);
    let names: Vec<String> = f.documents().await.into_iter().map(|(_, n)| n).collect();
    assert_eq!(names, vec!["keep.txt"]);

    // Taken off the list, the following sync imports it again.
    let entry_id = entry["id"].as_str().unwrap();
    let (status, body) = f
        .send("DELETE", &format!("/api/ignored/files/{entry_id}"), None)
        .await;
    assert!(status.is_success(), "{status} {body}");
    assert!(f.ignored().await.is_empty());
    assert_eq!(f.sync().await.0, 1);
    let names: Vec<String> = f.documents().await.into_iter().map(|(_, n)| n).collect();
    assert_eq!(names, vec!["drop.txt", "keep.txt"]);

    f.finish().await;
}

#[tokio::test]
async fn bulk_delete_and_clean_ups_record_synced_documents() {
    let f = Fixture::new().await;
    assert_eq!(f.sync().await.0, 2);

    let ids: Vec<Uuid> = f.documents().await.into_iter().map(|(id, _)| id).collect();
    let (status, body) = f
        .send(
            "POST",
            "/api/documents/bulk/delete",
            Some(json!({ "document_ids": ids })),
        )
        .await;
    assert!(status.is_success(), "{status} {body}");
    assert_eq!(f.ignored().await.len(), 2);

    let (processed, _) = f.sync().await;
    assert_eq!(processed, 0);
    assert!(f.documents().await.is_empty());

    // Bulk un-ignore brings both back.
    let entry_ids: Vec<String> = f
        .ignored()
        .await
        .iter()
        .map(|e| e["id"].as_str().unwrap().to_string())
        .collect();
    let (status, body) = f
        .send(
            "DELETE",
            "/api/ignored/files/bulk/delete",
            Some(json!({ "ignored_file_ids": entry_ids })),
        )
        .await;
    assert!(status.is_success(), "{status} {body}");
    assert_eq!(f.sync().await.0, 2);

    f.finish().await;
}

/// Uploads never consult the list, and deleting an uploaded document does
/// not add to it: only automatic ingestion is affected.
#[tokio::test]
async fn uploads_are_never_ignored() {
    let f = Fixture::new().await;
    assert_eq!(f.sync().await.0, 2);
    let drop_id = f.document_named("drop.txt").await;
    let (status, _) = f
        .send("DELETE", &format!("/api/documents/{drop_id}"), None)
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(f.ignored().await.len(), 1);

    let state = f.ctx.state();
    let ingestion =
        DocumentIngestionService::new(state.db.clone(), state.file_service.as_ref().clone());
    let info = FileIngestionInfo {
        relative_path: "drop.txt".into(),
        full_path: "drop.txt".into(),
        #[allow(deprecated)]
        path: "drop.txt".into(),
        name: "drop.txt".into(),
        size: 23,
        mime_type: "text/plain".into(),
        last_modified: None,
        etag: String::new(),
        is_directory: false,
        created_at: None,
        permissions: None,
        owner: None,
        group: None,
        metadata: None,
    };
    let bytes = b"a file the user deletes".to_vec();

    // The same bytes from a watch folder are ignored...
    let watched = ingestion
        .ingest_from_file_info(
            &info,
            bytes.clone(),
            f.user_id,
            DeduplicationPolicy::Skip,
            "watch_folder",
            None,
        )
        .await
        .unwrap();
    assert!(
        matches!(watched, IngestionResult::Ignored { .. }),
        "{watched:?}"
    );

    // ...but an upload of them is stored.
    let uploaded = ingestion
        .ingest_from_file_info(
            &info,
            bytes,
            f.user_id,
            DeduplicationPolicy::Skip,
            USER_UPLOAD_SOURCE_TYPE,
            None,
        )
        .await
        .unwrap();
    let IngestionResult::Created(document) = uploaded else {
        panic!("upload was not stored: {uploaded:?}");
    };

    // Deleting the uploaded document adds nothing to the list.
    let (status, _) = f
        .send("DELETE", &format!("/api/documents/{}", document.id), None)
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(f.ignored().await.len(), 1);

    f.finish().await;
}
