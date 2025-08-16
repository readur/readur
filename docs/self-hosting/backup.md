# Backup and Recovery Guide

## Overview

This guide covers comprehensive backup strategies for Readur, including database backups, document storage, configuration files, and disaster recovery procedures.

## What to Backup

### Critical Components

1. **PostgreSQL Database** - Contains all metadata, user data, and system configuration
2. **Document Storage** - Original documents and processed files
3. **Configuration Files** - Environment variables and settings
4. **SSL Certificates** - If using custom certificates
5. **Custom Code** - Any modifications or plugins

### Backup Priority Matrix

| Component | Priority | RPO | RTO | Backup Frequency |
|-----------|----------|-----|-----|------------------|
| Database | Critical | 1 hour | 30 min | Hourly |
| Documents | Critical | 24 hours | 2 hours | Daily |
| Config | High | 24 hours | 1 hour | On change |
| Logs | Medium | 7 days | N/A | Weekly |
| Cache | Low | N/A | N/A | Not required |

## Database Backup

### PostgreSQL Backup Methods

#### Method 1: pg_dump (Logical Backup)

```bash
#!/bin/bash
# backup-database.sh

# Configuration
DB_NAME="readur"
DB_USER="readur"
BACKUP_DIR="/backup/postgres"
DATE=$(date +%Y%m%d_%H%M%S)

# Create backup
pg_dump -U $DB_USER -d $DB_NAME -F custom -f "$BACKUP_DIR/readur_$DATE.dump"

# Compress backup
gzip "$BACKUP_DIR/readur_$DATE.dump"

# Keep only last 30 days
find $BACKUP_DIR -name "*.dump.gz" -mtime +30 -delete

# Upload to S3 (optional)
aws s3 cp "$BACKUP_DIR/readur_$DATE.dump.gz" s3://backup-bucket/postgres/
```

#### Method 2: Physical Backup with pg_basebackup

```bash
#!/bin/bash
# physical-backup.sh

# Stop application (optional for consistency)
docker-compose stop readur

# Perform base backup
pg_basebackup -U replicator -D /backup/pgdata_$(date +%Y%m%d) \
  -Fp -Xs -P -R

# Start application
docker-compose start readur
```

#### Method 3: Continuous Archiving (WAL)

```bash
# postgresql.conf
wal_level = replica
archive_mode = on
archive_command = 'test ! -f /archive/%f && cp %p /archive/%f'
max_wal_senders = 3
wal_keep_segments = 64
```

### Docker Database Backup

```bash
#!/bin/bash
# docker-db-backup.sh

# Backup database from Docker container
docker-compose exec -T postgres pg_dump -U readur readur | \
  gzip > backup_$(date +%Y%m%d_%H%M%S).sql.gz

# Alternative: Using docker run
docker run --rm \
  --network readur_default \
  postgres:14 \
  pg_dump -h postgres -U readur readur | \
  gzip > backup_$(date +%Y%m%d_%H%M%S).sql.gz
```

## Document Storage Backup

### Local Storage Backup

```bash
#!/bin/bash
# backup-documents.sh

SOURCE="/data/readur/documents"
BACKUP_DIR="/backup/documents"
DATE=$(date +%Y%m%d)

# Incremental backup with rsync
rsync -avz --delete \
  --backup --backup-dir="$BACKUP_DIR/incremental_$DATE" \
  "$SOURCE/" "$BACKUP_DIR/current/"

# Create tar archive
tar -czf "$BACKUP_DIR/documents_$DATE.tar.gz" \
  -C "$BACKUP_DIR" current/

# Keep only last 7 daily backups
find $BACKUP_DIR -name "documents_*.tar.gz" -mtime +7 -delete
```

### S3 Storage Backup

```bash
#!/bin/bash
# backup-s3.sh

# Sync S3 bucket to another bucket
aws s3 sync s3://readur-documents s3://readur-backup \
  --delete \
  --storage-class GLACIER_IR

# Or to local storage
aws s3 sync s3://readur-documents /backup/s3-documents \
  --delete
```

### Deduplication Strategy

```bash
#!/bin/bash
# dedup-backup.sh

# Use restic for deduplication
restic -r /backup/restic init

# Backup with deduplication
restic -r /backup/restic backup \
  /data/readur/documents \
  --tag documents \
  --host readur-server

# Prune old snapshots
restic -r /backup/restic forget \
  --keep-daily 7 \
  --keep-weekly 4 \
  --keep-monthly 12 \
  --prune
```

## Configuration Backup

### Environment and Settings

