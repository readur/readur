# Installation Guide

Readur runs as a single server process next to a PostgreSQL database. The usual way to run it is the Docker image `ghcr.io/readur/readur`, with Docker Compose or the Helm chart. To build and run it without Docker, see [Manual Installation](../installation.md#manual-installation).

## Prerequisites

- Docker with the Compose plugin (`docker compose version` should work), or a Kubernetes cluster with Helm for the Helm chart.
- Enough CPU and memory for OCR. OCR is the heaviest work Readur does; each of the `CONCURRENT_OCR_JOBS` parallel jobs (default `4`) runs Tesseract on one document.
- Disk space for your documents. Documents are stored under `UPLOAD_PATH` (`/app/uploads` in the container), or in S3 when the S3 backend is enabled.

The Docker image contains the `readur` binary at `/app/readur`, the web interface, Tesseract with all language packs, `ocrmypdf`, `poppler-utils` and `antiword`/`catdoc` for legacy Word files. Published tags are `latest` (the most recent release), release versions such as `2.6.0`, and `main` (built from the main branch).

## Docker Compose

### 1. Get the Compose File

The repository's `docker-compose.yml` builds the image from source (`build: .`):

```bash
git clone https://github.com/readur/readur.git
cd readur
```

To use the published image instead of building it, replace `build: .` in the `readur` service with `image: ghcr.io/readur/readur:latest`, or use the standalone compose file in the [Docker Quick Start](../quickstart/docker.md).

### 2. Configure

The compose file reads `JWT_SECRET`, `ADMIN_USERNAME`, `ADMIN_PASSWORD` and `ADMIN_EMAIL` from your shell or from a `.env` file next to it. All four are optional:

```bash
# .env
ADMIN_PASSWORD=choose-a-long-password
# Optional; without it a signing key is generated and stored in the database.
# JWT_SECRET=<output of: openssl rand -hex 32>
```

Other settings, such as `OCR_LANGUAGE` and `CONCURRENT_OCR_JOBS`, are set directly under `environment:` in `docker-compose.yml`. Change the PostgreSQL password there too (`POSTGRES_PASSWORD` of the `postgres` service and the password in `DATABASE_URL`) before the database is first created. See the [Configuration Guide](configuration.md).

The compose file stores documents in `./readur_uploads`, the watch folder in `./readur_watch`, and the database in the `postgres_data` volume.

### 3. Start

```bash
docker compose up -d --build
docker compose logs -f readur
```

Startup is complete when the log shows `✅ HTTP server bound to: 0.0.0.0:8000`. Database migrations run automatically before that.

### 4. Verify

```bash
docker compose ps
curl http://localhost:8000/api/health
```

The health endpoint answers:

```json
{
  "status": "ok"
}
```

## Kubernetes with Helm

The chart in `charts/readur` deploys Readur only; it does not deploy PostgreSQL. Provide a database and pass its connection string in `DATABASE_URL`.

The chart mounts an existing PersistentVolumeClaim for documents, named by `existingClaim` under `persistence.uploads` (installation fails without it), and creates a claim for the watch folder. Create the uploads claim first:

```yaml
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: readur-uploads
spec:
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: 50Gi
```

Then write a values file, `my-values.yaml`:

```yaml
controllers:
  main:
    containers:
      readur:
        image:
          repository: ghcr.io/readur/readur
          tag: latest
        env:
          DATABASE_URL: postgresql://readur:your_password@postgres.example.com:5432/readur
          PUBLIC_URL: https://readur.example.com
persistence:
  uploads:
    existingClaim: readur-uploads
ingress:
  main:
    enabled: true
    hosts:
      - host: readur.example.com
        paths:
          - path: /
            pathType: Prefix
            service:
              identifier: main
              port: http
```

The chart is built on the [bjw-s common library](https://github.com/bjw-s/helm-charts), so other values of that library work too. Install from a clone of the repository:

```bash
kubectl apply -f readur-uploads-pvc.yaml
helm dependency build charts/readur
helm install readur charts/readur -f my-values.yaml
```

The chart creates a Secret named `<release>-auth` with a random initial `ADMIN_PASSWORD`, kept across upgrades. `JWT_SECRET` is optional: without it Readur generates a signing key and stores it in its database. To supply your own values, set `auth.jwtSecret` / `auth.adminPassword`, or point `auth.existingSecret` at a Secret that may contain `JWT_SECRET` (at least 32 bytes) and `ADMIN_PASSWORD`.

**GitOps / `helm template`:** keeping the generated admin password across upgrades relies on Helm's `lookup` function, which only works when Helm talks to the cluster (`helm install` / `helm upgrade`). Renderers that do not, such as `helm template` and Argo CD, cannot see the existing Secret and generate a new value on every render. With these tools, set `auth.existingSecret` to a Secret you manage (for example with Sealed Secrets or External Secrets), or set `auth.adminPassword` explicitly.

Read the initial admin password (for a release named `readur`):

```bash
kubectl get secret readur-auth -o jsonpath='{.data.ADMIN_PASSWORD}' | base64 -d
```

Run only one Readur replica.

## Post-Installation Setup

### Sign In

Open `http://localhost:8000` (or your public URL) and sign in as `admin` (or your `ADMIN_USERNAME`).

The password is `ADMIN_PASSWORD` if you set it. Otherwise Readur generated a random password on first startup and wrote it to `.readur/initial-admin-password` inside `UPLOAD_PATH` (`/app/uploads/.readur/initial-admin-password` in the container, mode `0600`). It is not written to the logs; the log shows the file path. Read it with:

```bash
docker compose exec readur cat /app/uploads/.readur/initial-admin-password
```

After signing in and changing the password, delete the file.

### Reset the Admin Password

```bash
docker compose exec readur /app/readur reset-admin-password
```

This sets the password of the `ADMIN_USERNAME` account to `ADMIN_PASSWORD`, or, when that is empty, to a new random password printed to the terminal. With Kubernetes:

```bash
kubectl exec deployment/readur -- /app/readur reset-admin-password
```

### OCR Languages

The Docker image includes every Tesseract language pack, so no extra installation is needed. Set the default with `OCR_LANGUAGE` (for example `eng+deu`); each user can choose their own languages under **Settings** → **OCR**.

### Document Sources

Under **Sources** in the web interface, add WebDAV servers (such as Nextcloud or ownCloud), S3 buckets, or folders on the server. Folders must be mounted into the container, and unless `LOCAL_SOURCE_ALLOWED_PATHS` is set only administrators can add them. See the [Sources Guide](../sources-guide.md).

### Users

Administrators create users and set their role (user or admin) under **Settings** → **Users**. See the [User Management Guide](../user-management-guide.md). For single sign-on, see the [OIDC Setup Guide](../oidc-setup.md).

## Test Upload via the API

```bash
# Log in and keep the session token
token=$(curl -s -X POST http://localhost:8000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"your-admin-password"}' | jq -r .token)

# Upload a PDF; it is queued for OCR
curl -X POST http://localhost:8000/api/documents \
  -H "Authorization: Bearer $token" \
  -F "file=@sample.pdf"

# List your documents with their OCR status
curl -H "Authorization: Bearer $token" http://localhost:8000/api/documents
```

See the [API Reference](../api-reference.md) for the response formats.

## Troubleshooting

### Port Already in Use

Map a different host port in `docker-compose.yml`, keeping the container port `8000`:

```yaml
services:
  readur:
    ports:
      - "8080:8000"
```

### Database Connection Failed

```bash
docker compose logs postgres
docker compose logs readur
```

The compose file waits for PostgreSQL's health check before starting Readur. If the database credentials were changed after the `postgres_data` volume was first created, the old credentials still apply; either use them or recreate the volume with `docker compose down -v` (this deletes all data).

### OCR Is Not Progressing

An administrator can check the queue:

```bash
curl -H "Authorization: Bearer $token" http://localhost:8000/api/queue/stats
```

Large documents may need a higher `OCR_TIMEOUT_SECONDS`. The **Intake** page lists documents that need attention.

### More Detailed Logs

Set `RUST_LOG=debug` in the `environment:` section and restart:

```bash
docker compose up -d
docker compose logs -f readur
```

### Getting Help

- [Troubleshooting Guide](../troubleshooting.md)
- [GitHub Issues](https://github.com/readur/readur/issues)

## Next Steps

- [Configuration Guide](configuration.md) and [Configuration Reference](../configuration-reference.md)
- [User Guide](../user-guide.md)
- [Deployment Guide](../deployment.md): reverse proxy, TLS and backups
- [Sources Guide](../sources-guide.md)
- [API Reference](../api-reference.md)
