# S3 Storage Backend Guide

## Overview

Readur can store uploaded documents in Amazon S3 or an S3-compatible service (MinIO, RustFS, Wasabi, Backblaze B2 and similar) instead of the local filesystem. The S3 backend is part of the default build (the `s3` Cargo feature is on by default), so the published image `ghcr.io/readur/readur` includes it.

This page covers S3 as the storage backend for Readur's own files. Importing documents *from* an S3 bucket is a separate feature, configured per source in the UI; see the [Sources Guide](sources-guide.md).

What the backend does (`src/services/s3_service.rs`):

- Stores each document, its thumbnail and its processed OCR image as S3 objects (see [Storage Structure](#storage-structure)).
- Uses a multipart upload for files larger than 100 MB, in 16 MB parts.
- Retries failed requests up to 3 times (4 attempts in total) with exponential backoff starting at 100 ms.
- Applies per-attempt timeouts: 10 s to connect, 60 s read timeout, 600 s per request attempt.

## Prerequisites

1. A bucket, and an access key ID and secret access key for it.
2. Permissions for that key. Readur lists the bucket, checks it with HeadBucket, and reads, writes and deletes objects. A minimal IAM policy:

   ```json
   {
     "Version": "2012-10-17",
     "Statement": [
       {
         "Effect": "Allow",
         "Action": ["s3:ListBucket"],
         "Resource": "arn:aws:s3:::readur-documents"
       },
       {
         "Effect": "Allow",
         "Action": [
           "s3:GetObject",
           "s3:PutObject",
           "s3:DeleteObject",
           "s3:AbortMultipartUpload"
         ],
         "Resource": "arn:aws:s3:::readur-documents/*"
       }
     ]
   }
   ```

   Replace `readur-documents` with your bucket name. In IAM, `s3:PutObject` also covers the multipart upload calls and `s3:GetObject` covers HeadObject.

## Configuration

S3 storage is configured with environment variables only. The full list with defaults is in the [Configuration Reference](configuration-reference.md#s3-storage).

| Variable | Required | Meaning |
|----------|----------|---------|
| `S3_ENABLED` | Yes (or `STORAGE_BACKEND`) | `true` enables the S3 backend |
| `STORAGE_BACKEND` | No | `s3` is equivalent to `S3_ENABLED=true` |
| `S3_BUCKET_NAME` | Yes | Bucket name |
| `S3_ACCESS_KEY_ID` | Yes | Access key ID |
| `S3_SECRET_ACCESS_KEY` | Yes | Secret access key |
| `S3_REGION` | No | Region, default `us-east-1` |
| `S3_ENDPOINT_URL` | For S3-compatible services | Custom endpoint URL. `S3_ENDPOINT` is accepted as an alias when `S3_ENDPOINT_URL` is not set |
| `S3_FORCE_PATH_STYLE` | No | `true` forces path-style addressing, any other value virtual-hosted. `S3_PATH_STYLE` is an alias |

Credentials are only taken from these variables. Readur does not read the AWS shared credentials file, AWS profiles or instance roles.

When `S3_FORCE_PATH_STYLE` is unset:

- without a custom endpoint, virtual-hosted addressing is used (the AWS default);
- with a custom endpoint, Readur tries path-style first and falls back to virtual-hosted if a test listing of the bucket fails. If neither works it keeps path-style and logs `Could not auto-detect S3 addressing style ...`.

`S3_PREFIX` puts every object the storage backend writes below a prefix, so several installations can share a bucket. Surrounding slashes and spaces are trimmed (`/tenant/a/` and `tenant/a` are the same); an empty value or `/` means no prefix. S3 sources keep their own prefix, set per source in the web interface; `S3_PREFIX` only applies to where Readur stores documents.

### Example: AWS S3

```bash
S3_ENABLED=true
S3_BUCKET_NAME=readur-documents
S3_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE
S3_SECRET_ACCESS_KEY=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY
S3_REGION=us-west-2
```

### Example: MinIO or another S3-compatible service

```bash
S3_ENABLED=true
S3_BUCKET_NAME=readur-documents
S3_ACCESS_KEY_ID=minio-access-key
S3_SECRET_ACCESS_KEY=minio-secret-key
S3_REGION=us-east-1
S3_ENDPOINT_URL=http://minio.example.com:9000
```

Other services work the same way: set `S3_ENDPOINT_URL` to the service's S3 endpoint and `S3_REGION` to the region it expects. Hosts on private and LAN addresses are allowed; link-local addresses (which include cloud metadata services) are refused.

### Docker Compose

Add the variables to the `readur` service's `environment` in `docker-compose.yml`:

```yaml
services:
  readur:
    image: ghcr.io/readur/readur:latest
    environment:
      DATABASE_URL: postgresql://readur:readur@postgres/readur
      S3_ENABLED: "true"
      S3_BUCKET_NAME: readur-documents
      S3_ACCESS_KEY_ID: minio-access-key
      S3_SECRET_ACCESS_KEY: minio-secret-key
      S3_ENDPOINT_URL: http://minio:9000
    volumes:
      - ./data/uploads:/app/uploads
```

Keep the uploads volume (`UPLOAD_PATH`, `/app/uploads` in the image) even with S3: Readur still keeps generated thumbnails there, and it is where documents stored before S3 was enabled live. Readur also loads a `.env` file from its working directory if one exists.

### What happens at startup

On startup Readur prints which S3 variables it loaded (secrets are shown as `configured`), then connects to the bucket by listing one object and sending HeadBucket.

- If S3 is enabled but the bucket name or either key is missing, it prints `❌ S3 enabled but missing required configuration (bucket_name, access_key_id, or secret_access_key)` and uses local storage.
- If the connection test fails, it logs `❌ Failed to initialize configured storage backend: ...` followed by `🔄 Falling back to local storage...` and runs with local storage.

In both cases the server still starts, so check the log after enabling S3. On success the log contains `✅ File service initialized with s3 storage backend`.

## Storage Structure

Object keys (`src/services/s3_service.rs`), each below `S3_PREFIX` when it is set:

```text
documents/{user_id}/{year}/{month}/{document_id}.{extension}
thumbnails/{user_id}/{document_id}_thumb.jpg
processed_images/{user_id}/{document_id}_processed.png
```

The year and month are the UTC date at the time of upload. A document without an extension is stored as `documents/{user_id}/{year}/{month}/{document_id}`. In the database, the document's `file_path` becomes `s3://` followed by the key, for example `s3://documents/550e8400-e29b-41d4-a716-446655440000/2026/03/123e4567-e89b-12d3-a456-426614174000.pdf`.

Existing documents are always read and deleted by the key recorded at upload (`file_path`), not by recomputing it. Deleting a document deletes that object and its thumbnail and processed-image objects; an object that is already missing counts as deleted. Turning `S3_PREFIX` on or changing it does not move existing objects: they stay where they are and keep working (thumbnails and processed images stored without a prefix are still cleaned up), and only new uploads use the new prefix.

## Documents stored before S3 was enabled

Switching to S3 does not move existing files. Documents whose `file_path` is a local path keep working: Readur reads them from the local filesystem when the S3 lookup fails. To move them into the bucket, use the `migrate_to_s3` tool below.

## Migrating Local Files to S3

`migrate_to_s3` is a separate binary built from source (`src/bin/migrate_to_s3.rs`). It is not in the Docker image. Run it from a checkout of the repository, on a machine that can reach the database, the bucket and the local document files, with the same environment as the server (`DATABASE_URL` or the `POSTGRES_*` variables, `UPLOAD_PATH`, and the S3 variables). It reads each document's `file_path` as stored in the database, so relative paths are resolved from the directory you run it in.

```bash
# Back up the database first
pg_dump -U readur readur > readur_backup.sql

# Preview: lists the documents that would be migrated, changes nothing
cargo run --release --bin migrate_to_s3 -- --dry-run

# Migrate
cargo run --release --bin migrate_to_s3
```

Flags:

| Flag | Meaning |
|------|---------|
| `-d`, `--dry-run` | Only list what would be migrated |
| `--delete-local` | Delete each local file (and its local thumbnail and processed image) after it has been uploaded |
| `-l`, `--limit <N>` | Migrate at most N documents |
| `-u`, `--user-id <UUID>` | Only migrate this user's documents |
| `--enable-rollback` | On the first failed document, undo all migrations done in this run and exit with an error |
| `--resume-from <UUID>` | Start at this document (documents are taken oldest first). An ID that does not exist is an error |

For each document with a local `file_path` (paths starting with `s3://` are skipped) the tool:

1. reads the local file and uploads it under the key shown in [Storage Structure](#storage-structure);
2. sets the document's `file_path` to `s3://<key>`;
3. uploads `{UPLOAD_PATH}/thumbnails/{document_id}_thumb.jpg` and `{UPLOAD_PATH}/processed_images/{document_id}_processed.png` if they exist (a failure here is logged as a warning and does not fail the document);
4. with `--delete-local`, deletes the local files.

Which documents are picked up:

- Every document whose `file_path` does not start with `s3://` (only the given user's with `--user-id`), oldest first, fetched in pages of 500.
- `--limit` stops the run after that many documents; without it the tool goes through all of them.

Documents already in S3 are never selected, so a second run migrates only what is still local (new documents and earlier failures) and finds nothing once everything has been moved.

The tool writes its progress to `migration_state.json` in the current directory (every 10 migrated documents and at the end), including `completed_migrations` and `failed_migrations` with the error for each failed document. If a rollback itself fails, the errors are written to `rollback_errors.json`.

Rollback restores the database paths and deletes the uploaded objects. It does not restore local files, so do not combine `--enable-rollback` with `--delete-local`.

Logging for this tool is fixed at `info`; `RUST_LOG` does not change it.

### Verifying the migration

```sql
SELECT
    COUNT(*) FILTER (WHERE file_path LIKE 's3://%')     AS s3_documents,
    COUNT(*) FILTER (WHERE file_path NOT LIKE 's3://%') AS local_documents
FROM documents;

SELECT id, original_filename, file_path
FROM documents
WHERE file_path NOT LIKE 's3://%'
LIMIT 10;
```

## Bucket settings

Readur does not configure the bucket. Server-side encryption, versioning, lifecycle rules and replication are set on the bucket with your provider's tools and are transparent to Readur, with one caveat: do not use lifecycle rules that move objects to archive storage classes that need a restore before reading (such as S3 Glacier Flexible Retrieval or Deep Archive), because Readur reads objects directly with GetObject.

## Troubleshooting

See the [S3 Troubleshooting Guide](s3-troubleshooting.md).

## Next Steps

- [Configuration Reference](configuration-reference.md#s3-storage) for every S3 variable
- [S3 Troubleshooting Guide](s3-troubleshooting.md)
- [Migration Troubleshooting](administration/migration-troubleshooting.md)
