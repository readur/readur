# S3 Storage Troubleshooting Guide

## Overview

This guide addresses common issues encountered when using S3 storage with Readur and provides detailed solutions.

## Quick Diagnostics

### S3 Health Check Script

```bash
#!/bin/bash
# s3-health-check.sh

echo "Readur S3 Storage Health Check"
echo "=============================="

# Load configuration
source .env

# Check S3 connectivity
echo -n "1. Checking S3 connectivity... "
if aws s3 ls s3://$S3_BUCKET_NAME --region $S3_REGION > /dev/null 2>&1; then
    echo "✓ Connected"
else
    echo "✗ Failed"
    echo "   Error: Cannot connect to S3 bucket"
    exit 1
fi

# Check bucket permissions
echo -n "2. Checking bucket permissions... "
TEST_FILE="/tmp/readur-test-$$"
echo "test" > $TEST_FILE

if aws s3 cp $TEST_FILE s3://$S3_BUCKET_NAME/test-write-$$ --region $S3_REGION > /dev/null 2>&1; then
    echo "✓ Write permission OK"
    aws s3 rm s3://$S3_BUCKET_NAME/test-write-$$ --region $S3_REGION > /dev/null 2>&1
else
    echo "✗ Write permission failed"
fi
rm -f $TEST_FILE

# Check multipart upload
echo -n "3. Checking multipart upload capability... "
if aws s3api put-bucket-accelerate-configuration \
    --bucket $S3_BUCKET_NAME \
    --accelerate-configuration Status=Suspended \
    --region $S3_REGION > /dev/null 2>&1; then
    echo "✓ Multipart enabled"
else
    echo "⚠ May not have full permissions"
fi

echo ""
echo "Health check complete!"
```

## Common Issues and Solutions

### 1. Connection Issues

#### Problem: "Failed to access S3 bucket"

**Symptoms:**
- Error during startup
- Cannot upload documents
- Migration tool fails immediately

**Diagnosis:**
```bash
# Test basic connectivity
aws s3 ls s3://your-bucket-name

# Check credentials
aws sts get-caller-identity

# Verify region
aws s3api get-bucket-location --bucket your-bucket-name
```

**Solutions:**

1. **Incorrect credentials:**
   ```bash
   # Verify environment variables
   echo $S3_ACCESS_KEY_ID
   echo $S3_SECRET_ACCESS_KEY
   
   # Test with AWS CLI
   export AWS_ACCESS_KEY_ID=$S3_ACCESS_KEY_ID
   export AWS_SECRET_ACCESS_KEY=$S3_SECRET_ACCESS_KEY
   aws s3 ls
   ```

2. **Wrong region:**
   ```bash
   # Find correct region
   aws s3api get-bucket-location --bucket your-bucket-name
   
   # Update configuration
   export S3_REGION=correct-region
   ```

3. **Network issues:**
   ```bash
   # Test network connectivity
   curl -I https://s3.amazonaws.com
   
   # Check DNS resolution
   nslookup s3.amazonaws.com
   
   # Test with specific endpoint
   curl -I https://your-bucket.s3.amazonaws.com
   ```

### 2. Permission Errors

#### Problem: "AccessDenied: Access Denied"

**Symptoms:**
- Can list bucket but cannot upload
- Can upload but cannot delete
- Partial operations succeed

**Diagnosis:**
```bash
# Check IAM user permissions
aws iam get-user-policy --user-name readur-user --policy-name ReadurPolicy

# Test specific operations
aws s3api put-object --bucket your-bucket --key test.txt --body /tmp/test.txt
aws s3api get-object --bucket your-bucket --key test.txt /tmp/downloaded.txt
aws s3api delete-object --bucket your-bucket --key test.txt
```

**Solutions:**

1. **Update IAM policy:**
   ```json
   {
     "Version": "2012-10-17",
     "Statement": [
       {
         "Effect": "Allow",
         "Action": [
           "s3:ListBucket",
           "s3:GetBucketLocation"
         ],
         "Resource": "arn:aws:s3:::your-bucket-name"
       },
       {
         "Effect": "Allow",
         "Action": [
           "s3:PutObject",
           "s3:GetObject",
           "s3:DeleteObject",
           "s3:PutObjectAcl",
           "s3:GetObjectAcl"
         ],
         "Resource": "arn:aws:s3:::your-bucket-name/*"
       }
     ]
   }
   ```

2. **Check bucket policy:**
   ```bash
   aws s3api get-bucket-policy --bucket your-bucket-name
   ```

