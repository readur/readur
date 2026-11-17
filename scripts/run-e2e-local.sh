#!/bin/bash

# Local E2E test runner for Readur.
#
# Creates a scratch database, builds and starts the backend (which runs its migrations on
# startup), starts the Vite dev server proxying /api to it, and runs the Playwright suite
# against the dev server. Everything it starts or creates is stopped and dropped on exit.
#
# Settings (environment variables, all optional):
#   E2E_DB_HOST, E2E_DB_PORT, E2E_DB_USER, E2E_DB_PASSWORD
#                         PostgreSQL server to create the scratch database on
#                         (default postgres:postgres@localhost:5432)
#   E2E_DB_NAME           scratch database name (default readur_e2e_test); dropped and recreated
#   E2E_BACKEND_PORT      backend port (default 8001)
#   E2E_FRONTEND_PORT     Vite dev server port (default 5174)
#   E2E_CARGO_PROFILE     cargo profile to build the backend with (default release; `dev`
#                         reuses an existing debug build)
#   E2E_ADMIN_USERNAME, E2E_ADMIN_PASSWORD
#                         admin the backend seeds and the suite provisions test users through
#   CARGO                 cargo command (default cargo)
#   CARGO_TARGET_DIR      honoured when locating the built binary
#   PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH
#                         use a system Chromium instead of downloading one (NixOS, for example)
#
# Usage: scripts/run-e2e-local.sh [--headed|--debug|--ui] [-- <playwright args>]
# Without playwright args the two Chromium projects run, as in CI (.github/workflows/test-e2e.yml).

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

DB_HOST="${E2E_DB_HOST:-localhost}"
DB_PORT="${E2E_DB_PORT:-5432}"
DB_USER="${E2E_DB_USER:-postgres}"
DB_NAME="${E2E_DB_NAME:-readur_e2e_test}"
BACKEND_PORT="${E2E_BACKEND_PORT:-8001}"
FRONTEND_PORT="${E2E_FRONTEND_PORT:-5174}"
CARGO_PROFILE="${E2E_CARGO_PROFILE:-release}"
CARGO_CMD="${CARGO:-cargo}"
ADMIN_USER="${E2E_ADMIN_USERNAME:-admin}"
ADMIN_PASS="${E2E_ADMIN_PASSWORD:-readur-e2e-admin-password}"
LOG_DIR="$REPO_ROOT/frontend/test-results/e2e-local"

# libpq (psql, createdb, dropdb) reads the password from here.
export PGPASSWORD="${E2E_DB_PASSWORD:-postgres}"

BACKEND_PID=""
FRONTEND_PID=""
RUNTIME_DIR=""
DB_CREATED=false

print_status() { echo -e "${GREEN}[INFO]${NC} $1"; }
print_warning() { echo -e "${YELLOW}[WARN]${NC} $1"; }
print_error() { echo -e "${RED}[ERROR]${NC} $1" >&2; }

command_exists() { command -v "$1" >/dev/null 2>&1; }

# True when something accepts TCP connections on the port (needs no lsof/ss).
port_in_use() {
    (exec 3<>"/dev/tcp/127.0.0.1/$1") 2>/dev/null
}

# Wait until `url` answers, failing early if the process `pid` exits.
wait_for_service() {
    local url="$1" pid="$2" timeout="${3:-120}" waited=0
    print_status "Waiting for $url..."
    while [ "$waited" -lt "$timeout" ]; do
        if curl -fsS "$url" >/dev/null 2>&1; then
            print_status "$url is ready"
            return 0
        fi
        if ! kill -0 "$pid" 2>/dev/null; then
            print_error "The process serving $url exited"
            return 1
        fi
        sleep 2
        waited=$((waited + 2))
    done
    print_error "$url did not become ready within $timeout seconds"
    return 1
}

# Stop a process started with setsid, together with its children (npm runs vite as a child).
stop_group() {
    local pid="$1"
    [ -n "$pid" ] || return 0
    kill -TERM -- "-$pid" 2>/dev/null || kill -TERM "$pid" 2>/dev/null || true
    for _ in 1 2 3 4 5; do
        kill -0 "$pid" 2>/dev/null || return 0
        sleep 1
    done
    kill -KILL -- "-$pid" 2>/dev/null || true
}

cleanup() {
    print_status "Cleaning up..."
    stop_group "$FRONTEND_PID"
    stop_group "$BACKEND_PID"
    if [ "$DB_CREATED" = true ]; then
        dropdb --if-exists -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" "$DB_NAME" 2>/dev/null ||
            print_warning "Could not drop database $DB_NAME"
    fi
    if [ -n "$RUNTIME_DIR" ]; then
        rm -rf "$RUNTIME_DIR"
    fi
    print_status "Cleanup complete (logs: $LOG_DIR)"
}

usage() {
    echo "Usage: $0 [--headed|--debug|--ui] [-- <playwright args>]"
    echo ""
    echo "Options:"
    echo "  --headed    Run tests in headed mode (show browser)"
    echo "  --debug     Run tests in debug mode"
    echo "  --ui        Run tests with Playwright UI"
    echo "  --help      Show this help message"
    echo ""
    echo "Arguments after -- go to Playwright, e.g. -- e2e/auth.spec.ts --project=chromium"
}

# The cargo output directory for CARGO_PROFILE (cargo names the dev profile's directory debug).
binary_path() {
    local target_dir="${CARGO_TARGET_DIR:-$REPO_ROOT/target}"
    local profile_dir="$CARGO_PROFILE"
    [ "$profile_dir" = dev ] && profile_dir=debug
    echo "$target_dir/$profile_dir/readur"
}

