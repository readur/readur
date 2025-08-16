# Backup and Recovery Guide

This guide provides comprehensive backup strategies, procedures, and recovery plans for Readur deployments to ensure business continuity and data protection.

## Backup Strategy Overview

Readur requires backing up three critical components:
1. **PostgreSQL Database** - Document metadata, user data, settings
2. **File Storage** - Uploaded documents and processed files
3. **Configuration** - Environment variables and settings

## Database Backup

### Automated Database Backups

#### Using pg_dump

Create a backup script:

```bash
#!/bin/bash
# backup-database.sh
set -e  # Exit on error

# Configuration
BACKUP_DIR="/backups/database"
DB_NAME="readur"
DB_USER="readur"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="${BACKUP_DIR}/readur_db_${TIMESTAMP}.sql.gz"
RETENTION_DAYS=30

# Create backup directory
mkdir -p ${BACKUP_DIR}

# Perform backup with proper authentication
# Note: Use PGPASSWORD environment variable or .pgpass file for authentication
PGPASSWORD="${DB_PASSWORD}" pg_dump -h localhost -U ${DB_USER} -d ${DB_NAME} --no-owner --clean --if-exists | gzip > ${BACKUP_FILE}

# Verify backup
if [ $? -eq 0 ]; then
    echo "Backup successful: ${BACKUP_FILE}"
    echo "Size: $(du -h ${BACKUP_FILE} | cut -f1)"
    
    # Test restore capability
    gunzip -c ${BACKUP_FILE} | head -n 100 > /dev/null
    if [ $? -eq 0 ]; then
        echo "Backup file verified"
    else
        echo "WARNING: Backup file may be corrupted"
        exit 1
    fi
else
    echo "Backup failed!"
    exit 1
fi

# Remove old backups
find ${BACKUP_DIR} -name "readur_db_*.sql.gz" -mtime +${RETENTION_DAYS} -delete
echo "Cleaned up backups older than ${RETENTION_DAYS} days"
```

#### Using pg_basebackup for Physical Backups

```bash
#!/bin/bash
# physical-backup.sh

BACKUP_DIR="/backups/physical"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_PATH="${BACKUP_DIR}/readur_base_${TIMESTAMP}"

# Enable archive mode in PostgreSQL first
# In postgresql.conf:
# wal_level = replica
# archive_mode = on
# archive_command = 'cp %p /archive/%f'

# Perform physical backup
pg_basebackup -D ${BACKUP_PATH} -Ft -z -P -U replication_user

# Create restore point
psql -U readur -d readur -c "SELECT pg_create_restore_point('backup_${TIMESTAMP}');"
```

#### Continuous Archiving with WAL

```yaml
# docker-compose.yml addition for WAL archiving
services:
  postgres:
    environment:
      POSTGRES_INITDB_WALDIR: /var/lib/postgresql/wal
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - postgres_wal:/var/lib/postgresql/wal
      - ./postgresql.conf:/etc/postgresql/postgresql.conf
    command: postgres -c config_file=/etc/postgresql/postgresql.conf
```

PostgreSQL configuration:
```ini
# postgresql.conf
wal_level = replica
archive_mode = on
archive_command = 'test ! -f /archive/%f && cp %p /archive/%f'
archive_timeout = 300  # Force WAL switch every 5 minutes
```

### Database Backup Scheduling

#### Using Cron

```bash
# Add to crontab
# Daily backup at 2 AM
# IMPORTANT: Set DB_PASSWORD environment variable in crontab or use .pgpass file
0 2 * * * DB_PASSWORD='your_password' /opt/readur/scripts/backup-database.sh >> /var/log/readur-backup.log 2>&1

# Hourly incremental backup
0 * * * * /opt/readur/scripts/backup-incremental.sh >> /var/log/readur-backup.log 2>&1

# Weekly full backup on Sunday
0 3 * * 0 /opt/readur/scripts/backup-full.sh >> /var/log/readur-backup.log 2>&1
```

