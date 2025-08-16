# Migration Guide

This comprehensive guide covers all migration scenarios for Readur, including version upgrades, storage migrations, database migrations, and platform migrations.

## Version Upgrades

### Upgrading from v2.x to v3.x

#### Pre-Upgrade Checklist

- [ ] Review breaking changes in release notes
- [ ] Backup database and files
- [ ] Test upgrade in staging environment
- [ ] Schedule maintenance window
- [ ] Notify users of planned downtime

#### Upgrade Steps

1. **Stop the application**
```bash
docker-compose down
# or
systemctl stop readur
```

2. **Backup current state**
```bash
# Database backup
pg_dump $DATABASE_URL > backup_v2_$(date +%Y%m%d).sql

# File backup
tar -czf files_backup_v2_$(date +%Y%m%d).tar.gz /var/readur/uploads
```

3. **Update configuration**
```bash
# New environment variables in v3
echo "OIDC_ENABLED=false" >> .env
echo "FEATURE_MULTI_LANGUAGE_OCR=true" >> .env
echo "FEATURE_LABELS=true" >> .env
```

4. **Run database migrations**
```bash
# Pull new version
docker pull readur:v3.0.0

# Run migrations
docker run --rm \
  -e DATABASE_URL=$DATABASE_URL \
  readur:v3.0.0 \
  cargo run --bin migrate
```

5. **Update and start application**
```bash
# Update docker-compose.yml
sed -i 's/readur:v2/readur:v3.0.0/g' docker-compose.yml

# Start application
docker-compose up -d
```

6. **Verify upgrade**
```bash
# Check version
curl http://localhost:8080/api/version

# Check health
curl http://localhost:8080/health

# Verify migrations
psql $DATABASE_URL -c "SELECT * FROM _sqlx_migrations ORDER BY installed_on DESC LIMIT 5;"
```

### Rollback Procedure

If issues occur during upgrade:

```bash
# Stop v3
docker-compose down

# Restore database
psql $DATABASE_URL < backup_v2_$(date +%Y%m%d).sql

# Restore configuration
git checkout v2.x.x docker-compose.yml
git checkout v2.x.x .env

# Start v2
docker-compose up -d
```

## Storage Migration

### Local to S3 Migration

#### Prerequisites

- [ ] S3 bucket created and accessible
- [ ] IAM credentials with appropriate permissions
- [ ] Sufficient network bandwidth
- [ ] Migration tool installed: `cargo install --path . --bin migrate_to_s3`

#### Step 1: Prepare S3 Environment

```bash
# Test S3 access
aws s3 ls s3://readur-documents/

# Create bucket structure
aws s3api put-object --bucket readur-documents --key documents/
aws s3api put-object --bucket readur-documents --key thumbnails/
aws s3api put-object --bucket readur-documents --key processed/
```

#### Step 2: Configure S3 Settings

```yaml
# Update environment variables
S3_ENABLED: true
S3_BUCKET_NAME: readur-documents
S3_REGION: us-east-1
S3_ACCESS_KEY_ID: AKIAIOSFODNN7EXAMPLE
S3_SECRET_ACCESS_KEY: wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY
S3_PREFIX: production/
S3_STORAGE_CLASS: STANDARD_IA
```

#### Step 3: Run Migration

```bash
# Dry run first
./migrate_to_s3 --dry-run

# Expected output:
# Files to migrate: 5,432
# Total size: 45.6 GB
# Estimated time: 2.5 hours

# Run actual migration with progress tracking
./migrate_to_s3 \
  --enable-rollback \
  --batch-size 100 \
  --parallel-uploads 10 \
  --verbose \
  2>&1 | tee migration_$(date +%Y%m%d).log
```

#### Step 4: Verify Migration

```bash
# Check file count
aws s3 ls s3://readur-documents/documents/ --recursive | wc -l

# Verify random samples
./migrate_to_s3 --verify --sample-size 100

# Check database references
psql $DATABASE_URL <<EOF
SELECT COUNT(*) FROM documents WHERE file_path LIKE 's3://%';
SELECT COUNT(*) FROM documents WHERE file_path NOT LIKE 's3://%';
EOF
```

#### Step 5: Switch to S3 Storage

