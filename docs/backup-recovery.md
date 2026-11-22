# Backup and Recovery Guide

Readur has no built-in backup feature. Everything it stores is in two places, and both are backed up with standard tools:

1. **The PostgreSQL database**: users, document records and extracted text, labels, sources (including their credentials), settings, API keys, shared links, the OCR queue, and the generated JWT signing key when `JWT_SECRET` is not set.
2. **The document files**: with local storage, the directory in `UPLOAD_PATH` (`/app/uploads` in the Docker image); with S3 storage, the bucket in `S3_BUCKET_NAME` (below `S3_PREFIX` if set).

Also keep a copy of the environment you run Readur with (compose file, `.env`, Helm values), since secrets such as `JWT_SECRET`, OIDC client secrets and S3 credentials live there and not in the database. The watch folder (`WATCH_FOLDER`) holds the files you dropped in; Readur copies them into its storage and never changes or deletes the originals, so backing it up is optional.

For a Docker Compose walk-through with a ready-made script, see [Backup Strategies](self-hosting/backup.md).

## Database Backup

### pg_dump

`pg_dump` takes a consistent snapshot while Readur keeps running:

```bash
pg_dump "postgresql://readur:password@localhost:5432/readur" -Fc -f readur-$(date +%Y%m%d-%H%M%S).dump
```

Use the same host, database, user and password as Readur (`DATABASE_URL`, or the `POSTGRES_*` variables). To avoid a password on the command line, use a `~/.pgpass` file or `PGPASSWORD`.

The custom format (`-Fc`) is compressed and lets `pg_restore` restore selectively. A plain SQL dump (`-Fp`, restored with `psql`) works too.

### Scheduling

```bash
# crontab: daily dump at 02:00, keep 14 days
0 2 * * * pg_dump "postgresql://readur@localhost/readur" -Fc -f /backup/readur/db-$(date +\%Y\%m\%d).dump && find /backup/readur -name 'db-*.dump' -mtime +14 -delete
```

(`%` must be escaped as `\%` inside a crontab.)

### Point-in-time recovery

If losing up to a day of changes is too much, use PostgreSQL's continuous archiving (WAL archiving with a base backup) or a tool built on it such as pgBackRest, Barman or WAL-G, or the backup feature of your managed database service. Readur uses an ordinary PostgreSQL database and needs nothing special from these tools. See the PostgreSQL documentation on continuous archiving for the setup.

## File Storage Backup

### Local storage

The upload directory contains:

| Path | Contents |
|------|----------|
| `documents/` | The stored documents |
| `thumbnails/` | Generated thumbnails |
| `processed_images/` | Images produced during OCR preprocessing |
| `temp/` | Files in transit; does not need backing up |
| `.readur/initial-admin-password` | The generated initial admin password, if one was generated |

Back up the whole directory with rsync, tar, restic, borg or a filesystem snapshot:

```bash
rsync -a --delete --exclude temp/ /srv/readur/uploads/ /backup/readur/uploads/
```

```bash
tar -czf /backup/readur/uploads-$(date +%Y%m%d).tar.gz --exclude ./temp -C /srv/readur/uploads .
```

### S3 storage

Readur stores object keys in the database (as `s3://...` paths) and does not copy the bucket anywhere. Use the bucket's own protection, such as versioning, replication or provider snapshots, or copy it with a generic tool:

```bash
rclone sync s3remote:readur-documents /backup/readur/s3
```

## Keeping Database and Files Consistent

The database records where each file is stored. A database restored together with files from a different point in time can reference files that do not exist (those documents fail to open or download) or leave files that no document references (harmless, they only take space).

- Dump the database **before** copying the files, so the files are at least as new as the database.
- For a fully consistent pair, stop the Readur server (PostgreSQL can keep running) while you take both, then start it again.

## Verifying Backups

A backup you have never restored is a guess. At a minimum, check that each dump is readable:

```bash
pg_restore --list readur-20261117.dump > /dev/null && echo "dump OK"
```

Better, restore into a scratch database and count rows:

```bash
createdb -U readur readur_restore_test
pg_restore -U readur -d readur_restore_test --no-owner readur-20261117.dump
psql -U readur -d readur_restore_test -c "SELECT COUNT(*) FROM documents;"
psql -U readur -d readur_restore_test -c "SELECT COUNT(*) FROM users;"
dropdb -U readur readur_restore_test
```

The best test is a full restore on a separate machine: start Readur against the restored database and files, log in, and open a few documents.

## Recovery Procedures

### Full database restore

1. Stop the Readur server so nothing writes to the database.
2. Recreate the database and restore the dump:

   ```bash
   dropdb -U readur --if-exists readur
   createdb -U readur -O readur readur
   pg_restore -U readur -d readur --no-owner readur-20261117.dump
   ```

   The schema needs the `pg_trgm` extension, which the dump recreates. The restoring role must be allowed to create it (a superuser, or the database owner on PostgreSQL 13 and later, where `pg_trgm` is a trusted extension).

3. Restore the matching document files (below).
4. Start Readur. It checks the schema and applies any migrations the restored database is missing, then logs `SQLx migrations completed successfully`.

A dump taken from an older Readur version can be restored under a newer one; the missing migrations are applied at startup. An older Readur does not start on a database that a newer version has already migrated, so to go back to an older version, restore a dump taken before the upgrade.

### File restore

Local storage: put the backed-up directory back in place of `UPLOAD_PATH`:

```bash
rsync -a --delete /backup/readur/uploads/ /srv/readur/uploads/
```

S3 storage: restore the objects to the same bucket and prefix (from bucket versions, a replica, or your copy), so the keys stored in the database resolve again.

### Restoring onto a new server

1. Install Readur as described in the [Installation Guide](installation.md), with the same configuration as before (in particular `JWT_SECRET` if you set one, and the same storage backend settings), but do not start it yet.
2. Restore the database into the new PostgreSQL server.
3. Restore the files into the new `UPLOAD_PATH` or bucket.
4. Start Readur and check `GET /api/health`, then log in and open a few documents.

If `JWT_SECRET` was not set, the signing key is in the database and comes along with it. If the key differs from before, existing login sessions end and users log in again; API keys keep working because they are not signed with it.

### Lost admin password

Not a backup matter, but often asked for during recovery: reset it with the `readur reset-admin-password` command (see [CLI Tools](administration/cli-tools.md)).

## Monitoring Backups

Readur does not know about your backups, so watch them from the backup side: alert when the newest dump is older than expected, or when the backup job exits with an error. Readur's own health and metrics endpoints are described in [Monitoring](self-hosting/monitoring.md).

## Checklist

- The database is dumped regularly, and the dumps are copied off the server.
- The upload directory or bucket is backed up on the same schedule, after the database.
- The compose file, `.env` or Helm values are backed up, with their secrets stored securely.
- Backups are encrypted or kept on storage with restricted access.
- A restore has actually been tested, ideally on a separate machine.
