# Comprehensive Mock API Framework

A robust, TypeScript-first API mocking framework for the Readur2 project, built on MSW (Mock Service Worker) with comprehensive support for unit, integration, and E2E testing.

## 🚀 Features

- **Complete API Coverage**: Mock handlers for all API endpoints
- **TypeScript-First**: Full type safety with comprehensive type definitions  
- **Realistic Data**: Faker.js-powered data factories for consistent test data
- **Scenario-Based Testing**: Predefined scenarios for common testing situations
- **WebSocket Support**: Full WebSocket mocking for real-time features
- **Network Simulation**: Configurable delays, errors, and network conditions
- **React Integration**: Custom hooks for easy test setup and configuration
- **Performance Testing**: Built-in performance monitoring and benchmarking
- **Error Simulation**: Comprehensive error condition testing
- **Cross-Environment**: Works seamlessly in unit, integration, and E2E tests

## 📁 Project Structure

```
src/test/mocks/
├── api/                    # Core API framework
│   ├── server.ts          # MSW server configuration
│   └── types.ts           # TypeScript definitions
├── handlers/              # Request handlers
│   ├── index.ts           # Handler exports
│   ├── documents.ts       # Document API handlers
│   ├── auth.ts            # Authentication handlers
│   ├── search.ts          # Search API handlers
│   ├── queue.ts           # Queue management handlers
│   ├── sources.ts         # Source management handlers
│   ├── labels.ts          # Label management handlers
│   ├── users.ts           # User management handlers
│   ├── ocr.ts             # OCR service handlers
│   └── settings.ts        # Settings handlers
├── factories/             # Data factories
│   ├── index.ts           # Factory exports
│   ├── document.ts        # Document data factory
│   ├── user.ts            # User data factory
│   ├── source.ts          # Source data factory
│   ├── search.ts          # Search data factory
│   ├── queue.ts           # Queue data factory
│   ├── label.ts           # Label data factory
│   └── combined.ts        # Combined datasets
├── fixtures/              # Test fixtures
│   ├── index.ts           # Fixture exports
│   ├── scenarios.ts       # Test scenarios
│   ├── datasets.ts        # Predefined datasets
│   ├── performance.ts     # Performance test data
│   └── error-conditions.ts # Error scenarios
├── utils/                 # Utilities
│   ├── index.ts           # Utility exports
│   ├── config.ts          # Configuration utilities
│   ├── websocket.ts       # WebSocket mocking
│   └── react-hooks.ts     # React testing hooks
└── index.ts               # Main framework export
```

## 🎯 Quick Start

### Basic Setup

```typescript
import { setupMockApi } from '../test/mocks'

// Quick setup for common scenarios
const { cleanup } = await setupMockApi({
  scenario: 'ACTIVE_SYSTEM',
  networkCondition: 'fast',
  enableWebSocket: true
})

// Clean up after tests
afterAll(cleanup)
```

### Unit Tests

```typescript
import { renderWithMockApi, getDefaultTestUser } from '../test/test-utils'
import { useMockApi } from '../test/mocks'

test('document list renders correctly', () => {
  const { container } = renderWithMockApi(<DocumentList />, {
    scenario: 'ACTIVE_SYSTEM',
    authValues: { user: getDefaultTestUser(), isAuthenticated: true }
  })
  
  expect(container).toMatchSnapshot()
})

test('handles network errors gracefully', () => {
  const mockApi = useMockApi()
  mockApi.simulateNetworkError()
  
  renderWithMockApi(<DocumentList />)
  expect(screen.getByText(/network error/i)).toBeInTheDocument()
})
```

### Integration Tests

```typescript
import { setupAuthenticatedTest, withPerformanceMonitoring } from '../test/test-utils'

test('complete document upload workflow', withPerformanceMonitoring(async () => {
  const testUtils = setupAuthenticatedTest('user')
  
  // Upload document
  const file = new File(['test content'], 'test.pdf', { type: 'application/pdf' })
  await testUtils.addDocument(createMockDocument({ filename: 'test.pdf' }))
  
  // Verify document appears in list
  await waitFor(() => {
    expect(testUtils.documents).toHaveLength(1)
    expect(testUtils.documents[0].filename).toBe('test.pdf')
  })
}))
```

### E2E Tests (Playwright)