```bash
# Update configuration
docker-compose down
vim docker-compose.yml  # Set S3_ENABLED=true
docker-compose up -d

# Test file upload
curl -X POST http://localhost:8080/api/documents/upload \
  -H "Authorization: Bearer $TOKEN" \
  -F "file=@test.pdf"
```

### S3 to Local Migration

```bash
# Reverse migration if needed
./migrate_to_s3 \
  --direction s3-to-local \
  --enable-rollback \
  --verify
```

### Cross-Region S3 Migration

```bash
#!/bin/bash
# migrate-s3-regions.sh

SOURCE_BUCKET="readur-us-east-1"
DEST_BUCKET="readur-eu-west-1"
SOURCE_REGION="us-east-1"
DEST_REGION="eu-west-1"

# Set up replication
aws s3api put-bucket-replication \
  --bucket $SOURCE_BUCKET \
  --replication-configuration file://replication.json

# Wait for replication
aws s3api get-bucket-replication-status \
  --bucket $SOURCE_BUCKET

# Update application configuration
sed -i "s/$SOURCE_BUCKET/$DEST_BUCKET/g" .env
sed -i "s/$SOURCE_REGION/$DEST_REGION/g" .env

# Restart application
docker-compose restart
```

## Database Migrations

### PostgreSQL Version Upgrade

#### Upgrading from PostgreSQL 14 to 15

```bash
# 1. Dump database
pg_dumpall -h old_host -U postgres > dump.sql

# 2. Stop old PostgreSQL
systemctl stop postgresql-14

# 3. Install PostgreSQL 15
apt-get install postgresql-15

# 4. Initialize new cluster
/usr/lib/postgresql/15/bin/initdb -D /var/lib/postgresql/15/data

# 5. Restore database
psql -h new_host -U postgres < dump.sql

# 6. Update connection string
export DATABASE_URL="postgresql://readur:password@localhost:5432/readur?sslmode=require"

# 7. Test connection
psql $DATABASE_URL -c "SELECT version();"
```

### Database Server Migration

```bash
#!/bin/bash
# migrate-database.sh

OLD_DB="postgresql://user:pass@old-server/readur"
NEW_DB="postgresql://user:pass@new-server/readur"

# 1. Create new database
psql $NEW_DB -c "CREATE DATABASE readur;"

# 2. Dump schema and data
pg_dump $OLD_DB --no-owner --clean --if-exists > readur_dump.sql

# 3. Restore to new server
psql $NEW_DB < readur_dump.sql

# 4. Verify data
psql $NEW_DB <<EOF
SELECT COUNT(*) as documents FROM documents;
SELECT COUNT(*) as users FROM users;
SELECT MAX(created_at) as latest FROM documents;
EOF

# 5. Update application configuration
export DATABASE_URL=$NEW_DB

# 6. Test application
curl http://localhost:8080/health
```

### Schema Migrations

#### Running Pending Migrations

```bash
# Check migration status
cargo run --bin migrate status

# Run pending migrations
cargo run --bin migrate up

# Rollback last migration
cargo run --bin migrate down
```

#### Creating Custom Migrations

```sql
-- migrations/20250116_custom_index.up.sql
CREATE INDEX CONCURRENTLY idx_documents_created_month 
ON documents(date_trunc('month', created_at));

-- migrations/20250116_custom_index.down.sql
DROP INDEX IF EXISTS idx_documents_created_month;
```

## Platform Migration

### Docker to Kubernetes

#### Step 1: Prepare Kubernetes Manifests

```yaml
# deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: readur
spec:
  replicas: 3
  selector:
    matchLabels:
      app: readur
  template:
    metadata:
      labels:
        app: readur
    spec:
      containers:
      - name: readur
        image: readur:latest
        envFrom:
        - configMapRef:
            name: readur-config
        - secretRef:
            name: readur-secrets
        volumeMounts:
        - name: uploads
          mountPath: /app/uploads
      volumes:
      - name: uploads
        persistentVolumeClaim:
          claimName: readur-uploads
```

#### Step 2: Migrate Configuration

