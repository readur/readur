# Configuration Reference

This document provides a comprehensive reference for all configuration options available in Readur, including environment variables, configuration files, and runtime settings.

## Environment Variables

### Core Configuration

| Variable | Type | Default | Description | Required |
|----------|------|---------|-------------|----------|
| `DATABASE_URL` | String | `postgresql://readur:readur@localhost/readur` | PostgreSQL connection string (takes priority over individual vars) | Yes* |
| `POSTGRES_HOST` | String | `localhost` | PostgreSQL host (used if DATABASE_URL not set) | No |
| `POSTGRES_PORT` | String | `5432` | PostgreSQL port (used if DATABASE_URL not set) | No |
| `POSTGRES_DB` | String | `readur` | PostgreSQL database name (used if DATABASE_URL not set) | No |
| `POSTGRES_USER` | String | `readur` | PostgreSQL username (used if DATABASE_URL not set) | No |
| `POSTGRES_PASSWORD` | String | `readur` | PostgreSQL password (used if DATABASE_URL not set) | No |
| `SERVER_ADDRESS` | String | `0.0.0.0:8080` | Server bind address (host:port) | No |
| `SERVER_HOST` | String | `0.0.0.0` | Server host (used if SERVER_ADDRESS not set) | No |
| `SERVER_PORT` | String | `8080` | Server port (used if SERVER_ADDRESS not set) | No |
| `JWT_SECRET` | String | Auto-generated | Secret key for JWT tokens (min 32 chars) | Recommended |
| `SESSION_SECRET` | String | Auto-generated | Secret for session encryption | Recommended |
| `UPLOAD_PATH` | String | `./uploads` | Directory for file uploads | No |
| `ALLOWED_FILE_TYPES` | String | `pdf,txt,doc,docx,png,jpg,jpeg` | Comma-separated allowed extensions | No |
| `LOG_LEVEL` | String | `info` | Logging level (debug, info, warn, error) | No |
| `LOG_FORMAT` | String | `text` | Log format (text, json) | No |

### Authentication & Security

| Variable | Type | Default | Description | Required |
|----------|------|---------|-------------|----------|
| `AUTH_ENABLED` | Boolean | `true` | Enable authentication | No |
| `DEFAULT_USER_ROLE` | String | `viewer` | Default role for new users (admin, editor, viewer) | No |
| `AUTO_CREATE_USERS` | Boolean | `false` | Auto-create users on first login (OIDC) | No |
| `SESSION_TIMEOUT` | Integer | `3600` | Session timeout in seconds | No |
| `PASSWORD_MIN_LENGTH` | Integer | `8` | Minimum password length | No |
| `REQUIRE_EMAIL_VERIFICATION` | Boolean | `false` | Require email verification | No |
| `MAX_LOGIN_ATTEMPTS` | Integer | `5` | Maximum failed login attempts | No |
| `LOCKOUT_DURATION` | Integer | `900` | Account lockout duration (seconds) | No |

### OIDC/SSO Configuration

| Variable | Type | Default | Description | Required |
|----------|------|---------|-------------|----------|
| `OIDC_ENABLED` | Boolean | `false` | Enable OIDC authentication | No |
| `OIDC_CLIENT_ID` | String | - | OIDC client ID | If OIDC enabled |
| `OIDC_CLIENT_SECRET` | String | - | OIDC client secret | If OIDC enabled |
| `OIDC_ISSUER_URL` | String | - | OIDC issuer URL | If OIDC enabled |
| `OIDC_REDIRECT_URI` | String | - | OIDC redirect URI | If OIDC enabled |
| `OIDC_SCOPES` | String | `openid profile email` | OIDC scopes | No |
| `OIDC_USER_INFO_ENDPOINT` | String | Auto-discovered | User info endpoint | No |
| `OIDC_TOKEN_ENDPOINT` | String | Auto-discovered | Token endpoint | No |
| `OIDC_AUTH_ENDPOINT` | String | Auto-discovered | Authorization endpoint | No |

