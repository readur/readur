# Backup Strategies

Readur has no backup command or backup API. Its state lives in a few places, and you back them up with the usual tools: `pg_dump` for PostgreSQL and a file copy (or bucket copy) for the documents. This page shows how to do that for a Docker Compose installation; [Backup and Recovery](../backup-recovery.md) covers the same steps without Docker, plus restore details.

## What to Back Up

| What | Where | Why |
|------|-------|-----|
| PostgreSQL database | the `postgres_data` volume in the bundled `docker-compose.yml`, or your external database | Users, document records, extracted text, labels, sources and their credentials, settings, API keys, the OCR queue, and the generated JWT signing key (when `JWT_SECRET` is not set) |
| Document files (local storage) | `UPLOAD_PATH`: `/app/uploads` in the container, `./readur_uploads` on the host with the bundled compose file | The original files (`documents/`), thumbnails (`thumbnails/`) and processed OCR images (`processed_images/`). Also holds `.readur/initial-admin-password` when the initial admin password was generated |
| Document files (S3 storage) | the bucket named by `S3_BUCKET_NAME`, below `S3_PREFIX` if set | The same files, when the S3 backend is enabled |
| Your configuration | `docker-compose.yml`, `.env`, Helm values, reverse proxy config | The environment variables Readur runs with, including `JWT_SECRET`, OIDC client secrets and S3 credentials |
| Watch folder (optional) | `WATCH_FOLDER`: `/app/watch` in the container | Readur copies files out of the watch folder and never changes or deletes them there, so these are your originals. Back them up if Readur's copy is not enough for you |

The database and the document files belong together: the database records where every file is stored, so restore them as a pair taken at (nearly) the same time.

Treat the backups as sensitive. The database contains password hashes, API key hashes, source credentials (WebDAV, S3) and possibly the JWT signing key; the files are your documents.

## Database

With the bundled compose file, run `pg_dump` inside the `postgres` service and write the dump to the host:

```bash
docker compose exec -T postgres pg_dump -U readur -d readur -Fc > readur-db-$(date +%Y%m%d-%H%M%S).dump
```

`-Fc` produces PostgreSQL's compressed custom format, which `pg_restore` reads. `pg_dump` takes a consistent snapshot while Readur keeps running, so there is no need to stop anything for the database alone.

If your database runs elsewhere, run `pg_dump` against it with the same connection details as `DATABASE_URL`:

```bash
pg_dump "postgresql://readur:password@db.example.com:5432/readur" -Fc -f readur-db.dump
```

For continuous archiving and point-in-time recovery, use PostgreSQL's own WAL archiving or a tool built on it (pgBackRest, Barman, WAL-G). Readur needs nothing special from them.

## Document Files

### Local storage

Copy the upload directory. With the bundled compose file it is the `./readur_uploads` bind mount next to `docker-compose.yml`:

```bash
tar -czf readur-uploads-$(date +%Y%m%d-%H%M%S).tar.gz -C ./readur_uploads .
```

or incrementally:

```bash
rsync -a --delete ./readur_uploads/ /backup/readur/uploads/
```

The `temp/` subdirectory only holds files in transit and can be excluded (`--exclude temp/`).

If you mounted a named volume instead of a host directory, archive it through a throwaway container:

```bash
docker run --rm -v readur_uploads:/data:ro -v "$PWD":/backup alpine \
  tar -czf /backup/readur-uploads.tar.gz -C /data .
```

(The volume name is prefixed with the compose project name; `docker volume ls` shows it.)

### S3 storage

Readur does not copy the bucket anywhere. Protect it with the features of your storage provider, for example bucket versioning or replication, or copy it with a generic tool:

```bash
rclone sync s3remote:readur-documents /backup/readur/s3
```

## Consistency

Readur keeps writing files and database rows while it runs (uploads, watch folder imports, source syncs, OCR). For a backup you can restore without surprises:

1. Dump the database first, then copy the files. Files that appear after the dump are harmless extras; a database row whose file is missing is not.
2. For a fully consistent backup, stop Readur (not PostgreSQL) for the duration: `docker compose stop readur`, back up, `docker compose start readur`.

## Example Script

A complete nightly backup for the bundled compose setup. Run it from the directory that contains `docker-compose.yml`.

```bash
#!/usr/bin/env bash
# readur-backup.sh: back up the Readur database and uploads
set -euo pipefail

backup_root=/backup/readur
keep_days=14
stamp=$(date +%Y%m%d-%H%M%S)
target="$backup_root/$stamp"

mkdir -p "$target"

# 1. Database first
docker compose exec -T postgres pg_dump -U readur -d readur -Fc > "$target/readur-db.dump"

# 2. Then the uploaded documents (local storage)
tar -czf "$target/readur-uploads.tar.gz" --exclude ./temp -C ./readur_uploads .

# 3. The configuration
cp docker-compose.yml "$target/"
if [ -f .env ]; then cp .env "$target/"; fi

# Quick sanity check: the dump must be readable by pg_restore
docker compose exec -T postgres pg_restore --list < "$target/readur-db.dump" > /dev/null

# Remove backups older than keep_days
find "$backup_root" -mindepth 1 -maxdepth 1 -type d -mtime +"$keep_days" -exec rm -rf {} +

echo "Backup written to $target"
```

Schedule it with cron:

```bash
# crontab -e
30 2 * * * cd /opt/readur && ./readur-backup.sh >> /var/log/readur-backup.log 2>&1
```

Copy the backup directory off the machine (another host, a NAS, object storage); a backup on the same disk does not survive a disk failure.

## Restoring with Docker Compose

1. Stop Readur, keep PostgreSQL running:

   ```bash
   docker compose stop readur
   ```

2. Recreate the database and load the dump:

   ```bash
   docker compose exec -T postgres dropdb -U readur --if-exists readur
   docker compose exec -T postgres createdb -U readur -O readur readur
   docker compose exec -T postgres pg_restore -U readur -d readur --no-owner < readur-db.dump
   ```

3. Put the files back:

   ```bash
   find ./readur_uploads -mindepth 1 -delete
   tar -xzf readur-uploads.tar.gz -C ./readur_uploads
   ```

4. Start Readur and check that it comes up:

   ```bash
   docker compose start readur
   docker compose logs -f readur
   curl -f http://localhost:8000/api/health
   ```

On startup Readur applies any database migrations the dump does not have yet, so a dump from an older Readur version can be restored under a newer one. The reverse does not work: an older Readur refuses to start on a database that a newer version has already migrated.

If `JWT_SECRET` is set, restore it with the same value; otherwise the signing key comes back with the database. Either way, users who were logged in may need to log in again if the key changed.

## Kubernetes (Helm Chart)

The Helm chart does not include a database; back up the PostgreSQL server you point it at with `pg_dump` or your operator's backup feature. Documents live on the `uploads` PersistentVolumeClaim (mounted at `/app/uploads`) unless S3 is enabled; back it up with volume snapshots or a tool such as Velero.

## Related Documentation

- [Backup and Recovery](../backup-recovery.md)
- [Storage Configuration](./storage.md)
- [Migration Guide](../migration-guide.md)
- [Upgrading PostgreSQL](../postgres-upgrade.md)
- [Security Best Practices](../security-guide.md)
