# Storage Migration Guide

## Overview

Readur supports migrating documents between storage backends (Local ↔ S3) using a built-in migration tool. This enterprise-grade utility ensures safe, reliable data migration with comprehensive rollback capabilities.

## When You Need This

- **Moving from local filesystem to S3 cloud storage**
- **Switching between S3 buckets or regions**
- **Disaster recovery scenarios**
- **Infrastructure upgrades or server migrations**
- **Scaling to cloud-based storage**

## Migration Tool Features

✅ **Dry-run mode** - Test migration without making any changes  
✅ **Progress tracking** - Resume interrupted migrations from saved state  
✅ **Rollback capability** - Complete undo functionality if needed  
✅ **Batch processing** - Efficiently handle large datasets  
✅ **Associated files** - Automatically migrates thumbnails & processed images  
✅ **Data integrity** - Verifies successful uploads before cleanup  
✅ **Selective migration** - Migrate specific users or document sets  

## Prerequisites

### System Requirements
- Admin access to your Readur deployment
- Ability to run commands on the server (Docker exec or direct access)
- Sufficient disk space for temporary files during migration
- Network connectivity to target storage (S3)

### Before You Start
1. **Complete database backup**
   ```bash
   pg_dump readur > readur_backup_$(date +%Y%m%d).sql
   ```

2. **File system backup** (if migrating from local storage)
   ```bash
   tar -czf documents_backup_$(date +%Y%m%d).tar.gz /path/to/readur/uploads
   ```

3. **S3 credentials configured** (for S3 migrations)
   - Verify bucket access and permissions
   - Test connectivity with AWS CLI

## Step-by-Step Migration Process

### Step 1: Configure Target Storage

For S3 migrations, ensure environment variables are set:

```bash
# Required S3 configuration
export S3_BUCKET_NAME="your-readur-bucket"
export S3_ACCESS_KEY_ID="your-access-key"
export S3_SECRET_ACCESS_KEY="your-secret-key"
export S3_REGION="us-east-1"

# Optional: Custom endpoint for S3-compatible services
export S3_ENDPOINT="https://s3.amazonaws.com"
```

### Step 2: Test with Dry Run

**Always start with a dry run** to validate the migration plan:

```bash
# Docker deployment
docker exec readur-app cargo run --bin migrate_to_s3 -- --dry-run

# Direct deployment
./target/release/migrate_to_s3 --dry-run

# Dry run for specific user
docker exec readur-app cargo run --bin migrate_to_s3 -- --dry-run --user-id "uuid-here"
```

The dry run will show:
- Number of documents to migrate
- Estimated data transfer size
- Potential issues or conflicts
- Expected migration time

### Step 3: Run the Migration

Once dry run looks good, execute the actual migration:

```bash
# Full migration with rollback enabled (recommended)
docker exec readur-app cargo run --bin migrate_to_s3 -- --enable-rollback

# Migration with progress tracking
docker exec readur-app cargo run --bin migrate_to_s3 -- --enable-rollback --verbose

# User-specific migration
docker exec readur-app cargo run --bin migrate_to_s3 -- --enable-rollback --user-id "uuid-here"
```

### Step 4: Monitor Progress

The migration tool provides real-time progress updates:

```
📊 Migration Progress:
┌─────────────────────────────────────────────────────────────┐
│ Documents: 1,247 / 2,500 (49.9%)                          │
│ Data Transferred: 2.3 GB / 4.7 GB                         │
│ Time Elapsed: 00:15:32                                     │
│ ETA: 00:16:12                                              │
│ Current: uploading user_documents/report_2024.pdf         │
└─────────────────────────────────────────────────────────────┘
```

### Step 5: Verify Migration

After completion, verify the migration was successful:

```bash
# Check migration status
docker exec readur-app cargo run --bin migrate_to_s3 -- --status

# Verify document count matches
docker exec readur-app psql -d readur -c "SELECT COUNT(*) FROM documents;"

# Test document access through API
curl -H "Authorization: Bearer YOUR_TOKEN" \
     "https://your-readur-instance.com/api/documents/sample-uuid/download"
```

### Step 6: Update Configuration

Update your deployment configuration to use the new storage backend:

