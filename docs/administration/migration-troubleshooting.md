# Migration Troubleshooting Guide

This page covers problems with `migrate_to_s3`, the tool that moves documents from local storage into the S3 storage backend. How to run it, its flags and what it does are described in [Migrating Local Files to S3](../s3-storage-guide.md#migrating-local-files-to-s3). For S3 connection and permission errors in general, see the [S3 Troubleshooting Guide](../s3-troubleshooting.md).

## Before you start

- `migrate_to_s3` is built from source (`cargo run --release --bin migrate_to_s3`). It is not included in the Docker image, so `docker exec` into the Readur container cannot run it.
- Run it with the same environment as the server: database (`DATABASE_URL` or the `POSTGRES_*` variables), `UPLOAD_PATH`, and the S3 variables. It needs network access to PostgreSQL and the bucket, and read access to the document files.
- It reads each document's `file_path` exactly as stored in the database. Relative paths (for example `./uploads/...`) are resolved from the directory you run the tool in; absolute container paths (for example `/app/uploads/...`) must exist at the same path on the machine running the tool.
- Back up the database and the uploads directory first, and do a `--dry-run`.

## Startup errors

| Message | Cause and fix |
|---------|---------------|
| `S3 is not enabled in configuration. Set S3_ENABLED=true and provide S3 configuration.` | Neither `S3_ENABLED=true` nor `STORAGE_BACKEND=s3` is set in the tool's environment |
| `S3 configuration not found` | S3 is enabled but `S3_BUCKET_NAME`, `S3_ACCESS_KEY_ID` or `S3_SECRET_ACCESS_KEY` is empty |
| `Bucket name is required`, `Access key ID is required`, `Secret access key is required` | Same as above |
| `❌ S3 connection failed: ...` | The bucket test (list one object, HeadBucket) failed; the text after the colon is the cause. See [Connection test failures](../s3-troubleshooting.md#connection-test-failures) |
| Database connection errors | The tool cannot reach PostgreSQL with the configured `DATABASE_URL` / `POSTGRES_*` values |

## "No local documents found to migrate"

The tool only looks at a limited window of documents, newest first:

- without `--user-id`: the 500 most recent documents of each user (capped overall by `--limit`);
- with `--user-id`: the most recent documents of that user, up to `--limit` (default 1000).

Documents already in S3 count towards that window before they are filtered out. If a user has more documents than the window and the newest ones are already migrated, the tool reports nothing to do even though older local documents remain. Check:

```sql
SELECT user_id, COUNT(*) AS local_documents
FROM documents
WHERE file_path NOT LIKE 's3://%'
GROUP BY user_id;
```

Then run the tool per user with a limit above that user's total document count:

```bash
cargo run --release --bin migrate_to_s3 -- \
  --user-id 550e8400-e29b-41d4-a716-446655440000 \
  --limit 100000
```

## Per-document failures

A failed document is logged as `❌ Failed to migrate <filename>: <error>` and the tool moves on to the next one. At the end it logs `❌ Failed to migrate: N files` and `💾 Check migration_state.json for details on failures`. The process still exits successfully unless `--enable-rollback` was given.

Common errors:

| Error | Cause |
|-------|-------|
| `Local file not found: <path>` | The file at the stored `file_path` does not exist from where the tool runs. Run it from the server's working directory, or where the uploads directory is mounted at the same path |
| `Permission denied` | The user running the tool cannot read the file |
| `Failed to store file <key>: ...` / `Failed to complete multipart upload for <key>: ...` | The upload failed after 4 attempts; see [Upload and download failures](../s3-troubleshooting.md#upload-and-download-failures) |

Thumbnails and processed images that fail to upload only produce a warning (`Failed to migrate thumbnail for ...`); the document itself still counts as migrated.

Failed documents keep their local `file_path`, so running the tool again retries them. Documents that were migrated are skipped on the next run because their path starts with `s3://`.

## Reading migration_state.json

The tool writes `migration_state.json` in the directory it runs in, every 10 migrated documents and at the end of the run:

```json
{
  "started_at": "2026-03-01T10:00:00Z",
  "completed_migrations": [
    {
      "document_id": "123e4567-e89b-12d3-a456-426614174000",
      "user_id": "550e8400-e29b-41d4-a716-446655440000",
      "original_path": "./uploads/documents/123e4567-e89b-12d3-a456-426614174000.pdf",
      "s3_key": "documents/550e8400-e29b-41d4-a716-446655440000/2026/03/123e4567-e89b-12d3-a456-426614174000.pdf",
      "migrated_at": "2026-03-01T10:00:02Z",
      "associated_files": []
    }
  ],
  "failed_migrations": [
    {
      "document_id": "987fcdeb-51a2-43f1-b321-123456789abc",
      "original_path": "./uploads/documents/987fcdeb-51a2-43f1-b321-123456789abc.pdf",
      "error": "Local file not found: ./uploads/documents/987fcdeb-51a2-43f1-b321-123456789abc.pdf",
      "failed_at": "2026-03-01T10:00:03Z"
    }
  ],
  "total_files": 2,
  "processed_files": 2
}
```

`original_path` is the `file_path` before migration, which is what you need to undo a migration by hand. Each run overwrites the file, so copy it before running the tool again.

## Interrupted runs

If the tool is stopped part-way, every document migrated so far already has its `s3://` path in the database. Run the tool again: migrated documents are skipped and the rest are processed. `--resume-from <document UUID>` starts at a given position in the list instead; an ID that is not in the list is silently ignored and the run starts from the beginning.

A document interrupted between the upload and the database update keeps its local path and is uploaded again on the next run. If the month changed in between, the earlier upload stays in the bucket under the old month's key.

## Rollback

With `--enable-rollback`, the first failed document stops the run. The tool then, for every document migrated in this run (most recent first), restores the original `file_path` and deletes the uploaded document, thumbnail and processed-image objects. It exits with `Migration failed and was rolled back`.

- Rollback does not restore local files. If `--delete-local` was also given, the restored paths point to files that no longer exist. Do not combine the two flags.
- If some rollback steps fail, the tool writes `rollback_errors.json` with `rolled_back_count`, `rollback_errors` and the full `original_migration_state`, and logs `Rollback errors saved to rollback_errors.json for manual cleanup`.
- If the rollback fails as a whole, it saves `migration_state.json` and exits with `Migration failed and rollback also failed. Check migration state file for manual recovery.`

There is no separate command to roll back a completed run. To undo a document by hand, set its `file_path` back to the `original_path` from `migration_state.json` (only if the local file still exists):

```sql
UPDATE documents
SET file_path = './uploads/documents/123e4567-e89b-12d3-a456-426614174000.pdf'
WHERE id = '123e4567-e89b-12d3-a456-426614174000';
```

## Validating the result

```sql
-- Documents still on local storage
SELECT COUNT(*) FROM documents WHERE file_path NOT LIKE 's3://%';

-- Documents with an empty path
SELECT COUNT(*) FROM documents WHERE file_path IS NULL OR file_path = '';
```

Then open a few migrated documents in the UI, or download one through the API with `GET /api/documents/{id}/download` (see the [API Reference](../api-reference.md)). The server must be running with the S3 backend enabled to read `s3://` paths; check its startup log for `✅ File service initialized with s3 storage backend`.

## Logs

The tool logs to stdout at `info` level (fixed; `RUST_LOG` has no effect on it). Keep the output when reporting a problem:

```bash
cargo run --release --bin migrate_to_s3 2>&1 | tee migrate.log
```
