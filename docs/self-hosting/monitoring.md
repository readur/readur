# Monitoring

Readur exposes three things you can monitor from outside:

| What | Where | Access |
|------|-------|--------|
| Liveness check | `GET /api/health` | public |
| Metrics in Prometheus text format | `GET /metrics` | admin session or admin API key, or `Authorization: Bearer <METRICS_TOKEN>` |
| Metrics as JSON | `GET /api/metrics` | admin session or admin API key |

Logs are written to standard output. There is no built-in alerting, dashboard or notification sender; use the endpoints below with the monitoring tools you already run.

## Health Check

```bash
curl -f http://localhost:8000/api/health
```

```json
{
  "status": "ok"
}
```

The endpoint answers `200 OK` as long as the HTTP server is running. It does not query the database or the storage backend, so it tells you the process is up, not that everything behind it works. To check the database too, scrape `/metrics` or call `/api/metrics`: both run database queries and return `500` when the database cannot be reached.

The bundled `docker-compose.yml` already uses the health endpoint as the container health check:

```yaml
healthcheck:
  test: ["CMD", "curl", "-f", "http://localhost:8000/api/health"]
  interval: 30s
  timeout: 10s
  retries: 3
  start_period: 40s
```

`curl` is included in the Readur image, so this works without extra tools. The same URL works for uptime checkers and for Kubernetes liveness and readiness probes.

## Prometheus Metrics

`GET /metrics` returns the metrics in the Prometheus text exposition format (`text/plain; version=0.0.4`). Every figure is computed when the endpoint is called, mostly from database queries; Readur keeps no metric history of its own.

### Authentication

Set `METRICS_TOKEN` (at least 16 characters) and let Prometheus send it as a bearer token:

```bash
openssl rand -hex 32
```

Without `METRICS_TOKEN`, the endpoint accepts only an admin session token or an admin API key. A missing credential gets `401`, a non-admin one `403`.

```bash
curl -H "Authorization: Bearer $metrics_token" http://localhost:8000/metrics
```

### Scrape Configuration

```yaml
# prometheus.yml
scrape_configs:
  - job_name: readur
    metrics_path: /metrics
    scrape_interval: 60s
    authorization:
      type: Bearer
      # File containing the value of METRICS_TOKEN
      credentials_file: /etc/prometheus/readur-metrics-token
    static_configs:
      - targets:
          - readur.example.com:8000
```

Each scrape runs a set of database queries, so a scrape interval of a minute or more is sensible. Every sample carries an explicit timestamp (milliseconds since the epoch), which Prometheus accepts.

### Metric Reference

All metrics are gauges unless marked as counters. "Today" means since midnight of the current date in the database server's time zone.

**Documents and storage**

| Metric | Meaning |
|--------|---------|
| `readur_documents_total` | Number of documents |
| `readur_documents_uploaded_today` | Documents created today |
| `readur_storage_bytes` | Sum of the file sizes of all documents |
| `readur_avg_document_size_bytes` | Average document file size |
| `readur_documents_with_ocr` | Documents that have extracted text |
| `readur_documents_without_ocr` | Documents without extracted text |
| `readur_documents_by_type{type="..."}` | Documents per file extension; `type` is `pdf`, `jpeg`, `png`, `gif`, `tiff` or `other` |

**OCR queue**

| Metric | Meaning |
|--------|---------|
| `readur_ocr_queue_pending` | OCR jobs waiting |
| `readur_ocr_queue_processing` | OCR jobs running |
| `readur_ocr_queue_failed` | OCR jobs that failed |
| `readur_ocr_queue_depth` | Pending plus processing jobs |
| `readur_ocr_completed_today` | OCR jobs completed today |
| `readur_ocr_avg_processing_minutes` | Average time jobs waited in the queue. Omitted while there is no value |
| `readur_ocr_queue_oldest_pending_minutes` | Age of the oldest document waiting for OCR. Omitted while none is waiting |
| `readur_ocr_stuck_jobs` | Documents in the `processing` state that were last updated more than 30 minutes ago |
| `readur_ocr_confidence_score` | Average OCR confidence of documents completed in the last hour. Omitted when there are none |

**Users**

| Metric | Meaning |
|--------|---------|
| `readur_users_total` | Number of user accounts |
| `readur_users_registered_today` | Accounts created today |
| `readur_users_active_today` | Users who added at least one document today |

**Database**

| Metric | Meaning |
|--------|---------|
| `readur_db_connections_total` | Connections currently open in Readur's pool |
| `readur_db_connections_idle` | Idle connections in the pool |
| `readur_db_connections_active` | Connections in use |
| `readur_db_utilization_percent` | Active connections as a percentage of open connections |
| `readur_db_response_time_ms` | Time a `SELECT 1` took during this scrape |

**Data consistency**

| Metric | Meaning |
|--------|---------|
| `readur_data_consistency_score` | 100 when the OCR queue and the documents agree; 10 points are subtracted for each queue entry whose document no longer exists and each document marked completed whose queue entry is not |

**Placeholders**

These metrics are emitted but do not measure anything yet. Do not alert on them.

