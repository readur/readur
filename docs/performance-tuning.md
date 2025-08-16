# Performance Tuning Guide

This guide provides comprehensive performance optimization techniques for Readur deployments, covering database optimization, OCR processing, resource tuning, and monitoring strategies.

## Database Optimization

### PostgreSQL Configuration

Optimize PostgreSQL for document management workloads:

```ini
# postgresql.conf optimizations

# Memory settings
shared_buffers = 25% of RAM  # e.g., 4GB for 16GB system
effective_cache_size = 75% of RAM  # e.g., 12GB for 16GB system
work_mem = 32MB  # Per-operation memory
maintenance_work_mem = 512MB  # For VACUUM, indexes

# Connection pooling
max_connections = 200
max_prepared_transactions = 100

# Write performance
checkpoint_completion_target = 0.9
wal_buffers = 16MB
default_statistics_target = 100
random_page_cost = 1.1  # For SSD storage

# Query optimization
enable_partitionwise_join = on
enable_partitionwise_aggregate = on
jit = on  # Just-in-time compilation for complex queries
```

### Index Optimization

Critical indexes for performance:

```sql
-- Document search performance
CREATE INDEX CONCURRENTLY idx_documents_content_gin 
ON documents USING gin(to_tsvector('english', content));

CREATE INDEX CONCURRENTLY idx_documents_user_created 
ON documents(user_id, created_at DESC);

CREATE INDEX CONCURRENTLY idx_documents_status_updated 
ON documents(status, updated_at DESC) 
WHERE status IN ('pending', 'processing');

-- OCR queue performance
CREATE INDEX CONCURRENTLY idx_ocr_queue_priority 
ON ocr_queue(priority DESC, created_at ASC) 
WHERE status = 'pending';

CREATE INDEX CONCURRENTLY idx_ocr_queue_retry 
ON ocr_queue(retry_count, next_retry_at) 
WHERE status = 'failed' AND retry_count < max_retries;

-- Search optimization
CREATE INDEX CONCURRENTLY idx_documents_metadata 
ON documents USING gin(metadata jsonb_path_ops);

-- File hash for duplicate detection
CREATE INDEX CONCURRENTLY idx_documents_file_hash 
ON documents(file_hash) 
WHERE file_hash IS NOT NULL;
```

### Query Optimization

Optimize common queries:

```sql
-- Efficient document search with pagination
CREATE OR REPLACE FUNCTION search_documents_optimized(
    search_query TEXT,
    user_id_param UUID,
    limit_param INT DEFAULT 20,
    offset_param INT DEFAULT 0
) RETURNS TABLE (
    id UUID,
    title TEXT,
    content TEXT,
    rank REAL
) AS $$
BEGIN
    RETURN QUERY
    WITH ranked_docs AS (
        SELECT 
            d.id,
            d.title,
            d.content,
            ts_rank_cd(
                to_tsvector('english', d.content),
                plainto_tsquery('english', search_query)
            ) AS rank
        FROM documents d
        WHERE 
            d.user_id = user_id_param
            AND to_tsvector('english', d.content) @@ 
                plainto_tsquery('english', search_query)
    )
    SELECT * FROM ranked_docs
    ORDER BY rank DESC
    LIMIT limit_param
    OFFSET offset_param;
END;
$$ LANGUAGE plpgsql;

-- Efficient OCR queue fetch
CREATE OR REPLACE FUNCTION get_next_ocr_job() 
RETURNS ocr_queue AS $$
DECLARE
    job ocr_queue%ROWTYPE;
BEGIN
    SELECT * INTO job
    FROM ocr_queue
    WHERE status = 'pending'
    ORDER BY priority DESC, created_at ASC
    FOR UPDATE SKIP LOCKED
    LIMIT 1;
    
    IF FOUND THEN
        UPDATE ocr_queue 
        SET status = 'processing', 
            started_at = NOW()
        WHERE id = job.id;
    END IF;
    
    RETURN job;
END;
$$ LANGUAGE plpgsql;
```

### Database Maintenance

Regular maintenance schedule:

```bash
#!/bin/bash
# maintenance.sh - Run as a daily cron job

# Vacuum and analyze tables
psql -U readur -d readur_db <<EOF
VACUUM ANALYZE documents;
VACUUM ANALYZE ocr_queue;
VACUUM ANALYZE users;
REINDEX INDEX CONCURRENTLY idx_documents_content_gin;
EOF

# Update table statistics
psql -U readur -d readur_db <<EOF
ANALYZE documents;
ANALYZE ocr_queue;
EOF

# Clean up old data
psql -U readur -d readur_db <<EOF
DELETE FROM ocr_queue 
WHERE status = 'completed' 
  AND completed_at < NOW() - INTERVAL '30 days';

DELETE FROM notifications 
WHERE read = true 
  AND created_at < NOW() - INTERVAL '7 days';
EOF
```

