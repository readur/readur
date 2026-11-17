use anyhow::{ensure, Result};
use sqlx::pool::PoolConnection;
use sqlx::{PgConnection, Postgres, QueryBuilder, Row, Transaction};
use uuid::Uuid;

use super::helpers::{
    apply_document_filters, apply_pagination, apply_role_based_filter, apply_sort,
    map_row_to_document_with_progress, DocumentWithProgress, DOCUMENT_FIELDS, OCR_PROGRESS_FIELDS,
    OCR_PROGRESS_JOIN,
};
use super::search_query::{
    document_match, single_field_match, BooleanQuery, SqlPart, DOCUMENT_TSVECTOR, LEAF_LEXEMES_SQL,
    PHRASETO_TSQUERY, PLAINTO_TSQUERY, TO_TSQUERY,
};
use super::snippets::{build_snippets, Highlight};
use crate::db::sanitize::strip_nul;
use crate::db::Database;
use crate::models::{
    Document, EnhancedDocumentResponse, FuzzyThreshold, MonthCount, SearchMode, SearchRequest,
    SearchSnippet, UserRole,
};

/// A document's searchable text as one string, for fuzzy matching. It is
/// exactly the expression of the `idx_documents_content_trgm` trigram index,
/// so `<%` can be answered from that index instead of scanning every row.
const DOCUMENT_TEXT: &str = "(COALESCE(content, '') || ' ' || COALESCE(ocr_text, ''))";

/// The words and phrases to highlight in snippets for `search_query`. Boolean
/// queries contribute only their non-negated terms, without operators;
/// phrase mode the whole phrase, as one term; simple and fuzzy mode each
/// word. Surrounding double quotes are dropped.
pub(crate) fn highlight_terms(search_query: &str, mode: &SearchMode) -> Vec<String> {
    match mode {
        SearchMode::Boolean => BooleanQuery::parse(search_query).positive_terms(),
        SearchMode::Phrase => {
            let phrase = query_words(search_query).join(" ");
            if phrase.is_empty() {
                Vec::new()
            } else {
                vec![phrase]
            }
        }
        _ => query_words(search_query),
    }
}

/// Terms to highlight instead when no snippet contains `highlight_terms`:
/// in phrase mode the phrase's words, since the phrase matched after
/// stemming and stop-word removal ("invoices due" matches "invoice is
/// due") can be nowhere in the text as typed.
pub(crate) fn fallback_highlight_terms(search_query: &str, mode: &SearchMode) -> Vec<String> {
    match mode {
        SearchMode::Phrase => query_words(search_query),
        _ => Vec::new(),
    }
}

/// The whitespace-separated words of `search_query`, without surrounding
/// double quotes.
fn query_words(search_query: &str) -> Vec<String> {
    search_query
        .split_whitespace()
        .map(|term| term.trim_matches('"'))
        .filter(|term| !term.is_empty())
        .map(String::from)
        .collect()
}

/// Appends `parts`, binding each `SqlPart::Bind` as a parameter.
fn push_parts(query: &mut QueryBuilder<Postgres>, parts: &[SqlPart]) {
    for part in parts {
        match part {
            SqlPart::Sql(sql) => {
                query.push(*sql);
            }
            SqlPart::Bind(value) => {
                query.push_bind(value.clone());
            }
        }
    }
}

/// Appends the `, <rank> as search_rank` select column for a search query.
/// Ranking scores the whole document; which field matched does not matter.
fn push_search_rank(query: &mut QueryBuilder<Postgres>, search_query: &str, mode: &SearchMode) {
    let (function, input) = match mode {
        // How closely the best-matching stretch of the text is spelled like
        // the query (1.0 when the query's words appear as written).
        SearchMode::Fuzzy => {
            query.push(", word_similarity(");
            query.push_bind(strip_nul(search_query).into_owned());
            query.push(", ");
            query.push(DOCUMENT_TEXT);
            query.push(") as search_rank");
            return;
        }
        SearchMode::Simple => (PLAINTO_TSQUERY, strip_nul(search_query).into_owned()),
        SearchMode::Phrase => (PHRASETO_TSQUERY, strip_nul(search_query).into_owned()),
        SearchMode::Boolean => (TO_TSQUERY, BooleanQuery::parse(search_query).to_tsquery()),
    };
    query.push(", ts_rank(");
    query.push(DOCUMENT_TSVECTOR);
    query.push(", ");
    query.push(function);
    query.push_bind(input);
    query.push(")) as search_rank");
}

