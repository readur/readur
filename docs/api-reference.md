# API Reference

Readur's REST API is what the web interface itself uses: everything the interface can do, a script can do over HTTP. This page describes how to authenticate, how errors look, and every endpoint, with worked examples for the common tasks. The complete request and response schemas are also served by the running server as an OpenAPI document (see [Interactive Documentation](#interactive-documentation)).

## Table of Contents

- [Base URL](#base-url)
- [Interactive Documentation](#interactive-documentation)
- [Authentication](#authentication)
- [Errors](#errors)
- [Rate Limiting](#rate-limiting)
- [Pagination](#pagination)
- [Endpoints](#endpoints)
  - [Authentication](#authentication-endpoints)
  - [API Keys](#api-key-endpoints)
  - [Documents](#document-endpoints)
  - [Search](#search-endpoints)
  - [Labels](#labels)
  - [Comments](#comment-endpoints)
  - [Shared Links](#shared-link-endpoints)
  - [Notifications](#notification-endpoints)
  - [Sources](#source-endpoints)
  - [Source Errors and WebDAV](#source-error-and-webdav-endpoints)
  - [Ignored Files](#ignored-file-endpoints)
  - [OCR and Queue](#ocr-and-queue-endpoints)
  - [Settings](#settings-endpoints)
  - [Users](#user-endpoints)
  - [Health, Metrics and MCP](#health-metrics-and-mcp-endpoints)
- [Sync Progress WebSocket](#sync-progress-websocket)
- [Client Examples](#client-examples)
- [Limits](#limits)

## Base URL

Every endpoint except the Prometheus `/metrics` lives under `/api` on the server's own address, which is `http://localhost:8000` by default (`SERVER_PORT`, or `SERVER_ADDRESS` for host and port together). Behind a reverse proxy, use the public URL instead, over HTTPS:

```
https://readur.example.com/api
```

Request and response bodies are JSON (`Content-Type: application/json`) except where an endpoint says otherwise: uploads are `multipart/form-data`, and downloads, thumbnails and the Prometheus metrics are not JSON.

## Interactive Documentation

The server publishes its OpenAPI document at `/api-docs/openapi.json` and a Swagger UI for it at `/swagger-ui` (see the [Swagger UI Guide](swagger-ui-guide.md)). The OpenAPI document has the full schema of every annotated endpoint; this page adds the behaviour around them and covers the endpoints the OpenAPI document leaves out (API keys, comments, shared links, MCP and three document endpoints).

## Authentication

Every endpoint except those marked `public` in the endpoint tables needs a credential in the `Authorization` header:

```
Authorization: Bearer <token>
```

The token is either a session token from logging in or a personal API key. A missing, invalid, expired or revoked credential gets `401 Unauthorized`. A valid credential without the required role (an `admin` endpoint called by a regular user) gets `403 Forbidden`.

### Session Tokens

`POST /api/auth/login` exchanges a username and password for a session token (see [Log In](#log-in)). A session token is a JWT valid for `JWT_TTL_HOURS` hours (12 by default). There are no refresh tokens: when a session expires, log in again. `POST /api/auth/logout` and a password change revoke all of the user's sessions. Accounts that are deactivated, or self-registered and not yet approved, cannot log in (`403 Forbidden`).

### OIDC Sign-in

When OIDC is configured (see [OIDC Setup](oidc-setup.md)), the browser starts at `GET /api/auth/oidc/login`, which redirects to the identity provider. The provider sends the browser back to `GET /api/auth/oidc/callback`, and Readur then redirects it to `<PUBLIC_URL>/auth/callback#code=<one-time code>`. The frontend exchanges that code for a session with `POST /api/auth/oidc/exchange` and the body `{"code": "<one-time code>"}`; the response has the same format as the [Log In](#log-in) response, and an unknown, used or expired code gets `401 Unauthorized`. The code is single-use and short-lived, and the session token itself never appears in a URL.

### API Key Authentication

For scripts and integrations, users create personal API keys (see [API Key Endpoints](#api-key-endpoints)). A key is sent exactly like a session token:

```
Authorization: Bearer readur_pat_<token>
```

Keys start with `readur_pat_` so that secret scanners can recognise them. A key acts with all the permissions of the user who created it, and the user's current role is read on every request, so promoting or demoting the owner takes effect immediately. Keys are generated from 256 bits of OS randomness and stored only as a SHA-256 hash: the plaintext is shown once, when the key is created. A key may expire after 1 to 365 days or never, and can be revoked at any time; each user may hold at most 20 active keys. See the [Security Guide](security-guide.md#api-key-security) for handling advice.

## Errors

Every failed request answers with a 4xx or 5xx status and the same JSON object, whatever the endpoint:

| Field | Type | Meaning |
|-------|------|---------|
| `error` | string | A message for people. A server error (5xx) never includes internal detail such as SQL, file paths or stack traces |
| `code` | string | A stable code for programs, in UPPER_SNAKE_CASE |
| `status` | integer | The HTTP status, repeated |

Where an endpoint knows more than the status, `code` says so: `API_KEY_NOT_FOUND`, `USER_NOT_FOUND_BY_ID`, `LABEL_DUPLICATE_NAME`, `SOURCE_DUPLICATE_NAME`, `INVALID_QUERY`, `UPLOAD_TOO_LARGE` and so on; the per-area tables are under [Shared Links](#shared-links-error-codes), [API Keys](#api-keys-error-codes) and [Comments](#comments-error-codes). Otherwise `code` is the status's reason phrase in UPPER_SNAKE_CASE: `BAD_REQUEST`, `UNAUTHORIZED`, `FORBIDDEN`, `NOT_FOUND`, `METHOD_NOT_ALLOWED`, `CONFLICT`, `PAYLOAD_TOO_LARGE`, `UNPROCESSABLE_ENTITY`, `TOO_MANY_REQUESTS`, `INTERNAL_SERVER_ERROR`, `NOT_IMPLEMENTED`, `SERVICE_UNAVAILABLE`. Branch on `status` and `code`, not on the wording of `error`. The OpenAPI document describes the object as the `ApiErrorBody` schema.

A request body that is not valid JSON gets `400 Bad Request`, and one that lacks a required field or has a field of the wrong type `422 Unprocessable Entity`, before the endpoint runs; `error` then explains what is wrong with the body (see [Invalid Request Body](#invalid-request-body)).

A few more conventions hold across the API:

- Creating something (a label, source, user, API key, shared link or comment) answers `201 Created` with the new object. Uploading a document answers `200 OK`.
- Deleting one object answers `204 No Content`. Deleting something that does not exist, or that the caller cannot see, answers `404 Not Found` on every endpoint; deleting it again is therefore a `404`, not a second `204`.
- A feature the server has switched off answers `501 Not Implemented` (code `USER_WATCH_DISABLED` for the per-user watch directory endpoints while `ENABLE_PER_USER_WATCH` is off).

#### Error Response

```http
DELETE /api/auth/keys/{id}
Authorization: Bearer <token>
```

**Response:** `404 Not Found`
```json
{
  "error": "API key not found",
  "code": "API_KEY_NOT_FOUND",
  "status": 404
}
```

#### Invalid Query Error

```http
GET /api/documents?ocr_status=done
Authorization: Bearer <token>
```

**Response:** `400 Bad Request`
```json
{
  "error": "Invalid ocr_status filter; expected one of: pending, processing, completed, failed",
  "code": "INVALID_QUERY",
  "status": 400
}
```

#### Invalid Request Body

```http
POST /api/labels
Authorization: Bearer <token>
Content-Type: application/json
```

**Request body:**
```json
{
  "color": "#0969da"
}
```

**Response:** `422 Unprocessable Entity`
```json
{
  "error": "Failed to deserialize the JSON body into the target type: missing field `name` at line 3 column 1",
  "code": "UNPROCESSABLE_ENTITY",
  "status": 422
}
```

#### Authentication Error

```http
POST /api/auth/login
Content-Type: application/json
```

**Request body:**
```json
{
  "username": "alice",
  "password": "wrong-password"
}
```

**Response:** `401 Unauthorized`
```json
{
  "error": "Invalid username or password",
  "code": "UNAUTHORIZED",
  "status": 401
}
```

## Rate Limiting

Some endpoints limit how often one client or user may call them within a sliding window:

| What is limited | Limit | Endpoints |
|-----------------|-------|-----------|
| Failed logins for one username from one address | 10 per 15 minutes | `POST /api/auth/login`, `POST /api/auth/password` |
| Failed logins for one username | 100 per 15 minutes | `POST /api/auth/login`, `POST /api/auth/password` |
| Failed logins from one address | 50 per 15 minutes | `POST /api/auth/login` |
| Self-registrations from one address | 10 per hour | `POST /api/auth/register` |
| OIDC login starts and code exchanges from one address | 30 per minute | `GET /api/auth/oidc/login`, `POST /api/auth/oidc/exchange` |
| Shared link password checks from one address | 10 per minute | `POST /api/public/shared/{token}/verify`, and downloads or views that send a password |
| Shared link access from one address | 60 per minute | `GET /api/public/shared/{token}`, `POST /api/public/shared/{token}/download`, `POST /api/public/shared/{token}/view` |
| Comments created by one user | 10 per minute | `POST /api/comments/documents/{document_id}/comments` |
| Shared links created by one user | 20 per hour | `POST /api/shared/links` |
| API keys created by one user | 10 per hour | `POST /api/auth/keys` |

Only failed password attempts count against the login limits; a correct password does not. A limited request gets `429 Too Many Requests`. The authentication endpoints also send a `Retry-After` header with the seconds to wait; the shared link, comment and API key endpoints name the wait in the error message instead (`SHARED_LINK_RATE_LIMITED`, `COMMENT_RATE_LIMITED`, `API_KEY_RATE_LIMITED`).

## Pagination

List endpoints that page take `limit` (the page size) and `offset` (the number of items to skip) as query parameters. Defaults and maximums differ per endpoint and are given with it. `GET /api/documents` reports its paging position in a `pagination` object (see [List Documents](#list-documents)); the search endpoints report the total number of matches in `total`; the other list endpoints return a plain JSON array, so a short page means you have reached the end.

```http
GET /api/documents?limit=50&offset=100
```

## Endpoints

Each group starts with a table of its endpoints. The access column says who may call an endpoint: `public` needs no credential, `user` needs any signed-in user, `admin` needs an administrator, and `admin or self` lets a user act on their own account and an administrator on any. Regular users only ever see and change their own documents, labels, sources and other data.

### Authentication Endpoints

| Endpoint | Access | What it does |
|----------|--------|--------------|
| `POST /api/auth/login` | public | Exchange a username and password for a session token |
| `POST /api/auth/register` | public | Create an account, when self-registration is enabled |
| `GET /api/auth/config` | public | Which sign-in methods and optional features the server offers |
| `GET /api/auth/me` | user | The signed-in user |
| `POST /api/auth/logout` | user | Revoke all of the user's sessions |
| `POST /api/auth/password` | user | Change the user's own password |
| `GET /api/auth/oidc/login` | public | Start an OIDC sign-in (redirects to the identity provider) |
| `GET /api/auth/oidc/callback` | public | Where the identity provider returns the browser |
| `POST /api/auth/oidc/exchange` | public | Exchange the one-time OIDC code for a session |

#### Log In

```http
POST /api/auth/login
Content-Type: application/json
```

**Request body:**
```json
{
  "username": "alice",
  "password": "alice-password-1"
}
```

**Response:** `200 OK`
```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "user": {
    "id": "0b9e8d7c-6a5f-4e3d-2c1b-0a9f8e7d6c5b",
    "username": "alice",
    "email": "alice@example.com",
    "role": "user",
    "is_active": true
  }
}
```

`role` is `user` or `admin`. A wrong username or password gets `401 Unauthorized`, a deactivated or unapproved account `403 Forbidden`, and when `ALLOW_LOCAL_AUTH=false` every local login gets `403 Forbidden`.

#### Current User

```http
GET /api/auth/me
Authorization: Bearer <token>
```

**Response:** `200 OK`
```json
{
  "id": "0b9e8d7c-6a5f-4e3d-2c1b-0a9f8e7d6c5b",
  "username": "alice",
  "email": "alice@example.com",
  "role": "user",
  "is_active": true
}
```

#### Sign-in Options

```http
GET /api/auth/config
```

What the login page needs to know, here for a server with `ALLOW_REGISTRATION=true` and OIDC not configured.

**Response:** `200 OK`
```json
{
  "allow_local_auth": true,
  "allow_registration": true,
  "oidc_enabled": false,
  "enable_per_user_watch": false,
  "mcp_enabled": true
}
```

#### Register

```http
POST /api/auth/register
Content-Type: application/json
```

**Request body:**
```json
{
  "username": "bob",
  "email": "bob@example.com",
  "password": "bob-password-1"
}
```

**Response:** `200 OK`
```json
{
  "id": "1c2d3e4f-5a6b-4c7d-8e9f-0a1b2c3d4e5f",
  "username": "bob",
  "email": "bob@example.com",
  "role": "user",
  "is_active": false
}
```

Returns `403 Forbidden` unless the server runs with `ALLOW_REGISTRATION=true` (and local login is allowed). Registered accounts get the `user` role and are created inactive: an administrator must activate them (see [Activate a User](#activate-a-user)) before they can log in. A taken username or email, or a password that does not meet the password rules, gets `400 Bad Request`.

#### Change Own Password

```http
POST /api/auth/password
Authorization: Bearer <token>
Content-Type: application/json
```

**Request body:**
```json
{
  "current_password": "alice-password-1",
  "new_password": "a-new-longer-password"
}
```

**Response:** `200 OK` with a new session, in the same format as the [Log In](#log-in) response. All other sessions of the user are revoked. A wrong current password gets `401 Unauthorized`, and a new password that does not meet the password rules `400 Bad Request`.

#### Log Out

```http
POST /api/auth/logout
Authorization: Bearer <token>
```

**Response:** `204 No Content`. All sessions of the user are revoked, not only the one used for the request. API keys stay valid.

### API Key Endpoints

All three endpoints accept a session token or another API key.

| Endpoint | Access | What it does |
|----------|--------|--------------|
| `POST /api/auth/keys` | user | Create a key; the response holds its plaintext, once |
| `GET /api/auth/keys` | user | The user's keys (administrators may add `?all=true` for every user's) |
| `DELETE /api/auth/keys/{id}` | user | Revoke a key |

#### Create an API Key

```http
POST /api/auth/keys
Authorization: Bearer <token>
Content-Type: application/json
```

**Request body:**
```json
{
  "name": "backup-script",
  "expires_in_days": 90
}
```

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `name` | string | yes | 1 to 100 characters, to recognise the key by |
| `expires_in_days` | integer or null | no | 1 to 365. Omit it or send `null` for a key that never expires |

**Response:** `201 Created`
```json
{
  "api_key": {
    "id": "a1b2c3d4-e5f6-4890-abcd-ef1234567890",
    "user_id": "0b9e8d7c-6a5f-4e3d-2c1b-0a9f8e7d6c5b",
    "name": "backup-script",
    "key_prefix": "readur_pat_X",
    "expires_at": "2026-07-20T12:34:56Z",
    "last_used_at": null,
    "revoked_at": null,
    "is_expired": false,
    "created_at": "2026-04-21T12:34:56Z"
  },
  "plaintext": "readur_pat_XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"
}
```

`plaintext` is returned exactly once and cannot be recovered: if it is lost, revoke the key and create a new one. `key_prefix` is the first 12 characters, for recognising the key later. An invalid name or `expires_in_days` gets `400 Bad Request`, a user who already has 20 active keys `409 Conflict`, and more than 10 creations in an hour `429 Too Many Requests`.

#### List API Keys

```http
GET /api/auth/keys
Authorization: Bearer <token>
```

**Response:** `200 OK`
```json
[
  {
    "id": "a1b2c3d4-e5f6-4890-abcd-ef1234567890",
    "user_id": "0b9e8d7c-6a5f-4e3d-2c1b-0a9f8e7d6c5b",
    "name": "backup-script",
    "key_prefix": "readur_pat_X",
    "expires_at": "2026-07-20T12:34:56Z",
    "last_used_at": null,
    "revoked_at": null,
    "is_expired": false,
    "created_at": "2026-04-21T12:34:56Z"
  }
]
```

Neither the plaintext nor the stored hash ever appears here. Revoked keys stay in the list with `revoked_at` set.

#### Revoke an API Key

```http
DELETE /api/auth/keys/{id}
Authorization: Bearer <token>
```

**Response:** `204 No Content`. The key fails authentication from the next request on. Users may revoke their own keys and administrators any key. A key that does not exist, is already revoked or belongs to another user gets `404 Not Found`; a regular user cannot tell these cases apart.

#### API Keys Error Codes

| Code | HTTP Status | Meaning |
|------|-------------|---------|
| `API_KEY_NOT_FOUND` | 404 | The key does not exist or the caller cannot see it |
| `API_KEY_INVALID_REQUEST` | 400 | The name or `expires_in_days` is invalid |
| `API_KEY_MAX_REACHED` | 409 | The user already has 20 active keys |
| `API_KEY_PERMISSION_DENIED` | 403 | The caller may not do this |
| `API_KEY_RATE_LIMITED` | 429 | More than 10 keys created in the last hour |
| `API_KEY_INTERNAL_ERROR` | 500 | An unexpected server error |

### Document Endpoints

Administrators see and manage every user's documents; regular users only their own.

| Endpoint | Access | What it does |
|----------|--------|--------------|
| `GET /api/documents` | user | List documents, filtered, sorted and paged |
| `POST /api/documents` | user | Upload a document (multipart) and queue it for OCR |
| `GET /api/documents/{id}` | user | One document, in the format of a [List Documents](#list-documents) item |
| `DELETE /api/documents/{id}` | user | Delete a document and its file |
| `GET /api/documents/{id}/download` | user | The original file, as an attachment |
| `GET /api/documents/{id}/view` | user | The original file, for display in the browser |
| `GET /api/documents/{id}/thumbnail` | user | A JPEG thumbnail, when one can be made |
| `GET /api/documents/{id}/processed/image` | user | The preprocessed image OCR read, when one was kept |
| `GET /api/documents/{id}/ocr` | user | The extracted text and OCR details |
| `POST /api/documents/{id}/ocr/retry` | user | Run OCR again, optionally with other languages (`{"language": "deu"}` or `{"languages": ["eng", "deu"]}`) |
| `GET /api/documents/{id}/ocr/retry/history` | user | Earlier OCR retries of a document |
| `POST /api/documents/{id}/ocr/stop` | user | Cancel a document's queued or running OCR |
| `GET /api/documents/{id}/debug` | user | Processing details for troubleshooting a document |
| `GET /api/documents/{id}/validate` | user | Check that the stored file and the database record agree |
| `GET /api/documents/duplicates` | user | Groups of documents with identical content (`limit`, `offset`) |
| `GET /api/documents/failed` | user | Documents that failed processing (`limit`, `offset`, `stage`, `reason`) |
| `GET /api/documents/failed/ocr` | user | Documents whose OCR failed, with failure details (`limit`, `offset`) |
| `GET /api/documents/failed/{id}/view` | user | The file of a failed document, for display |
| `POST /api/documents/bulk/delete` | user | Delete several documents (`{"document_ids": [...]}`) |
| `DELETE /api/documents/cleanup/low/confidence` | user | Delete (or with `preview_only`, list) documents whose OCR confidence is at most `max_confidence` |
| `DELETE /api/documents/cleanup/failed/ocr` | user | Delete documents whose OCR failed |
| `GET /api/documents/ocr/stats` | user | OCR counts for the user's documents |
| `POST /api/documents/ocr/retry/bulk` | user | Run OCR again for a selection of documents |
| `GET /api/documents/ocr/retry/recommendations` | user | Groups of failed documents worth retrying, by failure reason |
| `GET /api/documents/ocr/retry/stats` | user | Retry counts by failure reason |

The two cleanup endpoints act on the caller's own documents, administrators included; an administrator passes `?all_users=true` to act on every user's.

#### Document List Parameters

`GET /api/documents` takes these query parameters, all optional:

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `limit` | integer | `25` | Page size |
| `offset` | integer | `0` | Number of documents to skip |
| `sort_by` | string | `created_at` | `created_at`, `updated_at`, `filename`, `file_size`, `ocr_status`, `mime_type` or `ocr_confidence` |
| `sort_order` | string | `desc` | `asc` or `desc` |
| `ocr_status` | string | | `pending`, `processing`, `completed` or `failed` |
| `mime_types` | comma-separated strings | | Documents of any of these MIME types |
| `label_ids` | comma-separated UUIDs | | Documents carrying any of these labels |
| `tags` | comma-separated strings | | Documents carrying any label with one of these names |
| `source_ids` | comma-separated UUIDs | | Documents ingested from any of these sources |
| `source_types` | comma-separated strings | | Documents from any of these source types; `direct_upload` also matches documents without a source |
| `created_from` | RFC 3339 timestamp | | Documents created at or after this instant |
| `created_to` | RFC 3339 timestamp | | Documents created at or before this instant |

Filters combine with AND. An unknown `sort_by` or `ocr_status`, or `created_from` later than `created_to`, gets `400 Bad Request`.

#### List Documents

```http
GET /api/documents?limit=1
Authorization: Bearer <token>
```

**Response:** `200 OK`
```json
{
  "documents": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "filename": "invoice-2024-03.pdf",
      "original_filename": "invoice-2024-03.pdf",
      "file_path": "/app/uploads/documents/550e8400-e29b-41d4-a716-446655440000.pdf",
      "file_size": 48213,
      "mime_type": "application/pdf",
      "tags": [],
      "labels": [],
      "created_at": "2024-03-10T09:00:00Z",
      "updated_at": "2024-03-10T09:00:00Z",
      "user_id": "0b9e8d7c-6a5f-4e3d-2c1b-0a9f8e7d6c5b",
      "file_hash": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
      "has_ocr_text": true,
      "ocr_confidence": 95.5,
      "ocr_word_count": 6,
      "ocr_processing_time_ms": 840,
      "ocr_status": "completed"
    }
  ],
  "pagination": {
    "total": 1,
    "limit": 1,
    "offset": 0,
    "has_more": false
  }
}
```

`labels` holds the document's labels in the format of [Create a Label](#create-a-label). `ocr_confidence` runs from 0 to 100. `ocr_status` is `pending`, `processing`, `completed` or `failed`. `has_more` says whether another page follows.

#### Upload a Document

```http
POST /api/documents
Authorization: Bearer <token>
Content-Type: multipart/form-data
```

The form fields:

| Field | Required | Description |
|-------|----------|-------------|
| `file` | yes | The file, with its filename |
| `ocr_language` | no | One OCR language code for this document, such as `deu` |
| `ocr_languages` | no | Several OCR language codes; repeat the field (or use `ocr_languages[]`) once per language |
| `label_ids` | no | Labels to attach: a label ID or comma-separated IDs; may repeat |

For example, with `curl -F "file=@receipt.txt;type=text/plain"`:

**Response:** `200 OK`
```json
{
  "id": "7d0c9a1e-2f3b-4c5d-8e6f-a1b2c3d4e5f6",
  "filename": "receipt.txt",
  "file_size": 16,
  "mime_type": "text/plain",
  "status": "success",
  "message": "Document uploaded successfully",
  "warnings": []
}
```

Uploading a file whose content the user already has stores nothing and answers with the existing document, `status` `duplicate` and the message `Document already exists`. `warnings` lists follow-up steps that failed after the file was stored (queueing OCR, attaching labels). A file over `MAX_FILE_SIZE_MB` gets `413 Payload Too Large`, and an unknown OCR language `400 Bad Request`.

#### Get OCR Text

```http
GET /api/documents/{id}/ocr
Authorization: Bearer <token>
```

**Response:** `200 OK`
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "filename": "invoice-2024-03.pdf",
  "has_ocr_text": true,
  "ocr_text": "Invoice 2024-03 Total due: 120.00 EUR",
  "ocr_confidence": 95.5,
  "ocr_status": "completed",
  "ocr_processing_time_ms": 840,
  "detected_language": null,
  "pages_processed": null
}
```

#### Retry OCR

```http
POST /api/documents/{id}/ocr/retry
Authorization: Bearer <token>
Content-Type: application/json
```

**Request body:** an empty object runs OCR with the user's default languages; `language` or `languages` picks others and saves them as the user's preference.
```json
{
  "languages": ["eng"]
}
```

**Response:** `200 OK`
```json
{
  "success": true,
  "queued": true,
  "message": "Document queued for OCR processing"
}
```

`success` says the request was handled; `queued` says whether it added a new OCR job. `404 Not Found` when the document does not exist or belongs to another user, `400 Bad Request` for a language that is not installed.

#### Retry OCR While Already Queued

When the document already has a pending or running OCR job (for example the server's own automatic retry), the request still succeeds but queues nothing, and the existing job carries on:

```http
POST /api/documents/{id}/ocr/retry
Authorization: Bearer <token>
Content-Type: application/json
```

**Request body:**
```json
{}
```

**Response:** `200 OK`
```json
{
  "success": true,
  "queued": false,
  "message": "The document is already queued for OCR"
}
```

#### Cancel OCR

Cancels the document's queued or running OCR:

```http
POST /api/documents/{id}/ocr/stop
Authorization: Bearer <token>
```

**Response:** `200 OK`
```json
{
  "success": true,
  "cancelled": true,
  "was_running": false,
  "document_status": "failed",
  "message": "Queued OCR was cancelled; the document is now 'failed' (reason 'cancelled') and can be retried"
}
```

A queued job is removed before any worker starts it. A running job is removed too (`was_running` is `true`) and its worker is stopped at once, freeing its OCR slot; nothing it produced is written over the document. `document_status` is the document's OCR status afterwards: a document without OCR text is left `failed` with `ocr_failure_reason` `cancelled`, and [Retry OCR](#retry-ocr) queues it again; a document whose earlier OCR had completed (a re-OCR was cancelled) stays `completed` and keeps its text, and `message` says which. With nothing to cancel the request still succeeds, with `cancelled` `false` and the document's current `document_status`. `404 Not Found` when the document does not exist or belongs to another user.

#### Delete a Document

```http
DELETE /api/documents/{id}
Authorization: Bearer <token>
```

**Response:** `204 No Content`, or `404 Not Found` when the document does not exist or belongs to another user. A document that a source sync imported is also put on its owner's [ignored list](#ignored-file-endpoints), so the next sync does not import it again.


### Search Endpoints

All search endpoints only see the documents the caller may see (their own; administrators see every document). See the [Advanced Search Guide](advanced-search.md) for the search modes, query syntax and worked examples.

| Endpoint | Access | What it does |
|----------|--------|--------------|
| `GET /api/search` | user | Simple-mode search, newest first by default, without ranking or snippets |
| `GET /api/search/enhanced` | user | Search in any mode, ranked by relevance, with highlighted snippets |
| `GET /api/search/timeline` | user | The number of matching documents per month, for the same parameters as a search |
| `GET /api/search/facets` | user | The caller's MIME types and label names, with document counts |

#### Search Parameters

`GET /api/search`, `GET /api/search/enhanced` and `GET /api/search/timeline` take these query parameters. All are optional, but a search needs a `query` of at least 2 characters or at least one filter. Unknown parameters are ignored. URL-encode values (`%20` for a space).

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `query` | string | (empty) | Search text, matched against the document content and OCR text. At most 1000 bytes |
| `search_mode` | string | `simple` | `simple` (every word), `phrase` (adjacent words in order), `boolean` (operators) or `fuzzy` (similar spelling, cutoff from the caller's fuzzy search threshold setting). `GET /api/search` always searches in simple mode |
| `limit` | integer | `25` | Page size, 1 to 1000 |
| `offset` | integer | `0` | Number of results to skip |
| `include_snippets` | boolean | `true` | Return snippets (enhanced search only) |
| `snippet_length` | integer | `200` | Approximate snippet size in bytes, 1 to 2000; any other value is a `400 Bad Request` (the MCP search tool answers it with a tool error) |
| `sort_by` | string | relevance | `created_at`, `updated_at`, `filename`, `file_size`, `ocr_status`, `mime_type` or `ocr_confidence`. Without it, enhanced search orders by relevance and `GET /api/search` newest first |
| `sort_order` | string | `desc` | `asc` or `desc` |
| `tags` | comma-separated strings | | Documents carrying any label with one of these names |
| `mime_types` | comma-separated strings | | Documents of any of these MIME types |
| `label_ids` | comma-separated UUIDs | | Documents carrying any of these labels; an invalid UUID is a 400 |
| `source_ids` | comma-separated UUIDs | | Documents ingested from any of these sources |
| `source_types` | comma-separated strings | | Documents from any of these source types; `direct_upload` also matches documents without a source |
| `ocr_status` | string | | `pending` (also documents without a status), `processing`, `completed` or `failed` |
| `created_from` | RFC 3339 timestamp | | Documents created at or after this instant |
| `created_to` | RFC 3339 timestamp | | Documents created at or before this instant |

Filters combine with AND. A query that is too short or too long, an out-of-range `limit`, `offset` or `snippet_length`, an unknown `search_mode`, `sort_by` or `ocr_status`, or `created_from` later than `created_to` is a `400 Bad Request`. Errors have the usual [error body](#errors). `GET /api/search` gives them specific codes such as `SEARCH_QUERY_TOO_SHORT`, and refuses more than 10000 matches with `413` (`SEARCH_TOO_MANY_RESULTS`).

The examples below run against a library of three documents: a PDF "invoice for march" (added 2024-03-10, labelled Finance), a PNG "scanned invoice" (2024-03-20) and a PNG "invoice copy" (2024-06-01).

#### Search Documents

```http
GET /api/search?query=invoice&limit=10
```

**Response:** `200 OK`

Simple-mode search. Returns the response below with `search_rank` set to `null`, no `snippets`, no `suggestions` and `query_time_ms` set to `0`.

#### Enhanced Search

```http
GET /api/search/enhanced?query=invoce&search_mode=fuzzy&mime_types=application/pdf
```

Search in any mode, ranked by relevance (`search_rank`; in fuzzy mode the pg_trgm word similarity of the query to the closest stretch of the text), with snippets. Each snippet's `highlight_ranges` mark the matches: in simple and boolean mode the query's words as typed; in phrase mode the whole phrase as one range (or its individual words when stemming means the phrase does not appear in the text as typed); in fuzzy mode the words spelled within the caller's threshold of a query word (so `invoce` highlights `invoice`), looking only at the first 512 KiB of each text. `start_offset` and `end_offset` locate the snippet in the document text and `highlight_ranges` locate matches in the snippet, all as UTF-8 byte offsets.

**Response:** `200 OK`
```json
{
  "documents": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "filename": "march_invoice.pdf",
      "original_filename": "march_invoice.pdf",
      "file_size": 2048,
      "mime_type": "application/pdf",
      "tags": [],
      "labels": [
        {
          "id": "6f1c2a8e-3b7d-4c1e-9a2f-0d4b5e6f7a8b",
          "user_id": "0b9e8d7c-6a5f-4e3d-2c1b-0a9f8e7d6c5b",
          "name": "Finance",
          "description": null,
          "color": "#0969da",
          "background_color": null,
          "icon": null,
          "created_at": "2024-03-01T08:00:00Z",
          "updated_at": "2024-03-01T08:00:00Z",
          "document_count": 0,
          "source_count": 0
        }
      ],
      "created_at": "2024-03-10T09:00:00Z",
      "updated_at": "2024-03-10T09:00:00Z",
      "source_id": null,
      "source_type": null,
      "has_ocr_text": true,
      "ocr_confidence": 95.5,
      "ocr_word_count": 3,
      "ocr_processing_time_ms": 1200,
      "ocr_status": "completed",
      "search_rank": 0.5714286,
      "snippets": [
        {
          "text": "invoice for march",
          "start_offset": 0,
          "end_offset": 17,
          "highlight_ranges": [{ "start": 0, "end": 7 }]
        }
      ]
    }
  ],
  "total": 1,
  "query_time_ms": 4,
  "suggestions": ["\"invoce\"", "invoce*"]
}
```

`total` counts the matches across all pages. While a document is being OCRed it also carries `ocr_progress_current` and `ocr_progress_total` (pages done and pages in total). `suggestions` are alternative queries (the query as a phrase, and with a trailing `*`) for queries longer than three characters.

#### Search Timeline

```http
GET /api/search/timeline?query=invoice
```

The number of documents matching the search per UTC creation month, over all matches rather than one page; pagination, snippet and sort parameters are ignored. Months without matches are omitted; oldest first.

**Response:** `200 OK`
```json
[
  { "month": "2024-03", "count": 2 },
  { "month": "2024-06", "count": 1 }
]
```

#### Search Facets

```http
GET /api/search/facets
```

The MIME types of the caller's documents and the names of all their labels, each with a document count (most used first, then by name), for building filters. Unused labels, such as the default labels every new account starts with, have a count of 0. Takes no parameters.

**Response:** `200 OK`
```json
{
  "mime_types": [
    { "value": "image/png", "count": 2 },
    { "value": "application/pdf", "count": 1 }
  ],
  "tags": [
    { "value": "Finance", "count": 1 },
    { "value": "Archive", "count": 0 },
    { "value": "Important", "count": 0 },
    { "value": "Personal", "count": 0 },
    { "value": "To Review", "count": 0 },
    { "value": "Work", "count": 0 }
  ]
}
```

### Labels

Labels belong to the user who creates them. Every new account starts with a few default labels.

| Endpoint | Access | What it does |
|----------|--------|--------------|
| `GET /api/labels` | user | The user's labels; with `?include_counts=true` each carries its `document_count` and `source_count` (otherwise both are 0) |
| `POST /api/labels` | user | Create a label |
| `GET /api/labels/{id}` | user | One label |
| `PUT /api/labels/{id}` | user | Change a label's `name`, `description`, `color`, `background_color` or `icon` (send only what changes) |
| `DELETE /api/labels/{id}` | user | Delete a label and remove it from every document |
| `GET /api/labels/documents/{document_id}` | user | A document's labels |
| `PUT /api/labels/documents/{document_id}` | user | Replace a document's labels |
| `POST /api/labels/documents/{document_id}/labels/{label_id}` | user | Add one label to a document |
| `DELETE /api/labels/documents/{document_id}/labels/{label_id}` | user | Remove one label from a document |
| `POST /api/labels/bulk/documents` | user | Add, remove or replace labels on many documents: `{"document_ids": [...], "label_ids": [...], "mode": "add"}`, with `mode` `add`, `remove` or `replace` |

#### Create a Label

```http
POST /api/labels
Authorization: Bearer <token>
Content-Type: application/json
```

**Request body:**
```json
{
  "name": "Invoices",
  "description": "Bills to pay",
  "color": "#0969da"
}
```

`name` and `color` (a `#rrggbb` value) are required; `description`, `background_color` and `icon` are optional.

**Response:** `201 Created`
```json
{
  "id": "6f1c2a8e-3b7d-4c1e-9a2f-0d4b5e6f7a8b",
  "user_id": "0b9e8d7c-6a5f-4e3d-2c1b-0a9f8e7d6c5b",
  "name": "Invoices",
  "description": "Bills to pay",
  "color": "#0969da",
  "background_color": null,
  "icon": null,
  "created_at": "2026-04-21T12:34:56Z",
  "updated_at": "2026-04-21T12:34:56Z",
  "document_count": 0,
  "source_count": 0
}
```

A name the user already uses gets `409 Conflict`.

#### Set a Document's Labels

```http
PUT /api/labels/documents/{document_id}
Authorization: Bearer <token>
Content-Type: application/json
```

**Request body:**
```json
{
  "label_ids": ["6f1c2a8e-3b7d-4c1e-9a2f-0d4b5e6f7a8b"]
}
```

**Response:** `200 OK` with the document's labels afterwards:
```json
[
  {
    "id": "6f1c2a8e-3b7d-4c1e-9a2f-0d4b5e6f7a8b",
    "user_id": "0b9e8d7c-6a5f-4e3d-2c1b-0a9f8e7d6c5b",
    "name": "Invoices",
    "description": null,
    "color": "#0969da",
    "background_color": null,
    "icon": null,
    "created_at": "2026-04-21T12:34:56Z",
    "updated_at": "2026-04-21T12:34:56Z",
    "document_count": 0,
    "source_count": 0
  }
]
```

The list replaces the document's labels: labels not listed are removed, and an empty list removes them all.

### Comment Endpoints

Comments belong to a document; every endpoint first checks that the caller may see the document (`404 Not Found` with `COMMENT_DOCUMENT_NOT_FOUND` otherwise).

| Endpoint | Access | What it does |
|----------|--------|--------------|
| `GET /api/comments/documents/{document_id}/comments` | user | Top-level comments, newest first, each with its reply count and first three replies (`limit` default 50, at most 100; `offset`) |
| `POST /api/comments/documents/{document_id}/comments` | user | Comment on the document, or reply to a comment |
| `GET /api/comments/documents/{document_id}/comments/{comment_id}/replies` | user | All replies to a comment, oldest first (`limit` default 50, at most 100; `offset`) |
| `PUT /api/comments/documents/{document_id}/comments/{comment_id}` | user | Change a comment's text (`{"content": "..."}`); only its author may, and it is then marked `is_edited` |
| `DELETE /api/comments/documents/{document_id}/comments/{comment_id}` | user | Delete a comment; its author or an administrator may |
| `GET /api/comments/documents/{document_id}/comments/count` | user | The number of comments on the document |

#### Create a Comment

```http
POST /api/comments/documents/{document_id}/comments
Authorization: Bearer <token>
Content-Type: application/json
```

**Request body:**
```json
{
  "content": "Paid on 12 March."
}
```

To reply, add `"parent_id"` with the comment's ID. Replies go one level deep: a reply to a reply gets `400 Bad Request` (`COMMENT_NESTING_TOO_DEEP`). The text must be 1 to 10,000 characters.

**Response:** `201 Created`
```json
{
  "id": "3e4f5a6b-7c8d-4e9f-a0b1-c2d3e4f5a6b7",
  "document_id": "550e8400-e29b-41d4-a716-446655440000",
  "user_id": "0b9e8d7c-6a5f-4e3d-2c1b-0a9f8e7d6c5b",
  "parent_id": null,
  "content": "Paid on 12 March.",
  "is_edited": false,
  "created_at": "2026-04-21T12:34:56Z",
  "updated_at": "2026-04-21T12:34:56Z",
  "username": "alice",
  "user_role": "user"
}
```

#### Count Comments

```http
GET /api/comments/documents/{document_id}/comments/count
Authorization: Bearer <token>
```

**Response:** `200 OK`
```json
{
  "count": 1
}
```

#### Comments Error Codes

| Code | HTTP Status | Meaning |
|------|-------------|---------|
| `COMMENT_NOT_FOUND` | 404 | The comment does not exist |
| `COMMENT_DOCUMENT_NOT_FOUND` | 404 | The document does not exist or the caller cannot see it |
| `COMMENT_PARENT_NOT_FOUND` | 404 | The comment being replied to does not exist |
| `COMMENT_PERMISSION_DENIED` | 403 | Changing or deleting another user's comment |
| `COMMENT_CONTENT_EMPTY` | 400 | The text is empty |
| `COMMENT_CONTENT_TOO_LONG` | 400 | The text is longer than 10,000 characters |
| `COMMENT_NESTING_TOO_DEEP` | 400 | Replying to a reply |
| `COMMENT_RATE_LIMITED` | 429 | More than 10 comments in the last minute |
| `COMMENT_INTERNAL_ERROR` | 500 | An unexpected server error |

### Shared Link Endpoints

A shared link gives anyone who has its URL read access to one document, optionally behind a password, an expiry time and a view limit.

| Endpoint | Access | What it does |
|----------|--------|--------------|
| `POST /api/shared/links` | user | Create a link to one of the caller's documents |
| `GET /api/shared/links` | user | The caller's links (administrators: every link) |
| `GET /api/shared/links/document/{document_id}` | user | The links to one document (administrators: every user's) |
| `DELETE /api/shared/links/{id}` | user | Revoke a link; its creator or an administrator may (`204 No Content`) |
| `GET /api/public/shared/{token}` | public | The shared document's name, size and type, and whether a password is needed; does not count as a view |
| `POST /api/public/shared/{token}/verify` | public | Check a link's password |
| `POST /api/public/shared/{token}/download` | public | The file as an attachment; send `{"password": "..."}` for a protected link. Counts as a view |
| `POST /api/public/shared/{token}/view` | public | The same, for display in the browser |

#### Create a Shared Link

```http
POST /api/shared/links
Authorization: Bearer <token>
Content-Type: application/json
```

**Request body:**
```json
{
  "document_id": "550e8400-e29b-41d4-a716-446655440000",
  "password": "open-sesame",
  "expires_at": "2030-12-31T23:59:59Z",
  "max_views": 10
}
```

Only `document_id` is required.

**Response:** `201 Created`
```json
{
  "id": "9a8b7c6d-5e4f-4a3b-9c2d-1e0f9a8b7c6d",
  "document_id": "550e8400-e29b-41d4-a716-446655440000",
  "token": "q6Xc1bK0yZr2uVw8ePs4LnTd",
  "url": "https://readur.example.com/shared/q6Xc1bK0yZr2uVw8ePs4LnTd",
  "has_password": true,
  "expires_at": "2030-12-31T23:59:59Z",
  "max_views": 10,
  "view_count": 0,
  "is_expired": false,
  "is_revoked": false,
  "created_at": "2026-04-21T12:34:56Z"
}
```

#### Get Shared Document Metadata

```http
GET /api/public/shared/{token}
```

**Response:** `200 OK`
```json
{
  "filename": "invoice-2024-03.pdf",
  "original_filename": "invoice-2024-03.pdf",
  "file_size": 48213,
  "mime_type": "application/pdf",
  "requires_password": false,
  "created_at": "2026-04-21T12:34:56Z"
}
```

#### Verify a Shared Link Password

```http
POST /api/public/shared/{token}/verify
Content-Type: application/json
```

**Request body:**
```json
{
  "password": "open-sesame"
}
```

**Response:** `200 OK`
```json
{
  "valid": true
}
```

A wrong password gets `403 Forbidden` (`SHARED_LINK_INVALID_PASSWORD`).

#### Shared Links Error Codes

| Code | HTTP Status | Meaning |
|------|-------------|---------|
| `SHARED_LINK_NOT_FOUND` | 404 | The token matches no link |
| `SHARED_LINK_DOCUMENT_NOT_FOUND` | 404 | The document does not exist or the caller cannot share it |
| `SHARED_LINK_EXPIRED` | 410 | The link is past its expiry time |
| `SHARED_LINK_REVOKED` | 410 | The link was revoked |
| `SHARED_LINK_MAX_VIEWS` | 410 | The link reached its view limit |
| `SHARED_LINK_PASSWORD_REQUIRED` | 401 | The link needs a password and none was sent |
| `SHARED_LINK_INVALID_PASSWORD` | 403 | The password is wrong |
| `SHARED_LINK_PERMISSION_DENIED` | 403 | The caller may not do this |
| `SHARED_LINK_RATE_LIMITED` | 429 | Too many requests from this address or user |
| `SHARED_LINK_INTERNAL_ERROR` | 500 | An unexpected server error |

### Notification Endpoints

Readur notifies users about finished syncs, OCR results and similar events.

| Endpoint | Access | What it does |
|----------|--------|--------------|
| `GET /api/notifications` | user | The user's notifications, newest first (`limit` default 25, `offset`) |
| `GET /api/notifications/summary` | user | The unread count and the most recent notifications |
| `POST /api/notifications/{id}/read` | user | Mark one notification read |
| `POST /api/notifications/read/all` | user | Mark all notifications read |
| `DELETE /api/notifications/{id}` | user | Delete a notification |

#### Notification Summary

```http
GET /api/notifications/summary
Authorization: Bearer <token>
```

**Response:** `200 OK`
```json
{
  "unread_count": 1,
  "recent_notifications": [
    {
      "id": "4b5c6d7e-8f9a-4b0c-9d1e-2f3a4b5c6d7e",
      "user_id": "0b9e8d7c-6a5f-4e3d-2c1b-0a9f8e7d6c5b",
      "notification_type": "success",
      "title": "Sync complete",
      "message": "Office NAS: 12 new documents",
      "read": false,
      "action_url": "/sources",
      "metadata": null,
      "created_at": "2026-04-21T12:34:56Z"
    }
  ]
}
```

`GET /api/notifications` returns a JSON array of the same notification objects.

### Source Endpoints

Sources import documents from a WebDAV server, an S3 bucket or a folder on the server (see the [Sources Guide](sources-guide.md)). Responses never include stored secrets: in the returned `config`, WebDAV sources report `has_password` and S3 sources `has_secret_access_key` instead. When updating a source, omit the password or secret access key, or send an empty string, to keep the stored value. Local folder sources must lie inside `LOCAL_SOURCE_ALLOWED_PATHS` when it is set; when it is not, only administrators may create them.

| Endpoint | Access | What it does |
|----------|--------|--------------|
| `GET /api/sources` | user | The user's sources |
| `POST /api/sources` | user | Create a source |
| `GET /api/sources/{id}` | user | One source (`source`), with its most recent documents (`recent_documents`) and sync progress (`sync_progress`) |
| `PUT /api/sources/{id}` | user | Change a source's `name`, `enabled` or `config` |
| `DELETE /api/sources/{id}` | user | Delete a source (stopping its sync first; `409 Conflict` if the sync does not stop) |
| `POST /api/sources/{id}/sync` | user | Start a sync now |
| `POST /api/sources/{id}/sync/stop` | user | Stop a running sync |
| `GET /api/sources/{id}/sync/status` | user | The progress of the running sync |
| `GET /api/sources/{id}/sync/progress/ws` | user | The same progress as a stream (see [Sync Progress WebSocket](#sync-progress-websocket)) |
| `POST /api/sources/{id}/scan/deep` | user | Rescan every folder of a WebDAV source, ignoring the change tracking |
| `POST /api/sources/{id}/test` | user | Test the connection of a saved source |
| `POST /api/sources/{id}/validate` | user | Check a source's configuration and health, and record the result |
| `POST /api/sources/{id}/estimate` | user | Estimate how many files a crawl of a saved source would find |
| `POST /api/sources/estimate` | user | The same for a configuration that is not saved yet |
| `POST /api/sources/test/connection` | user | Test the connection of a configuration that is not saved yet |
| `GET /api/sources/arrivals` | user | Documents added per day and ingestion lane (`days`) |

#### Create a Source

```http
POST /api/sources
Authorization: Bearer <token>
Content-Type: application/json
```

**Request body:**
```json
{
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
}
```

`source_type` is `webdav`, `s3` or `local_folder`, and `config` takes that type's fields (the OpenAPI schemas `WebDAVSourceConfig`, `S3SourceConfig` and `LocalFolderSourceConfig`).

**Response:** `201 Created`
```json
{
  "id": "2a3b4c5d-6e7f-4a8b-9c0d-1e2f3a4b5c6d",
  "name": "Office NAS",
  "source_type": "webdav",
  "enabled": true,
  "config": {
    "server_url": "https://nextcloud.example.com",
    "username": "alice",
    "has_password": true,
    "watch_folders": ["/Documents"],
    "file_extensions": ["pdf", "png", "jpg"],
    "auto_sync": true,
    "sync_interval_minutes": 60,
    "server_type": "nextcloud"
  },
  "status": "idle",
  "last_sync_at": null,
  "last_error": null,
  "last_error_at": null,
  "total_files_synced": 0,
  "total_files_pending": 0,
  "total_size_bytes": 0,
  "created_at": "2026-04-21T12:34:56Z",
  "updated_at": "2026-04-21T12:34:56Z",
  "total_documents": 0,
  "total_documents_ocr": 0,
  "validation_status": null,
  "last_validation_at": null,
  "validation_score": null,
  "validation_issues": null
}
```

An invalid configuration gets `400 Bad Request`, and a name the user already uses `409 Conflict`.

### Source Error and WebDAV Endpoints

Folders and files a source could not read are recorded as scan failures, which back off before being retried automatically. The three `/api/webdav/sync/` endpoints run the WebDAV sync configured in the user's settings, which predates sources.

| Endpoint | Access | What it does |
|----------|--------|--------------|
| `GET /api/source/errors` | user | Scan failures of all source types (filters: `source_type`, `error_type`, `severity`, `include_resolved`, `include_excluded`, `ready_for_retry`; `limit`, `offset`) |
| `GET /api/source/errors/stats` | user | Failure counts by type and severity (`source_type`) |
| `GET /api/source/errors/retry/candidates` | user | Failures due for a retry (`source_type`, `limit`) |
| `GET /api/source/errors/{failure_id}` | user | One failure |
| `POST /api/source/errors/{failure_id}/retry` | user | Retry a failure now |
| `POST /api/source/errors/{failure_id}/exclude` | user | Stop retrying a failed folder or file |
| `POST /api/source/errors/{failure_id}/resolve` | user | Mark a failure resolved |
| `GET /api/source/errors/type/{source_type}` | user | Scan failures of one source type (same filters) |
| `GET /api/source/errors/type/{source_type}/stats` | user | Failure counts for one source type |
| `GET /api/webdav/scan/failures` | user | WebDAV folder scan failures |
| `GET /api/webdav/scan/failures/retry/candidates` | user | WebDAV folders due for a retry |
| `GET /api/webdav/scan/failures/{id}` | user | One WebDAV scan failure |
| `POST /api/webdav/scan/failures/{id}/retry` | user | Retry a WebDAV folder now |
| `POST /api/webdav/scan/failures/{id}/exclude` | user | Stop retrying a WebDAV folder |
| `POST /api/webdav/test/connection` | user | Test a WebDAV connection |
| `POST /api/webdav/crawl/estimate` | user | Estimate how many files a crawl of the given folders would find |
| `GET /api/webdav/sync/status` | user | The state of the settings-based WebDAV sync |
| `POST /api/webdav/sync/start` | user | Start the settings-based WebDAV sync (`400 Bad Request` when WebDAV is not configured) |
| `POST /api/webdav/sync/cancel` | user | Cancel the settings-based WebDAV sync |

### Ignored File Endpoints

The ignored list holds files a user deleted after a source sync imported them. Deleting such a document (one at a time, in bulk, or with the low-confidence and failed-OCR clean-ups) adds an entry with the file's content hash, path, source type, source id (`source_identifier`) and the reason. From then on the user's automatic ingestion (source syncs, watch folders, batch import) skips that content; uploads are never checked. Taking an entry off the list lets the next sync import the file again. Deleting an uploaded document adds nothing.

| Endpoint | Access | What it does |
|----------|--------|--------------|
| `GET /api/ignored/files` | user | Ignored files (filters: `source_type`, `source_identifier`, `ignored_by`, `filename`; `limit` default 25, `offset`) |
| `GET /api/ignored/files/stats` | user | Counts and sizes of ignored files |
| `GET /api/ignored/files/{id}` | user | One ignored file |
| `DELETE /api/ignored/files/{id}` | user | Take a file off the ignored list |
| `DELETE /api/ignored/files/bulk/delete` | user | Take several files off the list (`{"ignored_file_ids": [...]}`) |

### OCR and Queue Endpoints

| Endpoint | Access | What it does |
|----------|--------|--------------|
| `GET /api/ocr/languages` | user | The OCR languages installed on the server, and the user's current choice |
| `GET /api/ocr/health` | user | Whether the OCR engine and its language data work |
| `GET /api/queue/stats` | admin | OCR queue counts |
| `GET /api/queue/status` | admin | Whether OCR processing is paused: `{"is_paused": false, "status": "running"}` |
| `POST /api/queue/pause` | admin | Pause OCR processing |
| `POST /api/queue/resume` | admin | Resume OCR processing |
| `POST /api/queue/requeue/failed` | admin | Put failed OCR jobs back in the queue: `{"requeued_count": 3}` |
| `POST /api/queue/enqueue/pending` | admin | Queue documents that have no OCR job yet |

#### Queue Statistics

```http
GET /api/queue/stats
Authorization: Bearer <admin token>
```

**Response:** `200 OK`
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

The two wait times are `null` while nothing is waiting.

### Settings Endpoints

| Endpoint | Access | What it does |
|----------|--------|--------------|
| `GET /api/settings` | user | The user's settings (OCR, search, display and WebDAV preferences) |
| `PUT /api/settings` | user | Change settings: send only the fields that change; the response holds all settings |
| `GET /api/settings/config` | admin | The server's effective configuration, without secrets |

The settings response never contains the stored WebDAV password; it reports `has_webdav_password` instead. The full list of fields is the OpenAPI schema `SettingsResponse` (and `UpdateSettings` for a change).

### User Endpoints

| Endpoint | Access | What it does |
|----------|--------|--------------|
| `GET /api/users` | admin | Every account |
| `POST /api/users` | admin | Create an account |
| `GET /api/users/{id}` | admin | One account |
| `PUT /api/users/{id}` | admin | Change an account's `username`, `email`, `password` or `is_active` |
| `DELETE /api/users/{id}` | admin | Delete an account (not your own); `404 Not Found` when there is no such account |
| `GET /api/users/{id}/watch/directory` | admin or self | The account's personal watch directory (see [Per-User Watch Directories](per-user-watch-directories.md)) |
| `POST /api/users/{id}/watch/directory` | admin or self | Create the personal watch directory (`{"ensure_created": true}`) |
| `DELETE /api/users/{id}/watch/directory` | admin | Remove an account's personal watch directory |

Users change their own password with [Change Own Password](#change-own-password). The watch directory endpoints answer `501 Not Implemented` (code `USER_WATCH_DISABLED`) unless the server runs with `ENABLE_PER_USER_WATCH=true`.

#### Create a User

```http
POST /api/users
Authorization: Bearer <admin token>
Content-Type: application/json
```

**Request body:**
```json
{
  "username": "carol",
  "email": "carol@example.com",
  "password": "carol-password-1",
  "role": "user"
}
```

`role` is `user` (the default) or `admin`.

**Response:** `201 Created`
```json
{
  "id": "5c6d7e8f-9a0b-4c1d-8e2f-3a4b5c6d7e8f",
  "username": "carol",
  "email": "carol@example.com",
  "role": "user",
  "is_active": true
}
```

#### Activate a User

Approves a self-registered account, which starts inactive.

```http
PUT /api/users/{id}
Authorization: Bearer <admin token>
Content-Type: application/json
```

**Request body:**
```json
{
  "is_active": true
}
```

**Response:** `200 OK`
```json
{
  "id": "1c2d3e4f-5a6b-4c7d-8e9f-0a1b2c3d4e5f",
  "username": "bob",
  "email": "bob@example.com",
  "role": "user",
  "is_active": true
}
```

An administrator cannot deactivate their own account.

### Health, Metrics and MCP Endpoints

| Endpoint | Access | What it does |
|----------|--------|--------------|
| `GET /api/health` | public | Liveness check |
| `GET /api/metrics` | admin | Database, OCR, document, user and system figures as JSON |
| `GET /metrics` | admin | The same figures in Prometheus text format; a scraper may instead send `Authorization: Bearer <METRICS_TOKEN>` |
| `POST /api/mcp` | user | The Model Context Protocol endpoint (see [AI Assistants (MCP)](mcp-guide.md)) |
| `GET /api/mcp` | user | Not used by the stateless MCP server (`405 Method Not Allowed`) |
| `DELETE /api/mcp` | user | Not used by the stateless MCP server (`405 Method Not Allowed`) |

#### Health Check

```http
GET /api/health
```

**Response:** `200 OK`
```json
{
  "status": "ok"
}
```

## Sync Progress WebSocket

`GET /api/sources/{id}/sync/progress/ws` streams the progress of a source's sync. Browsers cannot set an `Authorization` header on a WebSocket, so the session token or API key travels as the subprotocol `bearer.<token>` (an `Authorization` header works too, for clients that can send one). The request must come from an allowed origin (the server's own, or one in `CORS_ALLOWED_ORIGINS`).

```javascript
const ws = new WebSocket(
  `wss://readur.example.com/api/sources/${sourceId}/sync/progress/ws`,
  [`bearer.${token}`]
);
ws.onmessage = (event) => {
  const message = JSON.parse(event.data);
  if (message.type === 'progress') {
    console.log(`${message.data.files_processed} of ${message.data.files_found} files`);
  }
};
```

Every message is a JSON object with a `type` and a `data` object:

| `type` | When | `data` |
|--------|------|--------|
| `connection_confirmed` | Once, after connecting | `source_id`, `timestamp` (Unix seconds) |
| `progress` | Every second while a sync runs | The same fields as the `GET /api/sources/{id}/sync/status` response |
| `heartbeat` | Every second while no sync runs | `source_id`, `is_active` (`false`), `timestamp` |
| `error` | When the connection or the progress fails | `message`, `error_type` and sometimes `details` |

## Client Examples

### cURL

```bash
# Log in and keep the session token
token=$(curl -s -X POST http://localhost:8000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "alice", "password": "alice-password-1"}' | jq -r .token)

# Upload a document with an OCR language and a label
curl -X POST http://localhost:8000/api/documents \
  -H "Authorization: Bearer $token" \
  -F "file=@invoice.pdf" \
  -F "ocr_language=deu" \
  -F "label_ids=$label_id"

# Search
curl "http://localhost:8000/api/search/enhanced?query=invoice%202024" \
  -H "Authorization: Bearer $token" | jq '.documents[].filename'

# Read a document's text
curl http://localhost:8000/api/documents/$document_id/ocr \
  -H "Authorization: Bearer $token" | jq -r .ocr_text
```

### Python

```python
import requests

class ReadurClient:
    def __init__(self, base_url, api_key):
        # base_url is the server address, e.g. "http://localhost:8000"
        self.base_url = base_url
        self.headers = {"Authorization": f"Bearer {api_key}"}

    def upload(self, path):
        with open(path, "rb") as f:
            response = requests.post(
                f"{self.base_url}/api/documents",
                headers=self.headers,
                files={"file": f},
            )
        response.raise_for_status()
        return response.json()  # {"id": ..., "status": "success" or "duplicate", ...}

    def search(self, query, limit=25):
        response = requests.get(
            f"{self.base_url}/api/search/enhanced",
            headers=self.headers,
            params={"query": query, "limit": limit},
        )
        response.raise_for_status()
        return response.json()

client = ReadurClient("http://localhost:8000", "readur_pat_...")
print(client.upload("/path/to/invoice.pdf")["id"])
for doc in client.search("invoice 2024")["documents"]:
    print(doc["filename"], doc["search_rank"])
```

### TypeScript

```typescript
const baseURL = 'http://localhost:8000';

async function login(username: string, password: string): Promise<string> {
  const response = await fetch(`${baseURL}/api/auth/login`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ username, password }),
  });
  if (!response.ok) throw new Error((await response.json()).error);
  return (await response.json()).token;
}

async function upload(token: string, file: File) {
  const form = new FormData();
  form.append('file', file);
  const response = await fetch(`${baseURL}/api/documents`, {
    method: 'POST',
    headers: { Authorization: `Bearer ${token}` },
    body: form,
  });
  return response.json();
}

async function search(token: string, query: string) {
  const response = await fetch(
    `${baseURL}/api/search/enhanced?query=${encodeURIComponent(query)}`,
    { headers: { Authorization: `Bearer ${token}` } }
  );
  return response.json();
}
```

## Limits

- Request bodies, uploads included, may be at most `MAX_FILE_SIZE_MB` megabytes (50 by default); a larger request gets `413 Payload Too Large`.
- A search page holds at most 1000 results (`limit`), and `GET /api/search` refuses queries matching more than 10000 documents (`413`, `SEARCH_TOO_MANY_RESULTS`).
- Each user may hold 20 active API keys.
- The API has no version prefix; changes are listed in the release notes.
