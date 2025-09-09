#!/bin/bash

# WebDAV Stress Test Orchestrator
# Coordinates running comprehensive stress tests and collecting results

set -euo pipefail

# Configuration
STRESS_LEVEL="${STRESS_LEVEL:-medium}"
TEST_DURATION="${STRESS_TEST_DURATION:-300}"
WEBDAV_DUFS_URL="${WEBDAV_DUFS_URL:-http://dufs_webdav:8080}"
WEBDAV_USERNAME="${WEBDAV_USERNAME:-webdav_user}"
WEBDAV_PASSWORD="${WEBDAV_PASSWORD:-webdav_pass}"
LOOP_DETECTION_TIMEOUT="${LOOP_DETECTION_TIMEOUT:-60}"
RESULTS_DIR="/tmp/stress-results"

log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*" >&2
}

# Wait for WebDAV servers to be ready
wait_for_webdav_server() {
    local url="$1"
    local name="$2"
    local max_attempts=30
    local attempt=1
    
    log "Waiting for $name WebDAV server at $url..."
    
    while [ $attempt -le $max_attempts ]; do
        if curl -f -s --connect-timeout 5 --max-time 10 \
           --user "$WEBDAV_USERNAME:$WEBDAV_PASSWORD" \
           "$url/" > /dev/null 2>&1; then
            log "$name WebDAV server is ready"
            return 0
        fi
        
        log "Attempt $attempt/$max_attempts: $name server not ready yet..."
        sleep 5
        attempt=$((attempt + 1))
    done
    
    log "ERROR: $name WebDAV server did not become ready within timeout"
    return 1
}

# Setup test environment
setup_test_environment() {
    log "Setting up WebDAV stress test environment..."
    
    # Create results directory
    mkdir -p "$RESULTS_DIR/logs" "$RESULTS_DIR/reports"
    
    # Wait for WebDAV server
    wait_for_webdav_server "$WEBDAV_DUFS_URL" "Dufs"
    
    # Test WebDAV connectivity
    log "Testing WebDAV connectivity..."
    curl -f --user "$WEBDAV_USERNAME:$WEBDAV_PASSWORD" \
         -X PROPFIND -H "Depth: 0" \
         "$WEBDAV_DUFS_URL/" > /dev/null
    
    log "WebDAV connectivity test passed"
}

# Generate test data on WebDAV server
generate_webdav_test_data() {
    log "Generating test data on WebDAV server..."
    
    # Use a temporary directory to generate data, then upload to WebDAV
    local temp_dir="/tmp/webdav-test-data"
    rm -rf "$temp_dir"
    
    # Generate test data locally
    ./scripts/generate-webdav-test-data.sh \
        --webdav-root "$temp_dir" \
        --stress-level "$STRESS_LEVEL" \
        --include-git-repos \
        --include-symlinks \
        --include-large-directories \
        --include-unicode-names \
        --include-problematic-files \
        --verbose
    
    # Upload test data to WebDAV server using curl
    log "Uploading test data to WebDAV server..."
    upload_directory_to_webdav "$temp_dir" "$WEBDAV_DUFS_URL"
    
    # Cleanup local data
    rm -rf "$temp_dir"
    
    log "Test data generation and upload completed"
}

# Upload directory structure to WebDAV server
upload_directory_to_webdav() {
    local source_dir="$1"
    local webdav_base_url="$2"
    
    # Create directories first
    find "$source_dir" -type d | while read -r dir; do
        local rel_path="${dir#$source_dir}"
        if [ -n "$rel_path" ]; then
            local webdav_url="$webdav_base_url$rel_path"
            curl -f --user "$WEBDAV_USERNAME:$WEBDAV_PASSWORD" \
                 -X MKCOL "$webdav_url/" > /dev/null 2>&1 || true
        fi
    done
    
    # Upload files
    find "$source_dir" -type f | while read -r file; do
        local rel_path="${file#$source_dir}"
        local webdav_url="$webdav_base_url$rel_path"
        curl -f --user "$WEBDAV_USERNAME:$WEBDAV_PASSWORD" \
             -X PUT --data-binary "@$file" \
             "$webdav_url" > /dev/null 2>&1 || true
    done
}

