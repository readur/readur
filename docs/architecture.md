# Architecture Overview

This document provides a comprehensive overview of Readur's system architecture, component interactions, data flows, and design decisions.

## System Components

### High-Level Architecture

**Important:** Readur is designed as a single-instance, monolithic application. It does NOT support multiple server instances, clustering, or high availability configurations.

```
┌──────────────────────────────────────────────────────────────────┐
│                     Readur Single Instance                        │
│                                                                   │
│  ┌─────────────┐  ┌─────────────┐  ┌──────────────────────┐    │
│  │  Web Server │  │   Business  │  │  Background Services │    │
│  │    (Axum)   │  │    Logic    │  │  - OCR Worker        │    │
│  └─────────────┘  └─────────────┘  │  - File Watcher      │    │
│                                     │  - Queue Processor   │    │
│                                     └──────────────────────┘    │
└───────────────────────────┬──────────────────────────────────────┘
                            │
        ┌───────────────────▼─────────────────────┐
        │           Data Layer                    │
        │  ┌────────┐  ┌────────┐  ┌──────────┐ │
        │  │Database│  │Storage │  │   Queue   │ │
        │  │  (PG)  │  │(S3/FS) │  │(DB-based) │ │
        │  └────────┘  └────────┘  └──────────┘ │
        └──────────────────────────────────────────┘
```

### Component Breakdown

```
Readur Application Instance
├── Web Server (Axum)
│   ├── HTTP API Endpoints
│   ├── WebSocket Server
│   ├── Static File Server
│   └── Middleware Stack
├── Business Logic Layer
│   ├── Document Management
│   ├── Search Engine
│   ├── User Management
│   ├── OCR Processing
│   └── Source Synchronization
├── Data Access Layer
│   ├── Database Connection Pool
│   ├── File Storage Interface
│   ├── Cache Layer
│   └── External API Clients
└── Background Services
    ├── OCR Queue Worker
    ├── File Watcher
    ├── Source Scheduler
    └── Cleanup Tasks
```

## Data Flow Architecture

### Document Upload Flow

```
User Upload Request
        │
        ▼
[1] Nginx/Reverse Proxy
        │
        ├─── Rate Limiting
        ├─── Request Validation
        └─── Load Balancing
        │
        ▼
[2] Authentication Middleware
        │
        ├─── JWT Validation
        └─── Permission Check
        │
        ▼
[3] File Upload Handler
        │
        ├─── File Type Validation
        ├─── Size Validation
        └─── Virus Scanning (optional)
        │
        ▼
[4] Storage Service
        │
        ├─── Generate UUID
        ├─── Calculate Hash
        └─── Store File
        │
        ▼
[5] Database Transaction
        │
        ├─── Create Document Record
        ├─── Add Metadata
        └─── Queue for OCR
        │
        ▼
[6] OCR Queue
        │
        ├─── Priority Assignment
        └─── Worker Notification
        │
        ▼
[7] Response to Client
        │
        └─── Document ID + Status
```

### OCR Processing Pipeline

```
OCR Queue Entry
        │
        ▼
[1] Queue Worker Pickup
        │
        ├─── Lock Document
        └─── Update Status
        │
        ▼
[2] File Retrieval
        │
        ├─── Load from Storage
        └─── Verify Integrity
        │
        ▼
[3] Preprocessing
        │
        ├─── Image Enhancement
        ├─── Format Conversion
        └─── Page Splitting
        │
        ▼
[4] OCR Engine (Tesseract)
        │
        ├─── Language Detection
        ├─── Text Extraction
        └─── Confidence Scoring
        │
        ▼
[5] Post-processing
        │
        ├─── Text Cleaning
        ├─── Format Normalization
        └─── Metadata Extraction
        │
        ▼
[6] Database Update
        │
        ├─── Store Extracted Text
        ├─── Update Search Index
        └─── Record Metrics
        │
        ▼
[7] Notification
        │
        ├─── WebSocket Update
        └─── Email (if configured)
```

### Search Request Flow

