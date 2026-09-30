//! Integration tests for whitelisted sorting and filtering on the document list
//! (`GET /api/documents`) and search (`GET /api/search`, `GET /api/search/enhanced`)
//! endpoints.

use std::cmp::Ordering;
use std::collections::HashSet;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::{DateTime, Duration, TimeZone, Utc};
use serde_json::Value;
use tower::ServiceExt;
use uuid::Uuid;

use readur::models::{Document, DocumentFilters, DocumentSortField, SortOrder, UserRole};
use readur::test_utils::{TestAuthHelper, TestContext};

/// Column values of a seeded document, used to compute the expected order.
#[derive(Clone, Debug)]
struct Seed {
    id: Uuid,
    filename: &'static str,
    file_size: i64,
    mime_type: &'static str,
    ocr_status: Option<&'static str>,
    ocr_confidence: Option<f32>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    source_type: Option<&'static str>,
    source_id: Option<Uuid>,
    content: &'static str,
}

#[derive(Debug, PartialEq, PartialOrd)]
enum Key {
    Str(String),
    Int(i64),
    Float(f32),
    Time(DateTime<Utc>),
}

impl Seed {
    fn key(&self, field: DocumentSortField) -> Option<Key> {
        match field {
            DocumentSortField::CreatedAt => Some(Key::Time(self.created_at)),
            DocumentSortField::UpdatedAt => Some(Key::Time(self.updated_at)),
            DocumentSortField::Filename => Some(Key::Str(self.filename.to_string())),
            DocumentSortField::FileSize => Some(Key::Int(self.file_size)),
            DocumentSortField::OcrStatus => self.ocr_status.map(|s| Key::Str(s.to_string())),
            DocumentSortField::MimeType => Some(Key::Str(self.mime_type.to_string())),
            DocumentSortField::OcrConfidence => self.ocr_confidence.map(Key::Float),
        }
    }

    fn document(&self, user_id: Uuid) -> Document {
        Document {
            id: self.id,
            filename: self.filename.to_string(),
            original_filename: self.filename.to_string(),
            file_path: format!("/tmp/sort_filter/{}", self.id),
            file_size: self.file_size,
            mime_type: self.mime_type.to_string(),
            content: Some(self.content.to_string()),
            ocr_text: None,
            ocr_confidence: self.ocr_confidence,
            ocr_word_count: None,
            ocr_processing_time_ms: None,
            ocr_status: self.ocr_status.map(str::to_string),
            ocr_error: None,
            ocr_completed_at: None,
            ocr_retry_count: None,
            ocr_failure_reason: None,
            tags: vec![],
            created_at: self.created_at,
            updated_at: self.updated_at,
            user_id,
            file_hash: Some(format!("{:x}", Uuid::new_v4().as_u128())),
            original_created_at: None,
            original_modified_at: None,
            source_path: None,
            source_type: self.source_type.map(str::to_string),
            source_id: self.source_id,
            file_permissions: None,
            file_owner: None,
            file_group: None,
            source_metadata: None,
        }
    }
}

/// Expected ordering: key in the requested direction with NULLs last, then id
/// in the same direction.
fn expected_order(seeds: &[Seed], field: DocumentSortField, order: SortOrder) -> Vec<Uuid> {
    let mut sorted: Vec<&Seed> = seeds.iter().collect();
    sorted.sort_by(|a, b| {
        let key_cmp = match (a.key(field), b.key(field)) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Greater,
            (Some(_), None) => Ordering::Less,
            (Some(x), Some(y)) => {
                let c = x.partial_cmp(&y).unwrap();
                if order == SortOrder::Desc {
                    c.reverse()
                } else {
                    c
                }
            }
        };
        key_cmp.then_with(|| {
            let c = a.id.cmp(&b.id);
            if order == SortOrder::Desc {
                c.reverse()
            } else {
                c
            }
        })
    });
    sorted.into_iter().map(|s| s.id).collect()
}

fn day(n: i64) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap() + Duration::days(n)
}

