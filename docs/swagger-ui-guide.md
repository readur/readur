# Swagger UI Guide

Readur publishes an OpenAPI description of its REST API and serves a Swagger UI for browsing it and calling endpoints from the browser.

## Where to find it

| What | Path |
|------|------|
| Swagger UI | `/swagger-ui` (for example `https://readur.example.com/swagger-ui`) |
| OpenAPI document (JSON) | `/api-docs/openapi.json` |

In the web interface, open the account menu (your avatar) and choose **API documentation**; it opens the Swagger UI in a new tab.

Both paths are always served and are not behind a login: anyone who can reach the server can read the API description. Calling an endpoint that needs a login still requires a credential, exactly as it does outside Swagger UI. There are no settings to move or switch off the Swagger UI.

## What it contains

The OpenAPI document is generated from the server code, so its paths, parameters and schemas match the running version. Endpoints are grouped by tag: `auth`, `documents`, `labels`, `search`, `settings`, `users`, `queue`, `metrics`, `notifications`, `sources`, `webdav`, `ignored_files`, `ocr` and `health`.

Some endpoints are not in the OpenAPI document: API keys, comments, shared links, the MCP endpoint and a few document endpoints. The [API Reference](api-reference.md) covers every endpoint, including these, and describes the behaviour around them (errors, rate limits, access levels).

## Authorizing requests

Endpoints that need a login are marked with a lock. To call them with **Try it out**:

1. Get a token. Either create a personal API key in **Settings → API keys** (it starts with `readur_pat_`), or run `POST /api/auth/login` in Swagger UI with your username and password and copy the `token` from the response.
2. Click **Authorize** at the top of the page.
3. Paste the token into the `bearer_auth` field, without the word `Bearer` (Swagger UI adds it), and confirm.

Requests then carry `Authorization: Bearer <token>` and run with that user's permissions: an endpoint for administrators answers `403 Forbidden` to a regular user. Session tokens expire after `JWT_TTL_HOURS` hours (12 by default) and there are no refresh tokens; log in again when one expires. The same rate limits apply as to any other client (see [Rate Limiting](api-reference.md#rate-limiting)).

**Try it out** runs against the server's real data. A `DELETE` call deletes for real.

## Using the OpenAPI document elsewhere

The JSON at `/api-docs/openapi.json` can be imported into API tools or code generators that read OpenAPI:

```bash
curl -s https://readur.example.com/api-docs/openapi.json -o readur-openapi.json
```

Remember that the endpoints missing from the document (see above) have to be added by hand from the [API Reference](api-reference.md).

## Security notes

- Serve Readur over HTTPS (see [Reverse Proxy](REVERSE_PROXY.md)) so tokens typed into Swagger UI are not sent in clear text.
- A token pasted into **Authorize** is kept only in that browser tab. Use a short-lived API key for experiments and revoke it afterwards in **Settings → API keys**.