```
Search Query
        │
        ▼
[1] Query Parser
        │
        ├─── Tokenization
        ├─── Stemming
        └─── Query Expansion
        │
        ▼
[2] Search Executor
        │
        ├─── Full-Text Search (PostgreSQL)
        ├─── Filter Application
        └─── Ranking Algorithm
        │
        ▼
[3] Result Processing
        │
        ├─── Snippet Generation
        ├─── Highlighting
        └─── Facet Calculation
        │
        ▼
[4] Permission Filter
        │
        └─── User Access Check
        │
        ▼
[5] Response Assembly
        │
        ├─── Pagination
        ├─── Metadata Enrichment
        └─── JSON Serialization
```

## Queue Architecture

### OCR Queue System

```sql
-- Queue table structure
CREATE TABLE ocr_queue (
    id UUID PRIMARY KEY,
    document_id UUID REFERENCES documents(id),
    status VARCHAR(20), -- pending, processing, completed, failed
    priority INTEGER DEFAULT 5,
    retry_count INTEGER DEFAULT 0,
    max_retries INTEGER DEFAULT 3,
    created_at TIMESTAMP,
    started_at TIMESTAMP,
    completed_at TIMESTAMP,
    error_message TEXT,
    worker_id VARCHAR(100)
);

-- Efficient queue fetching with SKIP LOCKED
SELECT * FROM ocr_queue
WHERE status = 'pending'
ORDER BY priority DESC, created_at ASC
FOR UPDATE SKIP LOCKED
LIMIT 1;
```

### Queue Worker Architecture

```rust
// Queue processing with fixed thread pools
pub struct OcrQueueService {
    pool: PgPool,
    workers: Vec<JoinHandle<()>>,
    shutdown: Arc<AtomicBool>,
}

impl OcrQueueService {
    pub async fn start_workers(&self) {
        // Fixed thread allocation:
        // - OCR runtime: 3 threads
        // - Background runtime: 2 threads  
        // - Database runtime: 2 threads
        let ocr_workers = 3;
        
        for worker_id in 0..ocr_workers {
            let pool = self.pool.clone();
            let shutdown = self.shutdown.clone();
            
            let handle = tokio::spawn(async move {
                while !shutdown.load(Ordering::Relaxed) {
                    if let Some(job) = fetch_next_job(&pool).await {
                        process_ocr_job(job, &pool).await;
                    } else {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            });
            
            self.workers.push(handle);
        }
    }
}
```

## Storage Architecture

### Storage Abstraction Layer

```rust
// Storage trait for multiple backends
#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn store(&self, key: &str, data: &[u8]) -> Result<()>;
    async fn retrieve(&self, key: &str) -> Result<Vec<u8>>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn exists(&self, key: &str) -> Result<bool>;
    async fn list(&self, prefix: &str) -> Result<Vec<String>>;
}

// Implementations
pub struct LocalStorage { base_path: PathBuf }
pub struct S3Storage { bucket: String, client: S3Client }
pub struct AzureStorage { container: String, client: BlobClient }
```

### File Organization

```
Storage Root/
├── documents/
│   ├── {year}/{month}/{day}/
│   │   └── {uuid}.{extension}
├── thumbnails/
│   ├── {year}/{month}/{day}/
│   │   └── {uuid}_thumb.jpg
├── processed/
│   ├── ocr/
│   │   └── {uuid}_ocr.txt
│   └── metadata/
│       └── {uuid}_meta.json
└── temp/
    └── {session_id}/
        └── {temp_files}
```

## Database Schema

### Core Tables

```sql
-- Users table
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username VARCHAR(100) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    role VARCHAR(20) DEFAULT 'viewer',
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    last_login TIMESTAMP,
    settings JSONB DEFAULT '{}'::jsonb
);

-- Documents table
CREATE TABLE documents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    title VARCHAR(500),
    filename VARCHAR(255) NOT NULL,
    file_path VARCHAR(500) NOT NULL,
    file_hash VARCHAR(64),
    file_size BIGINT,
    mime_type VARCHAR(100),
    content TEXT,
    content_vector tsvector GENERATED ALWAYS AS (to_tsvector('english', content)) STORED,
    ocr_status VARCHAR(20) DEFAULT 'pending',
    ocr_confidence FLOAT,
    metadata JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    processed_at TIMESTAMP,
    source_id UUID REFERENCES sources(id),
    CONSTRAINT unique_file_hash UNIQUE(file_hash, user_id)
);

-- Create indexes for performance
CREATE INDEX idx_documents_content_vector ON documents USING gin(content_vector);
CREATE INDEX idx_documents_user_created ON documents(user_id, created_at DESC);
CREATE INDEX idx_documents_metadata ON documents USING gin(metadata jsonb_path_ops);
CREATE INDEX idx_documents_file_hash ON documents(file_hash) WHERE file_hash IS NOT NULL;
```