## OCR Processing Optimization

### Tesseract Configuration

Optimize Tesseract settings for speed vs accuracy:

```yaml
# Fast processing (lower accuracy)
OCR_ENGINE_MODE: 2  # Legacy + LSTM engines
OCR_PSM: 3  # Fully automatic page segmentation
OCR_TESSDATA_PREFIX: "/usr/share/tesseract-ocr/4.00/tessdata/fast"

# Balanced (recommended)
OCR_ENGINE_MODE: 1  # LSTM engine only
OCR_PSM: 3
OCR_DPI: 300
OCR_TESSDATA_PREFIX: "/usr/share/tesseract-ocr/4.00/tessdata"

# High accuracy (slower)
OCR_ENGINE_MODE: 1
OCR_PSM: 11  # Sparse text
OCR_DPI: 600
OCR_TESSDATA_PREFIX: "/usr/share/tesseract-ocr/4.00/tessdata/best"
```

### Image Preprocessing

Optimize images before OCR:

```rust
use image::{DynamicImage, ImageBuffer};

fn preprocess_for_ocr(img: DynamicImage) -> DynamicImage {
    let mut processed = img
        .grayscale()  // Convert to grayscale
        .adjust_contrast(20.0)  // Increase contrast
        .brighten(10);  // Adjust brightness
    
    // Resize if too large (maintain aspect ratio)
    if processed.width() > 3000 {
        processed = processed.resize(
            3000,
            3000 * processed.height() / processed.width(),
            image::imageops::FilterType::Lanczos3
        );
    }
    
    // Apply denoising
    processed = denoise(processed, 2);
    
    // Deskew if needed
    if let Some(angle) = detect_skew(&processed) {
        if angle.abs() > 0.5 {
            processed = rotate(&processed, -angle);
        }
    }
    
    processed
}
```

### Parallel Processing

Configure concurrent OCR workers:

```yaml
# OCR worker configuration
OCR_WORKER_COUNT: 4  # Number of parallel workers
OCR_QUEUE_SIZE: 100  # Maximum queue size
OCR_BATCH_SIZE: 10  # Documents per batch
OCR_TIMEOUT_SECONDS: 300  # Per-document timeout
```

Implement parallel processing:

```rust
use tokio::sync::Semaphore;
use std::sync::Arc;

async fn process_ocr_queue(pool: &PgPool, workers: usize) {
    let semaphore = Arc::new(Semaphore::new(workers));
    let mut tasks = Vec::new();
    
    loop {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let pool_clone = pool.clone();
        
        let task = tokio::spawn(async move {
            if let Some(job) = fetch_next_ocr_job(&pool_clone).await {
                let _result = process_ocr_job(job, &pool_clone).await;
            }
            drop(permit);
        });
        
        tasks.push(task);
        
        // Clean up completed tasks
        tasks.retain(|task| !task.is_finished());
        
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
```

## Memory and CPU Tuning

### Memory Configuration

Optimize memory allocation:

```yaml
# Application memory settings
RUST_MIN_STACK: 8388608  # 8MB stack size
RUST_BACKTRACE: 0  # Disable in production for performance

# Docker memory limits
docker run -d \
  --memory="4g" \
  --memory-swap="6g" \
  --memory-reservation="2g" \
  --cpus="2.0" \
  readur:latest
```

### CPU Optimization

Configure CPU affinity and priorities:

```bash
# Set CPU affinity for OCR workers
taskset -c 0-3 ./ocr_worker  # Use cores 0-3

# Adjust process priority
nice -n -5 ./readur_server  # Higher priority

# Configure thread pool
export TOKIO_WORKER_THREADS=8
export RAYON_NUM_THREADS=4
```

### Memory Pool Configuration

```rust
// Implement object pooling for frequent allocations
use object_pool::{Pool, Reusable};

lazy_static! {
    static ref BUFFER_POOL: Pool<Vec<u8>> = Pool::new(32, || Vec::with_capacity(1024 * 1024));
}

async fn process_document(data: &[u8]) -> Result<()> {
    let mut buffer = BUFFER_POOL.pull();
    buffer.clear();
    buffer.extend_from_slice(data);
    
    // Process using pooled buffer
    let result = process(&buffer).await?;
    
    // Buffer automatically returned to pool when dropped
    Ok(result)
}
```

