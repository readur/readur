# Development Guide

This guide covers contributing to Readur, setting up a development environment, testing, and code style guidelines.

## Table of Contents

- [Development Setup](#development-setup)
  - [Prerequisites](#prerequisites)
  - [Local Development](#local-development)
  - [Development with Docker](#development-with-docker)
- [Project Structure](#project-structure)
- [Testing](#testing)
  - [Backend Tests](#backend-tests)
  - [Frontend Tests](#frontend-tests)
  - [Integration Tests](#integration-tests)
  - [E2E Tests](#e2e-tests)
- [Code Style](#code-style)
  - [Rust Guidelines](#rust-guidelines)
  - [Frontend Guidelines](#frontend-guidelines)
- [Contributing](#contributing)
  - [Getting Started](#getting-started)
  - [Pull Request Process](#pull-request-process)
  - [Commit Guidelines](#commit-guidelines)
- [Debugging](#debugging)
- [Performance Profiling](#performance-profiling)

## Development Setup

### Prerequisites

- Rust 1.70+ and Cargo
- Node.js 18+ and npm
- PostgreSQL 14+
- Tesseract OCR 4.0+
- Git

### Local Development

1. **Clone the repository**:
```bash
git clone https://github.com/perfectra1n/readur.git
cd readur
```

**Set up the database:** Create a dedicated PostgreSQL database for development work.
```bash
# Create development database
sudo -u postgres psql
CREATE DATABASE readur_dev;
CREATE USER readur_dev WITH ENCRYPTED PASSWORD 'dev_password';
GRANT ALL PRIVILEGES ON DATABASE readur_dev TO readur_dev;
\q
```

**Configure environment:** Set up environment variables for local development.
```bash
# Copy example environment
cp .env.example .env.development

# Edit with your settings
DATABASE_URL=postgresql://readur_dev:dev_password@localhost/readur_dev
JWT_SECRET=dev-secret-key
```

**Run database migrations:** Apply database schema changes to your development database.
```bash
# Install sqlx-cli if needed
cargo install sqlx-cli

# Run migrations
sqlx migrate run
```

**Start the backend:** Launch the Rust backend server in development mode.
```bash
# Development mode with auto-reload
cargo watch -x run

# Or without auto-reload
cargo run
```

**Start the frontend:** Launch the React frontend development server.
```bash
cd frontend
npm install
npm run dev
```

### Development with Docker

For a consistent development environment:

```bash
# Start all services
docker compose -f docker-compose.yml -f docker-compose.dev.yml up

# Backend available at: http://localhost:8000
# Frontend dev server at: http://localhost:5173
# PostgreSQL at: localhost:5433
```

The development compose file includes:
- Volume mounts for hot reloading
- Exposed database port
- Debug logging enabled

## Project Structure

```
readur/
├── src/                    # Rust backend source
│   ├── main.rs            # Application entry point
│   ├── config.rs          # Configuration management
│   ├── models.rs          # Database models
│   ├── routes/            # API route handlers
│   ├── db/                # Database operations
│   ├── ocr.rs             # OCR processing
│   └── tests/             # Integration tests
├── frontend/              # React frontend
│   ├── src/
│   │   ├── components/    # React components
│   │   ├── pages/         # Page components
│   │   ├── services/      # API services
│   │   └── App.tsx        # Main app component
│   └── tests/             # Frontend tests
├── migrations/            # Database migrations
├── docs/                  # Documentation
└── tests/                 # E2E and integration tests
```

## Testing

Readur has comprehensive test coverage across unit, integration, and end-to-end tests.

### Backend Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_document_upload

# Run tests with coverage
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

Test categories:
- **Unit tests**: In `src/tests/`
- **Integration tests**: In `tests/`
- **Database tests**: Require `TEST_DATABASE_URL`

Example test:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_document_creation() {
        let doc = Document::new("test.pdf", "application/pdf");
        assert_eq!(doc.filename, "test.pdf");
    }
}
```

### Frontend Tests

```bash
cd frontend

# Run unit tests
npm test

# Run with coverage
npm run test:coverage

# Run in watch mode
npm run test:watch
```

Example test:
```typescript
import { render, screen } from '@testing-library/react';
import DocumentList from './DocumentList';

test('renders document list', () => {
  render(<DocumentList documents={[]} />);
  expect(screen.getByText(/No documents/i)).toBeInTheDocument();
});
```

### Integration Tests

```bash
# Run integration tests
docker compose -f docker-compose.test.yml up --abort-on-container-exit

# Or manually
cargo test --test '*' -- --test-threads=1
```

### E2E Tests

Using Playwright for end-to-end testing:

```bash
cd frontend

# Install Playwright
npm run e2e:install

# Run E2E tests
npm run e2e

# Run in UI mode
npm run e2e:ui
```

## Code Style

### Rust Guidelines

We follow the official Rust style guide with some additions:

```bash
# Format code
cargo fmt

# Check linting
cargo clippy -- -D warnings

# Check before committing
cargo fmt --check && cargo clippy
```

Style preferences:
- Use descriptive variable names
- Add documentation comments for public APIs
- Keep functions small and focused
- Use `Result` for error handling
- Prefer `&str` over `String` for function parameters

### Frontend Guidelines

```bash
# Format code
npm run format

# Lint check
npm run lint

# Type check
npm run type-check
```

Style preferences:
- Use functional components with hooks
- TypeScript for all new code
- Descriptive component and variable names
- Extract reusable logic into custom hooks
- Keep components focused and small

## Contributing

We welcome contributions!

### Getting Started

1. **Fork the repository**
2. **Create a feature branch**:
```bash
git checkout -b feature/amazing-feature
```

3. **Make your changes**
4. **Add tests** for new functionality
5. **Ensure all tests pass**:
```bash
cargo test
cd frontend && npm test
```

6. **Commit your changes** (see commit guidelines below)
7. **Push to your fork**:
```bash
git push origin feature/amazing-feature
```

8. **Open a Pull Request**

### Pull Request Process

1. **Update documentation** for any changed functionality
2. **Add tests** covering new code
3. **Ensure CI passes** (automated checks)
4. **Request review** from maintainers
5. **Address feedback** promptly
6. **Squash commits** if requested

### Commit Guidelines

We use conventional commits for clear history:

```
feat: add bulk document export
fix: resolve OCR timeout on large files
docs: update API authentication section
test: add coverage for search filters
refactor: simplify document processing pipeline
perf: optimize database queries for search
chore: update dependencies
```

Format:
```
<type>(<scope>): <subject>

<body>

<footer>
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `style`: Code style changes
- `refactor`: Code refactoring
- `perf`: Performance improvements
- `test`: Test additions/changes
- `chore`: Build process/auxiliary tool changes

## Debugging

### Backend Debugging

1. **Enable debug logging**:
```bash
RUST_LOG=debug cargo run
```

2. **Use VS Code debugger**:
```json
// .vscode/launch.json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug Readur",
      "cargo": {
        "args": ["build", "--bin=readur"],
        "filter": {
          "name": "readur",
          "kind": "bin"
        }
      },
      "args": [],
      "cwd": "${workspaceFolder}"
    }
  ]
}
```

**Database query logging:** Enable detailed SQL query logging for debugging database interactions.
```bash
RUST_LOG=sqlx=debug cargo run
```

### Frontend Debugging

1. **React DevTools**: Install browser extension
2. **Redux DevTools**: For state debugging
3. **Network tab**: Monitor API calls
4. **Console debugging**: Strategic `console.log`

## Performance Profiling

### Backend Profiling

```bash
# CPU profiling with flamegraph
cargo install flamegraph
cargo flamegraph --bin readur

# Memory profiling
valgrind --tool=massif target/release/readur
```

### Frontend Profiling

1. Use Chrome DevTools Performance tab
2. React Profiler for component performance
3. Lighthouse for overall performance audit

### Database Profiling

```sql
-- Enable query timing
\timing on

-- Analyze query plan
EXPLAIN ANALYZE SELECT * FROM documents WHERE ...;

-- Check slow queries
SELECT * FROM pg_stat_statements 
ORDER BY total_time DESC 
LIMIT 10;
```

## Additional Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [React Documentation](https://react.dev/)
- [PostgreSQL Documentation](https://www.postgresql.org/docs/)
- [Tesseract Documentation](https://tesseract-ocr.github.io/)
- [Testing Guide](TESTING.md)

## Getting Help

- **GitHub Issues**: For bug reports and feature requests
- **GitHub Discussions**: For questions and community support
- **Discord**: Join our community server (link in README)

## License

By contributing to Readur, you agree that your contributions will be licensed under the MIT License.