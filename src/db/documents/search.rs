use anyhow::Result;
use sqlx::{Postgres, QueryBuilder, Row, Transaction};
use uuid::Uuid;

use super::helpers::{
    apply_document_filters, apply_pagination, apply_role_based_filter, apply_sort,
    map_row_to_document_with_progress, DocumentWithProgress, DOCUMENT_FIELDS, OCR_PROGRESS_FIELDS,
    OCR_PROGRESS_JOIN,
};
use super::search_query::BooleanQuery;
use super::snippets::build_snippets;
use crate::db::sanitize::strip_nul;
use crate::db::Database;
use crate::models::{
    Document, EnhancedDocumentResponse, FuzzyThreshold, MonthCount, SearchMode, SearchRequest,
    SearchSnippet, UserRole,
};

/// A document's searchable text (content and OCR text) as one tsvector. It
/// matches the expression of the `idx_documents_content_search` GIN index, so
/// the planner can use it. Matching the whole document at once also keeps
/// negation meaningful: matched per column, `!draft` would succeed on a
/// document's empty content column even when its OCR text says "draft".
const DOCUMENT_TSVECTOR: &str =
    "to_tsvector('english', COALESCE(content, '') || ' ' || COALESCE(ocr_text, ''))";

/// A document's searchable text as one string, for fuzzy matching. It is
/// exactly the expression of the `idx_documents_content_trgm` trigram index,
/// so `<%` can be answered from that index instead of scanning every row.
const DOCUMENT_TEXT: &str = "(COALESCE(content, '') || ' ' || COALESCE(ocr_text, ''))";

/// SQL expression building the tsquery (or similarity operand) for a search mode.
fn tsquery_function(mode: &SearchMode) -> &'static str {
    match mode {
        SearchMode::Simple => "plainto_tsquery",
        SearchMode::Phrase => "phraseto_tsquery",
        // Boolean input is parsed and re-rendered by `BooleanQuery`, whose
        // output is always valid `to_tsquery` syntax.
        SearchMode::Boolean => "to_tsquery",
        // Fuzzy search does not use a tsquery
        SearchMode::Fuzzy => "",
    }
}

/// The text bound as the tsquery function's argument.
fn tsquery_input(search_query: &str, mode: &SearchMode) -> String {
    match mode {
        SearchMode::Boolean => BooleanQuery::parse(search_query).to_tsquery(),
        _ => strip_nul(search_query).into_owned(),
    }
}

/// The words and phrases to highlight in snippets for `search_query`. Boolean
/// queries contribute only their non-negated terms, without operators;
/// other modes split on whitespace and drop surrounding double quotes.
pub(crate) fn highlight_terms(search_query: &str, mode: &SearchMode) -> Vec<String> {
    match mode {
        SearchMode::Boolean => BooleanQuery::parse(search_query).positive_terms(),
        _ => search_query
            .split_whitespace()
            .map(|term| term.trim_matches('"'))
            .filter(|term| !term.is_empty())
            .map(String::from)
            .collect(),
    }
}

