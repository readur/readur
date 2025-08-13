# Configuration Reference

## Complete Configuration Options for Readur

This document provides a comprehensive reference for all configuration options available in Readur, including the new S3 storage backend and per-user watch directories introduced in version 2.5.4.

## Environment Variables

### Core Configuration

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `DATABASE_URL` | String | `postgresql://readur:readur@localhost/readur` | PostgreSQL connection string |
| `SERVER_ADDRESS` | String | `0.0.0.0:8000` | Server bind address (host:port) |
| `SERVER_HOST` | String | `0.0.0.0` | Server host (used if SERVER_ADDRESS not set) |
| `SERVER_PORT` | String | `8000` | Server port (used if SERVER_ADDRESS not set) |
| `JWT_SECRET` | String | `your-secret-key` | Secret key for JWT token generation (CHANGE IN PRODUCTION) |
| `UPLOAD_PATH` | String | `./uploads` | Local directory for temporary file uploads |
| `ALLOWED_FILE_TYPES` | String | `pdf,txt,doc,docx,png,jpg,jpeg` | Comma-separated list of allowed file extensions |

### S3 Storage Configuration

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `S3_ENABLED` | Boolean | `false` | Enable S3 storage backend |
| `S3_BUCKET_NAME` | String | - | S3 bucket name (required when S3_ENABLED=true) |
| `S3_ACCESS_KEY_ID` | String | - | AWS Access Key ID (required when S3_ENABLED=true) |
| `S3_SECRET_ACCESS_KEY` | String | - | AWS Secret Access Key (required when S3_ENABLED=true) |
| `S3_REGION` | String | `us-east-1` | AWS region for S3 bucket |
| `S3_ENDPOINT` | String | - | Custom S3 endpoint URL (for S3-compatible services) |

### Watch Directory Configuration

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `WATCH_FOLDER` | String | `./watch` | Global watch directory for file ingestion |
| `USER_WATCH_BASE_DIR` | String | `./user_watch` | Base directory for per-user watch folders |
| `ENABLE_PER_USER_WATCH` | Boolean | `false` | Enable per-user watch directories feature |
| `WATCH_INTERVAL_SECONDS` | Integer | `60` | Interval between watch folder scans |
| `FILE_STABILITY_CHECK_MS` | Integer | `2000` | Time to wait for file size stability |
| `MAX_FILE_AGE_HOURS` | Integer | `24` | Maximum age of files to process |

### OCR Configuration

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `OCR_LANGUAGE` | String | `eng` | Tesseract language code for OCR |
| `CONCURRENT_OCR_JOBS` | Integer | `4` | Number of concurrent OCR jobs |
| `OCR_TIMEOUT_SECONDS` | Integer | `300` | Timeout for OCR processing per document |
| `MAX_FILE_SIZE_MB` | Integer | `50` | Maximum file size for processing |

### Performance Configuration

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `MEMORY_LIMIT_MB` | Integer | `512` | Memory limit for processing operations |
| `CPU_PRIORITY` | String | `normal` | CPU priority (low, normal, high) |

### OIDC Authentication Configuration

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `OIDC_ENABLED` | Boolean | `false` | Enable OpenID Connect authentication |
| `OIDC_CLIENT_ID` | String | - | OIDC client ID |
| `OIDC_CLIENT_SECRET` | String | - | OIDC client secret |
| `OIDC_ISSUER_URL` | String | - | OIDC issuer URL |
| `OIDC_REDIRECT_URI` | String | - | OIDC redirect URI |

## Configuration Examples

### Basic Local Storage Setup

```bash
# .env file for local storage
DATABASE_URL=postgresql://readur:password@localhost/readur
SERVER_ADDRESS=0.0.0.0:8000
JWT_SECRET=your-secure-secret-key-change-this
UPLOAD_PATH=./uploads
WATCH_FOLDER=./watch
ALLOWED_FILE_TYPES=pdf,txt,doc,docx,png,jpg,jpeg,tiff,bmp
OCR_LANGUAGE=eng
CONCURRENT_OCR_JOBS=4
```

### S3 Storage with AWS