check_prerequisites() {
    print_status "Checking prerequisites..."
    local tool
    for tool in "${CARGO_CMD%% *}" npm npx psql createdb dropdb curl setsid; do
        if ! command_exists "$tool"; then
            print_error "$tool not found"
            exit 1
        fi
    done
    local port
    for port in "$BACKEND_PORT" "$FRONTEND_PORT"; do
        if port_in_use "$port"; then
            print_error "Port $port is already in use; free it or set E2E_BACKEND_PORT / E2E_FRONTEND_PORT"
            exit 1
        fi
    done
}

setup_database() {
    print_status "Creating database $DB_NAME on $DB_HOST:$DB_PORT..."
    dropdb --if-exists -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" "$DB_NAME"
    createdb -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" "$DB_NAME"
    DB_CREATED=true
}

start_backend() {
    print_status "Building backend (profile $CARGO_PROFILE)..."
    # shellcheck disable=SC2086 # CARGO may carry a wrapper and its arguments
    $CARGO_CMD build --profile "$CARGO_PROFILE" --bin readur
    local binary
    binary="$(binary_path)"
    if [ ! -x "$binary" ]; then
        print_error "Built binary not found at $binary"
        exit 1
    fi

    # Uploads and the watch folder live in a throwaway directory, not in the checkout.
    RUNTIME_DIR="$(mktemp -d -t readur-e2e.XXXXXX)"
    mkdir -p "$RUNTIME_DIR/uploads" "$RUNTIME_DIR/watch"

    # The server applies its migrations on startup (sqlx::migrate! in src/main.rs).
    print_status "Starting backend on port $BACKEND_PORT..."
    DATABASE_URL="postgresql://$DB_USER:$PGPASSWORD@$DB_HOST:$DB_PORT/$DB_NAME" \
        SERVER_ADDRESS="127.0.0.1:$BACKEND_PORT" \
        ADMIN_USERNAME="$ADMIN_USER" \
        ADMIN_PASSWORD="$ADMIN_PASS" \
        UPLOAD_PATH="$RUNTIME_DIR/uploads" \
        WATCH_FOLDER="$RUNTIME_DIR/watch" \
        setsid "$binary" >"$LOG_DIR/backend.log" 2>&1 &
    BACKEND_PID=$!

    if ! wait_for_service "http://127.0.0.1:$BACKEND_PORT/api/health" "$BACKEND_PID" 300; then
        print_error "Backend failed to start; see $LOG_DIR/backend.log"
        tail -n 40 "$LOG_DIR/backend.log" >&2 || true
        exit 1
    fi
}

start_frontend() {
    cd "$REPO_ROOT/frontend"
    if [ ! -d node_modules ]; then
        print_status "Installing frontend dependencies..."
        npm ci
    fi

    if [ -n "${PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH:-}" ]; then
        print_status "Using Chromium at $PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH"
        # playwright.config.ts records video on failure, which needs Playwright's own ffmpeg.
        npx playwright install ffmpeg ||
            print_warning "Could not install Playwright's ffmpeg; pages fail to open without it (point PLAYWRIGHT_BROWSERS_PATH at a writable directory)"
    else
        print_status "Installing the Playwright Chromium browser..."
        npx playwright install chromium
    fi

    # vite.config.ts proxies /api (HTTP and WebSocket) to VITE_API_PROXY_TARGET.
    print_status "Starting frontend dev server on port $FRONTEND_PORT..."
    VITE_API_PROXY_TARGET="http://127.0.0.1:$BACKEND_PORT" \
        setsid npm run dev -- --port "$FRONTEND_PORT" --strictPort >"$LOG_DIR/frontend.log" 2>&1 &
    FRONTEND_PID=$!

    if ! wait_for_service "http://localhost:$FRONTEND_PORT" "$FRONTEND_PID" 120; then
        print_error "Frontend failed to start; see $LOG_DIR/frontend.log"
        tail -n 40 "$LOG_DIR/frontend.log" >&2 || true
        exit 1
    fi
    # The proxy must reach the backend, or every spec fails on its first API call.
    if ! curl -fsS "http://localhost:$FRONTEND_PORT/api/health" >/dev/null; then
        print_error "The dev server does not proxy /api to the backend; see $LOG_DIR/frontend.log"
        exit 1
    fi
}

main() {
    local mode=""
    local playwright_args=()
    while [ "$#" -gt 0 ]; do
        case "$1" in
            --help | -h)
                usage
                exit 0
                ;;
            --headed | --debug | --ui)
                mode="$1"
                ;;
            --)
                shift
                playwright_args=("$@")
                break
                ;;
            *)
                print_error "Unknown option: $1"
                usage
                exit 1
                ;;
        esac
        shift
    done
    if [ "${#playwright_args[@]}" -eq 0 ]; then
        playwright_args=(--project=chromium --project=chromium-websocket)
    fi

    print_status "Starting Readur E2E test setup"
    check_prerequisites
    mkdir -p "$LOG_DIR"
    trap cleanup EXIT
    trap 'exit 130' INT TERM

    cd "$REPO_ROOT"
    setup_database
    start_backend
    start_frontend

    print_status "Running E2E tests..."
    export PLAYWRIGHT_BASE_URL="http://localhost:$FRONTEND_PORT"
    export E2E_ADMIN_USERNAME="$ADMIN_USER"
    export E2E_ADMIN_PASSWORD="$ADMIN_PASS"
    case "$mode" in
        --headed) npm run test:e2e:headed -- "${playwright_args[@]}" ;;
        --debug) npm run test:e2e:debug -- "${playwright_args[@]}" ;;
        --ui) npm run test:e2e:ui -- "${playwright_args[@]}" ;;
        *) npm run test:e2e -- "${playwright_args[@]}" ;;
    esac

    print_status "E2E tests completed"
}

main "$@"