/// The condition a document matches `search_query` (non-empty) in `mode`
/// by, as SQL. Boolean queries first look up how many lexemes each of
/// their leaves has (see `BooleanQuery::to_sql`) on `conn`.
async fn text_condition(
    conn: &mut PgConnection,
    search_query: &str,
    mode: &SearchMode,
) -> Result<Vec<SqlPart>> {
    let input = strip_nul(search_query);
    Ok(match mode {
        // Every word, anywhere in the document.
        SearchMode::Simple => document_match(PLAINTO_TSQUERY, &input),
        // The words in order, within one field.
        SearchMode::Phrase => {
            single_field_match(PHRASETO_TSQUERY, &input, Some((PLAINTO_TSQUERY, &input)))
        }
        SearchMode::Boolean => boolean_condition(conn, search_query).await?,
        // Word-level: the query against the closest stretch of the text,
        // not the whole text, so a long document containing a near-miss of
        // the query matches. The cutoff is `pg_trgm.word_similarity_threshold`,
        // set per search by `Database::begin_search`.
        SearchMode::Fuzzy => vec![
            SqlPart::Bind(input.into_owned()),
            SqlPart::Sql(" <% "),
            SqlPart::Sql(DOCUMENT_TEXT),
        ],
    })
}

/// A boolean query as SQL; `FALSE` when it has no searchable word.
async fn boolean_condition(conn: &mut PgConnection, search_query: &str) -> Result<Vec<SqlPart>> {
    let parsed = BooleanQuery::parse(search_query);
    let leaves = parsed.leaves();
    let lexemes: Vec<i32> = if leaves.is_empty() {
        Vec::new()
    } else {
        sqlx::query_scalar(LEAF_LEXEMES_SQL)
            .bind(&leaves)
            .fetch_all(&mut *conn)
            .await?
    };
    ensure!(
        lexemes.len() == leaves.len(),
        "lexeme counts for {} leaves, got {}",
        leaves.len(),
        lexemes.len()
    );
    Ok(parsed
        .to_sql(&lexemes)
        .unwrap_or_else(|| vec![SqlPart::Sql("FALSE")]))
}