```bash
#!/bin/bash
# backup-config.sh

CONFIG_DIR="/etc/readur"
BACKUP_DIR="/backup/config"
DATE=$(date +%Y%m%d_%H%M%S)

# Create config archive
tar -czf "$BACKUP_DIR/config_$DATE.tar.gz" \
  $CONFIG_DIR/.env \
  $CONFIG_DIR/docker-compose.yml \
  $CONFIG_DIR/nginx.conf \
  /etc/ssl/certs/readur* \
  /etc/systemd/system/readur*

# Encrypt sensitive configuration
gpg --encrypt --recipient backup@company.com \
  "$BACKUP_DIR/config_$DATE.tar.gz"

# Remove unencrypted file
rm "$BACKUP_DIR/config_$DATE.tar.gz"
```

## Automated Backup Solution

### Complete Backup Script

```bash
#!/bin/bash
# readur-backup.sh

set -e

# Configuration
BACKUP_ROOT="/backup"
S3_BUCKET="s3://company-backups/readur"
SLACK_WEBHOOK="https://hooks.slack.com/services/XXX"
DATE=$(date +%Y%m%d_%H%M%S)

# Functions
log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1"
}

notify() {
    curl -X POST -H 'Content-type: application/json' \
      --data "{\"text\":\"$1\"}" $SLACK_WEBHOOK
}

# Create backup directories
mkdir -p "$BACKUP_ROOT"/{database,documents,config,logs}

# 1. Database backup
log "Starting database backup..."
docker-compose exec -T postgres pg_dump -U readur readur | \
  gzip > "$BACKUP_ROOT/database/readur_$DATE.sql.gz"

# 2. Documents backup (if local storage)
if [ "$STORAGE_BACKEND" = "local" ]; then
    log "Starting documents backup..."
    rsync -avz --delete \
      /data/readur/documents/ \
      "$BACKUP_ROOT/documents/current/"
    
    tar -czf "$BACKUP_ROOT/documents/documents_$DATE.tar.gz" \
      -C "$BACKUP_ROOT/documents" current/
fi

# 3. Configuration backup
log "Starting configuration backup..."
tar -czf "$BACKUP_ROOT/config/config_$DATE.tar.gz" \
  .env docker-compose.yml

# 4. Upload to S3
log "Uploading to S3..."
aws s3 sync "$BACKUP_ROOT" "$S3_BUCKET" \
  --exclude "*/current/*" \
  --storage-class STANDARD_IA

# 5. Cleanup old backups
log "Cleaning up old backups..."
find "$BACKUP_ROOT/database" -name "*.sql.gz" -mtime +7 -delete
find "$BACKUP_ROOT/documents" -name "*.tar.gz" -mtime +7 -delete
find "$BACKUP_ROOT/config" -name "*.tar.gz" -mtime +30 -delete

# 6. Verify backup
BACKUP_SIZE=$(du -sh "$BACKUP_ROOT" | cut -f1)
log "Backup completed. Total size: $BACKUP_SIZE"

# 7. Send notification
notify "Readur backup completed successfully. Size: $BACKUP_SIZE"
```

### Cron Schedule

```bash
# /etc/crontab
# Hourly database backup
0 * * * * root /opt/readur/scripts/backup-database.sh

# Daily full backup at 2 AM
0 2 * * * root /opt/readur/scripts/readur-backup.sh

# Weekly configuration backup
0 3 * * 0 root /opt/readur/scripts/backup-config.sh
```

## Recovery Procedures

### Database Recovery

#### From pg_dump Backup

```bash
#!/bin/bash
# restore-database.sh

BACKUP_FILE="$1"

# Stop application
docker-compose stop readur

# Drop existing database
docker-compose exec postgres psql -U postgres -c "DROP DATABASE IF EXISTS readur;"
docker-compose exec postgres psql -U postgres -c "CREATE DATABASE readur OWNER readur;"

# Restore backup
gunzip -c "$BACKUP_FILE" | docker-compose exec -T postgres psql -U readur readur

# Run migrations
docker-compose exec readur alembic upgrade head

# Start application
docker-compose start readur
```

#### Point-in-Time Recovery

```bash
# Restore to specific time
recovery_target_time = '2024-01-15 14:30:00'

# Restore base backup
pg_basebackup -R -D /var/lib/postgresql/data

# Apply WAL logs
restore_command = 'cp /archive/%f %p'
recovery_target_time = '2024-01-15 14:30:00'
```

### Document Recovery

```bash
#!/bin/bash
# restore-documents.sh

BACKUP_FILE="$1"
TARGET_DIR="/data/readur/documents"

# Extract backup
tar -xzf "$BACKUP_FILE" -C /tmp/

# Restore with verification
rsync -avz --checksum \
  /tmp/current/ \
  "$TARGET_DIR/"

# Fix permissions
chown -R readur:readur "$TARGET_DIR"
chmod -R 755 "$TARGET_DIR"
```

### Full System Recovery

