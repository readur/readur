#!/bin/bash

# Local E2E Test Runner for Readur
# This script sets up and runs E2E tests locally

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
DB_NAME="readur_e2e_test"
DB_USER="postgres"
DB_PASSWORD="postgres"
DB_HOST="localhost"
DB_PORT="5432"
BACKEND_PORT="8001"
FRONTEND_PORT="5174"

# Function to print colored output
print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to check if port is in use
port_in_use() {
    lsof -i :"$1" >/dev/null 2>&1
}

# Function to wait for service
wait_for_service() {
    local url="$1"
    local timeout="${2:-60}"
    local counter=0
    
    print_status "Waiting for service at $url..."
    
    while [ $counter -lt $timeout ]; do
        if curl -f "$url" >/dev/null 2>&1; then
            print_status "Service is ready!"
            return 0
        fi
        sleep 2
        ((counter += 2))
    done
    
    print_error "Service at $url did not become ready within $timeout seconds"
    return 1
}

# Function to cleanup on exit
cleanup() {
    print_status "Cleaning up..."
    
    # Kill background processes
    if [ ! -z "$BACKEND_PID" ]; then
        kill $BACKEND_PID 2>/dev/null || true
    fi
    
    if [ ! -z "$FRONTEND_PID" ]; then
        kill $FRONTEND_PID 2>/dev/null || true
    fi
    
    # Drop test database
    PGPASSWORD=$DB_PASSWORD dropdb -h $DB_HOST -U $DB_USER $DB_NAME 2>/dev/null || true
    
    print_status "Cleanup complete"
}

# Set up trap to cleanup on exit
trap cleanup EXIT

# Main execution
main() {
    print_status "Starting Readur E2E Test Setup"
    
    # Check prerequisites
    print_status "Checking prerequisites..."
    
    if ! command_exists cargo; then
        print_error "Rust/Cargo not found. Please install Rust."
        exit 1
    fi
    
    if ! command_exists npm; then
        print_error "npm not found. Please install Node.js and npm."
        exit 1
    fi
    
    if ! command_exists psql; then
        print_error "PostgreSQL client not found. Please install PostgreSQL."
        exit 1
    fi
    
    # Check if ports are available
    if port_in_use $BACKEND_PORT; then
        print_error "Port $BACKEND_PORT is already in use. Please free it or change BACKEND_PORT in this script."
        exit 1
    fi
    
    if port_in_use $FRONTEND_PORT; then
        print_error "Port $FRONTEND_PORT is already in use. Please free it or change FRONTEND_PORT in this script."
        exit 1
    fi
    
    # Set up test database
    print_status "Setting up test database..."
    
    # Drop existing test database if it exists
    PGPASSWORD=$DB_PASSWORD dropdb -h $DB_HOST -U $DB_USER $DB_NAME 2>/dev/null || true
    
    # Create test database
    PGPASSWORD=$DB_PASSWORD createdb -h $DB_HOST -U $DB_USER $DB_NAME
    
    # Add vector extension if available
    PGPASSWORD=$DB_PASSWORD psql -h $DB_HOST -U $DB_USER -d $DB_NAME -c "CREATE EXTENSION IF NOT EXISTS vector;" 2>/dev/null || print_warning "Vector extension not available"
    
    # Run migrations
    print_status "Running database migrations..."
    export DATABASE_URL="postgresql://$DB_USER:$DB_PASSWORD@$DB_HOST:$DB_PORT/$DB_NAME"
    export TEST_MODE=true
    
    cargo run --bin migrate || {
        print_error "Failed to run migrations"
        exit 1
    }
    
    # Build backend
    print_status "Building backend..."
    cargo build --release
    
    # Start backend server
    print_status "Starting backend server on port $BACKEND_PORT..."
    DATABASE_URL="postgresql://$DB_USER:$DB_PASSWORD@$DB_HOST:$DB_PORT/$DB_NAME" \
    TEST_MODE=true \
    ROCKET_PORT=$BACKEND_PORT \
    ./target/release/readur > backend.log 2>&1 &
    BACKEND_PID=$!
    
    # Wait for backend to be ready
    wait_for_service "http://localhost:$BACKEND_PORT/health" || {
        print_error "Backend failed to start. Check backend.log for details."
        exit 1
    }
    
    # Install frontend dependencies
    print_status "Installing frontend dependencies..."
    cd frontend
    npm install
    
    # Install Playwright browsers
    print_status "Installing Playwright browsers..."
    npx playwright install
    
    # Start frontend dev server
    print_status "Starting frontend dev server on port $FRONTEND_PORT..."
    VITE_API_BASE_URL="http://localhost:$BACKEND_PORT" \
    npm run dev -- --port $FRONTEND_PORT > ../frontend.log 2>&1 &
    FRONTEND_PID=$!
    
    # Wait for frontend to be ready
    wait_for_service "http://localhost:$FRONTEND_PORT" || {
        print_error "Frontend failed to start. Check frontend.log for details."
        exit 1
    }
    
    # Run E2E tests
    print_status "Running E2E tests..."
    
    # Update Playwright config for local testing
    export PLAYWRIGHT_BASE_URL="http://localhost:$FRONTEND_PORT"
    
    if [ "$1" = "--headed" ]; then
        npm run test:e2e:headed
    elif [ "$1" = "--debug" ]; then
        npm run test:e2e:debug
    elif [ "$1" = "--ui" ]; then
        npm run test:e2e:ui
    else
        npm run test:e2e
    fi
    
    print_status "E2E tests completed!"
}

# Parse command line arguments
case "$1" in
    --help|-h)
        echo "Usage: $0 [--headed|--debug|--ui|--help]"
        echo ""
        echo "Options:"
        echo "  --headed    Run tests in headed mode (show browser)"
        echo "  --debug     Run tests in debug mode"
        echo "  --ui        Run tests with Playwright UI"
        echo "  --help      Show this help message"
        exit 0
        ;;
esac

# Run main function
main "$@"