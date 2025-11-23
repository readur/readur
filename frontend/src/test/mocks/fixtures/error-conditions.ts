/**
 * Error condition fixtures for testing error handling and resilience
 * Provides comprehensive error scenarios for robust testing
 */

import { MockConfig } from '../api/types'
import { createErrorConfig, HTTP_ERRORS } from '../utils/config'

/**
 * Common error scenarios for API testing
 */
export const ERROR_SCENARIOS = {
  // Authentication errors
  UNAUTHORIZED: createErrorConfig('UNAUTHORIZED', 'Authentication required'),
  FORBIDDEN: createErrorConfig('FORBIDDEN', 'Access denied'),
  TOKEN_EXPIRED: createErrorConfig('UNAUTHORIZED', 'Token has expired'),
  
  // Client errors
  BAD_REQUEST: createErrorConfig('BAD_REQUEST', 'Invalid request data'),
  NOT_FOUND: createErrorConfig('NOT_FOUND', 'Resource not found'),
  CONFLICT: createErrorConfig('CONFLICT', 'Resource already exists'),
  UNPROCESSABLE_ENTITY: createErrorConfig('UNPROCESSABLE_ENTITY', 'Validation failed'),
  
  // Server errors
  INTERNAL_SERVER_ERROR: createErrorConfig('INTERNAL_SERVER_ERROR', 'Internal server error'),
  BAD_GATEWAY: createErrorConfig('BAD_GATEWAY', 'Upstream server error'),
  SERVICE_UNAVAILABLE: createErrorConfig('SERVICE_UNAVAILABLE', 'Service temporarily unavailable'),
  GATEWAY_TIMEOUT: createErrorConfig('GATEWAY_TIMEOUT', 'Request timeout'),
  
  // Rate limiting
  RATE_LIMITED: createErrorConfig('TOO_MANY_REQUESTS', 'Rate limit exceeded'),
  
  // Network errors
  NETWORK_ERROR: {
    delay: 0,
    shouldFail: true,
    errorCode: 0,
    errorMessage: 'Network Error',
  } as MockConfig,
  
  TIMEOUT: {
    delay: 'infinite' as const,
    shouldFail: false,
  } as MockConfig,
}

/**
 * Specific error conditions for different API endpoints
 */
export const API_ERROR_CONDITIONS = {
  // Document errors
  DOCUMENT_NOT_FOUND: {
    endpoint: '/api/documents/:id',
    scenario: ERROR_SCENARIOS.NOT_FOUND,
    description: 'Document does not exist',
  },
  
  DOCUMENT_UPLOAD_FAILED: {
    endpoint: '/api/documents',
    scenario: ERROR_SCENARIOS.UNPROCESSABLE_ENTITY,
    description: 'Document upload validation failed',
  },
  
  OCR_PROCESSING_ERROR: {
    endpoint: '/api/documents/:id/ocr',
    scenario: ERROR_SCENARIOS.INTERNAL_SERVER_ERROR,
    description: 'OCR processing failed',
  },
  
  // Search errors
  SEARCH_TIMEOUT: {
    endpoint: '/api/search',
    scenario: ERROR_SCENARIOS.GATEWAY_TIMEOUT,
    description: 'Search request timed out',
  },
  
  INVALID_SEARCH_QUERY: {
    endpoint: '/api/search',
    scenario: ERROR_SCENARIOS.BAD_REQUEST,
    description: 'Invalid search query syntax',
  },
  
  // Source errors
  SOURCE_CONNECTION_FAILED: {
    endpoint: '/api/sources/:id/test',
    scenario: ERROR_SCENARIOS.BAD_GATEWAY,
    description: 'Cannot connect to source',
  },
  
  SOURCE_SYNC_ERROR: {
    endpoint: '/api/sources/:id/sync',
    scenario: ERROR_SCENARIOS.INTERNAL_SERVER_ERROR,
    description: 'Source synchronization failed',
  },
  
  // Authentication errors
  LOGIN_FAILED: {
    endpoint: '/api/auth/login',
    scenario: ERROR_SCENARIOS.UNAUTHORIZED,
    description: 'Invalid credentials',
  },
  
  SESSION_EXPIRED: {
    endpoint: '/api/auth/me',
    scenario: ERROR_SCENARIOS.UNAUTHORIZED,
    description: 'Session has expired',
  },
  
  // Queue errors
  QUEUE_FULL: {
    endpoint: '/api/queue/stats',
    scenario: ERROR_SCENARIOS.SERVICE_UNAVAILABLE,
    description: 'Processing queue is full',
  },
}

