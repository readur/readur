use sqlx::{Row, QueryBuilder, Postgres};
use uuid::Uuid;

use crate::models::{Document, DocumentFilters, DocumentSortField, SortOrder, UserRole};

/// Standard document fields for SELECT queries
pub const DOCUMENT_FIELDS: &str = r#"
    id, filename, original_filename, file_path, file_size, mime_type, 
    content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, 
    ocr_status, ocr_error, ocr_completed_at, ocr_retry_count, ocr_failure_reason, 
    tags, created_at, updated_at, user_id, file_hash, original_created_at, 
    original_modified_at, source_path, source_type, source_id, file_permissions, 
    file_owner, file_group, source_metadata
"#;

/// Maps a database row to a Document struct
/// This eliminates the ~15+ instances of duplicate row mapping code
pub fn map_row_to_document(row: &sqlx::postgres::PgRow) -> Document {
    Document {
        id: row.get("id"),
        filename: row.get("filename"),
        original_filename: row.get("original_filename"),
        file_path: row.get("file_path"),
        file_size: row.get("file_size"),
        mime_type: row.get("mime_type"),
        content: row.get("content"),
        ocr_text: row.get("ocr_text"),
        ocr_confidence: row.get("ocr_confidence"),
        ocr_word_count: row.get("ocr_word_count"),
        ocr_processing_time_ms: row.get("ocr_processing_time_ms"),
        ocr_status: row.get("ocr_status"),
        ocr_error: row.get("ocr_error"),
        ocr_completed_at: row.get("ocr_completed_at"),
        ocr_retry_count: row.get("ocr_retry_count"),
        ocr_failure_reason: row.get("ocr_failure_reason"),
        tags: row.get("tags"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        user_id: row.get("user_id"),
        file_hash: row.get("file_hash"),
        original_created_at: row.get("original_created_at"),
        original_modified_at: row.get("original_modified_at"),
        source_path: row.get("source_path"),
        source_type: row.get("source_type"),
        source_id: row.get("source_id"),
        file_permissions: row.get("file_permissions"),
        file_owner: row.get("file_owner"),
        file_group: row.get("file_group"),
        source_metadata: row.get("source_metadata"),
    }
}

/// Applies role-based filtering to a query builder
/// Admins can see all documents, regular users only see their own
pub fn apply_role_based_filter(
    query: &mut QueryBuilder<Postgres>, 
    user_id: Uuid, 
    role: UserRole
) {
    match role {
        UserRole::Admin => {
            // Admins can see all documents - no additional filter needed
        }
        UserRole::User => {
            query.push(" AND user_id = ");
            query.push_bind(user_id);
        }
    }
}

/// Applies pagination to a query builder
pub fn apply_pagination(query: &mut QueryBuilder<Postgres>, limit: i64, offset: i64) {
    query.push(" LIMIT ");
    query.push_bind(limit);
    query.push(" OFFSET ");
    query.push_bind(offset);
}

/// Extra SELECT columns exposing OCR page progress; pair with `OCR_PROGRESS_JOIN`.
pub const OCR_PROGRESS_FIELDS: &str =
    ", ocr_progress.progress_current AS ocr_progress_current, ocr_progress.progress_total AS ocr_progress_total";

/// Joins at most one actively processing `ocr_queue` row per document, so the
/// join can never duplicate document rows. Must follow `FROM documents`.
pub const OCR_PROGRESS_JOIN: &str = " LEFT JOIN LATERAL (\
    SELECT q.progress_current, q.progress_total FROM ocr_queue q \
    WHERE q.document_id = documents.id AND q.status = 'processing' \
    ORDER BY q.started_at DESC NULLS LAST LIMIT 1\
    ) ocr_progress ON TRUE";

/// A document row together with its active OCR progress, if any.
#[derive(Debug, Clone)]
pub struct DocumentWithProgress {
    pub document: Document,
    pub ocr_progress_current: Option<i32>,
    pub ocr_progress_total: Option<i32>,
}

/// Maps a row selected with `DOCUMENT_FIELDS` + `OCR_PROGRESS_FIELDS`.
pub fn map_row_to_document_with_progress(row: &sqlx::postgres::PgRow) -> DocumentWithProgress {
    DocumentWithProgress {
        document: map_row_to_document(row),
        ocr_progress_current: row.try_get("ocr_progress_current").unwrap_or(None),
        ocr_progress_total: row.try_get("ocr_progress_total").unwrap_or(None),
    }
}

/// Appends `AND ...` conditions for every set filter. User-supplied values are
/// only ever passed through `push_bind`.
pub fn apply_document_filters(query: &mut QueryBuilder<Postgres>, filters: &DocumentFilters) {
    if let Some(status) = filters.ocr_status.as_deref() {
        if status == "pending" {
            // Documents that never had a status set are pending as well
            query.push(" AND (ocr_status IS NULL OR ocr_status = ");
            query.push_bind(status.to_string());
            query.push(")");
        } else {
            query.push(" AND ocr_status = ");
            query.push_bind(status.to_string());
        }
    }

    if let Some(mime_types) = filters.mime_types.as_ref().filter(|v| !v.is_empty()) {
        query.push(" AND mime_type = ANY(");
        query.push_bind(mime_types.clone());
        query.push(")");
    }

    if let Some(label_ids) = filters.label_ids.as_ref().filter(|v| !v.is_empty()) {
        query.push(" AND documents.id IN (SELECT dl.document_id FROM document_labels dl WHERE dl.label_id = ANY(");
        query.push_bind(label_ids.clone());
        query.push("))");
    }

    if let Some(tags) = filters.tags.as_ref().filter(|v| !v.is_empty()) {
        query.push(" AND documents.id IN (SELECT dl.document_id FROM document_labels dl JOIN labels l ON dl.label_id = l.id WHERE l.name = ANY(");
        query.push_bind(tags.clone());
        query.push("))");
    }

    if let Some(source_ids) = filters.source_ids.as_ref().filter(|v| !v.is_empty()) {
        query.push(" AND source_id = ANY(");
        query.push_bind(source_ids.clone());
        query.push(")");
    }

    if let Some(source_types) = filters.source_types.as_ref().filter(|v| !v.is_empty()) {
        query.push(" AND (source_type = ANY(");
        query.push_bind(source_types.clone());
        query.push(")");
        if source_types.iter().any(|t| t == "direct_upload") {
            // Direct uploads are not always tagged with a source type
            query.push(" OR source_id IS NULL");
        }
        query.push(")");
    }

    if let Some(from) = filters.created_from {
        query.push(" AND created_at >= ");
        query.push_bind(from);
    }

    if let Some(to) = filters.created_to {
        query.push(" AND created_at <= ");
        query.push_bind(to);
    }
}

/// Appends the ORDER BY clause. Only static strings from the sort enums reach
/// the SQL text.
///
/// - explicit `sort`: `<column> <dir> NULLS LAST, id <dir>` (dir defaults to DESC)
/// - no `sort`, `has_rank`: `search_rank DESC, created_at DESC, id DESC`
/// - otherwise: `created_at <dir>, id <dir>` (dir defaults to DESC)
pub fn apply_sort(
    query: &mut QueryBuilder<Postgres>,
    sort: Option<DocumentSortField>,
    order: Option<SortOrder>,
    has_rank: bool,
) {
    let dir = order.unwrap_or(SortOrder::Desc).sql();
    match sort {
        Some(field) => {
            query.push(" ORDER BY ");
            query.push(field.sql_column());
            query.push(" ");
            query.push(dir);
            query.push(" NULLS LAST, id ");
            query.push(dir);
        }
        None if has_rank => {
            query.push(" ORDER BY search_rank DESC, created_at DESC, id DESC");
        }
        None => {
            query.push(" ORDER BY created_at ");
            query.push(dir);
            query.push(", id ");
            query.push(dir);
        }
    }
}

/// Helper to determine if a character is a word boundary for snippet generation
pub fn is_word_boundary(c: char) -> bool {
    c.is_whitespace() || c.is_ascii_punctuation()
}

/// Finds word boundary for snippet generation
pub fn find_word_boundary(text: &str, position: usize, search_forward: bool) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let start_pos = if position >= chars.len() { chars.len() - 1 } else { position };
    
    if search_forward {
        for i in start_pos..chars.len() {
            if is_word_boundary(chars[i]) {
                return text.char_indices().nth(i).map(|(idx, _)| idx).unwrap_or(text.len());
            }
        }
        text.len()
    } else {
        for i in (0..=start_pos).rev() {
            if is_word_boundary(chars[i]) {
                return text.char_indices().nth(i).map(|(idx, _)| idx).unwrap_or(0);
            }
        }
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "SELECT id FROM documents WHERE 1=1";

    fn order_by_sql(sort: Option<DocumentSortField>, order: Option<SortOrder>, has_rank: bool) -> String {
        let mut query = QueryBuilder::<Postgres>::new(BASE);
        apply_sort(&mut query, sort, order, has_rank);
        query.sql()[BASE.len()..].to_string()
    }

    #[test]
    fn explicit_sort_uses_only_the_whitelisted_column() {
        let all_columns: Vec<&str> = DocumentSortField::ALL.iter().map(|f| f.sql_column()).collect();
        for field in DocumentSortField::ALL {
            for order in [SortOrder::Asc, SortOrder::Desc] {
                let sql = order_by_sql(Some(field), Some(order), false);
                let dir = order.sql();
                assert_eq!(
                    sql,
                    format!(" ORDER BY {} {} NULLS LAST, id {}", field.sql_column(), dir, dir)
                );
                for other in all_columns.iter().filter(|c| **c != field.sql_column()) {
                    assert!(!sql.contains(other), "{:?} sort leaked column {}", field, other);
                }
            }
        }
    }

    #[test]
    fn filename_sorts_by_original_filename() {
        assert_eq!(DocumentSortField::Filename.sql_column(), "original_filename");
    }

    #[test]
    fn default_sorts_match_previous_behaviour() {
        assert_eq!(order_by_sql(None, None, false), " ORDER BY created_at DESC, id DESC");
        assert_eq!(
            order_by_sql(None, None, true),
            " ORDER BY search_rank DESC, created_at DESC, id DESC"
        );
        assert_eq!(
            order_by_sql(Some(DocumentSortField::FileSize), None, true),
            " ORDER BY file_size DESC NULLS LAST, id DESC"
        );
        assert_eq!(order_by_sql(None, Some(SortOrder::Asc), false), " ORDER BY created_at ASC, id ASC");
    }

    #[test]
    fn filters_only_use_bind_parameters() {
        let hostile = "x'); DROP TABLE documents; --".to_string();
        let filters = DocumentFilters {
            ocr_status: Some(hostile.clone()),
            mime_types: Some(vec![hostile.clone()]),
            label_ids: Some(vec![Uuid::new_v4()]),
            tags: Some(vec![hostile.clone()]),
            source_ids: Some(vec![Uuid::new_v4()]),
            source_types: Some(vec![hostile.clone(), "direct_upload".to_string()]),
            created_from: Some(chrono::Utc::now()),
            created_to: Some(chrono::Utc::now()),
        };
        let mut query = QueryBuilder::<Postgres>::new(BASE);
        apply_document_filters(&mut query, &filters);
        apply_sort(&mut query, Some(DocumentSortField::MimeType), Some(SortOrder::Asc), false);
        let sql = query.sql();

        assert!(!sql.contains("DROP"), "user input reached SQL text: {}", sql);
        assert!(!sql.contains('\''), "unexpected literal in SQL text: {}", sql);
        for fragment in [
            "ocr_status = $1",
            "mime_type = ANY($2)",
            "dl.label_id = ANY($3)",
            "l.name = ANY($4)",
            "source_id = ANY($5)",
            "source_type = ANY($6) OR source_id IS NULL",
            "created_at >= $7",
            "created_at <= $8",
            "ORDER BY mime_type ASC NULLS LAST, id ASC",
        ] {
            assert!(sql.contains(fragment), "missing {:?} in {}", fragment, sql);
        }
    }

    #[test]
    fn pending_status_also_matches_null() {
        let filters = DocumentFilters { ocr_status: Some("pending".to_string()), ..Default::default() };
        let mut query = QueryBuilder::<Postgres>::new(BASE);
        apply_document_filters(&mut query, &filters);
        assert!(query.sql().contains("AND (ocr_status IS NULL OR ocr_status = $1)"));
    }

    #[test]
    fn source_types_without_direct_upload_do_not_match_null_sources() {
        let filters = DocumentFilters { source_types: Some(vec!["webdav".to_string()]), ..Default::default() };
        let mut query = QueryBuilder::<Postgres>::new(BASE);
        apply_document_filters(&mut query, &filters);
        let sql = query.sql();
        assert!(sql.contains("AND (source_type = ANY($1))"));
        assert!(!sql.contains("source_id IS NULL"));
    }

    #[test]
    fn empty_filters_add_nothing() {
        let mut query = QueryBuilder::<Postgres>::new(BASE);
        apply_document_filters(&mut query, &DocumentFilters::default());
        assert_eq!(query.sql(), BASE);
    }
}