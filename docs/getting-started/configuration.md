# Configuration Guide

Configure Readur for your specific needs and optimize for your workload.

## Configuration Overview

Readur uses environment variables for configuration, making it easy to deploy in containerized environments. Configuration can be set through:

1. **Environment variables** - Direct system environment
2. **`.env` file** - Docker Compose automatically loads this
3. **`docker-compose.yml`** - Directly in the compose file
4. **Kubernetes ConfigMaps** - For K8s deployments

## Essential Configuration

### Security Settings

These MUST be changed from defaults in production:

```bash
# Generate secure secrets
JWT_SECRET=$(openssl rand -base64 32)
DB_PASSWORD=$(openssl rand -base64 32)

# CRITICAL: Always change JWT_SECRET from default!
# Default values are insecure and should never be used in production

# Set admin password
ADMIN_PASSWORD=your_secure_password_here

# Enable HTTPS (reverse proxy recommended)
FORCE_HTTPS=true
SECURE_COOKIES=true

# WARNING: Only disable SSL verification for development/testing
# S3_VERIFY_SSL=false  # NEVER use in production
```

### Database Configuration

```bash
# PostgreSQL connection
DATABASE_URL=postgresql://readur:${DB_PASSWORD}@postgres:5432/readur
# WARNING: Never include passwords directly in DATABASE_URL in config files

# Connection pool settings
DB_POOL_SIZE=20
DB_MAX_OVERFLOW=40
DB_POOL_TIMEOUT=30

# PostgreSQL specific optimizations
POSTGRES_SHARED_BUFFERS=256MB
POSTGRES_EFFECTIVE_CACHE_SIZE=1GB
```

### Storage Configuration

#### Local Storage (Default)

```bash
# File storage paths
UPLOAD_PATH=/app/uploads
TEMP_PATH=/app/temp

# Size limits
MAX_FILE_SIZE_MB=50
TOTAL_STORAGE_LIMIT_GB=100

# File types
ALLOWED_FILE_TYPES=pdf,png,jpg,jpeg,tiff,bmp,gif,txt,rtf,doc,docx
```

#### S3 Storage (Scalable)

```bash
# Enable S3 backend
STORAGE_BACKEND=s3
S3_ENABLED=true

# AWS S3
S3_BUCKET_NAME=readur-documents
S3_REGION=us-east-1
S3_ACCESS_KEY_ID=your_access_key
S3_SECRET_ACCESS_KEY=your_secret_key

# Or S3-compatible (MinIO, Wasabi, etc.)
S3_ENDPOINT=https://s3.example.com
S3_PATH_STYLE=true  # For MinIO
```

## OCR Configuration

### Language Settings

```bash
# Single language (fastest)
OCR_LANGUAGE=eng

# Multiple languages
OCR_LANGUAGE=eng+deu+fra+spa

# Available languages (partial list):
# eng - English
# deu - German (Deutsch)
# fra - French (Français)
# spa - Spanish (Español)
# ita - Italian (Italiano)
# por - Portuguese
# rus - Russian
# chi_sim - Chinese Simplified
# jpn - Japanese
# ara - Arabic
```

### Performance Tuning

```bash
# Concurrent processing
CONCURRENT_OCR_JOBS=3  # OCR runtime uses 3 threads
OCR_WORKER_THREADS=2   # Background runtime uses 2 threads
# Note: Database runtime also uses 2 threads

# Timeouts and limits
OCR_TIMEOUT_SECONDS=300
OCR_MAX_PAGES=500
MAX_FILE_SIZE_MB=100

# Memory management
OCR_MEMORY_LIMIT_MB=512  # Per job
ENABLE_MEMORY_PROFILING=false

# Processing options
OCR_DPI=300  # Higher = better quality, slower
ENABLE_PREPROCESSING=true
ENABLE_AUTO_ROTATION=true
ENABLE_DESKEW=true
```

### Quality vs Speed

#### High Quality (Slow)
```bash
OCR_QUALITY_PRESET=high
OCR_DPI=300
ENABLE_PREPROCESSING=true
ENABLE_DESKEW=true
ENABLE_AUTO_ROTATION=true
OCR_ENGINE_MODE=3  # LSTM only
```

#### Balanced (Default)
```bash
OCR_QUALITY_PRESET=balanced
OCR_DPI=200
ENABLE_PREPROCESSING=true
ENABLE_DESKEW=false
ENABLE_AUTO_ROTATION=true
OCR_ENGINE_MODE=2  # LSTM + Legacy
```

#### Fast (Lower Quality)
```bash
OCR_QUALITY_PRESET=fast
OCR_DPI=150
ENABLE_PREPROCESSING=false
ENABLE_DESKEW=false
ENABLE_AUTO_ROTATION=false
OCR_ENGINE_MODE=0  # Legacy only
```