#### Using Kubernetes CronJob

```yaml
apiVersion: batch/v1
kind: CronJob
metadata:
  name: readur-db-backup
spec:
  schedule: "0 2 * * *"  # Daily at 2 AM
  jobTemplate:
    spec:
      template:
        spec:
          containers:
          - name: backup
            image: postgres:15
            env:
            - name: PGPASSWORD
              valueFrom:
                secretKeyRef:
                  name: readur-db-secret
                  key: password
            command:
            - /bin/bash
            - -c
            - |
              pg_dump -h readur-db -U readur -d readur | \
              gzip | \
              aws s3 cp - s3://backups/readur/db/$(date +%Y%m%d_%H%M%S).sql.gz
          restartPolicy: OnFailure
```

## File Storage Backup

### Local Storage Backup

#### Using rsync

```bash
#!/bin/bash
# backup-files.sh
set -e  # Exit on error

SOURCE_DIR="/var/readur/uploads"
BACKUP_DIR="/backups/files"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_PATH="${BACKUP_DIR}/readur_files_${TIMESTAMP}"

# Create incremental backup using rsync
rsync -avz --link-dest=${BACKUP_DIR}/latest \
  ${SOURCE_DIR}/ ${BACKUP_PATH}/

# Update latest symlink
rm -f ${BACKUP_DIR}/latest
ln -s ${BACKUP_PATH} ${BACKUP_DIR}/latest

# Calculate backup size
BACKUP_SIZE=$(du -sh ${BACKUP_PATH} | cut -f1)
echo "Backup completed: ${BACKUP_PATH} (${BACKUP_SIZE})"
```

#### Using tar with compression

```bash
#!/bin/bash
# archive-files.sh
set -e  # Exit on error

SOURCE_DIR="/var/readur/uploads"
BACKUP_DIR="/backups/archives"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="${BACKUP_DIR}/readur_files_${TIMESTAMP}.tar.gz"

# Create compressed archive
tar -czf ${BACKUP_FILE} \
  --exclude='*.tmp' \
  --exclude='thumbnails/*' \
  -C ${SOURCE_DIR} .

# Verify archive
tar -tzf ${BACKUP_FILE} > /dev/null
if [ $? -eq 0 ]; then
    echo "Archive verified: ${BACKUP_FILE}"
else
    echo "Archive verification failed!"
    exit 1
fi
```

### S3 Storage Backup

#### S3 to S3 Replication

```yaml
# AWS S3 bucket replication configuration
{
  "Role": "arn:aws:iam::123456789012:role/replication-role",
  "Rules": [
    {
      "ID": "ReplicateAll",
      "Priority": 1,
      "Status": "Enabled",
      "Filter": {},
      "Destination": {
        "Bucket": "arn:aws:s3:::readur-backup-bucket",
        "ReplicationTime": {
          "Status": "Enabled",
          "Time": {
            "Minutes": 15
          }
        },
        "StorageClass": "GLACIER_IR"
      },
      "DeleteMarkerReplication": {
        "Status": "Enabled"
      }
    }
  ]
}
```

#### S3 to Local Backup

```bash
#!/bin/bash
# backup-s3-to-local.sh

S3_BUCKET="readur-documents"
LOCAL_BACKUP="/backups/s3"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_PATH="${LOCAL_BACKUP}/${TIMESTAMP}"

# Sync S3 to local
aws s3 sync s3://${S3_BUCKET} ${BACKUP_PATH}/ \
  --storage-class STANDARD_IA \
  --include "*" \
  --exclude "*.tmp"

# Create manifest
aws s3api list-objects-v2 \
  --bucket ${S3_BUCKET} \
  --query 'Contents[].{Key:Key,Size:Size,ETag:ETag}' \
  --output json > ${BACKUP_PATH}/manifest.json
```

## Configuration Backup