```bash
# .env file for AWS S3
DATABASE_URL=postgresql://readur:password@localhost/readur
SERVER_ADDRESS=0.0.0.0:8000
JWT_SECRET=your-secure-secret-key-change-this

# S3 Configuration
S3_ENABLED=true
S3_BUCKET_NAME=readur-production
S3_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE
S3_SECRET_ACCESS_KEY=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY
S3_REGION=us-west-2

# Still needed for temporary uploads
UPLOAD_PATH=./temp_uploads
```

### S3 with MinIO

```bash
# .env file for MinIO
DATABASE_URL=postgresql://readur:password@localhost/readur
SERVER_ADDRESS=0.0.0.0:8000
JWT_SECRET=your-secure-secret-key-change-this

# MinIO S3 Configuration
S3_ENABLED=true
S3_BUCKET_NAME=readur-bucket
S3_ACCESS_KEY_ID=minioadmin
S3_SECRET_ACCESS_KEY=minioadmin
S3_REGION=us-east-1
S3_ENDPOINT=http://minio:9000

UPLOAD_PATH=./temp_uploads
```

### Per-User Watch Directories

```bash
# .env file with per-user watch enabled
DATABASE_URL=postgresql://readur:password@localhost/readur
SERVER_ADDRESS=0.0.0.0:8000
JWT_SECRET=your-secure-secret-key-change-this

# Watch Directory Configuration
WATCH_FOLDER=./global_watch
USER_WATCH_BASE_DIR=/data/user_watches
ENABLE_PER_USER_WATCH=true
WATCH_INTERVAL_SECONDS=30
FILE_STABILITY_CHECK_MS=3000
MAX_FILE_AGE_HOURS=48
```

### High-Performance Configuration

```bash
# .env file for high-performance setup
DATABASE_URL=postgresql://readur:password@db-server/readur
SERVER_ADDRESS=0.0.0.0:8000
JWT_SECRET=your-secure-secret-key-change-this

# S3 for scalable storage
S3_ENABLED=true
S3_BUCKET_NAME=readur-highperf
S3_ACCESS_KEY_ID=your-key
S3_SECRET_ACCESS_KEY=your-secret
S3_REGION=us-east-1

# Performance tuning
CONCURRENT_OCR_JOBS=8
OCR_TIMEOUT_SECONDS=600
MAX_FILE_SIZE_MB=200
MEMORY_LIMIT_MB=2048
CPU_PRIORITY=high

# Faster watch scanning
WATCH_INTERVAL_SECONDS=10
FILE_STABILITY_CHECK_MS=1000
```

### OIDC with S3 Storage

```bash
# .env file for OIDC authentication with S3
DATABASE_URL=postgresql://readur:password@localhost/readur
SERVER_ADDRESS=0.0.0.0:8000
JWT_SECRET=your-secure-secret-key-change-this

# OIDC Configuration
OIDC_ENABLED=true
OIDC_CLIENT_ID=readur-client
OIDC_CLIENT_SECRET=your-oidc-secret
OIDC_ISSUER_URL=https://auth.example.com
OIDC_REDIRECT_URI=https://readur.example.com/api/auth/oidc/callback

# S3 Storage
S3_ENABLED=true
S3_BUCKET_NAME=readur-oidc
S3_ACCESS_KEY_ID=your-key
S3_SECRET_ACCESS_KEY=your-secret
S3_REGION=eu-west-1
```

## Docker Configuration

### Docker Compose with Environment File

```yaml
version: '3.8'

services:
  readur:
    image: readur:latest
    env_file: .env
    ports:
      - "8000:8000"
    volumes:
      - ./uploads:/app/uploads
      - ./watch:/app/watch
      - ./user_watch:/app/user_watch
    depends_on:
      - postgres
      - minio

  postgres:
    image: postgres:15
    environment:
      POSTGRES_USER: readur
      POSTGRES_PASSWORD: password
      POSTGRES_DB: readur
    volumes:
      - postgres_data:/var/lib/postgresql/data

  minio:
    image: minio/minio:latest
    command: server /data --console-address ":9001"
    environment:
      MINIO_ROOT_USER: minioadmin
      MINIO_ROOT_PASSWORD: minioadmin
    ports:
      - "9000:9000"
      - "9001:9001"
    volumes:
      - minio_data:/data

volumes:
  postgres_data:
  minio_data:
```

