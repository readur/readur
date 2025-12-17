use anyhow::Result;
use sqlx::{QueryBuilder, Postgres, Row};
use uuid::Uuid;

use crate::models::{Document, UserRole, SearchRequest, SearchMode, SearchSnippet, HighlightRange, EnhancedDocumentResponse};
use super::helpers::{map_row_to_document, apply_role_based_filter, apply_pagination, find_word_boundary, DOCUMENT_FIELDS};
use crate::db::Database;

impl Database {
    /// Performs basic document search with PostgreSQL full-text search
    pub async fn search_documents(&self, user_id: Uuid, search_request: &SearchRequest) -> Result<Vec<Document>> {
        let mut query = QueryBuilder::<Postgres>::new("SELECT ");
        query.push(DOCUMENT_FIELDS);
        query.push(" FROM documents WHERE user_id = ");
        query.push_bind(user_id);

        // Add search conditions
        if !search_request.query.trim().is_empty() {
            query.push(" AND (to_tsvector('english', COALESCE(content, '')) @@ plainto_tsquery('english', ");
            query.push_bind(&search_request.query);
            query.push(") OR to_tsvector('english', COALESCE(ocr_text, '')) @@ plainto_tsquery('english', ");
            query.push_bind(&search_request.query);
            query.push("))");
        }

        // Add label filtering (tags param contains label names)
        if let Some(ref tags) = search_request.tags {
            if !tags.is_empty() {
                query.push(" AND documents.id IN (SELECT dl.document_id FROM document_labels dl JOIN labels l ON dl.label_id = l.id WHERE l.name = ANY(");
                query.push_bind(tags);
                query.push("))");
            }
        }

        // Add MIME type filtering
        if let Some(ref mime_types) = search_request.mime_types {
            if !mime_types.is_empty() {
                query.push(" AND mime_type = ANY(");
                query.push_bind(mime_types);
                query.push(")");
            }
        }

        query.push(" ORDER BY created_at DESC");
        
        let limit = search_request.limit.unwrap_or(25);
        let offset = search_request.offset.unwrap_or(0);
        apply_pagination(&mut query, limit, offset);

        let rows = query.build().fetch_all(&self.pool).await?;
        Ok(rows.iter().map(map_row_to_document).collect())
    }

    /// Enhanced search with snippets and ranking
    pub async fn enhanced_search_documents(&self, user_id: Uuid, search_request: &SearchRequest) -> Result<Vec<EnhancedDocumentResponse>> {
        self.enhanced_search_documents_with_role(user_id, UserRole::User, search_request).await
    }