## Connection Pooling

### Database Connection Pool

Configure optimal pool settings:

```rust
use sqlx::postgres::PgPoolOptions;

let pool = PgPoolOptions::new()
    .max_connections(32)  // Maximum connections
    .min_connections(5)   // Minimum idle connections
    .connect_timeout(Duration::from_secs(5))
    .acquire_timeout(Duration::from_secs(10))
    .idle_timeout(Duration::from_secs(600))
    .max_lifetime(Duration::from_secs(1800))
    .connect(&database_url)
    .await?;
```

### Redis Connection Pool

If using Redis for caching:

```rust
use deadpool_redis::{Config, Runtime};

let cfg = Config {
    url: Some("redis://localhost:6379".to_string()),
    pool: Some(deadpool::managed::PoolConfig {
        max_size: 16,
        timeouts: deadpool::managed::Timeouts {
            wait: Some(Duration::from_secs(5)),
            create: Some(Duration::from_secs(5)),
            recycle: Some(Duration::from_secs(5)),
        },
        ..Default::default()
    }),
    ..Default::default()
};

let pool = cfg.create_pool(Some(Runtime::Tokio1))?;
```

## Caching Strategies

### Application-Level Caching

Implement multi-level caching:

```rust
use moka::future::Cache;
use std::time::Duration;

// L1 Cache: In-memory for hot data
lazy_static! {
    static ref L1_CACHE: Cache<String, Document> = Cache::builder()
        .max_capacity(1000)
        .time_to_live(Duration::from_secs(300))
        .build();
}

// L2 Cache: Redis for distributed caching
async fn get_document_cached(id: &str) -> Result<Document> {
    // Check L1 cache
    if let Some(doc) = L1_CACHE.get(id).await {
        return Ok(doc);
    }
    
    // Check L2 cache (Redis)
    if let Some(doc) = redis_get(id).await? {
        L1_CACHE.insert(id.to_string(), doc.clone()).await;
        return Ok(doc);
    }
    
    // Fetch from database
    let doc = fetch_from_db(id).await?;
    
    // Update caches
    L1_CACHE.insert(id.to_string(), doc.clone()).await;
    redis_set(id, &doc, 3600).await?;
    
    Ok(doc)
}
```

### Query Result Caching

Cache expensive query results:

```sql
-- Materialized view for search statistics
CREATE MATERIALIZED VIEW search_stats AS
SELECT 
    user_id,
    COUNT(*) as total_documents,
    SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END) as completed,
    SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) as failed,
    AVG(processing_time_ms) as avg_processing_time
FROM documents
GROUP BY user_id;

-- Refresh periodically
CREATE INDEX ON search_stats(user_id);
REFRESH MATERIALIZED VIEW CONCURRENTLY search_stats;
```

### Static Asset Caching

Configure CDN and browser caching:

```nginx
location /static/ {
    expires 1y;
    add_header Cache-Control "public, immutable";
    add_header Vary "Accept-Encoding";
    
    # Enable gzip
    gzip on;
    gzip_types text/css application/javascript image/svg+xml;
    gzip_vary on;
}

location /api/ {
    add_header Cache-Control "no-cache, no-store, must-revalidate";
    add_header Pragma "no-cache";
    add_header Expires "0";
}
```

## Performance Monitoring

### Key Metrics

Monitor these critical metrics:

```yaml
# Prometheus metrics configuration
metrics:
  - name: http_request_duration_seconds
    type: histogram
    buckets: [0.01, 0.05, 0.1, 0.5, 1, 5]
    
  - name: ocr_processing_duration_seconds
    type: histogram
    buckets: [1, 5, 10, 30, 60, 120]
    
  - name: database_query_duration_seconds
    type: histogram
    buckets: [0.001, 0.005, 0.01, 0.05, 0.1]
    
  - name: active_connections
    type: gauge
    
  - name: memory_usage_bytes
    type: gauge
    
  - name: cpu_usage_percent
    type: gauge
```

### Performance Dashboards

Grafana dashboard queries:

```promql
# Request latency P95
histogram_quantile(0.95, 
  rate(http_request_duration_seconds_bucket[5m]))

# OCR throughput
rate(ocr_documents_processed_total[5m])

# Database connection pool usage
database_connections_active / database_connections_max * 100

# Memory usage trend
rate(memory_usage_bytes[5m])
```

## Load Testing

### Load Test Configuration

Use k6 for load testing:

