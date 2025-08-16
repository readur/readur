# Troubleshooting Guide

This comprehensive guide helps diagnose and resolve common issues with Readur installations and operations.

## Quick Diagnosis

### Health Check Script

```bash
#!/bin/bash
# health-check.sh

echo "=== Readur System Health Check ==="
echo "Timestamp: $(date)"
echo ""

# Check services
echo "Service Status:"
systemctl is-active readur && echo "✓ Readur: Running" || echo "✗ Readur: Stopped"
systemctl is-active postgresql && echo "✓ PostgreSQL: Running" || echo "✗ PostgreSQL: Stopped"
systemctl is-active nginx && echo "✓ Nginx: Running" || echo "✗ Nginx: Stopped"
echo ""

# Check connectivity
echo "Connectivity:"
curl -s -o /dev/null -w "✓ Web Server: %{http_code}\n" http://localhost:8080/health
pg_isready -q && echo "✓ Database: Connected" || echo "✗ Database: Not reachable"
echo ""

# Check disk space
echo "Disk Usage:"
df -h | grep -E "Filesystem|/var|/uploads"
echo ""

# Check recent errors
echo "Recent Errors (last 10):"
journalctl -u readur --no-pager -n 10 | grep -i error || echo "No recent errors"
```

## Common Issues and Solutions

### Installation Issues

#### Problem: Docker container fails to start

**Symptoms:**
- Container exits immediately
- Status shows "Exited (1)"

**Diagnosis:**
```bash
# Check container logs
docker logs readur-app

# Check detailed events
docker events --filter container=readur-app

# Inspect container configuration
docker inspect readur-app
```

**Solutions:**

1. **Port conflict:**
```bash
# Check if port is in use
sudo lsof -i :8080

# Change port in docker-compose.yml
ports:
  - "8081:8080"  # Use different external port
```

2. **Database connection failure:**
```bash
# Verify database is running
docker ps | grep postgres

# Test database connection
docker exec readur-app psql $DATABASE_URL -c "SELECT 1"

# Fix connection string
export DATABASE_URL="postgresql://readur:password@db:5432/readur"
```

3. **Permission issues:**
```bash
# Fix volume permissions
sudo chown -R 1000:1000 ./uploads
sudo chmod -R 755 ./uploads

# Run with proper user
docker run --user $(id -u):$(id -g) readur:latest
```

#### Problem: Build fails with Rust compilation errors

**Solutions:**

1. **Update Rust toolchain:**
```bash
rustup update
rustup default stable
cargo clean
cargo build --release
```

2. **Clear build cache:**
```bash
rm -rf target/
rm Cargo.lock
cargo build --release
```

3. **Install missing dependencies:**
```bash
# Ubuntu/Debian
sudo apt-get install build-essential pkg-config libssl-dev

# macOS
brew install openssl pkg-config

# Set environment variables
export PKG_CONFIG_PATH="/usr/local/opt/openssl/lib/pkgconfig"
```

### Database Issues

#### Problem: Database connection refused

**Diagnosis:**
```bash
# Test direct connection
psql -h localhost -p 5432 -U readur -d readur

# Check PostgreSQL logs
sudo journalctl -u postgresql -n 50

# Verify PostgreSQL is listening
sudo netstat -tlnp | grep 5432
```

**Solutions:**

1. **PostgreSQL not running:**
```bash
# Start PostgreSQL
sudo systemctl start postgresql

# Enable auto-start
sudo systemctl enable postgresql
```

2. **Authentication failure:**
```bash
# Edit pg_hba.conf
sudo nano /etc/postgresql/15/main/pg_hba.conf

# Add/modify line:
local   all   readur   md5
host    all   readur   127.0.0.1/32   md5

# Reload configuration
sudo systemctl reload postgresql
```

3. **Database doesn't exist:**
```bash
# Create database and user
sudo -u postgres psql <<EOF
CREATE USER readur WITH PASSWORD 'your_password';
CREATE DATABASE readur OWNER readur;
GRANT ALL PRIVILEGES ON DATABASE readur TO readur;
EOF
```

#### Problem: Migration failures

**Diagnosis:**
```bash
# Check migration status
psql -U readur -d readur -c "SELECT * FROM _sqlx_migrations ORDER BY installed_on DESC LIMIT 5;"

# View migration errors
tail -n 100 /var/log/readur/migration.log
```

**Solutions:**

