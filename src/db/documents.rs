use anyhow::Result;
use sqlx::{Row, QueryBuilder};
use uuid::Uuid;

use crate::models::{Document, SearchRequest, SearchMode, SearchSnippet, HighlightRange, EnhancedDocumentResponse};
use crate::routes::labels::Label;
use super::Database;

impl Database {
    pub async fn create_document(&self, document: Document) -> Result<Document> {
        let row = sqlx::query(
            r#"
            INSERT INTO documents (id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, ocr_retry_count, ocr_failure_reason, tags, created_at, updated_at, user_id, file_hash, original_created_at, original_modified_at, source_metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24)
            RETURNING id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, ocr_retry_count, ocr_failure_reason, tags, created_at, updated_at, user_id, file_hash, original_created_at, original_modified_at, source_metadata
            "#
        )
        .bind(document.id)
        .bind(&document.filename)
        .bind(&document.original_filename)
        .bind(&document.file_path)
        .bind(document.file_size)
        .bind(&document.mime_type)
        .bind(&document.content)
        .bind(&document.ocr_text)
        .bind(document.ocr_confidence)
        .bind(document.ocr_word_count)
        .bind(document.ocr_processing_time_ms)
        .bind(&document.ocr_status)
        .bind(&document.ocr_error)
        .bind(document.ocr_completed_at)
        .bind(&document.tags)
        .bind(document.created_at)
        .bind(document.updated_at)
        .bind(document.user_id)
        .bind(&document.file_hash)
        .bind(document.original_created_at)
        .bind(document.original_modified_at)
        .bind(document.ocr_retry_count)
        .bind(&document.ocr_failure_reason)
        .bind(&document.source_metadata)
        .fetch_one(&self.pool)
        .await?;

        Ok(Document {
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
        })
    }

    pub async fn get_documents_by_user_with_role(&self, user_id: Uuid, user_role: crate::models::UserRole, limit: i64, offset: i64) -> Result<Vec<Document>> {
        let query = if user_role == crate::models::UserRole::Admin {
            // Admins can see all documents
            r#"
            SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, ocr_retry_count, ocr_failure_reason, tags, created_at, updated_at, user_id, file_hash, original_created_at, original_modified_at, source_metadata
            FROM documents 
            ORDER BY created_at DESC 
            LIMIT $1 OFFSET $2
            "#
        } else {
            // Regular users can only see their own documents
            r#"
            SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, ocr_retry_count, ocr_failure_reason, tags, created_at, updated_at, user_id, file_hash, original_created_at, original_modified_at, source_metadata
            FROM documents 
            WHERE user_id = $3 
            ORDER BY created_at DESC 
            LIMIT $1 OFFSET $2
            "#
        };

        let rows = if user_role == crate::models::UserRole::Admin {
            sqlx::query(query)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query(query)
                .bind(limit)
                .bind(offset)
                .bind(user_id)
                .fetch_all(&self.pool)
                .await?
        };

        let documents = rows
            .into_iter()
            .map(|row| Document {
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
            })
            .collect();

        Ok(documents)
    }

    pub async fn get_documents_by_user_with_role_and_filter(&self, user_id: Uuid, user_role: crate::models::UserRole, limit: i64, offset: i64, ocr_status: Option<&str>) -> Result<Vec<Document>> {
        let rows = match (user_role == crate::models::UserRole::Admin, ocr_status) {
            (true, Some(status)) => {
                // Admin with OCR filter
                sqlx::query(
                    r#"
                    SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, ocr_retry_count, ocr_failure_reason, tags, created_at, updated_at, user_id, file_hash, original_created_at, original_modified_at, source_metadata
                    FROM documents 
                    WHERE ocr_status = $3
                    ORDER BY created_at DESC 
                    LIMIT $1 OFFSET $2
                    "#
                )
                .bind(limit)
                .bind(offset)
                .bind(status)
                .fetch_all(&self.pool)
                .await?
            }
            (true, None) => {
                // Admin without OCR filter
                sqlx::query(
                    r#"
                    SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, ocr_retry_count, ocr_failure_reason, tags, created_at, updated_at, user_id, file_hash, original_created_at, original_modified_at, source_metadata
                    FROM documents 
                    ORDER BY created_at DESC 
                    LIMIT $1 OFFSET $2
                    "#
                )
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
            }
            (false, Some(status)) => {
                // Regular user with OCR filter
                sqlx::query(
                    r#"
                    SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, ocr_retry_count, ocr_failure_reason, tags, created_at, updated_at, user_id, file_hash, original_created_at, original_modified_at, source_metadata
                    FROM documents 
                    WHERE user_id = $3 AND ocr_status = $4
                    ORDER BY created_at DESC 
                    LIMIT $1 OFFSET $2
                    "#
                )
                .bind(limit)
                .bind(offset)
                .bind(user_id)
                .bind(status)
                .fetch_all(&self.pool)
                .await?
            }
            (false, None) => {
                // Regular user without OCR filter
                sqlx::query(
                    r#"
                    SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, ocr_retry_count, ocr_failure_reason, tags, created_at, updated_at, user_id, file_hash, original_created_at, original_modified_at, source_metadata
                    FROM documents 
                    WHERE user_id = $3 
                    ORDER BY created_at DESC 
                    LIMIT $1 OFFSET $2
                    "#
                )
                .bind(limit)
                .bind(offset)
                .bind(user_id)
                .fetch_all(&self.pool)
                .await?
            }
        };

        let documents = rows
            .into_iter()
            .map(|row| Document {
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
            })
            .collect();

        Ok(documents)
    }

    pub async fn get_documents_count_with_role_and_filter(&self, user_id: Uuid, user_role: crate::models::UserRole, ocr_status: Option<&str>) -> Result<i64> {
        let count = match (user_role == crate::models::UserRole::Admin, ocr_status) {
            (true, Some(status)) => {
                // Admin with OCR filter
                sqlx::query_scalar::<_, i64>(
                    "SELECT COUNT(*) FROM documents WHERE ocr_status = $1"
                )
                .bind(status)
                .fetch_one(&self.pool)
                .await?
            }
            (true, None) => {
                // Admin without OCR filter
                sqlx::query_scalar::<_, i64>(
                    "SELECT COUNT(*) FROM documents"
                )
                .fetch_one(&self.pool)
                .await?
            }
            (false, Some(status)) => {
                // Regular user with OCR filter
                sqlx::query_scalar::<_, i64>(
                    "SELECT COUNT(*) FROM documents WHERE user_id = $1 AND ocr_status = $2"
                )
                .bind(user_id)
                .bind(status)
                .fetch_one(&self.pool)
                .await?
            }
            (false, None) => {
                // Regular user without OCR filter
                sqlx::query_scalar::<_, i64>(
                    "SELECT COUNT(*) FROM documents WHERE user_id = $1"
                )
                .bind(user_id)
                .fetch_one(&self.pool)
                .await?
            }
        };

        Ok(count)
    }

