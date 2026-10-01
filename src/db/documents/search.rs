use anyhow::Result;
use sqlx::{Postgres, QueryBuilder, Row};
use uuid::Uuid;

use super::helpers::{
    apply_document_filters, apply_pagination, apply_role_based_filter, apply_sort,
    find_word_boundary, map_row_to_document_with_progress, DocumentWithProgress, DOCUMENT_FIELDS,
    OCR_PROGRESS_FIELDS, OCR_PROGRESS_JOIN,
};
use crate::db::Database;
use crate::models::{
    Document, EnhancedDocumentResponse, HighlightRange, MonthCount, SearchMode, SearchRequest,
    SearchSnippet, UserRole,
};

/// SQL expression building the tsquery (or similarity operand) for a search mode.
fn tsquery_function(mode: &SearchMode) -> &'static str {
    match mode {
        SearchMode::Simple => "plainto_tsquery",
        SearchMode::Phrase => "phraseto_tsquery",
        SearchMode::Boolean => "to_tsquery",
        // Fuzzy search does not use a tsquery
        SearchMode::Fuzzy => "",
    }
}

/// Appends the `, <rank> as search_rank` select column for a search query.
fn push_search_rank(query: &mut QueryBuilder<Postgres>, search_query: &str, mode: &SearchMode) {
    match mode {
        SearchMode::Fuzzy => {
            query.push(", similarity(COALESCE(content, '') || ' ' || COALESCE(ocr_text, ''), ");
            query.push_bind(search_query.to_string());
            query.push(") as search_rank");
        }
        _ => {
            query.push(", ts_rank(to_tsvector('english', COALESCE(content, '') || ' ' || COALESCE(ocr_text, '')), ");
            query.push(tsquery_function(mode));
            query.push("('english', ");
            query.push_bind(search_query.to_string());
            query.push(")) as search_rank");
        }
    }
}

/// Appends the `AND <match>` condition for a non-empty search query.
fn push_search_condition(
    query: &mut QueryBuilder<Postgres>,
    search_query: &str,
    mode: &SearchMode,
) {
    match mode {
        SearchMode::Fuzzy => {
            query.push(" AND similarity(COALESCE(content, '') || ' ' || COALESCE(ocr_text, ''), ");
            query.push_bind(search_query.to_string());
            query.push(") > 0.3");
        }
        _ => {
            let func = tsquery_function(mode);
            query.push(" AND (to_tsvector('english', COALESCE(content, '')) @@ ");
            query.push(func);
            query.push("('english', ");
            query.push_bind(search_query.to_string());
            query.push(") OR to_tsvector('english', COALESCE(ocr_text, '')) @@ ");
            query.push(func);
            query.push("('english', ");
            query.push_bind(search_query.to_string());
            query.push("))");
        }
    }
}

/// Appends the visibility, text-match and filter conditions shared by the
/// search count and the search timeline.
fn push_search_scope(
    query: &mut QueryBuilder<Postgres>,
    user_id: Uuid,
    user_role: UserRole,
    search_request: &SearchRequest,
    mode: &SearchMode,
) {
    apply_role_based_filter(query, user_id, user_role);
    let search_query = search_request.query.trim();
    if !search_query.is_empty() {
        push_search_condition(query, search_query, mode);
    }
    apply_document_filters(query, &search_request.filters());
}

impl Database {
    /// Performs basic document search with PostgreSQL full-text search.
    /// Only the user's own documents are searched; see `search_documents_with_role`.
    pub async fn search_documents(
        &self,
        user_id: Uuid,
        search_request: &SearchRequest,
    ) -> Result<Vec<Document>> {
        Ok(self
            .search_documents_with_role(user_id, UserRole::User, search_request)
            .await?
            .into_iter()
            .map(|d| d.document)
            .collect())
    }

    /// Basic full-text search (simple mode) with role-based access, filters and sorting.
    /// Defaults to newest first when no sort is given.
    pub async fn search_documents_with_role(
        &self,
        user_id: Uuid,
        user_role: UserRole,
        search_request: &SearchRequest,
    ) -> Result<Vec<DocumentWithProgress>> {
        let mut query = QueryBuilder::<Postgres>::new("SELECT ");
        query.push(DOCUMENT_FIELDS);
        query.push(OCR_PROGRESS_FIELDS);
        query.push(" FROM documents");
        query.push(OCR_PROGRESS_JOIN);
        query.push(" WHERE 1=1");

        apply_role_based_filter(&mut query, user_id, user_role);

        let search_query = search_request.query.trim();
        if !search_query.is_empty() {
            push_search_condition(&mut query, search_query, &SearchMode::Simple);
        }

        apply_document_filters(&mut query, &search_request.filters());
        apply_sort(
            &mut query,
            search_request.sort_by,
            search_request.sort_order,
            false,
        );

        let limit = search_request.limit.unwrap_or(25);
        let offset = search_request.offset.unwrap_or(0);
        apply_pagination(&mut query, limit, offset);

        let rows = query.build().fetch_all(&self.pool).await?;
        Ok(rows.iter().map(map_row_to_document_with_progress).collect())
    }

    /// Enhanced search with snippets and ranking
    pub async fn enhanced_search_documents(
        &self,
        user_id: Uuid,
        search_request: &SearchRequest,
    ) -> Result<Vec<EnhancedDocumentResponse>> {
        self.enhanced_search_documents_with_role(user_id, UserRole::User, search_request)
            .await
    }

