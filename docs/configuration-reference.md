# Configuration Reference

Readur is configured with environment variables, read once at startup; change one and restart the server for it to take effect. A `.env` file in the working directory is loaded first, and variables set in the environment take precedence over it. There is no configuration file and there are no command-line options for these settings. Per-user preferences (OCR language, search defaults and so on) are set in the web interface instead and stored in the database.

Every variable Readur reads is listed below; anything not listed here is ignored. The [Configuration Guide](configuration.md) has worked examples.

## Environment Variables

### Server and Database

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `DATABASE_URL` | String | built from the `POSTGRES_*` variables | PostgreSQL connection string (`postgresql://` or `postgres://`). Takes priority over the `POSTGRES_*` variables |
| `POSTGRES_HOST` | String | `localhost` | PostgreSQL host, used when `DATABASE_URL` is not set |
| `POSTGRES_PORT` | String | `5432` | PostgreSQL port, used when `DATABASE_URL` is not set |
| `POSTGRES_DB` | String | `readur` | PostgreSQL database name, used when `DATABASE_URL` is not set |
| `POSTGRES_USER` | String | `readur` | PostgreSQL user, used when `DATABASE_URL` is not set |
| `POSTGRES_PASSWORD` | String | `readur` | PostgreSQL password, used when `DATABASE_URL` is not set |
| `SERVER_ADDRESS` | String | `<SERVER_HOST>:<SERVER_PORT>` | Bind address as `host:port`. Takes priority over `SERVER_HOST` and `SERVER_PORT` |
| `SERVER_HOST` | String | `0.0.0.0` | Bind host, used when `SERVER_ADDRESS` is not set |
| `SERVER_PORT` | String | `8000` | Bind port, used when `SERVER_ADDRESS` is not set |
| `UPLOAD_PATH` | String | `./uploads` | Directory documents are stored in with local storage |
| `ALLOWED_FILE_TYPES` | String | `pdf,txt,doc,docx,png,jpg,jpeg` | Comma-separated file extensions imported from watch folders and by batch ingestion; other files there are skipped |
| `RUST_LOG` | String | `info` (with noisy dependencies quietened) | Log filter in [tracing `EnvFilter`](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html) syntax, e.g. `debug` or `readur=debug,sqlx=warn` |
| `DEBUG` | Boolean | `false` | Extra diagnostic log lines. Any value other than empty, `0` or `false` turns it on |

### Security and Access Control

These settings are read at startup. Invalid values (for example a malformed CIDR, an out-of-range `JWT_TTL_HOURS`, an invalid CORS origin, or a Boolean other than `true`/`false`/`1`/`0`/`yes`/`no`/`on`/`off`) stop the server with an error.

