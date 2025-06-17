.PHONY: help test test-unit test-integration test-frontend test-e2e test-all test-watch test-clean test-logs test-shell dev-up dev-down dev-logs build clean

# Default target
help:
	@echo "Readur Development and Testing Commands"
	@echo "======================================"
	@echo ""
	@echo "Testing Commands:"
	@echo "  make test              - Run all tests in isolated environment"
	@echo "  make test-unit         - Run unit tests only"
	@echo "  make test-integration  - Run integration tests only"
	@echo "  make test-frontend     - Run frontend tests only"
	@echo "  make test-e2e          - Run E2E tests (when implemented)"
	@echo "  make test-watch        - Run tests and keep containers running"
	@echo "  make test-clean        - Clean up test environment"
	@echo "  make test-logs         - View test container logs"
	@echo "  make test-shell        - Open shell in test container"
	@echo ""
	@echo "Development Commands:"
	@echo "  make dev-up            - Start development environment"
	@echo "  make dev-down          - Stop development environment"
	@echo "  make dev-logs          - View development logs"
	@echo "  make build             - Build all Docker images"
	@echo "  make clean             - Clean all Docker resources"

# Testing targets
test: test-all

test-all:
	@./run-tests.sh all

test-unit:
	@./run-tests.sh unit

test-integration:
	@./run-tests.sh integration

test-frontend:
	@./run-tests.sh frontend

test-e2e:
	@./run-tests.sh e2e

test-watch:
	@./run-tests.sh all keep-running

test-clean:
	@echo "Cleaning test environment..."
	@docker-compose -f docker-compose.test.yml -p readur_test down -v --remove-orphans 2>/dev/null || true
	@docker rm -f readur_postgres_test readur_app_test readur_frontend_test 2>/dev/null || true
	@docker network rm readur_test_network 2>/dev/null || true
	@rm -rf /tmp/test_uploads /tmp/test_watch 2>/dev/null || true

test-logs:
	@docker-compose -f docker-compose.test.yml -p readur_test logs -f

test-shell:
	@docker-compose -f docker-compose.test.yml -p readur_test exec readur_test /bin/bash

# Development targets
dev-up:
	@echo "Starting development environment..."
	@docker-compose up -d

dev-down:
	@echo "Stopping development environment..."
	@docker-compose down

dev-logs:
	@docker-compose logs -f

# Build targets
build:
	@echo "Building all images..."
	@docker-compose build
	@docker-compose -f docker-compose.test.yml build

# Clean targets
clean:
	@echo "Cleaning all Docker resources..."
	@docker-compose down -v --remove-orphans
	@docker-compose -f docker-compose.test.yml -p readur_test down -v --remove-orphans
	@docker system prune -f

# Specific test scenarios
test-ocr:
	@echo "Running OCR-specific tests..."
	@docker-compose -f docker-compose.test.yml -p readur_test exec -T readur_test \
		cargo test ocr --no-fail-fast

test-webdav:
	@echo "Running WebDAV-specific tests..."
	@docker-compose -f docker-compose.test.yml -p readur_test exec -T readur_test \
		cargo test webdav --no-fail-fast

test-performance:
	@echo "Running performance tests..."
	@docker-compose -f docker-compose.test.yml -p readur_test exec -T readur_test \
		cargo test performance --no-fail-fast

# Database operations
test-db-reset:
	@echo "Resetting test database..."
	@docker-compose -f docker-compose.test.yml -p readur_test exec -T postgres_test \
		psql -U readur_test -d postgres -c "DROP DATABASE IF EXISTS readur_test; CREATE DATABASE readur_test;"
	@docker-compose -f docker-compose.test.yml -p readur_test exec -T readur_test \
		sqlx migrate run

# CI/CD helpers
ci-test:
	@./run-tests.sh all

# Quick test for local development (runs faster subset)
quick-test:
	@echo "Running quick test suite..."
	@./run-tests.sh unit