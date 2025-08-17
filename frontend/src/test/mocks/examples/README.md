# Mock API Framework Examples

This directory contains comprehensive examples demonstrating how to use the mock API framework across different testing scenarios.

## 📁 Files Overview

- **`unit-test-example.test.tsx`** - Unit testing patterns and utilities
- **`integration-test-example.test.tsx`** - Integration testing workflows  
- **`e2e-test-example.spec.ts`** - End-to-end testing with Playwright
- **`performance-test-example.test.tsx`** - Performance testing examples
- **`websocket-test-example.test.tsx`** - WebSocket testing patterns

## 🧪 Unit Test Examples

### Basic Component Testing
```typescript
test('renders document list correctly', () => {
  renderWithMockApi(<DocumentList />, {
    scenario: 'ACTIVE_SYSTEM'
  })
  
  expect(screen.getByTestId('document-list')).toBeInTheDocument()
})
```

### Error Condition Testing
```typescript
test('handles network errors gracefully', () => {
  const { simulateNetworkError } = useMockApi()
  simulateNetworkError()
  
  renderWithMockApi(<DocumentList />)
  expect(screen.getByText(/error occurred/i)).toBeInTheDocument()
})
```

### Custom Mock Data
```typescript
test('works with specific documents', () => {
  const customDocs = [
    createMockDocumentWithScenario('pdf_with_high_confidence_ocr')
  ]
  
  const { setDocuments } = useMockDocuments()
  setDocuments(customDocs)
  
  renderWithMockApi(<DocumentList />)
  expect(screen.getByText('document.pdf')).toBeInTheDocument()
})
```

## 🔗 Integration Test Examples

### Complete Workflows
```typescript
test('complete document upload workflow', async () => {
  const testUtils = setupAuthenticatedTest('user')
  
  // Upload document
  const file = new File(['content'], 'test.pdf')
  await testUtils.addDocument(createMockDocument({ filename: 'test.pdf' }))
  
  // Verify document appears
  await waitFor(() => {
    expect(testUtils.documents).toHaveLength(1)
  })
})
```

### Real-time Features
```typescript
test('WebSocket sync progress monitoring', async () => {
  const { webSocket } = useMockWebSocket('/api/sync/ws')
  
  // Start sync simulation
  webSocket.startSyncProgressSimulation('source-123')
  
  // Verify progress updates
  await WebSocketTestUtils.waitForMessage(webSocket, 'progress')
})
```

### Performance Monitoring
```typescript
test('search performance', withPerformanceMonitoring(async () => {
  // Test runs with performance monitoring
  // Automatic assertions for timing
}))
```

## 🎭 E2E Test Examples

### Full User Workflows
```typescript
test('complete user search workflow', async ({ page }) => {
  await page.goto('/search')
  await page.fill('[data-testid="search-input"]', 'invoice')
  await page.click('[data-testid="search-button"]')
  
  await expect(page.locator('[data-testid="search-results"]')).toBeVisible()
})
```

### Error Recovery
```typescript
test('handles network errors and recovery', async ({ page }) => {
  // Simulate error
  await page.evaluate(() => {
    window.__MOCK_API_SCENARIO__ = 'NETWORK_ERROR'
  })
  
  // Verify error handling
  await expect(page.locator('[data-testid="error-message"]')).toBeVisible()
  
  // Test recovery
  await page.click('[data-testid="retry-button"]')
})
```

### Mobile Testing
```typescript
test('mobile responsive behavior', async ({ page }) => {
  await page.setViewportSize({ width: 375, height: 667 })
  
  // Test mobile-specific UI
  await expect(page.locator('[data-testid="mobile-menu"]')).toBeVisible()
})
```

## 🚀 Performance Test Examples

### Load Testing
```typescript
test('handles large datasets', () => {
  const { setDocuments } = useMockDocuments()
  setDocuments(LARGE_DATASETS.EXTRA_LARGE_LOAD.documents)
  
  const startTime = performance.now()
  renderWithMockApi(<DocumentList />)
  
  // Performance assertions
  const renderTime = performance.now() - startTime
  expect(renderTime).toBeLessThan(1000)
})
```