1. **Rollback failed migration:**
```bash
# Identify failed migration
psql -U readur -d readur -c "SELECT version FROM _sqlx_migrations WHERE success = false;"

# Manually rollback
psql -U readur -d readur < migrations/rollback/20250115_failed_migration.down.sql

# Remove from migrations table
psql -U readur -d readur -c "DELETE FROM _sqlx_migrations WHERE version = 20250115100000;"

# Retry migration
cargo run --bin migrate
```

2. **Fix constraint violations:**
```sql
-- Find duplicate entries
SELECT file_hash, COUNT(*) 
FROM documents 
GROUP BY file_hash 
HAVING COUNT(*) > 1;

-- Remove duplicates keeping newest
DELETE FROM documents a
USING documents b
WHERE a.id < b.id 
  AND a.file_hash = b.file_hash;
```

### OCR Processing Issues

#### Problem: OCR queue stuck

**Diagnosis:**
```bash
# Check queue status
psql -U readur -d readur <<EOF
SELECT status, COUNT(*) 
FROM ocr_queue 
GROUP BY status;
EOF

# Find stuck jobs
psql -U readur -d readur <<EOF
SELECT id, document_id, status, retry_count, error_message
FROM ocr_queue
WHERE status = 'processing' 
  AND started_at < NOW() - INTERVAL '1 hour';
EOF
```

**Solutions:**

1. **Reset stuck jobs:**
```bash
# Reset jobs stuck in processing
psql -U readur -d readur <<EOF
UPDATE ocr_queue 
SET status = 'pending', 
    started_at = NULL,
    worker_id = NULL
WHERE status = 'processing' 
  AND started_at < NOW() - INTERVAL '1 hour';
EOF

# Restart OCR workers
docker exec readur-app cargo run --bin enqueue_pending_ocr
```

2. **Clear failed jobs:**
```bash
# Retry failed jobs with reset
psql -U readur -d readur <<EOF
UPDATE ocr_queue 
SET status = 'pending',
    retry_count = 0,
    error_message = NULL
WHERE status = 'failed' 
  AND retry_count >= max_retries;
EOF
```

#### Problem: OCR produces garbled text

**Solutions:**

1. **Wrong language configuration:**
```bash
# Check current language
echo $OCR_LANGUAGE

# Update for document
psql -U readur -d readur <<EOF
UPDATE documents 
SET ocr_language = 'eng+fra+deu'
WHERE id = 'document_id';
EOF

# Re-queue for OCR
cargo run --bin enqueue_pending_ocr --document-id document_id --language "eng+fra+deu"
```

2. **Poor image quality:**
```bash
# Debug PDF extraction
cargo run --bin debug_pdf_extraction problem_document.pdf --verbose

# Preprocess image
convert input.pdf -density 300 -depth 8 -strip -background white -alpha off output.pdf
```

3. **Install additional language packs:**
```bash
# Install language data
sudo apt-get install tesseract-ocr-fra tesseract-ocr-deu tesseract-ocr-spa

# Verify installation
tesseract --list-langs
```

### Performance Issues

#### Problem: Slow search queries

**Diagnosis:**
```sql
-- Check query performance
EXPLAIN (ANALYZE, BUFFERS) 
SELECT * FROM documents 
WHERE to_tsvector('english', content) @@ plainto_tsquery('english', 'search term');

-- Check index usage
SELECT schemaname, tablename, indexname, idx_scan, idx_tup_read
FROM pg_stat_user_indexes
WHERE tablename = 'documents'
ORDER BY idx_scan;
```

**Solutions:**

1. **Missing indexes:**
```sql
-- Create missing indexes
CREATE INDEX CONCURRENTLY idx_documents_content_gin 
ON documents USING gin(to_tsvector('english', content));

-- Rebuild existing indexes
REINDEX INDEX CONCURRENTLY idx_documents_content_gin;
```

2. **Table bloat:**
```sql
-- Check table size
SELECT pg_size_pretty(pg_total_relation_size('documents'));

-- Vacuum and analyze
VACUUM (FULL, ANALYZE) documents;

-- For less downtime
VACUUM (ANALYZE) documents;
```

3. **Optimize configuration:**
```ini
# postgresql.conf
shared_buffers = 2GB
effective_cache_size = 6GB
work_mem = 64MB
maintenance_work_mem = 512MB
```

#### Problem: High memory usage

**Diagnosis:**
```bash
# Check memory usage
free -h
ps aux --sort=-%mem | head -10

# Check Readur process
ps aux | grep readur

# Memory map
pmap -x $(pgrep readur)
```

**Solutions:**