3. **Verify CORS configuration:**
   ```json
   {
     "CORSRules": [
       {
         "AllowedOrigins": ["*"],
         "AllowedMethods": ["GET", "PUT", "POST", "DELETE", "HEAD"],
         "AllowedHeaders": ["*"],
         "ExposeHeaders": ["ETag"],
         "MaxAgeSeconds": 3000
       }
     ]
   }
   ```

### 3. Upload Failures

#### Problem: Large files fail to upload

**Symptoms:**
- Small files upload successfully
- Large files timeout or fail
- "RequestTimeout" errors

**Diagnosis:**
```bash
# Check multipart upload configuration
aws s3api list-multipart-uploads --bucket your-bucket-name

# Test large file upload
dd if=/dev/zero of=/tmp/large-test bs=1M count=150
aws s3 cp /tmp/large-test s3://your-bucket-name/test-large
```

**Solutions:**

1. **Increase timeouts:**
   ```rust
   // In code configuration
   const UPLOAD_TIMEOUT: Duration = Duration::from_secs(3600);
   ```

2. **Optimize chunk size:**
   ```bash
   # For slow connections, use smaller chunks
   export S3_MULTIPART_CHUNK_SIZE=8388608  # 8MB chunks
   ```

3. **Resume failed uploads:**
   ```bash
   # List incomplete multipart uploads
   aws s3api list-multipart-uploads --bucket your-bucket-name
   
   # Abort stuck uploads
   aws s3api abort-multipart-upload \
     --bucket your-bucket-name \
     --key path/to/file \
     --upload-id UPLOAD_ID
   ```

### 4. S3-Compatible Service Issues

#### Problem: MinIO/Wasabi/Backblaze not working

**Symptoms:**
- AWS S3 works but compatible service doesn't
- "InvalidEndpoint" errors
- SSL certificate errors

**Solutions:**

1. **MinIO configuration:**
   ```bash
   # Correct endpoint format
   S3_ENDPOINT=http://minio.local:9000  # No https:// for local
   S3_ENDPOINT=https://minio.example.com  # With SSL
   
   # Path-style addressing
   S3_FORCE_PATH_STYLE=true
   ```

2. **Wasabi configuration:**
   ```bash
   S3_ENDPOINT=https://s3.wasabisys.com
   S3_REGION=us-east-1  # Or your Wasabi region
   ```

3. **SSL certificate issues:**
   ```bash
   # Disable SSL verification (development only!)
   export AWS_CA_BUNDLE=/path/to/custom-ca.crt
   
   # Or for self-signed certificates
   export NODE_TLS_REJECT_UNAUTHORIZED=0  # Not recommended for production
   ```

### 5. Migration Problems

#### Problem: Migration tool hangs or fails

**Symptoms:**
- Migration starts but doesn't progress
- "File not found" errors during migration
- Database inconsistencies after partial migration

**Diagnosis:**
```bash
# Check migration state
cat migration_state.json | jq '.'

# Find failed migrations
cat migration_state.json | jq '.failed_migrations'

# Check for orphaned files
find ./uploads -type f -name "*.pdf" | head -10
```

**Solutions:**

1. **Resume from last successful point:**
   ```bash
   # Get last successful migration
   LAST_ID=$(cat migration_state.json | jq -r '.completed_migrations[-1].document_id')
   
   # Resume migration
   cargo run --bin migrate_to_s3 --features s3 -- --resume-from $LAST_ID
   ```

2. **Fix missing local files:**
   ```sql
   -- Find documents with missing files
   SELECT id, filename, file_path 
   FROM documents 
   WHERE file_path NOT LIKE 's3://%'
   AND NOT EXISTS (
     SELECT 1 FROM pg_stat_file(file_path)
   );
   ```

3. **Rollback failed migration:**
   ```bash
   # Automatic rollback
   cargo run --bin migrate_to_s3 --features s3 -- --rollback
   
   # Manual cleanup
   psql $DATABASE_URL -c "UPDATE documents SET file_path = original_path WHERE file_path LIKE 's3://%';"
   ```

### 6. Performance Issues

#### Problem: Slow document retrieval from S3

**Symptoms:**
- Document downloads are slow
- High latency for thumbnail loading
- Timeouts on document preview

**Diagnosis:**
```bash
# Measure S3 latency
time aws s3 cp s3://your-bucket/test-file /tmp/test-download

# Check S3 transfer metrics
aws cloudwatch get-metric-statistics \
  --namespace AWS/S3 \
  --metric-name AllRequests \
  --dimensions Name=BucketName,Value=your-bucket \
  --start-time 2024-01-01T00:00:00Z \
  --end-time 2024-01-02T00:00:00Z \
  --period 3600 \
  --statistics Average
```

