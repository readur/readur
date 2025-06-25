# Database Guardrails for Concurrent Processing Safety

## Overview

This document outlines comprehensive database guardrails to prevent race conditions, data corruption, and consistency issues in concurrent processing environments. These guardrails were developed in response to OCR text corruption issues identified during high-volume concurrent file processing.

## 🚨 Critical Issues Identified

1. **OCR Text Corruption**: FileA's OCR text gets overwritten with FileB's data during concurrent processing
2. **Race Conditions**: Multiple workers updating the same document without proper isolation
3. **No Transaction Protection**: Database updates lack atomic transaction boundaries
4. **Missing Validation**: No document ID validation during OCR updates
5. **Connection Pool Exhaustion**: High concurrency can exhaust database connections

## 🛡️ Implemented Guardrails

### 1. Transaction-Based Operations (`src/db_guardrails.rs`)

#### `DocumentTransactionManager`
- **Atomic OCR Updates**: All OCR result updates wrapped in transactions
- **Row-Level Locking**: Uses `FOR UPDATE` to prevent concurrent modifications
- **Document Validation**: Verifies document exists and hasn't changed during processing
- **Data Quality Checks**: Validates OCR confidence, word count, and text consistency
- **Queue Cleanup**: Atomically removes completed items from OCR queue

```rust
// Example usage
let success = transaction_manager.update_ocr_with_validation(
    document_id,
    expected_filename,
    ocr_text,
    confidence,
    word_count,
    processing_time_ms,
).await?;
```

#### `DistributedLock`
- **Named Locks**: PostgreSQL advisory locks for critical sections
- **Timeout Support**: Prevents indefinite blocking
- **Resource Protection**: Guards shared resources during concurrent access

### 2. Database Constraints (`migrations/20240615000001_add_database_guardrails.sql`)

#### Data Integrity Constraints
```sql
-- OCR status validation
ALTER TABLE documents ADD CONSTRAINT check_ocr_status 
CHECK (ocr_status IN ('pending', 'processing', 'completed', 'failed'));

-- Confidence range validation
ALTER TABLE documents ADD CONSTRAINT check_ocr_confidence 
CHECK (ocr_confidence IS NULL OR (ocr_confidence >= 0 AND ocr_confidence <= 100));

-- Prevent duplicate queue entries
CREATE UNIQUE INDEX idx_ocr_queue_unique_pending_document 
ON ocr_queue (document_id) 
WHERE status IN ('pending', 'processing');
```

#### Referential Integrity
```sql
-- Cascade deletes to maintain consistency
ALTER TABLE ocr_queue 
ADD CONSTRAINT fk_ocr_queue_document_id 
FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE;
```

### 3. Database Triggers for Automatic Validation

#### OCR Consistency Trigger
```sql
CREATE TRIGGER trigger_validate_ocr_consistency
    BEFORE UPDATE ON documents
    FOR EACH ROW
    EXECUTE FUNCTION validate_ocr_consistency();
```

**Prevents:**
- Modifying completed OCR data
- Invalid confidence/word count combinations
- Missing metadata on completion

#### Automatic Queue Cleanup
```sql
CREATE TRIGGER trigger_cleanup_completed_ocr_queue
    AFTER UPDATE ON documents
    FOR EACH ROW
    EXECUTE FUNCTION cleanup_completed_ocr_queue();
```

**Benefits:**
- Automatically removes completed queue items
- Prevents orphaned queue entries
- Maintains queue consistency

### 4. Monitoring and Alerting (`src/db_monitoring.rs`)

#### Real-Time Health Monitoring
- **OCR Processing Health**: Tracks stuck jobs, failure rates, confidence levels
- **Queue Health**: Monitors queue size, worker count, processing times
- **Connection Pool Health**: Tracks utilization, response times
- **Data Consistency**: Validates referential integrity, identifies orphaned records

#### Automatic Recovery
```rust
// Auto-reset stuck jobs
if health.ocr_processing.stuck_jobs > 0 {
    let reset_count = monitor.reset_stuck_jobs().await?;
    warn!("Auto-recovery: Reset {} stuck OCR jobs", reset_count);
}
```