1. **Limit concurrent operations:**
```yaml
# Reduce concurrent OCR jobs
CONCURRENT_OCR_JOBS: 2

# Limit database connections
DATABASE_MAX_CONNECTIONS: 20

# Reduce batch sizes
BATCH_SIZE: 50
```

2. **Configure memory limits:**
```bash
# Docker memory limit
docker run -m 2g readur:latest

# Systemd memory limit
[Service]
MemoryMax=2G
MemoryHigh=1.5G
```

### File Storage Issues

#### Problem: File upload fails

**Diagnosis:**
```bash
# Check disk space
df -h /uploads

# Check permissions
ls -la /uploads

# Check file size limits
grep -i "size" /etc/nginx/nginx.conf
```

**Solutions:**

1. **Insufficient disk space:**
```bash
# Clean up old files
find /uploads/temp -type f -mtime +7 -delete

# Move to larger disk
mv /uploads /mnt/large-disk/uploads
ln -s /mnt/large-disk/uploads /uploads
```

2. **Permission denied:**
```bash
# Fix ownership
sudo chown -R readur:readur /uploads

# Fix permissions
sudo chmod -R 755 /uploads
```

3. **File size limit exceeded:**
```nginx
# nginx.conf
client_max_body_size 500M;

# Reload nginx
sudo nginx -s reload
```

#### Problem: S3 sync failures

**Diagnosis:**
```bash
# Test S3 connectivity
aws s3 ls s3://readur-bucket/

# Check credentials
aws configure list

# Test specific operation
aws s3 cp test.txt s3://readur-bucket/test.txt
```

**Solutions:**

1. **Authentication errors:**
```bash
# Update credentials
export AWS_ACCESS_KEY_ID="your_key"
export AWS_SECRET_ACCESS_KEY="your_secret"

# Or use IAM role
aws configure set role_arn arn:aws:iam::123456789:role/ReadurRole
```

2. **Network issues:**
```bash
# Test connectivity
curl -I https://s3.amazonaws.com

# Use VPC endpoint
export S3_ENDPOINT_URL="https://vpce-xxx.s3.amazonaws.com"
```

### Authentication Issues

#### Problem: Users cannot login

**Diagnosis:**
```bash
# Check authentication logs
grep -i "auth" /var/log/readur/app.log | tail -20

# Verify JWT secret
echo $JWT_SECRET

# Test password hashing
cargo run --bin test_auth
```

**Solutions:**

1. **JWT secret mismatch:**
```bash
# Ensure consistent secret across instances
export JWT_SECRET="same-secret-all-instances"

# Restart all instances
docker-compose restart
```

2. **Password reset:**
```bash
# Generate new password hash
cargo run --bin reset_password --username admin

# Or directly in database
psql -U readur -d readur <<EOF
UPDATE users 
SET password_hash = '$2b$12$hashed_password_here'
WHERE username = 'admin';
EOF
```

#### Problem: OIDC authentication fails

**Solutions:**

1. **Verify OIDC configuration:**
```bash
# Test OIDC endpoint
curl https://auth.example.com/.well-known/openid-configuration

# Verify redirect URI
echo $OIDC_REDIRECT_URI  # Must match exactly in provider
```

2. **Certificate issues:**
```bash
# Trust self-signed certificates
export NODE_TLS_REJECT_UNAUTHORIZED=0

# Or add to trust store
sudo cp provider-cert.crt /usr/local/share/ca-certificates/
sudo update-ca-certificates
```

### WebSocket Connection Issues

#### Problem: Real-time updates not working

**Diagnosis:**
```javascript
// Browser console test
const ws = new WebSocket('ws://localhost:8080/ws');
ws.onopen = () => console.log('Connected');
ws.onerror = (e) => console.error('Error:', e);
ws.onmessage = (e) => console.log('Message:', e.data);
```

**Solutions:**

1. **Proxy configuration:**
```nginx
# nginx.conf
location /ws {
    proxy_pass http://localhost:8080;
    proxy_http_version 1.1;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection "upgrade";
    proxy_read_timeout 86400;
}
```

2. **Firewall rules:**
```bash
# Allow WebSocket port
sudo ufw allow 8080/tcp

# Check iptables
sudo iptables -L -n | grep 8080
```

## Debug Logging

### Enable Detailed Logging

```bash
# Set environment variables
export RUST_LOG=debug
export RUST_BACKTRACE=full

# Or in configuration
LOG_LEVEL=debug
LOG_FORMAT=json
LOG_TO_FILE=true
LOG_FILE_PATH=/var/log/readur/debug.log
```

### Analyze Logs

