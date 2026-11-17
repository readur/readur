# Integration Guide

This page lists the ways Readur connects to other systems and links to the detailed guide for each. Readur's integration points are:

| Integration | What it is for | Details |
|-------------|----------------|---------|
| REST API | Scripts and applications that upload, search and read documents | [API Reference](api-reference.md) |
| MCP endpoint | AI assistants (Claude Code, Claude Desktop and other MCP clients) | [AI Assistants (MCP)](mcp-guide.md) |
| OpenID Connect | Single sign-on through your identity provider | [OIDC Setup](oidc-setup.md) |
| S3 storage | Keeping document files in an S3 or S3-compatible bucket | [S3 Storage Guide](s3-storage-guide.md) |
| Sources | Importing documents from WebDAV servers, S3 buckets and server folders | [Sources Guide](sources-guide.md) |
| Watch folders | Importing files dropped into a directory | [Watch Folder](WATCH_FOLDER.md) |
| Prometheus | Metrics scraping at `/metrics` | [Health Monitoring](health-monitoring-guide.md) |
| Reverse proxies | HTTPS, a public hostname, load balancing | [Reverse Proxy](REVERSE_PROXY.md) |

Readur does not send webhooks, email or chat messages, and has no message-queue, LDAP or cache integrations.

## REST API

Everything the web interface does goes through the REST API under `/api`, so anything a user can do in the browser can be automated. The [API Reference](api-reference.md) documents every endpoint; the server also publishes an OpenAPI document at `/api-docs/openapi.json` with a [Swagger UI](swagger-ui-guide.md) at `/swagger-ui`.

### Authentication

Send a credential with every request:

```
Authorization: Bearer <token>
```

The token is either:

- **A personal API key** (recommended for integrations). Create one in **Settings → API keys** or with `POST /api/auth/keys`. Keys start with `readur_pat_`, act with the permissions of the user who created them, can expire after 1 to 365 days, and can be revoked at any time. See [API Key Authentication](api-reference.md#api-key-authentication) and [API Key Security](security-guide.md#api-key-security).
- **A session token** from `POST /api/auth/login`. It expires after `JWT_TTL_HOURS` hours (12 by default) and there are no refresh tokens, so long-running integrations should use an API key instead.

Create an integration's key while logged in with a session token:

```bash
base=https://readur.example.com

session=$(curl -s -X POST "$base/api/auth/login" \
  -H "Content-Type: application/json" \
  -d '{"username": "alice", "password": "alice-password-1"}' | jq -r .token)

curl -s -X POST "$base/api/auth/keys" \
  -H "Authorization: Bearer $session" \
  -H "Content-Type: application/json" \
  -d '{"name": "scanner-upload", "expires_in_days": 90}' | jq -r .plaintext
```

The key (`plaintext`) is shown only in this response; store it in your secret manager.

### Examples with curl

```bash
base=https://readur.example.com
token=readur_pat_...

# Upload a document, with an OCR language
curl -s -X POST "$base/api/documents" \
  -H "Authorization: Bearer $token" \
  -F "file=@invoice.pdf" \
  -F "ocr_language=deu"

# Check its OCR status and read the text (use the "id" from the upload response)
curl -s "$base/api/documents/$document_id/ocr" \
  -H "Authorization: Bearer $token" | jq '{ocr_status, ocr_text}'

# Search
curl -s "$base/api/search/enhanced?query=invoice%202024&limit=10" \
  -H "Authorization: Bearer $token" | jq '.documents[] | {id, filename}'

# Download the original file
curl -s -o invoice.pdf "$base/api/documents/$document_id/download" \
  -H "Authorization: Bearer $token"
```

The upload answers with the new document's `id` and `status` (`success`, or `duplicate` when the user already has a file with the same content, in which case the existing document's `id` is returned). OCR runs in the background; `ocr_status` goes from `pending` to `processing` to `completed` or `failed`.

### Python

