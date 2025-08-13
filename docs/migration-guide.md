# Migration Guide: Local Storage to S3

## Overview

This guide provides step-by-step instructions for migrating your Readur installation from local filesystem storage to S3 storage. The migration process is designed to be safe, resumable, and reversible.

## Pre-Migration Checklist

### 1. System Requirements

- [ ] Readur compiled with S3 feature: `cargo build --release --features s3`
- [ ] Sufficient disk space for temporary operations (at least 2x largest file)
- [ ] Network bandwidth for uploading all documents to S3
- [ ] AWS CLI installed and configured (for verification)

### 2. S3 Prerequisites

- [ ] S3 bucket created and accessible
- [ ] IAM user with appropriate permissions
- [ ] Access keys generated and tested
- [ ] Bucket region identified
- [ ] Encryption settings configured (if required)
- [ ] Lifecycle policies reviewed

### 3. Backup Requirements

- [ ] Database backed up
- [ ] Local files backed up (optional but recommended)
- [ ] Configuration files saved
- [ ] Document count and total size recorded

## Migration Process

### Step 1: Prepare Environment

#### 1.1 Backup Database

```bash
# Create timestamped backup
BACKUP_DATE=$(date +%Y%m%d_%H%M%S)
pg_dump $DATABASE_URL > readur_backup_${BACKUP_DATE}.sql

# Verify backup
pg_restore --list readur_backup_${BACKUP_DATE}.sql | head -20
```

#### 1.2 Document Current State

```sql
-- Record current statistics
SELECT 
    COUNT(*) as total_documents,
    SUM(file_size) / 1024.0 / 1024.0 / 1024.0 as total_size_gb,
    COUNT(DISTINCT user_id) as unique_users
FROM documents;

-- Save document list
\copy (SELECT id, filename, file_path, file_size FROM documents) TO 'documents_pre_migration.csv' CSV HEADER;
```

#### 1.3 Calculate Migration Time

```bash
# Estimate migration duration
TOTAL_SIZE_GB=100  # From query above
UPLOAD_SPEED_MBPS=100  # Your upload speed
ESTIMATED_HOURS=$(echo "scale=2; ($TOTAL_SIZE_GB * 1024 * 8) / ($UPLOAD_SPEED_MBPS * 3600)" | bc)
echo "Estimated migration time: $ESTIMATED_HOURS hours"
```

### Step 2: Configure S3

#### 2.1 Create S3 Bucket

```bash
# Create bucket
aws s3api create-bucket \
    --bucket readur-production \
    --region us-east-1 \
    --create-bucket-configuration LocationConstraint=us-east-1

# Enable versioning
aws s3api put-bucket-versioning \
    --bucket readur-production \
    --versioning-configuration Status=Enabled

# Enable encryption
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

#### 2.2 Set Up IAM User

```bash
# Create policy file
cat > readur-s3-policy.json << 'EOF'
{
    "Version": "2012-10-17",
    "Statement": [
        {
            "Effect": "Allow",
            "Action": [
                "s3:ListBucket",
                "s3:GetBucketLocation"
            ],
            "Resource": "arn:aws:s3:::readur-production"
        },
        {
            "Effect": "Allow",
            "Action": [
                "s3:GetObject",
                "s3:PutObject",
                "s3:DeleteObject",
                "s3:GetObjectVersion",
                "s3:PutObjectAcl"
            ],
            "Resource": "arn:aws:s3:::readur-production/*"
        }
    ]
}
EOF

# Create IAM user and attach policy
aws iam create-user --user-name readur-s3-user
aws iam put-user-policy \
    --user-name readur-s3-user \
    --policy-name ReadurS3Access \
    --policy-document file://readur-s3-policy.json

# Generate access keys
aws iam create-access-key --user-name readur-s3-user > s3-credentials.json
```

#### 2.3 Configure Readur for S3

```bash
# Add to .env file
cat >> .env << 'EOF'
# S3 Configuration
S3_ENABLED=true
S3_BUCKET_NAME=readur-production
S3_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE
S3_SECRET_ACCESS_KEY=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY
S3_REGION=us-east-1
EOF

# Test configuration
source .env
aws s3 ls s3://$S3_BUCKET_NAME --region $S3_REGION
```

### Step 3: Run Migration

#### 3.1 Dry Run

```bash
# Preview migration without making changes
cargo run --bin migrate_to_s3 --features s3 -- --dry-run

