# Storage Configuration Guide

## Overview

Readur supports multiple storage backends for document management. Choose the backend that best fits your infrastructure and scaling needs.

## Storage Backends

### Local Storage

Best for single-server deployments and small installations.

#### Configuration

```bash
# In .env file
STORAGE_BACKEND=local
LOCAL_STORAGE_PATH=/data/readur/documents
LOCAL_STORAGE_MAX_SIZE_GB=500  # Optional: limit storage usage
```

#### Directory Structure

```
/data/readur/documents/
├── users/
│   ├── user1/
│   │   ├── uploads/
│   │   └── processed/
│   └── user2/
├── temp/
└── cache/
```

#### Permissions

Set proper ownership and permissions:

```bash
# Create directory structure
sudo mkdir -p /data/readur/documents
sudo chown -R readur:readur /data/readur/documents
sudo chmod 750 /data/readur/documents
```

#### Backup Considerations

- Use filesystem snapshots (LVM, ZFS, Btrfs)
- Rsync to backup location
- Exclude temp/ and cache/ directories

### S3-Compatible Storage

Recommended for production deployments requiring scalability.

#### AWS S3 Configuration

```bash
# In .env file
STORAGE_BACKEND=s3
S3_BUCKET=readur-documents
S3_REGION=us-east-1
S3_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE
S3_SECRET_ACCESS_KEY=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY

# Optional settings
S3_ENDPOINT=  # Leave empty for AWS
S3_USE_SSL=true
S3_VERIFY_SSL=true
S3_SIGNATURE_VERSION=s3v4
```

#### MinIO Configuration

For self-hosted S3-compatible storage:

```bash
STORAGE_BACKEND=s3
S3_BUCKET=readur
S3_ENDPOINT=https://minio.company.com:9000
S3_ACCESS_KEY_ID=minioadmin
S3_SECRET_ACCESS_KEY=minioadmin123
S3_USE_SSL=true
S3_VERIFY_SSL=false  # For self-signed certificates
S3_REGION=us-east-1  # MinIO default
```

#### Bucket Setup

Create and configure your S3 bucket:

```bash
# AWS CLI
aws s3api create-bucket --bucket readur-documents --region us-east-1

# Set lifecycle policy for temp files
aws s3api put-bucket-lifecycle-configuration \
  --bucket readur-documents \
  --lifecycle-configuration file://lifecycle.json
```

Lifecycle policy (lifecycle.json):
```json
{
  "Rules": [
    {
      "Id": "DeleteTempFiles",
      "Status": "Enabled",
      "Prefix": "temp/",
      "Expiration": {
        "Days": 7
      }
    }
  ]
}
```

#### IAM Permissions

Minimum required S3 permissions:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "s3:GetObject",
        "s3:PutObject",
        "s3:DeleteObject",
        "s3:ListBucket",
        "s3:GetBucketLocation"
      ],
      "Resource": [
        "arn:aws:s3:::readur-documents",
        "arn:aws:s3:::readur-documents/*"
      ]
    }
  ]
}
```

### WebDAV Storage

For integration with existing WebDAV servers.

#### Configuration

```bash
STORAGE_BACKEND=webdav
WEBDAV_URL=https://webdav.company.com/readur
WEBDAV_USERNAME=readur_user
WEBDAV_PASSWORD=secure_password
WEBDAV_VERIFY_SSL=true
```

#### Nextcloud Integration

```bash
STORAGE_BACKEND=webdav
WEBDAV_URL=https://nextcloud.company.com/remote.php/dav/files/readur/
WEBDAV_USERNAME=readur
WEBDAV_PASSWORD=app-password-here
```

## Storage Migration

### Migrating Between Backends

Use the built-in Rust migration tool:

```bash
# Migrate from local to S3
docker-compose exec readur cargo run --bin migrate_to_s3 -- \
  --batch-size 100 \
  --enable-rollback \
  --verbose

# Or using the compiled binary in production
docker-compose exec readur /app/migrate_to_s3 \
  --batch-size 100 \
  --enable-rollback
```

### Progressive Migration

Migrate in stages to minimize downtime:

```bash
# Stage 1: Test migration with dry run
docker-compose exec readur cargo run --bin migrate_to_s3 -- --dry-run

# Stage 2: Migrate specific user's documents
docker-compose exec readur cargo run --bin migrate_to_s3 -- \
  --user-id "user-uuid" \
  --enable-rollback

# Stage 3: Full migration with rollback capability
docker-compose exec readur cargo run --bin migrate_to_s3 -- \
  --enable-rollback \
  --batch-size 500 \
  --parallel-uploads 5

# Stage 4: Update configuration
# Update .env: STORAGE_BACKEND=s3
docker-compose restart readur
```

## Performance Optimization

### Local Storage

#### Filesystem Choice

- **ext4**: Good general performance
- **XFS**: Better for large files
- **ZFS**: Advanced features, snapshots, compression
- **Btrfs**: Copy-on-write, snapshots

#### Mount Options

```bash
# /etc/fstab optimization
/dev/sdb1 /data/readur ext4 defaults,noatime,nodiratime 0 2
```

#### RAID Configuration

For better performance and redundancy:

```bash
# RAID 10 for balanced performance/redundancy
mdadm --create /dev/md0 --level=10 --raid-devices=4 \
  /dev/sdb /dev/sdc /dev/sdd /dev/sde