| Variable | Type | Default | Description | Required |
|----------|------|---------|-------------|----------|
| `JWT_SECRET` | String | generated, stored in the database | Secret used to sign JWT tokens. When unset, a random key is generated on first start and stored in the database, shared by every instance using that database; run `readur rotate-jwt-secret` to replace it (restart the servers afterwards; all sessions end). When set to a value of at least 32 bytes that is not a published example (such as `your-secret-key`, `change-me` or `secret`), it takes precedence; a shorter or example value is ignored with a startup warning and the stored key is used instead. Generate with `openssl rand -hex 32` | No |
| `READUR_INSECURE_DEV_MODE` | Boolean | `false` | Uses a short or example `JWT_SECRET` instead of ignoring it. For throwaway local development and CI only; never set in production | No |
| `JWT_TTL_HOURS` | Integer | `12` | Lifetime of issued login tokens, in hours (1-720) | No |
| `ADMIN_USERNAME` | String | `admin` | Username of the initial admin account created on first startup; files imported from `WATCH_FOLDER` belong to this account | No |
| `ADMIN_EMAIL` | String | `<username>@localhost` | Email of the initial admin account | No |
| `ADMIN_PASSWORD` | String | - | Password of the initial admin account (min 8 chars). If unset, a random 24-character password is generated and written to a file (see `ADMIN_PASSWORD_FILE`); it is not written to the logs | No |
| `ADMIN_PASSWORD_FILE` | String | `<UPLOAD_PATH>/.readur/initial-admin-password` | Where a generated initial admin password is written (mode `0600`). In the Docker image this is `/app/uploads/.readur/initial-admin-password`. An existing file is never overwritten; if one is present when the admin account has to be created, startup stops and asks for it to be removed. The startup log shows the path | No |
| `ALLOW_REGISTRATION` | Boolean | `false` | Enables self-registration. Self-registered accounts are created disabled and must be approved (enabled) by an administrator in user management before they can sign in | No |
| `LOCAL_SOURCE_ALLOWED_PATHS` | String | - | Comma-separated directories that local folder sources may use. When set, every local folder source (including those created by admins) must be inside one of these directories; each entry must exist at startup. When unset, only admins can create local folder sources | No |
| `TRUSTED_PROXIES` | String | loopback and private networks | Comma-separated IPs or CIDRs of reverse proxies whose `X-Forwarded-For` header is trusted when determining the client IP (used for rate limiting). When unset, `127.0.0.0/8`, `::1/128`, `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16` and `fc00::/7` are trusted; `none` or an empty value trusts no proxy | No |
| `CORS_ALLOWED_ORIGINS` | String | - (none) | Comma-separated origins allowed to make cross-origin requests, each as `scheme://host[:port]` (a trailing slash is ignored). The server refuses to start if an entry is not a valid http(s) origin. The bundled frontend is same-origin and needs no entry | No |
| `METRICS_TOKEN` | String | - | Bearer token (min 16 chars) for Prometheus scrapes of `/metrics`. Without it, `/metrics` requires an admin session or admin API key | No |
| `PUBLIC_URL` | String | - | Public base URL of the Readur instance (e.g. `https://readur.example.com`). Used to build the post-login OIDC redirect and shared-link URLs; request headers are not used for this. When unset, OIDC falls back to the origin of `OIDC_REDIRECT_URI` and shared links fall back to `http://<SERVER_ADDRESS>` | Recommended behind a proxy |
| `OIDC_LINK_EXISTING_BY_EMAIL` | Boolean | `false` | Link an OIDC login to an existing local account with the same email address. Only applies when the identity provider reports `email_verified: true` | No |
| `ALLOW_LOCAL_AUTH` | Boolean | `true` | Allow username/password login (and, with `ALLOW_REGISTRATION`, self-registration). Set to `false` to allow OIDC login only | No |

Related API behavior:

- `POST /api/auth/logout` revokes all sessions of the current user.
- `POST /api/auth/password` lets a signed-in user change their own password.
- `/api/ocr/health` requires authentication, and the endpoint that ran OCR on an arbitrary server path has been removed.
- Source API responses do not include stored WebDAV passwords or S3 secret keys. The source config instead carries `has_password` / `has_secret_access_key` flags; omit the field or send an empty value on update to keep the stored secret. The settings response likewise has `has_webdav_password` instead of `webdav_password`.
- Document cleanup endpoints (`DELETE /api/documents/cleanup/low/confidence`, including its `preview_only` mode, and `DELETE /api/documents/cleanup/failed/ocr`) act on the caller's own documents, including for admins. An admin can pass `?all_users=true` to act on all users' documents.
- OIDC login completes at `/auth/callback#code=...`; the frontend exchanges the one-time code at `POST /api/auth/oidc/exchange`. The token is not placed in the URL query.

### OIDC/SSO Configuration

See the [OIDC Setup Guide](oidc-setup.md) for provider-specific instructions.

| Variable | Type | Default | Description | Required |
|----------|------|---------|-------------|----------|
| `OIDC_ENABLED` | Boolean | `false` | Enable OIDC login (`true`, `1`, `yes` or `on`) | No |
| `OIDC_CLIENT_ID` | String | - | OIDC client ID | If OIDC enabled |
| `OIDC_CLIENT_SECRET` | String | - | OIDC client secret. Without it Readur acts as a public client (PKCE only) | No |
| `OIDC_ISSUER_URL` | String | - | Issuer URL of the identity provider; endpoints are discovered from it | If OIDC enabled |
| `OIDC_REDIRECT_URI` | String | - | Callback URL registered with the provider, e.g. `https://readur.example.com/api/auth/oidc/callback` | If OIDC enabled |
| `OIDC_AUTO_REGISTER` | Boolean | `false` | Create an account on the first OIDC login of an unknown user | No |