## Source Synchronization

### Watch Folders

```bash
# Global watch folder
WATCH_FOLDER=/app/watch
WATCH_INTERVAL_SECONDS=60
FILE_STABILITY_CHECK_MS=2000

# Per-user watch folders
ENABLE_PER_USER_WATCH=true
USER_WATCH_BASE_DIR=/app/user_watch

# Processing rules
WATCH_PROCESS_HIDDEN_FILES=false
WATCH_RECURSIVE=true
WATCH_MAX_DEPTH=5
DELETE_AFTER_IMPORT=false
```

### WebDAV Sources

```bash
# Default WebDAV settings
WEBDAV_TIMEOUT_SECONDS=30
WEBDAV_MAX_RETRIES=3
WEBDAV_CHUNK_SIZE_MB=10
WEBDAV_VERIFY_SSL=true
```

### S3 Sources

```bash
# S3 sync settings
S3_SYNC_INTERVAL_MINUTES=30
S3_BATCH_SIZE=100
S3_MULTIPART_THRESHOLD_MB=100
S3_CONCURRENT_DOWNLOADS=4
```

## Authentication & Security

### Local Authentication

```bash
# Password policy
PASSWORD_MIN_LENGTH=12
PASSWORD_REQUIRE_UPPERCASE=true
PASSWORD_REQUIRE_NUMBERS=true
PASSWORD_REQUIRE_SPECIAL=true

# Session management
SESSION_TIMEOUT_MINUTES=60
REMEMBER_ME_DURATION_DAYS=30
MAX_LOGIN_ATTEMPTS=5
LOCKOUT_DURATION_MINUTES=15
```

### OIDC/SSO Configuration

```bash
# Enable OIDC
OIDC_ENABLED=true

# Provider configuration
OIDC_ISSUER=https://login.microsoftonline.com/tenant-id/v2.0
OIDC_CLIENT_ID=your-client-id
OIDC_CLIENT_SECRET=your-client-secret
OIDC_REDIRECT_URI=https://readur.example.com/auth/callback

# Optional settings
OIDC_SCOPE=openid profile email
OIDC_USER_CLAIM=email
OIDC_GROUPS_CLAIM=groups
OIDC_ADMIN_GROUP=readur-admins

# Auto-provisioning
OIDC_AUTO_CREATE_USERS=true
OIDC_DEFAULT_ROLE=user
```

## Search Configuration

### Search Engine

```bash
# PostgreSQL Full-Text Search settings
SEARCH_LANGUAGE=english
SEARCH_RANKING_NORMALIZATION=32
ENABLE_PHRASE_SEARCH=true
ENABLE_FUZZY_SEARCH=true
FUZZY_SEARCH_DISTANCE=2

# Search results
SEARCH_RESULTS_PER_PAGE=20
SEARCH_SNIPPET_LENGTH=200
SEARCH_HIGHLIGHT_TAG=mark
```

### Search Performance

```bash
# Index management
AUTO_REINDEX=true
REINDEX_SCHEDULE=0 3 * * *  # 3 AM daily
SEARCH_CACHE_TTL_SECONDS=300
SEARCH_CACHE_SIZE_MB=100

# Query optimization
MAX_SEARCH_TERMS=10
ENABLE_SEARCH_SUGGESTIONS=true
SUGGESTION_MIN_LENGTH=3
```

## Monitoring & Logging

### Logging Configuration

```bash
# Log levels: DEBUG, INFO, WARNING, ERROR, CRITICAL
LOG_LEVEL=INFO
LOG_FORMAT=json  # or text

# Log outputs
LOG_TO_FILE=true
LOG_FILE_PATH=/app/logs/readur.log
LOG_FILE_MAX_SIZE_MB=100
LOG_FILE_BACKUP_COUNT=10

# Detailed logging
LOG_SQL_QUERIES=false
LOG_HTTP_REQUESTS=true
LOG_OCR_DETAILS=false
```

### Health Monitoring

```bash
# Health check endpoints
HEALTH_CHECK_ENABLED=true
HEALTH_CHECK_PATH=/health
METRICS_ENABLED=true
METRICS_PATH=/metrics

# Alerting thresholds
ALERT_QUEUE_SIZE=100
ALERT_OCR_FAILURE_RATE=0.1
ALERT_DISK_USAGE_PERCENT=80
ALERT_MEMORY_USAGE_PERCENT=90
```

## Performance Optimization

### System Resources

```bash
# Memory limits
MEMORY_LIMIT_MB=2048
MEMORY_SOFT_LIMIT_MB=1536

# CPU settings
CPU_CORES=4
WORKER_PROCESSES=auto  # or specific number
WORKER_THREADS=2

# Connection limits
MAX_CONNECTIONS=100
CONNECTION_TIMEOUT=30
```

### Caching

