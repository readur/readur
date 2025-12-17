//! Integration tests for search pagination functionality.
//!
//! These tests verify that the `count_search_documents` method returns accurate
//! total counts for pagination, ensuring the fix for the pagination bug doesn't regress.

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use readur::test_utils::TestContext;
    use readur::models::{CreateUser, Document, SearchRequest, UserRole};
    use chrono::Utc;
    use uuid::Uuid;
    use std::collections::HashSet;
    use sqlx;

    /// Creates unique test user data with a given suffix for test isolation
    fn create_test_user_data(suffix: &str) -> CreateUser {
        let test_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .to_string();
        let unique_suffix = &test_id[test_id.len().saturating_sub(8)..];

        CreateUser {
            username: format!("testuser_{}_{}", suffix, unique_suffix),
            email: format!("test_{}_{}@example.com", suffix, unique_suffix),
            password: "password123".to_string(),
            role: Some(UserRole::User),
        }
    }

    /// Creates an admin user for role-based access tests
    fn create_admin_user_data(suffix: &str) -> CreateUser {
        let test_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .to_string();
        let unique_suffix = &test_id[test_id.len().saturating_sub(8)..];

        CreateUser {
            username: format!("admin_{}_{}", suffix, unique_suffix),
            email: format!("admin_{}_{}@example.com", suffix, unique_suffix),
            password: "password123".to_string(),
            role: Some(UserRole::Admin),
        }
    }

    /// Creates a searchable document with unique content
    fn create_searchable_document(user_id: Uuid, index: i32, mime_type: &str) -> Document {
        Document {
            id: Uuid::new_v4(),
            filename: format!("test_{}.txt", index),
            original_filename: format!("test_{}.txt", index),
            file_path: format!("/path/to/test_{}.txt", index),
            file_size: 1024,
            mime_type: mime_type.to_string(),
            content: Some(format!("Document {} with searchable content for pagination testing", index)),
            ocr_text: Some(format!("OCR text {} searchable pagination", index)),
            ocr_confidence: Some(95.0),
            ocr_word_count: Some(10),
            ocr_processing_time_ms: Some(800),
            ocr_status: Some("completed".to_string()),
            ocr_error: None,
            ocr_completed_at: Some(Utc::now()),
            tags: vec!["test".to_string(), "pagination".to_string()],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            user_id,
            file_hash: Some(format!("{:x}", Uuid::new_v4().as_u128())),
            original_created_at: None,
            original_modified_at: None,
            source_path: None,
            source_type: None,
            source_id: None,
            file_permissions: None,
            file_owner: None,
            file_group: None,
            source_metadata: None,
            ocr_retry_count: None,
            ocr_failure_reason: None,
        }
    }

    /// Test that count returns actual matching documents, not the limit
    #[tokio::test]
    async fn test_count_matches_actual_documents() {
        let ctx = TestContext::new().await;

        let result: Result<()> = async {
            let db = &ctx.state.db;
            let user = db.create_user(create_test_user_data("count1")).await?;

            // Create 15 documents with searchable content
            for i in 0..15 {
                db.create_document(create_searchable_document(user.id, i, "text/plain")).await?;
            }

            // Search with limit=5
            let request = SearchRequest {
                query: "searchable".to_string(),
                tags: None,
                mime_types: None,
                limit: Some(5),
                offset: Some(0),
                include_snippets: Some(false),
                snippet_length: None,
                search_mode: None,
            };

            let count = db.count_search_documents(user.id, UserRole::User, &request).await?;
            let results = db.search_documents(user.id, &request).await?;

            // Total should be 15, not 5
            assert_eq!(count, 15, "Count should be total matching docs (15), not limit (5)");
            assert_eq!(results.len(), 5, "Results should respect the limit of 5");

            Ok(())
        }.await;

        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }

        result.unwrap();
    }

    /// Test that total remains consistent across all pages
    #[tokio::test]
    async fn test_pagination_total_consistent() {
        let ctx = TestContext::new().await;

        let result: Result<()> = async {
            let db = &ctx.state.db;
            let user = db.create_user(create_test_user_data("consistent1")).await?;

            // Create 20 documents
            for i in 0..20 {
                db.create_document(create_searchable_document(user.id, i, "text/plain")).await?;
            }

            // Check total is same across all pages
            for offset in [0, 5, 10, 15] {
                let request = SearchRequest {
                    query: "searchable".to_string(),
                    tags: None,
                    mime_types: None,
                    limit: Some(5),
                    offset: Some(offset),
                    include_snippets: Some(false),
                    snippet_length: None,
                    search_mode: None,
                };
                let count = db.count_search_documents(user.id, UserRole::User, &request).await?;
                assert_eq!(count, 20, "Total should be consistent (20) at offset {}", offset);
            }

            Ok(())
        }.await;

        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }

        result.unwrap();
    }

    /// Test that iterating through all pages fetches all documents exactly once
    #[tokio::test]
    async fn test_pagination_fetches_all_documents() {
        let ctx = TestContext::new().await;

        let result: Result<()> = async {
            let db = &ctx.state.db;
            let user = db.create_user(create_test_user_data("fetchall1")).await?;

            // Create 17 documents (not evenly divisible by page size)
            for i in 0..17 {
                db.create_document(create_searchable_document(user.id, i, "text/plain")).await?;
            }

            let mut all_ids: HashSet<Uuid> = HashSet::new();
            let page_size = 5i64;

            // Fetch all pages
            for page in 0..4 {
                let request = SearchRequest {
                    query: "searchable".to_string(),
                    tags: None,
                    mime_types: None,
                    limit: Some(page_size),
                    offset: Some(page * page_size),
                    include_snippets: Some(false),
                    snippet_length: None,
                    search_mode: None,
                };
                let results = db.search_documents(user.id, &request).await?;

                for doc in results {
                    let is_new = all_ids.insert(doc.id);
                    assert!(is_new, "Document {} appeared on multiple pages", doc.id);
                }
            }

            assert_eq!(all_ids.len(), 17, "Should have fetched all 17 documents exactly once");

            Ok(())
        }.await;

        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }

        result.unwrap();
    }

    /// Test that count correctly filters by MIME type
    #[tokio::test]
    async fn test_pagination_with_mime_filter() {
        let ctx = TestContext::new().await;

        let result: Result<()> = async {
            let db = &ctx.state.db;
            let user = db.create_user(create_test_user_data("mime1")).await?;

            // Create 10 text/plain documents
            for i in 0..10 {
                db.create_document(create_searchable_document(user.id, i, "text/plain")).await?;
            }

            // Create 5 application/pdf documents
            for i in 10..15 {
                db.create_document(create_searchable_document(user.id, i, "application/pdf")).await?;
            }

            // Filter by text/plain only
            let request = SearchRequest {
                query: "searchable".to_string(),
                tags: None,
                mime_types: Some(vec!["text/plain".to_string()]),
                limit: Some(5),
                offset: Some(0),
                include_snippets: Some(false),
                snippet_length: None,
                search_mode: None,
            };

            let count = db.count_search_documents(user.id, UserRole::User, &request).await?;
            assert_eq!(count, 10, "Count should be 10 (only text/plain), not 15 (all docs)");

            // Filter by PDF only
            let request_pdf = SearchRequest {
                query: "searchable".to_string(),
                tags: None,
                mime_types: Some(vec!["application/pdf".to_string()]),
                limit: Some(5),
                offset: Some(0),
                include_snippets: Some(false),
                snippet_length: None,
                search_mode: None,
            };

            let count_pdf = db.count_search_documents(user.id, UserRole::User, &request_pdf).await?;
            assert_eq!(count_pdf, 5, "Count should be 5 (only PDFs)");

            Ok(())
        }.await;

        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }

        result.unwrap();
    }

    /// Test that count returns 0 when no documents match
    #[tokio::test]
    async fn test_pagination_empty_results() {
        let ctx = TestContext::new().await;

        let result: Result<()> = async {
            let db = &ctx.state.db;
            let user = db.create_user(create_test_user_data("empty1")).await?;

            // Create documents that won't match our query
            for i in 0..5 {
                let mut doc = create_searchable_document(user.id, i, "text/plain");
                doc.content = Some("This content has no matching words".to_string());
                doc.ocr_text = Some("OCR text without matches".to_string());
                db.create_document(doc).await?;
            }

            // Search for something that doesn't exist
            let request = SearchRequest {
                query: "xyznonexistent".to_string(),
                tags: None,
                mime_types: None,
                limit: Some(10),
                offset: Some(0),
                include_snippets: Some(false),
                snippet_length: None,
                search_mode: None,
            };

            let count = db.count_search_documents(user.id, UserRole::User, &request).await?;
            let results = db.search_documents(user.id, &request).await?;

            assert_eq!(count, 0, "Count should be 0 when no matches");
            assert_eq!(results.len(), 0, "Results should be empty when no matches");

            Ok(())
        }.await;

        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }

        result.unwrap();
    }

    /// Test that the last page returns remaining documents correctly
    #[tokio::test]
    async fn test_pagination_boundary_last_page() {
        let ctx = TestContext::new().await;

        let result: Result<()> = async {
            let db = &ctx.state.db;
            let user = db.create_user(create_test_user_data("boundary1")).await?;

            // Create 13 documents (13 % 5 = 3 on last page)
            for i in 0..13 {
                db.create_document(create_searchable_document(user.id, i, "text/plain")).await?;
            }

            // Request last page (offset 10, should return 3 docs)
            let request = SearchRequest {
                query: "searchable".to_string(),
                tags: None,
                mime_types: None,
                limit: Some(5),
                offset: Some(10),
                include_snippets: Some(false),
                snippet_length: None,
                search_mode: None,
            };

            let count = db.count_search_documents(user.id, UserRole::User, &request).await?;
            let results = db.search_documents(user.id, &request).await?;

            assert_eq!(count, 13, "Total count should still be 13");
            assert_eq!(results.len(), 3, "Last page should have 3 remaining documents");

            Ok(())
        }.await;

        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }

        result.unwrap();
    }

    /// Test that count is unaffected by limit/offset values
    #[tokio::test]
    async fn test_count_ignores_limit_offset() {
        let ctx = TestContext::new().await;

        let result: Result<()> = async {
            let db = &ctx.state.db;
            let user = db.create_user(create_test_user_data("ignore1")).await?;

            // Create 25 documents
            for i in 0..25 {
                db.create_document(create_searchable_document(user.id, i, "text/plain")).await?;
            }

            // Test with various limit/offset combinations
            let test_cases = vec![
                (1, 0),    // Tiny limit
                (100, 0),  // Large limit
                (5, 0),    // First page
                (5, 20),   // Near last page
                (5, 100),  // Past end
            ];

            for (limit, offset) in test_cases {
                let request = SearchRequest {
                    query: "searchable".to_string(),
                    tags: None,
                    mime_types: None,
                    limit: Some(limit),
                    offset: Some(offset),
                    include_snippets: Some(false),
                    snippet_length: None,
                    search_mode: None,
                };

                let count = db.count_search_documents(user.id, UserRole::User, &request).await?;
                assert_eq!(count, 25, "Count should always be 25 regardless of limit={}, offset={}", limit, offset);
            }

            Ok(())
        }.await;

        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }

        result.unwrap();
    }

    /// Test role-based access: users see only their own documents, admins see all
    #[tokio::test]
    async fn test_admin_sees_all_user_sees_own() {
        let ctx = TestContext::new().await;

        let result: Result<()> = async {
            let db = &ctx.state.db;

            // Create two regular users
            let user_a = db.create_user(create_test_user_data("usera")).await?;
            let user_b = db.create_user(create_test_user_data("userb")).await?;
            let admin = db.create_user(create_admin_user_data("admin")).await?;

            // Create 10 documents for user A
            for i in 0..10 {
                db.create_document(create_searchable_document(user_a.id, i, "text/plain")).await?;
            }

            // Create 5 documents for user B
            for i in 10..15 {
                db.create_document(create_searchable_document(user_b.id, i, "text/plain")).await?;
            }

            let request = SearchRequest {
                query: "searchable".to_string(),
                tags: None,
                mime_types: None,
                limit: Some(100),
                offset: Some(0),
                include_snippets: Some(false),
                snippet_length: None,
                search_mode: None,
            };

            // User A should see only their 10 documents
            let count_a = db.count_search_documents(user_a.id, UserRole::User, &request).await?;
            assert_eq!(count_a, 10, "User A should see only their 10 documents");

            // User B should see only their 5 documents
            let count_b = db.count_search_documents(user_b.id, UserRole::User, &request).await?;
            assert_eq!(count_b, 5, "User B should see only their 5 documents");

            // Admin should see all 15 documents
            let count_admin = db.count_search_documents(admin.id, UserRole::Admin, &request).await?;
            assert_eq!(count_admin, 15, "Admin should see all 15 documents");

            Ok(())
        }.await;

        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }

        result.unwrap();
    }

    /// Test pagination with text query filtering
    #[tokio::test]
    async fn test_pagination_with_text_query() {
        let ctx = TestContext::new().await;

        let result: Result<()> = async {
            let db = &ctx.state.db;
            let user = db.create_user(create_test_user_data("textq1")).await?;

            // Create documents with different content
            for i in 0..10 {
                let mut doc = create_searchable_document(user.id, i, "text/plain");
                if i < 6 {
                    doc.content = Some(format!("Document {} contains the word apple", i));
                } else {
                    doc.content = Some(format!("Document {} contains the word orange", i));
                }
                db.create_document(doc).await?;
            }

            // Search for "apple"
            let request_apple = SearchRequest {
                query: "apple".to_string(),
                tags: None,
                mime_types: None,
                limit: Some(3),
                offset: Some(0),
                include_snippets: Some(false),
                snippet_length: None,
                search_mode: None,
            };

            let count_apple = db.count_search_documents(user.id, UserRole::User, &request_apple).await?;
            let results_apple = db.search_documents(user.id, &request_apple).await?;

            assert_eq!(count_apple, 6, "Should find 6 documents with 'apple'");
            assert_eq!(results_apple.len(), 3, "Should return 3 (limit)");

            // Search for "orange"
            let request_orange = SearchRequest {
                query: "orange".to_string(),
                tags: None,
                mime_types: None,
                limit: Some(10),
                offset: Some(0),
                include_snippets: Some(false),
                snippet_length: None,
                search_mode: None,
            };

            let count_orange = db.count_search_documents(user.id, UserRole::User, &request_orange).await?;
            assert_eq!(count_orange, 4, "Should find 4 documents with 'orange'");

            Ok(())
        }.await;

        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }

        result.unwrap();
    }

    /// Test that count correctly filters by labels
    #[tokio::test]
    async fn test_pagination_with_label_filter() {
        let ctx = TestContext::new().await;

        let result: Result<()> = async {
            let db = &ctx.state.db;
            let user = db.create_user(create_test_user_data("label1")).await?;

            // Create a label using direct SQL (no db.create_label method exists)
            let label_id = Uuid::new_v4();
            sqlx::query(
                r#"
                INSERT INTO labels (id, user_id, name, description, color, is_system)
                VALUES ($1, $2, $3, $4, $5, $6)
                "#
            )
            .bind(label_id)
            .bind(user.id)
            .bind("important")
            .bind("Important documents")
            .bind("#ff0000")
            .bind(false)
            .execute(db.get_pool())
            .await?;

            // Create 10 documents, assign label to 6 of them
            for i in 0..10 {
                let doc = db.create_document(create_searchable_document(user.id, i, "text/plain")).await?;
                if i < 6 {
                    // Assign label to first 6 documents
                    sqlx::query(
                        "INSERT INTO document_labels (document_id, label_id, assigned_by) VALUES ($1, $2, $3)"
                    )
                    .bind(doc.id)
                    .bind(label_id)
                    .bind(user.id)
                    .execute(db.get_pool())
                    .await?;
                }
            }

            // Filter by label name
            let request = SearchRequest {
                query: "searchable".to_string(),
                tags: Some(vec!["important".to_string()]),
                mime_types: None,
                limit: Some(3),
                offset: Some(0),
                include_snippets: Some(false),
                snippet_length: None,
                search_mode: None,
            };

            let count = db.count_search_documents(user.id, UserRole::User, &request).await?;
            let results = db.search_documents(user.id, &request).await?;

            assert_eq!(count, 6, "Count should be 6 (only labeled docs), not 10 (all docs)");
            assert_eq!(results.len(), 3, "Results should respect limit of 3");

            // Test with non-existent label
            let request_none = SearchRequest {
                query: "searchable".to_string(),
                tags: Some(vec!["nonexistent".to_string()]),
                mime_types: None,
                limit: Some(3),
                offset: Some(0),
                include_snippets: Some(false),
                snippet_length: None,
                search_mode: None,
            };

            let count_none = db.count_search_documents(user.id, UserRole::User, &request_none).await?;
            assert_eq!(count_none, 0, "Count should be 0 for non-existent label");

            Ok(())
        }.await;

        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }

        result.unwrap();
    }

    /// Test filter-only search (empty query with MIME filter)
    #[tokio::test]
    async fn test_pagination_filter_only_no_query() {
        let ctx = TestContext::new().await;

        let result: Result<()> = async {
            let db = &ctx.state.db;
            let user = db.create_user(create_test_user_data("filteronly1")).await?;

            // Create mixed documents
            for i in 0..8 {
                db.create_document(create_searchable_document(user.id, i, "text/plain")).await?;
            }
            for i in 8..12 {
                db.create_document(create_searchable_document(user.id, i, "image/png")).await?;
            }

            // Filter by MIME type only (no text query)
            let request = SearchRequest {
                query: String::new(), // Empty query
                tags: None,
                mime_types: Some(vec!["image/png".to_string()]),
                limit: Some(2),
                offset: Some(0),
                include_snippets: Some(false),
                snippet_length: None,
                search_mode: None,
            };

            let count = db.count_search_documents(user.id, UserRole::User, &request).await?;
            assert_eq!(count, 4, "Should count 4 PNG images with empty query");

            Ok(())
        }.await;

        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }

        result.unwrap();
    }
}
