# Migration Troubleshooting Guide

## Common Issues and Solutions

### S3 Access Issues

#### "Access Denied" Errors
**Symptoms:**
- Migration fails with "Access Denied" messages
- S3 uploads return 403 errors

**Causes:**
- Insufficient IAM permissions
- Incorrect bucket policy
- Wrong AWS credentials

**Solutions:**

1. **Verify IAM Policy**
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
           "s3:GetBucketLocation"
         ],
         "Resource": [
           "arn:aws:s3:::your-bucket-name",
           "arn:aws:s3:::your-bucket-name/*"
         ]
       }
     ]
   }
   ```

2. **Test S3 Access**
   ```bash
   # Test bucket access
   aws s3 ls s3://your-bucket-name/
   
   # Test upload
   echo "test" | aws s3 cp - s3://your-bucket-name/test.txt
   
   # Clean up test file
   aws s3 rm s3://your-bucket-name/test.txt
   ```

3. **Check Environment Variables**
   ```bash
   # Verify credentials are set
   docker exec readur-app env | grep S3_
   
   # Should show:
   # S3_BUCKET_NAME=your-bucket
   # S3_ACCESS_KEY_ID=AKIA...
   # S3_SECRET_ACCESS_KEY=... (hidden)
   # S3_REGION=us-east-1
   ```

#### "Bucket Does Not Exist" Errors
**Solution:**
```bash
# Create the bucket
aws s3 mb s3://your-bucket-name --region us-east-1

# Or use different region
aws s3 mb s3://your-bucket-name --region eu-west-1
```

### Migration Interruption Issues

#### Network Timeout Errors
**Symptoms:**
- Migration stops with network timeout messages
- "Connection reset by peer" errors

**Solutions:**

1. **Resume Migration**
   ```bash
   # Find the latest state file
   docker exec readur-app ls -la /tmp/migration_state_*.json
   
   # Resume from state
   docker exec readur-app cargo run --bin migrate_to_s3 -- \
     --resume-from /tmp/migration_state_20241201_143022.json
   ```

2. **Reduce Batch Size**
   ```bash
   # Process smaller batches
   docker exec readur-app cargo run --bin migrate_to_s3 -- \
     --enable-rollback --batch-size 500
   ```

3. **Check Network Stability**
   ```bash
   # Test S3 connectivity
   ping s3.amazonaws.com
   
   # Test sustained transfer
   aws s3 cp /dev/zero s3://your-bucket/test-10mb --expected-size 10485760
   ```

#### Server Restart During Migration
**Solution:**
```bash
# Check for existing state files
docker exec readur-app find /tmp -name "migration_state_*.json" -mtime -1

# Resume from most recent state
docker exec readur-app cargo run --bin migrate_to_s3 -- \
  --resume-from /tmp/migration_state_LATEST.json
```

### Database Issues

#### "Database Connection Lost"
**Symptoms:**
- Migration fails with database connection errors
- PostgreSQL timeout messages

**Solutions:**

1. **Check Database Status**
   ```bash
   # Test database connection
   docker exec readur-app psql -d readur -c "SELECT version();"
   
   # Check connection limits
   docker exec readur-app psql -d readur -c \
     "SELECT setting FROM pg_settings WHERE name = 'max_connections';"
   ```

2. **Increase Connection Timeout**
   ```bash
   # Add to environment variables
   export DATABASE_TIMEOUT=300  # 5 minutes
   ```

3. **Check Transaction Locks**
   ```bash
   # Look for long-running transactions
   docker exec readur-app psql -d readur -c \
     "SELECT pid, state, query_start, query FROM pg_stat_activity WHERE state != 'idle';"
   ```

#### "Transaction Rollback" Errors
**Solution:**
```bash
# Check for conflicting processes
docker exec readur-app psql -d readur -c \
  "SELECT * FROM pg_locks WHERE NOT granted;"

# Restart migration with fresh transaction
docker exec readur-app cargo run --bin migrate_to_s3 -- \
  --enable-rollback --fresh-start
```

### File System Issues

#### "File Not Found" Errors
**Symptoms:**
- Migration reports files in database that don't exist on disk
- Inconsistent file counts

**Solutions:**

1. **Audit File Consistency**
   ```bash
   # Check for orphaned database records
   docker exec readur-app cargo run --bin migrate_to_s3 -- --audit-files
   ```

2. **Clean Up Database**
   ```bash
   # Remove orphaned records (BE CAREFUL!)
   docker exec readur-app psql -d readur -c \
     "DELETE FROM documents WHERE file_path NOT IN (
        SELECT DISTINCT file_path FROM documents 
        WHERE file_path IS NOT NULL
      );"
   ```

#### "Permission Denied" on Local Files
**Solution:**
```bash
# Check file permissions
docker exec readur-app ls -la /app/uploads/

