# Development Quick Start

## Purpose

Set up a Readur development environment in 10 minutes. This guide helps developers contribute to Readur or build custom integrations.

## Prerequisites

- Python 3.10+ installed
- Node.js 18+ and npm installed
- PostgreSQL 14+ running locally
- Redis server installed
- Git configured with GitHub access
- 8GB RAM recommended for development

## Step 1: Clone and Setup

Clone the repository and create a virtual environment:

```bash
# Clone repository
git clone https://github.com/readur/readur.git
cd readur

# Create Python virtual environment
python -m venv venv
source venv/bin/activate  # On Windows: venv\Scripts\activate

# Install Python dependencies
pip install -r requirements.txt
pip install -r requirements-dev.txt
```

## Step 2: Database Setup

Create and initialize the database:

```bash
# Create database
createdb readur_dev

# Set environment variables
export DATABASE_URL=postgresql://localhost/readur_dev
export REDIS_URL=redis://localhost:6379

# Run migrations
alembic upgrade head

# Create test user
python scripts/create_user.py --username dev --password dev123 --admin
```

## Step 3: Frontend Setup

Install and build frontend assets:

```bash
# Navigate to frontend directory
cd frontend

# Install dependencies
npm install

# Start development server
npm run dev
```

Keep this terminal running - the frontend will auto-reload on changes.

## Step 4: Backend Development Server

In a new terminal, start the backend:

```bash
# Activate virtual environment
source venv/bin/activate

# Set development environment
export FLASK_ENV=development
export FLASK_DEBUG=1

# Start backend server
python run.py
```

## Step 5: Start Background Workers

In another terminal, start the OCR worker:

```bash
# Activate virtual environment
source venv/bin/activate

# Start Celery worker
celery -A readur.worker worker --loglevel=info --concurrency=2
```

## Step 6: Access Development Instance

Your development environment is now running:

- **Frontend**: http://localhost:3000 (with hot reload)
- **Backend API**: http://localhost:5000
- **API Documentation**: http://localhost:5000/api/docs

Login with:
- Username: `dev`
- Password: `dev123`

## Development Workflow

### Code Structure

```
readur/
├── backend/          # Python Flask application
│   ├── api/         # REST API endpoints
│   ├── models/      # Database models
│   ├── services/    # Business logic
│   └── workers/     # Background tasks
├── frontend/        # React application
│   ├── src/
│   │   ├── components/
│   │   ├── pages/
│   │   └── services/
│   └── public/
├── tests/           # Test suites
└── scripts/         # Development utilities
```

### Making Changes

1. **Create feature branch**:
   ```bash
   git checkout -b feature/your-feature
   ```

2. **Backend changes**:
   - Edit Python files
   - Backend auto-reloads with Flask debug mode
   - Run tests: `pytest tests/`

3. **Frontend changes**:
   - Edit React components
   - Frontend auto-reloads with webpack dev server
   - Run tests: `npm test`

4. **Database changes**:
   ```bash
   # Create migration
   alembic revision --autogenerate -m "Description"
   
   # Apply migration
   alembic upgrade head
   ```

## Testing

### Run All Tests

```bash
# Backend tests
pytest tests/ -v

# Frontend tests
cd frontend && npm test

# End-to-end tests
pytest tests/e2e/ --browser chromium
```

### Test Coverage

```bash
# Generate coverage report
pytest --cov=readur --cov-report=html
open htmlcov/index.html
```

### Linting

```bash
# Python linting
flake8 readur/
black readur/ --check
mypy readur/

# JavaScript linting
cd frontend
npm run lint
npm run format:check
```

## Debugging

### Backend Debugging

Using VS Code:

```json
// .vscode/launch.json
{
  "version": "0.2.0",
  "configurations": [
    {
      "name": "Python: Flask",
      "type": "python",
      "request": "launch",
      "module": "flask",
      "env": {
        "FLASK_APP": "readur:create_app",
        "FLASK_ENV": "development"
      },
      "args": ["run", "--no-debugger", "--no-reload"],
      "jinja": true
    }
  ]
}
```

Using pdb:

```python
# Add breakpoint in code
import pdb; pdb.set_trace()
```

### Frontend Debugging

React Developer Tools:
1. Install browser extension
2. Open Developer Tools → Components tab
3. Inspect component state and props

Redux DevTools:
1. Install browser extension
2. View action history and state changes

### Database Debugging

```bash
# Connect to database
psql readur_dev

# View query logs
export DATABASE_ECHO=true
```

## Local Development Tools

### Mock Data

Generate test documents:

```bash
python scripts/generate_test_data.py --documents 100
```

### Performance Profiling

```bash
# Profile API endpoint
python -m cProfile -o profile.stats run.py

# Analyze results
python -m pstats profile.stats
```

### API Testing

Using httpie:

```bash
# Login
http POST localhost:5000/api/auth/login username=dev password=dev123

# Upload document
http POST localhost:5000/api/documents file@test.pdf \
  "Authorization: Bearer $TOKEN"
```

## Common Development Tasks

### Add New API Endpoint

1. Create route in `backend/api/`
2. Add service logic in `backend/services/`
3. Write tests in `tests/api/`
4. Update OpenAPI schema

### Add Frontend Feature

1. Create component in `frontend/src/components/`
2. Add route in `frontend/src/routes.js`
3. Create API service in `frontend/src/services/`
4. Write component tests

### Add Background Task

1. Define task in `backend/workers/tasks.py`
2. Add to task queue in service layer
3. Write worker tests
4. Update worker documentation

## Troubleshooting

### Dependencies Won't Install

```bash
# Update pip and setuptools
pip install --upgrade pip setuptools wheel

# Clear pip cache
pip cache purge

# Use specific Python version
python3.10 -m venv venv
```

### Database Connection Failed

```bash
# Check PostgreSQL is running
pg_isready

# Check connection
psql -U postgres -c "SELECT 1"

# Reset database
dropdb readur_dev
createdb readur_dev
alembic upgrade head
```

### Frontend Build Errors

```bash
# Clear node modules
rm -rf node_modules package-lock.json
npm install

# Clear build cache
npm run clean
npm run build
```

### OCR Worker Not Processing

```bash
# Check Redis connection
redis-cli ping

# Monitor worker logs
celery -A readur.worker worker --loglevel=debug

# Purge task queue
celery -A readur.worker purge
```

## Contributing

### Before Submitting PR

1. **Run all tests**: Ensure all tests pass
2. **Check linting**: Fix any style issues
3. **Update documentation**: Document new features
4. **Add tests**: Cover new functionality
5. **Test migrations**: Verify database changes

### Code Style

Follow project conventions:
- Python: PEP 8 with Black formatting
- JavaScript: ESLint + Prettier
- Commits: Conventional commits format
- Documentation: Markdown with proper headings

## Related Documentation

- [Architecture Overview](../architecture.md) - System design and components
- [API Reference](../api-reference.md) - Complete API documentation
- [Testing Guide](../dev/TESTING.md) - Comprehensive testing strategies
- [Contributing Guide](../dev/README.md) - Contribution guidelines