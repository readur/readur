# Troubleshooting Guide

This guide helps diagnose common problems with a Readur installation. The examples assume the Docker Compose setup from the [Deployment Guide](deployment.md), with services named `readur` and `postgres`; adjust the names to your setup.

## Quick Diagnosis

```bash
# Are both containers running and healthy?
docker compose ps

# Is the server answering? Expect {"status":"ok"}
curl -s http://localhost:8000/api/health

# Is PostgreSQL accepting connections?
docker compose exec postgres pg_isready -U readur

# Recent server log
docker compose logs --tail 200 readur

# Disk space for uploads and the database
df -h
```

Readur logs to standard output only; read the log with `docker compose logs` (or `kubectl logs`). At startup it prints every setting it loaded and whether the value came from the environment or is a default, which is the quickest way to check a configuration problem.

## Logging

| Setting | Effect |
|---------|--------|
| `RUST_LOG` | Log filter, default `info`. `RUST_LOG=debug` logs everything at debug level; `RUST_LOG=readur=debug` does so for Readur only |
| `DEBUG` | Adds extra diagnostic log lines when set to a value other than empty, `0` or `false` |
| `RUST_BACKTRACE` | `1` adds a backtrace to a crash report |

Set them in the `environment` of the `readur` service and recreate the container (`docker compose up -d`).

## Startup Problems

### The container exits right after starting

Read the last lines of the log (`docker compose logs readur`). Common messages:

- **`CRITICAL: Failed to connect to database`**: PostgreSQL is not reachable with the configured `DATABASE_URL` (or `POSTGRES_*` variables). See [Database connection refused](#database-connection-refused).
- **`CRITICAL: SQLx migrations failed!`**: the schema migrations, which run automatically at every start, could not be applied. See [Migration failures](#migration-failures).
- **`... initial-admin-password already exists; it may hold a previously generated password. Remove it, or set ADMIN_PASSWORD`**: Readur had to create the admin account (for example on a new, empty database) but a generated password file from an earlier installation is still in `/app/uploads/.readur/`. Remove the file or set `ADMIN_PASSWORD`.
- **`Failed to initialize storage backend`**: the upload directory is not writable (see [Permission denied](#permission-denied-when-the-container-writes-files)) or the S3 settings are wrong (see the [S3 Storage Guide](s3-storage-guide.md)).
- **An error naming a security setting** (for example `... has an unrecognized value ...; use true or false`, an invalid `TRUSTED_PROXIES` entry or an invalid `CORS_ALLOWED_ORIGINS` origin): fix the value; these settings stop the server when they are invalid.

### The port is already in use

```bash
sudo ss -ltnp | grep ':8000'
```

Change the host side of the port mapping (`"8080:8000"`), or the port Readur listens on with `SERVER_PORT`; then also change the container port in the mapping and the health check URL.

### A setting has no effect

- Check the name against the [Configuration Reference](configuration-reference.md); Readur ignores variables it does not know.
- Restart the container after the change; settings are read once at startup.
- A variable set in the environment (for example by Docker Compose) takes precedence over a `.env` file.
- An invalid number falls back to the default, with a warning in the startup log.
- `MEMORY_LIMIT_MB` and `CPU_PRIORITY` are only reported in the log; they do not change anything.

### `JWT_SECRET: ignored` in the log

The configured `JWT_SECRET` is shorter than 32 bytes or a published example value. Readur ignores it and signs sessions with the key stored in its database. Remove `JWT_SECRET` from the configuration, or set it to `openssl rand -hex 32` output.

## Database Issues

### Database connection refused

```bash
# Is PostgreSQL up?
docker compose ps postgres
docker compose logs --tail 50 postgres

# Can Readur's credentials connect? (uses the readur user and database)
docker compose exec postgres psql -U readur -d readur -c 'SELECT 1'
```

Check that:

- The host in `DATABASE_URL` is the Compose service name (`postgres`), not `localhost`; inside the `readur` container, `localhost` is the container itself.
- The port is the container port (5432), even if you publish PostgreSQL on another host port.
- The user, password and database match `POSTGRES_USER`, `POSTGRES_PASSWORD` and `POSTGRES_DB` of the `postgres` service. The postgres image applies those only when it initialises an empty data directory; changing them later does not change an existing database.
- Readur stops if it cannot connect at startup, so start it after PostgreSQL is ready: use `depends_on` with `condition: service_healthy` and a `pg_isready` health check, as in the [deployment example](deployment.md#production-docker-compose), together with a `restart` policy.

### Migration failures

Migrations run automatically every time the server starts; there is no separate migrate command. The log lists each migration as applied or pending, and the error of the one that failed.

```bash
# Migrations recorded in the database, newest first
docker compose exec postgres psql -U readur -d readur \
  -c 'SELECT version, description, success FROM _sqlx_migrations ORDER BY version DESC LIMIT 5;'
```

- **Permission errors** (for example on `CREATE EXTENSION "pg_trgm"`): the database user must own the database or otherwise be allowed to create the extension.
- **A migration's checksum does not match**, or the database has migrations the binary does not know: the image is older than the database (a downgrade). Run the image version the database was last used with, or restore a backup taken before the upgrade.
- Otherwise, keep the log and the database as they are, restore from backup if you need the instance running, and report the error.

### PostgreSQL fails after an automatic update

**Symptoms:** Readur worked until a container update; now the `postgres` container fails to start, or logs that the data directory was initialised by an incompatible version.

**Cause:** an auto-update tool (for example Watchtower or What's Up Docker) or an unpinned `postgres` tag moved PostgreSQL to a new major version. A PostgreSQL data directory can only be used by the major version that created it.

**Fix:** pin the image to the major version that created the data (for example `postgres:16`) and start it again; your data is intact. Then exclude the container from automatic updates, for example with labels:

```yaml
services:
  postgres:
    image: postgres:16
    labels:
      - "wud.trigger.exclude=docker.autoupdate"
      - "com.centurylinklabs.watchtower.monitor-only=true"
```

To move to a newer major version deliberately, follow the [PostgreSQL Upgrade Guide](postgres-upgrade.md). See also [GitHub Discussion #480](https://github.com/orgs/readur/discussions/480).

## Sign-in Problems

### Lost the admin password

When `ADMIN_PASSWORD` was not set, the initial password is in `/app/uploads/.readur/initial-admin-password` (in the Docker image) until you delete it:

```bash
docker compose exec readur cat /app/uploads/.readur/initial-admin-password
```

Otherwise reset it with the `reset-admin-password` command, which sets the password of the `ADMIN_USERNAME` account (default `admin`) to `ADMIN_PASSWORD`, or to a new random password printed to the terminal:

```bash
docker compose exec readur /app/readur reset-admin-password
```

### A user cannot sign in

- **Self-registered accounts** are created disabled. An administrator must enable them in user management.
- **Too many failed attempts:** after 10 failed passwords for one username from one address within 15 minutes, further attempts get `429 Too Many Requests` with a `Retry-After` header; see [Rate Limiting](api-reference.md#rate-limiting). Behind a reverse proxy, make sure `TRUSTED_PROXIES` covers the proxy so that clients are told apart by their own address.
- **Local login disabled:** with `ALLOW_LOCAL_AUTH=false`, only OIDC sign-in works.

### Everyone was signed out

Sessions end when the signing key changes: after `readur rotate-jwt-secret`, or when `JWT_SECRET` is set, changed or removed. Sessions also expire after `JWT_TTL_HOURS` (default 12). `POST /api/auth/logout` ends all of the user's sessions.

### OIDC sign-in fails

- `OIDC_REDIRECT_URI` must be exactly the redirect URI registered with the provider, normally `https://readur.example.com/api/auth/oidc/callback`.
- Set `PUBLIC_URL` to the address users open; Readur sends the browser there after sign-in.
- Check that the server can reach the issuer: `docker compose exec readur curl -s "$issuer/.well-known/openid-configuration"`, with `issuer` set to your `OIDC_ISSUER_URL`.
- An unknown user is only created when `OIDC_AUTO_REGISTER=true`. Linking to an existing local account by email needs `OIDC_LINK_EXISTING_BY_EMAIL=true` and a provider that reports the email as verified.
- The server log shows why a callback was rejected.

See the [OIDC Setup Guide](oidc-setup.md) for provider-specific settings.

## OCR Processing Issues

### Documents stay in the queue

```bash
# Queue counts (admin token or API key)
curl -s -H "Authorization: Bearer $token" http://localhost:8000/api/queue/stats

# Is OCR paused?
curl -s -H "Authorization: Bearer $token" http://localhost:8000/api/queue/status
```

- If `/api/queue/status` reports `"is_paused": true`, resume with `POST /api/queue/resume`.
- `pending` growing while `processing` stays at `CONCURRENT_OCR_JOBS` means OCR is busy, not stuck; see [Performance Tuning](performance-tuning.md).
- Jobs left in `processing` by a crash or restart are put back automatically: every 5 minutes (and once at startup), jobs that have been processing for more than 10 minutes return to `pending`, or become `failed` when their attempts are used up. No manual database update is needed.
- Documents that exist but never got a job (for example after a database restore) are queued with `POST /api/queue/enqueue/pending` (admin).

### OCR failed for a document

- The failed documents list on the **Intake** page, `GET /api/documents/failed` and `GET /api/documents/failed/ocr` show the failure reason for each document.
- `POST /api/documents/{id}/ocr/retry` runs OCR again, optionally with other languages (`{"languages": ["eng", "deu"]}`); `POST /api/documents/ocr/retry/bulk` retries a selection, and `POST /api/queue/requeue/failed` (admin) requeues failed jobs that have attempts left.
- `GET /api/documents/{id}/debug` (and, for administrators, the **Debug** section of the Settings page) shows the processing details of one document.

Common reasons:

| Failure reason | What to do |
|----------------|-----------|
| `ocr_timeout` | Raise `OCR_TIMEOUT_SECONDS` (keep it below 600; see [Performance Tuning](performance-tuning.md#choosing-ocr_timeout_seconds)) |
| `file_too_large` | A PDF over `MAX_PDF_SIZE_MB` (message `PDF file too large: ... MB (max: ... MB)`) or an Office document over `MAX_OFFICE_DOCUMENT_SIZE_MB`: raise the limit, or split the file |
| `file_corrupted`, `pdf_parsing_error` | The file could not be read; repair or re-export it and upload it again |
| `low_ocr_confidence` | No usable text was found: check the OCR language and the scan quality |
| `ocr_memory_limit` | The process ran out of memory: lower `CONCURRENT_OCR_JOBS` or give the container more memory |
| `unsupported_format` | The file type cannot be processed |

### OCR produces garbled text

- **Wrong language.** Each user chooses OCR languages on the Settings page; the server default is `OCR_LANGUAGE`. Retry the document with the right languages (`POST /api/documents/{id}/ocr/retry`).
- **Language not available.** `GET /api/ocr/languages` lists the installed languages. The Docker image includes all Tesseract language packs; on a source build, install the `tesseract-ocr-<lang>` packages and check with `tesseract --list-langs`.
- **Poor scans.** Try enabling or disabling **Enable Image Preprocessing** on the Settings page, or rescan at a higher resolution.
- **PDF text layer.** `debug_pdf_extraction`, a tool built from source, shows what each extraction method gets from a PDF; see [CLI Tools](administration/cli-tools.md).

`GET /api/ocr/health` checks that Tesseract and its language data work.

## Upload and Storage Issues

### Upload fails

| Response | Cause |
|----------|-------|
| `413 Payload Too Large` from Readur | The file is larger than `MAX_FILE_SIZE_MB` (default 50) |
| `413` from the reverse proxy (an HTML error page) | The proxy's own body limit, for example nginx `client_max_body_size` (1 MB by default) |
| `409 Conflict` | You already have a document with the same content |
| A server error, with a storage error in the log | The upload directory is full or not writable, or S3 is unreachable |

Check free space with `df -h` on the host directory mounted at `/app/uploads`.

### "read-only file system" when starting the container

Error such as `error while creating mount source path '...': mkdir /data: read-only file system`.

- **A relative path where an absolute one was meant.** `./DATA/AppData/readur/uploads` is created next to the compose file (in Portainer, next to Portainer's own working directory); `/DATA/AppData/readur/uploads` is the host directory.
- **A missing parent directory.** Create the directories first: `mkdir -p /DATA/AppData/readur/uploads /DATA/AppData/readur/watch`.
- **Case.** `/DATA` and `/data` are different directories on Linux.
- **A read-only filesystem.** Check with `mount | grep DATA`; some NAS systems mount certain paths read-only. Use another path.

See [Volume Path Syntax](deployment.md#volume-path-syntax).

### Permission denied when the container writes files

The Readur image runs as root unless you set `user:`, so with a plain bind mount this is rare. It happens when:

- you set `user:` in Compose, or run in Kubernetes (the Helm chart runs as UID 1000): the upload directory must be writable by that user, for example `sudo chown -R 1000:1000 /srv/readur/uploads`;
- the host path is on NFS or SMB with root squashing or restrictive share permissions.

Test from inside the container:

```bash
docker compose exec readur id
docker compose exec readur touch /app/uploads/.write-test
docker compose exec readur rm /app/uploads/.write-test
```

The watch folder only needs to be readable.

### Data disappeared after recreating the containers

- Make sure `/app/uploads` and the PostgreSQL data directory are on a bind mount or named volume; files written elsewhere in a container are lost when it is recreated.
- With `postgres:18` and later, mount the volume at `/var/lib/postgresql` (and set `PGDATA`, as in the repository's `docker-compose.yml`); with earlier versions, at `/var/lib/postgresql/data`.
- `docker compose down -v` **deletes named volumes**, including the database. Use `docker compose down` without `-v`.

```bash
docker volume ls
docker compose config | grep -A3 volumes
```

## Watch Folder and Sources

### Files in the watch folder are not imported

- Only extensions in `ALLOWED_FILE_TYPES` are imported (default `pdf,txt,doc,docx,png,jpg,jpeg`). Add others, such as `tiff`, if needed.
- Files last modified more than `MAX_FILE_AGE_HOURS` ago are skipped when that setting is set.
- A file is imported once its size has stayed the same for `FILE_STABILITY_CHECK_MS`.
- On NFS, SMB and similar mounts, changes are found by polling every `WATCH_INTERVAL_SECONDS`. If your mount is not recognised as a network mount, set `FORCE_POLLING_WATCH=1`.
- A file whose content you already have is not imported again.
- The log shows what the watcher does; set `RUST_LOG=debug` for more.

See [Watch Folder](WATCH_FOLDER.md).

### A source does not sync

- `POST /api/sources/{id}/test` checks the connection of a saved source.
- `GET /api/source/errors` lists scan failures with their reason. Failures are retried later; `POST /api/source/errors/{failure_id}/retry` retries one now and `POST /api/source/errors/{failure_id}/exclude` stops retrying it.
- Files larger than `MAX_FILE_SIZE_MB` are skipped, with a warning in the log.
- A local folder source must be inside `LOCAL_SOURCE_ALLOWED_PATHS` when that is set; when it is not set, only administrators can create local folder sources.

See the [Sources Guide](sources-guide.md).

## Reverse Proxy Issues

- **Live sync progress does not update:** the proxy must pass WebSocket upgrades for `GET /api/sources/{id}/sync/progress/ws` (nginx: `proxy_http_version 1.1` and the `Upgrade` and `Connection` headers), and keep the `Host` header or set `PUBLIC_URL`, because the browser's `Origin` must match one of them.
- **OIDC redirects or shared links point to the wrong address:** set `PUBLIC_URL`.
- **Large uploads fail:** raise the proxy's body size limit.

See [Reverse Proxy](REVERSE_PROXY.md) and [SSL/HTTPS Setup](deployment.md#sslhttps-setup).

## Performance Issues

### Slow search

Readur creates the full-text and trigram indexes it needs. If search becomes slow after a large import, refresh PostgreSQL's planner statistics:

```bash
docker compose exec postgres psql -U readur -d readur -c 'VACUUM (ANALYZE);'
```

and check PostgreSQL's memory settings; see [Performance Tuning](performance-tuning.md#postgresql).

### High memory or CPU use

- Lower `CONCURRENT_OCR_JOBS`; each running OCR job uses about one core and the memory of the page or image it reads.
- Limit the container (`deploy.resources.limits` in Compose, `resources.limits` in Kubernetes).
- Pause OCR with `POST /api/queue/pause` while the machine is needed for something else.

## Collecting Information for a Bug Report

```bash
docker compose ps > readur-diagnostics.txt
docker compose logs --tail 500 readur >> readur-diagnostics.txt
docker compose logs --tail 100 postgres >> readur-diagnostics.txt
```

Remove passwords, tokens and host names before sharing the log. Include the image tag, how you deploy (Compose, Kubernetes, NAS) and the steps that lead to the problem.

## Getting Help

- [GitHub Issues](https://github.com/readur/readur/issues) for bugs
- [GitHub Discussions](https://github.com/orgs/readur/discussions) for questions
