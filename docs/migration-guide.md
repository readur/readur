# Migration Guide

This page covers the migrations Readur actually supports: upgrading Readur itself, upgrading or moving PostgreSQL, moving local document storage to S3, moving to another server or platform, and bringing existing documents into Readur.

Before any of them, take a backup of the database and the document files (see [Backup and Recovery](backup-recovery.md)). Every rollback described below is "restore the backup".

## Upgrading Readur

### How schema migrations work

Database schema changes ship as SQL files in the `migrations/` directory of the source tree and are compiled into the `readur` binary. Every time the server starts, it compares them with the `_sqlx_migrations` table in the database and applies the missing ones in order, before it starts serving requests. There is no separate migration command to run and no way to skip or undo a migration.

The startup log lists each migration as `APPLIED` or `PENDING` and ends with `SQLx migrations completed successfully`. If a migration fails, the log shows `CRITICAL: SQLx migrations failed!` with the error, and the server exits; fix the cause (often database permissions or disk space) and start it again. See [Migration Troubleshooting](administration/migration-troubleshooting.md) for common problems.

Because migrations only go forward, a database that a newer Readur has migrated is not accepted by an older Readur: the older version refuses to start because the database contains migrations it does not know.

### Docker Compose

1. Read the release notes (`CHANGELOG.md` in the repository, or the GitHub release) for the versions you are skipping.
2. Back up the database and the upload directory.
3. Update and restart:

   If you run the published image (`ghcr.io/readur/readur`), set the new tag in `docker-compose.yml` (for example `ghcr.io/readur/readur:3.1.3`; `latest` follows releases, `main` follows the development branch), then:

   ```bash
   docker compose pull readur
   docker compose up -d readur
   ```

   If you build from the repository (the bundled `docker-compose.yml` uses `build: .`):

   ```bash
   git pull
   docker compose build readur
   docker compose up -d readur
   ```

4. Watch the startup log until the server is up, then check the health endpoint:

   ```bash
   docker compose logs -f readur
   curl -f http://localhost:8000/api/health
   ```

Pin a version tag rather than `latest` if you want upgrades to happen only when you decide.

### Helm

Set the image tag and upgrade the release:

```bash
helm upgrade readur ./charts/readur --reuse-values \
  --set controllers.main.containers.readur.image.tag=3.1.3
```

