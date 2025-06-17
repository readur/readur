#!/bin/bash

# Script to view latest test results

TEST_RESULTS_DIR="test-results"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

print_header() {
    echo -e "${BLUE}=== $1 ===${NC}"
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_info() {
    echo -e "${YELLOW}ℹ${NC} $1"
}

# Check if test results directory exists
if [ ! -d "$TEST_RESULTS_DIR" ]; then
    echo "No test results found. Run 'make test' first."
    exit 1
fi

# Parse command line arguments
ACTION="${1:-summary}"

case $ACTION in
    summary|s)
        print_header "Latest Test Summary"
        # Find the most recent summary file
        LATEST_SUMMARY=$(ls -t "$TEST_RESULTS_DIR"/reports/*_summary.txt 2>/dev/null | head -1)
        if [ -f "$LATEST_SUMMARY" ]; then
            cat "$LATEST_SUMMARY"
        else
            echo "No test summary found."
        fi
        ;;
    
    html|h)
        print_header "Opening HTML Report"
        # Find the most recent HTML report
        LATEST_HTML=$(ls -t "$TEST_RESULTS_DIR"/reports/*_test_report.html 2>/dev/null | head -1)
        if [ -f "$LATEST_HTML" ]; then
            print_success "Opening $LATEST_HTML"
            # Try to open with default browser
            if command -v xdg-open >/dev/null 2>&1; then
                xdg-open "$LATEST_HTML"
            elif command -v open >/dev/null 2>&1; then
                open "$LATEST_HTML"
            else
                print_info "Manual open required: $LATEST_HTML"
            fi
        else
            echo "No HTML report found."
        fi
        ;;
    
    logs|l)
        print_header "Available Test Logs"
        if [ -d "$TEST_RESULTS_DIR" ]; then
            echo "Unit Tests:"
            ls -la "$TEST_RESULTS_DIR"/unit/ 2>/dev/null || echo "  No unit test logs"
            echo ""
            echo "Integration Tests:"
            ls -la "$TEST_RESULTS_DIR"/integration/ 2>/dev/null || echo "  No integration test logs"
            echo ""
            echo "Frontend Tests:"
            ls -la "$TEST_RESULTS_DIR"/frontend/ 2>/dev/null || echo "  No frontend test logs"
            echo ""
            echo "Reports:"
            ls -la "$TEST_RESULTS_DIR"/reports/ 2>/dev/null || echo "  No reports"
        fi
        ;;
    
    unit|u)
        print_header "Latest Unit Test Results"
        LATEST_UNIT=$(ls -t "$TEST_RESULTS_DIR"/unit/*.log 2>/dev/null | head -1)
        if [ -f "$LATEST_UNIT" ]; then
            print_success "From: $LATEST_UNIT"
            echo ""
            cat "$LATEST_UNIT"
        else
            echo "No unit test results found."
        fi
        ;;
    
    integration|i)
        print_header "Latest Integration Test Results"
        LATEST_INTEGRATION=$(ls -t "$TEST_RESULTS_DIR"/integration/*.log 2>/dev/null | head -1)
        if [ -f "$LATEST_INTEGRATION" ]; then
            print_success "From: $LATEST_INTEGRATION"
            echo ""
            cat "$LATEST_INTEGRATION"
        else
            echo "No integration test results found."
        fi
        ;;
    
    frontend|f)
        print_header "Latest Frontend Test Results"
        LATEST_FRONTEND=$(ls -t "$TEST_RESULTS_DIR"/frontend/*.log 2>/dev/null | head -1)
        if [ -f "$LATEST_FRONTEND" ]; then
            print_success "From: $LATEST_FRONTEND"
            echo ""
            cat "$LATEST_FRONTEND"
        else
            echo "No frontend test results found."
        fi
        ;;
    
    clean|c)
        print_header "Cleaning Test Results"
        read -p "Are you sure you want to delete all test results? (y/N): " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            rm -rf "$TEST_RESULTS_DIR"/*
            print_success "Test results cleaned"
        else
            echo "Cancelled"
        fi
        ;;
    
    help|--help|-h)
        print_header "Test Results Viewer"
        echo "Usage: $0 [command]"
        echo ""
        echo "Commands:"
        echo "  summary, s     Show latest test summary (default)"
        echo "  html, h        Open latest HTML report in browser"
        echo "  logs, l        List all available test logs"
        echo "  unit, u        Show latest unit test results"
        echo "  integration, i Show latest integration test results"
        echo "  frontend, f    Show latest frontend test results"
        echo "  clean, c       Clean all test results"
        echo "  help, -h       Show this help"
        echo ""
        echo "Examples:"
        echo "  $0              # Show summary"
        echo "  $0 html         # Open HTML report"
        echo "  $0 unit         # Show unit test details"
        ;;
    
    *)
        echo "Unknown command: $ACTION"
        echo "Run '$0 help' for available commands."
        exit 1
        ;;
esac