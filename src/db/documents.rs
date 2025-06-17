use anyhow::Result;
use sqlx::{Row, QueryBuilder};
use uuid::Uuid;

use crate::models::{Document, SearchRequest, SearchMode, SearchSnippet, HighlightRange, EnhancedDocumentResponse};
use super::Database;

impl Database {
    pub async fn create_document(&self, document: Document) -> Result<Document> {
        let row = sqlx::query(
            r#"
            INSERT INTO documents (id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, tags, created_at, updated_at, user_id, file_hash)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)
            RETURNING id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, tags, created_at, updated_at, user_id, file_hash
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
            tags: row.get("tags"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            user_id: row.get("user_id"),
            file_hash: row.get("file_hash"),
        })
    }

    pub async fn get_documents_by_user_with_role(&self, user_id: Uuid, user_role: crate::models::UserRole, limit: i64, offset: i64) -> Result<Vec<Document>> {
        let query = if user_role == crate::models::UserRole::Admin {
            // Admins can see all documents
            r#"
            SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, tags, created_at, updated_at, user_id, file_hash
            FROM documents 
            ORDER BY created_at DESC 
            LIMIT $1 OFFSET $2
            "#
        } else {
            // Regular users can only see their own documents
            r#"
            SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, tags, created_at, updated_at, user_id, file_hash
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
                tags: row.get("tags"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                user_id: row.get("user_id"),
                file_hash: row.get("file_hash"),
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
                    SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, tags, created_at, updated_at, user_id
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
                    SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, tags, created_at, updated_at, user_id
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
                    SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, tags, created_at, updated_at, user_id
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
                    SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, tags, created_at, updated_at, user_id
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
                tags: row.get("tags"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                user_id: row.get("user_id"),
                file_hash: row.get("file_hash"),
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
            SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, tags, created_at, updated_at, user_id
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
                tags: row.get("tags"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                user_id: row.get("user_id"),
                file_hash: row.get("file_hash"),
            })
            .collect();

        Ok(documents)
    }

    pub async fn find_documents_by_filename(&self, filename: &str) -> Result<Vec<Document>> {
        let rows = sqlx::query(
            r#"
            SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, tags, created_at, updated_at, user_id
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
                tags: row.get("tags"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                user_id: row.get("user_id"),
                file_hash: row.get("file_hash"),
            })
            .collect();

        Ok(documents)
    }

    pub async fn search_documents(&self, user_id: Uuid, search: SearchRequest) -> Result<(Vec<Document>, i64)> {
        let mut query_builder = QueryBuilder::new(
            r#"
            SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, tags, created_at, updated_at, user_id,
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
                tags: row.get("tags"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                user_id: row.get("user_id"),
                file_hash: row.get("file_hash"),
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
                SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, tags, created_at, updated_at, user_id,
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
                SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, tags, created_at, updated_at, user_id,
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
                SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, tags, created_at, updated_at, user_id,
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
                SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, tags, created_at, updated_at, user_id,
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
            r#"SELECT * FROM documents 
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
                tags: row.get("tags"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                user_id: row.get("user_id"),
                file_hash: row.get("file_hash"),
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
            SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, tags, created_at, updated_at, user_id
            FROM documents 
            WHERE id = $1
            "#
        } else {
            // Regular users can only see their own documents
            r#"
            SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, tags, created_at, updated_at, user_id
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
                tags: row.get("tags"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                user_id: row.get("user_id"),
                file_hash: row.get("file_hash"),
            })),
            None => Ok(None),
        }
    }

    /// Check if a document with the given file hash already exists for the user
    pub async fn get_document_by_user_and_hash(&self, user_id: Uuid, file_hash: &str) -> Result<Option<Document>> {
        let row = sqlx::query(
            r#"
            SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, tags, created_at, updated_at, user_id, file_hash
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
                tags: row.get("tags"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                user_id: row.get("user_id"),
                file_hash: row.get("file_hash"),
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
}