| Metric | Value |
|--------|-------|
| `readur_uptime_seconds` (counter) | Always `3600` |
| `readur_memory_usage_bytes` | Always `0` |
| `readur_storage_usage_percent` | Always `0` |
| `readur_failed_logins_today` (counter) | Always `0` |
| `readur_document_access_today` (counter) | Documents created today (the same figure as `readur_documents_uploaded_today`), not document accesses |
| `readur_webdav_sessions_total`, `readur_webdav_sessions_successful`, `readur_webdav_sessions_failed`, `readur_webdav_success_rate`, `readur_webdav_files_processed`, `readur_webdav_bytes_processed`, `readur_webdav_avg_session_duration_seconds`, `readur_webdav_avg_processing_rate`, `readur_webdav_http_requests_total`, `readur_webdav_request_success_rate`, `readur_webdav_avg_request_duration_ms`, `readur_webdav_sessions_active_last_hour`, `readur_webdav_error_rate_last_hour` | Always `0`: WebDAV syncs do not report to this collector. Check the Sources page instead |

### Example Alert Rules

These rules only use metrics that carry real values. Adjust the thresholds to your workload.

```yaml
# readur-alerts.yml
groups:
  - name: readur
    rules:
      - alert: ReadurMetricsDown
        expr: up{job="readur"} == 0
        for: 5m
        annotations:
          summary: "Readur metrics cannot be scraped (server down, database unreachable or bad token)"
      - alert: ReadurOcrStuckJobs
        expr: readur_ocr_stuck_jobs > 0
        for: 15m
        annotations:
          summary: "Documents have been in OCR processing for more than 30 minutes"
      - alert: ReadurOcrBacklog
        expr: readur_ocr_queue_oldest_pending_minutes > 60
        for: 15m
        annotations:
          summary: "A document has been waiting for OCR for over an hour"
      - alert: ReadurOcrFailures
        expr: readur_ocr_queue_failed > 0
        for: 1h
        annotations:
          summary: "OCR jobs have been in the failed state for an hour"
      - alert: ReadurDataConsistency
        expr: readur_data_consistency_score < 100
        for: 30m
        annotations:
          summary: "OCR queue and documents disagree"
```

Because a failing database query makes `/metrics` return `500`, a database outage shows up as `up == 0` for the Readur job.

## JSON Metrics

`GET /api/metrics` (admin only) returns a smaller set of figures as JSON:

```bash
curl -H "Authorization: Bearer $admin_token" http://localhost:8000/api/metrics
```

```json
{
  "database": {
    "active_connections": 5,
    "total_queries_today": 0,
    "avg_query_time_ms": 0.0
  },
  "ocr": {
    "pending_jobs": 3,
    "processing_jobs": 1,
    "failed_jobs": 0,
    "completed_today": 42,
    "avg_processing_time_minutes": 1.5,
    "queue_depth": 4,
    "oldest_pending_minutes": 2.0
  },
  "documents": {
    "total_documents": 1250,
    "documents_uploaded_today": 42,
    "total_storage_bytes": 734003200,
    "avg_document_size_bytes": 587202.56,
    "documents_with_ocr": 1200,
    "documents_without_ocr": 50
  },
  "users": {
    "total_users": 4,
    "active_users_today": 2,
    "new_registrations_today": 0
  },
  "system": {
    "uptime_seconds": 3600,
    "app_version": "3.1.3",
    "rust_version": "unknown"
  },
  "timestamp": 1763380800
}
```

- `database.active_connections` is the number of connections open in the pool. `total_queries_today` and `avg_query_time_ms` are always `0`.
- `ocr.avg_processing_time_minutes` is the average queue wait time, like `readur_ocr_avg_processing_minutes`; it and `oldest_pending_minutes` are `null` when there is no value.
- `system.uptime_seconds` is always `3600`; `app_version` is the Readur version.
- `timestamp` is in Unix seconds.

The OCR queue alone is also available from `GET /api/queue/stats` (see [OCR and Queue Endpoints](../api-reference.md#ocr-and-queue-endpoints)).

## Logs

Readur logs to standard output in plain text; it writes no log files. Read the logs with your container runtime:

```bash
docker compose logs -f readur
kubectl logs -f <readur-pod-name>
```

`RUST_LOG` sets the log level using the tracing `EnvFilter` syntax:

```yaml
environment:
  RUST_LOG: info                    # the default
  # RUST_LOG: debug                 # everything at debug level
  # RUST_LOG: readur=debug,sqlx=warn
```

Without `RUST_LOG`, Readur logs at `info` and quietens two noisy dependencies (the PDF extractor and PostgreSQL notices). Setting `RUST_LOG` replaces that default entirely. To collect logs centrally, point your log shipper (Loki, Fluent Bit, Vector and so on) at the container output.

## Things Worth Watching

- **Is it up?** `GET /api/health` from an uptime checker, plus `up{job="readur"}` in Prometheus.
- **Is OCR keeping up?** `readur_ocr_queue_pending`, `readur_ocr_queue_oldest_pending_minutes` and `readur_ocr_stuck_jobs`. Failed jobs can be put back in the queue with `POST /api/queue/requeue/failed`.
- **Is the disk filling up?** Readur does not report free space (`readur_storage_usage_percent` is a placeholder). Watch the volume behind `UPLOAD_PATH` and the PostgreSQL data volume with node-exporter or your platform's volume metrics.
- **Source syncs:** the WebDAV metrics are placeholders; source health and errors are shown on the Sources page.

## Related Documentation

- [Health Monitoring Guide](../health-monitoring-guide.md)
- [Configuration Reference](../configuration-reference.md)
- [API Reference](../api-reference.md#health-metrics-and-mcp-endpoints)
- [Backup Strategies](./backup.md)
- [Performance Tuning](./performance.md)