### Storage Configuration

#### Local Storage

| Variable | Type | Default | Description | Required |
|----------|------|---------|-------------|----------|
| `STORAGE_TYPE` | String | `local` | Storage backend (local, s3, azure) | No |
| `LOCAL_STORAGE_PATH` | String | `./uploads` | Local storage directory | No |
| `TEMP_STORAGE_PATH` | String | `./uploads/temp` | Temporary files directory | No |
| `THUMBNAIL_PATH` | String | `./uploads/thumbnails` | Thumbnail storage directory | No |
| `BACKUP_PATH` | String | `./uploads/backups` | Backup directory | No |

#### S3 Storage

| Variable | Type | Default | Description | Required |
|----------|------|---------|-------------|----------|
| `S3_ENABLED` | Boolean | `false` | Enable S3 storage backend | No |
| `S3_BUCKET_NAME` | String | - | S3 bucket name | If S3 enabled |
| `S3_ACCESS_KEY_ID` | String | - | AWS Access Key ID | If S3 enabled |
| `S3_SECRET_ACCESS_KEY` | String | - | AWS Secret Access Key | If S3 enabled |
| `S3_REGION` | String | `us-east-1` | AWS region | No |
| `S3_ENDPOINT_URL` | String | - | Custom S3 endpoint (for MinIO, etc.) | No |
| `S3_PREFIX` | String | - | S3 key prefix | No |
| `S3_USE_SSL` | Boolean | `true` | Use HTTPS for S3 | No |
| `S3_VERIFY_SSL` | Boolean | `true` | Verify SSL certificates | No |
| `S3_STORAGE_CLASS` | String | `STANDARD` | S3 storage class | No |
| `S3_SERVER_SIDE_ENCRYPTION` | String | - | Server-side encryption (AES256, aws:kms) | No |
| `S3_KMS_KEY_ID` | String | - | KMS key ID for encryption | No |

### Watch Directory Configuration

| Variable | Type | Default | Description | Required |
|----------|------|---------|-------------|----------|
| `WATCH_FOLDER` | String | `./watch` | Global watch directory | No |
| `USER_WATCH_BASE_DIR` | String | `./user_watch` | Base directory for per-user folders | No |
| `ENABLE_PER_USER_WATCH` | Boolean | `false` | Enable per-user watch directories | No |
| `WATCH_INTERVAL_SECONDS` | Integer | `60` | Scan interval in seconds | No |
| `FILE_STABILITY_CHECK_MS` | Integer | `2000` | File stability check delay (ms) | No |
| `MAX_FILE_AGE_HOURS` | Integer | `24` | Maximum file age to process | No |
| `WATCH_RECURSIVE` | Boolean | `true` | Watch subdirectories recursively | No |
| `WATCH_FILE_PATTERNS` | String | `*` | File patterns to watch (glob) | No |
| `WATCH_IGNORE_PATTERNS` | String | `.*,~*,*.tmp` | Patterns to ignore | No |
| `MOVE_AFTER_PROCESSING` | Boolean | `false` | Move files after processing | No |
| `PROCESSED_FILES_DIR` | String | `./processed` | Directory for processed files | No |

### OCR Configuration