# Run stress tests with monitoring
run_stress_tests() {
    log "Starting WebDAV stress tests..."
    
    # Set environment variables for tests
    export WEBDAV_DUFS_URL="$WEBDAV_DUFS_URL"
    export WEBDAV_SERVER_URL="$WEBDAV_DUFS_URL"
    export WEBDAV_USERNAME="$WEBDAV_USERNAME"
    export WEBDAV_PASSWORD="$WEBDAV_PASSWORD"
    export STRESS_LEVEL="$STRESS_LEVEL"
    export STRESS_TEST_DURATION="$TEST_DURATION"
    export TEST_TIMEOUT_SECONDS="$TEST_DURATION"
    export LOOP_DETECTION_TIMEOUT="$LOOP_DETECTION_TIMEOUT"
    export CONCURRENT_SYNCS="4"
    export TRIGGER_TEST_LOOPS="true"
    export STRESS_RESULTS_DIR="$RESULTS_DIR"
    export RUST_LOG="info,webdav_loop_detection_stress=debug,readur::services::webdav=debug"
    export RUST_BACKTRACE="full"
    
    # Start readur server for testing (if needed)
    if [ "${START_READUR_SERVER:-true}" = "true" ]; then
        log "Starting readur server for stress testing..."
        ./readur > "$RESULTS_DIR/logs/readur-server.log" 2>&1 &
        local readur_pid=$!
        echo "$readur_pid" > "$RESULTS_DIR/readur.pid"
        
        # Wait for server to start
        sleep 5
    fi
    
    # Run the stress tests
    log "Executing stress test suite..."
    
    local test_start_time=$(date +%s)
    local test_exit_code=0
    
    # Run the new instrumented loop detection stress test
    log "Running instrumented WebDAV loop detection stress test..."
    timeout "$((TEST_DURATION + 60))" cargo run --release \
        --bin webdav_loop_detection_stress \
        > "$RESULTS_DIR/logs/loop-detection-stress.log" 2>&1 || test_exit_code=$?
    
    # Also run the original stress tests for comparison
    log "Running legacy stress tests for comparison..."
    timeout "$((TEST_DURATION + 60))" cargo test --release \
        --features stress-testing \
        --test webdav_stress_tests \
        -- --test-threads=4 --nocapture > "$RESULTS_DIR/logs/legacy-stress-tests.log" 2>&1 || {
        local legacy_exit_code=$?
        log "Legacy stress tests exited with code $legacy_exit_code"
    }
    
    local test_end_time=$(date +%s)
    local test_duration=$((test_end_time - test_start_time))
    
    log "Stress tests completed in ${test_duration}s with exit code $test_exit_code"
    
    # Stop readur server if we started it
    if [ -f "$RESULTS_DIR/readur.pid" ]; then
        local readur_pid=$(cat "$RESULTS_DIR/readur.pid")
        kill "$readur_pid" 2>/dev/null || true
        rm -f "$RESULTS_DIR/readur.pid"
    fi
    
    return $test_exit_code
}

# Analyze test results and generate reports
analyze_results() {
    log "Analyzing stress test results..."
    
    # Analyze logs for infinite loop patterns
    if [ -f "$RESULTS_DIR/logs/stress-tests.log" ]; then
        log "Running loop detection analysis..."
        
        python3 ./scripts/analyze-webdav-loops.py \
            --log-file "$RESULTS_DIR/logs/stress-tests.log" \
            --output "$RESULTS_DIR/reports/loop-analysis.json" \
            --github-actions || true
        
        # Generate summary report
        if [ -f "$RESULTS_DIR/reports/loop-analysis.json" ]; then
            local health_score=$(jq -r '.health_score // 0' "$RESULTS_DIR/reports/loop-analysis.json")
            local infinite_loops=$(jq -r '.summary.infinite_loops_detected // 0' "$RESULTS_DIR/reports/loop-analysis.json")
            
            log "WebDAV Health Score: $health_score/100"
            log "Infinite Loops Detected: $infinite_loops"
            
            if [ "$infinite_loops" -gt 0 ]; then
                log "WARNING: Infinite loop patterns detected!"
                jq -r '.infinite_loops[] | "  - \(.path): \(.type) (severity: \(.severity))"' \
                   "$RESULTS_DIR/reports/loop-analysis.json" | while read -r line; do
                    log "$line"
                done
            fi
        fi
    fi
    
    # Generate performance report
    log "Generating performance analysis..."
    
    cat > "$RESULTS_DIR/reports/performance-summary.json" << EOF
{
    "test_timestamp": "$(date -Iseconds)",
    "test_configuration": {
        "stress_level": "$STRESS_LEVEL",
        "test_duration_seconds": $TEST_DURATION,
        "webdav_server_url": "$WEBDAV_DUFS_URL",
        "loop_detection_timeout": $LOOP_DETECTION_TIMEOUT
    },
    "test_environment": {
        "container_id": "$(hostname)",
        "rust_version": "$(rustc --version)",
        "available_memory_mb": $(free -m | awk '/^Mem:/ {print $7}'),
        "cpu_cores": $(nproc)
    }
}
EOF
    
    # Create GitHub Actions summary if running in CI
    if [ "${GITHUB_ACTIONS:-false}" = "true" ]; then
        generate_github_summary
    fi
    
    log "Result analysis completed"
}