```bash
#!/bin/bash
# disaster-recovery.sh

set -e

# 1. Install Docker and dependencies
apt-get update
apt-get install -y docker.io docker-compose

# 2. Restore configuration
gpg --decrypt config_backup.tar.gz.gpg | tar -xzf - -C /etc/readur/

# 3. Pull Docker images
docker-compose pull

# 4. Restore database
gunzip -c database_backup.sql.gz | \
  docker-compose exec -T postgres psql -U readur

# 5. Restore documents
tar -xzf documents_backup.tar.gz -C /data/readur/

# 6. Start services
docker-compose up -d

# 7. Verify
curl -f http://localhost:8000/health || exit 1

echo "Recovery completed successfully"
```

## Backup Verification

### Automated Testing

```bash
#!/bin/bash
# verify-backup.sh

# Test database backup
TEST_DB="readur_test"

# Create test database
createdb $TEST_DB

# Restore backup to test database
gunzip -c "$1" | psql $TEST_DB

# Verify data integrity
RECORD_COUNT=$(psql -t -c "SELECT COUNT(*) FROM documents" $TEST_DB)
echo "Restored $RECORD_COUNT documents"

# Cleanup
dropdb $TEST_DB
```

### Backup Monitoring

```python
#!/usr/bin/env python3
# monitor-backups.py

import os
import time
from datetime import datetime, timedelta
import smtplib
from email.mime.text import MIMEText

BACKUP_DIR = "/backup"
MAX_AGE_HOURS = 25  # Alert if backup older than 25 hours

def check_backup_age(directory):
    latest_backup = None
    latest_time = 0
    
    for file in os.listdir(directory):
        if file.endswith('.gz'):
            file_time = os.path.getmtime(os.path.join(directory, file))
            if file_time > latest_time:
                latest_time = file_time
                latest_backup = file
    
    if latest_backup:
        age = time.time() - latest_time
        return latest_backup, age / 3600  # Age in hours
    return None, float('inf')

def send_alert(message):
    msg = MIMEText(message)
    msg['Subject'] = 'Readur Backup Alert'
    msg['From'] = 'monitor@company.com'
    msg['To'] = 'admin@company.com'
    
    s = smtplib.SMTP('localhost')
    s.send_message(msg)
    s.quit()

# Check each backup type
for backup_type in ['database', 'documents', 'config']:
    dir_path = os.path.join(BACKUP_DIR, backup_type)
    filename, age_hours = check_backup_age(dir_path)
    
    if age_hours > MAX_AGE_HOURS:
        send_alert(f"WARNING: {backup_type} backup is {age_hours:.1f} hours old")
    else:
        print(f"OK: {backup_type} backup is {age_hours:.1f} hours old")
```

## Cloud Backup Solutions

### AWS Backup Integration

```yaml
# CloudFormation template
Resources:
  BackupPlan:
    Type: AWS::Backup::BackupPlan
    Properties:
      BackupPlan:
        BackupPlanName: ReadurBackupPlan
        BackupPlanRule:
          - RuleName: DailyBackups
            TargetBackupVault: Default
            ScheduleExpression: "cron(0 5 ? * * *)"
            StartWindowMinutes: 60
            CompletionWindowMinutes: 120
            Lifecycle:
              DeleteAfterDays: 30
              MoveToColdStorageAfterDays: 7
```

### Backup to Multiple Destinations

```bash
#!/bin/bash
# multi-destination-backup.sh

BACKUP_FILE="readur_$(date +%Y%m%d).tar.gz"

# Local backup
cp "$BACKUP_FILE" /mnt/nas/backups/

# AWS S3
aws s3 cp "$BACKUP_FILE" s3://backup-bucket/

# Google Cloud Storage
gsutil cp "$BACKUP_FILE" gs://backup-bucket/

# Azure Blob Storage
az storage blob upload \
  --container-name backups \
  --name "$BACKUP_FILE" \
  --file "$BACKUP_FILE"
```

## Best Practices

### Security

1. **Encrypt backups** at rest and in transit
2. **Test recovery** procedures regularly
3. **Store backups** in multiple locations
4. **Rotate credentials** used for backup access
5. **Monitor backup** success and failures

### Testing

1. **Monthly recovery drills** to test procedures
2. **Quarterly full recovery** to separate environment
3. **Annual disaster recovery** exercise
4. **Document lessons learned** and update procedures

### Documentation

Maintain documentation for:
- Backup schedules and retention policies
- Recovery procedures and contact information
- RTO/RPO requirements
- Backup verification procedures
- Encryption keys and access credentials

## Related Documentation

- [Storage Configuration](./storage.md)
- [Migration Guide](../migration-guide.md)
- [Security Best Practices](../security-guide.md)
- [Monitoring Setup](./monitoring.md)