    /// Enhanced search with role-based access control
    pub async fn enhanced_search_documents_with_role(&self, user_id: Uuid, user_role: UserRole, search_request: &SearchRequest) -> Result<Vec<EnhancedDocumentResponse>> {
        let search_query = search_request.query.trim();
        let include_snippets = search_request.include_snippets.unwrap_or(true);
        let snippet_length = search_request.snippet_length.unwrap_or(200) as usize;

        let mut query = QueryBuilder::<Postgres>::new("SELECT ");
        query.push(DOCUMENT_FIELDS);
        
        // Add search ranking if there's a query
        if !search_query.is_empty() {
            match search_request.search_mode.as_ref().unwrap_or(&SearchMode::Simple) {
                SearchMode::Simple => {
                    query.push(", ts_rank(to_tsvector('english', COALESCE(content, '') || ' ' || COALESCE(ocr_text, '')), plainto_tsquery('english', ");
                    query.push_bind(search_query);
                    query.push(")) as search_rank");
                }
                SearchMode::Phrase => {
                    query.push(", ts_rank(to_tsvector('english', COALESCE(content, '') || ' ' || COALESCE(ocr_text, '')), phraseto_tsquery('english', ");
                    query.push_bind(search_query);
                    query.push(")) as search_rank");
                }
                SearchMode::Boolean => {
                    query.push(", ts_rank(to_tsvector('english', COALESCE(content, '') || ' ' || COALESCE(ocr_text, '')), to_tsquery('english', ");
                    query.push_bind(search_query);
                    query.push(")) as search_rank");
                }
                SearchMode::Fuzzy => {
                    query.push(", similarity(COALESCE(content, '') || ' ' || COALESCE(ocr_text, ''), ");
                    query.push_bind(search_query);
                    query.push(") as search_rank");
                }
            }
        } else {
            query.push(", 0.0 as search_rank");
        }

        query.push(" FROM documents WHERE 1=1");

        apply_role_based_filter(&mut query, user_id, user_role);

        // Add search conditions
        if !search_query.is_empty() {
            match search_request.search_mode.as_ref().unwrap_or(&SearchMode::Simple) {
                SearchMode::Simple => {
                    query.push(" AND (to_tsvector('english', COALESCE(content, '')) @@ plainto_tsquery('english', ");
                    query.push_bind(search_query);
                    query.push(") OR to_tsvector('english', COALESCE(ocr_text, '')) @@ plainto_tsquery('english', ");
                    query.push_bind(search_query);
                    query.push("))");
                }
                SearchMode::Phrase => {
                    query.push(" AND (to_tsvector('english', COALESCE(content, '')) @@ phraseto_tsquery('english', ");
                    query.push_bind(search_query);
                    query.push(") OR to_tsvector('english', COALESCE(ocr_text, '')) @@ phraseto_tsquery('english', ");
                    query.push_bind(search_query);
                    query.push("))");
                }
                SearchMode::Boolean => {
                    query.push(" AND (to_tsvector('english', COALESCE(content, '')) @@ to_tsquery('english', ");
                    query.push_bind(search_query);
                    query.push(") OR to_tsvector('english', COALESCE(ocr_text, '')) @@ to_tsquery('english', ");
                    query.push_bind(search_query);
                    query.push("))");
                }
                SearchMode::Fuzzy => {
                    query.push(" AND similarity(COALESCE(content, '') || ' ' || COALESCE(ocr_text, ''), ");
                    query.push_bind(search_query);
                    query.push(") > 0.3");
                }
            }
        }

        // Add label filtering (tags param contains label names)
        if let Some(ref tags) = search_request.tags {
            if !tags.is_empty() {
                query.push(" AND documents.id IN (SELECT dl.document_id FROM document_labels dl JOIN labels l ON dl.label_id = l.id WHERE l.name = ANY(");
                query.push_bind(tags);
                query.push("))");
            }
        }

        if let Some(ref mime_types) = search_request.mime_types {
            if !mime_types.is_empty() {
                query.push(" AND mime_type = ANY(");
                query.push_bind(mime_types);
                query.push(")");
            }
        }

        query.push(" ORDER BY search_rank DESC, created_at DESC");
        
        let limit = search_request.limit.unwrap_or(25);
        let offset = search_request.offset.unwrap_or(0);
        apply_pagination(&mut query, limit, offset);

        let rows = query.build().fetch_all(&self.pool).await?;

        let mut results = Vec::new();
        for row in rows {
            let document = map_row_to_document(&row);
            let search_rank: f32 = row.try_get("search_rank").unwrap_or(0.0);

            let snippets = if include_snippets && !search_query.is_empty() {
                self.generate_snippets(&document, search_query, snippet_length).await
            } else {
                Vec::new()
            };

            results.push(EnhancedDocumentResponse {
                id: document.id,
                filename: document.filename,
                original_filename: document.original_filename,
                file_size: document.file_size,
                mime_type: document.mime_type,
                tags: document.tags,
                created_at: document.created_at,
                has_ocr_text: document.ocr_text.is_some(),
                ocr_confidence: document.ocr_confidence,
                ocr_word_count: document.ocr_word_count,
                ocr_processing_time_ms: document.ocr_processing_time_ms,
                ocr_status: document.ocr_status,
                search_rank: Some(search_rank),
                snippets,
            });
        }

        Ok(results)
    }