/**
 * Progressive error scenarios that simulate degrading service
 */
export const PROGRESSIVE_ERROR_SCENARIOS = {
  GRADUAL_SLOWDOWN: [
    { delay: 100, shouldFail: false },
    { delay: 500, shouldFail: false },
    { delay: 1000, shouldFail: false },
    { delay: 2000, shouldFail: false },
    ERROR_SCENARIOS.TIMEOUT,
  ],
  
  INCREASING_FAILURES: [
    { delay: 200, shouldFail: false },
    { delay: 200, shouldFail: Math.random() > 0.9 }, // 10% failure
    { delay: 200, shouldFail: Math.random() > 0.7 }, // 30% failure
    { delay: 200, shouldFail: Math.random() > 0.5 }, // 50% failure
    { delay: 200, shouldFail: true }, // 100% failure
  ],
  
  INTERMITTENT_ISSUES: [
    { delay: 150, shouldFail: false },
    ERROR_SCENARIOS.INTERNAL_SERVER_ERROR,
    { delay: 150, shouldFail: false },
    { delay: 150, shouldFail: false },
    ERROR_SCENARIOS.GATEWAY_TIMEOUT,
    { delay: 150, shouldFail: false },
  ],
}

/**
 * Error recovery scenarios for testing resilience
 */
export const ERROR_RECOVERY_SCENARIOS = {
  TEMPORARY_OUTAGE: {
    phases: [
      { duration: 1000, config: { delay: 150, shouldFail: false } },
      { duration: 3000, config: ERROR_SCENARIOS.SERVICE_UNAVAILABLE },
      { duration: 1000, config: { delay: 150, shouldFail: false } },
    ],
    description: 'Temporary service outage with recovery',
  },
  
  FLAKY_SERVICE: {
    phases: [
      { duration: 2000, config: { delay: 150, shouldFail: Math.random() > 0.8 } },
      { duration: 1000, config: ERROR_SCENARIOS.INTERNAL_SERVER_ERROR },
      { duration: 2000, config: { delay: 150, shouldFail: false } },
    ],
    description: 'Intermittent service issues',
  },
  
  CASCADING_FAILURE: {
    phases: [
      { duration: 1000, config: { delay: 300, shouldFail: false } },
      { duration: 2000, config: ERROR_SCENARIOS.GATEWAY_TIMEOUT },
      { duration: 3000, config: ERROR_SCENARIOS.SERVICE_UNAVAILABLE },
      { duration: 2000, config: { delay: 800, shouldFail: false } },
      { duration: 1000, config: { delay: 200, shouldFail: false } },
    ],
    description: 'Cascading failure with gradual recovery',
  },
}

/**
 * Input validation error scenarios
 */
export const VALIDATION_ERROR_SCENARIOS = {
  EMPTY_REQUIRED_FIELD: {
    field: 'title',
    value: '',
    expectedError: 'Title is required',
  },
  
  INVALID_EMAIL: {
    field: 'email',
    value: 'invalid-email',
    expectedError: 'Please enter a valid email address',
  },
  
  PASSWORD_TOO_SHORT: {
    field: 'password',
    value: '123',
    expectedError: 'Password must be at least 8 characters',
  },
  
  INVALID_FILE_TYPE: {
    field: 'file',
    value: 'document.exe',
    expectedError: 'File type not allowed',
  },
  
  FILE_TOO_LARGE: {
    field: 'file',
    value: 'large-file.pdf',
    expectedError: 'File size exceeds maximum limit',
  },
  
  INVALID_URL: {
    field: 'webdav_url',
    value: 'not-a-url',
    expectedError: 'Please enter a valid URL',
  },
  
  INVALID_JSON: {
    field: 'metadata',
    value: '{"invalid": json}',
    expectedError: 'Invalid JSON format',
  },
}

