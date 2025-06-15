use anyhow::Result;
use chrono::Utc;
use sqlx::{PgPool, Row, postgres::PgPoolOptions};
use std::time::Duration;
use uuid::Uuid;
use tokio::time::{sleep, timeout};

use crate::models::{CreateUser, Document, SearchRequest, SearchMode, SearchSnippet, HighlightRange, EnhancedDocumentResponse, User};

#[derive(Clone)]
pub struct Database {
    pub pool: PgPool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(50)                          // Increased from 20 to handle more concurrent requests
            .acquire_timeout(Duration::from_secs(10))     // Increased from 3 to 10 seconds
            .idle_timeout(Duration::from_secs(600))       // 10 minute idle timeout
            .max_lifetime(Duration::from_secs(1800))      // 30 minute max lifetime
            .min_connections(5)                           // Maintain minimum connections
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }

    pub async fn new_with_pool_config(database_url: &str, max_connections: u32, min_connections: u32) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .acquire_timeout(Duration::from_secs(10))
            .idle_timeout(Duration::from_secs(600))
            .max_lifetime(Duration::from_secs(1800))
            .min_connections(min_connections)
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }
    
    pub fn get_pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn with_retry<T, F, Fut>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        const MAX_RETRIES: usize = 3;
        const BASE_DELAY_MS: u64 = 100;
        
        for attempt in 0..MAX_RETRIES {
            match timeout(Duration::from_secs(15), operation()).await {
                Ok(Ok(result)) => return Ok(result),
                Ok(Err(e)) if attempt == MAX_RETRIES - 1 => return Err(e),
                Ok(Err(e)) => {
                    tracing::warn!("Database operation failed, attempt {} of {}: {}", attempt + 1, MAX_RETRIES, e);
                }
                Err(_) if attempt == MAX_RETRIES - 1 => {
                    return Err(anyhow::anyhow!("Database operation timed out after {} retries", MAX_RETRIES));
                }
                Err(_) => {
                    tracing::warn!("Database operation timed out, attempt {} of {}", attempt + 1, MAX_RETRIES);
                }
            }
            
            // Exponential backoff with jitter
            let delay_ms = BASE_DELAY_MS * (2_u64.pow(attempt as u32));
            let jitter = (std::ptr::addr_of!(attempt) as usize) % (delay_ms as usize / 2 + 1);
            sleep(Duration::from_millis(delay_ms + jitter as u64)).await;
        }
        
        unreachable!()
    }

    pub async fn migrate(&self) -> Result<()> {
        // Create extensions
        sqlx::query(r#"CREATE EXTENSION IF NOT EXISTS "uuid-ossp""#)
            .execute(&self.pool)
            .await?;
        
        sqlx::query(r#"CREATE EXTENSION IF NOT EXISTS "pg_trgm""#)
            .execute(&self.pool)
            .await?;
        
        // Create users table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                username VARCHAR(255) UNIQUE NOT NULL,
                email VARCHAR(255) UNIQUE NOT NULL,
                password_hash VARCHAR(255) NOT NULL,
                role VARCHAR(10) DEFAULT 'user',
                created_at TIMESTAMPTZ DEFAULT NOW(),
                updated_at TIMESTAMPTZ DEFAULT NOW()
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        
        
        // Create documents table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS documents (
                id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                filename VARCHAR(255) NOT NULL,
                original_filename VARCHAR(255) NOT NULL,
                file_path VARCHAR(500) NOT NULL,
                file_size BIGINT NOT NULL,
                mime_type VARCHAR(100) NOT NULL,
                content TEXT,
                ocr_text TEXT,
                tags TEXT[] DEFAULT '{}',
                created_at TIMESTAMPTZ DEFAULT NOW(),
                updated_at TIMESTAMPTZ DEFAULT NOW(),
                user_id UUID REFERENCES users(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        
        // Create indexes
        sqlx::query(r#"CREATE INDEX IF NOT EXISTS idx_documents_user_id ON documents(user_id)"#)
            .execute(&self.pool)
            .await?;
        
        sqlx::query(r#"CREATE INDEX IF NOT EXISTS idx_documents_filename ON documents(filename)"#)
            .execute(&self.pool)
            .await?;
        
        sqlx::query(r#"CREATE INDEX IF NOT EXISTS idx_documents_mime_type ON documents(mime_type)"#)
            .execute(&self.pool)
            .await?;
        
        sqlx::query(r#"CREATE INDEX IF NOT EXISTS idx_documents_tags ON documents USING GIN(tags)"#)
            .execute(&self.pool)
            .await?;
        
        sqlx::query(r#"CREATE INDEX IF NOT EXISTS idx_documents_content_search ON documents USING GIN(to_tsvector('english', COALESCE(content, '') || ' ' || COALESCE(ocr_text, '')))"#)
            .execute(&self.pool)
            .await?;
        
        // Enhanced indexes for substring matching and similarity
        sqlx::query(r#"CREATE INDEX IF NOT EXISTS idx_documents_filename_trgm ON documents USING GIN(filename gin_trgm_ops)"#)
            .execute(&self.pool)
            .await?;
        
        sqlx::query(r#"CREATE INDEX IF NOT EXISTS idx_documents_content_trgm ON documents USING GIN((COALESCE(content, '') || ' ' || COALESCE(ocr_text, '')) gin_trgm_ops)"#)
            .execute(&self.pool)
            .await?;
        
        // Create settings table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS settings (
                id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                user_id UUID REFERENCES users(id) ON DELETE CASCADE UNIQUE,
                ocr_language VARCHAR(10) DEFAULT 'eng',
                concurrent_ocr_jobs INT DEFAULT 4,
                ocr_timeout_seconds INT DEFAULT 300,
                max_file_size_mb INT DEFAULT 50,
                allowed_file_types TEXT[] DEFAULT ARRAY['pdf', 'png', 'jpg', 'jpeg', 'tiff', 'bmp', 'txt'],
                auto_rotate_images BOOLEAN DEFAULT TRUE,
                enable_image_preprocessing BOOLEAN DEFAULT TRUE,
                search_results_per_page INT DEFAULT 25,
                search_snippet_length INT DEFAULT 200,
                fuzzy_search_threshold REAL DEFAULT 0.8,
                retention_days INT,
                enable_auto_cleanup BOOLEAN DEFAULT FALSE,
                enable_compression BOOLEAN DEFAULT FALSE,
                memory_limit_mb INT DEFAULT 512,
                cpu_priority VARCHAR(10) DEFAULT 'normal',
                enable_background_ocr BOOLEAN DEFAULT TRUE,
                ocr_page_segmentation_mode INT DEFAULT 3,
                ocr_engine_mode INT DEFAULT 3,
                ocr_min_confidence REAL DEFAULT 30.0,
                ocr_dpi INT DEFAULT 300,
                ocr_enhance_contrast BOOLEAN DEFAULT TRUE,
                ocr_remove_noise BOOLEAN DEFAULT TRUE,
                ocr_detect_orientation BOOLEAN DEFAULT TRUE,
                ocr_whitelist_chars TEXT,
                ocr_blacklist_chars TEXT,
                webdav_enabled BOOLEAN DEFAULT FALSE,
                webdav_server_url TEXT,
                webdav_username TEXT,
                webdav_password TEXT,
                webdav_watch_folders TEXT[] DEFAULT ARRAY['/Documents']::TEXT[],
                webdav_file_extensions TEXT[] DEFAULT ARRAY['pdf', 'png', 'jpg', 'jpeg', 'tiff', 'bmp', 'txt']::TEXT[],
                webdav_auto_sync BOOLEAN DEFAULT FALSE,
                webdav_sync_interval_minutes INTEGER DEFAULT 60,
                created_at TIMESTAMPTZ DEFAULT NOW(),
                updated_at TIMESTAMPTZ DEFAULT NOW()
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        
        // Run OCR queue migration - execute each statement separately
        self.run_ocr_queue_migration().await?;

        Ok(())
    }

    async fn run_ocr_queue_migration(&self) -> Result<()> {
        // Create OCR queue table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS ocr_queue (
                id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                document_id UUID REFERENCES documents(id) ON DELETE CASCADE,
                status VARCHAR(20) DEFAULT 'pending',
                priority INT DEFAULT 5,
                attempts INT DEFAULT 0,
                max_attempts INT DEFAULT 3,
                created_at TIMESTAMPTZ DEFAULT NOW(),
                started_at TIMESTAMPTZ,
                completed_at TIMESTAMPTZ,
                error_message TEXT,
                worker_id VARCHAR(100),
                processing_time_ms INT,
                file_size BIGINT,
                CONSTRAINT check_status CHECK (status IN ('pending', 'processing', 'completed', 'failed', 'cancelled'))
            )
            "#
        )
        .execute(&self.pool)
        .await?;

        // Create indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_ocr_queue_status ON ocr_queue(status, priority DESC, created_at)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_ocr_queue_document_id ON ocr_queue(document_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_ocr_queue_worker ON ocr_queue(worker_id) WHERE status = 'processing'")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_ocr_queue_created_at ON ocr_queue(created_at) WHERE status = 'pending'")
            .execute(&self.pool)
            .await?;

        // Add columns to documents table
        sqlx::query("ALTER TABLE documents ADD COLUMN IF NOT EXISTS ocr_status VARCHAR(20) DEFAULT 'pending'")
            .execute(&self.pool)
            .await?;

        sqlx::query("ALTER TABLE documents ADD COLUMN IF NOT EXISTS ocr_error TEXT")
            .execute(&self.pool)
            .await?;

        sqlx::query("ALTER TABLE documents ADD COLUMN IF NOT EXISTS ocr_completed_at TIMESTAMPTZ")
            .execute(&self.pool)
            .await?;

        // Create metrics table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS ocr_metrics (
                id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                date DATE DEFAULT CURRENT_DATE,
                hour INT DEFAULT EXTRACT(HOUR FROM NOW()),
                total_processed INT DEFAULT 0,
                total_failed INT DEFAULT 0,
                total_retried INT DEFAULT 0,
                avg_processing_time_ms INT,
                max_processing_time_ms INT,
                min_processing_time_ms INT,
                queue_depth INT,
                active_workers INT,
                UNIQUE(date, hour)
            )
            "#
        )
        .execute(&self.pool)
        .await?;

        // Create the statistics function
        sqlx::query(
            r#"
            CREATE OR REPLACE FUNCTION get_ocr_queue_stats()
            RETURNS TABLE (
                pending_count BIGINT,
                processing_count BIGINT,
                failed_count BIGINT,
                completed_today BIGINT,
                avg_wait_time_minutes DOUBLE PRECISION,
                oldest_pending_minutes DOUBLE PRECISION
            ) AS $$
            BEGIN
                RETURN QUERY
                SELECT 
                    COUNT(*) FILTER (WHERE status = 'pending') as pending_count,
                    COUNT(*) FILTER (WHERE status = 'processing') as processing_count,
                    COUNT(*) FILTER (WHERE status = 'failed' AND attempts >= max_attempts) as failed_count,
                    COUNT(*) FILTER (WHERE status = 'completed' AND completed_at >= CURRENT_DATE) as completed_today,
                    AVG(EXTRACT(EPOCH FROM (COALESCE(started_at, NOW()) - created_at))/60) FILTER (WHERE status IN ('processing', 'completed')) as avg_wait_time_minutes,
                    MAX(EXTRACT(EPOCH FROM (NOW() - created_at))/60) FILTER (WHERE status = 'pending') as oldest_pending_minutes
                FROM ocr_queue;
            END;
            $$ LANGUAGE plpgsql
            "#
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn create_user(&self, user: CreateUser) -> Result<User> {
        let password_hash = bcrypt::hash(&user.password, 12)?;
        let now = Utc::now();

        let row = sqlx::query(
            r#"
            INSERT INTO users (username, email, password_hash, role, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, username, email, password_hash, role, created_at, updated_at
            "#
        )
        .bind(&user.username)
        .bind(&user.email)
        .bind(&password_hash)
        .bind(user.role.as_ref().unwrap_or(&crate::models::UserRole::User).to_string())
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(User {
            id: row.get("id"),
            username: row.get("username"),
            email: row.get("email"),
            password_hash: row.get("password_hash"),
            role: row.get::<String, _>("role").try_into().unwrap_or(crate::models::UserRole::User),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    pub async fn get_user_by_username(&self, username: &str) -> Result<Option<User>> {
        let row = sqlx::query(
            "SELECT id, username, email, password_hash, role, created_at, updated_at FROM users WHERE username = $1"
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(User {
                id: row.get("id"),
                username: row.get("username"),
                email: row.get("email"),
                password_hash: row.get("password_hash"),
                role: row.get::<String, _>("role").try_into().unwrap_or(crate::models::UserRole::User),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })),
            None => Ok(None),
        }
    }

    pub async fn get_user_by_id(&self, id: Uuid) -> Result<Option<User>> {
        let row = sqlx::query(
            "SELECT id, username, email, password_hash, role, created_at, updated_at FROM users WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(User {
                id: row.get("id"),
                username: row.get("username"),
                email: row.get("email"),
                password_hash: row.get("password_hash"),
                role: row.get::<String, _>("role").try_into().unwrap_or(crate::models::UserRole::User),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })),
            None => Ok(None),
        }
    }

    pub async fn create_document(&self, document: Document) -> Result<Document> {
        let row = sqlx::query(
            r#"
            INSERT INTO documents (id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, tags, created_at, updated_at, user_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
            RETURNING id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, tags, created_at, updated_at, user_id
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
        })
    }

    pub async fn get_documents_by_user_with_role(&self, user_id: Uuid, user_role: crate::models::UserRole, limit: i64, offset: i64) -> Result<Vec<Document>> {
        let query = if user_role == crate::models::UserRole::Admin {
            // Admins can see all documents
            r#"
            SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, tags, created_at, updated_at, user_id
            FROM documents 
            ORDER BY created_at DESC 
            LIMIT $1 OFFSET $2
            "#
        } else {
            // Regular users can only see their own documents
            r#"
            SELECT id, filename, original_filename, file_path, file_size, mime_type, content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms, ocr_status, ocr_error, ocr_completed_at, tags, created_at, updated_at, user_id
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
            })
            .collect();

        Ok(documents)
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
            })
            .collect();

        Ok(documents)
    }

    pub async fn search_documents(&self, user_id: Uuid, search: SearchRequest) -> Result<(Vec<Document>, i64)> {
        let mut query_builder = sqlx::QueryBuilder::new(
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
            let mut builder = sqlx::QueryBuilder::new(
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

            let mut builder = sqlx::QueryBuilder::new(&format!(
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
            let mut builder = sqlx::QueryBuilder::new(
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

            let mut builder = sqlx::QueryBuilder::new(&format!(
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

    pub async fn get_all_users(&self) -> Result<Vec<User>> {
        let rows = sqlx::query(
            "SELECT id, username, email, password_hash, role, created_at, updated_at FROM users ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        let users = rows
            .into_iter()
            .map(|row| User {
                id: row.get("id"),
                username: row.get("username"),
                email: row.get("email"),
                password_hash: row.get("password_hash"),
                role: row.get::<String, _>("role").try_into().unwrap_or(crate::models::UserRole::User),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect();

        Ok(users)
    }

    pub async fn update_user(&self, id: Uuid, username: Option<String>, email: Option<String>, password: Option<String>) -> Result<User> {
        let user = self.get_user_by_id(id).await?.ok_or_else(|| anyhow::anyhow!("User not found"))?;
        
        let username = username.unwrap_or(user.username);
        let email = email.unwrap_or(user.email);
        let password_hash = if let Some(pwd) = password {
            bcrypt::hash(&pwd, 12)?
        } else {
            user.password_hash
        };

        let row = sqlx::query(
            r#"
            UPDATE users SET username = $1, email = $2, password_hash = $3, updated_at = NOW()
            WHERE id = $4
            RETURNING id, username, email, password_hash, role, created_at, updated_at
            "#
        )
        .bind(&username)
        .bind(&email)
        .bind(&password_hash)
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(User {
            id: row.get("id"),
            username: row.get("username"),
            email: row.get("email"),
            password_hash: row.get("password_hash"),
            role: row.get::<String, _>("role").try_into().unwrap_or(crate::models::UserRole::User),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    pub async fn delete_user(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn get_user_settings(&self, user_id: Uuid) -> Result<Option<crate::models::Settings>> {
        self.with_retry(|| async {
            let row = sqlx::query(
                r#"SELECT id, user_id, ocr_language, concurrent_ocr_jobs, ocr_timeout_seconds,
                   max_file_size_mb, allowed_file_types, auto_rotate_images, enable_image_preprocessing,
                   search_results_per_page, search_snippet_length, fuzzy_search_threshold,
                   retention_days, enable_auto_cleanup, enable_compression, memory_limit_mb,
                   cpu_priority, enable_background_ocr, ocr_page_segmentation_mode, ocr_engine_mode,
                   ocr_min_confidence, ocr_dpi, ocr_enhance_contrast, ocr_remove_noise,
                   ocr_detect_orientation, ocr_whitelist_chars, ocr_blacklist_chars,
                   ocr_brightness_boost, ocr_contrast_multiplier, ocr_noise_reduction_level, ocr_sharpening_strength,
                   ocr_morphological_operations, ocr_adaptive_threshold_window_size, ocr_histogram_equalization,
                   ocr_upscale_factor, ocr_max_image_width, ocr_max_image_height, save_processed_images,
                   ocr_quality_threshold_brightness, ocr_quality_threshold_contrast, ocr_quality_threshold_noise,
                   ocr_quality_threshold_sharpness, ocr_skip_enhancement,
                   webdav_enabled, webdav_server_url, webdav_username, webdav_password,
                   webdav_watch_folders, webdav_file_extensions, webdav_auto_sync, webdav_sync_interval_minutes,
                   created_at, updated_at
                   FROM settings WHERE user_id = $1"#
            )
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("Database query failed: {}", e))?;

        match row {
            Some(row) => Ok(Some(crate::models::Settings {
                id: row.get("id"),
                user_id: row.get("user_id"),
                ocr_language: row.get("ocr_language"),
                concurrent_ocr_jobs: row.get("concurrent_ocr_jobs"),
                ocr_timeout_seconds: row.get("ocr_timeout_seconds"),
                max_file_size_mb: row.get("max_file_size_mb"),
                allowed_file_types: row.get("allowed_file_types"),
                auto_rotate_images: row.get("auto_rotate_images"),
                enable_image_preprocessing: row.get("enable_image_preprocessing"),
                search_results_per_page: row.get("search_results_per_page"),
                search_snippet_length: row.get("search_snippet_length"),
                fuzzy_search_threshold: row.get("fuzzy_search_threshold"),
                retention_days: row.get("retention_days"),
                enable_auto_cleanup: row.get("enable_auto_cleanup"),
                enable_compression: row.get("enable_compression"),
                memory_limit_mb: row.get("memory_limit_mb"),
                cpu_priority: row.get("cpu_priority"),
                enable_background_ocr: row.get("enable_background_ocr"),
                ocr_page_segmentation_mode: row.get("ocr_page_segmentation_mode"),
                ocr_engine_mode: row.get("ocr_engine_mode"),
                ocr_min_confidence: row.get("ocr_min_confidence"),
                ocr_dpi: row.get("ocr_dpi"),
                ocr_enhance_contrast: row.get("ocr_enhance_contrast"),
                ocr_remove_noise: row.get("ocr_remove_noise"),
                ocr_detect_orientation: row.get("ocr_detect_orientation"),
                ocr_whitelist_chars: row.get("ocr_whitelist_chars"),
                ocr_blacklist_chars: row.get("ocr_blacklist_chars"),
                ocr_brightness_boost: row.get("ocr_brightness_boost"),
                ocr_contrast_multiplier: row.get("ocr_contrast_multiplier"),
                ocr_noise_reduction_level: row.get("ocr_noise_reduction_level"),
                ocr_sharpening_strength: row.get("ocr_sharpening_strength"),
                ocr_morphological_operations: row.get("ocr_morphological_operations"),
                ocr_adaptive_threshold_window_size: row.get("ocr_adaptive_threshold_window_size"),
                ocr_histogram_equalization: row.get("ocr_histogram_equalization"),
                ocr_upscale_factor: row.get("ocr_upscale_factor"),
                ocr_max_image_width: row.get("ocr_max_image_width"),
                ocr_max_image_height: row.get("ocr_max_image_height"),
                save_processed_images: row.get("save_processed_images"),
                ocr_quality_threshold_brightness: row.get("ocr_quality_threshold_brightness"),
                ocr_quality_threshold_contrast: row.get("ocr_quality_threshold_contrast"),
                ocr_quality_threshold_noise: row.get("ocr_quality_threshold_noise"),
                ocr_quality_threshold_sharpness: row.get("ocr_quality_threshold_sharpness"),
                ocr_skip_enhancement: row.get("ocr_skip_enhancement"),
                webdav_enabled: row.get("webdav_enabled"),
                webdav_server_url: row.get("webdav_server_url"),
                webdav_username: row.get("webdav_username"),
                webdav_password: row.get("webdav_password"),
                webdav_watch_folders: row.get("webdav_watch_folders"),
                webdav_file_extensions: row.get("webdav_file_extensions"),
                webdav_auto_sync: row.get("webdav_auto_sync"),
                webdav_sync_interval_minutes: row.get("webdav_sync_interval_minutes"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })),
            None => Ok(None),
        }
        }).await
    }

    pub async fn get_all_user_settings(&self) -> Result<Vec<crate::models::Settings>> {
        let rows = sqlx::query(
            r#"SELECT id, user_id, ocr_language, concurrent_ocr_jobs, ocr_timeout_seconds,
               max_file_size_mb, allowed_file_types, auto_rotate_images, enable_image_preprocessing,
               search_results_per_page, search_snippet_length, fuzzy_search_threshold,
               retention_days, enable_auto_cleanup, enable_compression, memory_limit_mb,
               cpu_priority, enable_background_ocr, ocr_page_segmentation_mode, ocr_engine_mode,
               ocr_min_confidence, ocr_dpi, ocr_enhance_contrast, ocr_remove_noise,
               ocr_detect_orientation, ocr_whitelist_chars, ocr_blacklist_chars,
               ocr_brightness_boost, ocr_contrast_multiplier, ocr_noise_reduction_level, ocr_sharpening_strength,
               ocr_morphological_operations, ocr_adaptive_threshold_window_size, ocr_histogram_equalization,
               ocr_upscale_factor, ocr_max_image_width, ocr_max_image_height, save_processed_images,
               ocr_quality_threshold_brightness, ocr_quality_threshold_contrast, ocr_quality_threshold_noise,
               ocr_quality_threshold_sharpness, ocr_skip_enhancement,
               webdav_enabled, webdav_server_url, webdav_username, webdav_password,
               webdav_watch_folders, webdav_file_extensions, webdav_auto_sync, webdav_sync_interval_minutes,
               created_at, updated_at
               FROM settings
               WHERE webdav_enabled = true AND webdav_auto_sync = true"#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut settings_list = Vec::new();
        for row in rows {
            settings_list.push(crate::models::Settings {
                id: row.get("id"),
                user_id: row.get("user_id"),
                ocr_language: row.get("ocr_language"),
                concurrent_ocr_jobs: row.get("concurrent_ocr_jobs"),
                ocr_timeout_seconds: row.get("ocr_timeout_seconds"),
                max_file_size_mb: row.get("max_file_size_mb"),
                allowed_file_types: row.get("allowed_file_types"),
                auto_rotate_images: row.get("auto_rotate_images"),
                enable_image_preprocessing: row.get("enable_image_preprocessing"),
                search_results_per_page: row.get("search_results_per_page"),
                search_snippet_length: row.get("search_snippet_length"),
                fuzzy_search_threshold: row.get("fuzzy_search_threshold"),
                retention_days: row.get("retention_days"),
                enable_auto_cleanup: row.get("enable_auto_cleanup"),
                enable_compression: row.get("enable_compression"),
                memory_limit_mb: row.get("memory_limit_mb"),
                cpu_priority: row.get("cpu_priority"),
                enable_background_ocr: row.get("enable_background_ocr"),
                ocr_page_segmentation_mode: row.get("ocr_page_segmentation_mode"),
                ocr_engine_mode: row.get("ocr_engine_mode"),
                ocr_min_confidence: row.get("ocr_min_confidence"),
                ocr_dpi: row.get("ocr_dpi"),
                ocr_enhance_contrast: row.get("ocr_enhance_contrast"),
                ocr_remove_noise: row.get("ocr_remove_noise"),
                ocr_detect_orientation: row.get("ocr_detect_orientation"),
                ocr_whitelist_chars: row.get("ocr_whitelist_chars"),
                ocr_blacklist_chars: row.get("ocr_blacklist_chars"),
                ocr_brightness_boost: row.get("ocr_brightness_boost"),
                ocr_contrast_multiplier: row.get("ocr_contrast_multiplier"),
                ocr_noise_reduction_level: row.get("ocr_noise_reduction_level"),
                ocr_sharpening_strength: row.get("ocr_sharpening_strength"),
                ocr_morphological_operations: row.get("ocr_morphological_operations"),
                ocr_adaptive_threshold_window_size: row.get("ocr_adaptive_threshold_window_size"),
                ocr_histogram_equalization: row.get("ocr_histogram_equalization"),
                ocr_upscale_factor: row.get("ocr_upscale_factor"),
                ocr_max_image_width: row.get("ocr_max_image_width"),
                ocr_max_image_height: row.get("ocr_max_image_height"),
                save_processed_images: row.get("save_processed_images"),
                ocr_quality_threshold_brightness: row.get("ocr_quality_threshold_brightness"),
                ocr_quality_threshold_contrast: row.get("ocr_quality_threshold_contrast"),
                ocr_quality_threshold_noise: row.get("ocr_quality_threshold_noise"),
                ocr_quality_threshold_sharpness: row.get("ocr_quality_threshold_sharpness"),
                ocr_skip_enhancement: row.get("ocr_skip_enhancement"),
                webdav_enabled: row.get("webdav_enabled"),
                webdav_server_url: row.get("webdav_server_url"),
                webdav_username: row.get("webdav_username"),
                webdav_password: row.get("webdav_password"),
                webdav_watch_folders: row.get("webdav_watch_folders"),
                webdav_file_extensions: row.get("webdav_file_extensions"),
                webdav_auto_sync: row.get("webdav_auto_sync"),
                webdav_sync_interval_minutes: row.get("webdav_sync_interval_minutes"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(settings_list)
    }

    pub async fn create_or_update_settings(&self, user_id: Uuid, settings: &crate::models::UpdateSettings) -> Result<crate::models::Settings> {
        // Get existing settings to merge with updates
        let existing = self.get_user_settings(user_id).await?;
        let defaults = crate::models::Settings::default();
        
        // Merge existing/defaults with updates
        let current = existing.unwrap_or_else(|| {
            let mut s = defaults;
            s.user_id = user_id;
            s
        });
        
        let row = sqlx::query(
            r#"
            INSERT INTO settings (
                user_id, ocr_language, concurrent_ocr_jobs, ocr_timeout_seconds,
                max_file_size_mb, allowed_file_types, auto_rotate_images, enable_image_preprocessing,
                search_results_per_page, search_snippet_length, fuzzy_search_threshold,
                retention_days, enable_auto_cleanup, enable_compression, memory_limit_mb,
                cpu_priority, enable_background_ocr, ocr_page_segmentation_mode, ocr_engine_mode,
                ocr_min_confidence, ocr_dpi, ocr_enhance_contrast, ocr_remove_noise,
                ocr_detect_orientation, ocr_whitelist_chars, ocr_blacklist_chars,
                ocr_brightness_boost, ocr_contrast_multiplier, ocr_noise_reduction_level, ocr_sharpening_strength,
                ocr_morphological_operations, ocr_adaptive_threshold_window_size, ocr_histogram_equalization,
                ocr_upscale_factor, ocr_max_image_width, ocr_max_image_height, save_processed_images,
                ocr_quality_threshold_brightness, ocr_quality_threshold_contrast, ocr_quality_threshold_noise,
                ocr_quality_threshold_sharpness, ocr_skip_enhancement,
                webdav_enabled, webdav_server_url, webdav_username, webdav_password,
                webdav_watch_folders, webdav_file_extensions, webdav_auto_sync, webdav_sync_interval_minutes
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32, $33, $34, $35, $36, $37, $38, $39, $40, $41, $42, $43, $44, $45, $46, $47, $48, $49, $50)
            ON CONFLICT (user_id) DO UPDATE SET
                ocr_language = $2,
                concurrent_ocr_jobs = $3,
                ocr_timeout_seconds = $4,
                max_file_size_mb = $5,
                allowed_file_types = $6,
                auto_rotate_images = $7,
                enable_image_preprocessing = $8,
                search_results_per_page = $9,
                search_snippet_length = $10,
                fuzzy_search_threshold = $11,
                retention_days = $12,
                enable_auto_cleanup = $13,
                enable_compression = $14,
                memory_limit_mb = $15,
                cpu_priority = $16,
                enable_background_ocr = $17,
                ocr_page_segmentation_mode = $18,
                ocr_engine_mode = $19,
                ocr_min_confidence = $20,
                ocr_dpi = $21,
                ocr_enhance_contrast = $22,
                ocr_remove_noise = $23,
                ocr_detect_orientation = $24,
                ocr_whitelist_chars = $25,
                ocr_blacklist_chars = $26,
                ocr_brightness_boost = $27,
                ocr_contrast_multiplier = $28,
                ocr_noise_reduction_level = $29,
                ocr_sharpening_strength = $30,
                ocr_morphological_operations = $31,
                ocr_adaptive_threshold_window_size = $32,
                ocr_histogram_equalization = $33,
                ocr_upscale_factor = $34,
                ocr_max_image_width = $35,
                ocr_max_image_height = $36,
                save_processed_images = $37,
                ocr_quality_threshold_brightness = $38,
                ocr_quality_threshold_contrast = $39,
                ocr_quality_threshold_noise = $40,
                ocr_quality_threshold_sharpness = $41,
                ocr_skip_enhancement = $42,
                webdav_enabled = $43,
                webdav_server_url = $44,
                webdav_username = $45,
                webdav_password = $46,
                webdav_watch_folders = $47,
                webdav_file_extensions = $48,
                webdav_auto_sync = $49,
                webdav_sync_interval_minutes = $50,
                updated_at = NOW()
            RETURNING id, user_id, ocr_language, concurrent_ocr_jobs, ocr_timeout_seconds,
                      max_file_size_mb, allowed_file_types, auto_rotate_images, enable_image_preprocessing,
                      search_results_per_page, search_snippet_length, fuzzy_search_threshold,
                      retention_days, enable_auto_cleanup, enable_compression, memory_limit_mb,
                      cpu_priority, enable_background_ocr, ocr_page_segmentation_mode, ocr_engine_mode,
                      ocr_min_confidence, ocr_dpi, ocr_enhance_contrast, ocr_remove_noise,
                      ocr_detect_orientation, ocr_whitelist_chars, ocr_blacklist_chars,
                      ocr_brightness_boost, ocr_contrast_multiplier, ocr_noise_reduction_level, ocr_sharpening_strength,
                      ocr_morphological_operations, ocr_adaptive_threshold_window_size, ocr_histogram_equalization,
                      ocr_upscale_factor, ocr_max_image_width, ocr_max_image_height, save_processed_images,
                      ocr_quality_threshold_brightness, ocr_quality_threshold_contrast, ocr_quality_threshold_noise,
                      ocr_quality_threshold_sharpness, ocr_skip_enhancement,
                      webdav_enabled, webdav_server_url, webdav_username, webdav_password,
                      webdav_watch_folders, webdav_file_extensions, webdav_auto_sync, webdav_sync_interval_minutes,
                      created_at, updated_at
            "#
        )
        .bind(user_id)
        .bind(settings.ocr_language.as_ref().unwrap_or(&current.ocr_language))
        .bind(settings.concurrent_ocr_jobs.unwrap_or(current.concurrent_ocr_jobs))
        .bind(settings.ocr_timeout_seconds.unwrap_or(current.ocr_timeout_seconds))
        .bind(settings.max_file_size_mb.unwrap_or(current.max_file_size_mb))
        .bind(settings.allowed_file_types.as_ref().unwrap_or(&current.allowed_file_types))
        .bind(settings.auto_rotate_images.unwrap_or(current.auto_rotate_images))
        .bind(settings.enable_image_preprocessing.unwrap_or(current.enable_image_preprocessing))
        .bind(settings.search_results_per_page.unwrap_or(current.search_results_per_page))
        .bind(settings.search_snippet_length.unwrap_or(current.search_snippet_length))
        .bind(settings.fuzzy_search_threshold.unwrap_or(current.fuzzy_search_threshold))
        .bind(settings.retention_days.unwrap_or(current.retention_days))
        .bind(settings.enable_auto_cleanup.unwrap_or(current.enable_auto_cleanup))
        .bind(settings.enable_compression.unwrap_or(current.enable_compression))
        .bind(settings.memory_limit_mb.unwrap_or(current.memory_limit_mb))
        .bind(settings.cpu_priority.as_ref().unwrap_or(&current.cpu_priority))
        .bind(settings.enable_background_ocr.unwrap_or(current.enable_background_ocr))
        .bind(settings.ocr_page_segmentation_mode.unwrap_or(current.ocr_page_segmentation_mode))
        .bind(settings.ocr_engine_mode.unwrap_or(current.ocr_engine_mode))
        .bind(settings.ocr_min_confidence.unwrap_or(current.ocr_min_confidence))
        .bind(settings.ocr_dpi.unwrap_or(current.ocr_dpi))
        .bind(settings.ocr_enhance_contrast.unwrap_or(current.ocr_enhance_contrast))
        .bind(settings.ocr_remove_noise.unwrap_or(current.ocr_remove_noise))
        .bind(settings.ocr_detect_orientation.unwrap_or(current.ocr_detect_orientation))
        .bind(settings.ocr_whitelist_chars.as_ref().unwrap_or(&current.ocr_whitelist_chars))
        .bind(settings.ocr_blacklist_chars.as_ref().unwrap_or(&current.ocr_blacklist_chars))
        .bind(settings.ocr_brightness_boost.unwrap_or(current.ocr_brightness_boost))
        .bind(settings.ocr_contrast_multiplier.unwrap_or(current.ocr_contrast_multiplier))
        .bind(settings.ocr_noise_reduction_level.unwrap_or(current.ocr_noise_reduction_level))
        .bind(settings.ocr_sharpening_strength.unwrap_or(current.ocr_sharpening_strength))
        .bind(settings.ocr_morphological_operations.unwrap_or(current.ocr_morphological_operations))
        .bind(settings.ocr_adaptive_threshold_window_size.unwrap_or(current.ocr_adaptive_threshold_window_size))
        .bind(settings.ocr_histogram_equalization.unwrap_or(current.ocr_histogram_equalization))
        .bind(settings.ocr_upscale_factor.unwrap_or(current.ocr_upscale_factor))
        .bind(settings.ocr_max_image_width.unwrap_or(current.ocr_max_image_width))
        .bind(settings.ocr_max_image_height.unwrap_or(current.ocr_max_image_height))
        .bind(settings.save_processed_images.unwrap_or(current.save_processed_images))
        .bind(settings.ocr_quality_threshold_brightness.unwrap_or(current.ocr_quality_threshold_brightness))
        .bind(settings.ocr_quality_threshold_contrast.unwrap_or(current.ocr_quality_threshold_contrast))
        .bind(settings.ocr_quality_threshold_noise.unwrap_or(current.ocr_quality_threshold_noise))
        .bind(settings.ocr_quality_threshold_sharpness.unwrap_or(current.ocr_quality_threshold_sharpness))
        .bind(settings.ocr_skip_enhancement.unwrap_or(current.ocr_skip_enhancement))
        .bind(settings.webdav_enabled.unwrap_or(current.webdav_enabled))
        .bind(settings.webdav_server_url.as_ref().unwrap_or(&current.webdav_server_url))
        .bind(settings.webdav_username.as_ref().unwrap_or(&current.webdav_username))
        .bind(settings.webdav_password.as_ref().unwrap_or(&current.webdav_password))
        .bind(settings.webdav_watch_folders.as_ref().unwrap_or(&current.webdav_watch_folders))
        .bind(settings.webdav_file_extensions.as_ref().unwrap_or(&current.webdav_file_extensions))
        .bind(settings.webdav_auto_sync.unwrap_or(current.webdav_auto_sync))
        .bind(settings.webdav_sync_interval_minutes.unwrap_or(current.webdav_sync_interval_minutes))
        .fetch_one(&self.pool)
        .await?;

        Ok(crate::models::Settings {
            id: row.get("id"),
            user_id: row.get("user_id"),
            ocr_language: row.get("ocr_language"),
            concurrent_ocr_jobs: row.get("concurrent_ocr_jobs"),
            ocr_timeout_seconds: row.get("ocr_timeout_seconds"),
            max_file_size_mb: row.get("max_file_size_mb"),
            allowed_file_types: row.get("allowed_file_types"),
            auto_rotate_images: row.get("auto_rotate_images"),
            enable_image_preprocessing: row.get("enable_image_preprocessing"),
            search_results_per_page: row.get("search_results_per_page"),
            search_snippet_length: row.get("search_snippet_length"),
            fuzzy_search_threshold: row.get("fuzzy_search_threshold"),
            retention_days: row.get("retention_days"),
            enable_auto_cleanup: row.get("enable_auto_cleanup"),
            enable_compression: row.get("enable_compression"),
            memory_limit_mb: row.get("memory_limit_mb"),
            cpu_priority: row.get("cpu_priority"),
            enable_background_ocr: row.get("enable_background_ocr"),
            ocr_page_segmentation_mode: row.get("ocr_page_segmentation_mode"),
            ocr_engine_mode: row.get("ocr_engine_mode"),
            ocr_min_confidence: row.get("ocr_min_confidence"),
            ocr_dpi: row.get("ocr_dpi"),
            ocr_enhance_contrast: row.get("ocr_enhance_contrast"),
            ocr_remove_noise: row.get("ocr_remove_noise"),
            ocr_detect_orientation: row.get("ocr_detect_orientation"),
            ocr_whitelist_chars: row.get("ocr_whitelist_chars"),
            ocr_blacklist_chars: row.get("ocr_blacklist_chars"),
            ocr_brightness_boost: row.get("ocr_brightness_boost"),
            ocr_contrast_multiplier: row.get("ocr_contrast_multiplier"),
            ocr_noise_reduction_level: row.get("ocr_noise_reduction_level"),
            ocr_sharpening_strength: row.get("ocr_sharpening_strength"),
            ocr_morphological_operations: row.get("ocr_morphological_operations"),
            ocr_adaptive_threshold_window_size: row.get("ocr_adaptive_threshold_window_size"),
            ocr_histogram_equalization: row.get("ocr_histogram_equalization"),
            ocr_upscale_factor: row.get("ocr_upscale_factor"),
            ocr_max_image_width: row.get("ocr_max_image_width"),
            ocr_max_image_height: row.get("ocr_max_image_height"),
            save_processed_images: row.get("save_processed_images"),
            ocr_quality_threshold_brightness: row.get("ocr_quality_threshold_brightness"),
            ocr_quality_threshold_contrast: row.get("ocr_quality_threshold_contrast"),
            ocr_quality_threshold_noise: row.get("ocr_quality_threshold_noise"),
            ocr_quality_threshold_sharpness: row.get("ocr_quality_threshold_sharpness"),
            ocr_skip_enhancement: row.get("ocr_skip_enhancement"),
            webdav_enabled: row.get("webdav_enabled"),
            webdav_server_url: row.get("webdav_server_url"),
            webdav_username: row.get("webdav_username"),
            webdav_password: row.get("webdav_password"),
            webdav_watch_folders: row.get("webdav_watch_folders"),
            webdav_file_extensions: row.get("webdav_file_extensions"),
            webdav_auto_sync: row.get("webdav_auto_sync"),
            webdav_sync_interval_minutes: row.get("webdav_sync_interval_minutes"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    // Notification methods
    pub async fn create_notification(&self, user_id: Uuid, notification: &crate::models::CreateNotification) -> Result<crate::models::Notification> {
        self.with_retry(|| async {
            let row = sqlx::query(
                r#"INSERT INTO notifications (user_id, notification_type, title, message, action_url, metadata)
                   VALUES ($1, $2, $3, $4, $5, $6)
                   RETURNING id, user_id, notification_type, title, message, read, action_url, metadata, created_at"#
            )
            .bind(user_id)
            .bind(&notification.notification_type)
            .bind(&notification.title)
            .bind(&notification.message)
            .bind(&notification.action_url)
            .bind(&notification.metadata)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("Database insert failed: {}", e))?;

        Ok(crate::models::Notification {
            id: row.get("id"),
            user_id: row.get("user_id"),
            notification_type: row.get("notification_type"),
            title: row.get("title"),
            message: row.get("message"),
            read: row.get("read"),
            action_url: row.get("action_url"),
            metadata: row.get("metadata"),
            created_at: row.get("created_at"),
        })
        }).await
    }

    pub async fn get_user_notifications(&self, user_id: Uuid, limit: i64, offset: i64) -> Result<Vec<crate::models::Notification>> {
        let rows = sqlx::query(
            r#"SELECT id, user_id, notification_type, title, message, read, action_url, metadata, created_at
               FROM notifications 
               WHERE user_id = $1 
               ORDER BY created_at DESC 
               LIMIT $2 OFFSET $3"#
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut notifications = Vec::new();
        for row in rows {
            notifications.push(crate::models::Notification {
                id: row.get("id"),
                user_id: row.get("user_id"),
                notification_type: row.get("notification_type"),
                title: row.get("title"),
                message: row.get("message"),
                read: row.get("read"),
                action_url: row.get("action_url"),
                metadata: row.get("metadata"),
                created_at: row.get("created_at"),
            });
        }

        Ok(notifications)
    }

    pub async fn get_unread_notification_count(&self, user_id: Uuid) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM notifications WHERE user_id = $1 AND read = false")
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;

        Ok(row.get("count"))
    }

    pub async fn mark_notification_read(&self, user_id: Uuid, notification_id: Uuid) -> Result<()> {
        sqlx::query("UPDATE notifications SET read = true WHERE id = $1 AND user_id = $2")
            .bind(notification_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn mark_all_notifications_read(&self, user_id: Uuid) -> Result<()> {
        sqlx::query("UPDATE notifications SET read = true WHERE user_id = $1 AND read = false")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn delete_notification(&self, user_id: Uuid, notification_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM notifications WHERE id = $1 AND user_id = $2")
            .bind(notification_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn get_notification_summary(&self, user_id: Uuid) -> Result<crate::models::NotificationSummary> {
        let unread_count = self.get_unread_notification_count(user_id).await?;
        let recent_notifications = self.get_user_notifications(user_id, 5, 0).await?;

        Ok(crate::models::NotificationSummary {
            unread_count,
            recent_notifications,
        })
    }

    // WebDAV sync state operations
    pub async fn get_webdav_sync_state(&self, user_id: Uuid) -> Result<Option<crate::models::WebDAVSyncState>> {
        self.with_retry(|| async {
            let row = sqlx::query(
                r#"SELECT id, user_id, last_sync_at, sync_cursor, is_running, files_processed, 
                   files_remaining, current_folder, errors, created_at, updated_at
                   FROM webdav_sync_state WHERE user_id = $1"#
            )
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("Database query failed: {}", e))?;

        match row {
            Some(row) => Ok(Some(crate::models::WebDAVSyncState {
                id: row.get("id"),
                user_id: row.get("user_id"),
                last_sync_at: row.get("last_sync_at"),
                sync_cursor: row.get("sync_cursor"),
                is_running: row.get("is_running"),
                files_processed: row.get("files_processed"),
                files_remaining: row.get("files_remaining"),
                current_folder: row.get("current_folder"),
                errors: row.get("errors"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })),
            None => Ok(None),
        }
        }).await
    }

    pub async fn update_webdav_sync_state(&self, user_id: Uuid, state: &crate::models::UpdateWebDAVSyncState) -> Result<()> {
        self.with_retry(|| async {
            sqlx::query(
                r#"INSERT INTO webdav_sync_state (user_id, last_sync_at, sync_cursor, is_running, 
                   files_processed, files_remaining, current_folder, errors, updated_at)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
                   ON CONFLICT (user_id) DO UPDATE SET
                   last_sync_at = EXCLUDED.last_sync_at,
                   sync_cursor = EXCLUDED.sync_cursor,
                   is_running = EXCLUDED.is_running,
                   files_processed = EXCLUDED.files_processed,
                   files_remaining = EXCLUDED.files_remaining,
                   current_folder = EXCLUDED.current_folder,
                   errors = EXCLUDED.errors,
                   updated_at = NOW()"#
            )
            .bind(user_id)
            .bind(state.last_sync_at)
            .bind(&state.sync_cursor)
            .bind(state.is_running)
            .bind(state.files_processed)
            .bind(state.files_remaining)
            .bind(&state.current_folder)
            .bind(&state.errors)
            .execute(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("Database update failed: {}", e))?;

            Ok(())
        }).await
    }

    // Reset any running WebDAV syncs on startup (handles server restart during sync)
    pub async fn reset_running_webdav_syncs(&self) -> Result<i64> {
        let result = sqlx::query(
            r#"UPDATE webdav_sync_state 
               SET is_running = false, 
                   current_folder = NULL,
                   errors = CASE 
                       WHEN array_length(errors, 1) IS NULL OR array_length(errors, 1) = 0 
                       THEN ARRAY['Sync interrupted by server restart']
                       ELSE array_append(errors, 'Sync interrupted by server restart')
                   END,
                   updated_at = NOW()
               WHERE is_running = true"#
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as i64)
    }

    // WebDAV file tracking operations
    pub async fn get_webdav_file_by_path(&self, user_id: Uuid, webdav_path: &str) -> Result<Option<crate::models::WebDAVFile>> {
        let row = sqlx::query(
            r#"SELECT id, user_id, webdav_path, etag, last_modified, file_size, 
               mime_type, document_id, sync_status, sync_error, created_at, updated_at
               FROM webdav_files WHERE user_id = $1 AND webdav_path = $2"#
        )
        .bind(user_id)
        .bind(webdav_path)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(crate::models::WebDAVFile {
                id: row.get("id"),
                user_id: row.get("user_id"),
                webdav_path: row.get("webdav_path"),
                etag: row.get("etag"),
                last_modified: row.get("last_modified"),
                file_size: row.get("file_size"),
                mime_type: row.get("mime_type"),
                document_id: row.get("document_id"),
                sync_status: row.get("sync_status"),
                sync_error: row.get("sync_error"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })),
            None => Ok(None),
        }
    }

    pub async fn create_or_update_webdav_file(&self, file: &crate::models::CreateWebDAVFile) -> Result<crate::models::WebDAVFile> {
        let row = sqlx::query(
            r#"INSERT INTO webdav_files (user_id, webdav_path, etag, last_modified, file_size, 
               mime_type, document_id, sync_status, sync_error)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
               ON CONFLICT (user_id, webdav_path) DO UPDATE SET
               etag = EXCLUDED.etag,
               last_modified = EXCLUDED.last_modified,
               file_size = EXCLUDED.file_size,
               mime_type = EXCLUDED.mime_type,
               document_id = EXCLUDED.document_id,
               sync_status = EXCLUDED.sync_status,
               sync_error = EXCLUDED.sync_error,
               updated_at = NOW()
               RETURNING id, user_id, webdav_path, etag, last_modified, file_size, 
               mime_type, document_id, sync_status, sync_error, created_at, updated_at"#
        )
        .bind(file.user_id)
        .bind(&file.webdav_path)
        .bind(&file.etag)
        .bind(file.last_modified)
        .bind(file.file_size)
        .bind(&file.mime_type)
        .bind(file.document_id)
        .bind(&file.sync_status)
        .bind(&file.sync_error)
        .fetch_one(&self.pool)
        .await?;

        Ok(crate::models::WebDAVFile {
            id: row.get("id"),
            user_id: row.get("user_id"),
            webdav_path: row.get("webdav_path"),
            etag: row.get("etag"),
            last_modified: row.get("last_modified"),
            file_size: row.get("file_size"),
            mime_type: row.get("mime_type"),
            document_id: row.get("document_id"),
            sync_status: row.get("sync_status"),
            sync_error: row.get("sync_error"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    pub async fn get_pending_webdav_files(&self, user_id: Uuid, limit: i64) -> Result<Vec<crate::models::WebDAVFile>> {
        let rows = sqlx::query(
            r#"SELECT id, user_id, webdav_path, etag, last_modified, file_size, 
               mime_type, document_id, sync_status, sync_error, created_at, updated_at
               FROM webdav_files 
               WHERE user_id = $1 AND sync_status = 'pending'
               ORDER BY created_at ASC
               LIMIT $2"#
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut files = Vec::new();
        for row in rows {
            files.push(crate::models::WebDAVFile {
                id: row.get("id"),
                user_id: row.get("user_id"),
                webdav_path: row.get("webdav_path"),
                etag: row.get("etag"),
                last_modified: row.get("last_modified"),
                file_size: row.get("file_size"),
                mime_type: row.get("mime_type"),
                document_id: row.get("document_id"),
                sync_status: row.get("sync_status"),
                sync_error: row.get("sync_error"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(files)
    }

    // Sources methods
    pub async fn create_source(&self, user_id: Uuid, source: &crate::models::CreateSource) -> Result<crate::models::Source> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        
        let row = sqlx::query(
            r#"INSERT INTO sources (id, user_id, name, source_type, enabled, config, status, created_at, updated_at)
               VALUES ($1, $2, $3, $4, $5, $6, 'idle', $7, $8)
               RETURNING *"#
        )
        .bind(id)
        .bind(user_id)
        .bind(&source.name)
        .bind(source.source_type.to_string())
        .bind(source.enabled.unwrap_or(true))
        .bind(&source.config)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(crate::models::Source {
            id: row.get("id"),
            user_id: row.get("user_id"),
            name: row.get("name"),
            source_type: row.get::<String, _>("source_type").try_into().map_err(|e: String| anyhow::anyhow!(e))?,
            enabled: row.get("enabled"),
            config: row.get("config"),
            status: row.get::<String, _>("status").try_into().map_err(|e: String| anyhow::anyhow!(e))?,
            last_sync_at: row.get("last_sync_at"),
            last_error: row.get("last_error"),
            last_error_at: row.get("last_error_at"),
            total_files_synced: row.get("total_files_synced"),
            total_files_pending: row.get("total_files_pending"),
            total_size_bytes: row.get("total_size_bytes"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    pub async fn get_source(&self, user_id: Uuid, source_id: Uuid) -> Result<Option<crate::models::Source>> {
        let row = sqlx::query(
            r#"SELECT * FROM sources WHERE id = $1 AND user_id = $2"#
        )
        .bind(source_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(crate::models::Source {
                id: row.get("id"),
                user_id: row.get("user_id"),
                name: row.get("name"),
                source_type: row.get::<String, _>("source_type").try_into().map_err(|e: String| anyhow::anyhow!(e))?,
                enabled: row.get("enabled"),
                config: row.get("config"),
                status: row.get::<String, _>("status").try_into().map_err(|e: String| anyhow::anyhow!(e))?,
                last_sync_at: row.get("last_sync_at"),
                last_error: row.get("last_error"),
                last_error_at: row.get("last_error_at"),
                total_files_synced: row.get("total_files_synced"),
                total_files_pending: row.get("total_files_pending"),
                total_size_bytes: row.get("total_size_bytes"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })),
            None => Ok(None),
        }
    }

    pub async fn get_sources(&self, user_id: Uuid) -> Result<Vec<crate::models::Source>> {
        let rows = sqlx::query(
            r#"SELECT * FROM sources WHERE user_id = $1 ORDER BY created_at DESC"#
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let mut sources = Vec::new();
        for row in rows {
            sources.push(crate::models::Source {
                id: row.get("id"),
                user_id: row.get("user_id"),
                name: row.get("name"),
                source_type: row.get::<String, _>("source_type").try_into().map_err(|e: String| anyhow::anyhow!(e))?,
                enabled: row.get("enabled"),
                config: row.get("config"),
                status: row.get::<String, _>("status").try_into().map_err(|e: String| anyhow::anyhow!(e))?,
                last_sync_at: row.get("last_sync_at"),
                last_error: row.get("last_error"),
                last_error_at: row.get("last_error_at"),
                total_files_synced: row.get("total_files_synced"),
                total_files_pending: row.get("total_files_pending"),
                total_size_bytes: row.get("total_size_bytes"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(sources)
    }

    pub async fn update_source(&self, user_id: Uuid, source_id: Uuid, update: &crate::models::UpdateSource) -> Result<crate::models::Source> {
        let mut query = String::from("UPDATE sources SET updated_at = NOW()");
        let mut bind_count = 1;

        if update.name.is_some() {
            bind_count += 1;
            query.push_str(&format!(", name = ${}", bind_count));
        }
        if update.enabled.is_some() {
            bind_count += 1;
            query.push_str(&format!(", enabled = ${}", bind_count));
        }
        if update.config.is_some() {
            bind_count += 1;
            query.push_str(&format!(", config = ${}", bind_count));
        }

        bind_count += 1;
        query.push_str(&format!(" WHERE id = ${}", bind_count));
        bind_count += 1;
        query.push_str(&format!(" AND user_id = ${} RETURNING *", bind_count));

        let mut query_builder = sqlx::query(&query);

        // Bind values in order
        if let Some(name) = &update.name {
            query_builder = query_builder.bind(name);
        }
        if let Some(enabled) = &update.enabled {
            query_builder = query_builder.bind(enabled);
        }
        if let Some(config) = &update.config {
            query_builder = query_builder.bind(config);
        }
        query_builder = query_builder.bind(source_id);
        query_builder = query_builder.bind(user_id);

        let row = query_builder.fetch_one(&self.pool).await?;

        Ok(crate::models::Source {
            id: row.get("id"),
            user_id: row.get("user_id"),
            name: row.get("name"),
            source_type: row.get::<String, _>("source_type").try_into().map_err(|e: String| anyhow::anyhow!(e))?,
            enabled: row.get("enabled"),
            config: row.get("config"),
            status: row.get::<String, _>("status").try_into().map_err(|e: String| anyhow::anyhow!(e))?,
            last_sync_at: row.get("last_sync_at"),
            last_error: row.get("last_error"),
            last_error_at: row.get("last_error_at"),
            total_files_synced: row.get("total_files_synced"),
            total_files_pending: row.get("total_files_pending"),
            total_size_bytes: row.get("total_size_bytes"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    pub async fn delete_source(&self, user_id: Uuid, source_id: Uuid) -> Result<bool> {
        let result = sqlx::query(
            r#"DELETE FROM sources WHERE id = $1 AND user_id = $2"#
        )
        .bind(source_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn update_source_status(&self, source_id: Uuid, status: crate::models::SourceStatus, error: Option<String>) -> Result<()> {
        if let Some(error_msg) = error {
            sqlx::query(
                r#"UPDATE sources 
                   SET status = $1, last_error = $2, last_error_at = NOW(), updated_at = NOW()
                   WHERE id = $3"#
            )
            .bind(status.to_string())
            .bind(error_msg)
            .bind(source_id)
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query(
                r#"UPDATE sources 
                   SET status = $1, updated_at = NOW()
                   WHERE id = $2"#
            )
            .bind(status.to_string())
            .bind(source_id)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    pub async fn update_source_sync_stats(&self, source_id: Uuid, files_synced: i64, files_pending: i64, size_bytes: i64) -> Result<()> {
        sqlx::query(
            r#"UPDATE sources 
               SET total_files_synced = $1, total_files_pending = $2, total_size_bytes = $3, 
                   last_sync_at = NOW(), updated_at = NOW()
               WHERE id = $4"#
        )
        .bind(files_synced)
        .bind(files_pending)
        .bind(size_bytes)
        .bind(source_id)
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
            });
        }

        Ok(documents)
    }

    // Source management operations
    pub async fn get_all_sources(&self) -> Result<Vec<crate::models::Source>> {
        let rows = sqlx::query(
            r#"SELECT id, user_id, name, source_type, enabled, config, status, 
               last_sync_at, last_error, last_error_at, total_files_synced, 
               total_files_pending, total_size_bytes, created_at, updated_at
               FROM sources ORDER BY created_at DESC"#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut sources = Vec::new();
        for row in rows {
            sources.push(crate::models::Source {
                id: row.get("id"),
                user_id: row.get("user_id"),
                name: row.get("name"),
                source_type: row.get::<String, _>("source_type").try_into()
                    .map_err(|e| anyhow::anyhow!("Invalid source type: {}", e))?,
                enabled: row.get("enabled"),
                config: row.get("config"),
                status: row.get::<String, _>("status").try_into()
                    .map_err(|e| anyhow::anyhow!("Invalid source status: {}", e))?,
                last_sync_at: row.get("last_sync_at"),
                last_error: row.get("last_error"),
                last_error_at: row.get("last_error_at"),
                total_files_synced: row.get("total_files_synced"),
                total_files_pending: row.get("total_files_pending"),
                total_size_bytes: row.get("total_size_bytes"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(sources)
    }

    pub async fn get_sources_for_sync(&self) -> Result<Vec<crate::models::Source>> {
        let rows = sqlx::query(
            r#"SELECT id, user_id, name, source_type, enabled, config, status, 
               last_sync_at, last_error, last_error_at, total_files_synced, 
               total_files_pending, total_size_bytes, created_at, updated_at
               FROM sources 
               WHERE enabled = true AND status != 'syncing'
               ORDER BY last_sync_at ASC NULLS FIRST"#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut sources = Vec::new();
        for row in rows {
            sources.push(crate::models::Source {
                id: row.get("id"),
                user_id: row.get("user_id"),
                name: row.get("name"),
                source_type: row.get::<String, _>("source_type").try_into()
                    .map_err(|e| anyhow::anyhow!("Invalid source type: {}", e))?,
                enabled: row.get("enabled"),
                config: row.get("config"),
                status: row.get::<String, _>("status").try_into()
                    .map_err(|e| anyhow::anyhow!("Invalid source status: {}", e))?,
                last_sync_at: row.get("last_sync_at"),
                last_error: row.get("last_error"),
                last_error_at: row.get("last_error_at"),
                total_files_synced: row.get("total_files_synced"),
                total_files_pending: row.get("total_files_pending"),
                total_size_bytes: row.get("total_size_bytes"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(sources)
    }

    pub async fn get_source_by_id(&self, source_id: Uuid) -> Result<Option<crate::models::Source>> {
        let row = sqlx::query(
            r#"SELECT id, user_id, name, source_type, enabled, config, status, 
               last_sync_at, last_error, last_error_at, total_files_synced, 
               total_files_pending, total_size_bytes, created_at, updated_at
               FROM sources WHERE id = $1"#
        )
        .bind(source_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(crate::models::Source {
                id: row.get("id"),
                user_id: row.get("user_id"),
                name: row.get("name"),
                source_type: row.get::<String, _>("source_type").try_into()
                    .map_err(|e| anyhow::anyhow!("Invalid source type: {}", e))?,
                enabled: row.get("enabled"),
                config: row.get("config"),
                status: row.get::<String, _>("status").try_into()
                    .map_err(|e| anyhow::anyhow!("Invalid source status: {}", e))?,
                last_sync_at: row.get("last_sync_at"),
                last_error: row.get("last_error"),
                last_error_at: row.get("last_error_at"),
                total_files_synced: row.get("total_files_synced"),
                total_files_pending: row.get("total_files_pending"),
                total_size_bytes: row.get("total_size_bytes"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            }))
        } else {
            Ok(None)
        }
    }

    // Processed images operations
    pub async fn create_processed_image(&self, processed_image: &crate::models::CreateProcessedImage) -> Result<crate::models::ProcessedImage> {
        let row = sqlx::query(
            r#"INSERT INTO processed_images 
               (document_id, user_id, original_image_path, processed_image_path, 
                processing_parameters, processing_steps, image_width, image_height, file_size)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
               RETURNING id, document_id, user_id, original_image_path, processed_image_path,
                         processing_parameters, processing_steps, image_width, image_height, 
                         file_size, created_at"#
        )
        .bind(processed_image.document_id)
        .bind(processed_image.user_id)
        .bind(&processed_image.original_image_path)
        .bind(&processed_image.processed_image_path)
        .bind(&processed_image.processing_parameters)
        .bind(&processed_image.processing_steps)
        .bind(processed_image.image_width)
        .bind(processed_image.image_height)
        .bind(processed_image.file_size)
        .fetch_one(&self.pool)
        .await?;

        Ok(crate::models::ProcessedImage {
            id: row.get("id"),
            document_id: row.get("document_id"),
            user_id: row.get("user_id"),
            original_image_path: row.get("original_image_path"),
            processed_image_path: row.get("processed_image_path"),
            processing_parameters: row.get("processing_parameters"),
            processing_steps: row.get("processing_steps"),
            image_width: row.get("image_width"),
            image_height: row.get("image_height"),
            file_size: row.get("file_size"),
            created_at: row.get("created_at"),
        })
    }

    pub async fn get_processed_image_by_document_id(&self, document_id: Uuid, user_id: Uuid) -> Result<Option<crate::models::ProcessedImage>> {
        let row = sqlx::query(
            r#"SELECT id, document_id, user_id, original_image_path, processed_image_path,
                      processing_parameters, processing_steps, image_width, image_height, 
                      file_size, created_at
               FROM processed_images 
               WHERE document_id = $1 AND user_id = $2
               ORDER BY created_at DESC
               LIMIT 1"#
        )
        .bind(document_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(crate::models::ProcessedImage {
                id: row.get("id"),
                document_id: row.get("document_id"),
                user_id: row.get("user_id"),
                original_image_path: row.get("original_image_path"),
                processed_image_path: row.get("processed_image_path"),
                processing_parameters: row.get("processing_parameters"),
                processing_steps: row.get("processing_steps"),
                image_width: row.get("image_width"),
                image_height: row.get("image_height"),
                file_size: row.get("file_size"),
                created_at: row.get("created_at"),
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn delete_processed_images_by_document_id(&self, document_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM processed_images WHERE document_id = $1")
            .bind(document_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}