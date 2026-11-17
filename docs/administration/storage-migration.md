# Storage Migration Guide

## Overview

Readur stores uploaded documents on the local filesystem (`UPLOAD_PATH`) by default, or in an S3-compatible bucket when S3 storage is enabled. Documents uploaded while local storage was active keep a local file path in the database. The `migrate_to_s3` tool copies those files into the S3 bucket and points the database records at the new S3 objects.

The tool only migrates **local to S3**. There is no tool for moving documents from S3 back to local storage or between buckets.

The full option reference is in [Command-Line Tools](./cli-tools.md#migrate_to_s3). This page describes the procedure.

## When You Need This

- You started Readur with local storage and want to switch to S3 storage without losing existing documents.

## What the Tool Does

For every selected document whose stored file path does not start with `s3://`, the tool:

1. Reads the file from the path stored in the database. If it does not exist, the document counts as failed.
2. Uploads it to the bucket under `documents/<user-id>/<year>/<month>/<document-id>.<extension>` (year and month of the migration run; files larger than the multipart threshold use a multipart upload).
3. Changes the document's file path in the database to `s3://<key>`.
4. Uploads the thumbnail `<UPLOAD_PATH>/thumbnails/<document-id>_thumb.jpg` to `thumbnails/<user-id>/<document-id>_thumb.jpg` and the processed image `<UPLOAD_PATH>/processed_images/<document-id>_processed.png` to `processed_images/<user-id>/<document-id>_processed.png`, if those files exist. A failure here is logged as a warning and does not fail the document.
5. With `--delete-local`, deletes the local original, thumbnail and processed image after they were uploaded.

Documents are processed one at a time, oldest first.

### Which documents are selected

Every document whose file path does not start with `s3://` (with `--user-id ID`, only that user's), oldest first. The tool reads them in pages of 500, so any number of documents is covered in one run. With `--limit N` it stops after N documents.

Documents already in S3 are never selected, so a later run migrates only what is still local (documents added since, and earlier failures), and a run after everything has moved finds nothing to do.

## Prerequisites

- A source checkout of Readur and a Rust toolchain. The tool is not included in the Docker image, which contains only the `readur` server binary.
- The same environment as the server: database settings (`DATABASE_URL` or the `POSTGRES_*` variables) and `UPLOAD_PATH`.
- S3 settings: `S3_ENABLED=true` (or `STORAGE_BACKEND=s3`), `S3_BUCKET_NAME`, `S3_ACCESS_KEY_ID`, `S3_SECRET_ACCESS_KEY`, and as needed `S3_REGION` (default `us-east-1`), `S3_ENDPOINT_URL` and `S3_FORCE_PATH_STYLE` for S3-compatible services. If `S3_PREFIX` is set, migrated documents, thumbnails and processed images are written below that prefix, just like new uploads. See the [configuration reference](../configuration-reference.md).
- Read access to the document files **at the paths stored in the database**. In the Docker setup these are absolute paths under `/app/uploads`, so the tool must run where the uploads directory is available at that same path. Relative paths are resolved against the tool's working directory.

### Before you start

1. Back up the database:
   ```bash
   pg_dump -h localhost -U readur readur > readur_backup_$(date +%Y%m%d).sql
   ```
2. Back up the uploads directory:
   ```bash
   tar -czf uploads_backup_$(date +%Y%m%d).tar.gz /path/to/uploads
   ```

## Step-by-Step Migration Process

### Step 1: Build the tool

```bash
git clone https://github.com/readur/readur.git
cd readur
cargo build --release --bin migrate_to_s3
```

The binary is `target/release/migrate_to_s3`. Alternatively, `cargo run --release --bin migrate_to_s3 -- <options>` builds and runs it in one step.

### Step 2: Set the environment

```bash
export DATABASE_URL="postgresql://readur:readur@localhost:5432/readur"
export UPLOAD_PATH="/app/uploads"
export S3_ENABLED=true
export S3_BUCKET_NAME="readur-documents"
export S3_ACCESS_KEY_ID="your-access-key"
export S3_SECRET_ACCESS_KEY="your-secret-key"
export S3_REGION="us-east-1"
# Only for S3-compatible services such as MinIO:
export S3_ENDPOINT_URL="https://s3.example.com"
```

If `S3_ENABLED` is not `true` (and `STORAGE_BACKEND` is not `s3`), the tool logs `S3 is not enabled in configuration. Set S3_ENABLED=true and provide S3 configuration.` and exits with status 1. If the bucket cannot be listed, it logs `S3 connection failed` and exits with status 1.

### Step 3: Dry run

```bash
./target/release/migrate_to_s3 --dry-run
```

A dry run connects to the database and to S3 (and fails the same way if either is unreachable), then lists the documents it would migrate, one log line each, and changes nothing:

```text
INFO migrate_to_s3: 📋 Found 2 documents with local file paths
INFO migrate_to_s3: 🔍 DRY RUN - Would migrate the following files:
INFO migrate_to_s3:   - invoice-2024.pdf (User: 3f6c1e2a-8d4b-4c1e-9a77-2b1f0c9d5e10, Size: 184320 bytes)
INFO migrate_to_s3:   - scan.png (User: 3f6c1e2a-8d4b-4c1e-9a77-2b1f0c9d5e10, Size: 92011 bytes)
INFO migrate_to_s3: 💡 Run without --dry-run to perform actual migration
```

Log lines also carry a timestamp. The log level of this tool is fixed at `info`; `RUST_LOG` does not change it.

### Step 4: Run the migration

```bash
./target/release/migrate_to_s3
```

The tool logs one `📦 Migrating: <filename> (<document-id>) [n/total]` line per document followed by a success or failure line, and finishes with `🎉 Migration completed!` and the number of migrated and failed files. Without `--enable-rollback`, a failed document is recorded and the tool continues with the next one; the run still exits with status 0.

Keep the local files (do not pass `--delete-local`) until you have checked the result.

### Step 5: Check the result

- Read `migration_state.json` in the directory you ran the tool from (see [State file](#state-file)). Its `failed_migrations` list names every document that was not migrated and why.
- Count documents still stored locally:
  ```bash
  psql -h localhost -U readur readur -c "SELECT COUNT(*) FROM documents WHERE file_path NOT LIKE 's3://%';"
  ```
- Run the tool again: documents already in S3 are skipped, so a second run only retries what is left.

### Step 6: Switch the server to S3

Give the server the same S3 settings, for example in `docker-compose.yml`:

```yaml
services:
  readur:
    environment:
      S3_ENABLED: "true"
      S3_BUCKET_NAME: readur-documents
      S3_ACCESS_KEY_ID: your-access-key
      S3_SECRET_ACCESS_KEY: your-secret-key
      S3_REGION: us-east-1
```

Restart Readur. Documents uploaded by the still-running server while the migration ran are stored locally; run the tool once more after the switch to move them.

The tool only deletes local files of documents it migrates in a run started with `--delete-local`; a later run skips documents already in S3 and does not delete their local copies. Remove those yourself once you are satisfied (keep the `.readur` directory under `UPLOAD_PATH`, which holds Readur's own files such as the generated initial admin password).

## Advanced Usage

### State file

The tool writes `migration_state.json` to its working directory after every 10 successfully migrated documents and at the end of the run. It contains:

- `started_at`
- `completed_migrations`: per document `document_id`, `user_id`, `original_path`, `s3_key`, `migrated_at`, and `associated_files` (each with `file_type` `thumbnail` or `processed_image`, `original_path`, `s3_key`)
- `failed_migrations`: per document `document_id`, `original_path`, `error`, `failed_at`
- `total_files`, `processed_files`

The file is a record for you; the tool never reads it back. Each run overwrites it.

### Rollback on failure

With `--enable-rollback`, the first failed document stops the run. The tool then undoes every migration completed in this run, newest first: it restores the original file path in the database and deletes the uploaded document, thumbnail and processed image objects from S3. It exits with status 1 (`Migration failed and was rolled back`).

- Errors during the rollback are written to `rollback_errors.json` in the working directory.
- If the rollback itself fails, the state is saved to `migration_state.json` for manual recovery.
- Local files removed by `--delete-local` are **not** restored. Do not combine `--delete-local` with `--enable-rollback` unless you have a backup of the uploads directory.
- There is no command to roll back an earlier, finished run.

### Resuming

Because documents already in S3 are skipped, rerunning the tool after an interruption continues with the remaining documents. `--resume-from <document-id>` additionally skips every document older than that one (it starts at that document); an ID that does not exist is an error.

```bash
./target/release/migrate_to_s3 --resume-from 7d1e4b9a-2c3f-4e5a-8b6c-1d2e3f4a5b6c
```

### Migrating one user

Find the user ID:

```bash
psql -h localhost -U readur readur -c "SELECT id, username FROM users;"
```

Then migrate that user's documents:

```bash
./target/release/migrate_to_s3 --user-id 3f6c1e2a-8d4b-4c1e-9a77-2b1f0c9d5e10 --limit 5000
```

## Security Considerations

- Pass credentials through environment variables, not on the command line.
- `migration_state.json` and `rollback_errors.json` contain document IDs, user IDs and file paths; delete them when you no longer need them.
- Configure bucket policies and server-side encryption on the S3 side as your requirements demand; the tool does not set them.

## Support

If you encounter issues during migration:

1. Check the [troubleshooting guide](./migration-troubleshooting.md).
2. Read the tool's log output and the `error` fields in `migration_state.json`.
3. Test the migration against a copy of the database and a test bucket first when possible.