    /// Generates search snippets with highlighted matches
    pub async fn generate_snippets(&self, document: &Document, search_query: &str, snippet_length: usize) -> Vec<SearchSnippet> {
        let mut snippets = Vec::new();
        let search_terms: Vec<&str> = search_query.split_whitespace().collect();

        // Search in content and OCR text
        let texts = vec![
            ("content", document.content.as_deref().unwrap_or("")),
            ("ocr_text", document.ocr_text.as_deref().unwrap_or(""))
        ];

        for (_source, text) in texts {
            if text.is_empty() {
                continue;
            }

            let text_lower = text.to_lowercase();
            for term in &search_terms {
                let term_lower = term.to_lowercase();
                let mut start_pos = 0;

                while let Some(match_pos) = text_lower[start_pos..].find(&term_lower) {
                    let absolute_match_pos = start_pos + match_pos;
                    
                    // Calculate snippet boundaries
                    let snippet_start = if absolute_match_pos >= snippet_length / 2 {
                        find_word_boundary(text, absolute_match_pos - snippet_length / 2, false)
                    } else {
                        0
                    };

                    let snippet_end = {
                        let desired_end = snippet_start + snippet_length;
                        if desired_end < text.len() {
                            find_word_boundary(text, desired_end, true)
                        } else {
                            text.len()
                        }
                    };

                    let snippet_text = &text[snippet_start..snippet_end];
                    
                    // Calculate highlight range relative to snippet
                    let highlight_start = absolute_match_pos - snippet_start;
                    let highlight_end = highlight_start + term.len();

                    let highlight_ranges = vec![HighlightRange {
                        start: highlight_start as i32,
                        end: highlight_end as i32,
                    }];

                    snippets.push(SearchSnippet {
                        text: snippet_text.to_string(),
                        start_offset: snippet_start as i32,
                        end_offset: snippet_end as i32,
                        highlight_ranges,
                    });

                    start_pos = absolute_match_pos + term.len();
                    
                    // Limit snippets per term
                    if snippets.len() >= 3 {
                        break;
                    }
                }
            }
        }

        // Remove duplicates and limit total snippets
        snippets.truncate(5);
        snippets
    }

    /// Counts total matching documents for pagination (without applying LIMIT/OFFSET)
    pub async fn count_search_documents(&self, user_id: Uuid, user_role: UserRole, search_request: &SearchRequest) -> Result<i64> {
        let search_query = search_request.query.trim();

        let mut query = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM documents WHERE 1=1");

        apply_role_based_filter(&mut query, user_id, user_role);

        // Add search conditions (same as enhanced_search_documents_with_role)
        if !search_query.is_empty() {
            match search_request.search_mode.as_ref().unwrap_or(&SearchMode::Simple) {
                SearchMode::Simple => {
                    query.push(" AND (to_tsvector('english', COALESCE(content, '')) @@ plainto_tsquery('english', ");
                    query.push_bind(search_query);
                    query.push(") OR to_tsvector('english', COALESCE(ocr_text, '')) @@ plainto_tsquery('english', ");
                    query.push_bind(search_query);
                    query.push("))");
                }
                SearchMode::Phrase => {
                    query.push(" AND (to_tsvector('english', COALESCE(content, '')) @@ phraseto_tsquery('english', ");
                    query.push_bind(search_query);
                    query.push(") OR to_tsvector('english', COALESCE(ocr_text, '')) @@ phraseto_tsquery('english', ");
                    query.push_bind(search_query);
                    query.push("))");
                }
                SearchMode::Boolean => {
                    query.push(" AND (to_tsvector('english', COALESCE(content, '')) @@ to_tsquery('english', ");
                    query.push_bind(search_query);
                    query.push(") OR to_tsvector('english', COALESCE(ocr_text, '')) @@ to_tsquery('english', ");
                    query.push_bind(search_query);
                    query.push("))");
                }
                SearchMode::Fuzzy => {
                    query.push(" AND similarity(COALESCE(content, '') || ' ' || COALESCE(ocr_text, ''), ");
                    query.push_bind(search_query);
                    query.push(") > 0.3");
                }
            }
        }

        // Add label filtering (tags param contains label names)
        if let Some(ref tags) = search_request.tags {
            if !tags.is_empty() {
                query.push(" AND documents.id IN (SELECT dl.document_id FROM document_labels dl JOIN labels l ON dl.label_id = l.id WHERE l.name = ANY(");
                query.push_bind(tags);
                query.push("))");
            }
        }

        // Add MIME type filtering
        if let Some(ref mime_types) = search_request.mime_types {
            if !mime_types.is_empty() {
                query.push(" AND mime_type = ANY(");
                query.push_bind(mime_types);
                query.push(")");
            }
        }

        let row: (i64,) = query.build_query_as().fetch_one(&self.pool).await?;
        Ok(row.0)
    }
}