### Environment and Secrets Backup

```bash
#!/bin/bash
# backup-config.sh

CONFIG_DIR="/etc/readur"
BACKUP_DIR="/backups/config"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="${BACKUP_DIR}/readur_config_${TIMESTAMP}.tar.gz.gpg"

# Collect configuration files
mkdir -p /tmp/config_backup
cp ${CONFIG_DIR}/.env /tmp/config_backup/
cp ${CONFIG_DIR}/readur.yml /tmp/config_backup/
cp /etc/nginx/sites-available/readur /tmp/config_backup/nginx.conf

# Export Kubernetes secrets
kubectl get secret readur-secrets -o yaml > /tmp/config_backup/k8s-secrets.yaml

# Encrypt and archive
tar -czf - -C /tmp/config_backup . | \
  gpg --symmetric --cipher-algo AES256 --output ${BACKUP_FILE}

# Clean up
rm -rf /tmp/config_backup

echo "Encrypted config backup: ${BACKUP_FILE}"
```

## Backup Verification

### Automated Backup Testing

```bash
#!/bin/bash
# verify-backups.sh

BACKUP_DIR="/backups"
TEST_DIR="/tmp/backup_test"
LOG_FILE="/var/log/backup_verification.log"

# Function to test database backup
test_db_backup() {
    local backup_file=$1
    local test_db="readur_test_restore"
    
    echo "Testing database backup: ${backup_file}" >> ${LOG_FILE}
    
    # Create test database
    createdb ${test_db}
    
    # Restore backup
    gunzip -c ${backup_file} | psql -d ${test_db} -q
    
    # Verify tables exist
    table_count=$(psql -d ${test_db} -t -c "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema='public'")
    
    # Clean up
    dropdb ${test_db}
    
    if [ ${table_count} -gt 0 ]; then
        echo "Database backup verified: ${table_count} tables found" >> ${LOG_FILE}
        return 0
    else
        echo "Database backup verification failed!" >> ${LOG_FILE}
        return 1
    fi
}

# Function to test file backup
test_file_backup() {
    local backup_file=$1
    
    echo "Testing file backup: ${backup_file}" >> ${LOG_FILE}
    
    # Extract sample files
    mkdir -p ${TEST_DIR}
    tar -xzf ${backup_file} -C ${TEST_DIR} --wildcards '*.pdf' --limit=10
    
    # Verify files
    file_count=$(find ${TEST_DIR} -name "*.pdf" | wc -l)
    
    # Clean up
    rm -rf ${TEST_DIR}
    
    if [ ${file_count} -gt 0 ]; then
        echo "File backup verified: ${file_count} files extracted" >> ${LOG_FILE}
        return 0
    else
        echo "File backup verification failed!" >> ${LOG_FILE}
        return 1
    fi
}

# Run verification
latest_db_backup=$(ls -t ${BACKUP_DIR}/database/*.sql.gz | head -1)
latest_file_backup=$(ls -t ${BACKUP_DIR}/files/*.tar.gz | head -1)

test_db_backup ${latest_db_backup}
test_file_backup ${latest_file_backup}
```

## Recovery Procedures

### Database Recovery

#### Point-in-Time Recovery

```bash
#!/bin/bash
# restore-to-point-in-time.sh

RESTORE_TIME="2025-01-15 14:30:00"
BACKUP_FILE="/backups/database/readur_db_20250115_020000.sql.gz"
WAL_ARCHIVE="/archive"

# Stop application
docker-compose stop readur

# Restore base backup
gunzip -c ${BACKUP_FILE} | psql -U readur -d readur

# Apply WAL logs up to restore time
cat > /tmp/recovery.conf <<EOF
restore_command = 'cp ${WAL_ARCHIVE}/%f %p'
recovery_target_time = '${RESTORE_TIME}'
recovery_target_action = 'promote'
EOF

# Start PostgreSQL in recovery mode
pg_ctl start -D /var/lib/postgresql/data -o "-c recovery_config_file=/tmp/recovery.conf"

# Wait for recovery
while [ -f /var/lib/postgresql/data/recovery.signal ]; do
    sleep 5
    echo "Recovery in progress..."
done

echo "Recovery completed to ${RESTORE_TIME}"
```