```typescript
import { test, expect } from '@playwright/test'

test.beforeEach(async ({ page }) => {
  // Setup mock API for browser environment
  await page.addInitScript(() => {
    window.__MOCK_API_SCENARIO__ = 'ACTIVE_SYSTEM'
  })
})

test('user can search documents', async ({ page }) => {
  await page.goto('/')
  await page.fill('[data-testid="search-input"]', 'invoice')
  await page.click('[data-testid="search-button"]')
  
  await expect(page.locator('[data-testid="search-results"]')).toBeVisible()
  await expect(page.locator('[data-testid="document-item"]')).toHaveCount(5)
})
```

## 🎭 Scenarios

### Predefined Scenarios

```typescript
import { TEST_SCENARIOS } from '../test/mocks/fixtures'

// Available scenarios:
TEST_SCENARIOS.EMPTY_SYSTEM          // Clean system with no data
TEST_SCENARIOS.NEW_USER_SETUP        // Fresh user account
TEST_SCENARIOS.ACTIVE_SYSTEM         // Normal operation with data
TEST_SCENARIOS.SYSTEM_UNDER_LOAD     // Heavy usage scenario
TEST_SCENARIOS.MULTI_USER_SYSTEM     // Multiple users
TEST_SCENARIOS.PROBLEMATIC_SYSTEM    // Error conditions
```

### Custom Scenarios

```typescript
import { generateScenarioDataset } from '../test/mocks/factories'

// Create custom scenario
const customScenario = generateScenarioDataset('my_scenario')
customScenario.documents = createMockDocuments(100)
customScenario.queueStats = createMockQueueStats({ pending_count: 50 })
```

## 🔧 Configuration

### Network Conditions

```typescript
import { NETWORK_CONDITIONS } from '../test/mocks/utils'

// Apply different network conditions
mockApi.setNetworkConfig(NETWORK_CONDITIONS.SLOW)
mockApi.setNetworkConfig(NETWORK_CONDITIONS.OFFLINE)
mockApi.setNetworkConfig(NETWORK_CONDITIONS.REALISTIC)
```

### Error Simulation

```typescript
import { ERROR_SCENARIOS } from '../test/mocks/fixtures'

// Simulate specific errors
mockApi.setNetworkConfig(ERROR_SCENARIOS.UNAUTHORIZED)
mockApi.setNetworkConfig(ERROR_SCENARIOS.INTERNAL_SERVER_ERROR)
mockApi.setNetworkConfig(ERROR_SCENARIOS.NETWORK_ERROR)
```

## 🧪 Data Factories

### Document Factory

```typescript
import { createMockDocument, createMockDocumentWithScenario } from '../test/mocks/factories'

// Basic document
const doc = createMockDocument()

// Scenario-specific documents
const pdfDoc = createMockDocumentWithScenario('pdf_with_high_confidence_ocr')
const failedDoc = createMockDocumentWithScenario('image_with_failed_ocr')
const recentDoc = createMockDocumentWithScenario('recently_uploaded')

// Batch creation
const docs = createMockDocuments(50, { 
  overrides: { user_id: 'specific-user' } 
})
```

### User Factory

```typescript
import { createMockUser, createMockUserWithScenario } from '../test/mocks/factories'

// Basic user
const user = createMockUser()

// Scenario-specific users
const admin = createMockUserWithScenario('admin_user')
const oidcUser = createMockUserWithScenario('oidc_user')
const newUser = createMockUserWithScenario('new_user')
```

## 🔌 WebSocket Testing

### Basic WebSocket Testing

```typescript
import { useMockWebSocket, WebSocketTestUtils } from '../test/mocks/utils'

test('sync progress updates', async () => {
  const { webSocket, connectionState, lastMessage } = useMockWebSocket(
    'ws://localhost:8000/api/sources/123/sync/progress/ws'
  )
  
  // Wait for connection
  await WebSocketTestUtils.waitForState(webSocket, WebSocket.OPEN)
  
  // Simulate progress updates
  webSocket.startSyncProgressSimulation('source-123', 'in_progress')
  
  // Wait for progress message
  const progressMsg = await WebSocketTestUtils.waitForMessage(webSocket, 'progress')
  expect(progressMsg.data.source_id).toBe('source-123')
})
```

### Realistic Sync Simulation

```typescript
test('realistic sync progress flow', () => {
  const ws = WebSocketTestUtils.createWebSocket('ws://localhost:8000/ws')
  
  // Simulate 10-second sync process
  WebSocketTestUtils.simulateRealisticSyncProgress(ws, 'source-123', 10000)
  
  // Test receives progress updates every 500ms
  // Progresses through: discovery → processing → cleanup → completed
})
```