#### Alert Management
- **Cooldown Periods**: Prevents alert spam
- **Severity Levels**: Critical, Warning, Healthy status
- **Multiple Channels**: Email, Slack, logs

### 5. Performance Optimizations

#### Specialized Indexes
```sql
-- Faster queue operations
CREATE INDEX CONCURRENTLY idx_documents_pending_ocr 
ON documents (created_at) WHERE ocr_status = 'pending';

-- Monitor stuck jobs
CREATE INDEX CONCURRENTLY idx_documents_processing_ocr 
ON documents (updated_at) WHERE ocr_status = 'processing';
```

#### Connection Pool Management
- **Separate Pools**: Web and background processing use different pools
- **Pool Monitoring**: Track utilization and response times
- **Dynamic Sizing**: Adjust pool size based on load

## 🔧 Implementation Recommendations

### 1. Immediate Actions (High Priority)

#### Replace Unsafe OCR Updates
**Current (Vulnerable):**
```rust
sqlx::query!(
    "UPDATE documents SET ocr_text = $2, ocr_status = 'completed' WHERE id = $1",
    document_id, ocr_text
).execute(&pool).await?;
```

**Recommended (Safe):**
```rust
let transaction_manager = DocumentTransactionManager::new(pool.clone());
transaction_manager.update_ocr_with_validation(
    document_id,
    expected_filename,
    ocr_text,
    confidence,
    word_count,
    processing_time_ms,
).await?;
```

#### Update OCR Queue Service
Replace direct database updates in `src/ocr_queue.rs:266-285` with transaction-safe operations.

### 2. Configuration Updates

#### Database Pool Configuration
```rust
// Increase pool sizes for high concurrency
let web_pool = Database::new_with_pool_config(&config.database_url, 30, 5).await?;
let background_pool = Database::new_with_pool_config(&config.database_url, 40, 8).await?;
```

#### OCR Worker Configuration
```rust
// Limit concurrent workers to prevent resource exhaustion
let ocr_service = OcrQueueService::new(
    background_db.clone(),
    enhanced_ocr_service,
    3 // Reduced from 4 for better stability
);
```

### 3. Monitoring Setup

#### Start Database Monitor
```rust
let monitor_config = MonitoringConfig {
    check_interval_secs: 30,
    stuck_job_threshold_minutes: 15,
    enable_auto_recovery: true,
    ..Default::default()
};

let monitor = Arc::new(DatabaseMonitor::new(pool.clone(), monitor_config));
tokio::spawn(async move {
    monitor.start().await;
});
```

#### Health Check Endpoint
```rust
#[get("/api/health/database")]
async fn database_health(monitor: Extension<Arc<DatabaseMonitor>>) -> Json<DatabaseHealth> {
    let health = monitor.get_current_health().await.unwrap_or_default();
    Json(health)
}
```

## 🧪 Testing Strategy

### Integration Tests
The corruption issue can be reliably reproduced using the tests in `tests/ocr_corruption_tests.rs`:

```bash
# Test concurrent processing (reproduces corruption)
cargo test test_high_volume_concurrent_ocr --test ocr_corruption_tests

# Test sequential processing (should pass)
cargo test test_rapid_sequential_uploads --test ocr_corruption_tests
```

### Load Testing
```bash
# Simulate high concurrent load
for i in {1..20}; do
    curl -X POST http://localhost:8000/api/documents \
         -H "Authorization: Bearer $TOKEN" \
         -F "file=@test_document_$i.txt" &
done
```

## 📊 Monitoring Metrics

### Key Performance Indicators

1. **OCR Processing Metrics**
   - Pending job count
   - Processing time distribution
   - Confidence score distribution
   - Failure rate per hour

2. **Queue Health Metrics**
   - Queue size over time
   - Oldest pending job age
   - Worker utilization
   - Throughput (jobs/minute)

3. **Database Health Metrics**
   - Connection pool utilization
   - Query response times
   - Stuck job count
   - Data consistency score