/// Appends the `, <rank> as search_rank` select column for a search query.
fn push_search_rank(query: &mut QueryBuilder<Postgres>, search_query: &str, mode: &SearchMode) {
    match mode {
        // How closely the best-matching stretch of the text is spelled like
        // the query (1.0 when the query's words appear as written).
        SearchMode::Fuzzy => {
            query.push(", word_similarity(");
            query.push_bind(strip_nul(search_query).into_owned());
            query.push(", ");
            query.push(DOCUMENT_TEXT);
            query.push(") as search_rank");
        }
        _ => {
            query.push(", ts_rank(");
            query.push(DOCUMENT_TSVECTOR);
            query.push(", ");
            query.push(tsquery_function(mode));
            query.push("('english', ");
            query.push_bind(tsquery_input(search_query, mode));
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
        // Word-level: the query against the closest stretch of the text,
        // not the whole text, so a long document containing a near-miss of
        // the query matches. The cutoff is `pg_trgm.word_similarity_threshold`,
        // set per search by `Database::begin_search`.
        SearchMode::Fuzzy => {
            query.push(" AND ");
            query.push_bind(strip_nul(search_query).into_owned());
            query.push(" <% ");
            query.push(DOCUMENT_TEXT);
        }
        _ => {
            query.push(" AND ");
            query.push(DOCUMENT_TSVECTOR);
            query.push(" @@ ");
            query.push(tsquery_function(mode));
            query.push("('english', ");
            query.push_bind(tsquery_input(search_query, mode));
            query.push(")");
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

/// The request's search mode (simple when unset).
fn request_mode(search_request: &SearchRequest) -> &SearchMode {
    search_request
        .search_mode
        .as_ref()
        .unwrap_or(&SearchMode::Simple)
}

impl Database {
    /// Opens the transaction a search runs in. In fuzzy mode it first sets
    /// `pg_trgm.word_similarity_threshold` to `fuzzy`: the `<%` operator,
    /// which the trigram index can serve, takes its cutoff from that setting
    /// rather than from an argument. `set_config(.., true)` scopes it to this
    /// transaction, so it never leaks to the next user of the pooled
    /// connection, and every query of the search sees the same cutoff.
    async fn begin_search(
        &self,
        mode: &SearchMode,
        fuzzy: FuzzyThreshold,
    ) -> Result<Transaction<'_, Postgres>> {
        let mut tx = self.pool.begin().await?;
        if matches!(mode, SearchMode::Fuzzy) {
            sqlx::query("SELECT set_config('pg_trgm.word_similarity_threshold', $1, true)")
                .bind(fuzzy.value().to_string())
                .execute(&mut *tx)
                .await?;
        }
        Ok(tx)
    }

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
        fuzzy: FuzzyThreshold,
    ) -> Result<Vec<EnhancedDocumentResponse>> {
        self.enhanced_search_documents_with_role(user_id, UserRole::User, search_request, fuzzy)
            .await
    }

    /// Enhanced search with role-based access control, filters and sorting.
    /// Defaults to relevance order when no sort is given. `fuzzy` is the
    /// cutoff of fuzzy mode (ignored by the other modes).
    pub async fn enhanced_search_documents_with_role(
        &self,
        user_id: Uuid,
        user_role: UserRole,
        search_request: &SearchRequest,
        fuzzy: FuzzyThreshold,
    ) -> Result<Vec<EnhancedDocumentResponse>> {
        let search_query = search_request.query.trim();
        let include_snippets = search_request.include_snippets.unwrap_or(true);
        let snippet_length = search_request.snippet_length.unwrap_or(200) as usize;
        let mode = request_mode(search_request);

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

        let mut tx = self.begin_search(mode, fuzzy).await?;
        let rows = query.build().fetch_all(&mut *tx).await?;
        tx.commit().await?;

        let terms = highlight_terms(search_query, mode);
        let mut results = Vec::new();
        for row in rows {
            let item = map_row_to_document_with_progress(&row);
            let search_rank: f32 = row.try_get("search_rank").unwrap_or(0.0);

            let snippets = if include_snippets && !search_query.is_empty() {
                self.generate_snippets(&item.document, &terms, snippet_length)
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

    /// Snippets of `document`'s content and OCR text around each
    /// occurrence of a highlight term (see `highlight_terms`).
    pub fn generate_snippets(
        &self,
        document: &Document,
        terms: &[String],
        snippet_length: usize,
    ) -> Vec<SearchSnippet> {
        let texts = [
            document.content.as_deref().unwrap_or(""),
            document.ocr_text.as_deref().unwrap_or(""),
        ];
        build_snippets(&texts, terms, snippet_length)
    }

    /// Counts total matching documents for pagination (without applying
    /// LIMIT/OFFSET), using the request's `search_mode` (simple when unset)
    /// and, in fuzzy mode, the cutoff `fuzzy`. Pass the same `fuzzy` as to
    /// `enhanced_search_documents_with_role` so the count matches the list.
    pub async fn count_search_documents(
        &self,
        user_id: Uuid,
        user_role: UserRole,
        search_request: &SearchRequest,
        fuzzy: FuzzyThreshold,
    ) -> Result<i64> {
        let mode = request_mode(search_request);
        let mut query = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM documents WHERE 1=1");
        push_search_scope(&mut query, user_id, user_role, search_request, mode);

        let mut tx = self.begin_search(mode, fuzzy).await?;
        let row: (i64,) = query.build_query_as().fetch_one(&mut *tx).await?;
        tx.commit().await?;
        Ok(row.0)
    }

    /// Counts the matches of basic search, which always matches in simple
    /// mode whatever the request's `search_mode`, so its count must too.
    pub async fn count_basic_search_documents(
        &self,
        user_id: Uuid,
        user_role: UserRole,
        search_request: &SearchRequest,
    ) -> Result<i64> {
        let mut query = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM documents WHERE 1=1");
        push_search_scope(
            &mut query,
            user_id,
            user_role,
            search_request,
            &SearchMode::Simple,
        );

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
        fuzzy: FuzzyThreshold,
    ) -> Result<Vec<MonthCount>> {
        let mode = request_mode(search_request);
        let mut query = QueryBuilder::<Postgres>::new(
            "SELECT to_char(date_trunc('month', created_at AT TIME ZONE 'UTC'), 'YYYY-MM') AS month, \
             COUNT(*) AS count FROM documents WHERE 1=1",
        );
        push_search_scope(&mut query, user_id, user_role, search_request, mode);
        query.push(" GROUP BY 1 ORDER BY 1");

        let mut tx = self.begin_search(mode, fuzzy).await?;
        let rows: Vec<(String, i64)> = query.build_query_as().fetch_all(&mut *tx).await?;
        tx.commit().await?;
        Ok(rows
            .into_iter()
            .map(|(month, count)| MonthCount { month, count })
            .collect())
    }
}