| Variable | Type | Default | Description | Required |
|----------|------|---------|-------------|----------|
| `OCR_ENABLED` | Boolean | `true` | Enable OCR processing | No |
| `OCR_LANGUAGE` | String | `eng` | Default OCR language(s) | No |
| `OCR_ENGINE` | String | `tesseract` | OCR engine (tesseract, cloud) | No |
| `CONCURRENT_OCR_JOBS` | Integer | CPU cores / 2 | Concurrent OCR workers | No |
| `OCR_TIMEOUT_SECONDS` | Integer | `300` | OCR timeout per document | No |
| `OCR_RETRY_ATTEMPTS` | Integer | `3` | OCR retry attempts | No |
| `OCR_RETRY_DELAY` | Integer | `60` | Delay between retries (seconds) | No |
| `OCR_CONFIDENCE_THRESHOLD` | Float | `0.6` | Minimum OCR confidence | No |
| `MAX_FILE_SIZE_MB` | Integer | `50` | Maximum file size for upload | No |
| `MAX_PDF_SIZE_MB` | Integer | `100` | Maximum PDF file size for OCR processing | No |
| `MAX_OFFICE_DOCUMENT_SIZE_MB` | Integer | `100` | Maximum Office document size for text extraction | No |
| `OCR_DPI` | Integer | `300` | DPI for image processing | No |
| `OCR_PSM` | Integer | `3` | Tesseract page segmentation mode | No |
| `OCR_OEM` | Integer | `1` | Tesseract OCR engine mode | No |
| `TESSERACT_DATA_PATH` | String | `/usr/share/tesseract-ocr/4.00/tessdata` | Tesseract data directory | No |

### Database Configuration

| Variable | Type | Default | Description | Required |
|----------|------|---------|-------------|----------|
| `DATABASE_MAX_CONNECTIONS` | Integer | `32` | Maximum database connections | No |
| `DATABASE_MIN_CONNECTIONS` | Integer | `5` | Minimum idle connections | No |
| `DATABASE_CONNECT_TIMEOUT` | Integer | `5` | Connection timeout (seconds) | No |
| `DATABASE_ACQUIRE_TIMEOUT` | Integer | `10` | Acquire timeout (seconds) | No |
| `DATABASE_IDLE_TIMEOUT` | Integer | `600` | Idle connection timeout | No |
| `DATABASE_MAX_LIFETIME` | Integer | `1800` | Max connection lifetime | No |
| `DATABASE_SSL_MODE` | String | `prefer` | SSL mode (disable, prefer, require) | No |
| `DATABASE_SSL_CERT` | String | - | Path to SSL certificate | No |
| `DATABASE_SSL_KEY` | String | - | Path to SSL key | No |
| `DATABASE_SSL_ROOT_CERT` | String | - | Path to root certificate | No |

### Performance & Resources

| Variable | Type | Default | Description | Required |
|----------|------|---------|-------------|----------|
| `MEMORY_LIMIT_MB` | Integer | `2048` | Memory limit in MB | No |
| `CPU_CORES` | Integer | Auto-detect | Number of CPU cores to use | No |
| `WORKER_THREADS` | Integer | CPU cores | Worker thread count | No |
| `BLOCKING_THREADS` | Integer | `512` | Blocking thread pool size | No |
| `CACHE_SIZE_MB` | Integer | `256` | In-memory cache size | No |
| `BATCH_SIZE` | Integer | `100` | Default batch processing size | No |
| `PARALLEL_UPLOADS` | Integer | `5` | Concurrent file uploads | No |
| `REQUEST_TIMEOUT` | Integer | `30` | HTTP request timeout (seconds) | No |
| `RATE_LIMIT_ENABLED` | Boolean | `true` | Enable rate limiting | No |
| `RATE_LIMIT_PER_MINUTE` | Integer | `100` | Requests per minute limit | No |

### Notification Configuration

| Variable | Type | Default | Description | Required |
|----------|------|---------|-------------|----------|
| `NOTIFICATIONS_ENABLED` | Boolean | `true` | Enable notifications | No |
| `EMAIL_ENABLED` | Boolean | `false` | Enable email notifications | No |
| `SMTP_HOST` | String | - | SMTP server host | If email enabled |
| `SMTP_PORT` | Integer | `587` | SMTP server port | No |
| `SMTP_USERNAME` | String | - | SMTP username | If email enabled |
| `SMTP_PASSWORD` | String | - | SMTP password | If email enabled |
| `SMTP_FROM_ADDRESS` | String | - | From email address | If email enabled |
| `SMTP_USE_TLS` | Boolean | `true` | Use TLS for SMTP | No |
| `WEBHOOK_ENABLED` | Boolean | `false` | Enable webhook notifications | No |
| `WEBHOOK_URL` | String | - | Webhook endpoint URL | If webhook enabled |
| `WEBHOOK_SECRET` | String | - | Webhook signing secret | No |