```yaml
# docker-compose.yml
environment:
  - STORAGE_BACKEND=s3
  - S3_BUCKET_NAME=your-readur-bucket
  - S3_ACCESS_KEY_ID=your-access-key
  - S3_SECRET_ACCESS_KEY=your-secret-key
  - S3_REGION=us-east-1
```

Restart the application to use the new storage configuration.

## Advanced Usage

### Resuming Interrupted Migrations

If a migration is interrupted, you can resume from the saved state:

```bash
# Resume from automatically saved state
docker exec readur-app cargo run --bin migrate_to_s3 -- --resume-from /tmp/migration_state.json

# Check what migrations are available to resume
ls /tmp/migration_state_*.json
```

### Rolling Back a Migration

If you need to undo a migration:

```bash
# Rollback using saved state file
docker exec readur-app cargo run --bin migrate_to_s3 -- --rollback /tmp/migration_state.json

# Verify rollback completion
docker exec readur-app cargo run --bin migrate_to_s3 -- --rollback-status
```

### Batch Processing Large Datasets

For very large document collections:

```bash
# Process in smaller batches
docker exec readur-app cargo run --bin migrate_to_s3 -- \
  --enable-rollback \
  --batch-size 1000 \
  --parallel-uploads 5
```

## Migration Scenarios

### Scenario 1: Local to S3 (Most Common)

```bash
# 1. Configure S3 credentials
export S3_BUCKET_NAME="company-readur-docs"
export S3_ACCESS_KEY_ID="AKIA..."
export S3_SECRET_ACCESS_KEY="..."

# 2. Test the migration
docker exec readur-app cargo run --bin migrate_to_s3 -- --dry-run

# 3. Run migration with safety features
docker exec readur-app cargo run --bin migrate_to_s3 -- --enable-rollback

# 4. Update docker-compose.yml to use S3
# 5. Restart application
```

### Scenario 2: S3 to Different S3 Bucket

```bash
# 1. Configure new bucket credentials
export S3_BUCKET_NAME="new-bucket-name"

# 2. Migrate to new bucket
docker exec readur-app cargo run --bin migrate_to_s3 -- --enable-rollback

# 3. Update configuration
```

### Scenario 3: Migrating Specific Users

```bash
# Get user IDs that need migration
docker exec readur-app psql -d readur -c \
  "SELECT id, email FROM users WHERE created_at > '2024-01-01';"

# Migrate each user individually
for user_id in $user_ids; do
  docker exec readur-app cargo run --bin migrate_to_s3 -- \
    --enable-rollback --user-id "$user_id"
done
```

## Performance Considerations

### Optimization Tips

1. **Network Bandwidth**: Migration speed depends on upload bandwidth to S3
2. **Parallel Processing**: The tool automatically optimizes concurrent uploads
3. **Large Files**: Files over 100MB use multipart uploads for better performance
4. **Memory Usage**: Migration is designed to use minimal memory regardless of file sizes

### Expected Performance

| Document Count | Typical Time | Network Impact |
|---------------|--------------|----------------|
| < 1,000       | 5-15 minutes | Low            |
| 1,000-10,000  | 30-90 minutes| Medium         |
| 10,000+       | 2-8 hours    | High           |

## Security Considerations

### Data Protection
- All transfers use HTTPS/TLS encryption
- Original files remain until migration is verified
- Database transactions ensure consistency
- Rollback preserves original state

### Access Control
- Migration tool respects existing file permissions
- S3 bucket policies should match security requirements
- Consider enabling S3 server-side encryption

### Audit Trail
- All migration operations are logged
- State files contain complete operation history
- Failed operations are tracked for debugging

## Next Steps

After successful migration:

1. **Monitor the application** for any storage-related issues
2. **Update backup procedures** to include S3 data
3. **Configure S3 lifecycle policies** for cost optimization
4. **Set up monitoring** for S3 usage and costs
5. **Clean up local files** once confident in migration success

## Support

If you encounter issues during migration:

1. Check the [troubleshooting guide](./migration-troubleshooting.md)
2. Review application logs for detailed error messages
3. Use the `--verbose` flag for detailed migration output
4. Keep state files for support debugging

Remember: **Always test migrations in a staging environment first** when possible.