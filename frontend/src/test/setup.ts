// Global test setup file for Vitest
// This file is automatically loaded before all tests

import '@testing-library/jest-dom'
import { vi, beforeAll, beforeEach, afterEach, afterAll } from 'vitest'
import { setupTestEnvironment } from './test-utils.tsx'
import { server } from './mocks/api/server'

// Setup global test environment
setupTestEnvironment()

// Setup MSW server for all tests
beforeAll(() => {
  server.listen({ onUnhandledRequest: 'warn' })
})

// Reset handlers between tests to ensure test isolation
beforeEach(() => {
  vi.resetAllMocks()
  server.resetHandlers()
})

// Clean up after each test
afterEach(() => {
  vi.clearAllMocks()
})

// Clean up after all tests
afterAll(() => {
  server.close()
})