    pub async fn get_documents_by_user(&self, user_id: Uuid, limit: i64, offset: i64) -> Result<Vec<Document>> {
        let rows = sqlx::query(
            r#"
            SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, ocr_retry_count, ocr_failure_reason, tags, created_at, updated_at, user_id, file_hash, original_created_at, original_modified_at, source_metadata
            FROM documents 
            WHERE user_id = $1 
            ORDER BY created_at DESC 
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let documents = rows
            .into_iter()
            .map(|row| Document {
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
            })
            .collect();

        Ok(documents)
    }

    pub async fn find_documents_by_filename(&self, filename: &str) -> Result<Vec<Document>> {
        let rows = sqlx::query(
            r#"
            SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, ocr_retry_count, ocr_failure_reason, tags, created_at, updated_at, user_id, file_hash, original_created_at, original_modified_at, source_metadata
            FROM documents 
            WHERE filename = $1 OR original_filename = $1
            ORDER BY created_at DESC
            "#
        )
        .bind(filename)
        .fetch_all(&self.pool)
        .await?;

        let documents = rows
            .into_iter()
            .map(|row| Document {
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
            })
            .collect();

        Ok(documents)
    }

    pub async fn search_documents(&self, user_id: Uuid, search: SearchRequest) -> Result<(Vec<Document>, i64)> {
        let mut query_builder = QueryBuilder::new(
            r#"
            SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, ocr_retry_count, ocr_failure_reason, tags, created_at, updated_at, user_id, file_hash, original_created_at, original_modified_at, source_metadata,
                   ts_rank(to_tsvector('english', COALESCE(content, '') || ' ' || COALESCE(ocr_text, '')), plainto_tsquery('english', "# 
        );
        
        query_builder.push_bind(&search.query);
        query_builder.push(")) as rank FROM documents WHERE user_id = ");
        query_builder.push_bind(user_id);
        query_builder.push(" AND to_tsvector('english', COALESCE(content, '') || ' ' || COALESCE(ocr_text, '')) @@ plainto_tsquery('english', ");
        query_builder.push_bind(&search.query);
        query_builder.push(")");

        if let Some(tags) = &search.tags {
            if !tags.is_empty() {
                query_builder.push(" AND tags && ");
                query_builder.push_bind(tags);
            }
        }

        if let Some(mime_types) = &search.mime_types {
            if !mime_types.is_empty() {
                query_builder.push(" AND mime_type = ANY(");
                query_builder.push_bind(mime_types);
                query_builder.push(")");
            }
        }

        query_builder.push(" ORDER BY rank DESC, created_at DESC");
        
        if let Some(limit) = search.limit {
            query_builder.push(" LIMIT ");
            query_builder.push_bind(limit);
        }
        
        if let Some(offset) = search.offset {
            query_builder.push(" OFFSET ");
            query_builder.push_bind(offset);
        }

        let rows = query_builder.build().fetch_all(&self.pool).await?;

        let documents = rows
            .into_iter()
            .map(|row| Document {
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
            })
            .collect();

        let total_row = sqlx::query(
            r#"
            SELECT COUNT(*) as total FROM documents 
            WHERE user_id = $1 
            AND to_tsvector('english', COALESCE(content, '') || ' ' || COALESCE(ocr_text, '')) @@ plainto_tsquery('english', $2)
            "#
        )
        .bind(user_id)
        .bind(&search.query)
        .fetch_one(&self.pool)
        .await?;

        let total: i64 = total_row.get("total");

        Ok((documents, total))
    }

    pub async fn enhanced_search_documents_with_role(&self, user_id: Uuid, user_role: crate::models::UserRole, search: SearchRequest) -> Result<(Vec<EnhancedDocumentResponse>, i64, u64)> {
        let start_time = std::time::Instant::now();
        
        // Build search query based on search mode with enhanced substring matching
        let search_mode = search.search_mode.as_ref().unwrap_or(&SearchMode::Simple);
        
        // For fuzzy mode, we'll use similarity matching which is better for substrings
        let use_similarity = matches!(search_mode, SearchMode::Fuzzy);
        
        let user_filter = if user_role == crate::models::UserRole::Admin {
            // Admins can search all documents
            ""
        } else {
            // Regular users can only search their own documents
            " AND user_id = "
        };
        
        let mut query_builder = if use_similarity {
            // Use trigram similarity for substring matching
            let mut builder = QueryBuilder::new(
                r#"
                SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, ocr_retry_count, ocr_failure_reason, tags, created_at, updated_at, user_id, file_hash, original_created_at, original_modified_at, source_metadata,
                       GREATEST(
                           similarity(filename, "#
            );
            builder.push_bind(&search.query);
            builder.push(r#"),
                           similarity(COALESCE(content, '') || ' ' || COALESCE(ocr_text, ''), "#);
            builder.push_bind(&search.query);
            builder.push(r#"),
                           ts_rank(to_tsvector('english', COALESCE(content, '') || ' ' || COALESCE(ocr_text, '')), plainto_tsquery('english', "#);
            builder.push_bind(&search.query);
            builder.push(r#"))
                       ) as rank
                FROM documents 
                WHERE (
                    filename % "#);
            builder.push_bind(&search.query);
            builder.push(r#" OR
                    (COALESCE(content, '') || ' ' || COALESCE(ocr_text, '')) % "#);
            builder.push_bind(&search.query);
            builder.push(r#" OR
                    to_tsvector('english', COALESCE(content, '') || ' ' || COALESCE(ocr_text, '')) @@ plainto_tsquery('english', "#);
            builder.push_bind(&search.query);
            builder.push(r#")
                )"#);
                
            if !user_filter.is_empty() {
                builder.push(user_filter);
                builder.push_bind(user_id);
            }
            
            builder
        } else {
            // Use traditional full-text search with enhanced ranking
            let query_function = match search_mode {
                SearchMode::Simple => "plainto_tsquery",
                SearchMode::Phrase => "phraseto_tsquery", 
                SearchMode::Boolean => "to_tsquery",
                SearchMode::Fuzzy => "plainto_tsquery", // fallback
            };

            let mut builder = QueryBuilder::new(&format!(
                r#"
                SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, ocr_retry_count, ocr_failure_reason, tags, created_at, updated_at, user_id, file_hash, original_created_at, original_modified_at, source_metadata,
                       GREATEST(
                           CASE WHEN filename ILIKE '%' || "#
            ));
            builder.push_bind(&search.query);
            builder.push(&format!(r#" || '%' THEN 0.8 ELSE 0 END,
                           ts_rank(to_tsvector('english', COALESCE(content, '') || ' ' || COALESCE(ocr_text, '')), {}('english', "#, query_function));
            builder.push_bind(&search.query);
            builder.push(&format!(r#"))
                       ) as rank
                FROM documents 
                WHERE (
                    filename ILIKE '%' || "#));
            builder.push_bind(&search.query);
            builder.push(&format!(r#" || '%' OR
                    to_tsvector('english', COALESCE(content, '') || ' ' || COALESCE(ocr_text, '')) @@ {}('english', "#, query_function));
            builder.push_bind(&search.query);
            builder.push(r#")
                )"#);
                
            if !user_filter.is_empty() {
                builder.push(user_filter);
                builder.push_bind(user_id);
            }
            
            builder
        };

        if let Some(tags) = &search.tags {
            if !tags.is_empty() {
                query_builder.push(" AND tags && ");
                query_builder.push_bind(tags);
            }
        }

        if let Some(mime_types) = &search.mime_types {
            if !mime_types.is_empty() {
                query_builder.push(" AND mime_type = ANY(");
                query_builder.push_bind(mime_types);
                query_builder.push(")");
            }
        }

        query_builder.push(" ORDER BY rank DESC, created_at DESC");
        
        if let Some(limit) = search.limit {
            query_builder.push(" LIMIT ");
            query_builder.push_bind(limit);
        }
        
        if let Some(offset) = search.offset {
            query_builder.push(" OFFSET ");
            query_builder.push_bind(offset);
        }

        let rows = query_builder.build().fetch_all(&self.pool).await?;

        let include_snippets = search.include_snippets.unwrap_or(true);
        let snippet_length = search.snippet_length.unwrap_or(200);

        let mut documents = Vec::new();
        for row in rows {
            let doc_id: Uuid = row.get("id");
            let content: Option<String> = row.get("content");
            let ocr_text: Option<String> = row.get("ocr_text");
            let rank: f32 = row.get("rank");

            let snippets = if include_snippets {
                self.generate_snippets(&search.query, content.as_deref(), ocr_text.as_deref(), snippet_length)
            } else {
                Vec::new()
            };

            documents.push(EnhancedDocumentResponse {
                id: doc_id,
                filename: row.get("filename"),
                original_filename: row.get("original_filename"),
                file_size: row.get("file_size"),
                mime_type: row.get("mime_type"),
                tags: row.get("tags"),
                created_at: row.get("created_at"),
                has_ocr_text: ocr_text.is_some(),
                ocr_confidence: row.get("ocr_confidence"),
                ocr_word_count: row.get("ocr_word_count"),
                ocr_processing_time_ms: row.get("ocr_processing_time_ms"),
                ocr_status: row.get("ocr_status"),
                search_rank: Some(rank),
                snippets,
            });
        }

        // Get the query function for total count
        let query_function = if use_similarity {
            "plainto_tsquery"
        } else {
            match search_mode {
                SearchMode::Simple => "plainto_tsquery",
                SearchMode::Phrase => "phraseto_tsquery", 
                SearchMode::Boolean => "to_tsquery",
                SearchMode::Fuzzy => "plainto_tsquery",
            }
        };

        let total_row = if user_role == crate::models::UserRole::Admin {
            sqlx::query(&format!(
                r#"
                SELECT COUNT(*) as total FROM documents 
                WHERE to_tsvector('english', COALESCE(content, '') || ' ' || COALESCE(ocr_text, '')) @@ {}('english', $1)
                "#, query_function
            ))
            .bind(&search.query)
            .fetch_one(&self.pool)
            .await?
        } else {
            sqlx::query(&format!(
                r#"
                SELECT COUNT(*) as total FROM documents 
                WHERE user_id = $1 
                AND to_tsvector('english', COALESCE(content, '') || ' ' || COALESCE(ocr_text, '')) @@ {}('english', $2)
                "#, query_function
            ))
            .bind(user_id)
            .bind(&search.query)
            .fetch_one(&self.pool)
            .await?
        };

        let total: i64 = total_row.get("total");
        let query_time = start_time.elapsed().as_millis() as u64;

        Ok((documents, total, query_time))
    }

    pub async fn enhanced_search_documents(&self, user_id: Uuid, search: SearchRequest) -> Result<(Vec<EnhancedDocumentResponse>, i64, u64)> {
        let start_time = std::time::Instant::now();
        
        // Build search query based on search mode with enhanced substring matching
        let search_mode = search.search_mode.as_ref().unwrap_or(&SearchMode::Simple);
        
        // For fuzzy mode, we'll use similarity matching which is better for substrings
        let use_similarity = matches!(search_mode, SearchMode::Fuzzy);
        
        let mut query_builder = if use_similarity {
            // Use trigram similarity for substring matching
            let mut builder = QueryBuilder::new(
                r#"
                SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, ocr_retry_count, ocr_failure_reason, tags, created_at, updated_at, user_id, file_hash, original_created_at, original_modified_at, source_metadata,
                       GREATEST(
                           similarity(filename, "#
            );
            builder.push_bind(&search.query);
            builder.push(r#"),
                           similarity(COALESCE(content, '') || ' ' || COALESCE(ocr_text, ''), "#);
            builder.push_bind(&search.query);
            builder.push(r#"),
                           ts_rank(to_tsvector('english', COALESCE(content, '') || ' ' || COALESCE(ocr_text, '')), plainto_tsquery('english', "#);
            builder.push_bind(&search.query);
            builder.push(r#"))
                       ) as rank
                FROM documents 
                WHERE user_id = "#);
            builder.push_bind(user_id);
            builder.push(r#" AND (
                    filename % "#);
            builder.push_bind(&search.query);
            builder.push(r#" OR
                    (COALESCE(content, '') || ' ' || COALESCE(ocr_text, '')) % "#);
            builder.push_bind(&search.query);
            builder.push(r#" OR
                    to_tsvector('english', COALESCE(content, '') || ' ' || COALESCE(ocr_text, '')) @@ plainto_tsquery('english', "#);
            builder.push_bind(&search.query);
            builder.push(r#")
                )"#);
            builder
        } else {
            // Use traditional full-text search with enhanced ranking
            let query_function = match search_mode {
                SearchMode::Simple => "plainto_tsquery",
                SearchMode::Phrase => "phraseto_tsquery", 
                SearchMode::Boolean => "to_tsquery",
                SearchMode::Fuzzy => "plainto_tsquery", // fallback
            };

            let mut builder = QueryBuilder::new(&format!(
                r#"
                SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, ocr_retry_count, ocr_failure_reason, tags, created_at, updated_at, user_id, file_hash, original_created_at, original_modified_at, source_metadata,
                       GREATEST(
                           CASE WHEN filename ILIKE '%' || "#
            ));
            builder.push_bind(&search.query);
            builder.push(&format!(r#" || '%' THEN 0.8 ELSE 0 END,
                           ts_rank(to_tsvector('english', COALESCE(content, '') || ' ' || COALESCE(ocr_text, '')), {}('english', "#, query_function));
            builder.push_bind(&search.query);
            builder.push(&format!(r#"))
                       ) as rank
                FROM documents 
                WHERE user_id = "#));
            builder.push_bind(user_id);
            builder.push(&format!(r#" AND (
                    filename ILIKE '%' || "#));
            builder.push_bind(&search.query);
            builder.push(&format!(r#" || '%' OR
                    to_tsvector('english', COALESCE(content, '') || ' ' || COALESCE(ocr_text, '')) @@ {}('english', "#, query_function));
            builder.push_bind(&search.query);
            builder.push(r#")
                )"#);
            builder
        };

        if let Some(tags) = &search.tags {
            if !tags.is_empty() {
                query_builder.push(" AND tags && ");
                query_builder.push_bind(tags);
            }
        }

        if let Some(mime_types) = &search.mime_types {
            if !mime_types.is_empty() {
                query_builder.push(" AND mime_type = ANY(");
                query_builder.push_bind(mime_types);
                query_builder.push(")");
            }
        }

        query_builder.push(" ORDER BY rank DESC, created_at DESC");
        
        if let Some(limit) = search.limit {
            query_builder.push(" LIMIT ");
            query_builder.push_bind(limit);
        }
        
        if let Some(offset) = search.offset {
            query_builder.push(" OFFSET ");
            query_builder.push_bind(offset);
        }

        let rows = query_builder.build().fetch_all(&self.pool).await?;

        let include_snippets = search.include_snippets.unwrap_or(true);
        let snippet_length = search.snippet_length.unwrap_or(200);

        let mut documents = Vec::new();
        for row in rows {
            let doc_id: Uuid = row.get("id");
            let content: Option<String> = row.get("content");
            let ocr_text: Option<String> = row.get("ocr_text");
            let rank: f32 = row.get("rank");

            let snippets = if include_snippets {
                self.generate_snippets(&search.query, content.as_deref(), ocr_text.as_deref(), snippet_length)
            } else {
                Vec::new()
            };

            documents.push(EnhancedDocumentResponse {
                id: doc_id,
                filename: row.get("filename"),
                original_filename: row.get("original_filename"),
                file_size: row.get("file_size"),
                mime_type: row.get("mime_type"),
                tags: row.get("tags"),
                created_at: row.get("created_at"),
                has_ocr_text: ocr_text.is_some(),
                ocr_confidence: row.get("ocr_confidence"),
                ocr_word_count: row.get("ocr_word_count"),
                ocr_processing_time_ms: row.get("ocr_processing_time_ms"),
                ocr_status: row.get("ocr_status"),
                search_rank: Some(rank),
                snippets,
            });
        }

        // Get the query function for total count
        let query_function = if use_similarity {
            "plainto_tsquery"
        } else {
            match search_mode {
                SearchMode::Simple => "plainto_tsquery",
                SearchMode::Phrase => "phraseto_tsquery", 
                SearchMode::Boolean => "to_tsquery",
                SearchMode::Fuzzy => "plainto_tsquery",
            }
        };

        let total_row = sqlx::query(&format!(
            r#"
            SELECT COUNT(*) as total FROM documents 
            WHERE user_id = $1 
            AND to_tsvector('english', COALESCE(content, '') || ' ' || COALESCE(ocr_text, '')) @@ {}('english', $2)
            "#, query_function
        ))
        .bind(user_id)
        .bind(&search.query)
        .fetch_one(&self.pool)
        .await?;

        let total: i64 = total_row.get("total");
        let query_time = start_time.elapsed().as_millis() as u64;

        Ok((documents, total, query_time))
    }

    fn generate_snippets(&self, query: &str, content: Option<&str>, ocr_text: Option<&str>, snippet_length: i32) -> Vec<SearchSnippet> {
        let mut snippets = Vec::new();
        
        // Combine content and OCR text
        let full_text = match (content, ocr_text) {
            (Some(c), Some(o)) => format!("{} {}", c, o),
            (Some(c), None) => c.to_string(),
            (None, Some(o)) => o.to_string(),
            (None, None) => return snippets,
        };

        // Enhanced substring matching for better context
        let query_terms: Vec<&str> = query.split_whitespace().collect();
        let text_lower = full_text.to_lowercase();
        let query_lower = query.to_lowercase();

        // Find exact matches first
        let mut match_positions = Vec::new();
        
        // 1. Look for exact query matches
        for (i, _) in text_lower.match_indices(&query_lower) {
            match_positions.push((i, query.len(), "exact"));
        }
        
        // 2. Look for individual term matches (substring matching)
        for term in &query_terms {
            if term.len() >= 3 { // Only match terms of reasonable length
                let term_lower = term.to_lowercase();
                for (i, _) in text_lower.match_indices(&term_lower) {
                    // Check if this isn't already part of an exact match
                    let is_duplicate = match_positions.iter().any(|(pos, len, _)| {
                        i >= *pos && i < *pos + *len
                    });
                    if !is_duplicate {
                        match_positions.push((i, term.len(), "term"));
                    }
                }
            }
        }
        
        // 3. Look for partial word matches (for "docu" -> "document" cases)
        for term in &query_terms {
            if term.len() >= 3 {
                let term_lower = term.to_lowercase();
                // Find words that start with our search term
                let words_regex = regex::Regex::new(&format!(r"\b{}[a-zA-Z]*\b", regex::escape(&term_lower))).unwrap();
                for mat in words_regex.find_iter(&text_lower) {
                    let is_duplicate = match_positions.iter().any(|(pos, len, _)| {
                        mat.start() >= *pos && mat.start() < *pos + *len
                    });
                    if !is_duplicate {
                        match_positions.push((mat.start(), mat.end() - mat.start(), "partial"));
                    }
                }
            }
        }

        // Sort matches by position and remove overlaps
        match_positions.sort_by_key(|&(pos, _, _)| pos);
        
        // Generate snippets around matches
        for (match_pos, match_len, _match_type) in match_positions.iter().take(5) {
            let context_size = (snippet_length as usize).saturating_sub(*match_len) / 2;
            
            let snippet_start = match_pos.saturating_sub(context_size);
            let snippet_end = std::cmp::min(
                match_pos + match_len + context_size,
                full_text.len()
            );

            // Find word boundaries to avoid cutting words
            let snippet_start = self.find_word_boundary(&full_text, snippet_start, true);
            let snippet_end = self.find_word_boundary(&full_text, snippet_end, false);

            if snippet_start < snippet_end && snippet_start < full_text.len() {
                let snippet_text = &full_text[snippet_start..snippet_end];
                
                // Find all highlight ranges within this snippet
                let mut highlight_ranges = Vec::new();
                let snippet_lower = snippet_text.to_lowercase();
                
                // Highlight exact query match
                for (match_start, _) in snippet_lower.match_indices(&query_lower) {
                    highlight_ranges.push(HighlightRange {
                        start: match_start as i32,
                        end: (match_start + query.len()) as i32,
                    });
                }
                
                // Highlight individual terms if no exact match
                if highlight_ranges.is_empty() {
                    for term in &query_terms {
                        if term.len() >= 3 {
                            let term_lower = term.to_lowercase();
                            for (match_start, _) in snippet_lower.match_indices(&term_lower) {
                                highlight_ranges.push(HighlightRange {
                                    start: match_start as i32,
                                    end: (match_start + term.len()) as i32,
                                });
                            }
                        }
                    }
                }

                // Remove duplicate highlights and sort
                highlight_ranges.sort_by_key(|r| r.start);
                highlight_ranges.dedup_by_key(|r| r.start);

                snippets.push(SearchSnippet {
                    text: snippet_text.to_string(),
                    start_offset: snippet_start as i32,
                    end_offset: snippet_end as i32,
                    highlight_ranges,
                });

                // Limit to avoid too many snippets
                if snippets.len() >= 3 {
                    break;
                }
            }
        }

        snippets
    }

    fn find_word_boundary(&self, text: &str, mut pos: usize, search_backward: bool) -> usize {
        if pos >= text.len() {
            return text.len();
        }
        
        let chars: Vec<char> = text.chars().collect();
        
        if search_backward {
            // Search backward for word boundary
            while pos > 0 && chars.get(pos.saturating_sub(1)).map_or(false, |c| c.is_alphanumeric()) {
                pos = pos.saturating_sub(1);
            }
        } else {
            // Search forward for word boundary
            while pos < chars.len() && chars.get(pos).map_or(false, |c| c.is_alphanumeric()) {
                pos += 1;
            }
        }
        
        // Convert back to byte position
        chars.iter().take(pos).map(|c| c.len_utf8()).sum()
    }

    pub async fn update_document_ocr(&self, id: Uuid, ocr_text: &str) -> Result<()> {
        sqlx::query("UPDATE documents SET ocr_text = $1, updated_at = NOW() WHERE id = $2")
            .bind(ocr_text)
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn get_recent_documents_for_source(&self, source_id: Uuid, limit: i64) -> Result<Vec<Document>> {
        let rows = sqlx::query(
            r#"SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, ocr_retry_count, ocr_failure_reason, tags, created_at, updated_at, user_id, file_hash, original_created_at, original_modified_at, source_metadata FROM documents 
               WHERE source_id = $1 
               ORDER BY created_at DESC 
               LIMIT $2"#
        )
        .bind(source_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut documents = Vec::new();
        for row in rows {
            documents.push(Document {
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
            });
        }

        Ok(documents)
    }

    pub async fn get_mime_type_facets(&self, user_id: Uuid, user_role: crate::models::UserRole) -> Result<Vec<(String, i64)>> {
        let query = if user_role == crate::models::UserRole::Admin {
            // Admins see facets for all documents
            r#"
            SELECT mime_type, COUNT(*) as count
            FROM documents
            GROUP BY mime_type
            ORDER BY count DESC
            "#
        } else {
            // Regular users see facets for their own documents
            r#"
            SELECT mime_type, COUNT(*) as count
            FROM documents
            WHERE user_id = $1
            GROUP BY mime_type
            ORDER BY count DESC
            "#
        };

        let rows = if user_role == crate::models::UserRole::Admin {
            sqlx::query(query)
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query(query)
                .bind(user_id)
                .fetch_all(&self.pool)
                .await?
        };

        let facets = rows
            .into_iter()
            .map(|row| (row.get("mime_type"), row.get("count")))
            .collect();

        Ok(facets)
    }

    pub async fn get_tag_facets(&self, user_id: Uuid, user_role: crate::models::UserRole) -> Result<Vec<(String, i64)>> {
        let query = if user_role == crate::models::UserRole::Admin {
            // Admins see facets for all documents
            r#"
            SELECT UNNEST(tags) as tag, COUNT(*) as count
            FROM documents
            GROUP BY tag
            ORDER BY count DESC
            "#
        } else {
            // Regular users see facets for their own documents
            r#"
            SELECT UNNEST(tags) as tag, COUNT(*) as count
            FROM documents
            WHERE user_id = $1
            GROUP BY tag
            ORDER BY count DESC
            "#
        };

        let rows = if user_role == crate::models::UserRole::Admin {
            sqlx::query(query)
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query(query)
                .bind(user_id)
                .fetch_all(&self.pool)
                .await?
        };

        let facets = rows
            .into_iter()
            .map(|row| (row.get("tag"), row.get("count")))
            .collect();

        Ok(facets)
    }

    pub async fn get_document_by_id(&self, document_id: Uuid, user_id: Uuid, user_role: crate::models::UserRole) -> Result<Option<Document>> {
        let query = if user_role == crate::models::UserRole::Admin {
            // Admins can see any document
            r#"
            SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, ocr_retry_count, ocr_failure_reason, tags, created_at, updated_at, user_id, file_hash, original_created_at, original_modified_at, source_metadata
            FROM documents 
            WHERE id = $1
            "#
        } else {
            // Regular users can only see their own documents
            r#"
            SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, ocr_retry_count, ocr_failure_reason, tags, created_at, updated_at, user_id, file_hash, original_created_at, original_modified_at, source_metadata
            FROM documents 
            WHERE id = $1 AND user_id = $2
            "#
        };

        let row = if user_role == crate::models::UserRole::Admin {
            sqlx::query(query)
                .bind(document_id)
                .fetch_optional(&self.pool)
                .await?
        } else {
            sqlx::query(query)
                .bind(document_id)
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await?
        };

        match row {
            Some(row) => Ok(Some(Document {
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
            })),
            None => Ok(None),
        }
    }

    /// Check if a document with the given file hash already exists for the user
    pub async fn get_document_by_user_and_hash(&self, user_id: Uuid, file_hash: &str) -> Result<Option<Document>> {
        let row = sqlx::query(
            r#"
            SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, ocr_retry_count, ocr_failure_reason, tags, created_at, updated_at, user_id, file_hash, original_created_at, original_modified_at, source_metadata
            FROM documents 
            WHERE user_id = $1 AND file_hash = $2
            LIMIT 1
            "#
        )
        .bind(user_id)
        .bind(file_hash)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(Document {
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
            })),
            None => Ok(None),
        }
    }

    /// Get documents grouped by duplicate hashes for a user
    pub async fn get_user_duplicates(&self, user_id: Uuid, user_role: crate::models::UserRole, limit: i64, offset: i64) -> Result<(Vec<serde_json::Value>, i64)> {
        let (docs_query, count_query) = if user_role == crate::models::UserRole::Admin {
            // Admins can see all duplicates
            (
                r#"
                SELECT 
                    file_hash,
                    COUNT(*) as duplicate_count,
                    MIN(created_at) as first_uploaded,
                    MAX(created_at) as last_uploaded,
                    json_agg(
                        json_build_object(
                            'id', id,
                            'filename', filename, 
                            'original_filename', original_filename,
                            'file_size', file_size,
                            'mime_type', mime_type,
                            'created_at', created_at,
                            'user_id', user_id
                        ) ORDER BY created_at
                    ) as documents
                FROM documents 
                WHERE file_hash IS NOT NULL
                GROUP BY file_hash 
                HAVING COUNT(*) > 1
                ORDER BY duplicate_count DESC, first_uploaded DESC
                LIMIT $1 OFFSET $2
                "#,
                r#"
                SELECT COUNT(*) as total FROM (
                    SELECT file_hash 
                    FROM documents 
                    WHERE file_hash IS NOT NULL
                    GROUP BY file_hash 
                    HAVING COUNT(*) > 1
                ) as duplicate_groups
                "#
            )
        } else {
            // Regular users see only their own duplicates
            (
                r#"
                SELECT 
                    file_hash,
                    COUNT(*) as duplicate_count,
                    MIN(created_at) as first_uploaded,
                    MAX(created_at) as last_uploaded,
                    json_agg(
                        json_build_object(
                            'id', id,
                            'filename', filename,
                            'original_filename', original_filename,
                            'file_size', file_size,
                            'mime_type', mime_type,
                            'created_at', created_at,
                            'user_id', user_id
                        ) ORDER BY created_at
                    ) as documents
                FROM documents 
                WHERE user_id = $3 AND file_hash IS NOT NULL
                GROUP BY file_hash 
                HAVING COUNT(*) > 1
                ORDER BY duplicate_count DESC, first_uploaded DESC
                LIMIT $1 OFFSET $2
                "#,
                r#"
                SELECT COUNT(*) as total FROM (
                    SELECT file_hash 
                    FROM documents 
                    WHERE user_id = $1 AND file_hash IS NOT NULL
                    GROUP BY file_hash 
                    HAVING COUNT(*) > 1
                ) as duplicate_groups
                "#
            )
        };

        let rows = if user_role == crate::models::UserRole::Admin {
            sqlx::query(docs_query)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query(docs_query)
                .bind(limit)
                .bind(offset)
                .bind(user_id)
                .fetch_all(&self.pool)
                .await?
        };

        let duplicates: Vec<serde_json::Value> = rows
            .into_iter()
            .map(|row| {
                serde_json::json!({
                    "file_hash": row.get::<String, _>("file_hash"),
                    "duplicate_count": row.get::<i64, _>("duplicate_count"),
                    "first_uploaded": row.get::<chrono::DateTime<chrono::Utc>, _>("first_uploaded"),
                    "last_uploaded": row.get::<chrono::DateTime<chrono::Utc>, _>("last_uploaded"),
                    "documents": row.get::<serde_json::Value, _>("documents")
                })
            })
            .collect();

        let total = if user_role == crate::models::UserRole::Admin {
            sqlx::query_scalar::<_, i64>(count_query)
                .fetch_one(&self.pool)
                .await?
        } else {
            sqlx::query_scalar::<_, i64>(count_query)
                .bind(user_id)
                .fetch_one(&self.pool)
                .await?
        };

        Ok((duplicates, total))
    }

    pub async fn get_document_labels(&self, document_id: Uuid) -> Result<Vec<Label>> {
        let labels = sqlx::query_as::<_, Label>(
            r#"
            SELECT 
                l.id, l.user_id, l.name, l.description, l.color, 
                l.background_color, l.icon, l.is_system, l.created_at, l.updated_at,
                0::bigint as document_count, 0::bigint as source_count
            FROM labels l
            INNER JOIN document_labels dl ON l.id = dl.label_id
            WHERE dl.document_id = $1
            ORDER BY l.name
            "#
        )
        .bind(document_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(labels)
    }

    pub async fn get_labels_for_documents(&self, document_ids: &[Uuid]) -> Result<std::collections::HashMap<Uuid, Vec<Label>>> {
        if document_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        let rows = sqlx::query(
            r#"
            SELECT 
                dl.document_id,
                l.id, l.user_id, l.name, l.description, l.color, 
                l.background_color, l.icon, l.is_system, l.created_at, l.updated_at
            FROM labels l
            INNER JOIN document_labels dl ON l.id = dl.label_id
            WHERE dl.document_id = ANY($1)
            ORDER BY dl.document_id, l.name
            "#
        )
        .bind(document_ids)
        .fetch_all(&self.pool)
        .await?;

        let mut labels_map: std::collections::HashMap<Uuid, Vec<Label>> = std::collections::HashMap::new();
        
        for row in rows {
            let document_id: Uuid = row.get("document_id");
            let label = Label {
                id: row.get("id"),
                user_id: row.get("user_id"),
                name: row.get("name"),
                description: row.get("description"),
                color: row.get("color"),
                background_color: row.get("background_color"),
                icon: row.get("icon"),
                is_system: row.get("is_system"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                document_count: 0,
                source_count: 0,
            };
            
            labels_map.entry(document_id).or_insert_with(Vec::new).push(label);
        }

        Ok(labels_map)
    }

    pub async fn delete_document(&self, document_id: Uuid, user_id: Uuid, user_role: crate::models::UserRole) -> Result<Option<Document>> {
        let document = if user_role == crate::models::UserRole::Admin {
            let row = sqlx::query(
                r#"
                DELETE FROM documents 
                WHERE id = $1
                RETURNING id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, tags, created_at, updated_at, user_id, file_hash, original_created_at, original_modified_at, source_metadata
                "#,
            )
            .bind(document_id)
            .fetch_optional(&self.pool)
            .await?;

            row.map(|r| Document {
                id: r.get("id"),
                filename: r.get("filename"),
                original_filename: r.get("original_filename"),
                file_path: r.get("file_path"),
                file_size: r.get("file_size"),
                mime_type: r.get("mime_type"),
                content: r.get("content"),
                ocr_text: r.get("ocr_text"),
                ocr_confidence: r.get("ocr_confidence"),
                ocr_word_count: r.get("ocr_word_count"),
                ocr_processing_time_ms: r.get("ocr_processing_time_ms"),
                ocr_status: r.get("ocr_status"),
                ocr_error: r.get("ocr_error"),
                ocr_completed_at: r.get("ocr_completed_at"),
                ocr_retry_count: r.get("ocr_retry_count"),
                ocr_failure_reason: r.get("ocr_failure_reason"),
                tags: r.get("tags"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
                user_id: r.get("user_id"),
                file_hash: r.get("file_hash"),
                original_created_at: r.get("original_created_at"),
                original_modified_at: r.get("original_modified_at"),
                source_metadata: r.get("source_metadata"),
            })
        } else {
            let row = sqlx::query(
                r#"
                DELETE FROM documents 
                WHERE id = $1 AND user_id = $2
                RETURNING id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, tags, created_at, updated_at, user_id, file_hash, original_created_at, original_modified_at, source_metadata
                "#,
            )
            .bind(document_id)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?;

            row.map(|r| Document {
                id: r.get("id"),
                filename: r.get("filename"),
                original_filename: r.get("original_filename"),
                file_path: r.get("file_path"),
                file_size: r.get("file_size"),
                mime_type: r.get("mime_type"),
                content: r.get("content"),
                ocr_text: r.get("ocr_text"),
                ocr_confidence: r.get("ocr_confidence"),
                ocr_word_count: r.get("ocr_word_count"),
                ocr_processing_time_ms: r.get("ocr_processing_time_ms"),
                ocr_status: r.get("ocr_status"),
                ocr_error: r.get("ocr_error"),
                ocr_completed_at: r.get("ocr_completed_at"),
                ocr_retry_count: r.get("ocr_retry_count"),
                ocr_failure_reason: r.get("ocr_failure_reason"),
                tags: r.get("tags"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
                user_id: r.get("user_id"),
                file_hash: r.get("file_hash"),
                original_created_at: r.get("original_created_at"),
                original_modified_at: r.get("original_modified_at"),
                source_metadata: r.get("source_metadata"),
            })
        };

        Ok(document)
    }

    pub async fn bulk_delete_documents(&self, document_ids: &[uuid::Uuid], user_id: uuid::Uuid, user_role: crate::models::UserRole) -> Result<Vec<Document>> {
        if document_ids.is_empty() {
            return Ok(Vec::new());
        }

        let deleted_documents = if user_role == crate::models::UserRole::Admin {
            let rows = sqlx::query(
                r#"
                DELETE FROM documents 
                WHERE id = ANY($1)
                RETURNING id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, tags, created_at, updated_at, user_id, file_hash, original_created_at, original_modified_at, source_metadata
                "#,
            )
            .bind(document_ids)
            .fetch_all(&self.pool)
            .await?;

            rows.into_iter().map(|r| Document {
                id: r.get("id"),
                filename: r.get("filename"),
                original_filename: r.get("original_filename"),
                file_path: r.get("file_path"),
                file_size: r.get("file_size"),
                mime_type: r.get("mime_type"),
                content: r.get("content"),
                ocr_text: r.get("ocr_text"),
                ocr_confidence: r.get("ocr_confidence"),
                ocr_word_count: r.get("ocr_word_count"),
                ocr_processing_time_ms: r.get("ocr_processing_time_ms"),
                ocr_status: r.get("ocr_status"),
                ocr_error: r.get("ocr_error"),
                ocr_completed_at: r.get("ocr_completed_at"),
                ocr_retry_count: r.get("ocr_retry_count"),
                ocr_failure_reason: r.get("ocr_failure_reason"),
                tags: r.get("tags"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
                user_id: r.get("user_id"),
                file_hash: r.get("file_hash"),
                original_created_at: r.get("original_created_at"),
                original_modified_at: r.get("original_modified_at"),
                source_metadata: r.get("source_metadata"),
            }).collect()
        } else {
            let rows = sqlx::query(
                r#"
                DELETE FROM documents 
                WHERE id = ANY($1) AND user_id = $2
                RETURNING id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, tags, created_at, updated_at, user_id, file_hash, original_created_at, original_modified_at, source_metadata
                "#,
            )
            .bind(document_ids)
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;

            rows.into_iter().map(|r| Document {
                id: r.get("id"),
                filename: r.get("filename"),
                original_filename: r.get("original_filename"),
                file_path: r.get("file_path"),
                file_size: r.get("file_size"),
                mime_type: r.get("mime_type"),
                content: r.get("content"),
                ocr_text: r.get("ocr_text"),
                ocr_confidence: r.get("ocr_confidence"),
                ocr_word_count: r.get("ocr_word_count"),
                ocr_processing_time_ms: r.get("ocr_processing_time_ms"),
                ocr_status: r.get("ocr_status"),
                ocr_error: r.get("ocr_error"),
                ocr_completed_at: r.get("ocr_completed_at"),
                ocr_retry_count: r.get("ocr_retry_count"),
                ocr_failure_reason: r.get("ocr_failure_reason"),
                tags: r.get("tags"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
                user_id: r.get("user_id"),
                file_hash: r.get("file_hash"),
                original_created_at: r.get("original_created_at"),
                original_modified_at: r.get("original_modified_at"),
                source_metadata: r.get("source_metadata"),
            }).collect()
        };

        Ok(deleted_documents)
    }


    pub async fn find_documents_by_confidence_threshold(&self, max_confidence: f32, user_id: uuid::Uuid, user_role: crate::models::UserRole) -> Result<Vec<Document>> {
        let documents = if user_role == crate::models::UserRole::Admin {
            let rows = sqlx::query(
                r#"
                SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, ocr_retry_count, ocr_failure_reason, tags, created_at, updated_at, user_id, file_hash, original_created_at, original_modified_at, source_metadata
                FROM documents 
                WHERE ocr_confidence IS NOT NULL AND ocr_confidence < $1
                ORDER BY ocr_confidence ASC, created_at DESC
                "#,
            )
            .bind(max_confidence)
            .fetch_all(&self.pool)
            .await?;

            rows.into_iter().map(|r| Document {
                id: r.get("id"),
                filename: r.get("filename"),
                original_filename: r.get("original_filename"),
                file_path: r.get("file_path"),
                file_size: r.get("file_size"),
                mime_type: r.get("mime_type"),
                content: r.get("content"),
                ocr_text: r.get("ocr_text"),
                ocr_confidence: r.get("ocr_confidence"),
                ocr_word_count: r.get("ocr_word_count"),
                ocr_processing_time_ms: r.get("ocr_processing_time_ms"),
                ocr_status: r.get("ocr_status"),
                ocr_error: r.get("ocr_error"),
                ocr_completed_at: r.get("ocr_completed_at"),
                ocr_retry_count: r.get("ocr_retry_count"),
                ocr_failure_reason: r.get("ocr_failure_reason"),
                tags: r.get("tags"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
                user_id: r.get("user_id"),
                file_hash: r.get("file_hash"),
                original_created_at: r.get("original_created_at"),
                original_modified_at: r.get("original_modified_at"),
                source_metadata: r.get("source_metadata"),
            }).collect()
        } else {
            let rows = sqlx::query(
                r#"
                SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, ocr_retry_count, ocr_failure_reason, tags, created_at, updated_at, user_id, file_hash, original_created_at, original_modified_at, source_metadata
                FROM documents 
                WHERE ocr_confidence IS NOT NULL AND ocr_confidence < $1 AND user_id = $2
                ORDER BY ocr_confidence ASC, created_at DESC
                "#,
            )
            .bind(max_confidence)
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;

            rows.into_iter().map(|r| Document {
                id: r.get("id"),
                filename: r.get("filename"),
                original_filename: r.get("original_filename"),
                file_path: r.get("file_path"),
                file_size: r.get("file_size"),
                mime_type: r.get("mime_type"),
                content: r.get("content"),
                ocr_text: r.get("ocr_text"),
                ocr_confidence: r.get("ocr_confidence"),
                ocr_word_count: r.get("ocr_word_count"),
                ocr_processing_time_ms: r.get("ocr_processing_time_ms"),
                ocr_status: r.get("ocr_status"),
                ocr_error: r.get("ocr_error"),
                ocr_completed_at: r.get("ocr_completed_at"),
                ocr_retry_count: r.get("ocr_retry_count"),
                ocr_failure_reason: r.get("ocr_failure_reason"),
                tags: r.get("tags"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
                user_id: r.get("user_id"),
                file_hash: r.get("file_hash"),
                original_created_at: r.get("original_created_at"),
                original_modified_at: r.get("original_modified_at"),
                source_metadata: r.get("source_metadata"),
            }).collect()
        };

        Ok(documents)
    }

    /// Find documents with failed OCR processing
    pub async fn find_failed_ocr_documents(&self, user_id: uuid::Uuid, user_role: crate::models::UserRole) -> Result<Vec<Document>> {
        let documents = if user_role == crate::models::UserRole::Admin {
            let rows = sqlx::query(
                r#"
                SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, ocr_retry_count, ocr_failure_reason, tags, created_at, updated_at, user_id, file_hash, original_created_at, original_modified_at, source_metadata
                FROM documents 
                WHERE ocr_status = 'failed' OR (ocr_confidence IS NULL AND ocr_status != 'pending' AND ocr_status != 'processing')
                ORDER BY created_at DESC
                "#,
            )
            .fetch_all(&self.pool)
            .await?;

            rows.into_iter().map(|r| Document {
                id: r.get("id"),
                filename: r.get("filename"),
                original_filename: r.get("original_filename"),
                file_path: r.get("file_path"),
                file_size: r.get("file_size"),
                mime_type: r.get("mime_type"),
                content: r.get("content"),
                ocr_text: r.get("ocr_text"),
                ocr_confidence: r.get("ocr_confidence"),
                ocr_word_count: r.get("ocr_word_count"),
                ocr_processing_time_ms: r.get("ocr_processing_time_ms"),
                ocr_status: r.get("ocr_status"),
                ocr_error: r.get("ocr_error"),
                ocr_completed_at: r.get("ocr_completed_at"),
                ocr_retry_count: r.get("ocr_retry_count"),
                ocr_failure_reason: r.get("ocr_failure_reason"),
                tags: r.get("tags"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
                user_id: r.get("user_id"),
                file_hash: r.get("file_hash"),
                original_created_at: r.get("original_created_at"),
                original_modified_at: r.get("original_modified_at"),
                source_metadata: r.get("source_metadata"),
            }).collect()
        } else {
            let rows = sqlx::query(
                r#"
                SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, ocr_retry_count, ocr_failure_reason, tags, created_at, updated_at, user_id, file_hash, original_created_at, original_modified_at, source_metadata
                FROM documents 
                WHERE (ocr_status = 'failed' OR (ocr_confidence IS NULL AND ocr_status != 'pending' AND ocr_status != 'processing')) AND user_id = $1
                ORDER BY created_at DESC
                "#,
            )
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;

            rows.into_iter().map(|r| Document {
                id: r.get("id"),
                filename: r.get("filename"),
                original_filename: r.get("original_filename"),
                file_path: r.get("file_path"),
                file_size: r.get("file_size"),
                mime_type: r.get("mime_type"),
                content: r.get("content"),
                ocr_text: r.get("ocr_text"),
                ocr_confidence: r.get("ocr_confidence"),
                ocr_word_count: r.get("ocr_word_count"),
                ocr_processing_time_ms: r.get("ocr_processing_time_ms"),
                ocr_status: r.get("ocr_status"),
                ocr_error: r.get("ocr_error"),
                ocr_completed_at: r.get("ocr_completed_at"),
                ocr_retry_count: r.get("ocr_retry_count"),
                ocr_failure_reason: r.get("ocr_failure_reason"),
                tags: r.get("tags"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
                user_id: r.get("user_id"),
                file_hash: r.get("file_hash"),
                original_created_at: r.get("original_created_at"),
                original_modified_at: r.get("original_modified_at"),
                source_metadata: r.get("source_metadata"),
            }).collect()
        };

        Ok(documents)
    }

    /// Find documents with low confidence or failed OCR (combined)
    pub async fn find_low_confidence_and_failed_documents(&self, max_confidence: f32, user_id: uuid::Uuid, user_role: crate::models::UserRole) -> Result<Vec<Document>> {
        let documents = if user_role == crate::models::UserRole::Admin {
            let rows = sqlx::query(
                r#"
                SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, ocr_retry_count, ocr_failure_reason, tags, created_at, updated_at, user_id, file_hash, original_created_at, original_modified_at, source_metadata
                FROM documents 
                WHERE (ocr_confidence IS NOT NULL AND ocr_confidence < $1) 
                   OR ocr_status = 'failed'
                ORDER BY 
                    CASE WHEN ocr_confidence IS NOT NULL THEN ocr_confidence ELSE -1 END ASC, 
                    created_at DESC
                "#,
            )
            .bind(max_confidence)
            .fetch_all(&self.pool)
            .await?;

            rows.into_iter().map(|r| Document {
                id: r.get("id"),
                filename: r.get("filename"),
                original_filename: r.get("original_filename"),
                file_path: r.get("file_path"),
                file_size: r.get("file_size"),
                mime_type: r.get("mime_type"),
                content: r.get("content"),
                ocr_text: r.get("ocr_text"),
                ocr_confidence: r.get("ocr_confidence"),
                ocr_word_count: r.get("ocr_word_count"),
                ocr_processing_time_ms: r.get("ocr_processing_time_ms"),
                ocr_status: r.get("ocr_status"),
                ocr_error: r.get("ocr_error"),
                ocr_completed_at: r.get("ocr_completed_at"),
                ocr_retry_count: r.get("ocr_retry_count"),
                ocr_failure_reason: r.get("ocr_failure_reason"),
                tags: r.get("tags"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
                user_id: r.get("user_id"),
                file_hash: r.get("file_hash"),
                original_created_at: r.get("original_created_at"),
                original_modified_at: r.get("original_modified_at"),
                source_metadata: r.get("source_metadata"),
            }).collect()
        } else {
            let rows = sqlx::query(
                r#"
                SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, ocr_retry_count, ocr_failure_reason, tags, created_at, updated_at, user_id, file_hash, original_created_at, original_modified_at, source_metadata
                FROM documents 
                WHERE ((ocr_confidence IS NOT NULL AND ocr_confidence < $1) 
                    OR ocr_status = 'failed')
                  AND user_id = $2
                ORDER BY 
                    CASE WHEN ocr_confidence IS NOT NULL THEN ocr_confidence ELSE -1 END ASC, 
                    created_at DESC
                "#,
            )
            .bind(max_confidence)
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;

            rows.into_iter().map(|r| Document {
                id: r.get("id"),
                filename: r.get("filename"),
                original_filename: r.get("original_filename"),
                file_path: r.get("file_path"),
                file_size: r.get("file_size"),
                mime_type: r.get("mime_type"),
                content: r.get("content"),
                ocr_text: r.get("ocr_text"),
                ocr_confidence: r.get("ocr_confidence"),
                ocr_word_count: r.get("ocr_word_count"),
                ocr_processing_time_ms: r.get("ocr_processing_time_ms"),
                ocr_status: r.get("ocr_status"),
                ocr_error: r.get("ocr_error"),
                ocr_completed_at: r.get("ocr_completed_at"),
                ocr_retry_count: r.get("ocr_retry_count"),
                ocr_failure_reason: r.get("ocr_failure_reason"),
                tags: r.get("tags"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
                user_id: r.get("user_id"),
                file_hash: r.get("file_hash"),
                original_created_at: r.get("original_created_at"),
                original_modified_at: r.get("original_modified_at"),
                source_metadata: r.get("source_metadata"),
            }).collect()
        };

        Ok(documents)
    }

    pub async fn count_documents_for_source(&self, source_id: Uuid) -> Result<(i64, i64)> {
        let row = sqlx::query(
            r#"
            SELECT 
                COUNT(*) as total_documents,
                COUNT(CASE WHEN ocr_status = 'completed' AND ocr_text IS NOT NULL THEN 1 END) as total_documents_ocr
            FROM documents 
            WHERE source_id = $1
            "#
        )
        .bind(source_id)
        .fetch_one(&self.pool)
        .await?;

        let total_documents: i64 = row.get("total_documents");
        let total_documents_ocr: i64 = row.get("total_documents_ocr");

        Ok((total_documents, total_documents_ocr))
    }

    pub async fn count_documents_for_sources(&self, source_ids: &[Uuid]) -> Result<Vec<(Uuid, i64, i64)>> {
        if source_ids.is_empty() {
            return Ok(vec![]);
        }

        let query = format!(
            r#"
            SELECT 
                source_id,
                COUNT(*) as total_documents,
                COUNT(CASE WHEN ocr_status = 'completed' AND ocr_text IS NOT NULL THEN 1 END) as total_documents_ocr
            FROM documents 
            WHERE source_id = ANY($1)
            GROUP BY source_id
            "#
        );

        let rows = sqlx::query(&query)
            .bind(source_ids)
            .fetch_all(&self.pool)
            .await?;

        let results = rows
            .into_iter()
            .map(|row| {
                let source_id: Uuid = row.get("source_id");
                let total_documents: i64 = row.get("total_documents");
                let total_documents_ocr: i64 = row.get("total_documents_ocr");
                (source_id, total_documents, total_documents_ocr)
            })
            .collect();

        Ok(results)
    }

    /// Create a failed document record
    pub async fn create_failed_document(
        &self,
        user_id: Uuid,
        filename: String,
        original_filename: Option<String>,
        original_path: Option<String>,
        file_path: Option<String>,
        file_size: Option<i64>,
        file_hash: Option<String>,
        mime_type: Option<String>,
        content: Option<String>,
        tags: Vec<String>,
        ocr_text: Option<String>,
        ocr_confidence: Option<f32>,
        ocr_word_count: Option<i32>,
        ocr_processing_time_ms: Option<i32>,
        failure_reason: String,
        failure_stage: String,
        existing_document_id: Option<Uuid>,
        ingestion_source: String,
        error_message: Option<String>,
        retry_count: Option<i32>,
    ) -> Result<Uuid> {
        let id = Uuid::new_v4();
        
        sqlx::query(
            r#"
            INSERT INTO failed_documents (
                id, user_id, filename, original_filename, original_path, file_path,
                file_size, file_hash, mime_type, content, tags, ocr_text, 
                ocr_confidence, ocr_word_count, ocr_processing_time_ms,
                failure_reason, failure_stage, existing_document_id,
                ingestion_source, error_message, retry_count, created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, 
                $16, $17, $18, $19, $20, $21, NOW(), NOW()
            )
            "#
        )
        .bind(id)
        .bind(user_id)
        .bind(&filename)
        .bind(&original_filename)
        .bind(&original_path)
        .bind(&file_path)
        .bind(file_size)
        .bind(&file_hash)
        .bind(&mime_type)
        .bind(&content)
        .bind(&tags)
        .bind(&ocr_text)
        .bind(ocr_confidence)
        .bind(ocr_word_count)
        .bind(ocr_processing_time_ms)
        .bind(&failure_reason)
        .bind(&failure_stage)
        .bind(existing_document_id)
        .bind(&ingestion_source)
        .bind(&error_message)
        .bind(retry_count)
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    /// Create a failed document from an existing document that failed OCR
    pub async fn create_failed_document_from_document(
        &self,
        document: &Document,
        failure_reason: String,
        error_message: Option<String>,
        retry_count: Option<i32>,
    ) -> Result<Uuid> {
        self.create_failed_document(
            document.user_id, // user_id is required in Document struct
            document.filename.clone(),
            Some(document.original_filename.clone()),
            None, // original_path - not available in Document model
            Some(document.file_path.clone()),
            Some(document.file_size),
            document.file_hash.clone(),
            Some(document.mime_type.clone()),
            document.content.clone(),
            document.tags.clone(),
            document.ocr_text.clone(),
            document.ocr_confidence,
            document.ocr_word_count,
            document.ocr_processing_time_ms,
            failure_reason,
            "ocr".to_string(), // OCR failure stage
            None, // existing_document_id
            "unknown".to_string(), // Default ingestion source - would need to be passed in for better tracking
            error_message,
            retry_count,
        ).await
    }
}