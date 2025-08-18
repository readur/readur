/**
 * Central export for the entire mock API framework
 * Provides a single entry point for all mocking functionality
 */

// Core API mocking
export { 
  server, 
  startMockApi, 
  stopMockApi, 
  resetMockApi, 
  useMockHandlers 
} from './api/server'

// Mock handlers
export { handlers } from './handlers'
export * from './handlers/documents'
export * from './handlers/auth'
export * from './handlers/search'
export * from './handlers/queue'
export * from './handlers/sources'
export * from './handlers/labels'
export * from './handlers/users'
export * from './handlers/ocr'
export * from './handlers/settings'

// Data factories
export * from './factories'

// Test fixtures and scenarios
export * from './fixtures'

// Utilities
export * from './utils'

// Types
export * from './api/types'

// React-specific utilities
export * from './react'

/**
 * Original setup function for full configuration
 */
export const setupMockApi = async (config: {
  scenario?: string
  networkCondition?: 'fast' | 'slow' | 'offline'
  enableWebSocket?: boolean
} = {}) => {
  const { scenario = 'ACTIVE_SYSTEM', networkCondition = 'fast', enableWebSocket = true } = config

  // Start the mock server
  await startMockApi('node')

  // Apply network conditions
  const { setDocumentMockConfig, setAuthMockConfig, setSearchMockConfig } = await import('./handlers/documents')
  
  let networkConfig = { delay: 100, shouldFail: false }
  switch (networkCondition) {
    case 'slow':
      networkConfig = { delay: 1000, shouldFail: false }
      break
    case 'offline':
      networkConfig = { delay: 0, shouldFail: true, errorCode: 0, errorMessage: 'Network Error' }
      break
  }

  setDocumentMockConfig(networkConfig)
  setAuthMockConfig(networkConfig)
  setSearchMockConfig(networkConfig)

  // Apply scenario if specified
  if (scenario) {
    const { getTestScenario } = await import('./fixtures/scenarios')
    const { setMockDocuments } = await import('./handlers/documents')
    const { setCurrentUser } = await import('./handlers/auth')
    
    const scenarioData = getTestScenario(scenario)
    if (scenarioData.documents) setMockDocuments(scenarioData.documents)
    if (scenarioData.users && scenarioData.users.length > 0) setCurrentUser(scenarioData.users[0])
  }

  // Enable WebSocket mocking if requested
  if (enableWebSocket) {
    const { enableWebSocketMocking } = await import('./utils/websocket')
    enableWebSocketMocking({ autoConnect: true, messageDelay: 50 })
  }

  return {
    cleanup: () => {
      stopMockApi('node')
      if (enableWebSocket) {
        import('./utils/websocket').then(({ disableWebSocketMocking, WebSocketTestUtils }) => {
          disableWebSocketMocking()
          WebSocketTestUtils.closeAllWebSockets()
        })
      }
    }
  }
}

/**
 * Quick setup for browser environment (E2E tests)
 */
export const setupMockApiBrowser = async (config: {
  scenario?: string
  networkCondition?: 'fast' | 'slow' | 'offline'
} = {}) => {
  const { scenario = 'ACTIVE_SYSTEM', networkCondition = 'fast' } = config

  // Start the mock worker
  await startMockApi('browser')

  // Apply configuration (similar to setupMockApi but for browser)
  return {
    cleanup: () => {
      stopMockApi('browser')
    }
  }
}

/**
 * Easy API for quick mock setup - One line configuration for common scenarios
 */
export const mockApi = {
  /**
   * Quick setup for tests with default scenarios
   * @example await mockApi.quick('ACTIVE_SYSTEM') 
   */
  quick: (scenario: string = 'ACTIVE_SYSTEM') => setupMockApi({ scenario }),
  
  /**
   * Quick setup for authenticated tests
   * @example await mockApi.withAuth()
   */
  withAuth: (userType: 'user' | 'admin' = 'user') => 
    setupMockApi({ scenario: userType === 'admin' ? 'ADMIN_USER' : 'ACTIVE_SYSTEM' }),
  
  /**
   * Quick setup for empty system tests
   * @example await mockApi.empty()
   */
  empty: () => setupMockApi({ scenario: 'EMPTY_SYSTEM' }),
  
  /**
   * Quick setup for slow network tests
   * @example await mockApi.slow()
   */
  slow: () => setupMockApi({ networkCondition: 'slow' }),
  
  /**
   * Quick setup for offline tests
   * @example await mockApi.offline()
   */
  offline: () => setupMockApi({ networkCondition: 'offline' }),

  /**
   * Full configuration setup
   * @example mockApi.custom({ scenario: 'MULTI_USER_SYSTEM', networkCondition: 'slow' })
   */
  custom: setupMockApi
}

/**
 * React Testing Framework - Easy access to React-specific utilities
 */
export { default as ReactMocks } from './react'

// Quick React setup functions
export {
  quickRender,
  fullMockRender,
  authenticatedRender,
  performanceRender,
  errorTestRender,
  TestEnvironment,
  testUtils,
  devUtils,
  commonWorkflows,
} from './react'