### Dashboard Queries
```sql
-- Real-time OCR status
SELECT 
    ocr_status,
    COUNT(*) as count,
    AVG(ocr_confidence) as avg_confidence
FROM documents 
GROUP BY ocr_status;

-- Queue processing rate
SELECT 
    DATE_TRUNC('minute', completed_at) as minute,
    COUNT(*) as completed_jobs
FROM ocr_queue 
WHERE completed_at > NOW() - INTERVAL '1 hour'
GROUP BY minute
ORDER BY minute;

-- Identify stuck jobs
SELECT * FROM find_stuck_ocr_jobs(30);
```

## 🔄 Maintenance Procedures

### Daily Tasks
```sql
-- Check and reset stuck jobs
SELECT reset_stuck_ocr_jobs(30);

-- Refresh statistics
SELECT refresh_ocr_stats();

-- Validate data consistency
SELECT * FROM ocr_stats;
```

### Weekly Tasks
```sql
-- Deep consistency check
SELECT 
    orphaned_queue_items,
    documents_without_files,
    inconsistent_ocr_states,
    data_integrity_score
FROM validate_database_consistency();

-- Performance analysis
ANALYZE documents;
ANALYZE ocr_queue;
```

### Emergency Procedures

#### Mass Stuck Job Recovery
```sql
-- Reset all stuck jobs older than 15 minutes
SELECT reset_stuck_ocr_jobs(15);

-- Clear orphaned queue items
DELETE FROM ocr_queue WHERE document_id NOT IN (SELECT id FROM documents);
```

#### Connection Pool Exhaustion
```bash
# Restart application to reset connection pools
systemctl restart readur

# Or adjust pool size dynamically (if supported)
# This would require application-level implementation
```

## 🔮 Future Enhancements

### 1. Advanced Monitoring
- **Prometheus/Grafana Integration**: Real-time dashboards
- **Custom Metrics**: Application-specific performance indicators
- **Predictive Alerting**: ML-based anomaly detection

### 2. Database Optimizations
- **Read Replicas**: Separate read and write workloads
- **Partitioning**: Time-based partitioning for large tables
- **Connection Pooling**: PgBouncer for better connection management

### 3. Application-Level Improvements
- **Circuit Breakers**: Fail fast when database is unhealthy
- **Retry Logic**: Exponential backoff with jitter
- **Graceful Degradation**: Continue processing when possible

### 4. Data Archival
- **Hot/Cold Storage**: Move old documents to cheaper storage
- **Retention Policies**: Automatic cleanup of old processing logs
- **Backup Validation**: Regular backup integrity checks

## 📋 Checklist for Implementation

### Phase 1: Critical Safety (Week 1)
- [ ] Deploy database constraints migration
- [ ] Replace unsafe OCR update code with transaction manager
- [ ] Add monitoring for stuck jobs
- [ ] Set up basic alerting

### Phase 2: Enhanced Monitoring (Week 2)
- [ ] Deploy full monitoring system
- [ ] Create health check endpoints
- [ ] Set up automated recovery procedures
- [ ] Configure alert notifications

### Phase 3: Performance Optimization (Week 3)
- [ ] Optimize database indexes
- [ ] Tune connection pool sizes
- [ ] Implement load balancing
- [ ] Add performance dashboards

### Phase 4: Testing and Validation (Week 4)
- [ ] Run comprehensive load tests
- [ ] Validate corruption fixes
- [ ] Document operational procedures
- [ ] Train team on monitoring tools

## 🎯 Success Criteria

1. **Zero OCR Corruption**: No instances of FileA getting FileB's OCR text
2. **Improved Reliability**: 99.9% uptime for OCR processing
3. **Better Observability**: Real-time visibility into system health
4. **Faster Recovery**: Automatic recovery from common issues
5. **Scalable Performance**: Handle 10x current load without degradation

## 📞 Support and Escalation

### Monitoring Alerts
- **Critical**: Immediate response required (< 15 minutes)
- **Warning**: Investigation needed (< 2 hours)
- **Info**: Regular monitoring (next business day)

### Escalation Path
1. **Level 1**: Automatic recovery attempts
2. **Level 2**: Development team notification
3. **Level 3**: Database administrator involvement
4. **Level 4**: System architecture review

This comprehensive guardrail system provides multiple layers of protection against race conditions and data corruption while maintaining high performance and observability.