**Solutions:**

1. **Enable S3 Transfer Acceleration:**
   ```bash
   aws s3api put-bucket-accelerate-configuration \
     --bucket your-bucket-name \
     --accelerate-configuration Status=Enabled
   
   # Update endpoint
   S3_ENDPOINT=https://your-bucket.s3-accelerate.amazonaws.com
   ```

2. **Implement caching:**
   ```nginx
   # Nginx caching configuration
   proxy_cache_path /var/cache/nginx/s3 levels=1:2 keys_zone=s3_cache:10m max_size=1g;
   
   location /api/documents/ {
       proxy_cache s3_cache;
       proxy_cache_valid 200 1h;
       proxy_cache_key "$request_uri";
   }
   ```

3. **Use CloudFront CDN:**
   ```bash
   # Create CloudFront distribution
   aws cloudfront create-distribution \
     --origin-domain-name your-bucket.s3.amazonaws.com \
     --default-root-object index.html
   ```

## Advanced Debugging

### Enable Debug Logging

```bash
# Set environment variables
export RUST_LOG=readur=debug,aws_sdk_s3=debug,aws_config=debug
export RUST_BACKTRACE=full

# Run Readur with debug output
./readur 2>&1 | tee readur-debug.log
```

### S3 Request Logging

```bash
# Enable S3 access logging
aws s3api put-bucket-logging \
  --bucket your-bucket-name \
  --bucket-logging-status '{
    "LoggingEnabled": {
      "TargetBucket": "your-logs-bucket",
      "TargetPrefix": "s3-access-logs/"
    }
  }'
```

### Network Troubleshooting

```bash
# Trace S3 requests
tcpdump -i any -w s3-traffic.pcap host s3.amazonaws.com

# Analyze with Wireshark
wireshark s3-traffic.pcap

# Check MTU issues
ping -M do -s 1472 s3.amazonaws.com
```

## Monitoring and Alerts

### CloudWatch Metrics

```bash
# Create alarm for high error rate
aws cloudwatch put-metric-alarm \
  --alarm-name s3-high-error-rate \
  --alarm-description "Alert when S3 error rate is high" \
  --metric-name 4xxErrors \
  --namespace AWS/S3 \
  --statistic Sum \
  --period 300 \
  --threshold 10 \
  --comparison-operator GreaterThanThreshold \
  --evaluation-periods 2
```

### Log Analysis

```bash
# Parse S3 access logs
aws s3 sync s3://your-logs-bucket/s3-access-logs/ ./logs/

# Find errors
grep -E "4[0-9]{2}|5[0-9]{2}" ./logs/*.log | head -20

# Analyze request patterns
awk '{print $8}' ./logs/*.log | sort | uniq -c | sort -rn | head -20
```

## Recovery Procedures

### Corrupted S3 Data

```bash
# Verify object integrity
aws s3api head-object --bucket your-bucket --key path/to/document.pdf

# Restore from versioning
aws s3api list-object-versions --bucket your-bucket --prefix path/to/

# Restore specific version
aws s3api get-object \
  --bucket your-bucket \
  --key path/to/document.pdf \
  --version-id VERSION_ID \
  /tmp/recovered-document.pdf
```

### Database Inconsistency

```sql
-- Find orphaned S3 references
SELECT id, file_path 
FROM documents 
WHERE file_path LIKE 's3://%'
AND file_path NOT IN (
  SELECT 's3://' || key FROM s3_inventory_table
);

-- Update paths after bucket migration
UPDATE documents 
SET file_path = REPLACE(file_path, 's3://old-bucket/', 's3://new-bucket/')
WHERE file_path LIKE 's3://old-bucket/%';
```

## Prevention Best Practices

1. **Regular Health Checks**: Run diagnostic scripts daily
2. **Monitor Metrics**: Set up CloudWatch dashboards
3. **Test Failover**: Regularly test backup procedures
4. **Document Changes**: Keep configuration changelog
5. **Capacity Planning**: Monitor storage growth trends

## Getting Help

If issues persist after following this guide:

1. **Collect Diagnostics**:
   ```bash
   ./collect-diagnostics.sh > diagnostics.txt
   ```

2. **Check Logs**:
   - Application logs: `journalctl -u readur -n 1000`
   - S3 access logs: Check CloudWatch or S3 access logs
   - Database logs: `tail -f /var/log/postgresql/*.log`

3. **Contact Support**:
   - Include diagnostics output
   - Provide configuration (sanitized)
   - Describe symptoms and timeline
   - Share any error messages