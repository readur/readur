# Performance Tuning Guide

## Overview

This guide provides comprehensive performance optimization strategies for Readur deployments, from small personal instances to large enterprise installations.

## Performance Baseline

### System Metrics to Monitor

| Metric | Target | Warning | Critical |
|--------|--------|---------|----------|
| CPU Usage | <60% | 60-80% | >80% |
| Memory Usage | <70% | 70-85% | >85% |
| Disk I/O Wait | <10% | 10-25% | >25% |
| Database Connections | <60% | 60-80% | >80% |
| Response Time (p95) | <500ms | 500-1000ms | >1000ms |
| OCR Queue Length | <100 | 100-500 | >500 |

## Database Optimization

### PostgreSQL Tuning

#### Connection Pool Settings

```bash
# postgresql.conf
max_connections = 200
shared_buffers = 256MB  # 25% of available RAM
effective_cache_size = 1GB  # 50-75% of available RAM
work_mem = 4MB
maintenance_work_mem = 64MB

# Write performance
checkpoint_segments = 32
checkpoint_completion_target = 0.9
wal_buffers = 16MB

# Query optimization
random_page_cost = 1.1  # For SSD storage
effective_io_concurrency = 200  # For SSD
default_statistics_target = 100
```

#### Application Connection Pooling

```bash
# Readur configuration
DATABASE_POOL_SIZE=20
DATABASE_MAX_OVERFLOW=10
DATABASE_POOL_TIMEOUT=30
DATABASE_POOL_RECYCLE=3600
DATABASE_STATEMENT_TIMEOUT=30000  # 30 seconds
```

### Query Optimization

#### Index Creation

```sql
-- Essential indexes for performance
CREATE INDEX CONCURRENTLY idx_documents_user_created 
  ON documents(user_id, created_at DESC);

CREATE INDEX CONCURRENTLY idx_documents_ocr_status 
  ON documents(ocr_status) 
  WHERE ocr_status IN ('pending', 'processing');

CREATE INDEX CONCURRENTLY idx_documents_search 
  ON documents USING gin(to_tsvector('english', content));

-- Partial indexes for common queries
CREATE INDEX CONCURRENTLY idx_recent_documents 
  ON documents(created_at DESC) 
  WHERE created_at > CURRENT_DATE - INTERVAL '30 days';
```

#### Query Analysis

```sql
-- Enable query logging for slow queries
ALTER SYSTEM SET log_min_duration_statement = 1000;  -- Log queries over 1 second
ALTER SYSTEM SET log_statement = 'all';
SELECT pg_reload_conf();

-- Analyze query performance
EXPLAIN (ANALYZE, BUFFERS) 
SELECT * FROM documents 
WHERE user_id = '123' 
  AND created_at > '2024-01-01'
ORDER BY created_at DESC 
LIMIT 100;
```

### Database Maintenance

```bash
#!/bin/bash
# maintenance.sh - Run weekly

# Vacuum and analyze
docker-compose exec postgres vacuumdb -U readur -d readur -z -v

# Reindex for better performance
docker-compose exec postgres reindexdb -U readur -d readur

# Update statistics
docker-compose exec postgres psql -U readur -d readur -c "ANALYZE;"

# Clean up old data via database queries
docker-compose exec readur psql -U readur -d readur -c \
  "DELETE FROM sessions WHERE last_activity < NOW() - INTERVAL '30 days';"

# Check for orphaned files
docker-compose exec readur psql -U readur -d readur -c \
  "SELECT COUNT(*) FROM documents WHERE file_path NOT IN (SELECT path FROM files);"
```

## OCR Performance

### OCR Worker Configuration

```bash
# Optimize based on CPU cores and RAM
OCR_WORKERS=4  # Number of parallel workers
OCR_MAX_PARALLEL=8  # Max concurrent OCR operations
OCR_QUEUE_SIZE=1000  # Queue buffer size
OCR_BATCH_SIZE=10  # Documents per batch
OCR_TIMEOUT=300  # Seconds per document

# Memory management
OCR_MAX_MEMORY_MB=1024  # Per worker memory limit
OCR_TEMP_DIR=/tmp/ocr  # Use fast storage for temp files

# Tesseract optimization
TESSERACT_THREAD_LIMIT=2  # Threads per OCR job
TESSERACT_PSM=3  # Page segmentation mode
TESSERACT_OEM=1  # OCR engine mode (LSTM)
```

### OCR Processing Strategies

#### Priority Queue Implementation