```python
import time
import requests

BASE = "https://readur.example.com"
HEADERS = {"Authorization": "Bearer readur_pat_..."}


def upload(path, language=None):
    data = {"ocr_language": language} if language else {}
    with open(path, "rb") as f:
        r = requests.post(f"{BASE}/api/documents", headers=HEADERS,
                          files={"file": f}, data=data)
    r.raise_for_status()
    return r.json()["id"]


def wait_for_text(document_id, timeout=600):
    deadline = time.time() + timeout
    while time.time() < deadline:
        r = requests.get(f"{BASE}/api/documents/{document_id}/ocr", headers=HEADERS)
        r.raise_for_status()
        body = r.json()
        if body["ocr_status"] in ("completed", "failed"):
            return body
        time.sleep(5)
    raise TimeoutError(document_id)


def search(query, limit=25):
    r = requests.get(f"{BASE}/api/search/enhanced", headers=HEADERS,
                     params={"query": query, "limit": limit})
    r.raise_for_status()
    return r.json()["documents"]


doc_id = upload("scan.pdf", language="eng")
print(wait_for_text(doc_id)["ocr_text"])
for doc in search("invoice 2024"):
    print(doc["id"], doc["filename"])
```

### JavaScript / TypeScript

```typescript
const base = 'https://readur.example.com';
const apiKey = 'readur_pat_...'; // load it from your secret store
const headers = { Authorization: `Bearer ${apiKey}` };

async function upload(file: Blob, filename: string): Promise<string> {
  const form = new FormData();
  form.append('file', file, filename);
  const response = await fetch(`${base}/api/documents`, { method: 'POST', headers, body: form });
  if (!response.ok) throw new Error(`upload failed: ${response.status}`);
  return (await response.json()).id;
}

async function search(query: string) {
  const url = `${base}/api/search/enhanced?query=${encodeURIComponent(query)}`;
  const response = await fetch(url, { headers });
  if (!response.ok) throw new Error(`search failed: ${response.status}`);
  return (await response.json()).documents as Array<{ id: string; filename: string }>;
}
```

### Errors and limits

