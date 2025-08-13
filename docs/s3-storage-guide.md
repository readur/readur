# S3 Storage Backend Guide for Readur

## Overview

Starting with version 2.5.4, Readur supports Amazon S3 and S3-compatible storage services as an alternative to local filesystem storage. This implementation provides full support for AWS S3, MinIO, Wasabi, Backblaze B2, and other S3-compatible services with automatic multipart upload for files larger than 100MB, structured storage paths with year/month organization, and automatic retry mechanisms with exponential backoff.

This guide provides comprehensive instructions for configuring, deploying, and managing Readur with S3 storage.

### Key Benefits

- **Scalability**: Unlimited storage capacity without local disk constraints
- **Durability**: 99.999999999% (11 9's) durability with AWS S3
- **Cost-Effective**: Pay only for what you use with various storage tiers
- **Global Access**: Access documents from anywhere with proper credentials
- **Backup**: Built-in versioning and cross-region replication capabilities

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Configuration](#configuration)
3. [Migration from Local Storage](#migration-from-local-storage)
4. [Storage Structure](#storage-structure)
5. [Performance Optimization](#performance-optimization)
6. [Troubleshooting](#troubleshooting)
7. [Best Practices](#best-practices)

## Prerequisites

Before configuring S3 storage, ensure you have:

1. **S3 Bucket Access**
   - An AWS S3 bucket or S3-compatible service (MinIO, Wasabi, Backblaze B2, etc.)
   - Access Key ID and Secret Access Key with appropriate permissions
   - Bucket name and region information

2. **Required S3 Permissions**
   ```json
   {
     "Version": "2012-10-17",
     "Statement": [
       {
         "Effect": "Allow",
         "Action": [
           "s3:PutObject",
           "s3:GetObject",
           "s3:DeleteObject",
           "s3:ListBucket",
           "s3:HeadObject",
           "s3:HeadBucket",
           "s3:AbortMultipartUpload",
           "s3:CreateMultipartUpload",
           "s3:UploadPart",
           "s3:CompleteMultipartUpload"
         ],
         "Resource": [
           "arn:aws:s3:::your-bucket-name/*",
           "arn:aws:s3:::your-bucket-name"
         ]
       }
     ]
   }
   ```

3. **Readur Build Requirements**
   - Readur must be compiled with the `s3` feature flag enabled
   - Build command: `cargo build --release --features s3`

## Configuration

### Environment Variables

Configure S3 storage by setting the following environment variables:

```bash
# Enable S3 storage backend
S3_ENABLED=true

# Required S3 credentials
S3_BUCKET_NAME=readur-documents
S3_ACCESS_KEY_ID=your-access-key-id
S3_SECRET_ACCESS_KEY=your-secret-access-key
S3_REGION=us-east-1

# Optional: For S3-compatible services (MinIO, Wasabi, etc.)
S3_ENDPOINT=https://s3-compatible-endpoint.com
```

### Configuration File Example (.env)

```bash
# Database Configuration
DATABASE_URL=postgresql://readur:password@localhost/readur

# Server Configuration
SERVER_ADDRESS=0.0.0.0:8000
JWT_SECRET=your-secure-jwt-secret

# S3 Storage Configuration
S3_ENABLED=true
S3_BUCKET_NAME=readur-production
S3_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE
S3_SECRET_ACCESS_KEY=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY
S3_REGION=us-west-2

# Optional S3 endpoint for compatible services
# S3_ENDPOINT=https://minio.example.com

# Upload Configuration
UPLOAD_PATH=./temp_uploads
MAX_FILE_SIZE_MB=500
```

### S3-Compatible Services Configuration

#### MinIO
```bash
S3_ENABLED=true
S3_BUCKET_NAME=readur-bucket
S3_ACCESS_KEY_ID=minioadmin
S3_SECRET_ACCESS_KEY=minioadmin
S3_REGION=us-east-1
S3_ENDPOINT=http://localhost:9000
```

#### Wasabi
```bash
S3_ENABLED=true
S3_BUCKET_NAME=readur-bucket
S3_ACCESS_KEY_ID=your-wasabi-key
S3_SECRET_ACCESS_KEY=your-wasabi-secret
S3_REGION=us-east-1
S3_ENDPOINT=https://s3.wasabisys.com
```

#### Backblaze B2
```bash
S3_ENABLED=true
S3_BUCKET_NAME=readur-bucket
S3_ACCESS_KEY_ID=your-b2-key-id
S3_SECRET_ACCESS_KEY=your-b2-application-key
S3_REGION=us-west-002
S3_ENDPOINT=https://s3.us-west-002.backblazeb2.com
```

## Migration from Local Storage

### Using the Migration Tool

Readur includes a migration utility to transfer existing local files to S3:

1. **Prepare for Migration**
   ```bash
   # Backup your database first
   pg_dump readur > readur_backup.sql
   
   # Set S3 configuration
   export S3_ENABLED=true
   export S3_BUCKET_NAME=readur-production
   export S3_ACCESS_KEY_ID=your-key
   export S3_SECRET_ACCESS_KEY=your-secret
   export S3_REGION=us-east-1
   ```

2. **Run Dry Run First**
   ```bash
   # Preview what will be migrated
   cargo run --bin migrate_to_s3 --features s3 -- --dry-run
   ```

3. **Execute Migration**
   ```bash
   # Migrate all files
   cargo run --bin migrate_to_s3 --features s3
   
   # Migrate with options
   cargo run --bin migrate_to_s3 --features s3 -- \
     --delete-local \           # Delete local files after successful upload
     --limit 100 \              # Limit to 100 files (for testing)
     --enable-rollback          # Enable automatic rollback on failure
   ```

4. **Migrate Specific User's Files**
   ```bash
   cargo run --bin migrate_to_s3 --features s3 -- \
     --user-id 550e8400-e29b-41d4-a716-446655440000
   ```

5. **Resume Failed Migration**
   ```bash
   # Resume from specific document ID
   cargo run --bin migrate_to_s3 --features s3 -- \
     --resume-from 550e8400-e29b-41d4-a716-446655440001
   ```

### Migration Process Details

The migration tool performs the following steps:

1. Connects to database and S3
2. Identifies all documents with local file paths
3. For each document:
   - Reads the local file
   - Uploads to S3 with structured path
   - Updates database with S3 path
   - Migrates associated thumbnails and processed images
   - Optionally deletes local files
4. Tracks migration state for recovery
5. Supports rollback on failure

### Post-Migration Verification

```sql
-- Check migrated documents
SELECT 
    COUNT(*) FILTER (WHERE file_path LIKE 's3://%') as s3_documents,
    COUNT(*) FILTER (WHERE file_path NOT LIKE 's3://%') as local_documents
FROM documents;

-- Find any remaining local files
SELECT id, filename, file_path 
FROM documents 
WHERE file_path NOT LIKE 's3://%'
LIMIT 10;
```

## Storage Structure

### S3 Path Organization

Readur uses a structured path format in S3:

```
bucket-name/
├── documents/
│   └── {user_id}/
│       └── {year}/
│           └── {month}/
│               └── {document_id}.{extension}
├── thumbnails/
│   └── {user_id}/
│       └── {document_id}_thumb.jpg
└── processed_images/
    └── {user_id}/
        └── {document_id}_processed.png
```

### Example Paths

```
readur-production/
├── documents/
│   └── 550e8400-e29b-41d4-a716-446655440000/
│       └── 2024/
│           └── 03/
│               ├── 123e4567-e89b-12d3-a456-426614174000.pdf
│               └── 987fcdeb-51a2-43f1-b321-123456789abc.docx
├── thumbnails/
│   └── 550e8400-e29b-41d4-a716-446655440000/
│       ├── 123e4567-e89b-12d3-a456-426614174000_thumb.jpg
│       └── 987fcdeb-51a2-43f1-b321-123456789abc_thumb.jpg
└── processed_images/
    └── 550e8400-e29b-41d4-a716-446655440000/
        ├── 123e4567-e89b-12d3-a456-426614174000_processed.png
        └── 987fcdeb-51a2-43f1-b321-123456789abc_processed.png
```

## Performance Optimization

### Multipart Upload

Readur automatically uses multipart upload for files larger than 100MB:

- **Chunk Size**: 16MB per part
- **Automatic Retry**: Exponential backoff with up to 3 retries
- **Progress Tracking**: Real-time upload progress via WebSocket

### Network Optimization

1. **Region Selection**: Choose S3 region closest to your Readur server
2. **Transfer Acceleration**: Enable S3 Transfer Acceleration for global users
3. **CloudFront CDN**: Use CloudFront for serving frequently accessed documents

### Caching Strategy

```nginx
# Nginx caching configuration for S3-backed documents
location /api/documents/ {
    proxy_cache_valid 200 1h;
    proxy_cache_valid 404 1m;
    proxy_cache_bypass $http_authorization;
    add_header X-Cache-Status $upstream_cache_status;
}
```

## Troubleshooting

### Common Issues and Solutions

#### 1. S3 Connection Errors

**Error**: "Failed to access S3 bucket"

**Solution**:
```bash
# Verify credentials
aws s3 ls s3://your-bucket-name --profile readur

# Check IAM permissions
aws iam get-user-policy --user-name readur-user --policy-name ReadurS3Policy

# Test connectivity
curl -I https://s3.amazonaws.com/your-bucket-name
```

#### 2. Upload Failures

**Error**: "Failed to store file: RequestTimeout"

**Solution**:
- Check network connectivity
- Verify S3 endpoint configuration
- Increase timeout values if using S3-compatible service
- Monitor S3 request metrics in AWS CloudWatch

#### 3. Permission Denied

**Error**: "AccessDenied: Access Denied"

**Solution**:
```bash
# Verify bucket policy
aws s3api get-bucket-policy --bucket your-bucket-name

# Check object ACLs
aws s3api get-object-acl --bucket your-bucket-name --key test-object

# Ensure CORS configuration for web access
aws s3api put-bucket-cors --bucket your-bucket-name --cors-configuration file://cors.json
```

#### 4. Migration Stuck

**Problem**: Migration process hangs or fails repeatedly

**Solution**:
```bash
# Check migration state
cat migration_state.json | jq '.failed_migrations'

# Resume from last successful migration
LAST_SUCCESS=$(cat migration_state.json | jq -r '.completed_migrations[-1].document_id')
cargo run --bin migrate_to_s3 --features s3 -- --resume-from $LAST_SUCCESS

# Force rollback if needed
cargo run --bin migrate_to_s3 --features s3 -- --rollback
```

### Debugging S3 Operations

Enable detailed S3 logging:

```bash
# Set environment variables for debugging
export RUST_LOG=readur=debug,aws_sdk_s3=debug
export AWS_SDK_LOAD_CONFIG=true

# Run Readur with debug logging
cargo run --features s3
```

### Performance Monitoring

Monitor S3 performance metrics:

```sql
-- Query document upload times
SELECT 
    DATE(created_at) as upload_date,
    AVG(file_size / 1024.0 / 1024.0) as avg_size_mb,
    COUNT(*) as documents_uploaded,
    AVG(EXTRACT(EPOCH FROM (updated_at - created_at))) as avg_processing_time_seconds
FROM documents
WHERE file_path LIKE 's3://%'
GROUP BY DATE(created_at)
ORDER BY upload_date DESC;
```

## Best Practices

### 1. Security

- **Encryption**: Enable S3 server-side encryption (SSE-S3 or SSE-KMS)
- **Access Control**: Use IAM roles instead of access keys when possible
- **Bucket Policies**: Implement least-privilege bucket policies
- **VPC Endpoints**: Use VPC endpoints for private S3 access

```bash
# Enable default encryption on bucket
aws s3api put-bucket-encryption \
    --bucket readur-production \
    --server-side-encryption-configuration '{
        "Rules": [{
            "ApplyServerSideEncryptionByDefault": {
                "SSEAlgorithm": "AES256"
            }
        }]
    }'
```

### 2. Cost Optimization

- **Lifecycle Policies**: Archive old documents to Glacier
- **Intelligent-Tiering**: Enable for automatic cost optimization
- **Request Metrics**: Monitor and optimize S3 request patterns

```json
{
  "Rules": [{
    "Id": "ArchiveOldDocuments",
    "Status": "Enabled",
    "Transitions": [{
      "Days": 90,
      "StorageClass": "GLACIER"
    }],
    "NoncurrentVersionTransitions": [{
      "NoncurrentDays": 30,
      "StorageClass": "GLACIER"
    }]
  }]
}
```

### 3. Reliability

- **Versioning**: Enable S3 versioning for document recovery
- **Cross-Region Replication**: Set up for disaster recovery
- **Backup Strategy**: Regular backups to separate bucket or region

```bash
# Enable versioning
aws s3api put-bucket-versioning \
    --bucket readur-production \
    --versioning-configuration Status=Enabled

# Set up replication
aws s3api put-bucket-replication \
    --bucket readur-production \
    --replication-configuration file://replication.json
```

### 4. Monitoring

Set up CloudWatch alarms for:
- High error rates
- Unusual request patterns
- Storage quota approaching
- Failed multipart uploads

```bash
# Create CloudWatch alarm for S3 errors
aws cloudwatch put-metric-alarm \
    --alarm-name readur-s3-errors \
    --alarm-description "Alert on S3 4xx errors" \
    --metric-name 4xxErrors \
    --namespace AWS/S3 \
    --statistic Sum \
    --period 300 \
    --threshold 10 \
    --comparison-operator GreaterThanThreshold
```

### 5. Compliance

- **Data Residency**: Ensure S3 region meets data residency requirements
- **Audit Logging**: Enable S3 access logging and AWS CloudTrail
- **Retention Policies**: Implement compliant data retention policies
- **GDPR Compliance**: Implement proper data deletion procedures

```bash
# Enable access logging
aws s3api put-bucket-logging \
    --bucket readur-production \
    --bucket-logging-status '{
        "LoggingEnabled": {
            "TargetBucket": "readur-logs",
            "TargetPrefix": "s3-access/"
        }
    }'
```

## Next Steps

- Review the [Configuration Reference](./configuration-reference.md) for all S3 options
- Explore [S3 Troubleshooting Guide](./s3-troubleshooting.md) for common issues and solutions
- Check [Migration Guide](./migration-guide.md) for moving from local to S3 storage
- Read [Deployment Guide](./deployment.md) for production deployment best practices