```

### S3 Storage

#### Connection Pooling

```bash
# Optimize S3 connections
S3_MAX_CONNECTIONS=100
S3_CONNECTION_TIMEOUT=30
S3_READ_TIMEOUT=300
```

#### Transfer Acceleration

```bash
# Enable for AWS S3
S3_USE_ACCELERATE_ENDPOINT=true
S3_MULTIPART_THRESHOLD=64MB
S3_MULTIPART_CHUNKSIZE=16MB
```

#### CDN Integration

Use CloudFront for read-heavy workloads:

```bash
# Serve documents through CDN
CDN_ENABLED=true
CDN_URL=https://d1234567890.cloudfront.net
CDN_PRIVATE_KEY=/etc/readur/cloudfront-key.pem
```

## Storage Monitoring

### Disk Usage Monitoring

```bash
# Check local storage usage
df -h /data/readur/documents

# Monitor growth rate
du -sh /data/readur/documents/* | sort -rh

# Set up alerts
cat > /etc/cron.d/storage-alert << EOF
0 * * * * root df /data/readur | awk '\$5+0 > 80' && mail -s "Storage Alert" admin@company.com
EOF
```

### S3 Metrics

Monitor S3 usage and costs:

```bash
# Get bucket size
aws s3 ls s3://readur-documents --recursive --summarize | grep "Total Size"

# CloudWatch metrics
aws cloudwatch get-metric-statistics \
  --namespace AWS/S3 \
  --metric-name BucketSizeBytes \
  --dimensions Name=BucketName,Value=readur-documents \
  --start-time 2024-01-01T00:00:00Z \
  --end-time 2024-01-31T23:59:59Z \
  --period 86400 \
  --statistics Average
```

## Troubleshooting

### Local Storage Issues

#### Disk Full

```bash
# Find large files
find /data/readur -type f -size +100M -exec ls -lh {} \;

# Clean temporary files
find /data/readur/documents/temp -mtime +7 -delete

# Check for orphaned files
# This is typically done via database queries or custom scripts
docker-compose exec readur psql -U readur -d readur -c \
  "SELECT * FROM documents WHERE file_path NOT IN (SELECT path FROM files)"
```

#### Permission Errors

```bash
# Fix ownership
sudo chown -R readur:readur /data/readur/documents

# Fix permissions
find /data/readur/documents -type d -exec chmod 755 {} \;
find /data/readur/documents -type f -exec chmod 644 {} \;
```

### S3 Issues

#### Connection Errors

```bash
# Test S3 connectivity
aws s3 ls s3://readur-documents --debug

# Check credentials
aws sts get-caller-identity

# Verify S3 environment variables are set correctly
# Should use S3_ACCESS_KEY_ID and S3_SECRET_ACCESS_KEY
docker-compose exec readur env | grep -E '^S3_' | sed 's/=.*/=***/'

# Verify bucket policy
aws s3api get-bucket-policy --bucket readur-documents
```

#### Slow Uploads

```bash
# Increase multipart settings
S3_MULTIPART_THRESHOLD=32MB
S3_MULTIPART_CHUNKSIZE=8MB
S3_MAX_CONCURRENCY=10

# Enable transfer acceleration (AWS only)
aws s3api put-bucket-accelerate-configuration \
  --bucket readur-documents \
  --accelerate-configuration Status=Enabled
```

## Best Practices

### Security

1. **Encryption at rest**:
   - Local: Use encrypted filesystems (LUKS)
   - S3: Enable SSE-S3 or SSE-KMS

2. **Encryption in transit**:
   - Always use HTTPS/TLS
   - Verify SSL certificates

3. **Access control**:
   - Principle of least privilege
   - Regular credential rotation
   - IP whitelisting where possible

### Backup Strategy

1. **3-2-1 Rule**:
   - 3 copies of data
   - 2 different storage types
   - 1 offsite backup

2. **Testing**:
   - Regular restore tests
   - Document recovery procedures
   - Monitor backup completion

3. **Retention**:
   - Define retention policies
   - Automate old backup cleanup
   - Comply with regulations

### Capacity Planning

1. **Growth estimation**:
   ```
   Daily documents × Average size × Retention days = Required storage
   ```

2. **Buffer space**:
   - Keep 20% free space minimum
   - Monitor growth trends
   - Plan upgrades proactively

3. **Cost optimization**:
   - Use lifecycle policies
   - Archive old documents
   - Compress where appropriate

## Related Documentation

- [S3 Storage Guide](../s3-storage-guide.md)
- [Storage Migration](../administration/storage-migration.md)
- [Backup Strategies](./backup.md)
- [Performance Tuning](./performance.md)