#### Full Database Restore

```bash
#!/bin/bash
# full-restore.sh

BACKUP_FILE=$1

if [ -z "${BACKUP_FILE}" ]; then
    echo "Usage: $0 <backup_file>"
    exit 1
fi

# Confirm restoration
read -p "This will replace the entire database. Continue? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    exit 1
fi

# Stop application
systemctl stop readur

# Drop and recreate database
psql -U postgres <<EOF
DROP DATABASE IF EXISTS readur;
CREATE DATABASE readur OWNER readur;
EOF

# Restore backup
gunzip -c ${BACKUP_FILE} | psql -U readur -d readur

# Verify restoration
psql -U readur -d readur -c "SELECT COUNT(*) FROM documents;"

# Restart application
systemctl start readur

echo "Database restored from ${BACKUP_FILE}"
```

### File Storage Recovery

#### Local Storage Recovery

```bash
#!/bin/bash
# restore-files.sh

BACKUP_PATH=$1
RESTORE_PATH="/var/readur/uploads"

if [ -z "${BACKUP_PATH}" ]; then
    echo "Usage: $0 <backup_path>"
    exit 1
fi

# Create restore point
mv ${RESTORE_PATH} ${RESTORE_PATH}.old

# Restore files
if [[ ${BACKUP_PATH} == *.tar.gz ]]; then
    # Extract archive
    mkdir -p ${RESTORE_PATH}
    tar -xzf ${BACKUP_PATH} -C ${RESTORE_PATH}
else
    # Copy directory
    cp -r ${BACKUP_PATH} ${RESTORE_PATH}
fi

# Fix permissions
chown -R readur:readur ${RESTORE_PATH}
chmod -R 755 ${RESTORE_PATH}

# Verify restoration
file_count=$(find ${RESTORE_PATH} -type f | wc -l)
echo "Restored ${file_count} files to ${RESTORE_PATH}"

# Clean up old backup
read -p "Remove old backup? (y/N) " -n 1 -r
if [[ $REPLY =~ ^[Yy]$ ]]; then
    rm -rf ${RESTORE_PATH}.old
fi
```

#### S3 Recovery

```bash
#!/bin/bash
# restore-s3.sh

SOURCE_BUCKET="readur-backup"
DEST_BUCKET="readur-documents"
RESTORE_DATE="2025-01-15"

# List available backups
echo "Available backups:"
aws s3 ls s3://${SOURCE_BUCKET}/ --recursive | grep ${RESTORE_DATE}

# Restore specific backup
aws s3 sync s3://${SOURCE_BUCKET}/${RESTORE_DATE}/ s3://${DEST_BUCKET}/ \
  --delete \
  --exact-timestamps

# Verify restoration
object_count=$(aws s3 ls s3://${DEST_BUCKET}/ --recursive | wc -l)
echo "Restored ${object_count} objects to ${DEST_BUCKET}"
```

## Disaster Recovery Plan

### RTO and RPO Targets

| Component | RPO (Recovery Point Objective) | RTO (Recovery Time Objective) |
|-----------|-------------------------------|------------------------------|
| Database | 1 hour | 2 hours |
| File Storage | 4 hours | 4 hours |
| Full System | 4 hours | 8 hours |

### Recovery Runbook