## 🚀 Performance Testing

### Performance Monitoring

```typescript
import { withPerformanceMonitoring, PERFORMANCE_BENCHMARKS } from '../test/mocks'

test('search performance', withPerformanceMonitoring(async () => {
  const startMeasurement = PerformanceTestUtils.startMeasurement('search')
  
  // Perform search
  await searchService.enhancedSearch({ query: 'test' })
  
  const duration = startMeasurement()
  
  // Assert performance
  PERFORMANCE_ASSERTIONS.assertPerformance('search', duration, 'SEARCH_RESPONSE')
}))
```

### Load Testing

```typescript
import { LARGE_DATASETS, CONCURRENCY_SCENARIOS } from '../test/mocks/fixtures'

test('handles large document sets', () => {
  const testUtils = useMockTestUtils()
  testUtils.setDocuments(LARGE_DATASETS.EXTRA_LARGE_LOAD.documents)
  
  // Test with 5000 documents
  renderWithMockApi(<DocumentList />)
  // Verify performance metrics
})
```

## 🔄 React Hooks

### useMockApi

```typescript
import { useMockApi } from '../test/mocks'

const TestComponent = () => {
  const mockApi = useMockApi({
    scenario: 'ACTIVE_SYSTEM',
    defaultDelay: 100,
    resetOnUnmount: true
  })
  
  const handleSlowNetwork = () => {
    mockApi.simulateSlowNetwork(2000)
  }
  
  const handleNetworkError = () => {
    mockApi.simulateNetworkError()
  }
  
  return (
    <div>
      <button onClick={handleSlowNetwork}>Simulate Slow Network</button>
      <button onClick={handleNetworkError}>Simulate Error</button>
      <p>Current scenario: {mockApi.currentScenario}</p>
    </div>
  )
}
```

### useMockAuth

```typescript
import { useMockAuth } from '../test/mocks'

const AuthTestComponent = () => {
  const { isAuthenticated, currentUser, login, logout } = useMockAuth()
  
  const handleLogin = () => {
    login(getDefaultTestUser())
  }
  
  return (
    <div>
      {isAuthenticated ? (
        <div>
          <p>Welcome, {currentUser?.username}</p>
          <button onClick={logout}>Logout</button>
        </div>
      ) : (
        <button onClick={handleLogin}>Login</button>
      )}
    </div>
  )
}
```

### useMockDocuments

```typescript
import { useMockDocuments } from '../test/mocks'

const DocumentTestComponent = () => {
  const { documents, addDocument, removeDocument, clearDocuments } = useMockDocuments()
  
  const handleAddDocument = () => {
    addDocument(createMockDocument({ filename: 'new-doc.pdf' }))
  }
  
  return (
    <div>
      <p>Documents: {documents.length}</p>
      <button onClick={handleAddDocument}>Add Document</button>
      <button onClick={clearDocuments}>Clear All</button>
    </div>
  )
}
```

## 🔍 Debugging

### Logging and Inspection

```typescript
import { server } from '../test/mocks'

// Log all requests
server.use(
  rest.all('*', (req, res, ctx) => {
    console.log(`${req.method} ${req.url}`)
    return req.passthrough()
  })
)

// Inspect mock state
import { resetMockDocuments, setMockDocuments } from '../test/mocks/handlers'

// Reset and inspect
resetMockDocuments()
console.log('Documents reset')

// Set specific test data
setMockDocuments([createMockDocument({ filename: 'debug.pdf' })])
```

### Error Tracking

```typescript
import { ErrorTestUtils } from '../test/mocks/fixtures'

// Track errors during tests
const errorHandler = (error, context) => {
  ErrorTestUtils.logError(error, context)
}

// Later, assert errors were handled
ErrorTestUtils.assertErrorHandled('Network Error', 'document-upload')
```

## 🏗️ Extending the Framework

### Adding New Handlers

```typescript
// Create new handler file: handlers/notifications.ts
import { http, HttpResponse } from 'msw'
import { createMockResponse, DEFAULT_MOCK_CONFIG } from '../utils/config'

export const notificationHandlers = [
  http.get('/api/notifications', async () => {
    const notifications = createMockNotifications()
    return HttpResponse.json(createMockResponse(notifications, DEFAULT_MOCK_CONFIG))
  })
]

// Add to handlers/index.ts
export { notificationHandlers } from './notifications'
export const handlers = [
  ...existingHandlers,
  ...notificationHandlers,
]
```