### Search Optimization

```sql
-- Full-text search function
CREATE OR REPLACE FUNCTION search_documents(
    query_text TEXT,
    user_id_param UUID,
    limit_param INT DEFAULT 20,
    offset_param INT DEFAULT 0
) RETURNS TABLE (
    id UUID,
    title TEXT,
    content TEXT,
    rank REAL,
    snippet TEXT
) AS $$
BEGIN
    RETURN QUERY
    WITH search_query AS (
        SELECT plainto_tsquery('english', query_text) AS q
    ),
    ranked_results AS (
        SELECT 
            d.id,
            d.title,
            d.content,
            ts_rank_cd(d.content_vector, sq.q) AS rank,
            ts_headline(
                'english',
                d.content,
                sq.q,
                'MaxWords=30, MinWords=15, StartSel=<mark>, StopSel=</mark>'
            ) AS snippet
        FROM documents d, search_query sq
        WHERE 
            d.user_id = user_id_param
            AND d.content_vector @@ sq.q
    )
    SELECT * FROM ranked_results
    ORDER BY rank DESC
    LIMIT limit_param
    OFFSET offset_param;
END;
$$ LANGUAGE plpgsql;
```

## Synchronization Architecture

### WebDAV Sync

```rust
pub struct WebDavSync {
    client: WebDavClient,
    db: Arc<DbConnection>,
    progress: Arc<Mutex<SyncProgress>>,
}

impl WebDavSync {
    pub async fn smart_sync(&self) -> Result<SyncResult> {
        // 1. Fetch remote file list with ETags
        let remote_files = self.client.list_files().await?;
        
        // 2. Compare with local database
        let local_files = self.db.get_source_files().await?;
        
        // 3. Determine changes
        let changes = self.calculate_changes(&remote_files, &local_files);
        
        // 4. Process changes in batches
        for batch in changes.chunks(100) {
            self.process_batch(batch).await?;
            self.update_progress().await?;
        }
        
        // 5. Clean up deleted files
        self.process_deletions(&remote_files, &local_files).await?;
        
        Ok(SyncResult { 
            added: changes.added.len(),
            updated: changes.updated.len(),
            deleted: changes.deleted.len()
        })
    }
}
```

### Source Scheduler

```rust
pub struct SourceScheduler {
    sources: Arc<RwLock<Vec<Source>>>,
    executor: Arc<ThreadPool>,
}

impl SourceScheduler {
    pub async fn run(&self) {
        loop {
            let now = Utc::now();
            let sources = self.sources.read().await;
            
            for source in sources.iter() {
                if source.should_sync(now) {
                    let source_clone = source.clone();
                    self.executor.spawn(async move {
                        match source_clone.sync().await {
                            Ok(result) => log::info!("Sync completed: {:?}", result),
                            Err(e) => log::error!("Sync failed: {}", e),
                        }
                    });
                }
            }
            
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }
}
```

## Performance Optimization

### Connection Pooling

```rust
// Database connection pool configuration
let pool = PgPoolOptions::new()
    .max_connections(32)
    .min_connections(5)
    .connect_timeout(Duration::from_secs(5))
    .acquire_timeout(Duration::from_secs(10))
    .idle_timeout(Duration::from_secs(600))
    .max_lifetime(Duration::from_secs(1800))
    .connect(&database_url)
    .await?;
```

### Caching Strategy