```python
# priority_queue.py
from celery import Celery
from kombu import Queue, Exchange

app = Celery('readur')

# Define priority queues
app.conf.task_routes = {
    'ocr.process_document': {'queue': 'ocr', 'routing_key': 'ocr.normal'},
    'ocr.process_urgent': {'queue': 'ocr_priority', 'routing_key': 'ocr.high'},
}

app.conf.task_queues = (
    Queue('ocr', Exchange('ocr'), routing_key='ocr.normal', priority=5),
    Queue('ocr_priority', Exchange('ocr'), routing_key='ocr.high', priority=10),
)

# Worker configuration
app.conf.worker_prefetch_multiplier = 1
app.conf.task_acks_late = True
```

#### Batch Processing

```bash
# Re-queue pending OCR documents during off-hours
0 2 * * * docker-compose exec readur /app/enqueue_pending_ocr
```

## Storage Optimization

### File System Performance

#### Local Storage

```bash
# Mount options for better performance
/dev/sdb1 /data ext4 defaults,noatime,nodiratime,nobarrier 0 2

# For XFS
/dev/sdb1 /data xfs defaults,noatime,nodiratime,allocsize=64m 0 2

# Enable compression (Btrfs)
mount -o compress=lzo /dev/sdb1 /data
```

#### Storage Layout

```
/data/
├── readur/
│   ├── documents/      # Main storage (SSD recommended)
│   ├── temp/           # Temporary files (tmpfs or fast SSD)
│   ├── cache/          # Cache directory (SSD)
│   └── thumbnails/     # Generated thumbnails (can be slower storage)
```

### S3 Optimization

```bash
# S3 transfer optimization
S3_MAX_CONNECTIONS=100
S3_MAX_BANDWIDTH=100MB  # Limit bandwidth if needed
S3_MULTIPART_THRESHOLD=64MB
S3_MULTIPART_CHUNKSIZE=16MB
S3_MAX_CONCURRENCY=10
S3_USE_ACCELERATE_ENDPOINT=true  # AWS only

# Connection pooling
S3_CONNECTION_POOL_SIZE=50
S3_CONNECTION_TIMEOUT=30
S3_READ_TIMEOUT=60
```

## Caching Strategy

### Redis Configuration

```bash
# redis.conf
maxmemory 4gb
maxmemory-policy allkeys-lru
save ""  # Disable persistence for cache-only use
tcp-keepalive 60
timeout 300

# Performance tuning
tcp-backlog 511
databases 2
hz 10
```

### Application Caching

```bash
# Cache configuration
CACHE_TYPE=redis
CACHE_REDIS_URL=redis://localhost:6379/0
CACHE_DEFAULT_TIMEOUT=3600  # 1 hour
CACHE_THRESHOLD=1000  # Max cached items

# Specific cache TTLs
CACHE_SEARCH_RESULTS_TTL=600  # 10 minutes
CACHE_USER_SESSIONS_TTL=3600  # 1 hour
CACHE_DOCUMENT_METADATA_TTL=86400  # 24 hours
CACHE_THUMBNAILS_TTL=604800  # 7 days
```

### CDN Integration

```nginx
# Serve static files through CDN
location /static/ {
    expires 30d;
    add_header Cache-Control "public, immutable";
    add_header Vary "Accept-Encoding";
}

location /media/thumbnails/ {
    expires 7d;
    add_header Cache-Control "public";
}
```

## Application Optimization

### Gunicorn/Uvicorn Configuration

```bash
# Gunicorn settings
GUNICORN_WORKERS=4  # 2-4 x CPU cores
GUNICORN_WORKER_CLASS=uvicorn.workers.UvicornWorker
GUNICORN_WORKER_CONNECTIONS=1000
GUNICORN_MAX_REQUESTS=1000
GUNICORN_MAX_REQUESTS_JITTER=50
GUNICORN_TIMEOUT=30
GUNICORN_KEEPALIVE=5

# Thread pool
GUNICORN_THREADS=4
GUNICORN_THREAD_WORKERS=2
```

### Async Processing

```python
# async_config.py
import asyncio
from concurrent.futures import ThreadPoolExecutor

# Configure async settings
ASYNC_MAX_WORKERS = 10
ASYNC_QUEUE_SIZE = 100
executor = ThreadPoolExecutor(max_workers=ASYNC_MAX_WORKERS)

# Background task processing
CELERY_WORKER_CONCURRENCY = 4
CELERY_WORKER_PREFETCH_MULTIPLIER = 1
CELERY_TASK_TIME_LIMIT = 300
CELERY_TASK_SOFT_TIME_LIMIT = 240
```

## Network Optimization

### HTTP/2 Configuration

