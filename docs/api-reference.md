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

#### Reprocess Document

```bash
POST /api/documents/{id}/reprocess
Authorization: Bearer <jwt_token>
```

#### Get Failed OCR Jobs

```bash
GET /api/queue/failed
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

## WebSocket API

Connect to receive real-time updates:

```javascript
const ws = new WebSocket('ws://localhost:8000/ws');

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('Event:', data);
};

// Authenticate
ws.send(JSON.stringify({
  type: 'auth',
  token: 'your_jwt_token'
}));
```

Event types:
- `document.uploaded` - New document uploaded
- `ocr.completed` - OCR processing completed
- `ocr.failed` - OCR processing failed
- `source.sync.completed` - Source sync finished

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
GET /api/openapi.json
```

You can use this with tools like Swagger UI or to generate client libraries.

## SDK Support

Official SDKs are planned for:
- Python
- JavaScript/TypeScript
- Go
- Ruby

Check the [GitHub repository](https://github.com/perfectra1n/readur) for the latest SDK availability.