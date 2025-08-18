/**
 * MSW Server configuration for API mocking
 * Provides centralized mock server setup for both Node.js and browser environments
 */

import { setupServer } from 'msw/node'
import { handlers } from '../handlers'

// Server for Node.js environment (unit/integration tests)
const server = setupServer(...handlers)

// Worker for browser environment (E2E tests) - lazy loaded only when needed
let worker: any = null

/**
 * Start the mock server based on environment
 * @param environment - 'node' for unit/integration tests, 'browser' for E2E tests
 */
export const startMockApi = async (environment: 'node' | 'browser' = 'node') => {
  if (environment === 'node') {
    server.listen({
      onUnhandledRequest: 'warn',
    })
  } else {
    // Lazy load the browser worker only when needed
    if (!worker) {
      const { setupWorker } = await import('msw/browser')
      worker = setupWorker(...handlers)
    }
    await worker.start({
      onUnhandledRequest: 'warn',
    })
  }
}

/**
 * Stop the mock server
 */
export const stopMockApi = (environment: 'node' | 'browser' = 'node') => {
  if (environment === 'node') {
    server.close()
  } else {
    worker.stop()
  }
}

/**
 * Reset all handlers to their initial state
 */
export const resetMockApi = () => {
  server.resetHandlers()
}

/**
 * Use runtime request handlers (for dynamic mocking in tests)
 */
export const useMockHandlers = (...newHandlers: any[]) => {
  server.use(...newHandlers)
}

export { server }