# Fix permissions if needed
docker exec readur-app chown -R readur:readur /app/uploads/
```

### Performance Issues

#### Very Slow Migration
**Symptoms:**
- Migration takes much longer than expected
- Low upload speeds to S3

**Solutions:**

1. **Check Network Performance**
   ```bash
   # Test upload speed to S3
   dd if=/dev/zero bs=1M count=100 | aws s3 cp - s3://your-bucket/speedtest.dat
   
   # Check result and clean up
   aws s3 rm s3://your-bucket/speedtest.dat
   ```

2. **Optimize Migration Settings**
   ```bash
   # Increase parallel uploads
   docker exec readur-app cargo run --bin migrate_to_s3 -- \
     --enable-rollback --parallel-uploads 10
   ```

3. **Use Multipart Upload Threshold**
   ```bash
   # Lower threshold for multipart uploads
   docker exec readur-app cargo run --bin migrate_to_s3 -- \
     --enable-rollback --multipart-threshold 50MB
   ```

#### High Memory Usage
**Solution:**
```bash
# Monitor container memory
docker stats readur-app

# Reduce batch size if needed
docker exec readur-app cargo run --bin migrate_to_s3 -- \
  --enable-rollback --batch-size 100
```

### Rollback Issues

#### "Cannot Rollback - State File Missing"
**Solution:**
```bash
# Look for backup state files
docker exec readur-app find /tmp -name "*migration*" -type f

# Manual rollback (use with caution)
docker exec readur-app psql -d readur -c \
  "UPDATE documents SET file_path = REPLACE(file_path, 's3://', '/app/uploads/');"
```

#### "Partial Rollback Completed"
**Symptoms:**
- Some files rolled back, others still in S3
- Database in inconsistent state

**Solution:**
```bash
# Complete the rollback manually
docker exec readur-app cargo run --bin migrate_to_s3 -- \
  --force-rollback --state-file /tmp/migration_state_backup.json

# Verify database consistency
docker exec readur-app cargo run --bin migrate_to_s3 -- --verify-state
```

## Validation Commands

### Pre-Migration Checks
```bash
# Check database connectivity
docker exec readur-app psql -d readur -c "SELECT COUNT(*) FROM documents;"

# Verify S3 access
aws s3 ls s3://your-bucket-name/

# Check disk space
docker exec readur-app df -h /app/uploads/
```

### Post-Migration Validation
```bash
# Compare document counts
LOCAL_COUNT=$(docker exec readur-app find /app/uploads -type f | wc -l)
DB_COUNT=$(docker exec readur-app psql -d readur -t -c "SELECT COUNT(*) FROM documents;")
S3_COUNT=$(aws s3 ls s3://your-bucket/documents/ --recursive | wc -l)

echo "Local files: $LOCAL_COUNT"
echo "Database records: $DB_COUNT" 
echo "S3 objects: $S3_COUNT"

# Test random document access
RANDOM_DOC=$(docker exec readur-app psql -d readur -t -c \
  "SELECT id FROM documents ORDER BY RANDOM() LIMIT 1;")
curl -I "https://your-readur-instance.com/api/documents/$RANDOM_DOC/download"
```

### Health Checks
```bash
# Application health
curl -f https://your-readur-instance.com/health

# Storage backend test
docker exec readur-app cargo run --bin migrate_to_s3 -- --test-storage

# Database integrity
docker exec readur-app psql -d readur -c \
  "SELECT COUNT(*) FROM documents WHERE file_path IS NULL OR file_path = '';"
```

## Recovery Procedures

### Emergency Stop
```bash
# Stop running migration
docker exec readur-app pkill -f migrate_to_s3

# Check for partial state
ls /tmp/migration_state_*.json

# Decide: resume, rollback, or restart
```

### Data Recovery
```bash
# Restore from backup (if needed)
docker exec -i readur-db psql -U readur -d readur < readur_backup.sql

# Restore files from backup
docker exec readur-app tar -xzf documents_backup.tar.gz -C /
```

### Clean Start
```bash
# Remove all migration state files
docker exec readur-app rm -f /tmp/migration_state_*.json

# Start fresh migration
docker exec readur-app cargo run --bin migrate_to_s3 -- \
  --enable-rollback --fresh-start
```

## Getting Help

### Log Analysis
```bash
# View migration logs
docker logs readur-app | grep -i migration

# Check application logs
docker exec readur-app tail -f /var/log/readur/app.log

# Database logs
docker logs readur-db | tail -100
```

### Debug Information
When reporting issues, include:

1. **Migration command used**
2. **Error messages** (full text)
3. **State file contents** (if available)
4. **System information**:
   ```bash
   docker --version
   docker exec readur-app cargo --version
   docker exec readur-app psql --version
   ```
5. **Environment configuration** (sanitized):
   ```bash
   docker exec readur-app env | grep -E "(S3_|DATABASE_)" | sed 's/=.*/=***/'
   ```

### Support Checklist

Before requesting support:

- [ ] Checked this troubleshooting guide
- [ ] Reviewed application logs  
- [ ] Verified S3 credentials and permissions
- [ ] Tested basic S3 connectivity
- [ ] Confirmed database is accessible
- [ ] Have backup of data before migration
- [ ] Can provide error messages and command used

## Prevention Tips

1. **Always test in staging first**
2. **Use dry-run mode before real migration**
3. **Ensure adequate disk space and memory**
4. **Verify S3 permissions before starting**
5. **Keep multiple backup copies**
6. **Monitor migration progress actively**
7. **Have rollback plan ready**

Remember: When in doubt, it's better to rollback and investigate than to continue with a problematic migration.