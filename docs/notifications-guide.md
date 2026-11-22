# Notifications Guide

Readur has two separate kinds of notifications:

- **Server notifications** are stored in the database for each user when a background job finishes (mainly source syncs). They are read through the API.
- **Browser notifications** appear under the bell icon in the web interface. They are kept in the open browser tab only.

Readur does not send email, chat (Slack, Teams) or webhook notifications, and there are no notification settings.

## Browser notifications (the bell)

The bell in the top bar shows an unread count and opens a list of recent notifications. These are created by the web interface itself while you use it, currently when files you upload in that tab finish uploading: one notification for a single file ("File Uploaded" / "Upload Failed"), or one summary ("Batch Upload Complete") when several files finish within about two seconds of each other.

The list keeps the 50 most recent entries. **Mark all as read** and **Clear all** act on this list only. It is not saved anywhere: reloading the page, logging in on another device or opening another tab starts with an empty list. The bell does not show server notifications.

## Server notifications

The server stores a notification for the owner of a source or sync when one of these happens:

| Event | `notification_type` | Title |
|-------|---------------------|-------|
| A source sync (scheduled or started by hand) imported new files | `success` | Source Sync Completed |
| A source sync finished, but some files or folders failed | `warning` | Source Sync Finished With Errors |
| A source sync failed | `error` | Source Sync Failed |
| A deep scan of a WebDAV source completed | `success` | Deep Scan Completed |
| A deep scan finished with errors | `warning` | Deep Scan Finished With Errors |
| A deep scan failed | `error` | Deep Scan Failed |
| A source health check found serious problems (see [Health Monitoring](health-monitoring-guide.md#source-health-checks)) | `warning` or `error` | Source Validation Warning / Source Validation Failed |
| A scheduled sync of the WebDAV connection in the user's settings imported files | `success` | WebDAV Sync Completed |
| That scheduled WebDAV sync failed | `error` | WebDAV Sync Failed |
| A manually started settings WebDAV sync finished or failed | `success` / `error` | Manual WebDAV Sync Completed / Manual WebDAV Sync Failed |
| A running settings WebDAV sync was cancelled | `info` | WebDAV Sync Cancelled |

A source sync that imports nothing new and has no errors, or that is cancelled, does not create a notification. After every source sync that was not cancelled, Readur also runs the source's health check. Uploads, OCR results and logins do not create server notifications. Notifications are kept until the user deletes them.

### Reading them through the API

| Endpoint | What it does |
|----------|--------------|
| `GET /api/notifications` | The user's notifications, newest first. `limit` (default 25) and `offset` page through them |
| `GET /api/notifications/summary` | The number of unread notifications and the 5 most recent ones |
| `POST /api/notifications/{id}/read` | Mark one notification read |
| `POST /api/notifications/read/all` | Mark all of the user's notifications read |
| `DELETE /api/notifications/{id}` | Delete one notification |

Every endpoint needs a session token or API key and acts only on the caller's own notifications; an administrator cannot read other users' notifications. Marking or deleting an ID that does not exist (or belongs to someone else) changes nothing and still answers `200 OK`.

```bash
token="readur_pat_..."   # an API key from Settings -> API keys

# Unread count and the five newest notifications
curl -s https://readur.example.com/api/notifications/summary \
  -H "Authorization: Bearer $token"

# Mark everything read
curl -s -X POST https://readur.example.com/api/notifications/read/all \
  -H "Authorization: Bearer $token"
```

A notification looks like this:

```json
{
  "id": "4b5c6d7e-8f9a-4b0c-9d1e-2f3a4b5c6d7e",
  "user_id": "0b9e8d7c-6a5f-4e3d-2c1b-0a9f8e7d6c5b",
  "notification_type": "warning",
  "title": "Source Sync Finished With Errors",
  "message": "Office NAS: 2 of 40 files failed",
  "read": false,
  "action_url": "/sources",
  "metadata": {
    "source_type": "webdav",
    "source_id": "2a3b4c5d-6e7f-4a8b-9c0d-1e2f3a4b5c6d",
    "files_processed": 38,
    "files_failed": 2,
    "folders_failed": 0
  },
  "created_at": "2026-04-21T12:34:56Z"
}
```

- `notification_type` is `success`, `info`, `warning` or `error`.
- `action_url` is a path in the web interface related to the event (`/documents`, `/sources` or `/settings`).
- `metadata` holds event details such as the source ID and type; its keys depend on the event and may be `null`.

`GET /api/notifications/summary` answers `{"unread_count": <number>, "recent_notifications": [...]}` with the same objects. See [Notification Endpoints](api-reference.md#notification-endpoints) in the API Reference.

To be alerted outside Readur, poll `GET /api/notifications/summary` from a script with an API key, or monitor the server with Prometheus (see [Health Monitoring](health-monitoring-guide.md)).