The new pod applies the migrations when it starts. The chart keeps the generated admin password and the `<release>-auth` Secret across upgrades (see [Helm Chart Values](configuration-reference.md#helm-chart-values)).

### Rolling back an upgrade

1. Stop the new version.
2. Restore the database dump taken before the upgrade (see [Full database restore](backup-recovery.md#full-database-restore)).
3. Start the previous version (previous image tag or git revision).

Documents added after the backup are not in the restored database; their files stay in storage without a database record.

## Upgrading PostgreSQL

Readur needs no changes when PostgreSQL gets a new major version, but the data directory has to be converted: a new PostgreSQL major version cannot start on the old one's data directory. The bundled `docker-compose.yml` uses `postgres:18`. Follow [Upgrading PostgreSQL](postgres-upgrade.md) for the dump-and-restore procedure with Docker.

## Moving to Another Database Server

1. Stop the Readur server.
2. Dump the old database and restore it on the new server:

   ```bash
   pg_dump "postgresql://readur:password@old-db.example.com:5432/readur" -Fc -f readur.dump
   createdb -h new-db.example.com -U readur -O readur readur
   pg_restore -h new-db.example.com -U readur -d readur --no-owner readur.dump
   ```

3. Point Readur at the new server with `DATABASE_URL` (or the `POSTGRES_HOST`, `POSTGRES_PORT`, `POSTGRES_DB`, `POSTGRES_USER` and `POSTGRES_PASSWORD` variables) and start it.

The schema uses the `pg_trgm` extension; the role that restores the dump must be allowed to create it.

## Moving Document Storage

### Local storage to S3

The `migrate_to_s3` tool uploads locally stored documents to S3 and rewrites their database records to point at the uploaded objects. It is a separate binary built from the source tree; the Docker image does not contain it. Full walk-through: [Storage Migration](administration/storage-migration.md).

1. Back up the database and the upload directory.
2. Stop the Readur server so that no new uploads land in local storage during the migration.
3. On a machine with the source tree, network access to the database and S3, and the upload directory mounted at the same `UPLOAD_PATH` the server used, set the same environment as the server plus the S3 settings:

   ```bash
   export DATABASE_URL=postgresql://readur:password@db.example.com:5432/readur
   export UPLOAD_PATH=/srv/readur/uploads
   export S3_ENABLED=true
   export S3_BUCKET_NAME=readur-documents
   export S3_ACCESS_KEY_ID=your-access-key
   export S3_SECRET_ACCESS_KEY=your-secret-key
   export S3_REGION=us-east-1
   # For MinIO and other S3-compatible services:
   export S3_ENDPOINT_URL=https://minio.example.com
   ```

4. See what would be migrated:

   ```bash
   cargo run --release --bin migrate_to_s3 -- --dry-run
   ```

5. Run it:

   ```bash
   cargo run --release --bin migrate_to_s3 -- --enable-rollback
   ```

6. Add the same S3 variables to the server's configuration and start it.

The tool's options (from `src/bin/migrate_to_s3.rs`):

| Option | Effect |
|--------|--------|
| `-d`, `--dry-run` | List the documents that would be migrated, change nothing |
| `--delete-local` | Delete each local file (and its thumbnail and processed image) after a successful upload |
| `-l`, `--limit <N>` | Migrate at most N documents |
| `-u`, `--user-id <UUID>` | Only migrate this user's documents |
| `--enable-rollback` | When a document fails, point every document migrated in this run back at its original local path, delete the uploaded objects and stop. This only helps while the local files still exist, so do not combine it with `--delete-local` |
| `--resume-from <DOCUMENT_ID>` | Start at this document (documents are taken oldest first) instead of at the oldest |

Documents whose path already starts with `s3://` are skipped. The tool writes the migrated and failed documents to `migration_state.json` in the current directory (and rollback problems to `rollback_errors.json`); without `--enable-rollback` it carries on past failed documents and lists them there. Keep the local files (do not use `--delete-local`) until you have checked that documents open after switching the server to S3.

The tool goes through every local document in one run, oldest first, however many there are; a second run finds only what is still local.

### S3 to a different bucket or provider

Readur stores each document's object key, not the bucket, in the database. To move to another bucket, copy every object with its key unchanged, then change `S3_BUCKET_NAME` (and `S3_ENDPOINT_URL`, `S3_REGION` and the credentials if the provider changes) and restart Readur. Any S3 copy tool works, for example:

```bash
rclone sync oldremote:readur-documents newremote:readur-documents-new
```

Stop Readur during the copy, or run a final sync after stopping it, so that no upload is missed.

### S3 back to local storage

There is no tool for this direction. Documents uploaded to S3 keep `s3://` paths in the database, and turning S3 off does not change them.

## Moving to a New Server

1. Install Readur on the new server with the same configuration (see the [Installation Guide](installation.md)), but do not start it.
2. Stop Readur on the old server.
3. Move the database (dump and restore as above, or move the PostgreSQL data volume as a whole if both servers run the same PostgreSQL major version).
4. Copy the upload directory to the new `UPLOAD_PATH`. Readur stores local paths in the database, so mount the directory at the same path inside the container (`/app/uploads` in the Docker image) or set the same `UPLOAD_PATH` as before. With S3 storage there is nothing to copy.
5. Keep the same `JWT_SECRET` if you set one. If you did not, the generated key is in the database and moves with it.
6. Start Readur and check `GET /api/health`, log in, and open a few documents.

### Docker Compose to Kubernetes

The steps are the same, with the Helm chart (`charts/readur`) on the receiving end:

- The chart does not deploy PostgreSQL. Restore the dump into the database you give it through the `DATABASE_URL` environment variable.
- Documents go on the `uploads` PersistentVolumeClaim, mounted at `/app/uploads` like in the Docker image. Copy the files into it, for example with `kubectl cp` into a running pod, or use S3 storage instead.
- To keep a `JWT_SECRET` you set yourself, put it in the chart's `auth.jwtSecret` value or in a Secret named by `auth.existingSecret`.

## Bringing Existing Documents into Readur

Readur has no importer for other document management systems. Existing files come in the same ways as new ones, and get OCR on the way:

- **Upload** in the web interface or with `POST /api/documents` (see the [File Upload Guide](file-upload-guide.md)).
- **Watch folder:** copy the files into `WATCH_FOLDER` (see [Watch Folder](WATCH_FOLDER.md)). Only extensions in `ALLOWED_FILE_TYPES` are imported.
- **Sources:** let Readur sync a WebDAV share, a local folder or an S3 bucket (see the [Sources Guide](sources-guide.md)).
- **`batch_ingest`:** a developer tool in the source tree that imports a whole directory for one user: `cargo run --release --bin batch_ingest -- /path/to/files --user-id <UUID>` (add `--monitor` to follow progress). It reads the same environment as the server (see [CLI Tools](administration/cli-tools.md)).

Metadata from another system (tags, folders, custom fields) is not imported. Labels can be created and assigned afterwards in the interface or through the API.
