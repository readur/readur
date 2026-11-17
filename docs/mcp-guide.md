# AI Assistants (MCP)

Readur has a built-in [Model Context Protocol](https://modelcontextprotocol.io) (MCP) server. MCP is an open protocol that lets AI assistants such as Claude Code, Claude Desktop and other MCP clients call tools on your behalf. Once connected, an assistant can search your documents, read their OCR text, upload new files, manage labels and check on processing — all with your account's permissions.

The endpoint is part of the Readur server itself; there is nothing extra to install or run. It replaces the separate `readur-mcp` Node.js server, which is no longer needed.

## At a glance

| | |
|---|---|
| Endpoint | `https://readur.example.com/api/mcp` |
| Transport | Streamable HTTP (stateless, JSON responses) |
| Authentication | `Authorization: Bearer <token>` — a personal API key (`readur_pat_...`) or a session token |
| Enabled by | `MCP_ENABLED` (default `true`) |

## Connecting a client

### 1. Create an API key

In Readur, open **Settings → API keys** and select **Create API key**. Copy the key when it is shown — it is displayed only once.

When MCP is enabled, the new-key dialog and the **Connect an AI assistant** panel on the same page show ready-to-paste configuration for your server, with the key already filled in.

See [API Key Authentication](api-reference.md#api-key-authentication) for more about keys.

### 2. Add Readur to your client

#### Claude Code

```bash
claude mcp add --transport http readur https://readur.example.com/api/mcp \
  --header "Authorization: Bearer readur_pat_XXXXXXXX"
```

Run `claude mcp list` to check the connection, then ask Claude something like *"Find my invoices from March in Readur"*.

#### Claude Desktop

Claude Desktop's `claude_desktop_config.json` only launches local (stdio) servers — a `url` entry is ignored — so it reaches Readur through the [`mcp-remote`](https://www.npmjs.com/package/mcp-remote) bridge, which needs Node.js. Add this, then restart Claude Desktop:

```json
{
  "mcpServers": {
    "readur": {
      "command": "npx",
      "args": ["-y", "mcp-remote", "https://readur.example.com/api/mcp", "--header", "Authorization:${READUR_AUTH}"],
      "env": {
        "READUR_AUTH": "Bearer readur_pat_XXXXXXXX"
      }
    }
  }
}
```

The header value lives in `env` because a space inside an `args` entry is split apart on some platforms. (Desktop's *Custom Connector* setting expects OAuth, which Readur's API keys are not.)

#### Clients with native HTTP support

Clients that connect to remote servers themselves and read an `mcpServers` file (for example Claude Code's `.mcp.json`) take a block like this:

```json
{
  "mcpServers": {
    "readur": {
      "type": "http",
      "url": "https://readur.example.com/api/mcp",
      "headers": {
        "Authorization": "Bearer readur_pat_XXXXXXXX"
      }
    }
  }
}
```

Some clients name the transport `streamable-http` instead of `http`.

#### Any other client

Point the client at `https://readur.example.com/api/mcp` using the Streamable HTTP transport and send the `Authorization: Bearer` header with every request. The server is stateless: no session ID is issued, and each request is authenticated on its own.

## Available tools

Every tool acts as the user who owns the key and sees exactly what that user sees in the web interface and REST API — no more.

### Account and server

| Tool | What it does |
|------|--------------|
| `whoami` | The account the assistant is connected as |
| `get_settings` | Your Readur settings (OCR language, search defaults, and so on) |
| `get_queue_stats` | OCR queue status: pending, processing and failed jobs |
| `get_ocr_languages` | OCR languages available on this server |

### Documents

| Tool | What it does |
|------|--------------|
| `list_documents` | List documents, newest first, with optional MIME type and OCR status filters (paginated) |
| `get_document` | Metadata for one document: filename, size, type, OCR status |
| `get_document_ocr_text` | The extracted text of a document, in pages (`offset` / `max_chars`) so large documents don't overflow the assistant's context |
| `upload_document` | Upload a file (base64-encoded), optionally with labels; it is queued for OCR |
| `retry_document_ocr` | Queue OCR again for a document |
| `delete_document` | Delete a document. **Only available when `MCP_ALLOW_DELETE=true`** |

### Search

| Tool | What it does |
|------|--------------|
| `search_documents` | Full-text search over document names and OCR text |
| `enhanced_search` | Search with a mode (simple, phrase, fuzzy, boolean) and text snippets around each match. See [Boolean search syntax](#boolean-search-syntax) |
| `get_search_facets` | Document counts per MIME type and tag, for narrowing a search |

#### Boolean search syntax

With `search_mode: "boolean"`, `enhanced_search` (and the web UI's operator mode, `GET /api/search/enhanced?search_mode=boolean`) understands:

| Syntax | Meaning |
|--------|---------|
| `invoice paid`, `invoice & paid`, `invoice AND paid` | Both words |
| `invoice \| receipt`, `invoice OR receipt` | Either word |
| `!draft`, `NOT draft`, `-draft` | Leave a word out. A `-` only negates when attached to what follows (`e-mail` is one word) |
| `(invoice \| receipt) & 2024` | Parentheses group. AND binds tighter than OR, so `a \| b c` means `a \| (b & c)` |
| `"total due"` | Exact phrase. `-"draft copy"` and `NOT (draft \| copy)` negate a phrase or a group |
| `inv:*`, `inv*` | Prefix: words starting with `inv` (invoice, inventory) |
| `total <-> due`, `invoice <2> 17` | Followed by: the second word right after the first, or N words after it |

`AND`, `OR` and `NOT` work in any case (as search words they are stop words that never match). Words are stemmed, so `invoices` also finds `invoice`. Malformed input never fails the search: stray operators and empty groups are ignored, and an unclosed parenthesis or quote is closed at the end of the query. Snippets highlight only the words a match must contain, not negated ones.

### Labels

| Tool | What it does |
|------|--------------|
| `list_labels` | Your labels, optionally with document counts |
| `create_label` | Create a label with a name, colour, description and icon |
| `update_label` | Rename or restyle a label |
| `get_document_labels` | The labels on a document |
| `add_document_label` | Add a label to a document |
| `remove_document_label` | Remove a label from a document |

### Sources

| Tool | What it does |
|------|--------------|
| `list_sources` | Your WebDAV, local folder and S3 sources (credentials are never returned) |
| `get_source_sync_status` | Sync progress and last result for one source |

### Resources

Documents are also exposed as MCP resources through the template `readur://documents/{id}`. A client that supports resources can attach the original file to the conversation: text files are returned as text, other files as binary content up to 10 MiB.

## Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `MCP_ENABLED` | `true` | Serve the MCP endpoint at `/api/mcp`. When `false`, the endpoint returns `404` and the settings page hides the **Connect an AI assistant** panel |
| `MCP_ALLOW_DELETE` | `false` | Expose the `delete_document` tool. Leave it off unless you want assistants to be able to delete documents |

Both are read at startup. See the [Configuration Reference](configuration-reference.md#ai-assistants-mcp).

If Readur runs behind a reverse proxy, `/api/mcp` needs no special handling: it is an ordinary HTTP `POST` endpoint and does not use server-sent events or long-lived connections.

## Security

- **A key is the user.** An assistant holding an API key can do anything that user can do through the tools above. Give it a key from an account with only the access it needs.
- **Use a dedicated key per client**, with an expiry, so you can tell them apart under **Last used** and revoke one without affecting the others.
- **Revoke in settings.** Revoking a key under **Settings → API keys** cuts off any client using it immediately.
- **Deletion is off by default.** `delete_document` is not offered to clients unless `MCP_ALLOW_DELETE=true`. Other tools can change data (upload, labels, OCR retry) but do not remove documents.
- **Treat document text as untrusted.** Text extracted from documents is passed to the assistant as-is. A document can contain instructions aimed at the assistant ("prompt injection"); review what an assistant proposes to do, especially before allowing uploads or label changes.
- **Every request is authenticated.** Requests without a valid token get `401 Unauthorized`; there is no anonymous access.

## Migrating from readur-mcp

The separate `readur-mcp` server is superseded by this endpoint. To switch:

1. Remove the `readur-mcp` entry from your client configuration (and stop the process, if you ran it as a service).
2. Add Readur as shown in [Connecting a client](#connecting-a-client), using an API key instead of a username and password.

Tool names are similar but not identical, and parameters use `snake_case`; assistants pick up the new list automatically.

## Troubleshooting

| Symptom | Cause |
|---------|-------|
| `401 Unauthorized` | Missing `Authorization` header, or the key is invalid, expired or revoked |
| `404 Not Found` | `MCP_ENABLED=false` on the server, or the URL is missing `/api/mcp` |
| `delete_document` is missing | Expected unless `MCP_ALLOW_DELETE=true` |
| OCR text tool reports a status instead of text | The document hasn't finished OCR yet; check again later or use `retry_document_ocr` if it failed |
