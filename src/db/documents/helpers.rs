use sqlx::{Row, QueryBuilder, Postgres};
use uuid::Uuid;

use crate::models::{Document, UserRole};

/// Standard document fields for SELECT queries
pub const DOCUMENT_FIELDS: &str = r#"
    id, filename, original_filename, file_path, file_size, mime_type, 
    content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, 
    ocr_status, ocr_error, ocr_completed_at, ocr_retry_count, ocr_failure_reason, 
    tags, created_at, updated_at, user_id, file_hash, original_created_at, 
    original_modified_at, source_metadata
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