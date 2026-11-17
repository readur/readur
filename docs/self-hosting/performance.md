# Performance Tuning Guide

## Overview

Readur is a single Rust binary that serves the web interface and API, runs the OCR queue and syncs sources in one process, with PostgreSQL holding all metadata and OCR text. There are no worker processes, caches or connection-pool settings to tune. Most of the work is OCR, so performance comes down to how many documents are OCRed at once, how much CPU and memory the server has, and how PostgreSQL is set up.

Every setting Readur reads is in the [Configuration Reference](../configuration-reference.md); the ones that matter for performance are below. They are environment variables read at startup, so restart the server after changing one.

## OCR Throughput

| Variable | Default | Effect |
|----------|---------|--------|
| `CONCURRENT_OCR_JOBS` | `4` | Documents OCRed in parallel. Values are clamped to 1-15 |
| `OCR_TIMEOUT_SECONDS` | `300` | Time limit for processing one document; a document that takes longer fails |
| `MAX_PDF_SIZE_MB` | `100` | Larger PDFs are not processed for text |
| `MAX_OFFICE_DOCUMENT_SIZE_MB` | `100` | Larger Office documents are not processed for text |
| `MAX_FILE_SIZE_MB` | `50` | Largest accepted request body, and so the largest upload |
| `OCR_LANGUAGE` | `eng` | Default OCR language(s); every extra language in a combination such as `eng+deu` makes OCR slower |

OCR is CPU-bound. A reasonable starting point for `CONCURRENT_OCR_JOBS` is the number of CPU cores you can give to OCR, lowered if the server runs short of memory during large batches. Large scanned PDFs need noticeably more memory than text documents.

```bash
CONCURRENT_OCR_JOBS=8
OCR_TIMEOUT_SECONDS=600
```

`CONCURRENT_OCR_JOBS` is server-wide. `OCR_TIMEOUT_SECONDS` is the time limit for documents whose owner has not set their own **OCR Timeout** in **Settings → OCR**; either stops at 600 seconds.

Per-user OCR settings do change the work done for that user's documents: the selected languages, the OCR timeout, DPI and engine options, and the image enhancement options in **Settings → OCR** (preprocessing scanned images takes extra time; a higher DPI renders larger page images).

### Managing the Queue