# Review output
# Expected output:
# 🔍 DRY RUN - Would migrate the following files:
#   - document1.pdf (User: 123e4567..., Size: 2.5 MB)
#   - report.docx (User: 987fcdeb..., Size: 1.2 MB)
# 💡 Run without --dry-run to perform actual migration
```

#### 3.2 Partial Migration (Testing)

```bash
# Migrate only 10 files first
cargo run --bin migrate_to_s3 --features s3 -- --limit 10

# Verify migrated files
aws s3 ls s3://$S3_BUCKET_NAME/documents/ --recursive | head -20

# Check database updates
psql $DATABASE_URL -c "SELECT id, filename, file_path FROM documents WHERE file_path LIKE 's3://%' LIMIT 10;"
```

#### 3.3 Full Migration

```bash
# Run full migration with progress tracking
cargo run --bin migrate_to_s3 --features s3 -- \
    --enable-rollback \
    2>&1 | tee migration_$(date +%Y%m%d_%H%M%S).log

# Monitor progress in another terminal
watch -n 5 'cat migration_state.json | jq "{processed: .processed_files, total: .total_files, failed: .failed_migrations | length}"'
```

#### 3.4 Migration with Local File Deletion

```bash
# Only after verifying successful migration
cargo run --bin migrate_to_s3 --features s3 -- \
    --delete-local \
    --enable-rollback
```

### Step 4: Verify Migration

#### 4.1 Database Verification

```sql
-- Check migration completeness
SELECT 
    COUNT(*) FILTER (WHERE file_path LIKE 's3://%') as s3_documents,
    COUNT(*) FILTER (WHERE file_path NOT LIKE 's3://%') as local_documents,
    COUNT(*) as total_documents
FROM documents;

-- Find any failed migrations
SELECT id, filename, file_path 
FROM documents 
WHERE file_path NOT LIKE 's3://%'
ORDER BY created_at DESC
LIMIT 20;

-- Verify path format
SELECT DISTINCT 
    substring(file_path from 1 for 50) as path_prefix,
    COUNT(*) as document_count
FROM documents
GROUP BY path_prefix
ORDER BY document_count DESC;
```

#### 4.2 S3 Verification

```bash
# Count objects in S3
aws s3 ls s3://$S3_BUCKET_NAME/documents/ --recursive --summarize | grep "Total Objects"

# Verify file structure
aws s3 ls s3://$S3_BUCKET_NAME/ --recursive | head -50

# Check specific document
DOCUMENT_ID="123e4567-e89b-12d3-a456-426614174000"
aws s3 ls s3://$S3_BUCKET_NAME/documents/ --recursive | grep $DOCUMENT_ID
```

#### 4.3 Application Testing

```bash
# Restart Readur with S3 configuration
systemctl restart readur

# Test document upload
curl -X POST https://readur.example.com/api/documents \
    -H "Authorization: Bearer $TOKEN" \
    -F "file=@test-document.pdf"

# Test document retrieval
curl -X GET https://readur.example.com/api/documents/$DOCUMENT_ID/download \
    -H "Authorization: Bearer $TOKEN" \
    -o downloaded-test.pdf

# Verify downloaded file
md5sum test-document.pdf downloaded-test.pdf
```

### Step 5: Post-Migration Tasks

#### 5.1 Update Backup Procedures

```bash
# Create S3 backup script
cat > backup-s3.sh << 'EOF'
#!/bin/bash
# Backup S3 data to another bucket
BACKUP_BUCKET="readur-backup-$(date +%Y%m%d)"
aws s3api create-bucket --bucket $BACKUP_BUCKET --region us-east-1
aws s3 sync s3://readur-production s3://$BACKUP_BUCKET --storage-class GLACIER
EOF

chmod +x backup-s3.sh
```

#### 5.2 Set Up Monitoring

```bash
# Create CloudWatch dashboard
aws cloudwatch put-dashboard \
    --dashboard-name ReadurS3 \
    --dashboard-body file://cloudwatch-dashboard.json
```

#### 5.3 Clean Up Local Storage

```bash
# After confirming successful migration
# Remove old upload directories (CAREFUL!)
du -sh ./uploads ./thumbnails ./processed_images

# Archive before deletion
tar -czf pre_migration_files_$(date +%Y%m%d).tar.gz ./uploads ./thumbnails ./processed_images