/// Appends the visibility, text-match and filter conditions shared by the
/// search list, count and timeline. `condition` is the text match from
/// `text_condition`, empty when the query is.
fn push_search_scope(
    query: &mut QueryBuilder<Postgres>,
    user_id: Uuid,
    user_role: UserRole,
    search_request: &SearchRequest,
    condition: &[SqlPart],
) {
    apply_role_based_filter(query, user_id, user_role);
    if !condition.is_empty() {
        query.push(" AND ");
        push_parts(query, condition);
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

/// Whether a search in `mode` must run in a transaction: only fuzzy mode,
/// to scope its similarity cutoff (see `Database::begin_search`).
fn needs_transaction(mode: &SearchMode) -> bool {
    matches!(mode, SearchMode::Fuzzy)
}

/// The connection one search runs its queries on.
enum SearchSession {
    Plain(PoolConnection<Postgres>),
    Fuzzy(Transaction<'static, Postgres>),
}

impl SearchSession {
    fn conn(&mut self) -> &mut PgConnection {
        match self {
            Self::Plain(conn) => conn,
            Self::Fuzzy(tx) => tx,
        }
    }

    /// Ends the search, committing its transaction if it has one.
    async fn finish(self) -> Result<()> {
        if let Self::Fuzzy(tx) = self {
            tx.commit().await?;
        }
        Ok(())
    }
}

/// The text condition of `search_request` on `session`; empty for an empty query.
async fn request_condition(
    session: &mut SearchSession,
    search_request: &SearchRequest,
) -> Result<Vec<SqlPart>> {
    let search_query = search_request.query.trim();
    if search_query.is_empty() {
        return Ok(Vec::new());
    }
    text_condition(session.conn(), search_query, request_mode(search_request)).await
}

impl Database {
    /// Opens the connection a search runs on. Only fuzzy mode gets a
    /// transaction, in which it first sets `pg_trgm.word_similarity_threshold`
    /// to `fuzzy`: the `<%` operator, which the trigram index can serve,
    /// takes its cutoff from that setting rather than from an argument.
    /// `set_config(.., true)` scopes it to the transaction, so it never leaks
    /// to the next user of the pooled connection, and every query of the
    /// search sees the same cutoff. Other modes need no transaction.
    async fn begin_search(
        &self,
        mode: &SearchMode,
        fuzzy: FuzzyThreshold,
    ) -> Result<SearchSession> {
        if !needs_transaction(mode) {
            return Ok(SearchSession::Plain(self.pool.acquire().await?));
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT set_config('pg_trgm.word_similarity_threshold', $1, true)")
            .bind(fuzzy.setting())
            .execute(&mut *tx)
            .await?;
        Ok(SearchSession::Fuzzy(tx))
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
        push_search_scope(
            &mut query,
            user_id,
            user_role,
            search_request,
            &basic_condition(search_request),
        );
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
        let mode = request_mode(search_request);
        let mut session = self.begin_search(mode, fuzzy).await?;
        let condition = request_condition(&mut session, search_request).await?;

        let mut query = QueryBuilder::<Postgres>::new("SELECT ");
        query.push(DOCUMENT_FIELDS);
        query.push(OCR_PROGRESS_FIELDS);
        if search_query.is_empty() {
            query.push(", 0.0 as search_rank");
        } else {
            push_search_rank(&mut query, search_query, mode);
        }
        query.push(" FROM documents");
        query.push(OCR_PROGRESS_JOIN);
        query.push(" WHERE 1=1");
        push_search_scope(&mut query, user_id, user_role, search_request, &condition);
        apply_sort(
            &mut query,
            search_request.sort_by,
            search_request.sort_order,
            !search_query.is_empty(),
        );

        let limit = search_request.limit.unwrap_or(25);
        let offset = search_request.offset.unwrap_or(0);
        apply_pagination(&mut query, limit, offset);

        let rows = query.build().fetch_all(session.conn()).await?;
        session.finish().await?;

        let items: Vec<(DocumentWithProgress, f32)> = rows
            .iter()
            .map(|row| {
                let rank = row.try_get("search_rank").unwrap_or(0.0);
                (map_row_to_document_with_progress(row), rank)
            })
            .collect();
        let include_snippets = search_request.include_snippets.unwrap_or(true);
        let (items, snippets) = if include_snippets && !search_query.is_empty() {
            let options = SnippetOptions {
                terms: highlight_terms(search_query, mode),
                fallback_terms: fallback_highlight_terms(search_query, mode),
                snippet_length: search_request.snippet_length.unwrap_or(200) as usize,
                highlight: Highlight::for_mode(mode, fuzzy),
            };
            // Off the async runtime: a page of multi-megabyte texts takes a
            // while to scan.
            tokio::task::spawn_blocking(move || {
                let snippets: Vec<Vec<SearchSnippet>> = items
                    .iter()
                    .map(|(item, _)| options.document_snippets(&item.document))
                    .collect();
                (items, snippets)
            })
            .await?
        } else {
            let none = items.iter().map(|_| Vec::new()).collect();
            (items, none)
        };

        let mut results: Vec<EnhancedDocumentResponse> = items
            .into_iter()
            .zip(snippets)
            .map(|((item, rank), snippets)| {
                EnhancedDocumentResponse::from_document(
                    item.document,
                    Some(rank),
                    snippets,
                    item.ocr_progress_current,
                    item.ocr_progress_total,
                )
            })
            .collect();
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
        let mut session = self
            .begin_search(request_mode(search_request), fuzzy)
            .await?;
        let condition = request_condition(&mut session, search_request).await?;
        let mut query = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM documents WHERE 1=1");
        push_search_scope(&mut query, user_id, user_role, search_request, &condition);

        let row: (i64,) = query.build_query_as().fetch_one(session.conn()).await?;
        session.finish().await?;
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
            &basic_condition(search_request),
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
        let mut session = self
            .begin_search(request_mode(search_request), fuzzy)
            .await?;
        let condition = request_condition(&mut session, search_request).await?;
        let mut query = QueryBuilder::<Postgres>::new(
            "SELECT to_char(date_trunc('month', created_at AT TIME ZONE 'UTC'), 'YYYY-MM') AS month, \
             COUNT(*) AS count FROM documents WHERE 1=1",
        );
        push_search_scope(&mut query, user_id, user_role, search_request, &condition);
        query.push(" GROUP BY 1 ORDER BY 1");

        let rows: Vec<(String, i64)> = query.build_query_as().fetch_all(session.conn()).await?;
        session.finish().await?;
        Ok(rows
            .into_iter()
            .map(|(month, count)| MonthCount { month, count })
            .collect())
    }
}

/// The simple-mode text condition basic search matches by; empty for an
/// empty query.
fn basic_condition(search_request: &SearchRequest) -> Vec<SqlPart> {
    let search_query = search_request.query.trim();
    if search_query.is_empty() {
        Vec::new()
    } else {
        document_match(PLAINTO_TSQUERY, &strip_nul(search_query))
    }
}

/// What a page of search results gets snippets for, and how.
struct SnippetOptions {
    terms: Vec<String>,
    fallback_terms: Vec<String>,
    snippet_length: usize,
    highlight: Highlight,
}

impl SnippetOptions {
    /// Snippets of `document`'s content and OCR text around each match of
    /// a highlight term (see `highlight_terms`): as typed, or in fuzzy mode
    /// the words the search's trigram cutoff matched (see `Highlight`).
    /// When no term occurs, the fallback terms are tried.
    fn document_snippets(&self, document: &Document) -> Vec<SearchSnippet> {
        let texts = [
            document.content.as_deref().unwrap_or(""),
            document.ocr_text.as_deref().unwrap_or(""),
        ];
        let snippets = build_snippets(&texts, &self.terms, self.snippet_length, self.highlight);
        if snippets.is_empty() && !self.fallback_terms.is_empty() {
            return build_snippets(
                &texts,
                &self.fallback_terms,
                self.snippet_length,
                self.highlight,
            );
        }
        snippets
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_fuzzy_search_runs_in_a_transaction() {
        assert!(needs_transaction(&SearchMode::Fuzzy));
        for mode in [SearchMode::Simple, SearchMode::Phrase, SearchMode::Boolean] {
            assert!(!needs_transaction(&mode), "{mode:?}");
        }
    }

    #[test]
    fn phrase_mode_highlights_the_whole_phrase() {
        let phrase = SearchMode::Phrase;
        assert_eq!(highlight_terms("\"total   due\"", &phrase), ["total due"]);
        assert_eq!(highlight_terms("  Total\tdue ", &phrase), ["Total due"]);
        assert!(highlight_terms("\"\"", &phrase).is_empty());
        assert_eq!(
            fallback_highlight_terms("\"total due\"", &phrase),
            ["total", "due"]
        );
        let simple = SearchMode::Simple;
        assert_eq!(highlight_terms("total due", &simple), ["total", "due"]);
        assert!(fallback_highlight_terms("total due", &simple).is_empty());
    }

    #[test]
    fn conditions_bind_every_piece_of_user_text() {
        let hostile = "x'); DROP TABLE documents; --";
        let parts = document_match(PLAINTO_TSQUERY, hostile);
        assert!(parts.contains(&SqlPart::Bind(hostile.to_string())));
        for part in &parts {
            if let SqlPart::Sql(sql) = part {
                assert!(!sql.contains("DROP"), "{sql}");
            }
        }
        let basic = basic_condition(&SearchRequest {
            query: format!(" {hostile} "),
            ..Default::default()
        });
        assert!(basic.contains(&SqlPart::Bind(hostile.to_string())));
        assert!(basic_condition(&SearchRequest::default()).is_empty());
    }

    /// Every phrase-like condition is prefiltered on the combined tsvector,
    /// so the GIN index still serves it (no per-field index exists).
    #[tokio::test]
    async fn text_conditions_are_served_by_the_full_text_index() {
        let ctx = crate::test_utils::TestContext::new().await;
        let mut tx = ctx.state().db.pool.begin().await.unwrap();
        sqlx::query("SET LOCAL enable_seqscan = off")
            .execute(&mut *tx)
            .await
            .unwrap();
        for (query, mode) in [
            ("total due", SearchMode::Simple),
            ("total due", SearchMode::Phrase),
            ("invoice", SearchMode::Boolean),
            ("\"total due\"", SearchMode::Boolean),
            ("total <-> due -draft", SearchMode::Boolean),
            ("(invoice | receipt) & \"total due\"", SearchMode::Boolean),
        ] {
            let condition = text_condition(&mut tx, query, &mode).await.unwrap();
            let mut explain =
                QueryBuilder::<Postgres>::new("EXPLAIN SELECT id FROM documents WHERE ");
            push_parts(&mut explain, &condition);
            let plan: Vec<String> = explain
                .build_query_scalar()
                .fetch_all(&mut *tx)
                .await
                .unwrap();
            let plan = plan.join("\n");
            assert!(
                plan.contains("idx_documents_content_search") && !plan.contains("Seq Scan"),
                "{mode:?} {query:?}:\n{plan}"
            );
        }
    }
}
