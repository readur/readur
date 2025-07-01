# API Reference

Readur provides a comprehensive REST API for integrating with external systems and building custom workflows.

## Table of Contents

- [Base URL](#base-url)
- [Authentication](#authentication)
- [Error Handling](#error-handling)
- [Rate Limiting](#rate-limiting)
- [Endpoints](#endpoints)
  - [Authentication](#authentication-endpoints)
  - [Documents](#document-endpoints)
  - [Search](#search-endpoints)
  - [OCR Queue](#ocr-queue-endpoints)
  - [Settings](#settings-endpoints)
  - [Sources](#sources-endpoints)
  - [Labels](#labels-endpoints)
  - [Users](#user-endpoints)
- [WebSocket API](#websocket-api)
- [Examples](#examples)

## Base URL

```
http://localhost:8000/api
```

For production deployments, replace with your configured domain and ensure HTTPS is used.

## Authentication

Readur uses JWT (JSON Web Token) authentication. Include the token in the Authorization header:

```
Authorization: Bearer <jwt_token>
```

### Obtaining a Token

```bash
POST /api/auth/login
Content-Type: application/json

{
  "username": "admin",
  "password": "your_password"
}
```

Response:
```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "user": {
    "id": 1,
    "username": "admin",
    "email": "admin@example.com",
    "role": "admin"
  }
}
```

## Error Handling

All API errors follow a consistent format:

```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Invalid request parameters",
    "details": {
      "field": "email",
      "reason": "Invalid email format"
    }
  }
}
```

Common HTTP status codes:
- `200` - Success
- `201` - Created
- `400` - Bad Request
- `401` - Unauthorized
- `403` - Forbidden
- `404` - Not Found
- `422` - Validation Error
- `500` - Internal Server Error

## Rate Limiting

API requests are rate-limited to prevent abuse:
- Authenticated users: 1000 requests per hour
- Unauthenticated users: 100 requests per hour

Rate limit headers:
```
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 999
X-RateLimit-Reset: 1640995200
```

## Endpoints

### Authentication Endpoints

#### Register New User

```bash
POST /api/auth/register
Content-Type: application/json

{
  "username": "john_doe",
  "email": "john@example.com",
  "password": "secure_password"
}
```

#### Login

```bash
POST /api/auth/login
Content-Type: application/json

{
  "username": "john_doe",
  "password": "secure_password"
}
```

#### Get Current User

```bash
GET /api/auth/me
Authorization: Bearer <jwt_token>
```

#### OIDC Login (Redirect)

```bash
GET /api/auth/oidc/login
```

Redirects to the configured OIDC provider for authentication.

#### OIDC Callback

```bash
GET /api/auth/oidc/callback?code=<auth_code>&state=<state>
```

Handles the callback from the OIDC provider and issues a JWT token.

#### Logout

```bash
POST /api/auth/logout
Authorization: Bearer <jwt_token>
```

### Document Endpoints

#### Upload Document

```bash
POST /api/documents
Authorization: Bearer <jwt_token>
Content-Type: multipart/form-data

file: <binary_file_data>
tags: ["invoice", "2024"]  # Optional
```

Response:
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "filename": "invoice_2024.pdf",
  "mime_type": "application/pdf",
  "size": 1048576,
  "uploaded_at": "2024-01-01T00:00:00Z",
  "ocr_status": "pending"
}
```

#### List Documents

```bash
GET /api/documents?limit=50&offset=0&sort=-uploaded_at
Authorization: Bearer <jwt_token>
```

Query parameters:
- `limit` - Number of results (default: 50, max: 100)
- `offset` - Pagination offset
- `sort` - Sort field (prefix with `-` for descending)
- `mime_type` - Filter by MIME type
- `ocr_status` - Filter by OCR status
- `tag` - Filter by tag

#### Get Document Details

```bash
GET /api/documents/{id}
Authorization: Bearer <jwt_token>
```

#### Download Document

```bash
GET /api/documents/{id}/download
Authorization: Bearer <jwt_token>
```

#### Delete Document

```bash
DELETE /api/documents/{id}
Authorization: Bearer <jwt_token>
```

#### Update Document

```bash
PATCH /api/documents/{id}
Authorization: Bearer <jwt_token>
Content-Type: application/json

{
  "tags": ["invoice", "paid", "2024"]
}
```

#### Get Document Debug Information

```bash
GET /api/documents/{id}/debug
Authorization: Bearer <jwt_token>
```

Response:
```json
{
  "document_id": "550e8400-e29b-41d4-a716-446655440000",
  "processing_pipeline": {
    "upload": "completed",
    "ocr_queue": "completed", 
    "ocr_processing": "completed",
    "validation": "completed"
  },
  "ocr_details": {
    "confidence": 89.5,
    "word_count": 342,
    "processing_time": 4.2
  },
  "file_info": {
    "mime_type": "application/pdf",
    "size": 1048576,
    "pages": 3
  }
}
```

#### Get Document Thumbnail

```bash
GET /api/documents/{id}/thumbnail
Authorization: Bearer <jwt_token>
```

#### Get Document OCR Text

```bash
GET /api/documents/{id}/ocr
Authorization: Bearer <jwt_token>
```

#### Get Document Processed Image

```bash
GET /api/documents/{id}/processed-image
Authorization: Bearer <jwt_token>
```

#### View Document in Browser

```bash
GET /api/documents/{id}/view
Authorization: Bearer <jwt_token>
```

#### Get Failed Documents

```bash
GET /api/documents/failed?limit=50&offset=0
Authorization: Bearer <jwt_token>
```

Query parameters:
- `limit` - Number of results (default: 50)
- `offset` - Pagination offset
- `stage` - Filter by failure stage
- `reason` - Filter by failure reason

#### View Failed Document

```bash
GET /api/documents/failed/{id}/view
Authorization: Bearer <jwt_token>
```

#### Get Duplicate Documents

```bash
GET /api/documents/duplicates?limit=50&offset=0
Authorization: Bearer <jwt_token>
```

#### Delete Low Confidence Documents

```bash
POST /api/documents/delete-low-confidence
Authorization: Bearer <jwt_token>
Content-Type: application/json

{
  "confidence_threshold": 70.0,
  "preview_only": false
}
```

#### Delete Failed OCR Documents

```bash
POST /api/documents/delete-failed-ocr
Authorization: Bearer <jwt_token>
Content-Type: application/json

{
  "preview_only": false
}
```

#### Bulk Delete Documents

```bash
DELETE /api/documents
Authorization: Bearer <jwt_token>
Content-Type: application/json

{
  "document_ids": ["550e8400-e29b-41d4-a716-446655440000", "..."]
}
```

### Search Endpoints

#### Search Documents

```bash
GET /api/search?query=invoice&limit=20
Authorization: Bearer <jwt_token>
```

Query parameters:
- `query` - Search query (required)
- `limit` - Number of results
- `offset` - Pagination offset
- `mime_types` - Comma-separated MIME types
- `tags` - Comma-separated tags
- `date_from` - Start date (ISO 8601)
- `date_to` - End date (ISO 8601)

Response:
```json
{
  "results": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "filename": "invoice_2024.pdf",
      "snippet": "...invoice for services rendered in Q1 2024...",
      "score": 0.95,
      "highlights": ["invoice", "2024"]
    }
  ],
  "total": 42,
  "limit": 20,
  "offset": 0
}
```

#### Advanced Search

```bash
POST /api/search/advanced
Authorization: Bearer <jwt_token>
Content-Type: application/json

{
  "query": "invoice",
  "filters": {
    "mime_types": ["application/pdf"],
    "tags": ["unpaid"],
    "date_range": {
      "from": "2024-01-01",
      "to": "2024-12-31"
    },
    "file_size": {
      "min": 1024,
      "max": 10485760
    }
  },
  "options": {
    "fuzzy": true,
    "snippet_length": 200
  }
}
```

### OCR Queue Endpoints

#### Get Queue Status

```bash
GET /api/queue/status
Authorization: Bearer <jwt_token>
```

Response:
```json
{
  "pending": 15,
  "processing": 3,
  "completed_today": 127,
  "failed_today": 2,
  "average_processing_time": 4.5
}
```

#### Retry OCR Processing

```bash
POST /api/documents/{id}/retry-ocr
Authorization: Bearer <jwt_token>
```

#### Get Failed OCR Jobs

```bash
GET /api/queue/failed
Authorization: Bearer <jwt_token>
```

#### Get Queue Statistics

```bash
GET /api/queue/stats
Authorization: Bearer <jwt_token>
```

Response:
```json
{
  "pending_count": 15,
  "processing_count": 3,
  "failed_count": 2,
  "completed_today": 127,
  "average_processing_time_seconds": 4.5,
  "queue_health": "healthy"
}
```

#### Requeue Failed Items

```bash
POST /api/queue/requeue-failed
Authorization: Bearer <jwt_token>
```

#### Enqueue Pending Documents

```bash
POST /api/queue/enqueue-pending
Authorization: Bearer <jwt_token>
```

#### Pause OCR Processing

```bash
POST /api/queue/pause
Authorization: Bearer <jwt_token>
```

#### Resume OCR Processing

```bash
POST /api/queue/resume
Authorization: Bearer <jwt_token>
```

### Settings Endpoints

#### Get User Settings

```bash
GET /api/settings
Authorization: Bearer <jwt_token>
```

#### Update User Settings

```bash
PUT /api/settings
Authorization: Bearer <jwt_token>
Content-Type: application/json

{
  "ocr_language": "eng",
  "search_results_per_page": 50,
  "enable_notifications": true
}
```

### Sources Endpoints

#### List Sources

```bash
GET /api/sources
Authorization: Bearer <jwt_token>
```

#### Create Source

```bash
POST /api/sources
Authorization: Bearer <jwt_token>
Content-Type: application/json

{
  "name": "Network Drive",
  "type": "local_folder",
  "config": {
    "path": "/mnt/network/documents",
    "scan_interval": 3600
  },
  "enabled": true
}
```

#### Update Source

```bash
PUT /api/sources/{id}
Authorization: Bearer <jwt_token>
Content-Type: application/json

{
  "enabled": false
}
```

#### Delete Source

```bash
DELETE /api/sources/{id}
Authorization: Bearer <jwt_token>
```

#### Sync Source

```bash
POST /api/sources/{id}/sync
Authorization: Bearer <jwt_token>
```

#### Stop Source Sync

```bash
POST /api/sources/{id}/sync/stop
Authorization: Bearer <jwt_token>
```

#### Test Source Connection

```bash
POST /api/sources/{id}/test
Authorization: Bearer <jwt_token>
```

#### Estimate Source Crawl

```bash
POST /api/sources/{id}/estimate
Authorization: Bearer <jwt_token>
```

#### Estimate Crawl with Configuration

```bash
POST /api/sources/estimate
Authorization: Bearer <jwt_token>
Content-Type: application/json

{
  "source_type": "webdav",
  "config": {
    "url": "https://example.com/webdav",
    "username": "user",
    "password": "pass"
  }
}
```

#### Test Connection with Configuration

```bash
POST /api/sources/test-connection
Authorization: Bearer <jwt_token>
Content-Type: application/json

{
  "source_type": "webdav", 
  "config": {
    "url": "https://example.com/webdav",
    "username": "user",
    "password": "pass"
  }
}
```

### WebDAV Endpoints

#### Test WebDAV Connection

```bash
POST /api/webdav/test-connection
Authorization: Bearer <jwt_token>
Content-Type: application/json

{
  "url": "https://example.com/webdav",
  "username": "user",
  "password": "pass"
}
```

#### Estimate WebDAV Crawl

```bash
POST /api/webdav/estimate-crawl
Authorization: Bearer <jwt_token>
Content-Type: application/json

{
  "url": "https://example.com/webdav",
  "username": "user", 
  "password": "pass"
}
```

#### Get WebDAV Sync Status

```bash
GET /api/webdav/sync-status
Authorization: Bearer <jwt_token>
```

#### Start WebDAV Sync

```bash
POST /api/webdav/start-sync
Authorization: Bearer <jwt_token>
Content-Type: application/json

{
  "url": "https://example.com/webdav",
  "username": "user",
  "password": "pass"
}
```

#### Cancel WebDAV Sync

```bash
POST /api/webdav/cancel-sync
Authorization: Bearer <jwt_token>
```

### Labels Endpoints

#### List Labels

```bash
GET /api/labels
Authorization: Bearer <jwt_token>
```

#### Create Label

```bash
POST /api/labels
Authorization: Bearer <jwt_token>
Content-Type: application/json

{
  "name": "Important",
  "color": "#FF0000"
}
```

#### Update Label

```bash
PUT /api/labels/{id}
Authorization: Bearer <jwt_token>
Content-Type: application/json

{
  "name": "Very Important",
  "color": "#FF00FF"
}
```

#### Delete Label

```bash
DELETE /api/labels/{id}
Authorization: Bearer <jwt_token>
```

### User Endpoints

#### List Users (Admin Only)

```bash
GET /api/users
Authorization: Bearer <jwt_token>
```

#### Get User

```bash
GET /api/users/{id}
Authorization: Bearer <jwt_token>
```

#### Update User

```bash
PUT /api/users/{id}
Authorization: Bearer <jwt_token>
Content-Type: application/json

{
  "email": "newemail@example.com",
  "role": "user"
}
```

#### Delete User (Admin Only)

```bash
DELETE /api/users/{id}
Authorization: Bearer <jwt_token>
```

### Notifications Endpoints

#### List Notifications

```bash
GET /api/notifications?limit=50&offset=0
Authorization: Bearer <jwt_token>
```

#### Get Notification Summary

```bash
GET /api/notifications/summary
Authorization: Bearer <jwt_token>
```

Response:
```json
{
  "unread_count": 5,
  "total_count": 23,
  "latest_notification": {
    "id": 1,
    "type": "ocr_completed",
    "message": "OCR processing completed for document.pdf",
    "created_at": "2024-01-01T12:00:00Z"
  }
}
```

#### Mark Notification as Read

```bash
POST /api/notifications/{id}/read
Authorization: Bearer <jwt_token>
```

#### Mark All Notifications as Read

```bash
POST /api/notifications/read-all
Authorization: Bearer <jwt_token>
```

#### Delete Notification

```bash
DELETE /api/notifications/{id}
Authorization: Bearer <jwt_token>
```

### Ignored Files Endpoints

#### List Ignored Files

```bash
GET /api/ignored-files?limit=50&offset=0
Authorization: Bearer <jwt_token>
```

Query parameters:
- `limit` - Number of results (default: 50)
- `offset` - Pagination offset
- `filename` - Filter by filename
- `source_type` - Filter by source type

#### Get Ignored Files Statistics

```bash
GET /api/ignored-files/stats
Authorization: Bearer <jwt_token>
```

Response:
```json
{
  "total_ignored_files": 42,
  "total_size_bytes": 104857600,
  "most_recent_ignored_at": "2024-01-01T12:00:00Z"
}
```

#### Get Ignored File Details

```bash
GET /api/ignored-files/{id}
Authorization: Bearer <jwt_token>
```

#### Remove File from Ignored List

```bash
DELETE /api/ignored-files/{id}
Authorization: Bearer <jwt_token>
```

#### Bulk Remove Files from Ignored List

```bash
DELETE /api/ignored-files/bulk-delete
Authorization: Bearer <jwt_token>
Content-Type: application/json

{
  "ignored_file_ids": [1, 2, 3, 4]
}
```

### Metrics Endpoints

#### Get System Metrics

```bash
GET /api/metrics
Authorization: Bearer <jwt_token>
```

#### Get Prometheus Metrics

```bash
GET /metrics
```

Returns Prometheus-formatted metrics (no authentication required).

### Health Check

#### Health Check

```bash
GET /api/health
```

Response:
```json
{
  "status": "healthy",
  "timestamp": "2024-01-01T12:00:00Z",
  "version": "1.0.0"
}
```

## Examples

### Python Example

```python
import requests

# Configuration
BASE_URL = "http://localhost:8000/api"
USERNAME = "admin"
PASSWORD = "your_password"

# Login
response = requests.post(f"{BASE_URL}/auth/login", json={
    "username": USERNAME,
    "password": PASSWORD
})
token = response.json()["token"]
headers = {"Authorization": f"Bearer {token}"}

# Upload document
with open("document.pdf", "rb") as f:
    files = {"file": ("document.pdf", f, "application/pdf")}
    response = requests.post(
        f"{BASE_URL}/documents",
        headers=headers,
        files=files
    )
    document_id = response.json()["id"]

# Search documents
response = requests.get(
    f"{BASE_URL}/search",
    headers=headers,
    params={"query": "invoice 2024"}
)
results = response.json()["results"]
```

### JavaScript Example

```javascript
// Configuration
const BASE_URL = 'http://localhost:8000/api';

// Login
async function login(username, password) {
  const response = await fetch(`${BASE_URL}/auth/login`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ username, password })
  });
  const data = await response.json();
  return data.token;
}

// Upload document
async function uploadDocument(token, file) {
  const formData = new FormData();
  formData.append('file', file);
  
  const response = await fetch(`${BASE_URL}/documents`, {
    method: 'POST',
    headers: { 'Authorization': `Bearer ${token}` },
    body: formData
  });
  return response.json();
}

// Search documents
async function searchDocuments(token, query) {
  const response = await fetch(
    `${BASE_URL}/search?query=${encodeURIComponent(query)}`,
    {
      headers: { 'Authorization': `Bearer ${token}` }
    }
  );
  return response.json();
}
```

### cURL Examples

```bash
# Login
TOKEN=$(curl -s -X POST http://localhost:8000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"your_password"}' \
  | jq -r .token)

# Upload document
curl -X POST http://localhost:8000/api/documents \
  -H "Authorization: Bearer $TOKEN" \
  -F "file=@document.pdf"

# Search documents
curl -X GET "http://localhost:8000/api/search?query=invoice" \
  -H "Authorization: Bearer $TOKEN"

# Get document
curl -X GET http://localhost:8000/api/documents/550e8400-e29b-41d4-a716-446655440000 \
  -H "Authorization: Bearer $TOKEN"
```

## OpenAPI Specification

The complete OpenAPI specification is available at:
```
GET /api-docs/openapi.json
```

Interactive Swagger UI documentation is available at:
```
GET /swagger-ui
```

You can use this with tools like Swagger UI or to generate client libraries.

## SDK Support

Official SDKs are planned for:
- Python
- JavaScript/TypeScript
- Go
- Ruby

Check the [GitHub repository](https://github.com/perfectra1n/readur) for the latest SDK availability.