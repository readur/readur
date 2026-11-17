# Storage Configuration Guide

## Overview

Readur stores document files in one of two places:

- **Local storage** (the default): a directory on the server, set with `UPLOAD_PATH`.
- **S3 storage**: a bucket on Amazon S3 or an S3-compatible service (MinIO, RustFS and others).

Document metadata, OCR text, labels, users and settings are always in PostgreSQL, whichever backend holds the files. Both need backing up; see [Backup Strategies](./backup.md).

WebDAV servers, local folders and S3 buckets can also be *sources* that Readur imports documents from. Sources are not storage backends: they are added per user in the web interface (**Sources**), and imported files are copied into the storage backend configured here. See the [Sources Guide](../sources-guide.md) and [Watch Folder](../WATCH_FOLDER.md).

## Local Storage

Local storage is used unless S3 is enabled. Documents go under `UPLOAD_PATH` (default `./uploads`; `/app/uploads` in the Docker image).

```bash
UPLOAD_PATH=/data/readur/uploads
```

At startup Readur creates these subdirectories:

```text
<UPLOAD_PATH>/
├── documents/          # uploaded and imported documents, named <document_id>.<extension>
├── thumbnails/         # <document_id>_thumb.jpg
├── processed_images/   # <document_id>_processed.png, the preprocessed image OCR ran on
├── temp/
└── backups/
```

When `ADMIN_PASSWORD` is not set, the generated initial admin password is also written to `<UPLOAD_PATH>/.readur/initial-admin-password` (see [Authentication](./authentication.md#the-initial-administrator)).

The Readur process must be able to create and write these directories. The Docker image runs as root, so a bind mount works without changes; when running the binary directly, give its user ownership of `UPLOAD_PATH`:

```bash
sudo mkdir -p /data/readur/uploads
sudo chown -R readur:readur /data/readur/uploads
```

In Docker Compose, keep `/app/uploads` on a volume, or documents are lost when the container is recreated:

```yaml
services:
  readur:
    image: ghcr.io/readur/readur:latest
    environment:
      UPLOAD_PATH: /app/uploads
    volumes:
      - ./readur_uploads:/app/uploads
```

Temporary files written during OCR go to `/tmp` (inside the container when you run the Docker image), not to `UPLOAD_PATH`.

## S3 Storage

S3 storage is enabled with `S3_ENABLED=true` (or the equivalent `STORAGE_BACKEND=s3`) together with a bucket name and both keys:

```bash
S3_ENABLED=true
S3_BUCKET_NAME=readur-documents
S3_REGION=us-east-1
S3_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE
S3_SECRET_ACCESS_KEY=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY
```

For an S3-compatible service, add its endpoint. Path-style addressing is used automatically when an endpoint is set; `S3_FORCE_PATH_STYLE` overrides that:

```bash
S3_ENABLED=true
S3_BUCKET_NAME=readur
S3_ENDPOINT_URL=http://minio:9000
S3_ACCESS_KEY_ID=minio-access-key
S3_SECRET_ACCESS_KEY=minio-secret-key
```

| Variable | Default | Purpose |
|----------|---------|---------|
| `S3_ENABLED` | `false` | `true` enables the S3 backend |
| `STORAGE_BACKEND` | `local` | `s3` enables the S3 backend (same as `S3_ENABLED=true`) |
| `S3_BUCKET_NAME` | - | Bucket name (required) |
| `S3_ACCESS_KEY_ID` | - | Access key ID (required) |
| `S3_SECRET_ACCESS_KEY` | - | Secret access key (required) |
| `S3_REGION` | `us-east-1` | Region |
| `S3_ENDPOINT_URL` | - | Endpoint of an S3-compatible service (alias `S3_ENDPOINT`) |
| `S3_FORCE_PATH_STYLE` | auto | `true` forces path-style addressing (alias `S3_PATH_STYLE`) |

Readur does not create or configure the bucket; create it first. Encryption, versioning and lifecycle rules are set on the bucket with your provider's tools.

### Startup Checks

At startup Readur tests the bucket by listing one object and sending HeadBucket. **If S3 is enabled but misconfigured, the server still starts, using local storage:**

- A missing bucket name or key logs `❌ S3 enabled but missing required configuration (bucket_name, access_key_id, or secret_access_key)`.
- A failed connection test logs `❌ Failed to initialize configured storage backend: ...` followed by `🔄 Falling back to local storage...`.

Check the log after enabling S3; on success it contains `✅ File service initialized with s3 storage backend`:

```bash
docker compose logs readur | grep "storage backend"
```

### Object Layout

Each key is below `S3_PREFIX` when it is set:

```text
documents/<user_id>/<year>/<month>/<document_id>.<extension>
thumbnails/<user_id>/<document_id>_thumb.jpg
processed_images/<user_id>/<document_id>_processed.png
```

Keep the `UPLOAD_PATH` volume even with S3: documents stored before S3 was enabled stay there and keep working, and the generated admin password file is written there.

### Permissions

The credentials need to list the bucket and read, write and delete objects in it. On AWS, a minimal policy:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": ["s3:ListBucket"],
      "Resource": ["arn:aws:s3:::readur-documents"]
    },
    {
      "Effect": "Allow",
      "Action": ["s3:GetObject", "s3:PutObject", "s3:DeleteObject", "s3:AbortMultipartUpload"],
      "Resource": ["arn:aws:s3:::readur-documents/*"]
    }
  ]
}
```

Files larger than 100 MB are uploaded with multipart uploads.

See the [S3 Storage Guide](../s3-storage-guide.md) for more detail and the [S3 Troubleshooting Guide](../s3-troubleshooting.md) for connection problems.

## Moving Existing Documents to S3

Enabling S3 does not move documents that are already stored locally. The `migrate_to_s3` tool uploads them and updates their paths in the database. It is built from source and not included in the Docker image:

```bash
# Preview what would be migrated
cargo run --release --bin migrate_to_s3 -- --dry-run

# Migrate
cargo run --release --bin migrate_to_s3
```

Its flags are `--dry-run`, `--delete-local`, `--limit <N>`, `--user-id <UUID>`, `--enable-rollback` and `--resume-from <UUID>`. Back up the database first and read [Storage Migration](../administration/storage-migration.md) and the [S3 Storage Guide](../s3-storage-guide.md#migrating-local-files-to-s3) before running it. There is no tool for moving documents from S3 back to local storage.

## Monitoring Disk Usage

Readur does not limit storage use. The admin metrics (`GET /api/metrics`, `documents.total_storage_bytes`) report the total size of all documents as recorded in the database; for free space, watch the filesystem or bucket with your usual tools:

```bash
# Local storage
df -h /data/readur/uploads
du -sh /data/readur/uploads/*

# Inside the container
docker compose exec readur du -sh /app/uploads
```

## Related Documentation

- [S3 Storage Guide](../s3-storage-guide.md)
- [Storage Migration](../administration/storage-migration.md)
- [Sources Guide](../sources-guide.md)
- [Backup Strategies](./backup.md)
- [Performance Tuning](./performance.md)