```javascript
// load-test.js
import http from 'k6/http';
import { check, sleep } from 'k6';

export let options = {
    stages: [
        { duration: '2m', target: 100 }, // Ramp up
        { duration: '5m', target: 100 }, // Stay at 100 users
        { duration: '2m', target: 200 }, // Ramp up
        { duration: '5m', target: 200 }, // Stay at 200 users
        { duration: '2m', target: 0 },   // Ramp down
    ],
    thresholds: {
        http_req_duration: ['p(95)<500'], // 95% of requests under 500ms
        http_req_failed: ['rate<0.1'],    // Error rate under 10%
    },
};

export default function() {
    // Search test
    let searchRes = http.get('http://localhost:8080/api/search?q=test');
    check(searchRes, {
        'search status is 200': (r) => r.status === 200,
        'search response time < 500ms': (r) => r.timings.duration < 500,
    });
    
    sleep(1);
    
    // Upload test
    let uploadRes = http.post('http://localhost:8080/api/upload', {
        file: open('./test.pdf', 'b'),
    });
    check(uploadRes, {
        'upload status is 201': (r) => r.status === 201,
    });
    
    sleep(2);
}
```

### Benchmarking OCR Performance

```bash
#!/bin/bash
# benchmark-ocr.sh

echo "OCR Performance Benchmark"
echo "========================="

# Test different configurations
for config in "fast" "balanced" "accurate"; do
    echo "Testing $config configuration..."
    
    export OCR_CONFIG=$config
    time ./ocr_benchmark --input ./test_docs/ --output ./results_$config/
    
    echo "Results for $config:"
    echo "  Documents processed: $(ls ./results_$config/ | wc -l)"
    echo "  Average accuracy: $(cat ./results_$config/accuracy.txt)"
    echo ""
done
```

## Optimization Checklist

### Database Optimization
- [ ] Indexes are properly configured
- [ ] Query plans are optimized
- [ ] Connection pooling is tuned
- [ ] Vacuum and analyze run regularly
- [ ] Slow query log is monitored
- [ ] Table partitioning for large tables

### Application Optimization
- [ ] Memory pools are configured
- [ ] Thread pools are sized correctly
- [ ] Caching is implemented
- [ ] Batch processing is used where applicable
- [ ] Async I/O is utilized
- [ ] Resource leaks are monitored

### OCR Optimization
- [ ] Image preprocessing is enabled
- [ ] Parallel processing is configured
- [ ] Appropriate accuracy settings
- [ ] Queue management is optimized
- [ ] Retry logic is efficient
- [ ] Resource limits are set

### Infrastructure Optimization
- [ ] CPU cores are allocated properly
- [ ] Memory is sufficient
- [ ] Storage is fast (SSD/NVMe)
- [ ] Network latency is minimized
- [ ] Load balancing is configured
- [ ] Auto-scaling is enabled

## Troubleshooting Performance Issues

### High Memory Usage

```bash
# Check memory usage by process
ps aux --sort=-%mem | head -10

# Analyze memory allocations
valgrind --leak-check=full --show-leak-kinds=all ./readur

# Profile memory usage
heaptrack ./readur
heaptrack_gui heaptrack.readur.*.gz
```

### Slow Queries

```sql
-- Enable slow query logging
ALTER SYSTEM SET log_min_duration_statement = 1000; -- Log queries over 1 second
SELECT pg_reload_conf();

-- Find slow queries
SELECT 
    query,
    calls,
    mean_exec_time,
    total_exec_time
FROM pg_stat_statements
ORDER BY mean_exec_time DESC
LIMIT 10;

-- Analyze query plan
EXPLAIN (ANALYZE, BUFFERS, VERBOSE) 
SELECT * FROM documents WHERE content ILIKE '%search%';
```

### CPU Bottlenecks

```bash
# Profile CPU usage
perf record -g ./readur
perf report

# Generate flame graph
cargo install flamegraph
cargo flamegraph --bin readur

# Check CPU-bound processes
top -H -p $(pgrep readur)
```

## Best Practices Summary

1. **Monitor First**: Always measure before optimizing
2. **Cache Aggressively**: Cache at multiple levels
3. **Batch Operations**: Process in batches when possible
4. **Async Everything**: Use async I/O for all operations
5. **Index Strategically**: Create indexes based on query patterns
6. **Pool Resources**: Use connection and object pools
7. **Profile Regularly**: Profile in production-like environments
8. **Test Under Load**: Regular load testing reveals bottlenecks
9. **Document Changes**: Track all performance optimizations
10. **Incremental Improvements**: Optimize iteratively, not all at once