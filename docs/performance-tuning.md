# Performance Tuning Guide

Readur has few performance settings. Most of the work is OCR, so the main levers are how many documents are OCRed at once, how long one document may take, how large a file is accepted, and how much CPU and memory the container gets. PostgreSQL is tuned in PostgreSQL itself.

All server settings are environment variables, read at startup; restart Readur after changing one. The [Configuration Reference](configuration-reference.md) lists every variable.

## How Processing Works

- Every new document (upload, watch folder, source sync) gets a job in the OCR queue. One worker loop takes jobs in priority order and runs up to `CONCURRENT_OCR_JOBS` of them at the same time.
- Images are read with Tesseract (linked as a library). The Tesseract call runs on a blocking thread, so each running job can keep one CPU core busy.
- PDFs without embedded images are read with `pdftotext` first, which is fast. If that yields too little usable text, or the PDF contains images, the pages are rendered and OCRed, and `ocrmypdf` is the last fallback. Readur starts `ocrmypdf` without a `--jobs` option, so `ocrmypdf` uses its own default and may use several cores for one document.
- Office documents and plain text are extracted without OCR.
- The server's internal thread counts are fixed in the code (separate async runtimes with 3 threads for the OCR worker, 2 for source syncs and maintenance, 2 for database work). There are no settings for them; `CONCURRENT_OCR_JOBS` is what controls OCR parallelism.
- Readur opens two database connection pools: up to 20 connections for web requests and up to 30 for background work.

## Server Settings

| Variable | Default | Effect |
|----------|---------|--------|
| `CONCURRENT_OCR_JOBS` | `4` | Documents OCRed in parallel. Values are clamped to 1-15. Each running job needs roughly one CPU core and the memory of one decoded page or image |
| `OCR_TIMEOUT_SECONDS` | `300` | Time limit for the OCR of one document whose owner has not saved their own **OCR Timeout**. A job that exceeds it fails; the queue tries it again until its attempts (3 by default) are used up |
| `MAX_FILE_SIZE_MB` | `50` | Largest request body, so the largest upload; a user's own **Max File Size** can only lower it. Source syncs skip files larger than this |
| `MAX_PDF_SIZE_MB` | `100` | Larger PDFs are not processed for text |
| `MAX_OFFICE_DOCUMENT_SIZE_MB` | `100` | Larger Office documents are not processed for text |
| `OCR_LANGUAGE` | `eng` | Default OCR language. Every extra language in a combination (such as `eng+deu`) makes Tesseract slower |

`MEMORY_LIMIT_MB` and `CPU_PRIORITY` are only printed in the startup log; they do not limit memory or change process priority. Use container limits instead (below).

### Choosing `CONCURRENT_OCR_JOBS`

- Start with the number of CPU cores you want Readur to use, and lower it if the host runs out of memory or other services become slow.
- On small hosts (2 cores, 2-4 GB of RAM, low-end NAS models) use 1 or 2.
- Raising it beyond the number of cores does not make OCR faster.

### Choosing `OCR_TIMEOUT_SECONDS`

- Large scanned PDFs take the longest. If they fail with a timeout, raise the limit.
- Every 5 minutes, Readur resets jobs that have been in `processing` for more than 10 minutes back to `pending` (or to `failed` once their attempts are used up), to recover jobs from a crashed or restarted server. A job that legitimately runs longer than 10 minutes is caught by this too, so OCR timeouts (the server's and each user's) stop at 600 seconds: a larger value acts as 600.

## Per-User OCR Settings

Each user's OCR options are stored in the database and edited on the **Settings** page (or with `PUT /api/settings`). These affect the quality and the cost of OCR for that user's documents:

