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
TEST_RESULTS_DIR="test-results"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")

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

# Function to strip ANSI color codes
strip_ansi() {
    sed 's/\x1b\[[0-9;]*m//g'
}

# Function to save output to file
save_output() {
    local test_type=$1
    local output=$2
    local status=$3
    local output_file="${TEST_RESULTS_DIR}/${test_type}/${TIMESTAMP}_${test_type}_${status}.log"
    
    echo "$output" | strip_ansi > "$output_file"
    echo "Results saved to: $output_file"
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
    
    # Run tests locally with test database URL and capture output
    local output
    local exit_code
    
    output=$(DATABASE_URL="postgresql://readur_test:readur_test@localhost:5433/readur_test" \
        cargo test --lib --no-fail-fast 2>&1)
    exit_code=$?
    
    # Display output in terminal
    echo "$output"
    
    if [ $exit_code -eq 0 ]; then
        print_success "Unit tests passed"
        save_output "unit" "$output" "passed"
    else
        print_error "Unit tests failed"
        save_output "unit" "$output" "failed"
        exit 1
    fi
}

# Function to run integration tests
run_integration_tests() {
    print_status "Running integration tests..."
    
    # Run integration tests locally with test database URL and API URL
    local output
    local exit_code
    
    output=$(DATABASE_URL="postgresql://readur_test:readur_test@localhost:5433/readur_test" \
        TEST_DATABASE_URL="postgresql://readur_test:readur_test@localhost:5433/readur_test" \
        API_URL="http://localhost:8001" \
        cargo test --test '*' --no-fail-fast 2>&1)
    exit_code=$?
    
    # Display output in terminal
    echo "$output"
    
    if [ $exit_code -eq 0 ]; then
        print_success "Integration tests passed"
        save_output "integration" "$output" "passed"
    else
        print_error "Integration tests failed"
        save_output "integration" "$output" "failed"
        exit 1
    fi
}

# Function to run frontend tests
run_frontend_tests() {
    print_status "Running frontend tests..."
    
    # Run frontend tests in a separate container
    local output
    local exit_code
    
    output=$(docker compose -f $COMPOSE_FILE -p $COMPOSE_PROJECT_NAME \
        --profile frontend-tests run --rm frontend_test 2>&1)
    exit_code=$?
    
    # Display output in terminal
    echo "$output"
    
    if [ $exit_code -eq 0 ]; then
        print_success "Frontend tests passed"
        save_output "frontend" "$output" "passed"
    else
        print_error "Frontend tests failed"
        save_output "frontend" "$output" "failed"
        exit 1
    fi
}

# Function to run E2E tests (placeholder for future implementation)
run_e2e_tests() {
    print_warning "E2E tests not yet implemented"
    # TODO: Add E2E test implementation using Playwright or Cypress
}