### Monitoring & Metrics

| Variable | Type | Default | Description | Required |
|----------|------|---------|-------------|----------|
| `METRICS_ENABLED` | Boolean | `true` | Enable metrics collection | No |
| `PROMETHEUS_ENABLED` | Boolean | `false` | Enable Prometheus metrics | No |
| `PROMETHEUS_PORT` | Integer | `9090` | Prometheus metrics port | No |
| `HEALTH_CHECK_PATH` | String | `/health` | Health check endpoint | No |
| `READY_CHECK_PATH` | String | `/ready` | Readiness check endpoint | No |
| `METRICS_PATH` | String | `/metrics` | Metrics endpoint | No |
| `TRACING_ENABLED` | Boolean | `false` | Enable distributed tracing | No |
| `JAEGER_ENDPOINT` | String | - | Jaeger collector endpoint | If tracing enabled |
| `TRACE_SAMPLE_RATE` | Float | `0.1` | Trace sampling rate (0-1) | No |

### Network Configuration

| Variable | Type | Default | Description | Required |
|----------|------|---------|-------------|----------|
| `CORS_ENABLED` | Boolean | `true` | Enable CORS | No |
| `CORS_ALLOWED_ORIGINS` | String | `*` | Allowed CORS origins | No |
| `CORS_ALLOWED_METHODS` | String | `GET,POST,PUT,DELETE,OPTIONS` | Allowed HTTP methods | No |
| `CORS_ALLOWED_HEADERS` | String | `*` | Allowed headers | No |
| `CORS_MAX_AGE` | Integer | `3600` | CORS preflight cache (seconds) | No |
| `PROXY_COUNT` | Integer | `0` | Number of reverse proxies | No |
| `TRUSTED_PROXIES` | String | - | Comma-separated trusted proxy IPs | No |
| `WEBSOCKET_ENABLED` | Boolean | `true` | Enable WebSocket support | No |
| `WEBSOCKET_MAX_CONNECTIONS` | Integer | `1000` | Maximum WebSocket connections | No |

### Feature Flags

| Variable | Type | Default | Description | Required |
|----------|------|---------|-------------|----------|
| `FEATURE_ADVANCED_SEARCH` | Boolean | `true` | Enable advanced search | No |
| `FEATURE_LABELS` | Boolean | `true` | Enable document labels | No |
| `FEATURE_SOURCES` | Boolean | `true` | Enable external sources | No |
| `FEATURE_ANALYTICS` | Boolean | `true` | Enable analytics dashboard | No |
| `FEATURE_NOTIFICATIONS` | Boolean | `true` | Enable notifications | No |
| `FEATURE_MULTI_LANGUAGE_OCR` | Boolean | `true` | Enable multi-language OCR | No |
| `FEATURE_WEBDAV` | Boolean | `true` | Enable WebDAV sync | No |
| `FEATURE_API_V2` | Boolean | `false` | Enable API v2 endpoints | No |

## Database Connection Priority

The database connection can be configured in two ways:

1. **Using `DATABASE_URL`** (takes priority if set):
   ```bash
   DATABASE_URL=postgresql://username:password@host:port/database
   ```

2. **Using individual PostgreSQL variables** (used if `DATABASE_URL` is not set):
   ```bash
   POSTGRES_HOST=localhost
   POSTGRES_PORT=5432
   POSTGRES_DB=readur
   POSTGRES_USER=readur
   POSTGRES_PASSWORD=your_password
   ```

This flexibility allows for easy deployment across different platforms:
- **Docker/Kubernetes**: Often provide individual variables
- **Heroku/Railway**: Typically provide `DATABASE_URL`
- **Local Development**: Use either method based on preference

## Configuration Files

### Main Configuration (readur.yml)

