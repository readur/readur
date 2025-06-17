#!/bin/bash

# Test runner script for Readur
# This script orchestrates all tests in an isolated environment

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
COMPOSE_FILE="docker-compose.test.yml"
COMPOSE_PROJECT_NAME="readur_test"
TEST_TIMEOUT=600  # 10 minutes timeout for all tests

# Function to print colored output
print_status() {
    echo -e "${BLUE}[TEST]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[✓]${NC} $1"
}

print_error() {
    echo -e "${RED}[✗]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[!]${NC} $1"
}

# Function to cleanup test environment
cleanup() {
    print_status "Cleaning up test environment..."
    docker compose -f $COMPOSE_FILE -p $COMPOSE_PROJECT_NAME down -v --remove-orphans 2>/dev/null || true
    
    # Force remove any lingering containers with our test names
    docker rm -f readur_postgres_test readur_app_test readur_frontend_test 2>/dev/null || true
    
    # Force remove the test network if it exists
    docker network rm readur_test_network 2>/dev/null || true
    
    # Remove any test artifacts
    rm -rf /tmp/test_uploads /tmp/test_watch 2>/dev/null || true
}

# Function to wait for service to be healthy
wait_for_service() {
    local service=$1
    local max_attempts=30
    local attempt=0
    
    print_status "Waiting for $service to be healthy..."
    
    while [ $attempt -lt $max_attempts ]; do
        if docker compose -f $COMPOSE_FILE -p $COMPOSE_PROJECT_NAME ps | grep -q "${service}.*healthy"; then
            print_success "$service is healthy"
            return 0
        fi
        
        attempt=$((attempt + 1))
        sleep 2
    done
    
    print_error "$service failed to become healthy after $max_attempts attempts"
    return 1
}

# Parse command line arguments
TEST_TYPE="${1:-all}"
KEEP_RUNNING="${2:-false}"

# Trap to ensure cleanup on exit
trap cleanup EXIT INT TERM

# Main test execution
main() {
    print_status "Starting Readur test suite (type: $TEST_TYPE)"
    
    # Force cleanup any existing test environment to avoid conflicts
    print_status "Ensuring clean test environment..."
    cleanup
    
    # Extra cleanup for stubborn resources
    print_status "Removing any conflicting resources..."
    docker ps -a | grep -E "readur_(postgres|app|frontend)_test" | awk '{print $1}' | xargs -r docker rm -f 2>/dev/null || true
    docker network ls | grep "readur_test_network" | awk '{print $1}' | xargs -r docker network rm 2>/dev/null || true
    
    # Load test environment variables
    if [ -f .env.test ]; then
        print_status "Loading test environment variables..."
        export $(grep -v '^#' .env.test | xargs)
    fi
    
    # Build test images
    print_status "Building test images..."
    docker compose -f $COMPOSE_FILE -p $COMPOSE_PROJECT_NAME build
    
    # Start test infrastructure
    print_status "Starting test infrastructure..."
    docker compose -f $COMPOSE_FILE -p $COMPOSE_PROJECT_NAME up -d postgres_test
    
    # Wait for PostgreSQL to be ready
    wait_for_service "postgres_test"
    
    # The application runs SQLx migrations automatically at startup
    print_status "Application will run database migrations on startup..."
    
    # Start the application
    print_status "Starting Readur test instance..."
    docker compose -f $COMPOSE_FILE -p $COMPOSE_PROJECT_NAME up -d readur_test
    
    # Wait for application to be ready
    wait_for_service "readur_test"
    
    # Execute tests based on type
    case $TEST_TYPE in
        unit)
            run_unit_tests
            ;;
        integration)
            run_integration_tests
            ;;
        frontend)
            run_frontend_tests
            ;;
        e2e)
            run_e2e_tests
            ;;
        all)
            run_unit_tests
            run_integration_tests
            run_frontend_tests
            ;;
        *)
            print_error "Invalid test type: $TEST_TYPE"
            echo "Usage: $0 [unit|integration|frontend|e2e|all] [keep-running]"
            exit 1
            ;;
    esac
    
    # Keep containers running if requested (useful for debugging)
    if [ "$KEEP_RUNNING" = "keep-running" ]; then
        print_warning "Keeping test containers running. Press Ctrl+C to stop and cleanup."
        print_status "Test services:"
        echo "  - PostgreSQL: localhost:5433"
        echo "  - Readur API: http://localhost:8001"
        echo "  - Logs: docker compose -f $COMPOSE_FILE -p $COMPOSE_PROJECT_NAME logs -f"
        
        # Wait for user to press Ctrl+C
        read -r -d '' _ </dev/tty
    fi
}

# Function to run unit tests
run_unit_tests() {
    print_status "Running unit tests..."
    
    # Run tests locally with test database URL
    if DATABASE_URL="postgresql://readur_test:readur_test@localhost:5433/readur_test" \
        cargo test --lib --no-fail-fast; then
        print_success "Unit tests passed"
    else
        print_error "Unit tests failed"
        exit 1
    fi
}

# Function to run integration tests
run_integration_tests() {
    print_status "Running integration tests..."
    
    # Run integration tests locally with test database URL and API URL
    if DATABASE_URL="postgresql://readur_test:readur_test@localhost:5433/readur_test" \
        TEST_DATABASE_URL="postgresql://readur_test:readur_test@localhost:5433/readur_test" \
        API_URL="http://localhost:8001" \
        cargo test --test '*' --no-fail-fast; then
        print_success "Integration tests passed"
    else
        print_error "Integration tests failed"
        exit 1
    fi
}

# Function to run frontend tests
run_frontend_tests() {
    print_status "Running frontend tests..."
    
    # Run frontend tests in a separate container
    if docker compose -f $COMPOSE_FILE -p $COMPOSE_PROJECT_NAME \
        --profile frontend-tests run --rm frontend_test; then
        print_success "Frontend tests passed"
    else
        print_error "Frontend tests failed"
        exit 1
    fi
}

# Function to run E2E tests (placeholder for future implementation)
run_e2e_tests() {
    print_warning "E2E tests not yet implemented"
    # TODO: Add E2E test implementation using Playwright or Cypress
}

# Function to show test results summary
show_summary() {
    print_status "Test Summary:"
    docker compose -f $COMPOSE_FILE -p $COMPOSE_PROJECT_NAME logs readur_test | \
        grep -E "(test result:|passed|failed)" | tail -20
}

# Run main function
main

# Show summary
show_summary

print_success "All tests completed successfully!"