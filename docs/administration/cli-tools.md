# Command-Line Tools Reference

## Overview

Readur has two kinds of command-line tools:

- **The `readur` binary**, which is the server. Besides starting the server it has two maintenance subcommands. This is the only binary in the Docker image (`/app/readur`).
- **Developer and admin tools** in `src/bin/`. They are not part of the Docker image; build and run them from a source checkout with Cargo.

**Available CLI Tools:**

| Command | Purpose |
|---------|---------|
| `readur serve` | Start the web server (the default when no subcommand is given) |
| `readur reset-admin-password` | Reset the admin user's password |
| `readur rotate-jwt-secret` | Replace the JWT signing key stored in the database |
| `migrate_to_s3` | Move documents stored on the local filesystem to S3 |
| `enqueue_pending_ocr` | Add documents with pending OCR status to the OCR queue |
| `batch_ingest` | Import every file in a directory for one user |
| `debug_pdf_extraction` | Compare text extraction methods on one PDF |
| `test_metadata` | Print the metadata extracted from a few test files |
| `analyze-webdav-performance` | Turn a WebDAV stress test metrics file into a report |
| `test_runner` | Run the unit, integration and frontend test suites |

## readur

```text
Readur document management system

Usage: readur [COMMAND]

Commands:
  serve                 Start the Readur web server (default)
  reset-admin-password  Reset the admin user's password
  rotate-jwt-secret     Replace the JWT signing key stored in the database (signs out every session)
  help                  Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

All settings come from environment variables (see the [configuration reference](../configuration-reference.md)); there are no command-line options for settings. Every subcommand reads the database settings (`DATABASE_URL` or the `POSTGRES_*` variables).

In the Docker image the working directory is `/app` and `/app` is not on `PATH`, so call the binary as `/app/readur` inside a container.

## reset-admin-password

**Purpose:** Reset the admin user's password when locked out or for credential rotation.

This is a subcommand of the main `readur` binary, not a separate tool.

### Usage
```bash
readur reset-admin-password
```

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `ADMIN_USERNAME` | Username of the account to reset | `admin` |
| `ADMIN_PASSWORD` | New password to set (at least 8 characters; shorter values are rejected) | A random 24-character password, printed to the terminal |

The account must already exist; otherwise the command fails with `Admin user '<name>' not found`. An empty `ADMIN_PASSWORD` counts as unset.

In a container, the variables set in the container's environment apply. The repository's `docker-compose.yml` passes `ADMIN_USERNAME` and `ADMIN_PASSWORD` from your `.env` file; if `ADMIN_PASSWORD` is set there, that value is used instead of a random one.

### Examples

#### Docker Compose
```bash
# Random password (when ADMIN_PASSWORD is empty in the container environment)
docker compose exec readur /app/readur reset-admin-password

# Specific password
docker compose exec -e ADMIN_PASSWORD="a-new-long-password" readur /app/readur reset-admin-password

# Different account
docker compose exec -e ADMIN_USERNAME="alice" readur /app/readur reset-admin-password
```

#### Binary or source deployments
```bash
readur reset-admin-password

ADMIN_PASSWORD="a-new-long-password" readur reset-admin-password

# From a source checkout
cargo run --release --bin readur -- reset-admin-password
```

#### Kubernetes
```bash
# The Deployment is named after the Helm release
kubectl exec deploy/readur -- /app/readur reset-admin-password

kubectl exec deploy/readur -- env ADMIN_PASSWORD="a-new-long-password" /app/readur reset-admin-password
```

### Example Output
```text
🔑 RESET ADMIN PASSWORD
============================================================

==============================================
  ADMIN PASSWORD RESET SUCCESSFUL
==============================================

Username: admin
Password: xK9mP2nQ7rT4wY6zA3cF8gH1

⚠️  SAVE THESE CREDENTIALS IMMEDIATELY!
⚠️  This password will not be shown again.

==============================================
```

The configuration loader prints its own lines between the header and the result box.

### Security Notes

- Pass the password through `ADMIN_PASSWORD`, never as a command-line argument (there is none).
- The password is printed to the terminal, not written to the logs.
- Requires database connectivity.

## rotate-jwt-secret

**Purpose:** Replace the JWT signing key that Readur generated and stored in the database.

### Usage
```bash
readur rotate-jwt-secret

# In a container
docker compose exec readur /app/readur rotate-jwt-secret
```

The command stores a new random key in the database and prints:

```text
✅ A new JWT signing key was stored in the database.
   Restart every running Readur server to load it. All existing sessions
   are then invalid and users have to sign in again.
