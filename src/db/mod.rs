use anyhow::Result;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::time::Duration;
use tokio::time::{sleep, timeout};
use serde::{Serialize, Deserialize};

pub mod users;
pub mod documents;
pub mod settings;
pub mod notifications;
pub mod webdav;
pub mod sources;
pub mod source_errors;
pub mod images;
pub mod ignored_files;
pub mod constraint_validation;
pub mod ocr_retry;

#[derive(Debug, Serialize, Deserialize)]
pub struct DatabasePoolHealth {
    pub size: u32,
    pub num_idle: usize,
    pub is_closed: bool,
}

#[derive(Clone)]
pub struct Database {
    pub pool: PgPool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(50)                          // Increased from 20 to handle more concurrent requests
            .acquire_timeout(Duration::from_secs(30))     // Increased from 10 to 30 seconds
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
            .acquire_timeout(Duration::from_secs(10))    // Short timeout for fast failure
            .idle_timeout(Duration::from_secs(30))       // Very short idle timeout for tests
            .max_lifetime(Duration::from_secs(300))      // Short lifetime for tests
            .min_connections(min_connections)
            .test_before_acquire(false)                  // Disable validation for speed
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }
    
    pub fn get_pool(&self) -> &PgPool {
        &self.pool
    }

    /// Close the database connection pool with simplified, fast approach
    pub async fn close(&self) {
        if self.pool.is_closed() {
            return; // Already closed, nothing to do
        }
        
        // Directly close the pool without complex timeout logic
        // The sqlx pool.close() is designed to be fast and reliable
        self.pool.close().await;
    }

    /// Get database connection pool health information
    pub fn get_pool_health(&self) -> DatabasePoolHealth {
        DatabasePoolHealth {
            size: self.pool.size(),
            num_idle: self.pool.num_idle(),
            is_closed: self.pool.is_closed(),
        }
    }

    /// Check if the database pool is healthy and has available connections
    pub async fn check_pool_health(&self) -> Result<bool> {
        // Try to acquire a connection with a short timeout to check health
        match tokio::time::timeout(
            Duration::from_secs(5), 
            self.pool.acquire()
        ).await {
            Ok(Ok(_conn)) => Ok(true),
            Ok(Err(e)) => {
                tracing::warn!("Database pool health check failed: {}", e);
                Ok(false)
            }
            Err(_) => {
                tracing::warn!("Database pool health check timed out");
                Ok(false)
            }
        }
    }



    /// Execute a simple query with enhanced error handling and retries
    pub async fn execute_with_retry<F, T, Fut>(&self, operation_name: &str, operation: F) -> Result<T>
    where
        F: Fn(&PgPool) -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<T>> + Send,
        T: Send,
    {
        const MAX_RETRIES: usize = 3;
        const BASE_DELAY_MS: u64 = 100;
        
        for attempt in 0..MAX_RETRIES {
            // Check pool health before attempting operation
            if attempt > 0 {
                if let Ok(false) = self.check_pool_health().await {
                    tracing::warn!("Database pool unhealthy on attempt {} for {}", attempt + 1, operation_name);
                    let delay_ms = BASE_DELAY_MS * (2_u64.pow(attempt as u32));
                    sleep(Duration::from_millis(delay_ms)).await;
                    continue;
                }
            }
            
            match timeout(Duration::from_secs(30), operation(&self.pool)).await {
                Ok(Ok(result)) => {
                    if attempt > 0 {
                        tracing::info!("Database operation '{}' succeeded on retry attempt {}", operation_name, attempt + 1);
                    }
                    return Ok(result);
                }
                Ok(Err(e)) => {
                    if attempt == MAX_RETRIES - 1 {
                        tracing::error!("Database operation '{}' failed after {} attempts: {}", operation_name, MAX_RETRIES, e);
                        return Err(e);
                    }
                    
                    // Check if this is a connection pool timeout or similar transient error
                    let error_msg = e.to_string().to_lowercase();
                    let is_retryable = error_msg.contains("pool") || 
                                     error_msg.contains("timeout") || 
                                     error_msg.contains("connection") ||
                                     error_msg.contains("busy");
                    
                    if is_retryable {
                        tracing::warn!("Retryable database error on attempt {} for '{}': {}", attempt + 1, operation_name, e);
                        let delay_ms = BASE_DELAY_MS * (2_u64.pow(attempt as u32));
                        sleep(Duration::from_millis(delay_ms)).await;
                    } else {
                        tracing::error!("Non-retryable database error for '{}': {}", operation_name, e);
                        return Err(e);
                    }
                }
                Err(_) => {
                    if attempt == MAX_RETRIES - 1 {
                        tracing::error!("Database operation '{}' timed out after {} attempts", operation_name, MAX_RETRIES);
                        return Err(anyhow::anyhow!("Database operation '{}' timed out after {} retries", operation_name, MAX_RETRIES));
                    }
                    
                    tracing::warn!("Database operation '{}' timed out on attempt {}", operation_name, attempt + 1);
                    let delay_ms = BASE_DELAY_MS * (2_u64.pow(attempt as u32));
                    sleep(Duration::from_millis(delay_ms)).await;
                }
            }
        }
        
        unreachable!()
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
        
        // Create users table with OIDC support
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                username VARCHAR(255) UNIQUE NOT NULL,
                email VARCHAR(255) UNIQUE NOT NULL,
                password_hash VARCHAR(255),
                role VARCHAR(20) DEFAULT 'user',
                created_at TIMESTAMPTZ DEFAULT NOW(),
                updated_at TIMESTAMPTZ DEFAULT NOW(),
                oidc_subject VARCHAR(255),
                oidc_issuer VARCHAR(255),
                oidc_email VARCHAR(255),
                auth_provider VARCHAR(50) DEFAULT 'local',
                CONSTRAINT check_auth_method CHECK (
                    (auth_provider = 'local' AND password_hash IS NOT NULL) OR 
                    (auth_provider = 'oidc' AND oidc_subject IS NOT NULL AND oidc_issuer IS NOT NULL)
                ),
                CONSTRAINT check_user_role CHECK (role IN ('admin', 'user'))
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        
        // Create indexes for OIDC
        sqlx::query(r#"CREATE INDEX IF NOT EXISTS idx_users_oidc_subject_issuer ON users(oidc_subject, oidc_issuer)"#)
            .execute(&self.pool)
            .await?;
            
        sqlx::query(r#"CREATE INDEX IF NOT EXISTS idx_users_auth_provider ON users(auth_provider)"#)
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

        // NOTE: get_ocr_queue_stats() function is now managed by SQLx migrations
        // See migrations/20250708000001_simplify_ocr_queue_stats.sql for current implementation

        Ok(())
    }
}