Administrators can watch and control the OCR queue in the web interface or through the API (see [OCR and Queue Endpoints](../api-reference.md#ocr-and-queue-endpoints)):

```bash
# Queue counts: pending, processing, failed, completed today, wait times
curl -H "Authorization: Bearer $token" http://localhost:8000/api/queue/stats

# Pause OCR during busy hours, resume later
curl -X POST -H "Authorization: Bearer $token" http://localhost:8000/api/queue/pause
curl -X POST -H "Authorization: Bearer $token" http://localhost:8000/api/queue/resume

# Retry failed jobs
curl -X POST -H "Authorization: Bearer $token" http://localhost:8000/api/queue/requeue/failed
```

## Watch Folders and Sources

| Variable | Default | Effect |
|----------|---------|--------|
| `WATCH_INTERVAL_SECONDS` | `30` | Polling interval for network filesystems, and the minimum interval between re-processing the same file |
| `FILE_STABILITY_CHECK_MS` | `1000` | How long a file's size must stay unchanged before it is imported |
| `FORCE_POLLING_WATCH` | unset | Poll instead of using filesystem events (any value turns it on) |
| `ALLOWED_FILE_TYPES` | `pdf,txt,doc,docx,png,jpg,jpeg` | Extensions imported from watch folders; others are skipped |

Local folders are watched with filesystem events; paths that look like NFS, SMB or CIFS mounts are always polled. Polling a large network share is more expensive than events, so a longer `WATCH_INTERVAL_SECONDS` reduces load on both the server and the share. See [Watch Folder](../WATCH_FOLDER.md).

Sync intervals of WebDAV, local folder and S3 sources are set per source in the web interface (see the [Sources Guide](../sources-guide.md)).

## PostgreSQL

Readur opens two connection pools per server process, with up to 20 connections for web requests and up to 30 for background work (OCR, sources, maintenance), so it can use up to 50 connections. PostgreSQL's default `max_connections` of 100 is enough for one Readur server; raise it if other applications share the database. The pool sizes are fixed.

Search uses PostgreSQL full-text search, and the tables and indexes Readur needs are created by its database migrations, which run automatically at startup. You do not need to add indexes yourself.

General PostgreSQL tuning applies. A starting point for a dedicated database server with 8 GB of RAM and SSD storage (adjust to your hardware):

```ini
# postgresql.conf
shared_buffers = 2GB
effective_cache_size = 6GB
work_mem = 16MB
maintenance_work_mem = 512MB
random_page_cost = 1.1
effective_io_concurrency = 200
```

With the official `postgres` image, these can be passed as `-c` options on the container command. Autovacuum, which is on by default, keeps the tables in shape; there is no Readur-specific maintenance job to schedule. To find slow queries, enable `log_min_duration_statement` or the `pg_stat_statements` extension in PostgreSQL.

## Storage

Keep `UPLOAD_PATH` and the PostgreSQL data directory on fast local storage (SSD) when you can. OCR writes temporary files to `/tmp`, so make sure it has room for the largest documents you process.

With S3 storage, every document read and write goes to the bucket, so latency to the S3 endpoint affects uploads, previews and OCR. Place the bucket (or your MinIO server) close to the Readur server. There are no S3 tuning settings; files over 100 MB are uploaded in 16 MB parts. See [Storage Configuration](./storage.md).

## Reverse Proxy

Readur does not compress responses. Enabling compression in your reverse proxy (for example `gzip on;` in NGINX) speeds up the web interface on slow links. Make sure the proxy accepts request bodies as large as `MAX_FILE_SIZE_MB` and allows long enough timeouts for large uploads; see [Reverse Proxy Configuration](./reverse-proxy.md).

## Scaling

Run one Readur server per database. Readur runs its OCR queue, watch folders and source syncs inside the server process and is not designed to run as several replicas behind a load balancer; scale it vertically by giving the server more CPU cores (and raising `CONCURRENT_OCR_JOBS`) and more memory.

In Docker Compose you can reserve or cap resources for the container:

```yaml
services:
  readur:
    image: ghcr.io/readur/readur:latest
    environment:
      CONCURRENT_OCR_JOBS: "8"
    deploy:
      resources:
        limits:
          cpus: "8"
          memory: 8G
```

If you cap memory, leave enough headroom for `CONCURRENT_OCR_JOBS` large documents at once, or lower the job count.

## Monitoring Performance

The admin metrics show where time goes. `GET /api/metrics` returns them as JSON and `/metrics` in Prometheus format (see [Monitoring](./monitoring.md)). Useful Prometheus metrics include:

| Metric | What it shows |
|--------|---------------|
| `readur_ocr_queue_pending` | Documents waiting for OCR |
| `readur_ocr_queue_processing` | Documents being OCRed now |
| `readur_ocr_queue_oldest_pending_minutes` | How long the oldest waiting document has waited |
| `readur_ocr_avg_processing_minutes` | Average OCR time per document |
| `readur_ocr_stuck_jobs` | OCR jobs stuck in the processing state |
| `readur_db_connections_active` | Database connections in use |
| `readur_db_response_time_ms` | Database response time |
| `readur_memory_usage_bytes` | Memory used by the Readur process |

A pending count that keeps growing means documents arrive faster than OCR finishes them: raise `CONCURRENT_OCR_JOBS` if CPU and memory allow, or give the server more cores.

## Troubleshooting

**OCR is slow or the queue keeps growing:** check CPU use with `docker stats`; if the CPU is not saturated, raise `CONCURRENT_OCR_JOBS`. Check whether users have selected several OCR languages or heavy image enhancement.

**Documents fail with a timeout:** raise `OCR_TIMEOUT_SECONDS`, then retry them with `POST /api/queue/requeue/failed`.

**The server runs out of memory during large imports:** lower `CONCURRENT_OCR_JOBS`, or pause the queue and let it work through the backlog outside busy hours.

**Large uploads fail:** raise `MAX_FILE_SIZE_MB` and the body size limit of your reverse proxy.

**More detail in the logs:** set `RUST_LOG=debug` (or `RUST_LOG=readur=debug` to leave dependencies quiet) and restart.

## Related Documentation

- [Configuration Reference](../configuration-reference.md)
- [Monitoring](./monitoring.md)
- [Storage Configuration](./storage.md)
- [Architecture Overview](../architecture.md)