# Function to generate detailed test report
generate_test_report() {
    local report_file="${TEST_RESULTS_DIR}/reports/${TIMESTAMP}_test_report.html"
    
    print_status "Generating test report..."
    
    cat > "$report_file" << 'EOF'
<!DOCTYPE html>
<html>
<head>
    <title>Readur Test Results</title>
    <meta charset="UTF-8">
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; background: #f5f5f5; }
        .container { max-width: 1200px; margin: 0 auto; background: white; padding: 20px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }
        .header { text-align: center; margin-bottom: 30px; }
        .timestamp { color: #666; font-size: 14px; }
        .summary { display: flex; gap: 20px; margin-bottom: 30px; }
        .summary-box { flex: 1; padding: 15px; border-radius: 4px; text-align: center; }
        .passed { background: #d4edda; border: 1px solid #c3e6cb; color: #155724; }
        .failed { background: #f8d7da; border: 1px solid #f5c6cb; color: #721c24; }
        .skipped { background: #fff3cd; border: 1px solid #ffeaa7; color: #856404; }
        .test-section { margin-bottom: 30px; }
        .test-title { font-size: 18px; font-weight: bold; margin-bottom: 10px; border-bottom: 2px solid #eee; padding-bottom: 5px; }
        .log-output { background: #f8f9fa; border: 1px solid #e9ecef; padding: 15px; border-radius: 4px; overflow-x: auto; }
        .log-output pre { margin: 0; font-family: 'Courier New', monospace; font-size: 12px; white-space: pre-wrap; }
        .status-passed { color: #28a745; }
        .status-failed { color: #dc3545; }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🧪 Readur Test Results</h1>
            <div class="timestamp">Test run: TIMESTAMP_PLACEHOLDER</div>
        </div>
EOF

    # Add timestamp
    sed -i "s/TIMESTAMP_PLACEHOLDER/$(date)/" "$report_file"
    
    # Add summary section
    local total_tests=0
    local passed_tests=0
    local failed_tests=0
    
    # Count test results from saved files
    if ls "${TEST_RESULTS_DIR}"/unit/*_passed.log > /dev/null 2>&1; then
        passed_tests=$((passed_tests + 1))
        total_tests=$((total_tests + 1))
    fi
    if ls "${TEST_RESULTS_DIR}"/unit/*_failed.log > /dev/null 2>&1; then
        failed_tests=$((failed_tests + 1))
        total_tests=$((total_tests + 1))
    fi
    if ls "${TEST_RESULTS_DIR}"/integration/*_passed.log > /dev/null 2>&1; then
        passed_tests=$((passed_tests + 1))
        total_tests=$((total_tests + 1))
    fi
    if ls "${TEST_RESULTS_DIR}"/integration/*_failed.log > /dev/null 2>&1; then
        failed_tests=$((failed_tests + 1))
        total_tests=$((total_tests + 1))
    fi
    if ls "${TEST_RESULTS_DIR}"/frontend/*_passed.log > /dev/null 2>&1; then
        passed_tests=$((passed_tests + 1))
        total_tests=$((total_tests + 1))
    fi
    if ls "${TEST_RESULTS_DIR}"/frontend/*_failed.log > /dev/null 2>&1; then
        failed_tests=$((failed_tests + 1))
        total_tests=$((total_tests + 1))
    fi
    
    cat >> "$report_file" << EOF
        <div class="summary">
            <div class="summary-box">
                <h3>Total Test Suites</h3>
                <div style="font-size: 24px; font-weight: bold;">$total_tests</div>
            </div>
            <div class="summary-box passed">
                <h3>Passed</h3>
                <div style="font-size: 24px; font-weight: bold;">$passed_tests</div>
            </div>
            <div class="summary-box failed">
                <h3>Failed</h3>
                <div style="font-size: 24px; font-weight: bold;">$failed_tests</div>
            </div>
        </div>
EOF

    # Add test results sections
    for test_type in unit integration frontend; do
        for log_file in "${TEST_RESULTS_DIR}/${test_type}"/${TIMESTAMP}_*.log; do
            if [ -f "$log_file" ]; then
                local basename=$(basename "$log_file")
                local status=""
                if [[ "$basename" == *"_passed.log" ]]; then
                    status="passed"
                elif [[ "$basename" == *"_failed.log" ]]; then
                    status="failed"
                fi
                
                cat >> "$report_file" << EOF
        <div class="test-section">
            <div class="test-title">
                $(echo "${test_type^}" | sed 's/_/ /g') Tests 
                <span class="status-$status">[$status]</span>
            </div>
            <div class="log-output">
                <pre>$(cat "$log_file")</pre>
            </div>
        </div>
EOF
            fi
        done
    done
    
    cat >> "$report_file" << 'EOF'
    </div>
</body>
</html>
EOF

    print_success "Test report generated: $report_file"
    
    # Also create a simple text summary
    local summary_file="${TEST_RESULTS_DIR}/reports/${TIMESTAMP}_summary.txt"
    cat > "$summary_file" << EOF
READUR TEST SUMMARY
==================
Test Run: $(date)
Total Test Suites: $total_tests
Passed: $passed_tests
Failed: $failed_tests

Individual Results:
EOF

    for test_type in unit integration frontend; do
        for log_file in "${TEST_RESULTS_DIR}/${test_type}"/${TIMESTAMP}_*.log; do
            if [ -f "$log_file" ]; then
                local basename=$(basename "$log_file")
                local status=""
                if [[ "$basename" == *"_passed.log" ]]; then
                    status="✓ PASSED"
                elif [[ "$basename" == *"_failed.log" ]]; then
                    status="✗ FAILED"
                fi
                echo "- ${test_type^} Tests: $status" >> "$summary_file"
            fi
        done
    done
    
    echo "" >> "$summary_file"
    echo "Detailed logs available in: ${TEST_RESULTS_DIR}/" >> "$summary_file"
    echo "HTML Report: $report_file" >> "$summary_file"
    
    print_success "Summary saved: $summary_file"
}

# Function to show test results summary
show_summary() {
    print_status "Test Summary:"
    
    # Show recent results
    if [ -f "${TEST_RESULTS_DIR}/reports/${TIMESTAMP}_summary.txt" ]; then
        cat "${TEST_RESULTS_DIR}/reports/${TIMESTAMP}_summary.txt"
    else
        echo "No summary file found"
    fi
}

# Run main function
main

# Generate test report
generate_test_report

# Show summary
show_summary

print_success "All tests completed successfully!"