/**
 * Business logic error scenarios
 */
export const BUSINESS_LOGIC_ERRORS = {
  DUPLICATE_DOCUMENT: {
    action: 'upload_document',
    scenario: ERROR_SCENARIOS.CONFLICT,
    description: 'Document with same hash already exists',
  },
  
  INSUFFICIENT_PERMISSIONS: {
    action: 'delete_document',
    scenario: ERROR_SCENARIOS.FORBIDDEN,
    description: 'User does not have permission to delete this document',
  },
  
  QUOTA_EXCEEDED: {
    action: 'upload_document',
    scenario: ERROR_SCENARIOS.UNPROCESSABLE_ENTITY,
    description: 'Storage quota exceeded',
  },
  
  INVALID_SOURCE_CONFIG: {
    action: 'create_source',
    scenario: ERROR_SCENARIOS.BAD_REQUEST,
    description: 'Source configuration is invalid',
  },
  
  SOURCE_ALREADY_SYNCING: {
    action: 'start_sync',
    scenario: ERROR_SCENARIOS.CONFLICT,
    description: 'Source is already being synchronized',
  },
}

/**
 * Error testing utilities
 */
export class ErrorTestUtils {
  private static errorLog: Array<{ timestamp: Date; error: any; context?: string }> = []
  
  /**
   * Log an error for testing analysis
   */
  static logError(error: any, context?: string) {
    this.errorLog.push({
      timestamp: new Date(),
      error,
      context,
    })
  }
  
  /**
   * Get all logged errors
   */
  static getErrorLog() {
    return [...this.errorLog]
  }
  
  /**
   * Clear error log
   */
  static clearErrorLog() {
    this.errorLog = []
  }
  
  /**
   * Simulate progressive error scenario
   */
  static createProgressiveErrorHandler(scenario: MockConfig[]) {
    let currentStep = 0
    
    return () => {
      const config = scenario[currentStep] || scenario[scenario.length - 1]
      currentStep = Math.min(currentStep + 1, scenario.length - 1)
      return config
    }
  }
  
  /**
   * Assert that specific error was handled correctly
   */
  static assertErrorHandled(expectedError: string, context?: string) {
    const recentErrors = this.errorLog
      .filter(entry => context ? entry.context === context : true)
      .slice(-5) // Last 5 errors
    
    const errorFound = recentErrors.some(entry => 
      entry.error.message?.includes(expectedError) || 
      JSON.stringify(entry.error).includes(expectedError)
    )
    
    if (!errorFound) {
      throw new Error(`Expected error '${expectedError}' was not handled properly`)
    }
  }
  
  /**
   * Create error boundary test component
   */
  static createErrorBoundary(onError?: (error: Error, errorInfo: any) => void) {
    return class ErrorBoundary extends Error {
      constructor(message: string) {
        super(message)
        if (onError) {
          onError(this, { componentStack: 'test component' })
        }
        ErrorTestUtils.logError(this, 'error-boundary')
      }
    }
  }
}

/**
 * Error condition test cases
 */
export const ERROR_TEST_CASES = [
  {
    name: 'Network Disconnection',
    setup: () => ERROR_SCENARIOS.NETWORK_ERROR,
    expectedBehavior: 'Show offline message',
    recoveryAction: 'Retry when online',
  },
  {
    name: 'Server Overload',
    setup: () => ERROR_SCENARIOS.SERVICE_UNAVAILABLE,
    expectedBehavior: 'Show service unavailable message',
    recoveryAction: 'Automatic retry with backoff',
  },
  {
    name: 'Authentication Failure',
    setup: () => ERROR_SCENARIOS.UNAUTHORIZED,
    expectedBehavior: 'Redirect to login page',
    recoveryAction: 'User re-authenticates',
  },
  {
    name: 'Request Timeout',
    setup: () => ERROR_SCENARIOS.TIMEOUT,
    expectedBehavior: 'Show timeout message',
    recoveryAction: 'Allow manual retry',
  },
  {
    name: 'Rate Limiting',
    setup: () => ERROR_SCENARIOS.RATE_LIMITED,
    expectedBehavior: 'Show rate limit message',
    recoveryAction: 'Automatic retry after delay',
  },
]