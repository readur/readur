# Upgrading PostgreSQL

PostgreSQL major version upgrades (e.g., 15 → 16) require a database migration. The internal storage format changes between major versions, so you cannot simply change the image tag.

> **Note:** Minor version upgrades (e.g., 16.4 → 16.8) are safe and only require changing the image tag and restarting the container.

## Prerequisites

- Familiarity with the command line and Docker
- Access to your `docker-compose.yml` file and data directories
- Sufficient disk space for the database backup (at least 2x your current database size)

## Step-by-Step Upgrade Guide

### 1. Stop the Readur application container

Prevent new data writes during the migration:

```bash
docker compose stop readur
```

### 2. Create a full backup

This is a critical safety step. Create a SQL dump of your PostgreSQL database:

```bash
# Find your PostgreSQL container name
docker ps --format "{{.Names}}" | grep postgres

# Create a SQL dump (replace 'readur-postgres' with your container name)
docker exec -t readur-postgres pg_dumpall -U readur > readur_backup.sql

# Verify the dump file exists and has content
ls -lh readur_backup.sql
head -50 readur_backup.sql  # Preview the dump to ensure it's valid
```

### 3. Stop the PostgreSQL container

```bash
docker compose stop postgres
```

### 4. Back up the current database data directory

This provides an additional safety net:

```bash
# If using named volume (recommended)
docker volume create postgres_data_backup
docker run --rm -v postgres_data:/source -v postgres_data_backup:/backup alpine cp -a /source/. /backup/

# If using host path (e.g., ./postgres or /volume1/docker/readur/postgres)
mv ./postgres ./postgres_old_backup
```

### 5. Update docker-compose.yml

Change the PostgreSQL version to your target version:

```yaml
# Before
image: postgres:15

# After
image: postgres:16
```

### 6. Remove the old data volume

The new PostgreSQL version needs a fresh data directory:

```bash
# If using named volume
docker volume rm postgres_data

# If using host path, it was already moved in step 4
```

### 7. Start the new PostgreSQL container

Docker will create a new, empty data directory:

```bash
docker compose up -d postgres

# Wait for it to be healthy
docker compose ps
```

### 8. Import the database dump

Restore your data into the new PostgreSQL instance:

```bash
docker exec -i readur-postgres psql -U readur -d postgres < readur_backup.sql
```

> **Note:** If you get "the input device is not a TTY" error, use `-T` instead of `-i`:
> ```bash
> docker exec -T readur-postgres psql -U readur -d postgres < readur_backup.sql
> ```

### 9. Start Readur

```bash
docker compose up -d
```

### 10. Verify the upgrade

- Check logs for errors: `docker compose logs -f readur`
- Confirm all documents are accessible in the web UI
- Test search functionality
- Verify OCR processing works on a test document

### 11. Clean up

**Only after confirming everything works properly:**

```bash
# Remove old data backup
rm -rf ./postgres_old_backup

# Or for named volumes:
docker volume rm postgres_data_backup

# Remove dump file
rm readur_backup.sql
```

## Troubleshooting

| Error | Solution |
|-------|----------|
| `FATAL: database files are incompatible with server` | You tried to use old data with new postgres. Remove the data volume and follow the migration steps above. |
| `role "readur" does not exist` | The dump didn't restore properly. Re-run step 8. |
| `could not connect to server` | Wait for postgres healthcheck to pass. Check `docker compose logs postgres`. |
| `permission denied` | Ensure the postgres container has write access to the data directory. Use named volumes. |

## Alternative Methods

### Using pg_upgrade (Advanced)

For large databases where dump/restore takes too long, you can use [tianon/docker-postgres-upgrade](https://github.com/tianon/docker-postgres-upgrade):

```bash
docker run --rm \
  -v postgres_data_old:/var/lib/postgresql/15/data \
  -v postgres_data_new:/var/lib/postgresql/16/data \
  tianon/postgres-upgrade:15-to-16 --link
```

This is faster but more complex. See the project documentation for details.

### Using pgautoupgrade (Automatic)

The [pgautoupgrade/docker-pgautoupgrade](https://github.com/pgautoupgrade/docker-pgautoupgrade) image automatically detects and upgrades your database:

```yaml
postgres:
  image: pgautoupgrade/pgautoupgrade:16-alpine
```

**Warning:** This performs in-place upgrades. Always have backups before using this approach.

## Further Reading

- [Backup and Recovery Guide](backup-recovery.md)
- [Deployment Guide](deployment.md)
- [PostgreSQL Official Upgrade Documentation](https://www.postgresql.org/docs/current/upgrading.html)