```yaml
# readur.yml - Main configuration file
server:
  address: 0.0.0.0
  port: 8080
  workers: 4

database:
  url: postgresql://readur:password@localhost/readur
  max_connections: 32
  min_connections: 5

storage:
  type: s3  # or 'local'
  s3:
    bucket: readur-documents
    region: us-east-1
    prefix: documents/

ocr:
  enabled: true
  language: eng+fra+deu
  concurrent_jobs: 4
  timeout: 300

auth:
  jwt_secret: ${JWT_SECRET}
  session_timeout: 3600
  
oidc:
  enabled: false
  client_id: ${OIDC_CLIENT_ID}
  client_secret: ${OIDC_CLIENT_SECRET}
  issuer_url: https://auth.example.com
```

### Docker Compose Override

```yaml
# docker-compose.override.yml
version: '3.8'
services:
  readur:
    environment:
      - DATABASE_URL=postgresql://readur:${DB_PASSWORD}@db:5432/readur
      - JWT_SECRET=${JWT_SECRET}
      - S3_ACCESS_KEY_ID=${AWS_ACCESS_KEY_ID}
      - S3_SECRET_ACCESS_KEY=${AWS_SECRET_ACCESS_KEY}
      - LOG_LEVEL=debug
    volumes:
      - ./config/readur.yml:/app/config/readur.yml:ro
```

### Kubernetes ConfigMap

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: readur-config
data:
  SERVER_PORT: "8080"
  DATABASE_MAX_CONNECTIONS: "50"
  OCR_LANGUAGE: "eng+spa+fra"
  CONCURRENT_OCR_JOBS: "8"
  LOG_LEVEL: "info"
  CORS_ALLOWED_ORIGINS: "https://app.example.com"
```

### Kubernetes Secret

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: readur-secrets
type: Opaque
stringData:
  DATABASE_URL: "postgresql://readur:password@postgres:5432/readur"
  JWT_SECRET: "your-secure-random-secret-min-32-chars"
  S3_ACCESS_KEY_ID: "AKIAIOSFODNN7EXAMPLE"
  S3_SECRET_ACCESS_KEY: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
```

## Configuration Precedence

Configuration values are loaded in the following order (later sources override earlier ones):

1. **Default values** (built into application)
2. **Configuration file** (`readur.yml`)
3. **Environment variables**
4. **Command-line arguments**
5. **Database settings** (for user-specific settings)

## Multi-Environment Configuration

### Development

```bash
# .env.development
DATABASE_URL=postgresql://readur:readur@localhost/readur_dev
SERVER_PORT=8080
LOG_LEVEL=debug
OCR_ENABLED=false
S3_ENABLED=false
CORS_ALLOWED_ORIGINS=http://localhost:3000
```

### Staging

```bash
# .env.staging
DATABASE_URL=postgresql://readur:${DB_PASSWORD}@db-staging.internal/readur_staging
SERVER_PORT=8080
LOG_LEVEL=info
S3_ENABLED=true
S3_BUCKET_NAME=readur-staging
CORS_ALLOWED_ORIGINS=https://staging.readur.app
```

### Production

```bash
# .env.production
DATABASE_URL=postgresql://readur:${DB_PASSWORD}@db-prod.internal/readur_prod
SERVER_PORT=8080
LOG_LEVEL=warn
S3_ENABLED=true
S3_BUCKET_NAME=readur-production
CORS_ALLOWED_ORIGINS=https://readur.app
RATE_LIMIT_ENABLED=true
PROMETHEUS_ENABLED=true
```

## Dynamic Configuration

Some settings can be changed at runtime without restart:

### User Settings (per-user)
- Theme preference
- Language preference
- Items per page
- Email notifications
- OCR default language

### System Settings (admin only)
- OCR concurrent jobs
- Rate limits
- Feature flags
- Notification settings

Access via API:
```bash
# Get current settings
GET /api/settings

# Update settings
PUT /api/settings
{
  "ocr_concurrent_jobs": 6,
  "rate_limit_per_minute": 200
}
```