```rust
// Multi-level caching
pub struct CacheManager {
    l1_cache: Arc<DashMap<String, CachedItem>>, // In-memory
    l2_cache: Option<RedisClient>,               // Redis (optional)
}

impl CacheManager {
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        // Check L1 cache
        if let Some(item) = self.l1_cache.get(key) {
            if !item.is_expired() {
                return Some(item.value.clone());
            }
        }
        
        // Check L2 cache
        if let Some(redis) = &self.l2_cache {
            if let Ok(value) = redis.get(key).await {
                self.l1_cache.insert(key.to_string(), value.clone());
                return Some(value);
            }
        }
        
        None
    }
}
```

### Batch Processing

```rust
// Batch document processing
pub async fn batch_process_documents(
    documents: Vec<Document>,
    batch_size: usize,
) -> Result<Vec<ProcessResult>> {
    let semaphore = Arc::new(Semaphore::new(batch_size));
    let mut tasks = Vec::new();
    
    for doc in documents {
        let permit = semaphore.clone().acquire_owned().await?;
        let task = tokio::spawn(async move {
            let result = process_document(doc).await;
            drop(permit);
            result
        });
        tasks.push(task);
    }
    
    let results = futures::future::join_all(tasks).await;
    Ok(results.into_iter().filter_map(Result::ok).collect())
}
```

## Security Architecture

### Authentication Flow

```
┌─────────┐      ┌─────────┐      ┌─────────┐
│ Client  │─────►│   API   │─────►│   Auth  │
└─────────┘      └─────────┘      └─────────┘
     │                │                 │
     │   POST /login  │   Validate      │
     │   {user,pass}  │   Credentials   │
     │                │                 │
     │◄───────────────┼─────────────────┤
     │   JWT Token    │   Generate      │
     │                │   Token         │
     │                │                 │
     │   GET /api/*   │   Verify        │
     │   Auth: Bearer │   JWT           │
     │                │                 │
     │◄───────────────┼─────────────────┤
     │   API Response │   Authorized    │
```

### Permission Model

```rust
// Role-based access control
#[derive(Debug, Clone, PartialEq)]
pub enum Role {
    Admin,    // Full system access
    Editor,   // Create, read, update, delete own documents
    Viewer,   // Read-only access to own documents
}

impl Role {
    pub fn can_upload(&self) -> bool {
        matches!(self, Role::Admin | Role::Editor)
    }
    
    pub fn can_delete(&self) -> bool {
        matches!(self, Role::Admin | Role::Editor)
    }
    
    pub fn can_manage_users(&self) -> bool {
        matches!(self, Role::Admin)
    }
    
    pub fn can_configure_system(&self) -> bool {
        matches!(self, Role::Admin)
    }
}
```

## Scalability Considerations

### Single-Instance Optimization

Since Readur is a single-instance application, scaling is achieved through:

1. **Vertical Scaling**: Increase CPU, RAM, and storage on the single server
2. **Storage Offloading**: Use S3 or compatible object storage
3. **Database Optimization**: Tune PostgreSQL for better performance
4. **Queue Management**: Optimize OCR queue processing

```yaml
# Docker Compose single-instance configuration
version: '3.8'
services:
  readur:
    image: ghcr.io/readur/readur:main
    # Single instance only - do NOT use replicas
    deploy:
      replicas: 1  # MUST be 1
      resources:
        limits:
          cpus: '4'     # Increase for better performance
          memory: 4G    # Increase for larger workloads
    environment:
      - DATABASE_URL=postgresql://db:5432/readur
      - CONCURRENT_OCR_JOBS=3  # Fixed thread pool
    depends_on:
      - db
```

### Database Sharding Strategy

```sql
-- Partition documents by user_id for horizontal scaling
CREATE TABLE documents_partition_template (
    LIKE documents INCLUDING ALL
) PARTITION BY HASH (user_id);

-- Create partitions
CREATE TABLE documents_part_0 PARTITION OF documents_partition_template
    FOR VALUES WITH (modulus 4, remainder 0);
CREATE TABLE documents_part_1 PARTITION OF documents_partition_template
    FOR VALUES WITH (modulus 4, remainder 1);
CREATE TABLE documents_part_2 PARTITION OF documents_partition_template
    FOR VALUES WITH (modulus 4, remainder 2);
CREATE TABLE documents_part_3 PARTITION OF documents_partition_template
    FOR VALUES WITH (modulus 4, remainder 3);
```

## Monitoring and Observability

### Metrics Collection