```bash
# Enable caching layers
ENABLE_CACHE=true
CACHE_TYPE=redis  # or memory

# Redis cache (if used)
REDIS_URL=redis://redis:6379/0
REDIS_MAX_CONNECTIONS=50

# Cache TTLs
DOCUMENT_CACHE_TTL=3600
SEARCH_CACHE_TTL=300
USER_CACHE_TTL=1800
```

### Queue Management

```bash
# Background job processing
QUEUE_TYPE=database  # or redis
MAX_QUEUE_SIZE=1000
QUEUE_POLL_INTERVAL=5

# Job priorities
OCR_JOB_PRIORITY=5
SYNC_JOB_PRIORITY=3
CLEANUP_JOB_PRIORITY=1

# Retry configuration
MAX_JOB_RETRIES=3
RETRY_DELAY_SECONDS=60
EXPONENTIAL_BACKOFF=true
```

## Environment-Specific Configurations

### Development

```bash
# .env.development
DEBUG=true
LOG_LEVEL=DEBUG
RELOAD_ON_CHANGE=true
CONCURRENT_OCR_JOBS=1
DISABLE_RATE_LIMITING=true
```

### Staging

```bash
# .env.staging
DEBUG=false
LOG_LEVEL=INFO
CONCURRENT_OCR_JOBS=2
ENABLE_PROFILING=true
MOCK_EXTERNAL_SERVICES=true
```

### Production

```bash
# .env.production
DEBUG=false
LOG_LEVEL=WARNING
CONCURRENT_OCR_JOBS=8
ENABLE_RATE_LIMITING=true
SECURE_COOKIES=true
FORCE_HTTPS=true
```

## Configuration Validation

### Check Configuration

```bash
# Validate current configuration
docker exec readur python validate_config.py

# Test specific settings
docker exec readur python -c "
from config import settings
print(f'OCR Languages: {settings.OCR_LANGUAGE}')
print(f'Storage Backend: {settings.STORAGE_BACKEND}')
print(f'Max File Size: {settings.MAX_FILE_SIZE_MB}MB')
"
```

### Common Validation Errors

```bash
# Missing required S3 credentials
ERROR: S3_ENABLED=true but S3_BUCKET_NAME not set

# Invalid language code
ERROR: OCR_LANGUAGE 'xyz' not supported

# Insufficient resources
WARNING: CONCURRENT_OCR_JOBS=8 but only 2 CPU cores available
```

## Configuration Best Practices

### Security

1. **Never commit secrets** - Use `.env` files and add to `.gitignore`
2. **Change JWT_SECRET immediately** - Never use default values
3. **Rotate secrets regularly** - Especially JWT_SECRET and API keys
4. **Use strong passwords** - Minimum 16 characters for admin
5. **Enable HTTPS** - Always in production
6. **Restrict file types** - Only allow necessary formats
7. **Never expose secrets in command lines** - They appear in process lists
8. **Always verify SSL certificates** - Only disable for local development

### Performance

1. **Match workers to cores** - CONCURRENT_OCR_JOBS ≤ CPU cores
2. **Monitor memory usage** - Adjust limits based on usage
3. **Use S3 for scale** - Local storage limited by disk
4. **Enable caching** - Reduces database load
5. **Tune PostgreSQL** - Adjust shared_buffers and work_mem

### Reliability

1. **Set reasonable timeouts** - Prevent hanging jobs
2. **Configure retries** - Handle transient failures
3. **Enable health checks** - For load balancer integration
4. **Set up logging** - Essential for troubleshooting
5. **Regular backups** - Automate database backups

## Configuration Examples

### Small Office (5-10 users)

```bash
# Minimal resources, local storage
CONCURRENT_OCR_JOBS=2
MEMORY_LIMIT_MB=1024
STORAGE_BACKEND=local
MAX_FILE_SIZE_MB=20
SEARCH_CACHE_TTL=600
```

### Medium Business (50-100 users)

```bash
# Balanced performance, S3 storage
CONCURRENT_OCR_JOBS=4
MEMORY_LIMIT_MB=4096
STORAGE_BACKEND=s3
MAX_FILE_SIZE_MB=50
ENABLE_CACHE=true
CACHE_TYPE=redis
```

### Enterprise (500+ users)

```bash
# High performance, full features
CONCURRENT_OCR_JOBS=16
MEMORY_LIMIT_MB=16384
STORAGE_BACKEND=s3
MAX_FILE_SIZE_MB=100
ENABLE_CACHE=true
CACHE_TYPE=redis
QUEUE_TYPE=redis
OIDC_ENABLED=true
```

## Next Steps

- [Installation Guide](installation.md) - Deploy Readur
- [User Guide](../user-guide.md) - Learn the interface
- [API Reference](../api-reference.md) - Integrate with Readur
- [Deployment Guide](../deployment.md) - Production setup