### Kubernetes ConfigMap

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: readur-config
data:
  DATABASE_URL: "postgresql://readur:password@postgres-service/readur"
  SERVER_ADDRESS: "0.0.0.0:8000"
  S3_ENABLED: "true"
  S3_BUCKET_NAME: "readur-k8s"
  S3_REGION: "us-east-1"
  ENABLE_PER_USER_WATCH: "true"
  USER_WATCH_BASE_DIR: "/data/user_watches"
  CONCURRENT_OCR_JOBS: "6"
  MAX_FILE_SIZE_MB: "100"
```

## Configuration Validation

### Required Variables

When S3 is enabled, the following variables are required:
- `S3_BUCKET_NAME`
- `S3_ACCESS_KEY_ID`
- `S3_SECRET_ACCESS_KEY`

When OIDC is enabled, the following variables are required:
- `OIDC_CLIENT_ID`
- `OIDC_CLIENT_SECRET`
- `OIDC_ISSUER_URL`
- `OIDC_REDIRECT_URI`

### Validation Script

```bash
#!/bin/bash
# validate-config.sh

# Check required variables
check_var() {
    if [ -z "${!1}" ]; then
        echo "ERROR: $1 is not set"
        exit 1
    fi
}

# Load environment
source .env

# Always required
check_var DATABASE_URL
check_var JWT_SECRET

# Check S3 requirements
if [ "$S3_ENABLED" = "true" ]; then
    check_var S3_BUCKET_NAME
    check_var S3_ACCESS_KEY_ID
    check_var S3_SECRET_ACCESS_KEY
fi

# Check OIDC requirements
if [ "$OIDC_ENABLED" = "true" ]; then
    check_var OIDC_CLIENT_ID
    check_var OIDC_CLIENT_SECRET
    check_var OIDC_ISSUER_URL
    check_var OIDC_REDIRECT_URI
fi

echo "Configuration valid!"
```

## Migration from Previous Versions

### From 2.5.3 to 2.5.4

New configuration options in 2.5.4:

```bash
# New S3 storage options
S3_ENABLED=false
S3_BUCKET_NAME=
S3_ACCESS_KEY_ID=
S3_SECRET_ACCESS_KEY=
S3_REGION=us-east-1
S3_ENDPOINT=

# New per-user watch directories
USER_WATCH_BASE_DIR=./user_watch
ENABLE_PER_USER_WATCH=false
```

No changes required for existing installations unless you want to enable new features.

## Troubleshooting Configuration

### Common Issues

1. **S3 Connection Failed**
   - Verify S3_BUCKET_NAME exists
   - Check S3_ACCESS_KEY_ID and S3_SECRET_ACCESS_KEY are correct
   - Ensure S3_REGION matches bucket region
   - For S3-compatible services, verify S3_ENDPOINT is correct

2. **Per-User Watch Not Working**
   - Ensure ENABLE_PER_USER_WATCH=true
   - Verify USER_WATCH_BASE_DIR exists and is writable
   - Check directory permissions

3. **JWT Authentication Failed**
   - Ensure JWT_SECRET is consistent across restarts
   - Use a strong, unique secret in production

### Debug Mode

Enable debug logging:

```bash
export RUST_LOG=debug
export RUST_BACKTRACE=1
```

### Configuration Testing

Test S3 configuration:

```bash
aws s3 ls s3://$S3_BUCKET_NAME --profile readur-test
```

Test database connection:

```bash
psql $DATABASE_URL -c "SELECT version();"
```

## Security Considerations

1. **Never commit `.env` files to version control**
2. **Use strong, unique values for JWT_SECRET**
3. **Rotate S3 access keys regularly**
4. **Use IAM roles when running on AWS**
5. **Enable S3 bucket encryption**
6. **Restrict S3 bucket policies to minimum required permissions**
7. **Use HTTPS for S3_ENDPOINT when possible**
8. **Implement network security groups for database access**