```

Running servers keep using the old key until they are restarted. When `JWT_SECRET` is set (non-empty), it takes precedence over the stored key; the command then warns that the stored key is not used and that you have to change `JWT_SECRET` and restart instead. It fails if the server has never been started against the database (migrations not applied yet).

## migrate_to_s3

**Purpose:** Copy documents stored on the local filesystem into the S3 bucket and update their database records. The procedure is described in the [Storage Migration Guide](./storage-migration.md).

The tool is not in the Docker image. Run it from a source checkout with the same environment as the server (`DATABASE_URL` or `POSTGRES_*`, `UPLOAD_PATH`, and the `S3_*` settings with `S3_ENABLED=true` or `STORAGE_BACKEND=s3`), on a machine where the document files are readable at the paths stored in the database.

### Usage
```bash
cargo run --release --bin migrate_to_s3 -- [OPTIONS]

# or, after cargo build --release --bin migrate_to_s3
./target/release/migrate_to_s3 [OPTIONS]
```

### Command Options

| Option | Description |
|--------|-------------|
| `-d`, `--dry-run` | Log the documents that would be migrated and exit without changing anything. Still requires working database and S3 connections |
| `--delete-local` | Delete the local file, thumbnail and processed image of each document after uploading it |
| `-l`, `--limit <N>` | Without `--user-id`: stop collecting documents once N are gathered. With `--user-id`: the number of that user's newest documents to consider (default 1000) |
| `-u`, `--user-id <ID>` | Only migrate documents owned by this user (UUID) |
| `--enable-rollback` | On the first failed document, undo every migration of this run (restore database paths, delete the uploaded S3 objects) and exit with status 1 |
| `--resume-from <ID>` | Skip the selected documents before this document ID. Starts from the beginning if the ID is not in the selection |
| `-h`, `--help` | Print help |

Without `--user-id`, the tool considers the newest 500 documents of each user. Documents whose path already starts with `s3://` are skipped, so running the tool again retries only what is left.

### Behaviour

- Exits with status 1 if S3 is not enabled, the S3 bucket cannot be listed, or the database is unreachable.
- Without `--enable-rollback`, a failed document is recorded and the run continues; the exit status is 0.
- Writes `migration_state.json` to the working directory every 10 migrated documents and at the end, and `rollback_errors.json` if a rollback had errors. The tool never reads these files back.
- Rollback does not restore local files deleted by `--delete-local`.
- The log level is fixed at `info`; `RUST_LOG` has no effect on this tool.

### Examples

```bash
# Preview
cargo run --release --bin migrate_to_s3 -- --dry-run

# Migrate everything, undo this run if any document fails
cargo run --release --bin migrate_to_s3 -- --enable-rollback

# One user, up to 5000 documents
cargo run --release --bin migrate_to_s3 -- --user-id 3f6c1e2a-8d4b-4c1e-9a77-2b1f0c9d5e10 --limit 5000

# Migrate and delete the local copies
cargo run --release --bin migrate_to_s3 -- --delete-local
```

## enqueue_pending_ocr

**Purpose:** Add documents whose OCR status is `pending` but which are not in the OCR queue.

### Usage
```bash
RUST_LOG=info cargo run --release --bin enqueue_pending_ocr
```

The tool takes no arguments. It reads the server's database and storage settings, then selects documents with `ocr_status = 'pending'`, a file path, no OCR queue entry, and a MIME type of `image/*`, `application/pdf` or `text/plain`. Each is queued with a priority based on its size (10 for up to 1 MB, 8 up to 5 MB, 6 up to 10 MB, 4 up to 50 MB, 2 above). Afterwards it logs the queue statistics (pending, processing, failed, completed today). A running Readur server picks the queued documents up.

Without `RUST_LOG`, only errors are printed; set `RUST_LOG=info` to see what it does.

## batch_ingest

**Purpose:** Import every file in a directory as documents of one user and queue them for OCR.

### Usage
```bash
cargo run --release --bin batch_ingest -- <DIRECTORY> --user-id <UUID> [--monitor]
```

| Argument | Description |
|----------|-------------|
| `<DIRECTORY>` | Directory to ingest files from (required) |
| `-u`, `--user-id <UUID>` | User the documents are assigned to (required) |
| `-m`, `--monitor` | After ingestion, monitor the OCR queue progress |

Uses the server's database and storage settings from the environment.

## debug_pdf_extraction