- **OCR languages** (`ocr_language`, `preferred_languages`, `primary_language`): fewer languages means faster OCR.
- **Enable Image Preprocessing** (`enable_image_preprocessing`): when on, images are analysed and, if they fall below the quality thresholds, enhanced (brightness, contrast, noise reduction, sharpening) before OCR. Turning it off saves CPU time on clean scans.
- **Skip enhancement** (`ocr_skip_enhancement`) and the quality thresholds (`ocr_quality_threshold_*`): decide when preprocessing enhances an image.
- **Maximum image size** and **upscale factor** (`ocr_max_image_width`, `ocr_max_image_height`, `ocr_upscale_factor`): preprocessing scales images to fit within the maximum size (never above 2048 pixels on the long edge), then by the upscale factor.
- **Page segmentation mode** and **OCR engine mode** (`ocr_page_segmentation_mode`, 0-13; `ocr_engine_mode`, 0-3): passed to Tesseract for images and PDF pages, and to ocrmypdf as `--tesseract-pagesegmode` and `--tesseract-oem`. Engine modes 0 and 2 need Tesseract's legacy language data; without it OCR of that user's documents fails with an error naming the engine mode.
- **DPI** (`ocr_dpi`, 70-1200, default 300): the resolution PDF pages are rendered at for OCR, and the resolution Tesseract assumes for images. Higher values cost time and memory roughly with their square.
- **Character whitelist / blacklist** (`ocr_whitelist_chars`, `ocr_blacklist_chars`): restrict the characters Tesseract may recognise, in every OCR path.
- **OCR Timeout** (`ocr_timeout_seconds`, 10-600): the time limit for each of the user's documents, in place of `OCR_TIMEOUT_SECONDS`.
- **Max File Size** (`max_file_size_mb`, 1-500, empty for the server's limit): the largest file the user may upload. `MAX_FILE_SIZE_MB` still caps it.
- **Enable Background OCR** (`enable_background_ocr`): when off, the user's uploads and synced files are stored but not queued for OCR; retry OCR on a document to process it.
- **Save processed images** (`save_processed_images`): keeps the preprocessed image for each document, which costs disk space.

How many documents are OCRed at once is server-wide (`CONCURRENT_OCR_JOBS`), so it is not a user setting.

## Container Resources

Readur has no memory or CPU limit of its own. Limit the container instead, for example in Docker Compose:

```yaml
services:
  readur:
    image: ghcr.io/readur/readur:latest
    environment:
      CONCURRENT_OCR_JOBS: "2"
    deploy:
      resources:
        limits:
          cpus: "2.0"
          memory: 2G
```

or with resource limits in Kubernetes (see the Helm values example in the [Deployment Guide](deployment.md#kubernetes)). If the container is killed for using too much memory, lower `CONCURRENT_OCR_JOBS` rather than only raising the limit.

Temporary files from PDF OCR are written to `/tmp` in the container; make sure it has room for a few copies of your largest PDF.

## PostgreSQL

Readur creates its own tables and indexes through its migrations, including full-text (GIN) and trigram indexes on document content and file names; you do not need to add indexes. Tune PostgreSQL in PostgreSQL, for example with `-c` options on the official image:

```yaml
services:
  postgres:
    image: postgres:18
    command:
      - postgres
      - -c
      - shared_buffers=512MB
      - -c
      - effective_cache_size=2GB
      - -c
      - work_mem=16MB
      - -c
      - maintenance_work_mem=256MB
```

- A common starting point is `shared_buffers` at about a quarter of the memory you give PostgreSQL and `effective_cache_size` at about three quarters.
- `max_connections` must cover Readur's two pools (up to 50 connections per Readur process) plus your own tools. The PostgreSQL default of 100 is enough for one Readur instance.
- Leave autovacuum on. After a large import you can run `VACUUM (ANALYZE);` in `psql` to refresh planner statistics immediately.
- Put the database on SSD storage if you can; full-text search reads its indexes from disk.

## Monitoring Throughput

- `GET /api/queue/stats` (admin) returns the queue counts: `pending`, `processing`, `failed`, `completed_today`, `avg_wait_time_minutes` and `oldest_pending_minutes`. A steadily growing `pending` count means OCR cannot keep up with incoming documents.
- `POST /api/queue/pause` and `POST /api/queue/resume` (admin) pause and resume OCR, for example during a large import or a backup.
- `GET /api/metrics` (JSON) and `/metrics` (Prometheus text) report database, OCR, document, user and system figures; see [Health, Metrics and MCP Endpoints](api-reference.md#health-metrics-and-mcp-endpoints).
- Set `RUST_LOG=debug` for detailed processing logs, or `RUST_LOG=readur=debug` to keep dependencies quiet.

## Large Imports

- For a one-off import of many files, use a watch folder or `batch_ingest` (see [CLI Tools](administration/cli-tools.md)) rather than many parallel uploads.
- A file whose content the user already has (same SHA-256 hash) is not stored or processed again.
- Pausing the queue (`POST /api/queue/pause`) lets files be ingested first and OCRed later.

## Related Documentation

- [Configuration Reference](configuration-reference.md)
- [Troubleshooting](troubleshooting.md)
- [Deployment Guide](deployment.md)