```bash
# Create ConfigMap from .env
kubectl create configmap readur-config --from-env-file=.env

# Create Secrets
kubectl create secret generic readur-secrets \
  --from-literal=DATABASE_URL=$DATABASE_URL \
  --from-literal=JWT_SECRET=$JWT_SECRET \
  --from-literal=S3_SECRET_ACCESS_KEY=$S3_SECRET_ACCESS_KEY
```

#### Step 3: Migrate Storage

```yaml
# persistent-volume.yaml
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: readur-uploads
spec:
  accessModes:
    - ReadWriteMany
  resources:
    requests:
      storage: 100Gi
  storageClassName: nfs-storage
```

#### Step 4: Deploy to Kubernetes

```bash
# Apply manifests
kubectl apply -f deployment.yaml
kubectl apply -f service.yaml
kubectl apply -f ingress.yaml

# Verify deployment
kubectl get pods -l app=readur
kubectl logs -l app=readur --tail=50
```

### On-Premise to Cloud

#### AWS Migration

```bash
# 1. Set up RDS PostgreSQL
aws rds create-db-instance \
  --db-instance-identifier readur-prod \
  --db-instance-class db.t3.medium \
  --engine postgres \
  --engine-version 15.2 \
  --allocated-storage 100 \
  --master-username readur \
  --master-user-password $DB_PASSWORD

# 2. Migrate database
pg_dump $LOCAL_DATABASE_URL | \
  psql postgresql://readur:$DB_PASSWORD@readur-prod.xyz.rds.amazonaws.com/readur

# 3. Set up ECS/Fargate
aws ecs create-cluster --cluster-name readur-cluster

# 4. Create task definition
aws ecs register-task-definition \
  --cli-input-json file://task-definition.json

# 5. Create service
aws ecs create-service \
  --cluster readur-cluster \
  --service-name readur \
  --task-definition readur:1 \
  --desired-count 3
```

#### Azure Migration

```bash
# 1. Create Resource Group
az group create --name readur-rg --location eastus

# 2. Create PostgreSQL
az postgres server create \
  --resource-group readur-rg \
  --name readur-db \
  --admin-user readur \
  --admin-password $DB_PASSWORD \
  --sku-name B_Gen5_2

# 3. Create Container Instances
az container create \
  --resource-group readur-rg \
  --name readur \
  --image readur:latest \
  --cpu 2 \
  --memory 4 \
  --environment-variables \
    DATABASE_URL=$AZURE_DB_URL \
    S3_ENABLED=false
```

## Data Migration

### Bulk Document Import

```bash
#!/bin/bash
# bulk-import.sh

SOURCE_DIR="/legacy/documents"
USER_ID="admin"
BATCH_SIZE=1000

# Import with progress tracking
find $SOURCE_DIR -type f \( -name "*.pdf" -o -name "*.docx" \) | \
  xargs -P 4 -n $BATCH_SIZE \
  cargo run --bin batch_ingest -- \
    --user-id $USER_ID \
    --ocr-enabled \
    --skip-duplicates
```

### User Migration

```sql
-- Migrate users from legacy system
INSERT INTO users (id, username, email, password_hash, role, created_at)
SELECT 
  gen_random_uuid(),
  legacy_username,
  legacy_email,
  crypt(legacy_password, gen_salt('bf')),
  CASE 
    WHEN legacy_role = 'admin' THEN 'admin'
    WHEN legacy_role = 'power_user' THEN 'editor'
    ELSE 'viewer'
  END,
  legacy_created_date
FROM legacy_users;
```

### Metadata Migration

```python
# migrate_metadata.py
import psycopg2
import json

def migrate_metadata():
    # Connect to databases
    legacy_conn = psycopg2.connect("dbname=legacy")
    readur_conn = psycopg2.connect("dbname=readur")
    
    # Fetch legacy metadata
    legacy_cur = legacy_conn.cursor()
    legacy_cur.execute("SELECT file_id, metadata FROM legacy_documents")
    
    # Transform and insert
    readur_cur = readur_conn.cursor()
    for file_id, old_metadata in legacy_cur:
        new_metadata = transform_metadata(old_metadata)
        readur_cur.execute(
            "UPDATE documents SET metadata = %s WHERE legacy_id = %s",
            (json.dumps(new_metadata), file_id)
        )
    
    readur_conn.commit()

def transform_metadata(old):
    """Transform legacy metadata format"""
    return {
        "title": old.get("document_title"),
        "author": old.get("created_by"),
        "tags": old.get("keywords", "").split(","),
        "legacy_id": old.get("id")
    }
```