### Custom Data Factories

```typescript
// Create new factory: factories/notification.ts
import { faker } from '@faker-js/faker'

export const createMockNotification = (overrides = {}) => ({
  id: faker.string.uuid(),
  message: faker.lorem.sentence(),
  type: faker.helpers.arrayElement(['info', 'warning', 'error', 'success']),
  created_at: faker.date.recent().toISOString(),
  read: faker.datatype.boolean(),
  ...overrides
})

export const createMockNotifications = (count = 5) => 
  Array.from({ length: count }, () => createMockNotification())
```

### Custom Scenarios

```typescript
// Add to fixtures/scenarios.ts
export const NOTIFICATION_TEST_SCENARIOS = {
  UNREAD_NOTIFICATIONS: {
    name: 'Unread Notifications',
    description: 'System with multiple unread notifications',
    data: {
      notifications: createMockNotifications(10).map(n => ({ ...n, read: false }))
    }
  }
}
```

## 📋 Best Practices

### 1. Consistent Data

```typescript
// Always use factories for consistent data
✅ const doc = createMockDocument()
❌ const doc = { id: '1', filename: 'test.pdf' } // Incomplete data
```

### 2. Scenario-Based Testing

```typescript
// Use predefined scenarios when possible
✅ renderWithMockApi(<Component />, { scenario: 'ACTIVE_SYSTEM' })
❌ // Setting up mock data manually each time
```

### 3. Realistic Network Conditions

```typescript
// Test with realistic delays for integration tests
✅ setNetworkConfig(NETWORK_CONDITIONS.REALISTIC)
❌ setNetworkConfig({ delay: 0 }) // Too fast for integration tests
```

### 4. Error Condition Testing

```typescript
// Always test error conditions
test('handles server errors gracefully', () => {
  mockApi.simulateError('server')
  renderWithMockApi(<Component />)
  expect(screen.getByText(/error occurred/i)).toBeInTheDocument()
})
```

### 5. Cleanup

```typescript
// Always clean up mock state
afterEach(() => {
  resetMockApi()
  vi.clearAllMocks()
})
```

## 🤝 Contributing

When adding new features to the mock framework:

1. **Add Type Definitions**: Extend `api/types.ts` with new interfaces
2. **Create Handlers**: Add new handlers in the appropriate `handlers/` file
3. **Build Factories**: Create data factories in `factories/` for new entities
4. **Add Scenarios**: Define test scenarios in `fixtures/scenarios.ts`
5. **Write Tests**: Test the mock functionality itself
6. **Update Documentation**: Keep this README up to date

## 🔧 Troubleshooting

### Common Issues

**MSW Not Intercepting Requests**
```typescript
// Ensure server is started in test setup
beforeAll(() => {
  server.listen({ onUnhandledRequest: 'warn' })
})
```

**WebSocket Not Connecting**
```typescript
// Check WebSocket mocking is enabled
enableWebSocketMocking({ autoConnect: true })
```

**TypeScript Errors**
```typescript
// Make sure to import types properly
import type { MockDocument, MockConfig } from '../test/mocks/api/types'
```

**Slow Tests**
```typescript
// Use faster configurations for unit tests
mockApi.setNetworkConfig({ delay: 10 })
```

**Memory Leaks**
```typescript
// Always clean up WebSocket connections
afterEach(() => {
  WebSocketTestUtils.closeAllWebSockets()
})
```

## 📈 Performance Guidelines

- **Unit Tests**: Use `delay: 0-50ms` for fast execution
- **Integration Tests**: Use `delay: 100-300ms` for realistic timing
- **E2E Tests**: Use `delay: 200-500ms` to match real network conditions
- **Load Tests**: Use large datasets from `LARGE_DATASETS`
- **Performance Tests**: Monitor with `withPerformanceMonitoring()`

## 🎯 Examples Repository

Check the `src/test/mocks/examples/` directory for comprehensive examples:

- Basic unit test setups
- Integration test scenarios  
- E2E test configurations
- Performance testing examples
- Error handling patterns
- WebSocket testing examples
- Custom extension examples

---

This mock framework provides everything needed for comprehensive testing of the Readur2 application. It's designed to be easy to use, maintain, and extend while providing realistic test conditions across all testing scenarios.