Failed requests answer with a 4xx or 5xx status; most bodies carry an `error` message and a stable code (see [Errors](api-reference.md#errors)). Some endpoints are rate limited and answer `429 Too Many Requests` (see [Rate Limiting](api-reference.md#rate-limiting)). Uploads may be at most `MAX_FILE_SIZE_MB` megabytes (50 by default). Browser applications on another origin can call the API only if that origin is listed in `CORS_ALLOWED_ORIGINS`.

### Following events

Readur does not push events to other systems. To react to finished syncs, poll the user's server notifications (`GET /api/notifications/summary`, see the [Notifications Guide](notifications-guide.md)); to wait for OCR, poll `GET /api/documents/{id}/ocr` as in the Python example. The progress of a running source sync is also available as a WebSocket stream at `GET /api/sources/{id}/sync/progress/ws` (see [Sync Progress WebSocket](api-reference.md#sync-progress-websocket)).

## AI assistants (MCP)

Readur serves a [Model Context Protocol](https://modelcontextprotocol.io) endpoint at `/api/mcp`, so AI assistants can search, read, upload and label documents with a user's permissions. It authenticates with the same API keys as the REST API and is on by default (`MCP_ENABLED`). Setup for Claude Code, Claude Desktop and other clients is in [AI Assistants (MCP)](mcp-guide.md).

## Single sign-on (OIDC)

Readur can sign users in through any OpenID Connect provider that publishes a discovery document, such as Keycloak, Authentik, Auth0, Microsoft Entra ID (Azure AD) or Google. The essential settings:

```bash
OIDC_ENABLED=true
OIDC_ISSUER_URL=https://sso.example.com/realms/readur
OIDC_CLIENT_ID=readur
OIDC_CLIENT_SECRET=client-secret-from-your-provider
OIDC_REDIRECT_URI=https://readur.example.com/api/auth/oidc/callback
PUBLIC_URL=https://readur.example.com
```

Register `OIDC_REDIRECT_URI` as the redirect URI at the provider. `OIDC_AUTO_REGISTER`, `OIDC_LINK_EXISTING_BY_EMAIL` and `ALLOW_LOCAL_AUTH` control account creation, linking and whether password login stays available. Provider-specific steps are in [OIDC Setup](oidc-setup.md#identity-provider-setup).

## Storage: S3 and S3-compatible services

By default Readur stores document files on the local filesystem under `UPLOAD_PATH`. It can instead store them in an S3 bucket, on AWS or on an S3-compatible service such as MinIO:

```bash
S3_ENABLED=true
S3_BUCKET_NAME=readur-documents
S3_ACCESS_KEY_ID=readur
S3_SECRET_ACCESS_KEY=change-me-to-a-long-secret
S3_REGION=us-east-1
# Only for S3-compatible services:
S3_ENDPOINT_URL=https://s3.example.com
```

Path-style addressing is used automatically when a custom endpoint is set (`S3_FORCE_PATH_STYLE` overrides this), and `S3_PREFIX` puts every stored object below a prefix. Documents stored locally before S3 was enabled can be moved with the `migrate_to_s3` tool. See the [S3 Storage Guide](s3-storage-guide.md) and the [S3 Storage settings](configuration-reference.md#s3-storage).

## Importing documents

### Sources

A source imports documents on a schedule from somewhere they already live. Each user creates their own sources in the web interface or with `POST /api/sources`:

| `source_type` | Imports from |
|---------------|--------------|
| `webdav` | A WebDAV server: Nextcloud, ownCloud or a generic WebDAV server |
| `s3` | An S3 bucket or S3-compatible service |
| `local_folder` | A directory on the Readur server, such as a mounted NFS or SMB share. Only administrators may create these unless `LOCAL_SOURCE_ALLOWED_PATHS` is set |

For example, a WebDAV source on Nextcloud that syncs every hour:

```bash
curl -s -X POST https://readur.example.com/api/sources \
  -H "Authorization: Bearer $token" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Office NAS",
    "source_type": "webdav",
    "enabled": true,
    "config": {
      "server_url": "https://nextcloud.example.com",
      "username": "alice",
      "password": "app-password",
      "watch_folders": ["/Documents"],
      "file_extensions": ["pdf", "png", "jpg"],
      "auto_sync": true,
      "sync_interval_minutes": 60,
      "server_type": "nextcloud"
    }
  }'
```

An S3 source's `config` takes `bucket_name`, `region`, `access_key_id`, `secret_access_key`, `endpoint_url` (for S3-compatible services), `force_path_style`, `prefix`, `watch_folders` (prefixes to read), `file_extensions`, `auto_sync` and `sync_interval_minutes`. A local folder source's takes `watch_folders`, `file_extensions`, `auto_sync`, `sync_interval_minutes`, `recursive` and `follow_symlinks`. Stored passwords and secret keys are never returned by the API. See the [Sources Guide](sources-guide.md) and [Source Endpoints](api-reference.md#source-endpoints).

### Watch folders

Readur imports files that appear in `WATCH_FOLDER` (`/app/watch` in the Docker image). This suits scanners and other tools that can save to a network share: point them at a share mounted into the container. Imported files are not changed or deleted. Files from the global watch folder belong to the user named `admin`. Only extensions in `ALLOWED_FILE_TYPES` are imported, and network filesystems are polled every `WATCH_INTERVAL_SECONDS`.

With `ENABLE_PER_USER_WATCH=true`, each user also gets a directory under `USER_WATCH_BASE_DIR`, and files placed there belong to that user. See [Watch Folder](WATCH_FOLDER.md) and [Per-User Watch Directories](per-user-watch-directories.md).

## Monitoring with Prometheus

`GET /metrics` serves metrics in Prometheus text format (OCR queue, documents, users, database, storage, WebDAV syncs). Set `METRICS_TOKEN` to a random value of at least 16 characters and give it to Prometheus:

```yaml
scrape_configs:
  - job_name: readur
    metrics_path: /metrics
    scheme: https
    authorization:
      type: Bearer
      credentials: "the value of METRICS_TOKEN"
    static_configs:
      - targets: ["readur.example.com"]
```

Without `METRICS_TOKEN`, `/metrics` accepts only an administrator's session token or API key. For load balancers and container health checks use `GET /api/health`, which needs no credential. See the [Health Monitoring Guide](health-monitoring-guide.md).

## Reverse proxies

Readur serves plain HTTP on port 8000 (`SERVER_PORT`). In production put it behind a reverse proxy such as nginx, Traefik, Caddy or Apache that serves HTTPS. The proxy should:

- allow request bodies at least as large as `MAX_FILE_SIZE_MB`,
- pass WebSocket upgrades (used for sync progress at `/api/sources/{id}/sync/progress/ws`),
- send `X-Forwarded-For`, so that per-client rate limits see the real client address.

On the Readur side, set `PUBLIC_URL` to the public `https://` address (used for OIDC redirects and shared-link URLs) and, if the proxy is not on a loopback or private address, list it in `TRUSTED_PROXIES`. Configuration examples for each proxy are in [Reverse Proxy](REVERSE_PROXY.md).