`OIDC_LINK_EXISTING_BY_EMAIL`, `PUBLIC_URL` and `ALLOW_LOCAL_AUTH` also affect OIDC login; see [Security and Access Control](#security-and-access-control).

### S3 Storage

See the [S3 Storage Guide](s3-storage-guide.md). The S3 backend is used only when it is enabled and the bucket name and both keys are set.

| Variable | Type | Default | Description | Required |
|----------|------|---------|-------------|----------|
| `S3_ENABLED` | Boolean | `false` | `true` enables the S3 storage backend. Equivalent to `STORAGE_BACKEND=s3` | No |
| `STORAGE_BACKEND` | String | `local` | `s3` enables the S3 storage backend (equivalent to `S3_ENABLED=true`); anything else keeps the local filesystem | No |
| `S3_BUCKET_NAME` | String | - | Bucket name | If S3 enabled |
| `S3_ACCESS_KEY_ID` | String | - | Access key ID | If S3 enabled |
| `S3_SECRET_ACCESS_KEY` | String | - | Secret access key | If S3 enabled |
| `S3_REGION` | String | `us-east-1` | Region | No |
| `S3_ENDPOINT_URL` | String | - | Custom endpoint for S3-compatible services (MinIO, RustFS, etc.). Alias: `S3_ENDPOINT` (used when `S3_ENDPOINT_URL` is not set) | No |
| `S3_ENDPOINT` | String | - | Alias of `S3_ENDPOINT_URL` | No |
| `S3_FORCE_PATH_STYLE` | Boolean | auto | `true` forces path-style addressing, any other value virtual-hosted style. Unset: path-style when a custom endpoint is set, otherwise virtual-hosted. Alias: `S3_PATH_STYLE` | No |
| `S3_PATH_STYLE` | Boolean | auto | Alias of `S3_FORCE_PATH_STYLE`, used when that is not set | No |
| `S3_PREFIX` | String | - | Prefix for every object key the storage backend writes (documents, thumbnails, processed images). Surrounding slashes and spaces are trimmed (`/tenant/a/` is `tenant/a`); empty or `/` means no prefix. Changing it does not move existing objects, which stay readable and deletable under the keys recorded at upload. S3 sources have their own prefix, set per source | No |

### Watch Folders

See [Watch Folder](WATCH_FOLDER.md) and [Per-User Watch Directories](per-user-watch-directories.md).

| Variable | Type | Default | Description | Required |
|----------|------|---------|-------------|----------|
| `WATCH_FOLDER` | String | `./watch` | Directory whose new files are imported automatically | No |
| `WATCH_INTERVAL_SECONDS` | Integer | `30` | Minimum interval between re-processing the same file; also the polling interval for network filesystems | No |
| `FILE_STABILITY_CHECK_MS` | Integer | `1000` | How long a file's size must stay unchanged before it is imported (milliseconds) | No |
| `MAX_FILE_AGE_HOURS` | Integer | - (no limit) | Skip files last modified longer ago than this many hours | No |
| `FORCE_POLLING_WATCH` | Flag | unset | Poll instead of using filesystem events, even on local filesystems. Any value turns it on (even `false`); unset it to turn it off. Paths that look like NFS/SMB/CIFS mounts are always polled | No |
| `ENABLE_PER_USER_WATCH` | Boolean | `false` | Give each user a watch directory under `USER_WATCH_BASE_DIR` (`true`, `1`, `yes` or `on`) | No |
| `USER_WATCH_BASE_DIR` | String | `./user_watch` | Parent directory of the per-user watch directories | No |

### OCR and Processing

| Variable | Type | Default | Description | Required |
|----------|------|---------|-------------|----------|
| `OCR_LANGUAGE` | String | `eng` | Default Tesseract language code(s), e.g. `eng` or `eng+deu`; users can choose their own in Settings | No |
| `CONCURRENT_OCR_JOBS` | Integer | `4` | Documents OCRed in parallel, clamped to 1-15 | No |
| `OCR_TIMEOUT_SECONDS` | Integer | `300` | Time limit for processing one document | No |
| `MAX_FILE_SIZE_MB` | Integer | `50` | Largest accepted request body, and so the largest upload | No |
| `MAX_PDF_SIZE_MB` | Integer | `100` | Largest PDF that is processed for text | No |
| `MAX_OFFICE_DOCUMENT_SIZE_MB` | Integer | `100` | Largest Office document that is processed for text | No |
| `OCR_LOW_MEMORY_MODE` | Boolean | `false` | Read only by the image OCR service behind `GET /api/ocr/health`; document processing does not use it | No |
| `TEMP_DIR` | String | `/tmp` | Has no effect on document processing, which writes its temporary files to `/tmp` | No |
| `MEMORY_LIMIT_MB` | Integer | `512` | Reported in the startup log only; it does not limit anything | No |
| `CPU_PRIORITY` | String | `normal` | Reported in the startup log only; it does not change process priority | No |

### AI Assistants (MCP)

| Variable | Type | Default | Description | Required |
|----------|------|---------|-------------|----------|
| `MCP_ENABLED` | Boolean | `true` | Serve the Model Context Protocol endpoint at `/api/mcp`. Every request still needs an API key or session token. When `false`, the endpoint returns `404`. See [AI Assistants (MCP)](mcp-guide.md) | No |
| `MCP_ALLOW_DELETE` | Boolean | `false` | Offer the `delete_document` tool to MCP clients | No |

## Database Connection Priority

The database connection can be configured in two ways:

1. **Using `DATABASE_URL`** (takes priority if set):
   ```bash
   DATABASE_URL=postgresql://username:password@host:port/database
   ```

2. **Using individual PostgreSQL variables** (used if `DATABASE_URL` is not set):
   ```bash
   POSTGRES_HOST=localhost
   POSTGRES_PORT=5432
   POSTGRES_DB=readur
   POSTGRES_USER=readur
   POSTGRES_PASSWORD=your_password
   ```

This flexibility allows for easy deployment across different platforms:
- **Docker/Kubernetes**: Often provide individual variables
- **Heroku/Railway**: Typically provide `DATABASE_URL`
- **Local Development**: Use either method based on preference

## Helm Chart Values

The Helm chart (`charts/readur`) manages the initial `ADMIN_PASSWORD` in a Secret named `<release>-auth`. When no value is given, it is generated randomly on install and kept on upgrade. The Secret has `helm.sh/resource-policy: keep`, so it also remains after `helm uninstall`. `JWT_SECRET` is optional: set `auth.jwtSecret` (or include it in `auth.existingSecret`) to manage it yourself; otherwise Readur keeps a generated key in its database. A `JWT_SECRET` already stored in the Secret by an earlier chart version is kept.

| Value | Default | Description |
|-------|---------|-------------|
| `auth.existingSecret` | `""` | Name of an existing Secret to use instead. It must contain `JWT_SECRET` (at least 32 bytes) and may contain `ADMIN_PASSWORD` |
| `auth.jwtSecret` | `""` | Explicit JWT secret; generated (64 characters) when empty |
| `auth.adminPassword` | `""` | Explicit initial admin password; generated when empty |

**GitOps / `helm template`:** keeping generated values across upgrades relies on Helm's `lookup` function, which only works when Helm talks to the cluster (`helm install` / `helm upgrade`). Renderers that do not, such as `helm template` and Argo CD, cannot see the existing Secret and would generate new random values on every render, signing out all users and changing the admin password. With these tools, set `auth.existingSecret` to a Secret you manage (for example with Sealed Secrets or External Secrets), or set both `auth.jwtSecret` and `auth.adminPassword` explicitly.

```bash
# Read the initial admin password
kubectl get secret <release>-auth -o jsonpath='{.data.ADMIN_PASSWORD}' | base64 -d
```

## Command-Line Commands

The `readur` binary starts the server by default (`readur` or `readur serve`). It also has two maintenance commands, which read the same environment variables:

- `readur reset-admin-password`: sets the password of the `ADMIN_USERNAME` account to `ADMIN_PASSWORD`, or to a new random password printed to the terminal.
- `readur rotate-jwt-secret`: replaces the JWT signing key stored in the database (all sessions end; restart the servers afterwards).

## Kubernetes Example

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: readur-config
data:
  SERVER_PORT: "8000"
  OCR_LANGUAGE: "eng+spa+fra"
  CONCURRENT_OCR_JOBS: "8"
  RUST_LOG: "info"
  CORS_ALLOWED_ORIGINS: "https://app.example.com"
---
apiVersion: v1
kind: Secret
metadata:
  name: readur-secrets
type: Opaque
stringData:
  DATABASE_URL: "postgresql://readur:password@postgres:5432/readur"
  S3_ACCESS_KEY_ID: "AKIAIOSFODNN7EXAMPLE"
  S3_SECRET_ACCESS_KEY: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
```

## Troubleshooting Configuration

At startup the server prints the settings it loaded, marking which came from the environment and which are defaults. Invalid security settings stop startup with an error; invalid numbers fall back to the default with a warning in that log.

```bash
# More detailed logs
RUST_LOG=debug readur
```

- **A variable has no effect:** check the name against the tables above (anything else is ignored), that there are no spaces around `=`, and that the server was restarted.
- **A `.env` value is ignored:** a variable set in the environment (for example by Docker Compose) takes precedence over `.env`.
