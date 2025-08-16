# API Reference

Readur provides a comprehensive REST API for integrating with external systems and building custom workflows.

## Table of Contents

- [Base URL](#base-url)
- [Authentication](#authentication)
- [Error Handling](#error-handling)
- [Rate Limiting](#rate-limiting)
- [Pagination](#pagination)
- [Endpoints](#endpoints)
  - [Authentication](#authentication-endpoints)
  - [Documents](#document-endpoints)
  - [Search](#search-endpoints)
  - [OCR Queue](#ocr-queue-endpoints)
  - [Settings](#settings-endpoints)
  - [Sources](#sources-endpoints)
  - [Labels](#labels-endpoints)
  - [Users](#user-endpoints)
  - [Notifications](#notification-endpoints)
  - [Metrics](#metrics-endpoints)
- [WebSocket API](#websocket-api)
- [Examples](#examples)

## Base URL

```
http://localhost:8080/api
```

For production deployments, replace with your configured domain and ensure HTTPS is used.

## Authentication

Readur supports multiple authentication methods:

### JWT Authentication

Include the token in the Authorization header:

```
Authorization: Bearer <jwt_token>
```

#### Obtaining a Token

```bash
POST /api/auth/login
Content-Type: application/json

{
  "username": "admin",
  "password": "your_password"
}
```

**Response:**
```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "user": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "username": "admin",
    "email": "admin@example.com",
    "role": "admin"
  },
  "expires_at": "2025-01-16T12:00:00Z"
}
```

#### Refresh Token

```bash
POST /api/auth/refresh
Authorization: Bearer <expired_token>
```

### OIDC Authentication

For OIDC/SSO authentication:

```bash
GET /api/auth/oidc/login
```

This will redirect to your configured OIDC provider. After successful authentication, the callback URL will receive the token.

## Error Handling

All API errors follow a consistent format:

```json
{
  "error": {
    "code": "DOCUMENT_NOT_FOUND",
    "message": "Document with ID 'abc123' not found",
    "details": {
      "document_id": "abc123",
      "user_id": "user456"
    },
    "timestamp": "2025-01-15T10:30:45Z",
    "request_id": "req_xyz789"
  }
}
```

### Error Codes

| Code | HTTP Status | Description |
|------|------------|-------------|
| `UNAUTHORIZED` | 401 | Invalid or missing authentication |
| `FORBIDDEN` | 403 | Insufficient permissions |
| `NOT_FOUND` | 404 | Resource not found |
| `VALIDATION_ERROR` | 400 | Invalid request parameters |
| `DUPLICATE_RESOURCE` | 409 | Resource already exists |
| `RATE_LIMITED` | 429 | Too many requests |
| `INTERNAL_ERROR` | 500 | Server error |
| `SERVICE_UNAVAILABLE` | 503 | Service temporarily unavailable |

## Rate Limiting

API requests are rate limited per user:

- **Default limit**: 100 requests per minute
- **Burst limit**: 20 requests
- **Upload endpoints**: 10 requests per minute

Rate limit headers are included in responses:

```
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 95
X-RateLimit-Reset: 1642248360
```

## Pagination

List endpoints support pagination using query parameters:

```bash
GET /api/documents?page=1&per_page=20&sort=created_at&order=desc
```

**Pagination Parameters:**
- `page`: Page number (default: 1)
- `per_page`: Items per page (default: 20, max: 100)
- `sort`: Sort field
- `order`: Sort order (`asc` or `desc`)

**Response includes pagination metadata:**
```json
{
  "data": [...],
  "pagination": {
    "page": 1,
    "per_page": 20,
    "total": 150,
    "total_pages": 8,
    "has_next": true,
    "has_prev": false
  }
}
```

## Endpoints

### Authentication Endpoints

#### Login

```http
POST /api/auth/login
```

**Request Body:**
```json
{
  "username": "string",
  "password": "string"
}
```

**Response:** `200 OK`
```json
{
  "token": "string",
  "user": {
    "id": "uuid",
    "username": "string",
    "email": "string",
    "role": "admin|editor|viewer"
  }
}
```

#### Register

```http
POST /api/auth/register
```

**Request Body:**
```json
{
  "username": "string",
  "email": "string",
  "password": "string"
}
```

**Response:** `201 Created`

#### Logout

```http
POST /api/auth/logout
Authorization: Bearer <token>
```

**Response:** `200 OK`

### Document Endpoints

#### List Documents

```http
GET /api/documents
```

**Query Parameters:**
- `page`: Page number
- `per_page`: Items per page
- `status`: Filter by status (`pending`, `processing`, `completed`, `failed`)
- `source_id`: Filter by source
- `label_ids`: Comma-separated label IDs
- `date_from`: Start date (ISO 8601)
- `date_to`: End date (ISO 8601)

**Response:** `200 OK`
```json
{
  "data": [
    {
      "id": "uuid",
      "title": "Document Title",
      "filename": "document.pdf",
      "content": "Extracted text content...",
      "status": "completed",
      "mime_type": "application/pdf",
      "size": 1048576,
      "created_at": "2025-01-15T10:00:00Z",
      "updated_at": "2025-01-15T10:05:00Z",
      "ocr_confidence": 0.95,
      "labels": ["label1", "label2"],
      "metadata": {
        "author": "John Doe",
        "pages": 10
      }
    }
  ],
  "pagination": {...}
}
```

#### Get Document

```http
GET /api/documents/{id}
```

**Response:** `200 OK`

#### Upload Document

```http
POST /api/documents/upload
Content-Type: multipart/form-data
```

**Request Body:**
- `file`: File to upload (required)
- `title`: Document title (optional)
- `labels`: Comma-separated label IDs (optional)
- `ocr_enabled`: Enable OCR (default: true)
- `language`: OCR language code (default: "eng")

**Response:** `201 Created`
```json
{
  "id": "uuid",
  "message": "Document uploaded successfully",
  "ocr_queued": true
}
```

#### Bulk Upload

```http
POST /api/documents/bulk-upload
Content-Type: multipart/form-data
```

**Request Body:**
- `files`: Multiple files
- `default_labels`: Labels to apply to all files

**Response:** `201 Created`
```json
{
  "uploaded": 5,
  "failed": 0,
  "documents": [...]
}
```

#### Update Document

```http
PUT /api/documents/{id}
```

**Request Body:**
```json
{
  "title": "Updated Title",
  "labels": ["label1", "label3"],
  "metadata": {
    "custom_field": "value"
  }
}
```

**Response:** `200 OK`

#### Delete Document

```http
DELETE /api/documents/{id}
```

**Response:** `204 No Content`

#### Download Document

```http
GET /api/documents/{id}/download
```

**Response:** `200 OK` with file attachment

#### Get Document Thumbnail

```http
GET /api/documents/{id}/thumbnail
```

**Query Parameters:**
- `width`: Thumbnail width (default: 200)
- `height`: Thumbnail height (default: 200)

**Response:** `200 OK` with image

#### Retry OCR

```http
POST /api/documents/{id}/retry-ocr
```

**Request Body:**
```json
{
  "language": "eng+spa",
  "priority": "high"
}
```

**Response:** `200 OK`

### Search Endpoints

#### Search Documents

```http
GET /api/search
```

**Query Parameters:**
- `q`: Search query (required)
- `page`: Page number
- `per_page`: Items per page
- `filters`: JSON-encoded filters
- `highlight`: Enable highlighting (default: true)
- `fuzzy`: Enable fuzzy search (default: false)

**Response:** `200 OK`
```json
{
  "results": [
    {
      "document_id": "uuid",
      "title": "Document Title",
      "score": 0.95,
      "highlights": [
        {
          "field": "content",
          "snippet": "...matched <mark>text</mark> here..."
        }
      ]
    }
  ],
  "total": 42,
  "facets": {
    "mime_types": {
      "application/pdf": 30,
      "text/plain": 12
    },
    "labels": {
      "important": 15,
      "archive": 27
    }
  }
}
```

#### Advanced Search

```http
POST /api/search/advanced
```

**Request Body:**
```json
{
  "query": {
    "must": [
      {"field": "content", "value": "invoice", "type": "match"}
    ],
    "should": [
      {"field": "title", "value": "2024", "type": "contains"}
    ],
    "must_not": [
      {"field": "labels", "value": "draft", "type": "exact"}
    ]
  },
  "filters": {
    "date_range": {
      "from": "2024-01-01",
      "to": "2024-12-31"
    },
    "mime_types": ["application/pdf"],
    "min_confidence": 0.8
  },
  "sort": [
    {"field": "created_at", "order": "desc"}
  ],
  "page": 1,
  "per_page": 20
}
```

### OCR Queue Endpoints

#### Get Queue Status

```http
GET /api/ocr/queue/status
```

**Response:** `200 OK`
```json
{
  "pending": 15,
  "processing": 3,
  "completed_today": 142,
  "failed": 2,
  "average_processing_time": 5.2,
  "estimated_completion": "2025-01-15T11:30:00Z"
}
```

#### List Queue Items

```http
GET /api/ocr/queue
```

**Query Parameters:**
- `status`: Filter by status
- `priority`: Filter by priority

**Response:** `200 OK`

#### Update Queue Priority

```http
PUT /api/ocr/queue/{id}/priority
```

**Request Body:**
```json
{
  "priority": "high"
}
```

#### Cancel OCR Job

```http
DELETE /api/ocr/queue/{id}
```

### Settings Endpoints

#### Get User Settings

```http
GET /api/settings
```

**Response:** `200 OK`
```json
{
  "theme": "dark",
  "language": "en",
  "notifications_enabled": true,
  "ocr_default_language": "eng",
  "items_per_page": 20
}
```

#### Update Settings

```http
PUT /api/settings
```

**Request Body:**
```json
{
  "theme": "light",
  "notifications_enabled": false
}
```

### Sources Endpoints

#### List Sources

```http
GET /api/sources
```

**Response:** `200 OK`
```json
{
  "sources": [
    {
      "id": "uuid",
      "name": "Shared Documents",
      "type": "webdav",
      "url": "https://nextcloud.example.com/remote.php/dav/files/user/",
      "status": "active",
      "last_sync": "2025-01-15T10:00:00Z",
      "next_sync": "2025-01-15T11:00:00Z",
      "document_count": 150
    }
  ]
}
```

#### Create Source

```http
POST /api/sources
```

**Request Body:**
```json
{
  "name": "Company Drive",
  "type": "webdav",
  "url": "https://drive.company.com/dav/",
  "username": "user",
  "password": "encrypted_password",
  "sync_interval": 3600,
  "recursive": true,
  "file_patterns": ["*.pdf", "*.docx"]
}
```

#### Update Source

```http
PUT /api/sources/{id}
```

#### Delete Source

```http
DELETE /api/sources/{id}
```

#### Trigger Source Sync

```http
POST /api/sources/{id}/sync
```

**Response:** `202 Accepted`
```json
{
  "message": "Sync started",
  "job_id": "job_123"
}
```

#### Get Sync Status

```http
GET /api/sources/{id}/sync-status
```

### Labels Endpoints

#### List Labels

```http
GET /api/labels
```

**Response:** `200 OK`
```json
{
  "labels": [
    {
      "id": "uuid",
      "name": "Important",
      "color": "#FF5733",
      "description": "High priority documents",
      "document_count": 42,
      "created_by": "admin",
      "created_at": "2025-01-01T00:00:00Z"
    }
  ]
}
```

#### Create Label

```http
POST /api/labels
```

**Request Body:**
```json
{
  "name": "Archive",
  "color": "#808080",
  "description": "Archived documents"
}
```

#### Update Label

```http
PUT /api/labels/{id}
```

#### Delete Label

```http
DELETE /api/labels/{id}
```

#### Assign Label to Documents

```http
POST /api/labels/{id}/assign
```

**Request Body:**
```json
{
  "document_ids": ["doc1", "doc2", "doc3"]
}
```

### User Endpoints

#### List Users (Admin only)

```http
GET /api/users
```

#### Get User Profile

```http
GET /api/users/profile
```

#### Update Profile

```http
PUT /api/users/profile
```

**Request Body:**
```json
{
  "email": "newemail@example.com",
  "display_name": "John Doe"
}
```

#### Change Password

```http
POST /api/users/change-password
```

**Request Body:**
```json
{
  "current_password": "old_password",
  "new_password": "new_secure_password"
}
```

### Notification Endpoints

#### List Notifications

```http
GET /api/notifications
```

**Query Parameters:**
- `unread_only`: Show only unread notifications

**Response:** `200 OK`
```json
{
  "notifications": [
    {
      "id": "uuid",
      "type": "ocr_completed",
      "title": "OCR Processing Complete",
      "message": "Document 'Invoice.pdf' has been processed",
      "read": false,
      "created_at": "2025-01-15T10:00:00Z",
      "data": {
        "document_id": "doc123"
      }
    }
  ]
}
```

#### Mark as Read

```http
PUT /api/notifications/{id}/read
```

#### Mark All as Read

```http
PUT /api/notifications/read-all
```

### Metrics Endpoints

#### System Metrics

```http
GET /api/metrics/system
```

**Response:** `200 OK`
```json
{
  "cpu_usage": 45.2,
  "memory_usage": 67.8,
  "disk_usage": 34.5,
  "active_connections": 23,
  "uptime_seconds": 864000
}
```

#### OCR Analytics

```http
GET /api/metrics/ocr
```

**Response:** `200 OK`
```json
{
  "total_processed": 5432,
  "success_rate": 0.98,
  "average_processing_time": 4.5,
  "languages_used": {
    "eng": 4500,
    "spa": 700,
    "fra": 232
  },
  "daily_stats": [...]
}
```

## WebSocket API

Connect to real-time updates:

```javascript
const ws = new WebSocket('ws://localhost:8080/ws');

ws.onopen = () => {
  // Authenticate
  ws.send(JSON.stringify({
    type: 'auth',
    token: 'your_jwt_token'
  }));
  
  // Subscribe to events
  ws.send(JSON.stringify({
    type: 'subscribe',
    events: ['ocr_progress', 'sync_progress', 'notifications']
  }));
};

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  
  switch(data.type) {
    case 'ocr_progress':
      console.log(`OCR Progress: ${data.progress}% for document ${data.document_id}`);
      break;
    case 'sync_progress':
      console.log(`Sync Progress: ${data.processed}/${data.total} files`);
      break;
    case 'notification':
      console.log(`New notification: ${data.message}`);
      break;
  }
};
```

### WebSocket Events

| Event Type | Description | Data Structure |
|------------|-------------|----------------|
| `ocr_progress` | OCR processing updates | `{document_id, progress, status}` |
| `sync_progress` | Source sync updates | `{source_id, processed, total, current_file}` |
| `notification` | Real-time notifications | `{id, type, title, message}` |
| `document_created` | New document added | `{document_id, title, source}` |
| `document_updated` | Document modified | `{document_id, changes}` |
| `queue_status` | OCR queue updates | `{pending, processing, completed}` |

## Examples

### Python Client Example

```python
import requests
import json

class ReadurClient:
    def __init__(self, base_url, username, password):
        self.base_url = base_url
        self.token = self._authenticate(username, password)
        self.headers = {'Authorization': f'Bearer {self.token}'}
    
    def _authenticate(self, username, password):
        response = requests.post(
            f'{self.base_url}/auth/login',
            json={'username': username, 'password': password}
        )
        return response.json()['token']
    
    def upload_document(self, file_path):
        with open(file_path, 'rb') as f:
            files = {'file': f}
            response = requests.post(
                f'{self.base_url}/documents/upload',
                headers=self.headers,
                files=files
            )
        return response.json()
    
    def search(self, query):
        response = requests.get(
            f'{self.base_url}/search',
            headers=self.headers,
            params={'q': query}
        )
        return response.json()

# Usage
client = ReadurClient('http://localhost:8080/api', 'admin', 'password')
result = client.upload_document('/path/to/document.pdf')
print(f"Document uploaded: {result['id']}")

search_results = client.search('invoice 2024')
for result in search_results['results']:
    print(f"Found: {result['title']} (score: {result['score']})")
```

### JavaScript/TypeScript Example

```typescript
class ReadurAPI {
  private token: string;
  private baseURL: string;

  constructor(baseURL: string) {
    this.baseURL = baseURL;
  }

  async login(username: string, password: string): Promise<void> {
    const response = await fetch(`${this.baseURL}/auth/login`, {
      method: 'POST',
      headers: {'Content-Type': 'application/json'},
      body: JSON.stringify({username, password})
    });
    const data = await response.json();
    this.token = data.token;
  }

  async uploadDocument(file: File): Promise<any> {
    const formData = new FormData();
    formData.append('file', file);

    const response = await fetch(`${this.baseURL}/documents/upload`, {
      method: 'POST',
      headers: {'Authorization': `Bearer ${this.token}`},
      body: formData
    });
    return response.json();
  }

  async search(query: string): Promise<any> {
    const response = await fetch(
      `${this.baseURL}/search?q=${encodeURIComponent(query)}`,
      {headers: {'Authorization': `Bearer ${this.token}`}}
    );
    return response.json();
  }
}

// Usage
const api = new ReadurAPI('http://localhost:8080/api');
await api.login('admin', 'password');

const fileInput = document.getElementById('file-input') as HTMLInputElement;
if (fileInput.files?.[0]) {
  const result = await api.uploadDocument(fileInput.files[0]);
  console.log('Uploaded:', result.id);
}
```

### cURL Examples

```bash
# Login and save token
TOKEN=$(curl -s -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"password"}' \
  | jq -r '.token')

# Upload document
curl -X POST http://localhost:8080/api/documents/upload \
  -H "Authorization: Bearer $TOKEN" \
  -F "file=@document.pdf" \
  -F "labels=important,invoice"

# Search documents
curl -X GET "http://localhost:8080/api/search?q=invoice%202024" \
  -H "Authorization: Bearer $TOKEN" | jq

# Get OCR queue status
curl -X GET http://localhost:8080/api/ocr/queue/status \
  -H "Authorization: Bearer $TOKEN" | jq

# Create a new source
curl -X POST http://localhost:8080/api/sources \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Company Drive",
    "type": "webdav",
    "url": "https://drive.company.com/dav/",
    "username": "user",
    "password": "pass",
    "sync_interval": 3600
  }'
```

## API Versioning

The API uses URL versioning. The current version is v1. Future versions will be available at:

```
/api/v2/...
```

Deprecated endpoints will include a `Deprecation` header with the sunset date:

```
Deprecation: Sun, 01 Jul 2025 00:00:00 GMT
```

## SDK Support

Official SDKs are available for:

- Python: `pip install readur-sdk`
- JavaScript/TypeScript: `npm install @readur/sdk`
- Go: `go get github.com/readur/readur-go-sdk`

## API Limits

- Maximum request size: 100MB (configurable)
- Maximum file upload: 500MB
- Maximum bulk upload: 10 files
- Maximum search results: 1000
- WebSocket connections per user: 5
- API calls per minute: 100 (configurable)