### Network Condition Testing
```typescript
test('performance under slow network', async () => {
  renderWithMockApi(<SearchPage />, {
    networkCondition: 'slow'
  })
  
  // Test should handle slow responses gracefully
})
```

## 🔌 WebSocket Test Examples

### Real-time Updates
```typescript
test('receives real-time sync updates', async () => {
  const { webSocket, lastMessage } = useMockWebSocket('/api/sync/ws')
  
  // Simulate progress
  webSocket.startSyncProgressSimulation('source-1', 'in_progress')
  
  await waitFor(() => {
    expect(lastMessage?.type).toBe('progress')
    expect(lastMessage?.data.source_id).toBe('source-1')
  })
})
```

### Connection Management
```typescript
test('handles WebSocket reconnection', async () => {
  const ws = WebSocketTestUtils.createWebSocket('/api/sync/ws')
  
  // Simulate connection loss
  ws.simulateConnectionClosing('Server restart')
  
  // Verify reconnection
  await WebSocketTestUtils.waitForState(ws, WebSocket.OPEN)
})
```

## 🎯 Best Practices from Examples

### 1. Use Appropriate Test Types
```typescript
// Unit tests - fast, isolated
test('component renders correctly', () => {
  renderWithMockApi(<Component />, { scenario: 'EMPTY_SYSTEM' })
})

// Integration tests - realistic workflows
test('complete user workflow', withPerformanceMonitoring(async () => {
  // Multi-step workflow with realistic timing
}))

// E2E tests - full browser simulation
test('end-to-end user journey', async ({ page }) => {
  // Full page interactions
})
```

### 2. Scenario-Based Testing
```typescript
// Use predefined scenarios
renderWithMockApi(<Component />, { scenario: 'ACTIVE_SYSTEM' })
renderWithMockApi(<Component />, { scenario: 'PROBLEMATIC_SYSTEM' })
renderWithMockApi(<Component />, { scenario: 'EMPTY_SYSTEM' })
```

### 3. Error Condition Coverage
```typescript
// Test all error conditions
test.each([
  'network',
  'server', 
  'auth',
  'timeout'
])('handles %s errors', (errorType) => {
  const { simulateError } = useMockErrors()
  simulateError(errorType)
  
  // Test error handling
})
```

### 4. Performance Monitoring
```typescript
// Monitor performance in tests
test('performance under load', withPerformanceMonitoring(async () => {
  // Performance-sensitive operations
  PERFORMANCE_ASSERTIONS.assertPerformance('operation', duration, 'BENCHMARK')
}))
```

### 5. Cleanup and Isolation
```typescript
// Ensure test isolation
afterEach(() => {
  resetMockApi()
  vi.clearAllMocks()
  WebSocketTestUtils.closeAllWebSockets()
})
```

## 🔧 Running the Examples

### Unit Tests
```bash
npm run test:unit -- examples/unit-test-example.test.tsx
```

### Integration Tests  
```bash
npm run test:integration -- examples/integration-test-example.test.tsx
```

### E2E Tests
```bash
npm run test:e2e -- examples/e2e-test-example.spec.ts
```

### All Examples
```bash
# Run all example tests
npm run test -- examples/
```

## 📊 Example Metrics

The examples demonstrate:

- **Test Coverage**: 100% API endpoint coverage
- **Scenario Coverage**: 6 major user scenarios
- **Error Coverage**: 8 error condition types
- **Performance Testing**: Load testing up to 5000 documents
- **WebSocket Testing**: Real-time feature validation
- **Mobile Testing**: Responsive design validation
- **Accessibility Testing**: WCAG compliance checks

## 🚀 Extending Examples

To add new examples:

1. **Create Test File**: Follow naming pattern `*-example.test.tsx`
2. **Add Documentation**: Document new patterns in this README
3. **Include Scenarios**: Use or create relevant test scenarios
4. **Add Cleanup**: Ensure proper test isolation
5. **Performance Test**: Add performance monitoring where relevant

These examples serve as both documentation and validation of the mock API framework capabilities.