    /// Enhanced search with role-based access control, filters and sorting.
    /// Defaults to relevance order when no sort is given.
    pub async fn enhanced_search_documents_with_role(
        &self,
        user_id: Uuid,
        user_role: UserRole,
        search_request: &SearchRequest,
    ) -> Result<Vec<EnhancedDocumentResponse>> {
        let search_query = search_request.query.trim();
        let include_snippets = search_request.include_snippets.unwrap_or(true);
        let snippet_length = search_request.snippet_length.unwrap_or(200) as usize;
        let mode = search_request
            .search_mode
            .as_ref()
            .unwrap_or(&SearchMode::Simple);

        let mut query = QueryBuilder::<Postgres>::new("SELECT ");
        query.push(DOCUMENT_FIELDS);
        query.push(OCR_PROGRESS_FIELDS);

        // Add search ranking if there's a query
        if !search_query.is_empty() {
            push_search_rank(&mut query, search_query, mode);
        } else {
            query.push(", 0.0 as search_rank");
        }

        query.push(" FROM documents");
        query.push(OCR_PROGRESS_JOIN);
        query.push(" WHERE 1=1");

        apply_role_based_filter(&mut query, user_id, user_role);

        if !search_query.is_empty() {
            push_search_condition(&mut query, search_query, mode);
        }

        apply_document_filters(&mut query, &search_request.filters());
        apply_sort(
            &mut query,
            search_request.sort_by,
            search_request.sort_order,
            !search_query.is_empty(),
        );

        let limit = search_request.limit.unwrap_or(25);
        let offset = search_request.offset.unwrap_or(0);
        apply_pagination(&mut query, limit, offset);

        let rows = query.build().fetch_all(&self.pool).await?;

        let mut results = Vec::new();
        for row in rows {
            let item = map_row_to_document_with_progress(&row);
            let search_rank: f32 = row.try_get("search_rank").unwrap_or(0.0);

            let snippets = if include_snippets && !search_query.is_empty() {
                self.generate_snippets(&item.document, search_query, snippet_length)
                    .await
            } else {
                Vec::new()
            };

            results.push(EnhancedDocumentResponse::from_document(
                item.document,
                Some(search_rank),
                snippets,
                item.ocr_progress_current,
                item.ocr_progress_total,
            ));
        }

        self.attach_labels_to_search_results(&mut results).await?;

        Ok(results)
    }

    /// Batch-loads labels onto search results.
    pub async fn attach_labels_to_search_results(
        &self,
        results: &mut [EnhancedDocumentResponse],
    ) -> Result<()> {
        if results.is_empty() {
            return Ok(());
        }
        let ids: Vec<Uuid> = results.iter().map(|r| r.id).collect();
        let mut labels: std::collections::HashMap<Uuid, _> = self
            .get_labels_for_documents(&ids)
            .await?
            .into_iter()
            .collect();
        for result in results.iter_mut() {
            if let Some(doc_labels) = labels.remove(&result.id) {
                result.labels = doc_labels;
            }
        }
        Ok(())
    }

    /// Generates search snippets with highlighted matches
    pub async fn generate_snippets(
        &self,
        document: &Document,
        search_query: &str,
        snippet_length: usize,
    ) -> Vec<SearchSnippet> {
        let mut snippets = Vec::new();
        let search_terms: Vec<&str> = search_query.split_whitespace().collect();

        // Search in content and OCR text
        let texts = vec![
            ("content", document.content.as_deref().unwrap_or("")),
            ("ocr_text", document.ocr_text.as_deref().unwrap_or("")),
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

    /// Counts total matching documents for pagination (without applying LIMIT/OFFSET),
    /// using the request's `search_mode` (simple when unset).
    pub async fn count_search_documents(
        &self,
        user_id: Uuid,
        user_role: UserRole,
        search_request: &SearchRequest,
    ) -> Result<i64> {
        let mode = search_request
            .search_mode
            .as_ref()
            .unwrap_or(&SearchMode::Simple);
        self.count_search_documents_in_mode(user_id, user_role, search_request, mode)
            .await
    }

    /// Counts total matching documents in the given mode, ignoring the
    /// request's `search_mode`. The basic search list always matches in simple
    /// mode, so its count must too.
    pub async fn count_search_documents_in_mode(
        &self,
        user_id: Uuid,
        user_role: UserRole,
        search_request: &SearchRequest,
        mode: &SearchMode,
    ) -> Result<i64> {
        let mut query = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM documents WHERE 1=1");
        push_search_scope(&mut query, user_id, user_role, search_request, mode);

        let row: (i64,) = query.build_query_as().fetch_one(&self.pool).await?;
        Ok(row.0)
    }

    /// Counts every document matching the search (same predicate as
    /// `count_search_documents`) per UTC creation month, ascending. Months
    /// without matches are omitted.
    pub async fn search_timeline(
        &self,
        user_id: Uuid,
        user_role: UserRole,
        search_request: &SearchRequest,
    ) -> Result<Vec<MonthCount>> {
        let mode = search_request
            .search_mode
            .as_ref()
            .unwrap_or(&SearchMode::Simple);
        let mut query = QueryBuilder::<Postgres>::new(
            "SELECT to_char(date_trunc('month', created_at AT TIME ZONE 'UTC'), 'YYYY-MM') AS month, \
             COUNT(*) AS count FROM documents WHERE 1=1",
        );
        push_search_scope(&mut query, user_id, user_role, search_request, mode);
        query.push(" GROUP BY 1 ORDER BY 1");

        let rows: Vec<(String, i64)> = query.build_query_as().fetch_all(&self.pool).await?;
        Ok(rows
            .into_iter()
            .map(|(month, count)| MonthCount { month, count })
            .collect())
    }
}