fn field_name(field: DocumentSortField) -> &'static str {
    match field {
        DocumentSortField::CreatedAt => "created_at",
        DocumentSortField::UpdatedAt => "updated_at",
        DocumentSortField::Filename => "filename",
        DocumentSortField::FileSize => "file_size",
        DocumentSortField::OcrStatus => "ocr_status",
        DocumentSortField::MimeType => "mime_type",
        DocumentSortField::OcrConfidence => "ocr_confidence",
    }
}

fn order_name(order: SortOrder) -> &'static str {
    match order {
        SortOrder::Asc => "asc",
        SortOrder::Desc => "desc",
    }
}

async fn get(ctx: &TestContext, uri: &str, token: &str) -> (StatusCode, Value) {
    let response = ctx
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(uri)
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

fn ids_of(docs: &Value) -> Vec<Uuid> {
    docs.as_array()
        .expect("documents array")
        .iter()
        .map(|d| Uuid::parse_str(d["id"].as_str().unwrap()).unwrap())
        .collect()
}

/// Walks every page of `GET /api/documents?<params>` with limit 2 and returns
/// the concatenated ids plus the reported total.
async fn walk_list(ctx: &TestContext, token: &str, params: &str) -> (Vec<Uuid>, i64) {
    let mut ids = Vec::new();
    let mut offset = 0;
    let mut total = -1;
    loop {
        let uri = format!("/api/documents?limit=2&offset={}&{}", offset, params);
        let (status, body) = get(ctx, &uri, token).await;
        assert_eq!(status, StatusCode::OK, "GET {} -> {}", uri, body);
        let page_total = body["pagination"]["total"].as_i64().unwrap();
        if total >= 0 {
            assert_eq!(page_total, total, "total must be stable across pages");
        }
        total = page_total;
        let page = ids_of(&body["documents"]);
        assert!(page.len() <= 2);
        ids.extend(page);
        if !body["pagination"]["has_more"].as_bool().unwrap() {
            break;
        }
        offset += 2;
    }
    (ids, total)
}

async fn create_label(ctx: &TestContext, user_id: Uuid, name: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO labels (user_id, name, color) VALUES ($1, $2, '#123456') RETURNING id",
    )
    .bind(user_id)
    .bind(name)
    .fetch_one(&ctx.state.db.pool)
    .await
    .unwrap()
}

