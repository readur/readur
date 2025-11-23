// Integration test setup file
// Configures mock API for integration testing scenarios

import '@testing-library/jest-dom'
import { vi, beforeEach, afterEach, beforeAll, afterAll } from 'vitest'
import { setupTestEnvironment } from './test-utils.tsx'
import { 
  server, 
  resetMockApi, 
  enableWebSocketMocking, 
  disableWebSocketMocking,
  setDocumentMockConfig,
  setAuthMockConfig,
  setSearchMockConfig,
  NETWORK_CONDITIONS,
} from './mocks'

// Setup global test environment
setupTestEnvironment()

// Setup MSW server for integration testing
beforeAll(() => {
  // Start the mock server
  server.listen({
    onUnhandledRequest: 'warn',
  })
  
  // Configure realistic network conditions for integration tests
  const realisticConfig = NETWORK_CONDITIONS.REALISTIC
  setDocumentMockConfig(realisticConfig)
  setAuthMockConfig(realisticConfig)
  setSearchMockConfig(realisticConfig)
  
  // Enable WebSocket mocking with realistic settings
  enableWebSocketMocking({
    autoConnect: true,
    messageDelay: 100, // More realistic for integration tests
    heartbeatInterval: 5000,
    simulateReconnects: true,
    maxReconnects: 3,
  })
})

// Reset handlers between tests but maintain realistic timing
beforeEach(() => {
  vi.resetAllMocks()
  resetMockApi()
  
  // Restore realistic network conditions
  const realisticConfig = NETWORK_CONDITIONS.REALISTIC
  setDocumentMockConfig(realisticConfig)
  setAuthMockConfig(realisticConfig)
  setSearchMockConfig(realisticConfig)
})

// Clean up after each test
afterEach(() => {
  vi.clearAllMocks()
})

// Clean up after all tests
afterAll(() => {
  server.close()
  disableWebSocketMocking()
})