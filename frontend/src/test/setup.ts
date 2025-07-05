// Global test setup file for Vitest
// This file is automatically loaded before all tests

import '@testing-library/jest-dom'
import { vi } from 'vitest'
import { setupTestEnvironment } from './test-utils.tsx'

// Setup global test environment
setupTestEnvironment()

// Additional global setup can be added here
// For example:
// - Global error handlers
// - Test timeouts
// - Common test data
// - Global test utilities

// Increase test timeout for async operations
beforeEach(() => {
  vi.resetAllMocks()
})

// Clean up after each test
afterEach(() => {
  vi.clearAllMocks()
})