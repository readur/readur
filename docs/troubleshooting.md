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

**Port conflict:** Check if the port is already in use and resolve the conflict.
```bash
# Check if port is in use
sudo lsof -i :8080

# Change port in docker-compose.yml
ports:
  - "8081:8080"  # Use different external port
```

**Database connection failure:** Verify that the database service is running and accessible.
```bash
# Verify database is running
docker ps | grep postgres

# Test database connection
docker exec readur-app psql $DATABASE_URL -c "SELECT 1"

# Fix connection string
export DATABASE_URL="postgresql://readur:password@db:5432/readur"
```

**Permission issues:** Resolve file and directory permission problems that prevent the container from accessing required resources.
```bash
# Fix volume permissions
sudo chown -R 1000:1000 ./uploads
sudo chmod -R 755 ./uploads

# Run with proper user
docker run --user $(id -u):$(id -g) readur:latest
```

#### Problem: Auto-update tools break PostgreSQL

**Symptoms:**
- Readur was working but suddenly stopped after a container update
- PostgreSQL container fails to start with "postgres: not found" error
- Container logs show version mismatch errors

**Cause:** Container auto-update tools like [WUD (What's Up Docker)](https://github.com/getwud/wud) or [Watchtower](https://github.com/containrrr/watchtower) may automatically update PostgreSQL to a version incompatible with Readur or with your existing data.

**Solution:** Exclude the PostgreSQL container from auto-updates by adding labels to your docker-compose.yml:

```yaml
postgres:
  image: postgres:16
  # Exclude from WUD and Watchtower auto-updates
  labels:
    - "wud.trigger.exclude=docker.autoupdate"
    - "com.centurylinklabs.watchtower.monitor-only=true"
  # ... rest of configuration
```

If PostgreSQL was already updated, you'll need to restore from backup or migrate your data:

```bash
# Stop containers
docker compose down

# Remove the corrupted postgres volume
docker volume rm readur_postgres_data

# Re-pull the correct version
docker pull postgres:16

# Restart (this creates a fresh database)
docker compose up -d

# If you have a backup, restore it:
gunzip -c backup.sql.gz | docker exec -i readur-postgres psql -U readur
```

For more details, see [GitHub Discussion #480](https://github.com/orgs/readur/discussions/480).

#### Problem: Build fails with Rust compilation errors

**Solutions:**

**Update Rust toolchain:** Ensure you're using the latest stable Rust version to avoid compatibility issues.
```bash
rustup update
rustup default stable
cargo clean
cargo build --release
```

**Clear build cache:** Remove stale build artifacts that may be causing compilation problems.
```bash
rm -rf target/
rm Cargo.lock
cargo build --release
```

**Install missing dependencies:** Ensure all required system dependencies are installed for compilation.
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

**PostgreSQL not running:** Start the PostgreSQL service and ensure it's configured to start automatically.
```bash
# Start PostgreSQL
sudo systemctl start postgresql

# Enable auto-start
sudo systemctl enable postgresql
```

**Authentication failure:** Configure PostgreSQL to accept connections from the Readur application.
```bash
# Edit pg_hba.conf
sudo nano /etc/postgresql/15/main/pg_hba.conf

# Add/modify line:
local   all   readur   md5
host    all   readur   127.0.0.1/32   md5

# Reload configuration
sudo systemctl reload postgresql
```

**Database doesn't exist:** Create the required database and user with appropriate permissions.
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

**Rollback failed migration:** When a database migration fails, identify the problematic migration and roll it back safely.
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

**Fix constraint violations:** Resolve database integrity issues that prevent migrations from completing.
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

**Reset stuck jobs:** Reset OCR jobs that have been stuck in processing state for an extended period.
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

**Clear failed jobs:** Reset failed OCR jobs to allow them to be retried.
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

**Wrong language configuration:** Verify and correct the OCR language settings for better text recognition.
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

**Poor image quality:** Improve the input image quality to enhance OCR accuracy.
```bash
# Debug PDF extraction
cargo run --bin debug_pdf_extraction problem_document.pdf --verbose

# Preprocess image
convert input.pdf -density 300 -depth 8 -strip -background white -alpha off output.pdf
```

**Install additional language packs:** Add support for additional languages if your documents contain text in multiple languages.
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

**Missing indexes:** Create or rebuild database indexes to improve query performance.
```sql
-- Create missing indexes
CREATE INDEX CONCURRENTLY idx_documents_content_gin 
ON documents USING gin(to_tsvector('english', content));

-- Rebuild existing indexes
REINDEX INDEX CONCURRENTLY idx_documents_content_gin;
```

**Table bloat:** Reduce database table bloat that can slow down queries.
```sql
-- Check table size
SELECT pg_size_pretty(pg_total_relation_size('documents'));

-- Vacuum and analyze
VACUUM (FULL, ANALYZE) documents;

-- For less downtime
VACUUM (ANALYZE) documents;
```

**Optimize configuration:** Tune PostgreSQL configuration parameters for better performance.
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

**Limit concurrent operations:** Reduce the number of simultaneous operations to decrease memory pressure.
```yaml
# Reduce concurrent OCR jobs
CONCURRENT_OCR_JOBS: 2

# Limit database connections
DATABASE_MAX_CONNECTIONS: 20

# Reduce batch sizes
BATCH_SIZE: 50
```

**Configure memory limits:** Set explicit memory limits to prevent the application from consuming excessive resources.
```bash
# Docker memory limit
docker run -m 2g readur:latest

# Systemd memory limit
[Service]
MemoryMax=2G
MemoryHigh=1.5G
```

### Docker Volume Mount Issues

#### Problem: "read-only file system" error when starting container

**Symptoms:**
- Container fails to start
- Error: `error while creating mount source path '...': mkdir /data: read-only file system`
- Error: `cannot create directory '...': Read-only file system`

**Diagnosis:**
```bash
# Check if the path exists on the host
ls -la /DATA/AppData/readur/

# Check filesystem mount options
mount | grep -E "DATA|data"

# Verify Docker can write to the parent directory
docker run --rm -v /DATA:/test alpine touch /test/testfile && echo "Writable" || echo "Read-only"
```

**Common Causes and Solutions:**

**Cause 1: Using relative path instead of absolute path**

This is the most common issue, especially with Portainer deployments:

```yaml
# WRONG - Relative path (starts with ./)
volumes:
  - ./DATA/AppData/readur/uploads:/app/uploads

# CORRECT - Absolute path (starts with /)
volumes:
  - /DATA/AppData/readur/uploads:/app/uploads
```

The `./` prefix means "relative to the docker-compose.yml location," which may not be where you expect, especially in Portainer.

**Cause 2: Parent directory doesn't exist**

Docker cannot create nested directories if parent directories don't exist:

```bash
# Create the full directory structure first
mkdir -p /DATA/AppData/readur/uploads
mkdir -p /DATA/AppData/readur/watch
mkdir -p /DATA/AppData/readur/postgres_data

# Set appropriate permissions
chmod 755 /DATA/AppData/readur/uploads
chmod 755 /DATA/AppData/readur/watch
```

**Cause 3: Case sensitivity mismatch**

Linux filesystems are case-sensitive:

```bash
# These are DIFFERENT directories on Linux:
/DATA/AppData    # Uppercase
/data/appdata    # Lowercase

# Check what actually exists
ls -la / | grep -i data
```

**Cause 4: Filesystem mounted as read-only**

Some NAS systems mount certain paths as read-only:

```bash
# Check mount options
mount | grep DATA

# If read-only, you may need to use a different path
# or remount with write permissions (consult your NAS documentation)
```

#### Problem: "permission denied" when container tries to write files

**Symptoms:**
- Container starts but crashes when writing files
- Error: `Permission denied` in container logs
- Uploaded files fail to save

**Diagnosis:**
```bash
# Check host directory ownership
ls -la /path/to/uploads/

# Check what user the container runs as
docker exec readur id

# Test write access from container
docker exec readur touch /app/uploads/test.txt
```

**Solutions:**

**Fix ownership on host:**
```bash
# Option 1: Change ownership to match container user (usually UID 1000)
sudo chown -R 1000:1000 /path/to/uploads

# Option 2: Make directory world-writable (less secure)
sudo chmod -R 777 /path/to/uploads
```

**Use Docker user mapping:**
```yaml
services:
  readur:
    user: "${UID}:${GID}"  # Run as current user
    volumes:
      - /path/to/uploads:/app/uploads
```

#### Problem: Volume data not persisting after container restart

**Symptoms:**
- Data disappears after `docker-compose down && docker-compose up`
- Uploaded documents are gone after restart

**Diagnosis:**
```bash
# List all volumes
docker volume ls

# Inspect volume to see mount point
docker volume inspect readur_uploads

# Check if using bind mount or named volume
docker inspect readur | grep -A 10 "Mounts"
```

**Solutions:**

**Ensure volumes are defined correctly:**
```yaml
services:
  readur:
    volumes:
      # Bind mount - data stored at host path
      - /srv/readur/uploads:/app/uploads

      # OR named volume - Docker manages storage
      - readur_uploads:/app/uploads

volumes:
  readur_uploads:  # Must be defined if using named volume
```

**Don't use `docker-compose down -v`:**
```bash
# This DELETES volumes (and your data!)
docker-compose down -v  # DANGEROUS

# This preserves volumes
docker-compose down     # Safe
docker-compose up -d
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

**Insufficient disk space:** Free up disk space or move the storage location to a disk with more capacity.
```bash
# Clean up old files
find /uploads/temp -type f -mtime +7 -delete

# Move to larger disk
mv /uploads /mnt/large-disk/uploads
ln -s /mnt/large-disk/uploads /uploads
```

**Permission denied:** Ensure the application has the necessary permissions to read and write files.
```bash
# Fix ownership
sudo chown -R readur:readur /uploads

# Fix permissions
sudo chmod -R 755 /uploads
```

**File size limit exceeded:** Increase the maximum file size limits in your web server or application configuration.
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

**Authentication errors:** Verify and update your S3 credentials or IAM role configuration.
```bash
# Update credentials
export AWS_ACCESS_KEY_ID="your_key"
export AWS_SECRET_ACCESS_KEY="your_secret"

# Or use IAM role
aws configure set role_arn arn:aws:iam::123456789:role/ReadurRole
```

**Network issues:** Troubleshoot network connectivity problems that prevent S3 access.
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

**JWT secret mismatch:** Ensure all application instances use the same JWT secret for token validation.
```bash
# Ensure consistent secret across instances
export JWT_SECRET="same-secret-all-instances"

# Restart all instances
docker-compose restart
```

**Password reset:** Reset user passwords when authentication issues prevent normal login.
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

**Verify OIDC configuration:** Check that your OIDC provider settings are correctly configured and accessible.
```bash
# Test OIDC endpoint
curl https://auth.example.com/.well-known/openid-configuration

# Verify redirect URI
echo $OIDC_REDIRECT_URI  # Must match exactly in provider
```

**Certificate issues:** Resolve SSL/TLS certificate problems that prevent OIDC authentication.
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

**Proxy configuration:** Configure your reverse proxy to properly handle WebSocket connections.
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

**Firewall rules:** Ensure firewall settings allow WebSocket connections on the required ports.
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