```rust
// Prometheus metrics
lazy_static! {
    static ref HTTP_REQUESTS: IntCounterVec = register_int_counter_vec!(
        "http_requests_total",
        "Total HTTP requests",
        &["method", "endpoint", "status"]
    ).unwrap();
    
    static ref OCR_PROCESSING_TIME: HistogramVec = register_histogram_vec!(
        "ocr_processing_duration_seconds",
        "OCR processing time",
        &["language", "status"]
    ).unwrap();
    
    static ref ACTIVE_USERS: IntGauge = register_int_gauge!(
        "active_users_total",
        "Number of active users"
    ).unwrap();
}
```

### Distributed Tracing

```rust
// OpenTelemetry integration
use opentelemetry::trace::Tracer;

pub async fn process_document_traced(doc: Document) -> Result<()> {
    let tracer = opentelemetry::global::tracer("readur");
    let span = tracer.start("process_document");
    let cx = Context::current_with_span(span);
    
    // Trace document loading
    let _load_span = tracer.start_with_context("load_document", &cx);
    let file_data = load_file(&doc.file_path).await?;
    
    // Trace OCR processing
    let _ocr_span = tracer.start_with_context("ocr_processing", &cx);
    let text = extract_text(&file_data).await?;
    
    // Trace database update
    let _db_span = tracer.start_with_context("update_database", &cx);
    update_document_content(&doc.id, &text).await?;
    
    Ok(())
}
```

## Deployment Architecture

### Kubernetes Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: readur
spec:
  replicas: 3
  selector:
    matchLabels:
      app: readur
  template:
    metadata:
      labels:
        app: readur
    spec:
      containers:
      - name: readur
        image: ghcr.io/readur/readur:main
        ports:
        - containerPort: 8080
        resources:
          requests:
            memory: "1Gi"
            cpu: "500m"
          limits:
            memory: "2Gi"
            cpu: "1000m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: readur-secrets
              key: database-url
```

## Development Workflow

### Local Development Setup

```bash
# Development environment
docker-compose -f docker-compose.dev.yml up -d

# Database migrations
cargo run --bin migrate

# Run with hot reload
cargo watch -x run

# Frontend development
cd frontend && npm run dev
```

### Testing Strategy

```rust
// Unit test example
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_document_processing() {
        let doc = create_test_document();
        let result = process_document(doc).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, "completed");
    }
    
    #[tokio::test]
    async fn test_search_functionality() {
        let pool = create_test_pool().await;
        seed_test_data(&pool).await;
        
        let results = search_documents("test query", &pool).await;
        assert!(!results.is_empty());
    }
}
```

## Future Architecture Considerations

### Planned Enhancements

1. **Elasticsearch Integration**: For advanced search capabilities
2. **Machine Learning Pipeline**: For document classification and smart tagging
3. **Microservices Migration**: Separate OCR, search, and storage services
4. **GraphQL API**: Alternative to REST for flexible querying
5. **Event Sourcing**: For audit trail and time-travel debugging
6. **Multi-tenancy**: Support for multiple organizations

### Technology Roadmap

- **Q1 2025**: Redis caching layer
- **Q2 2025**: Elasticsearch integration
- **Q3 2025**: ML-based document classification
- **Q4 2025**: Microservices architecture

## Architecture Decision Records (ADRs)

### ADR-001: Use Rust for Backend

**Status**: Accepted  
**Context**: Need high performance and memory safety  
**Decision**: Use Rust with Axum framework  
**Consequences**: Steep learning curve but excellent performance

### ADR-002: PostgreSQL for Primary Database

**Status**: Accepted  
**Context**: Need reliable ACID compliance and full-text search  
**Decision**: Use PostgreSQL with built-in FTS  
**Consequences**: Single point of failure without replication

### ADR-003: Monolithic Single-Instance Architecture

**Status**: Accepted  
**Context**: Simpler architecture, easier deployment and maintenance  
**Decision**: Single-instance monolithic application without clustering support  
**Consequences**: 
- Pros: Simple deployment, no distributed system complexity, easier debugging
- Cons: No high availability, scaling limited to vertical scaling
- Note: This is a deliberate design choice for simplicity and reliability