## Migration Validation

### Validation Checklist

```bash
#!/bin/bash
# validate-migration.sh

echo "=== Migration Validation ==="

# 1. Document count
echo -n "Document count match: "
OLD_COUNT=$(psql $OLD_DB -t -c "SELECT COUNT(*) FROM documents")
NEW_COUNT=$(psql $NEW_DB -t -c "SELECT COUNT(*) FROM documents")
[ "$OLD_COUNT" -eq "$NEW_COUNT" ] && echo "✓" || echo "✗"

# 2. User count
echo -n "User count match: "
OLD_USERS=$(psql $OLD_DB -t -c "SELECT COUNT(*) FROM users")
NEW_USERS=$(psql $NEW_DB -t -c "SELECT COUNT(*) FROM users")
[ "$OLD_USERS" -eq "$NEW_USERS" ] && echo "✓" || echo "✗"

# 3. File integrity
echo -n "File integrity check: "
MISSING=$(comm -23 \
  <(find /old/uploads -type f -name "*.pdf" | sort) \
  <(find /new/uploads -type f -name "*.pdf" | sort) | wc -l)
[ "$MISSING" -eq 0 ] && echo "✓" || echo "✗ ($MISSING missing)"

# 4. Search functionality
echo -n "Search working: "
RESULTS=$(curl -s "http://localhost:8080/api/search?q=test" | jq '.results | length')
[ "$RESULTS" -gt 0 ] && echo "✓" || echo "✗"

# 5. OCR queue
echo -n "OCR queue healthy: "
PENDING=$(psql $NEW_DB -t -c "SELECT COUNT(*) FROM ocr_queue WHERE status='pending'")
echo "$PENDING pending jobs"
```

## Rollback Procedures

### Emergency Rollback

```bash
#!/bin/bash
# emergency-rollback.sh

BACKUP_DATE=$1
if [ -z "$BACKUP_DATE" ]; then
    echo "Usage: $0 YYYYMMDD"
    exit 1
fi

echo "⚠️  WARNING: This will rollback to backup from $BACKUP_DATE"
read -p "Continue? (y/N) " -n 1 -r
echo

if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    exit 1
fi

# Stop application
docker-compose down

# Restore database
psql $DATABASE_URL < backup_${BACKUP_DATE}.sql

# Restore files
tar -xzf files_backup_${BACKUP_DATE}.tar.gz -C /

# Restore configuration
cp .env.backup_${BACKUP_DATE} .env

# Start application
docker-compose up -d

echo "✓ Rollback completed"
```

### Partial Rollback

```sql
-- Rollback specific migration
BEGIN;
DELETE FROM _sqlx_migrations WHERE version = '20250115100000';
-- Run down migration SQL here
COMMIT;
```

## Post-Migration Tasks

### Performance Optimization

```sql
-- Rebuild indexes
REINDEX DATABASE readur;

-- Update statistics
ANALYZE;

-- Vacuum database
VACUUM (FULL, ANALYZE);
```

### Security Audit

```bash
# Check permissions
psql $DATABASE_URL -c "\dp"

# Verify encryption
aws s3api get-bucket-encryption --bucket readur-documents

# Test authentication
curl -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"test"}'
```

### Monitoring Setup

```yaml
# Set up alerts for migrated system
- alert: HighErrorRate
  expr: rate(errors_total[5m]) > 0.05
  annotations:
    summary: "High error rate after migration"
    
- alert: SlowQueries
  expr: pg_stat_database_blks_hit_ratio < 0.95
  annotations:
    summary: "Database cache hit ratio low after migration"
```

## Migration Best Practices

1. **Always test in staging** before production migration
2. **Take comprehensive backups** before starting
3. **Document the process** for future reference
4. **Monitor closely** after migration
5. **Have a rollback plan** ready
6. **Communicate with users** about downtime
7. **Validate data integrity** at each step
8. **Keep migration logs** for troubleshooting
9. **Update documentation** after migration
10. **Plan for peak traffic** when coming back online