```nginx
server {
    listen 443 ssl http2;
    
    # HTTP/2 settings
    http2_max_field_size 16k;
    http2_max_header_size 32k;
    http2_max_requests 1000;
    
    # Keep-alive
    keepalive_timeout 65;
    keepalive_requests 100;
}
```

### Load Balancing

```nginx
upstream readur_backend {
    least_conn;  # Or ip_hash for session affinity
    
    server backend1:8000 weight=5 max_fails=3 fail_timeout=30s;
    server backend2:8000 weight=5 max_fails=3 fail_timeout=30s;
    server backend3:8000 weight=3 backup;
    
    keepalive 32;
}
```

## Monitoring and Profiling

### Performance Monitoring Stack

```yaml
# docker-compose.monitoring.yml
services:
  prometheus:
    image: prom/prometheus
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
    ports:
      - "9090:9090"
  
  grafana:
    image: grafana/grafana
    ports:
      - "3000:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
  
  node_exporter:
    image: prom/node-exporter
    ports:
      - "9100:9100"
```

### Application Profiling

```python
# profile_middleware.py
import cProfile
import pstats
import io

class ProfilingMiddleware:
    def __init__(self, app):
        self.app = app
        
    def __call__(self, environ, start_response):
        if 'profile' in environ.get('QUERY_STRING', ''):
            profiler = cProfile.Profile()
            profiler.enable()
            
            response = self.app(environ, start_response)
            
            profiler.disable()
            stream = io.StringIO()
            stats = pstats.Stats(profiler, stream=stream)
            stats.sort_stats('cumulative')
            stats.print_stats(20)
            
            print(stream.getvalue())
            
            return response
        return self.app(environ, start_response)
```

## Scaling Strategies

### Horizontal Scaling

```yaml
# docker-compose.scale.yml
services:
  readur:
    deploy:
      replicas: 3
      resources:
        limits:
          cpus: '2'
          memory: 4G
        reservations:
          cpus: '1'
          memory: 2G
  
  ocr-worker:
    deploy:
      replicas: 5
      resources:
        limits:
          cpus: '1'
          memory: 2G
```

### Vertical Scaling Guidelines

| Users | CPU | RAM | Storage | Database |
|-------|-----|-----|---------|----------|
| 1-10 | 2 cores | 4GB | 100GB | Shared |
| 10-50 | 4 cores | 8GB | 500GB | Dedicated 2 cores, 4GB |
| 50-100 | 8 cores | 16GB | 1TB | Dedicated 4 cores, 8GB |
| 100-500 | 16 cores | 32GB | 5TB | Cluster |
| 500+ | Multiple servers | 64GB+ | Object storage | Cluster with replicas |

## Optimization Checklist

### Quick Wins

- [ ] Enable gzip compression
- [ ] Set appropriate cache headers
- [ ] Configure connection pooling
- [ ] Enable query result caching
- [ ] Optimize database indexes
- [ ] Tune OCR worker count
- [ ] Configure Redis caching
- [ ] Enable HTTP/2

### Advanced Optimizations

- [ ] Implement read replicas
- [ ] Set up CDN for static files
- [ ] Enable database partitioning
- [ ] Implement queue priorities
- [ ] Configure auto-scaling
- [ ] Set up performance monitoring
- [ ] Implement rate limiting
- [ ] Enable connection multiplexing

## Troubleshooting Performance Issues

### High CPU Usage

```bash
# Identify CPU-intensive processes
top -H -p $(pgrep -d',' readur)

# Check OCR worker load
docker-compose exec readur celery inspect active

# Profile Python code
python -m cProfile -o profile.stats app.py
```

### Memory Issues

```bash
# Check memory usage
free -h
docker stats

# Find memory leaks
docker-compose exec readur python -m tracemalloc

# Adjust memory limits
docker update --memory 4g readur_container
```

### Slow Queries

```sql
-- Find slow queries
SELECT query, calls, mean_exec_time, total_exec_time
FROM pg_stat_statements
ORDER BY mean_exec_time DESC
LIMIT 10;

-- Check missing indexes
SELECT schemaname, tablename, attname, n_distinct, correlation
FROM pg_stats
WHERE schemaname = 'public'
  AND n_distinct > 100
  AND correlation < 0.1
ORDER BY n_distinct DESC;
```

## Related Documentation

- [Architecture Overview](../architecture.md)
- [Monitoring Guide](./monitoring.md)
- [Database Guardrails](../dev/DATABASE_GUARDRAILS.md)
- [OCR Optimization](../dev/OCR_OPTIMIZATION_GUIDE.md)