#### Phase 1: Assessment (15 minutes)
```bash
#!/bin/bash
# assess-damage.sh

echo "=== Disaster Recovery Assessment ==="
echo "Time: $(date)"

# Check database
pg_isready -h localhost -p 5432 -U readur
DB_STATUS=$?

# Check file storage
df -h /var/readur/uploads
STORAGE_STATUS=$?

# Check application
curl -f http://localhost:8080/health
APP_STATUS=$?

echo "Database status: ${DB_STATUS}"
echo "Storage status: ${STORAGE_STATUS}"
echo "Application status: ${APP_STATUS}"

if [ ${DB_STATUS} -ne 0 ]; then
    echo "ACTION: Database recovery required"
fi
if [ ${STORAGE_STATUS} -ne 0 ]; then
    echo "ACTION: Storage recovery required"
fi
if [ ${APP_STATUS} -ne 0 ]; then
    echo "ACTION: Application recovery required"
fi
```

#### Phase 2: Recovery Execution

```bash
#!/bin/bash
# execute-recovery.sh

RECOVERY_LOG="/var/log/disaster_recovery_$(date +%Y%m%d_%H%M%S).log"

# Function to log with timestamp
log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" | tee -a ${RECOVERY_LOG}
}

log "Starting disaster recovery"

# 1. Restore database
log "Restoring database..."
latest_db_backup=$(ls -t /backups/database/*.sql.gz | head -1)
./full-restore.sh ${latest_db_backup} >> ${RECOVERY_LOG} 2>&1

# 2. Restore files
log "Restoring file storage..."
latest_file_backup=$(ls -t /backups/files/*.tar.gz | head -1)
./restore-files.sh ${latest_file_backup} >> ${RECOVERY_LOG} 2>&1

# 3. Restore configuration
log "Restoring configuration..."
latest_config_backup=$(ls -t /backups/config/*.tar.gz.gpg | head -1)
gpg --decrypt ${latest_config_backup} | tar -xzf - -C /etc/readur/

# 4. Start services
log "Starting services..."
systemctl start postgresql
systemctl start readur
systemctl start nginx

# 5. Verify recovery
log "Verifying recovery..."
sleep 30
./health-check.sh >> ${RECOVERY_LOG} 2>&1

log "Disaster recovery completed"
```

## Important Architecture Note

**Readur is designed as a single-instance application.** It does not support high availability configurations with multiple server instances, database replication, or clustering. For reliability:

- Implement regular automated backups
- Use reliable storage (RAID, cloud storage)
- Monitor system health
- Have a tested recovery procedure

### Single-Instance Best Practices

1. **Regular Backups**: Schedule frequent backups based on your RPO requirements
2. **Monitoring**: Set up alerts for system failures
3. **Quick Recovery**: Keep recovery scripts ready and tested
4. **Hardware Redundancy**: Use RAID for storage, redundant power supplies
5. **Cloud Deployment**: Consider cloud platforms with built-in reliability features

## Backup Storage Management

### Retention Policies

```bash
#!/bin/bash
# manage-retention.sh

# Retention periods (days)
DAILY_RETENTION=7
WEEKLY_RETENTION=28
MONTHLY_RETENTION=365

BACKUP_DIR="/backups"

# Keep daily backups for 7 days
find ${BACKUP_DIR}/daily -mtime +${DAILY_RETENTION} -delete

# Keep weekly backups for 4 weeks
find ${BACKUP_DIR}/weekly -mtime +${WEEKLY_RETENTION} -delete

# Keep monthly backups for 1 year
find ${BACKUP_DIR}/monthly -mtime +${MONTHLY_RETENTION} -delete

# Archive old backups to glacier
find ${BACKUP_DIR}/monthly -mtime +30 -name "*.tar.gz" | while read file; do
    aws s3 cp ${file} s3://readur-archive/ --storage-class GLACIER
    rm ${file}
done
```

### Storage Monitoring

