# S3 Storage Troubleshooting Guide

This page covers problems with the S3 storage backend. Configuration is described in the [S3 Storage Guide](s3-storage-guide.md). Readur logs to stdout; use `docker compose logs readur` (or your platform's equivalent) to read the messages quoted below, and `RUST_LOG` to change the log level (see the [Configuration Reference](configuration-reference.md)).

## First check: is S3 actually in use?

Readur does not refuse to start when S3 cannot be used. It falls back to local storage and keeps running, so a misconfiguration can go unnoticed. Look at the startup log:

| Log line | Meaning |
|----------|---------|
| `✅ File service initialized with s3 storage backend` | S3 is in use |
| `⚠️  S3_ENABLED: false (using default - env var not set)` | Neither `S3_ENABLED=true` nor `STORAGE_BACKEND=s3` is set |
| `❌ S3 enabled but missing required configuration (bucket_name, access_key_id, or secret_access_key)` | One of `S3_BUCKET_NAME`, `S3_ACCESS_KEY_ID`, `S3_SECRET_ACCESS_KEY` is empty; local storage is used |
| `❌ Failed to initialize configured storage backend: ...` then `🔄 Falling back to local storage...` | The connection test failed; the text after the colon is the cause; local storage is used |

Documents uploaded while Readur ran on the fallback are stored locally. They remain readable after S3 starts working, and `migrate_to_s3` can move them into the bucket later.

## Connection test failures

At startup Readur lists one object in the bucket and sends HeadBucket. Errors from these calls look like:

```text
Failed to access S3 bucket readur-documents: <SDK error with service code and message>
Cannot access bucket readur-documents: <SDK error>
```

The SDK error includes the service error code. Common causes:

| Error code or text | Cause |
|--------------------|-------|
| `InvalidAccessKeyId`, `SignatureDoesNotMatch` | Wrong `S3_ACCESS_KEY_ID` or `S3_SECRET_ACCESS_KEY` |
| `AccessDenied` | The key lacks `s3:ListBucket` on the bucket (see the policy in the [S3 Storage Guide](s3-storage-guide.md#prerequisites)) |
| `NoSuchBucket` | Wrong `S3_BUCKET_NAME`, or the bucket is in a different account/endpoint |
| `PermanentRedirect`, `AuthorizationHeaderMalformed` | `S3_REGION` does not match the bucket's region |
| `dispatch failure`, connection refused, DNS errors | Wrong `S3_ENDPOINT_URL`, or the endpoint is not reachable from the Readur container |
| `The server address is not allowed` | The endpoint resolves to a refused address (link-local such as `169.254.x.x`, cloud metadata addresses, the unspecified or broadcast address). Private and LAN addresses are allowed |
| Certificate or TLS errors | The endpoint's certificate is not trusted. Readur has no option to skip certificate verification; use a certificate the client trusts, or plain `http://` on a trusted network |

To check the same credentials outside Readur, use your provider's CLI or any S3 client with the same endpoint, region, bucket and keys.

### S3-compatible services and addressing style

With a custom `S3_ENDPOINT_URL` and no `S3_FORCE_PATH_STYLE`, Readur tries path-style first and then virtual-hosted, each with a 3-second test listing. When both fail it logs:

```text
Could not auto-detect S3 addressing style for bucket 'readur-documents'; defaulting to path-style. Set S3_FORCE_PATH_STYLE=true|false to override.
```

That message means neither test succeeded; the real cause shows up in the connection test error that follows. If your service only supports one style, set `S3_FORCE_PATH_STYLE=true` (path-style, `http://<endpoint>/<bucket>/<key>`) or `false` (virtual-hosted, `http://<bucket>.<endpoint>/<key>`). With virtual-hosted addressing the host name `<bucket>.<endpoint host>` must resolve.

When running MinIO or similar in the same Docker Compose project, use the service name as the host (for example `http://minio:9000`), not `localhost`, which refers to the Readur container itself.

## Upload and download failures

Every S3 request is retried up to 3 times with exponential backoff (100 ms, 200 ms, 400 ms). Retries are logged as:

```text
S3 operation 'store_file: documents/...' failed (attempt 1/4), retrying in 100ms: ...
```

and a final failure as `S3 operation '...' failed after 4 attempts: ...`. Error texts by operation:

| Message | Operation |
|---------|-----------|
| `Failed to store file <key>: ...` | Single-request upload (files up to 100 MB) |
| `Failed to initiate multipart upload for <key>: ...`, `Failed to complete multipart upload for <key>: ...` | Multipart upload (files over 100 MB, 16 MB parts) |
| `Failed to download S3 object <key>: ...` | Reading a document |
| `S3 object <key> download incomplete: expected N bytes, received M` | The connection ended before the whole object arrived |
| `Timed out reading S3 object <key> after 600s` | Reading the object body took longer than 10 minutes |
| `Failed to delete file <key>: ...` | Deleting an object |

Request limits are fixed in the code: 10 s to connect, 60 s read timeout, 600 s per request attempt. They are not configurable.

If a multipart upload fails to complete, Readur tries to abort it (logging `Failed to abort multipart upload: ...` if that also fails). Incomplete multipart uploads that were not aborted still occupy storage in the bucket; a lifecycle rule that aborts incomplete multipart uploads after a few days cleans them up.

Uploads larger than `MAX_FILE_SIZE_MB` (default 50) are rejected before S3 is involved.

## Documents that will not open after enabling S3

Documents stored before S3 was enabled have a local `file_path` and are read from the local filesystem. If they cannot be opened, check that the uploads directory (`UPLOAD_PATH`, `/app/uploads` in the image) is still mounted with the same contents. Documents stored in S3 have a `file_path` starting with `s3://`:

```sql
SELECT id, original_filename, file_path
FROM documents
WHERE id = '123e4567-e89b-12d3-a456-426614174000';
```

The part after `s3://` is the object key in the bucket; check with your provider's tools that the object exists.

## Objects left in the bucket after deleting documents

Deleting a document deletes the object at the key recorded at upload (its `file_path`), plus its thumbnail and processed image; an object that is already missing counts as deleted. Earlier versions recomputed the document's key from the current month when deleting, so documents deleted in a later month than their upload left their object behind under `documents/{user_id}/{year}/{month}/`. Such leftovers can be found by comparing the keys in the bucket with the `file_path` values in the `documents` table.

## Migration tool problems

See [Migration Troubleshooting](administration/migration-troubleshooting.md) for `migrate_to_s3`.

## Debug logging

```bash
RUST_LOG=readur=debug,aws_sdk_s3=debug,aws_config=debug
RUST_BACKTRACE=1
```

Set these in the container environment and restart Readur. `aws_sdk_s3=debug` logs every S3 request; turn it off again afterwards.