```bash
# Filter by severity
grep -E "ERROR|WARN" /var/log/readur/app.log

# Track specific request
grep "request_id=req_123" /var/log/readur/app.log

# Parse JSON logs
jq '.level == "error"' /var/log/readur/app.json

# Real-time monitoring
tail -f /var/log/readur/app.log | grep --line-buffered ERROR
```

## Performance Profiling

### CPU Profiling

```bash
# Using perf
perf record -g -p $(pgrep readur)
perf report

# Using flamegraph
cargo install flamegraph
cargo flamegraph --bin readur
```

### Memory Profiling

```bash
# Using valgrind
valgrind --leak-check=full --show-leak-kinds=all ./readur

# Using heaptrack
heaptrack ./readur
heaptrack_gui heaptrack.readur.*.gz
```

### Database Query Analysis

```sql
-- Enable query logging
ALTER SYSTEM SET log_statement = 'all';
ALTER SYSTEM SET log_duration = on;
SELECT pg_reload_conf();

-- Check slow queries
SELECT query, mean_exec_time, calls
FROM pg_stat_statements
WHERE mean_exec_time > 1000
ORDER BY mean_exec_time DESC
LIMIT 10;
```

## Recovery Procedures

### Emergency Database Recovery

```bash
#!/bin/bash
# emergency-db-recovery.sh

# Stop application
systemctl stop readur

# Backup current state
pg_dump -U readur -d readur > emergency_backup_$(date +%Y%m%d_%H%M%S).sql

# Try to repair
psql -U readur -d readur <<EOF
REINDEX DATABASE readur;
VACUUM FULL ANALYZE;
EOF

# If that fails, restore from backup
psql -U readur -d readur < last_known_good_backup.sql

# Restart application
systemctl start readur
```

### File System Recovery

```bash
#!/bin/bash
# recover-files.sh

# Check filesystem
fsck -y /dev/sda1

# Recover deleted files
extundelete /dev/sda1 --restore-all

# Verify file integrity
find /uploads -type f -exec md5sum {} \; > checksums.txt
md5sum -c checksums.txt
```

## Monitoring Commands

### System Health Monitoring

```bash
# One-liner health check
watch -n 5 'echo "=== System Health ===" && \
  systemctl is-active readur postgresql nginx && \
  echo "" && \
  echo "=== Resources ===" && \
  free -h | head -2 && \
  echo "" && \
  df -h | grep -E "/$|uploads" && \
  echo "" && \
  echo "=== Database ===" && \
  psql -U readur -d readur -t -c "SELECT status, COUNT(*) FROM ocr_queue GROUP BY status;" && \
  echo "" && \
  echo "=== Recent Errors ===" && \
  journalctl -u readur -n 5 --no-pager | grep -i error'
```

## Getting Help

### Collect Diagnostic Information

```bash
#!/bin/bash
# collect-diagnostics.sh

DIAG_DIR="readur-diagnostics-$(date +%Y%m%d_%H%M%S)"
mkdir -p $DIAG_DIR

# System information
uname -a > $DIAG_DIR/system.txt
free -h >> $DIAG_DIR/system.txt
df -h >> $DIAG_DIR/system.txt

# Service status
systemctl status readur > $DIAG_DIR/readur-status.txt
systemctl status postgresql > $DIAG_DIR/postgres-status.txt

# Recent logs
journalctl -u readur -n 1000 > $DIAG_DIR/readur-logs.txt
tail -n 1000 /var/log/readur/app.log > $DIAG_DIR/app-logs.txt

# Configuration (sanitized)
env | grep -E "^READUR_|^DATABASE_|^OCR_" | sed 's/PASSWORD=.*/PASSWORD=***/' > $DIAG_DIR/config.txt

# Database stats
psql -U readur -d readur <<EOF > $DIAG_DIR/db-stats.txt
SELECT version();
SELECT COUNT(*) as documents FROM documents;
SELECT COUNT(*) as users FROM users;
SELECT status, COUNT(*) FROM ocr_queue GROUP BY status;
EOF

# Create archive
tar -czf $DIAG_DIR.tar.gz $DIAG_DIR/
echo "Diagnostics collected in $DIAG_DIR.tar.gz"
```

### Support Channels

- **GitHub Issues**: https://github.com/readur/readur/issues
- **Discord Community**: https://discord.gg/readur
- **Documentation**: https://readur.app/docs
- **Email Support**: support@readur.app (for enterprise customers)

When reporting issues, include:
1. Diagnostic archive from above script
2. Steps to reproduce the issue
3. Expected vs actual behavior
4. Any error messages or screenshots
5. Your deployment environment details