async fn assign_label(ctx: &TestContext, document_id: Uuid, label_id: Uuid) {
    sqlx::query("INSERT INTO document_labels (document_id, label_id) VALUES ($1, $2)")
        .bind(document_id)
        .bind(label_id)
        .execute(&ctx.state.db.pool)
        .await
        .unwrap();
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

struct Fixture {
    seeds: Vec<Seed>,
    other_seeds: Vec<Seed>,
    user_token: String,
    other_token: String,
    admin_token: String,
    user_id: Uuid,
    label_a: Uuid,
    label_b: Uuid,
    webdav_source: Uuid,
    folder_source: Uuid,
}

async fn seed(ctx: &TestContext) -> Fixture {
    let auth = TestAuthHelper::new(ctx.app.clone());
    let user = auth.create_test_user().await;
    let other = auth.create_test_user().await;
    let admin = auth.create_admin_user().await;
    let user_token = auth.login_user(&user.username, &user.password).await;
    let other_token = auth.login_user(&other.username, &other.password).await;
    let admin_token = auth.login_user(&admin.username, &admin.password).await;
    let user_id = user.user_response.id;
    let other_id = other.user_response.id;

    let webdav_source = create_source(ctx, user_id, "Nextcloud", "webdav").await;
    let folder_source = create_source(ctx, user_id, "Scans", "local_folder").await;

    // Values are chosen so that text columns only differ in plain lowercase
    // letters (collation-independent) and several columns contain ties and NULLs.
    let seeds = vec![
        Seed {
            id: Uuid::new_v4(),
            filename: "alpha.pdf",
            file_size: 5000,
            mime_type: "application/pdf",
            ocr_status: Some("completed"),
            ocr_confidence: Some(91.5),
            created_at: day(0),
            updated_at: day(9),
            source_type: Some("webdav"),
            source_id: Some(webdav_source),
            content: "invoice invoice invoice invoice quarterly invoice",
        },
        Seed {
            id: Uuid::new_v4(),
            filename: "bravo.png",
            file_size: 1200,
            mime_type: "image/png",
            ocr_status: Some("failed"),
            ocr_confidence: None,
            created_at: day(1),
            updated_at: day(1),
            source_type: Some("webdav"),
            source_id: Some(webdav_source),
            content: "holiday photo",
        },
        Seed {
            id: Uuid::new_v4(),
            filename: "charlie.txt",
            file_size: 1200,
            mime_type: "text/plain",
            ocr_status: Some("pending"),
            ocr_confidence: Some(40.0),
            created_at: day(2),
            updated_at: day(7),
            source_type: Some("local_folder"),
            source_id: Some(folder_source),
            content: "an invoice reminder",
        },
        Seed {
            id: Uuid::new_v4(),
            filename: "delta.pdf",
            file_size: 99000,
            mime_type: "application/pdf",
            ocr_status: None,
            ocr_confidence: None,
            created_at: day(3),
            updated_at: day(3),
            source_type: Some("direct_upload"),
            source_id: None,
            content: "tax return",
        },
        Seed {
            id: Uuid::new_v4(),
            filename: "echo.jpeg",
            file_size: 300,
            mime_type: "image/jpeg",
            ocr_status: Some("processing"),
            ocr_confidence: Some(75.0),
            created_at: day(4),
            updated_at: day(12),
            source_type: None,
            source_id: None,
            content: "invoice scan",
        },
        Seed {
            id: Uuid::new_v4(),
            filename: "foxtrot.pdf",
            file_size: 5000,
            mime_type: "application/pdf",
            ocr_status: Some("completed"),
            ocr_confidence: Some(91.5),
            created_at: day(5),
            updated_at: day(5),
            source_type: Some("direct_upload"),
            source_id: None,
            content: "contract invoice terms",
        },
        Seed {
            id: Uuid::new_v4(),
            filename: "golf.txt",
            file_size: 42,
            mime_type: "text/plain",
            ocr_status: Some("completed"),
            ocr_confidence: Some(12.0),
            created_at: day(6),
            updated_at: day(6),
            source_type: None,
            source_id: None,
            content: "shopping list",
        },
    ];
    for s in &seeds {
        ctx.state
            .db
            .create_document(s.document(user_id))
            .await
            .unwrap();
    }

    let other_seeds = vec![
        Seed {
            id: Uuid::new_v4(),
            filename: "hotel.pdf",
            file_size: 10,
            mime_type: "application/pdf",
            ocr_status: Some("completed"),
            ocr_confidence: Some(88.0),
            created_at: day(2),
            updated_at: day(2),
            source_type: None,
            source_id: None,
            content: "invoice from another user",
        },
        Seed {
            id: Uuid::new_v4(),
            filename: "india.txt",
            file_size: 20,
            mime_type: "text/plain",
            ocr_status: Some("pending"),
            ocr_confidence: None,
            created_at: day(3),
            updated_at: day(3),
            source_type: None,
            source_id: None,
            content: "notes",
        },
    ];
    for s in &other_seeds {
        ctx.state
            .db
            .create_document(s.document(other_id))
            .await
            .unwrap();
    }

    let label_a = create_label(ctx, user_id, "finance").await;
    let label_b = create_label(ctx, user_id, "personal").await;
    assign_label(ctx, seeds[0].id, label_a).await;
    assign_label(ctx, seeds[2].id, label_a).await;
    assign_label(ctx, seeds[5].id, label_a).await;
    assign_label(ctx, seeds[1].id, label_b).await;
    assign_label(ctx, seeds[5].id, label_b).await;

    // Active OCR progress for the processing document, and a finished queue row
    // for another document whose progress must not be reported.
    sqlx::query(
        "INSERT INTO ocr_queue (document_id, status, progress_current, progress_total, started_at) \
         VALUES ($1, 'processing', 2, 5, NOW())",
    )
    .bind(seeds[4].id)
    .execute(&ctx.state.db.pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO ocr_queue (document_id, status, progress_current, progress_total) VALUES ($1, 'completed', 5, 5)")
        .bind(seeds[0].id)
        .execute(&ctx.state.db.pool)
        .await
        .unwrap();

    Fixture {
        seeds,
        other_seeds,
        user_token,
        other_token,
        admin_token,
        user_id,
        label_a,
        label_b,
        webdav_source,
        folder_source,
    }
}

fn id_set(ids: &[Uuid]) -> HashSet<Uuid> {
    ids.iter().copied().collect()
}

fn seed_ids<'a>(seeds: impl IntoIterator<Item = &'a Seed>) -> HashSet<Uuid> {
    seeds.into_iter().map(|s| s.id).collect()
}

#[tokio::test]
async fn list_sorts_every_field_across_pages() {
    let ctx = TestContext::new().await;
    let fx = seed(&ctx).await;

    // Default order: newest first
    let (ids, total) = walk_list(&ctx, &fx.user_token, "").await;
    assert_eq!(total, fx.seeds.len() as i64);
    assert_eq!(
        ids,
        expected_order(&fx.seeds, DocumentSortField::CreatedAt, SortOrder::Desc)
    );

    for field in DocumentSortField::ALL {
        for order in [SortOrder::Asc, SortOrder::Desc] {
            let params = format!(
                "sort_by={}&sort_order={}",
                field_name(field),
                order_name(order)
            );
            let (ids, total) = walk_list(&ctx, &fx.user_token, &params).await;
            assert_eq!(total, fx.seeds.len() as i64, "{}", params);
            assert_eq!(
                id_set(&ids).len(),
                ids.len(),
                "pages overlap for {}",
                params
            );
            assert_eq!(
                ids,
                expected_order(&fx.seeds, field, order),
                "wrong order for {}",
                params
            );

            // A second walk yields the identical sequence (stable ordering)
            let (again, _) = walk_list(&ctx, &fx.user_token, &params).await;
            assert_eq!(ids, again, "unstable order for {}", params);
        }
    }

    // sort_by without sort_order defaults to descending
    let (ids, _) = walk_list(&ctx, &fx.user_token, "sort_by=file_size").await;
    assert_eq!(
        ids,
        expected_order(&fx.seeds, DocumentSortField::FileSize, SortOrder::Desc)
    );

    // Invalid sort parameters are rejected
    for bad in [
        "sort_by=file_path",
        "sort_by=created_at%20DESC%3B%20DROP%20TABLE%20documents",
        "sort_order=sideways",
    ] {
        let (status, _) = get(&ctx, &format!("/api/documents?{}", bad), &fx.user_token).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{}", bad);
    }

    ctx.cleanup_and_close().await.ok();
}

#[tokio::test]
async fn list_filters_counts_roles_and_progress() {
    let ctx = TestContext::new().await;
    let fx = seed(&ctx).await;
    let s = &fx.seeds;

    let cases: Vec<(String, HashSet<Uuid>)> = vec![
        (
            format!("label_ids={}", fx.label_a),
            seed_ids([&s[0], &s[2], &s[5]]),
        ),
        (
            format!("label_ids={},{}", fx.label_a, fx.label_b),
            seed_ids([&s[0], &s[1], &s[2], &s[5]]),
        ),
        (
            format!("source_ids={}", fx.webdav_source),
            seed_ids([&s[0], &s[1]]),
        ),
        (
            format!("source_ids={},{}", fx.webdav_source, fx.folder_source),
            seed_ids([&s[0], &s[1], &s[2]]),
        ),
        ("source_types=webdav".to_string(), seed_ids([&s[0], &s[1]])),
        // direct_upload also matches documents without any source
        (
            "source_types=direct_upload".to_string(),
            seed_ids([&s[3], &s[4], &s[5], &s[6]]),
        ),
        (
            "source_types=local_folder,direct_upload".to_string(),
            seed_ids([&s[2], &s[3], &s[4], &s[5], &s[6]]),
        ),
        (
            "mime_types=application/pdf".to_string(),
            seed_ids([&s[0], &s[3], &s[5]]),
        ),
        (
            "mime_types=image/png,image/jpeg".to_string(),
            seed_ids([&s[1], &s[4]]),
        ),
        (
            "ocr_status=completed".to_string(),
            seed_ids([&s[0], &s[5], &s[6]]),
        ),
        ("ocr_status=failed".to_string(), seed_ids([&s[1]])),
        ("ocr_status=processing".to_string(), seed_ids([&s[4]])),
        // pending includes documents without a status
        ("ocr_status=pending".to_string(), seed_ids([&s[2], &s[3]])),
        ("tags=personal".to_string(), seed_ids([&s[1], &s[5]])),
        (
            "created_from=2026-01-03T12:00:00Z".to_string(),
            seed_ids([&s[2], &s[3], &s[4], &s[5], &s[6]]),
        ),
        (
            "created_to=2026-01-02T12:00:00Z".to_string(),
            seed_ids([&s[0], &s[1]]),
        ),
        (
            "created_from=2026-01-02T00:00:00Z&created_to=2026-01-05T00:00:00Z".to_string(),
            seed_ids([&s[1], &s[2], &s[3]]),
        ),
        (
            format!(
                "label_ids={}&mime_types=application/pdf&ocr_status=completed",
                fx.label_a
            ),
            seed_ids([&s[0], &s[5]]),
        ),
    ];

    for (params, expected) in &cases {
        let (ids, total) = walk_list(
            &ctx,
            &fx.user_token,
            &format!("{}&sort_by=filename&sort_order=asc", params),
        )
        .await;
        assert_eq!(id_set(&ids), *expected, "filter {}", params);
        assert_eq!(
            ids.len(),
            expected.len(),
            "duplicates for filter {}",
            params
        );
        assert_eq!(
            total,
            expected.len() as i64,
            "count mismatch for filter {}",
            params
        );
    }

    // The DB count helper agrees with the filtered list
    let filters = DocumentFilters {
        label_ids: Some(vec![fx.label_a]),
        ..Default::default()
    };
    let count = ctx
        .state
        .db
        .count_documents_filtered(fx.user_id, UserRole::User, &filters)
        .await
        .unwrap();
    let listed = ctx
        .state
        .db
        .list_documents_filtered(fx.user_id, UserRole::User, &filters, None, None, 100, 0)
        .await
        .unwrap();
    assert_eq!(count, 3);
    assert_eq!(listed.len() as i64, count);

    // Invalid filters are rejected
    for bad in [
        "ocr_status=done",
        "label_ids=not-a-uuid",
        "source_ids=1,2",
        "created_from=yesterday",
    ] {
        let (status, _) = get(&ctx, &format!("/api/documents?{}", bad), &fx.user_token).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{}", bad);
    }

    // Role isolation: users only see their own documents, admins see all
    let (other_ids, other_total) = walk_list(&ctx, &fx.other_token, "").await;
    assert_eq!(id_set(&other_ids), seed_ids(&fx.other_seeds));
    assert_eq!(other_total, fx.other_seeds.len() as i64);
    let (foreign, foreign_total) =
        walk_list(&ctx, &fx.other_token, &format!("label_ids={}", fx.label_a)).await;
    assert!(foreign.is_empty());
    assert_eq!(foreign_total, 0);

    let (admin_ids, admin_total) =
        walk_list(&ctx, &fx.admin_token, "sort_by=filename&sort_order=asc").await;
    let all: HashSet<Uuid> = seed_ids(fx.seeds.iter().chain(fx.other_seeds.iter()));
    assert_eq!(id_set(&admin_ids), all);
    assert_eq!(admin_total, all.len() as i64);
    let (admin_pdf, _) = walk_list(&ctx, &fx.admin_token, "mime_types=application/pdf").await;
    assert_eq!(
        id_set(&admin_pdf),
        seed_ids([&s[0], &s[3], &s[5], &fx.other_seeds[0]])
    );

    // OCR progress and labels are attached to list rows without duplicating them
    let (status, body) = get(
        &ctx,
        "/api/documents?limit=50&ocr_status=processing",
        &fx.user_token,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let docs = body["documents"].as_array().unwrap();
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0]["ocr_progress_current"], 2);
    assert_eq!(docs[0]["ocr_progress_total"], 5);

    let (_, body) = get(&ctx, "/api/documents?limit=50", &fx.user_token).await;
    let foxtrot = body["documents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == s[5].id.to_string())
        .unwrap();
    let mut label_names: Vec<&str> = foxtrot["labels"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["name"].as_str().unwrap())
        .collect();
    label_names.sort();
    assert_eq!(label_names, vec!["finance", "personal"]);
    assert!(foxtrot.get("ocr_progress_current").is_none());

    ctx.cleanup_and_close().await.ok();
}

#[tokio::test]
async fn search_supports_sort_and_filters() {
    let ctx = TestContext::new().await;
    let fx = seed(&ctx).await;
    let s = &fx.seeds;
    // Documents whose content matches "invoice"
    let invoice_docs = seed_ids([&s[0], &s[2], &s[4], &s[5]]);

    // Enhanced search: default order is by relevance
    let (status, body) = get(
        &ctx,
        "/api/search/enhanced?query=invoice&limit=50",
        &fx.user_token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let ids = ids_of(&body["documents"]);
    assert_eq!(id_set(&ids), invoice_docs);
    assert_eq!(body["total"], 4);
    assert_eq!(
        ids[0], s[0].id,
        "the document mentioning invoice most often ranks first"
    );
    let ranks: Vec<f64> = body["documents"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["search_rank"].as_f64().unwrap())
        .collect();
    assert!(
        ranks.windows(2).all(|w| w[0] >= w[1]),
        "ranks not descending: {:?}",
        ranks
    );

    // Enhanced search with an explicit sort, walked page by page
    let invoice_seeds: Vec<Seed> = s
        .iter()
        .filter(|d| invoice_docs.contains(&d.id))
        .cloned()
        .collect();
    for (field, order) in [
        (DocumentSortField::FileSize, SortOrder::Asc),
        (DocumentSortField::Filename, SortOrder::Desc),
        (DocumentSortField::OcrConfidence, SortOrder::Asc),
    ] {
        let mut walked = Vec::new();
        for offset in (0..4).step_by(2) {
            let uri = format!(
                "/api/search/enhanced?query=invoice&limit=2&offset={}&sort_by={}&sort_order={}",
                offset,
                field_name(field),
                order_name(order)
            );
            let (status, body) = get(&ctx, &uri, &fx.user_token).await;
            assert_eq!(status, StatusCode::OK, "{}", body);
            assert_eq!(body["total"], 4);
            walked.extend(ids_of(&body["documents"]));
        }
        assert_eq!(
            walked,
            expected_order(&invoice_seeds, field, order),
            "{:?} {:?}",
            field,
            order
        );
    }

    // Query + filters + sort together
    let uri = format!(
        "/api/search/enhanced?query=invoice&label_ids={}&sort_by=created_at&sort_order=asc",
        fx.label_a
    );
    let (status, body) = get(&ctx, &uri, &fx.user_token).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(ids_of(&body["documents"]), vec![s[0].id, s[2].id, s[5].id]);
    assert_eq!(body["total"], 3);

    let (_, body) = get(
        &ctx,
        "/api/search/enhanced?query=invoice&source_types=direct_upload",
        &fx.user_token,
    )
    .await;
    assert_eq!(
        id_set(&ids_of(&body["documents"])),
        seed_ids([&s[4], &s[5]])
    );
    assert_eq!(body["total"], 2);

    // Filters alone are enough for a search without a query
    let (status, body) = get(
        &ctx,
        &format!("/api/search/enhanced?source_ids={}", fx.webdav_source),
        &fx.user_token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(
        id_set(&ids_of(&body["documents"])),
        seed_ids([&s[0], &s[1]])
    );

    // The response carries labels, source fields, updated_at and OCR progress
    let (_, body) = get(
        &ctx,
        "/api/search/enhanced?query=invoice&limit=50",
        &fx.user_token,
    )
    .await;
    let docs = body["documents"].as_array().unwrap();
    let alpha = docs
        .iter()
        .find(|d| d["id"] == s[0].id.to_string())
        .unwrap();
    assert_eq!(alpha["labels"][0]["name"], "finance");
    assert_eq!(alpha["source_id"], fx.webdav_source.to_string());
    assert_eq!(alpha["source_type"], "webdav");
    assert!(
        alpha.get("ocr_progress_current").is_none(),
        "only processing queue rows report progress"
    );
    let updated: DateTime<Utc> = alpha["updated_at"].as_str().unwrap().parse().unwrap();
    assert_eq!(updated, s[0].updated_at);
    let echo = docs
        .iter()
        .find(|d| d["id"] == s[4].id.to_string())
        .unwrap();
    assert_eq!(echo["ocr_progress_current"], 2);
    assert_eq!(echo["ocr_progress_total"], 5);
    assert!(echo["source_id"].is_null());

    // Basic search: default order stays newest first; sort and filters apply
    let (status, body) = get(&ctx, "/api/search?query=invoice&limit=50", &fx.user_token).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(
        ids_of(&body["documents"]),
        expected_order(
            &invoice_seeds,
            DocumentSortField::CreatedAt,
            SortOrder::Desc
        )
    );
    assert_eq!(body["total"], 4);

    let uri = format!(
        "/api/search?query=invoice&mime_types=application/pdf&sort_by=filename&sort_order=asc&label_ids={}",
        fx.label_b
    );
    let (status, body) = get(&ctx, &uri, &fx.user_token).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(ids_of(&body["documents"]), vec![s[5].id]);
    assert_eq!(body["total"], 1);
    let foxtrot = &body["documents"][0];
    assert_eq!(foxtrot["labels"].as_array().unwrap().len(), 2);
    assert_eq!(foxtrot["source_type"], "direct_upload");

    let (_, body) = get(
        &ctx,
        "/api/search?query=invoice&sort_by=file_size&sort_order=asc&limit=50",
        &fx.user_token,
    )
    .await;
    assert_eq!(
        ids_of(&body["documents"]),
        expected_order(&invoice_seeds, DocumentSortField::FileSize, SortOrder::Asc)
    );

    // Short query without filters is still rejected; invalid sort/filter values are 400
    for uri in [
        "/api/search?query=a",
        "/api/search?query=invoice&sort_by=content",
        "/api/search?query=invoice&ocr_status=done",
        "/api/search?query=invoice&label_ids=zzz",
        "/api/search/enhanced?query=a",
        "/api/search/enhanced?query=invoice&sort_order=random",
        "/api/search/enhanced?query=invoice&ocr_status=done",
    ] {
        let (status, _) = get(&ctx, uri, &fx.user_token).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{}", uri);
    }

    // Role isolation in search: the other user sees only their own match, admin sees all
    let (_, body) = get(
        &ctx,
        "/api/search/enhanced?query=invoice&limit=50",
        &fx.other_token,
    )
    .await;
    assert_eq!(
        id_set(&ids_of(&body["documents"])),
        seed_ids([&fx.other_seeds[0]])
    );
    let (_, body) = get(&ctx, "/api/search?query=invoice&limit=50", &fx.other_token).await;
    assert_eq!(
        id_set(&ids_of(&body["documents"])),
        seed_ids([&fx.other_seeds[0]])
    );
    assert_eq!(body["total"], 1);
    let (_, body) = get(&ctx, "/api/search?query=invoice&limit=50", &fx.admin_token).await;
    assert_eq!(body["total"], 5);
    assert_eq!(ids_of(&body["documents"]).len(), 5);
    let (_, body) = get(
        &ctx,
        "/api/search/enhanced?query=invoice&limit=50",
        &fx.admin_token,
    )
    .await;
    assert_eq!(body["total"], 5);

    ctx.cleanup_and_close().await.ok();
}