**Purpose:** Run each PDF text extraction method on one file and show what Readur would pick.

### Usage
```bash
cargo run --release --bin debug_pdf_extraction -- <pdf_file_path>
```

It runs `pdftotext -layout`, `ocrmypdf --sidecar` and direct extraction on the file, prints the word count and the first characters of each result, and ends with an `Expected result:` line. `pdftotext` and `ocrmypdf` must be installed.

## test_metadata

**Purpose:** Developer check of metadata extraction.

```bash
cargo run --bin test_metadata
```

It takes no arguments. It reads `test_files/portrait_100x200.png`, `test_files/single_page_v14.pdf`, `test_files/comprehensive_text.txt` and `test_files/test_format.json` relative to the working directory (run it from the repository root), skips files that are missing, and prints the extracted metadata as JSON.

## analyze-webdav-performance

**Purpose:** Analyze a WebDAV stress test metrics file (used in CI).

```bash
cargo run --bin analyze-webdav-performance -- --metrics-file <FILE> [--output-format <FORMAT>] [--output-file <FILE>]
```

| Option | Description |
|--------|-------------|
| `--metrics-file <FILE>` | Stress test metrics JSON file (required) |
| `--output-format <FORMAT>` | `json`, `markdown` (default) or `github-summary` |
| `--output-file <FILE>` | Write the report to this file instead of stdout |

Exit status: 0 for a healthy or warning result, 1 for a critical result, 2 if the health is unknown.

## test_runner

**Purpose:** An old wrapper that runs test suites from the repository root. It is out of date: the `unit` and `integration` suites it runs are Cargo test targets named `unit_tests` and `integration_tests`, which no longer exist, so those suites fail. Run the tests as described in the [Testing Guide](../dev/TESTING.md) instead.

### Usage
```bash
cargo run --bin test_runner -- [TEST_TYPE]
```

| TEST_TYPE | Runs |
|-----------|------|
| `unit` | the missing `unit_tests` target |
| `integration` | the missing `integration_tests` target, only when a server answers `http://localhost:8000/api/health` |
| `frontend` | `npm test -- --run` in `frontend/` |
| `all` (default) | Unit and frontend tests, plus integration tests if the server is running |

It prints a summary and exits with status 1 if any suite failed. Other arguments are rejected with a usage message.

## Environment Variables

The tools read the same environment variables as the server; the full list is in the [configuration reference](../configuration-reference.md). The ones most relevant here:

| Variable | Purpose |
|----------|---------|
| `DATABASE_URL` | PostgreSQL connection string (or use the `POSTGRES_*` variables) |
| `UPLOAD_PATH` | Local storage directory |
| `S3_ENABLED` | `true` to enable S3 storage (required by `migrate_to_s3`) |
| `S3_BUCKET_NAME` | Bucket name |
| `S3_ACCESS_KEY_ID` | Access key ID |
| `S3_SECRET_ACCESS_KEY` | Secret access key |
| `S3_REGION` | Region (default `us-east-1`) |
| `S3_ENDPOINT_URL` | Custom endpoint for S3-compatible services, for example `https://s3.example.com` (alias: `S3_ENDPOINT`) |
| `ADMIN_USERNAME` | Account reset by `reset-admin-password` |
| `ADMIN_PASSWORD` | Password set by `reset-admin-password` |
| `RUST_LOG` | Log filter (not used by `migrate_to_s3`) |
| `RUST_BACKTRACE` | `1` or `full` to print backtraces on errors |

**Security Warning:** Do not put secrets such as `S3_SECRET_ACCESS_KEY` directly on a shared command line, where they can show up in shell history and process lists. Export them in the environment or load them from a file.

## Troubleshooting

### Tool not found in the container
Only `/app/readur` is in the Docker image. Build the other tools from a source checkout (see each section).

### Database connection errors
Check that `DATABASE_URL` (or the `POSTGRES_*` variables) points at the same database as the server and that the database is reachable from where you run the tool.

### Getting help
```bash
docker compose exec readur /app/readur --help
cargo run --release --bin migrate_to_s3 -- --help
cargo run --release --bin batch_ingest -- --help
```

`enqueue_pending_ocr`, `debug_pdf_extraction`, `test_metadata` and `test_runner` have no `--help` option.

## Security Considerations

- Run these tools only as an administrator with access to the database credentials.
- `migrate_to_s3` state files contain document IDs, user IDs and file paths.
- Back up the database and uploads before running tools that change data (`migrate_to_s3`, `batch_ingest`, `enqueue_pending_ocr`).