## Configuration Validation

Readur validates configuration on startup:

```bash
# Test configuration
readur --config-test

# Validate specific file
readur --config-file readur.yml --validate

# Show effective configuration
readur --show-config
```

Common validation errors:

**Invalid database URL:** The database connection string format is incorrect or malformed.
   ```
   Error: Invalid DATABASE_URL format
   Expected: postgresql://user:pass@host:port/database
   ```

2. **Missing required S3 credentials**
   ```
   Error: S3_ENABLED=true but S3_ACCESS_KEY_ID not set
   ```

3. **Path conflicts**
   ```
   Error: UPLOAD_PATH and WATCH_FOLDER cannot be the same directory
   ```

## Best Practices

### Security

**Never commit secrets:** Always use environment variables or secret management systems to protect sensitive information.
   ```bash
   # Use environment variables
   JWT_SECRET=${JWT_SECRET}
   
   # Or use secret management
   JWT_SECRET=$(vault kv get -field=jwt_secret secret/readur)
   ```

**Use strong secrets:** Generate cryptographically secure secrets with sufficient entropy.
   ```bash
   # Generate secure secrets
   openssl rand -hex 32
   ```

**Rotate secrets regularly:** Implement a schedule for rotating sensitive credentials.
   ```bash
   # Quarterly rotation
   0 0 1 */3 * /scripts/rotate-secrets.sh
   ```

### Performance

**Tune database connections:** Configure the optimal number of database connections based on your system's resources.
   ```bash
   # Formula: connections = (worker_threads * 2) + management_connections
   DATABASE_MAX_CONNECTIONS=$(($(nproc) * 2 + 5))
   ```

**Optimize OCR workers:** Set the appropriate number of concurrent OCR workers to balance performance and resource usage.
   ```bash
   # Formula: ocr_workers = cpu_cores / 2
   CONCURRENT_OCR_JOBS=$(($(nproc) / 2))
   ```

**Configure caching:** Set up appropriate cache sizes to improve response times while managing memory usage.
   ```bash
   # Cache size based on available memory
   CACHE_SIZE_MB=$(($(free -m | awk 'NR==2{print $7}') / 4))
   ```

### Monitoring

**Enable metrics in production:** Turn on metrics collection to monitor system performance and health.
   ```bash
   METRICS_ENABLED=true
   PROMETHEUS_ENABLED=true
   ```

**Set appropriate log levels:** Configure logging verbosity based on your environment and debugging needs.
   ```bash
   # Production
   LOG_LEVEL=warn
   
   # Debugging
   LOG_LEVEL=debug
   ```

**Configure alerts:** Set up alerting to be notified of critical system events.
   ```bash
   WEBHOOK_URL=https://alerts.example.com/readur
   ```

## Troubleshooting Configuration

### Debug Configuration Loading

```bash
# Enable verbose configuration logging
RUST_LOG=readur::config=debug readur

# Show configuration sources
readur --config-sources
```

### Common Issues

1. **Environment variable not loading**
   - Check variable name (must match exactly)
   - Verify no spaces around `=`
   - Check for quotes in values

2. **Configuration file ignored**
   - Verify file path
   - Check YAML syntax
   - Ensure proper permissions

3. **Settings not taking effect**
   - Check configuration precedence
   - Verify no overrides
   - Some settings require restart

## Migration from Previous Versions

### From v1.x to v2.x

```bash
# Migration script
#!/bin/bash

# Update environment variables
sed -i 's/STORAGE_PATH/UPLOAD_PATH/g' .env
sed -i 's/OCR_WORKERS/CONCURRENT_OCR_JOBS/g' .env

# Add new required variables
echo "S3_ENABLED=false" >> .env
echo "ENABLE_PER_USER_WATCH=false" >> .env
```

### From v2.x to v3.x

```bash
# New variables in v3.x
echo "OIDC_ENABLED=false" >> .env
echo "FEATURE_MULTI_LANGUAGE_OCR=true" >> .env
```