# Generate GitHub Actions summary
generate_github_summary() {
    if [ -z "${GITHUB_STEP_SUMMARY:-}" ]; then
        return
    fi
    
    log "Generating GitHub Actions summary..."
    
    cat >> "$GITHUB_STEP_SUMMARY" << EOF
# WebDAV Stress Test Results

## Configuration
- **Stress Level**: $STRESS_LEVEL
- **Test Duration**: ${TEST_DURATION}s
- **WebDAV Server**: $WEBDAV_DUFS_URL

## Results Summary
EOF
    
    if [ -f "$RESULTS_DIR/reports/loop-analysis.json" ]; then
        local health_score=$(jq -r '.health_score // 0' "$RESULTS_DIR/reports/loop-analysis.json")
        local infinite_loops=$(jq -r '.summary.infinite_loops_detected // 0' "$RESULTS_DIR/reports/loop-analysis.json")
        local total_directories=$(jq -r '.summary.total_directories_scanned // 0' "$RESULTS_DIR/reports/loop-analysis.json")
        local total_errors=$(jq -r '.summary.total_errors // 0' "$RESULTS_DIR/reports/loop-analysis.json")
        
        cat >> "$GITHUB_STEP_SUMMARY" << EOF
- **Health Score**: $health_score/100
- **Directories Scanned**: $total_directories
- **Infinite Loops Detected**: $infinite_loops
- **Total Errors**: $total_errors

## Recommendations
EOF
        
        if [ -f "$RESULTS_DIR/reports/loop-analysis.json" ]; then
            jq -r '.recommendations[]?' "$RESULTS_DIR/reports/loop-analysis.json" | while read -r rec; do
                echo "- $rec" >> "$GITHUB_STEP_SUMMARY"
            done
        fi
    else
        echo "- Analysis data not available" >> "$GITHUB_STEP_SUMMARY"
    fi
    
    cat >> "$GITHUB_STEP_SUMMARY" << EOF

## Artifacts
- Test logs: Available in workflow artifacts
- Analysis reports: Available in workflow artifacts
EOF
}

# Cleanup function
cleanup() {
    log "Cleaning up stress test environment..."
    
    # Kill any remaining processes
    if [ -f "$RESULTS_DIR/readur.pid" ]; then
        local readur_pid=$(cat "$RESULTS_DIR/readur.pid")
        kill "$readur_pid" 2>/dev/null || true
        rm -f "$RESULTS_DIR/readur.pid"
    fi
    
    # Create final artifact archive
    if command -v tar > /dev/null; then
        tar -czf "$RESULTS_DIR/stress-test-artifacts.tar.gz" -C "$RESULTS_DIR" . 2>/dev/null || true
        log "Artifacts archived to: $RESULTS_DIR/stress-test-artifacts.tar.gz"
    fi
}

# Main execution
main() {
    local exit_code=0
    
    log "=== WebDAV Stress Test Orchestrator Starting ==="
    log "Configuration:"
    log "  - Stress Level: $STRESS_LEVEL"
    log "  - Test Duration: ${TEST_DURATION}s"
    log "  - WebDAV Server: $WEBDAV_DUFS_URL"
    log "  - Results Directory: $RESULTS_DIR"
    
    # Set up trap for cleanup
    trap cleanup EXIT
    
    # Execute test phases
    setup_test_environment || exit_code=$?
    
    if [ $exit_code -eq 0 ]; then
        generate_webdav_test_data || exit_code=$?
    fi
    
    if [ $exit_code -eq 0 ]; then
        run_stress_tests || exit_code=$?
    fi
    
    # Always run analysis, even if tests failed
    analyze_results
    
    if [ $exit_code -eq 0 ]; then
        log "=== WebDAV Stress Tests PASSED ==="
    else
        log "=== WebDAV Stress Tests FAILED (exit code: $exit_code) ==="
    fi
    
    return $exit_code
}

# Run main function
main "$@"