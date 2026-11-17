# OCR Queue

How OCR work is queued and processed. The code is in `src/ocr/queue.rs` (worker and maintenance), `src/ocr/queue_enqueue.rs` (adding jobs) and `src/ocr/priority.rs` (priorities).

## The queue table

OCR jobs are rows in the `ocr_queue` table (created in `migrations/20250620100001_add_ocr_queue.sql`, with later additions):

```sql
CREATE TABLE IF NOT EXISTS ocr_queue (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id UUID REFERENCES documents(id) ON DELETE CASCADE,
    status VARCHAR(20) DEFAULT 'pending',
    priority INT DEFAULT 5,
    attempts INT DEFAULT 0,
    max_attempts INT DEFAULT 3,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    error_message TEXT,
    worker_id VARCHAR(100),
    processing_time_ms INT,
    file_size BIGINT,
    CONSTRAINT check_status CHECK (status IN ('pending', 'processing', 'completed', 'failed', 'cancelled'))
);
```

`progress_current` and `progress_total` (page-level progress) were added in `migrations/20250916000001_add_ocr_progress_fields.sql`.

A document has at most one pending or processing job. This is enforced by the partial unique index `idx_ocr_queue_unique_pending_document`; every enqueue uses `ON CONFLICT ... DO NOTHING`, so enqueueing a document that is already queued returns `EnqueueOutcome::AlreadyQueued` instead of adding a second job.

The document's own OCR state is kept on the `documents` row (`ocr_status`, `ocr_error`, `ocr_failure_reason`, and the OCR text and metadata).

## Priorities

Workers claim the highest `priority` first, then the oldest `created_at`. Priorities (`src/ocr/priority.rs`):

| Queued by | Up to 1 MB | Up to 5 MB | Up to 10 MB | Up to 50 MB | Larger |
|-----------|-----------|-----------|------------|------------|--------|
| Automatic ingestion (watch folder, batch import, source sync) | 10 | 8 | 6 | 4 | 2 |
| Bulk OCR retry | 15 | 12 | 10 | 8 | 6 |
| User action (upload, single-document retry) | 5 | 5 | 5 | 5 | 5 |

## The worker

`main.rs` starts one worker (`OcrQueueService::start_worker`) on a dedicated OCR runtime. Its loop:

1. If processing is paused, sleep 5 seconds and check again.
2. Claim a job in a transaction: select the next claimable job (`status = 'pending' AND attempts < max_attempts`, ordered by priority and age) with `FOR UPDATE SKIP LOCKED`, set it to `processing` with `started_at`, `worker_id` and `attempts + 1`, and set the document's `ocr_status` from `pending` to `processing`. If another worker took the job in between, retry up to 3 times.
3. If there was nothing to claim, sleep 5 seconds.
4. Otherwise wait for a slot (a semaphore sized to the concurrency limit) and process the job in a new task.

The concurrency limit is `CONCURRENT_OCR_JOBS` (default 4), clamped to 1-15. Each job is processed within `OCR_TIMEOUT_SECONDS` (default 300); PDFs larger than `MAX_PDF_SIZE_MB` and Office documents larger than `MAX_OFFICE_DOCUMENT_SIZE_MB` are not processed for text. The worker ID is `worker-<hostname>-<uuid>`.

### Success and failure

- On success the job becomes `completed` with `completed_at` and `processing_time_ms`, and the OCR result is written to the document.
- On failure the error is stored in `error_message`. If the job still has attempts left (`attempts < max_attempts`) it goes back to `pending` and is retried on a later loop; otherwise it becomes `failed` and `OCR job ... permanently failed after max attempts` is logged. Retries are not delayed.
- Results that fail the quality check (`validate_ocr_quality` in `src/ocr/enhanced.rs`: for example confidence below 5% or no text at all) mark the document as failed with `ocr_failure_reason = 'low_ocr_confidence'`. Confidence below the user's `ocr_min_confidence` setting is only logged as a warning.

### Pausing

Admins can pause and resume processing with `POST /api/queue/pause` and `POST /api/queue/resume`; `GET /api/queue/status` reports the state. Pausing stops new jobs from being claimed; jobs already running finish. The paused state is held in memory and resets to running when the server restarts.

## Maintenance

A background task in `main.rs` runs every 5 minutes:

- **Stale jobs**: jobs that have been `processing` for more than 10 minutes (for example after a crash) are reset to `pending`, or set to `failed` if they have no attempts left (`recover_stale_items(10)`).
- **Cleanup**: `completed` jobs whose `completed_at` is more than 7 days old are deleted (`cleanup_completed(7)`).

## API

All queue endpoints require an admin. Full details are in the [API Reference](../api-reference.md).

| Endpoint | What it does |
|----------|--------------|
| `GET /api/queue/stats` | Queue counts and wait times |
| `GET /api/queue/status` | Whether processing is paused |
| `POST /api/queue/pause` | Pause processing |
| `POST /api/queue/resume` | Resume processing |
| `POST /api/queue/requeue/failed` | Reset `failed` jobs that still have attempts left to `pending` with `attempts = 0`; returns `{"requeued_count": 3}` |
| `POST /api/queue/enqueue/pending` | Queue documents that have no OCR job yet |

`GET /api/queue/stats` comes from the SQL function `get_queue_statistics()`:

```json
{
  "pending": 0,
  "processing": 0,
  "failed": 0,
  "completed_today": 0,
  "avg_wait_time_minutes": null,
  "oldest_pending_minutes": null
}
```

## Batch ingestion tool

`batch_ingest` (`src/bin/batch_ingest.rs`) imports a directory for one user and queues the files for OCR. It is built from source and not included in the Docker image:

```bash
cargo run --release --bin batch_ingest -- /path/to/files \
  --user-id 550e8400-e29b-41d4-a716-446655440000 --monitor
```

Options: `-u`/`--user-id <UUID>` (required) and `-m`/`--monitor` (keep running and print queue progress after the import). It uses the server's environment variables (database, `UPLOAD_PATH`, storage). Files are processed in batches of 1000 with at most 50 concurrent file reads; these limits are fixed in `src/ingestion/batch_ingest.rs`.

## Useful queries

```sql
-- Failed jobs, newest first
SELECT id, document_id, attempts, max_attempts, error_message, created_at
FROM ocr_queue
WHERE status = 'failed'
ORDER BY created_at DESC;

-- Same numbers as GET /api/queue/stats
SELECT * FROM get_queue_statistics();
```