# Remove directories
rm -rf ./uploads/* ./thumbnails/* ./processed_images/*
```

## Rollback Procedures

### Automatic Rollback

If migration fails with `--enable-rollback`:

```bash
# Rollback will automatically:
# 1. Restore database paths to original values
# 2. Delete uploaded S3 objects
# 3. Save rollback state to rollback_errors.json
```

### Manual Rollback

#### Step 1: Restore Database

```sql
-- Revert file paths to local
UPDATE documents 
SET file_path = regexp_replace(file_path, '^s3://[^/]+/', './uploads/')
WHERE file_path LIKE 's3://%';

-- Or restore from backup
psql $DATABASE_URL < readur_backup_${BACKUP_DATE}.sql
```

#### Step 2: Remove S3 Objects

```bash
# Delete all migrated objects
aws s3 rm s3://$S3_BUCKET_NAME/documents/ --recursive
aws s3 rm s3://$S3_BUCKET_NAME/thumbnails/ --recursive
aws s3 rm s3://$S3_BUCKET_NAME/processed_images/ --recursive
```

#### Step 3: Restore Configuration

```bash
# Disable S3 in configuration
sed -i 's/S3_ENABLED=true/S3_ENABLED=false/' .env

# Restart application
systemctl restart readur
```

## Troubleshooting Migration Issues

### Issue: Migration Hangs

```bash
# Check current progress
tail -f migration_*.log

# View migration state
cat migration_state.json | jq '.processed_files, .failed_migrations'

# Resume from last successful
LAST_ID=$(cat migration_state.json | jq -r '.completed_migrations[-1].document_id')
cargo run --bin migrate_to_s3 --features s3 -- --resume-from $LAST_ID
```

### Issue: Permission Errors

```bash
# Verify IAM permissions
aws s3api put-object \
    --bucket $S3_BUCKET_NAME \
    --key test.txt \
    --body /tmp/test.txt

# Check bucket policy
aws s3api get-bucket-policy --bucket $S3_BUCKET_NAME
```

### Issue: Network Timeouts

```bash
# Use screen/tmux for long migrations
screen -S migration
cargo run --bin migrate_to_s3 --features s3

# Detach: Ctrl+A, D
# Reattach: screen -r migration
```

## Migration Optimization

### Parallel Upload

```bash
# Split migration by user
for USER_ID in $(psql $DATABASE_URL -t -c "SELECT DISTINCT user_id FROM documents"); do
    cargo run --bin migrate_to_s3 --features s3 -- --user-id $USER_ID &
done
```

### Bandwidth Management

```bash
# Limit upload bandwidth (if needed)
trickle -u 10240 cargo run --bin migrate_to_s3 --features s3
```

### Progress Monitoring

```bash
# Real-time statistics
watch -n 10 'echo "=== Migration Progress ===" && \
    cat migration_state.json | jq "{
        progress_pct: ((.processed_files / .total_files) * 100),
        processed: .processed_files,
        total: .total_files,
        failed: .failed_migrations | length,
        elapsed: now - (.started_at | fromdate),
        rate_per_hour: (.processed_files / ((now - (.started_at | fromdate)) / 3600))
    }"'
```

## Post-Migration Validation

### Data Integrity Check

```bash
# Generate checksums for S3 objects
aws s3api list-objects-v2 --bucket $S3_BUCKET_NAME --prefix documents/ \
    --query 'Contents[].{Key:Key, ETag:ETag}' \
    --output json > s3_checksums.json

# Compare with database
psql $DATABASE_URL -c "SELECT id, file_path, file_hash FROM documents" > db_checksums.txt
```

### Performance Testing

```bash
# Benchmark S3 retrieval
time for i in {1..100}; do
    curl -s https://readur.example.com/api/documents/random/download > /dev/null
done
```

## Success Criteria

Migration is considered successful when:

- [ ] All documents have S3 paths in database
- [ ] No failed migrations in migration_state.json
- [ ] Application can upload new documents to S3
- [ ] Application can retrieve existing documents from S3
- [ ] Thumbnails and processed images are accessible
- [ ] Performance meets acceptable thresholds
- [ ] Backup procedures are updated and tested

## Next Steps

1. Monitor S3 costs and usage
2. Implement CloudFront CDN if needed
3. Set up cross-region replication for disaster recovery
4. Configure S3 lifecycle policies for cost optimization
5. Update documentation and runbooks