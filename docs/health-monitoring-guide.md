# Health Monitoring Guide

Readur offers these ways to check that it is working:

- a liveness endpoint for load balancers and container health checks,
- an OCR health endpoint,
- metrics in JSON and in Prometheus format,
- a periodic health check of every source, shown in the web interface.

Readur has no built-in alerting (no email or webhook alerts) and no monitoring settings. To be alerted, scrape `/metrics` with Prometheus and alert there, or poll the endpoints below from your monitoring system. For a wider view of running Readur in production, see [Monitoring](self-hosting/monitoring.md).

## Health check endpoints

| Endpoint | Access | What it tells you |
|----------|--------|-------------------|
| `GET /api/health` | public | The HTTP server is up and answering. It does not check the database |
| `GET /api/ocr/health` | any signed-in user | Whether Tesseract is installed and which language data it has |
| `GET /api/metrics` | admin | Database, OCR queue, document, user and system figures as JSON |
| `GET /metrics` | admin, or the `METRICS_TOKEN` bearer token | The figures in Prometheus text format |
| `POST /api/sources/{id}/validate` | owner of the source | Starts a health check of one source in the background |

See [Health, Metrics and MCP Endpoints](api-reference.md#health-metrics-and-mcp-endpoints) in the API Reference.

### Liveness: `/api/health`

```bash
curl -s http://localhost:8000/api/health
```

```json
{
  "status": "ok"
}
```

The bundled `docker-compose.yml` uses this endpoint as the container health check:

```yaml
healthcheck:
  test: ["CMD", "curl", "-f", "http://localhost:8000/api/health"]
  interval: 30s
  timeout: 10s
  retries: 3
```

Because `/api/health` does not touch the database, use `/api/metrics` or `/metrics` (which do) if you also want to know that the database answers.

### OCR: `/api/ocr/health`

```bash
curl -s http://localhost:8000/api/ocr/health -H "Authorization: Bearer $token"
```

The response has `status` (`healthy` or `unhealthy`), `tesseract_installed`, `available_languages` (the Tesseract language codes found), `diagnostics` and `errors` (a list of problems when unhealthy).

### Metrics: `/api/metrics` and `/metrics`

`GET /api/metrics` (administrators) returns a JSON object with these sections:

| Section | Fields |
|---------|--------|
| `database` | `active_connections`, `total_queries_today`, `avg_query_time_ms` |
| `ocr` | `pending_jobs`, `processing_jobs`, `failed_jobs`, `completed_today`, `avg_processing_time_minutes`, `queue_depth`, `oldest_pending_minutes` |
| `documents` | `total_documents`, `documents_uploaded_today`, `total_storage_bytes`, `avg_document_size_bytes`, `documents_with_ocr`, `documents_without_ocr` |
| `users` | `total_users`, `active_users_today`, `new_registrations_today` |
| `system` | `uptime_seconds`, `app_version`, `rust_version` |

It also has a `timestamp` (Unix seconds).

`GET /metrics` serves similar figures for Prometheus. It accepts an administrator's session token or API key, or, when `METRICS_TOKEN` is set, `Authorization: Bearer <METRICS_TOKEN>`. All metric names start with `readur_`; useful ones for alerting include:

| Metric | Meaning |
|--------|---------|
| `readur_ocr_queue_pending` | OCR jobs waiting |
| `readur_ocr_queue_processing` | OCR jobs running |
| `readur_ocr_queue_failed` | OCR jobs that failed |
| `readur_ocr_queue_oldest_pending_minutes` | Age of the oldest waiting OCR job |
| `readur_ocr_stuck_jobs` | OCR jobs stuck in the processing state |
| `readur_db_connections_active` | Database connections in use |
| `readur_db_response_time_ms` | Database response time |
| `readur_storage_bytes` | Total size of stored documents |
| `readur_uptime_seconds` | Seconds since the server started |

Run `curl -s -H "Authorization: Bearer $token" http://localhost:8000/metrics` to see the full list with the help text of each metric. A Prometheus scrape job:

```yaml
scrape_configs:
  - job_name: readur
    metrics_path: /metrics
    authorization:
      type: Bearer
      credentials: "the value of METRICS_TOKEN"
    static_configs:
      - targets: ["readur.example.com:8000"]
```

## Source health checks

Readur checks the health of every enabled source:

- after each sync of the source that was not cancelled,
- about every 30 minutes otherwise (the scheduler looks once a minute for sources whose last check is older than 30 minutes),
- when you click **Check health** in the source's details in the web interface, or call `POST /api/sources/{id}/validate`.

A check starts at a score of 100 and subtracts points for each problem it finds:

| Check | Problem | Points |
|-------|---------|--------|
| Configuration | The source's configuration is invalid (marks the source `critical`) | 30 |
| Connectivity | The WebDAV server, S3 bucket or local folder cannot be reached | 25 |
| Sync pattern | More than 80% of recent syncs found no files | 20 |
| Sync pattern | Fewer than two syncs, and the source is more than a day old | 10 |
| Errors | The source reported an error in the last 24 hours | 15 |
| Errors | The source is currently in the error state | 25 |

The result is stored on the source as `validation_status` (`healthy`, `warning` or `critical`), `validation_score` (0-100), `validation_issues` (a JSON list of the problems, each with a `type`, `severity`, `message` and `recommendation`) and `last_validation_at`. These fields are part of every source returned by `GET /api/sources` and `GET /api/sources/{id}`. In the web interface, the source's details show the status and score, its problems are listed there, and sources with a warning or critical status are marked in the source lists.

When a check ends `critical` or with a score below 50, the source's owner also gets a server notification ("Source Validation Failed" or "Source Validation Warning"; see the [Notifications Guide](notifications-guide.md)).

## Troubleshooting

- **`/api/health` does not answer:** the server is not running or not reachable. Check the container logs (`docker compose logs readur`) and that port 8000 is published or proxied.
- **`/api/ocr/health` is `unhealthy`:** read `errors`. A missing language usually means its Tesseract language data is not installed in the image.
- **`readur_ocr_queue_pending` keeps growing:** OCR cannot keep up. Raise `CONCURRENT_OCR_JOBS` if the machine has spare CPU, and look for failing jobs in the logs.
- **A source shows `warning` or `critical`:** open the source's details to read the problems and their recommendations, fix the cause (credentials, URL, folder permissions), then click **Check health** again.
- **`/metrics` answers `401` or `403`:** send `Authorization: Bearer <METRICS_TOKEN>`, or an administrator's token or API key.
