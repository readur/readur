# OCR Queue System Improvements

This document describes the major improvements made to handle large-scale OCR processing of 100k+ files.

## Key Improvements

### 1. **Database-Backed Queue System**
- Replaced direct processing with persistent queue table
- Added retry mechanisms and failure tracking
- Implemented priority-based processing
- Added recovery for crashed workers

### 2. **Worker Pool Architecture**
- Dedicated OCR worker processes with concurrency control
- Configurable number of concurrent jobs
- Graceful shutdown and error handling
- Automatic stale job recovery

### 3. **Batch Processing Support**
- Dedicated CLI tool for bulk ingestion
- Processes files in configurable batches (default: 1000)
- Concurrent file I/O with semaphore limiting
- Progress monitoring and statistics

### 4. **Priority-Based Processing**
Priority levels based on file size:
- **Priority 10**: ≤ 1MB files (highest)
- **Priority 8**: 1-5MB files
- **Priority 6**: 5-10MB files
- **Priority 4**: 10-50MB files
- **Priority 2**: > 50MB files (lowest)

### 5. **Monitoring & Observability**
- Real-time queue statistics API
- Progress tracking and ETAs
- Failed job requeuing
- Automatic cleanup of old completed jobs

## Database Schema

### OCR Queue Table
```sql
CREATE TABLE ocr_queue (
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
    file_size BIGINT
);
```

### Document Status Tracking
- `ocr_status`: Current OCR processing status
- `ocr_error`: Error message if OCR failed
- `ocr_completed_at`: Timestamp when OCR completed

## API Endpoints

### Queue Status
```
GET /api/queue/stats
```
Returns:
```json
{
    "pending": 1500,
    "processing": 8,
    "failed": 12,
    "completed_today": 5420,
    "avg_wait_time_minutes": 3.2,
    "oldest_pending_minutes": 15.7
}
```

### Requeue Failed Jobs
```
POST /api/queue/requeue-failed
```
Requeues all failed jobs that haven't exceeded max attempts.

## CLI Tools

### Batch Ingestion
```bash
# Ingest all files from a directory
cargo run --bin batch_ingest /path/to/files --user-id 00000000-0000-0000-0000-000000000000

# Ingest and monitor progress
cargo run --bin batch_ingest /path/to/files --user-id USER_ID --monitor
```

## Configuration

### Environment Variables
- `OCR_CONCURRENT_JOBS`: Number of concurrent OCR workers (default: 4)
- `OCR_TIMEOUT_SECONDS`: OCR processing timeout (default: 300)
- `QUEUE_BATCH_SIZE`: Batch size for processing (default: 1000)
- `MAX_CONCURRENT_IO`: Max concurrent file operations (default: 50)

### User Settings
Users can configure:
- `concurrent_ocr_jobs`: Max concurrent jobs for their documents
- `ocr_timeout_seconds`: Processing timeout
- `enable_background_ocr`: Enable/disable automatic OCR

## Performance Optimizations

### 1. **Memory Management**
- Streaming file reads for large files
- Configurable memory limits per worker
- Automatic cleanup of temporary data

### 2. **I/O Optimization**
- Batch database operations
- Connection pooling
- Concurrent file processing with limits

### 3. **Resource Control**
- CPU priority settings
- Memory limit enforcement
- Configurable worker counts

### 4. **Failure Handling**
- Exponential backoff for retries
- Separate failed job recovery
- Automatic stale job detection

## Monitoring & Maintenance

### Automatic Tasks
- **Stale Recovery**: Every 5 minutes, recover jobs stuck in processing
- **Cleanup**: Daily cleanup of completed jobs older than 7 days
- **Health Checks**: Worker health monitoring and restart

### Manual Operations
```sql
-- Check queue health
SELECT * FROM get_ocr_queue_stats();

-- Find problematic jobs
SELECT * FROM ocr_queue WHERE status = 'failed' ORDER BY created_at;

-- Requeue specific job
UPDATE ocr_queue SET status = 'pending', attempts = 0 WHERE id = 'job-id';
```

## Scalability Improvements

### For 100k+ Files:
1. **Horizontal Scaling**: Multiple worker instances across servers
2. **Database Optimization**: Partitioned queue tables by date
3. **Caching**: Redis cache for frequently accessed metadata
4. **Load Balancing**: Distribute workers across multiple machines

### Performance Metrics:
- **Throughput**: ~500-1000 files/hour per worker (depends on file size)
- **Memory Usage**: ~100MB per worker + file size
- **Database Load**: Optimized with proper indexing and batching

## Migration Guide

### From Old System:
1. Run database migration: `migrations/001_add_ocr_queue.sql`
2. Update application code to use queue endpoints
3. Monitor existing processing and let queue drain
4. Start new workers with queue system

### Zero-Downtime Migration:
1. Deploy new code with feature flag disabled
2. Run migration scripts
3. Enable queue processing gradually
4. Monitor and adjust worker counts as needed