```bash
#!/bin/bash
# monitor-backup-storage.sh

BACKUP_DIR="/backups"
THRESHOLD=80  # Alert when usage exceeds 80%

# Check disk usage
usage=$(df ${BACKUP_DIR} | awk 'NR==2 {print int($5)}')

if [ ${usage} -gt ${THRESHOLD} ]; then
    echo "WARNING: Backup storage at ${usage}% capacity"
    
    # Send alert
    curl -X POST https://alerts.example.com/webhook \
      -H "Content-Type: application/json" \
      -d "{\"message\": \"Backup storage critical: ${usage}% used\"}"
    
    # Trigger cleanup
    ./manage-retention.sh
fi

# Report backup statistics
echo "=== Backup Storage Report ==="
echo "Total backups: $(find ${BACKUP_DIR} -type f | wc -l)"
echo "Storage used: $(du -sh ${BACKUP_DIR} | cut -f1)"
echo "Oldest backup: $(ls -t ${BACKUP_DIR}/**/*.gz | tail -1)"
echo "Latest backup: $(ls -t ${BACKUP_DIR}/**/*.gz | head -1)"
```

## Testing and Validation

### Disaster Recovery Drill

```bash
#!/bin/bash
# dr-drill.sh

# Schedule: Run quarterly
# Purpose: Validate recovery procedures

TEST_ENV="dr-test"
START_TIME=$(date +%s)

echo "=== Starting Disaster Recovery Drill ==="
date

# 1. Create test environment
docker-compose -f docker-compose.test.yml up -d

# 2. Restore from backup
./execute-recovery.sh --env ${TEST_ENV}

# 3. Validate data integrity
psql -h localhost -p 5433 -U readur -d readur_test <<EOF
SELECT COUNT(*) as document_count FROM documents;
SELECT COUNT(*) as user_count FROM users;
SELECT COUNT(DISTINCT file_hash) as unique_files FROM documents;
EOF

# 4. Test application functionality
curl -f http://localhost:8081/health
curl -f http://localhost:8081/api/documents

# 5. Clean up
docker-compose -f docker-compose.test.yml down

END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

echo "=== Disaster Recovery Drill Completed ==="
echo "Duration: ${DURATION} seconds"
echo "Target RTO: 8 hours (28800 seconds)"

if [ ${DURATION} -lt 28800 ]; then
    echo "✓ RTO target met"
else
    echo "✗ RTO target exceeded"
fi
```

## Security Considerations

### Backup Encryption

```bash
# Encrypt backups at rest
openssl enc -aes-256-cbc -salt -in backup.tar.gz -out backup.tar.gz.enc

# Decrypt for restoration
openssl enc -aes-256-cbc -d -in backup.tar.gz.enc -out backup.tar.gz
```

### Access Control

```bash
# Set proper permissions
chmod 600 /backups/database/*.sql.gz
chmod 600 /backups/config/*.gpg
chown -R backup:backup /backups

# Restrict backup script execution
chmod 750 /opt/readur/scripts/backup-*.sh
```

## Monitoring and Alerting

### Backup Monitoring

```yaml
# Prometheus alert rules
groups:
  - name: backup_alerts
    rules:
      - alert: BackupFailed
        expr: backup_last_success_timestamp < (time() - 86400)
        annotations:
          summary: "Backup has not succeeded in 24 hours"
          
      - alert: BackupStorageFull
        expr: backup_storage_usage_percent > 90
        annotations:
          summary: "Backup storage is {{ $value }}% full"
          
      - alert: BackupVerificationFailed
        expr: backup_verification_success == 0
        annotations:
          summary: "Backup verification failed"
```

## Checklist

### Daily Tasks
- [ ] Verify overnight backups completed
- [ ] Check backup storage capacity
- [ ] Review backup logs for errors

### Weekly Tasks
- [ ] Test restore procedure with sample data
- [ ] Verify backup retention policy
- [ ] Update backup documentation

### Monthly Tasks
- [ ] Full backup verification
- [ ] Review and update RTO/RPO targets
- [ ] Test failover procedures

### Quarterly Tasks
- [ ] Complete disaster recovery drill
- [ ] Review and update recovery runbooks
- [ ] Audit backup security and encryption