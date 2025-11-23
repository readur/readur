/**
 * Configuration utilities for mock API responses
 * Provides consistent configuration patterns across all mock handlers
 */

import { MockConfig, MockScenario, MockApiError } from '../api/types'

/**
 * Default mock configuration
 */
export const DEFAULT_MOCK_CONFIG: MockConfig = {
  delay: 0,
  shouldFail: false,
  errorCode: 500,
  errorMessage: 'Internal Server Error',
}

/**
 * Common delay configurations
 */
export const DELAYS = {
  INSTANT: 0,
  FAST: 100,
  REALISTIC: 300,
  SLOW: 1000,
  VERY_SLOW: 3000,
  NETWORK_TIMEOUT: 10000,
  INFINITE: 'infinite' as const,
}

/**
 * Common HTTP error codes and messages
 */
export const HTTP_ERRORS = {
  BAD_REQUEST: { code: 400, message: 'Bad Request' },
  UNAUTHORIZED: { code: 401, message: 'Unauthorized' },
  FORBIDDEN: { code: 403, message: 'Forbidden' },
  NOT_FOUND: { code: 404, message: 'Not Found' },
  CONFLICT: { code: 409, message: 'Conflict' },
  UNPROCESSABLE_ENTITY: { code: 422, message: 'Unprocessable Entity' },
  TOO_MANY_REQUESTS: { code: 429, message: 'Too Many Requests' },
  INTERNAL_SERVER_ERROR: { code: 500, message: 'Internal Server Error' },
  BAD_GATEWAY: { code: 502, message: 'Bad Gateway' },
  SERVICE_UNAVAILABLE: { code: 503, message: 'Service Unavailable' },
  GATEWAY_TIMEOUT: { code: 504, message: 'Gateway Timeout' },
}

/**
 * Create a mock configuration with defaults
 */
export const createMockConfig = (overrides: Partial<MockConfig> = {}): MockConfig => ({
  ...DEFAULT_MOCK_CONFIG,
  ...overrides,
})

/**
 * Create an error configuration
 */
export const createErrorConfig = (
  errorType: keyof typeof HTTP_ERRORS,
  customMessage?: string,
  delay?: number
): MockConfig => {
  const error = HTTP_ERRORS[errorType]
  return createMockConfig({
    shouldFail: true,
    errorCode: error.code,
    errorMessage: customMessage || error.message,
    delay: delay ?? DELAYS.REALISTIC,
  })
}

/**
 * Create a delay configuration
 */
export const createDelayConfig = (delay: number | 'infinite'): MockConfig => ({
  ...DEFAULT_MOCK_CONFIG,
  delay,
})

/**
 * Network simulation configurations
 */
export const NETWORK_CONDITIONS = {
  FAST: createDelayConfig(DELAYS.FAST),
  REALISTIC: createDelayConfig(DELAYS.REALISTIC),
  SLOW: createDelayConfig(DELAYS.SLOW),
  VERY_SLOW: createDelayConfig(DELAYS.VERY_SLOW),
  TIMEOUT: createDelayConfig(DELAYS.INFINITE),
  OFFLINE: createErrorConfig('SERVICE_UNAVAILABLE', 'Network Error'),
}

/**
 * Predefined scenarios for common testing situations
 */
export const COMMON_SCENARIOS: Record<string, MockScenario> = {
  SUCCESS_FAST: {
    name: 'Success (Fast)',
    description: 'Successful response with minimal delay',
    config: NETWORK_CONDITIONS.FAST,
  },
  SUCCESS_REALISTIC: {
    name: 'Success (Realistic)',
    description: 'Successful response with realistic network delay',
    config: NETWORK_CONDITIONS.REALISTIC,
  },
  SUCCESS_SLOW: {
    name: 'Success (Slow)',
    description: 'Successful response with slow network conditions',
    config: NETWORK_CONDITIONS.SLOW,
  },
  ERROR_UNAUTHORIZED: {
    name: 'Unauthorized Error',
    description: 'Authentication required error',
    config: createErrorConfig('UNAUTHORIZED'),
  },
  ERROR_FORBIDDEN: {
    name: 'Forbidden Error',
    description: 'Access denied error',
    config: createErrorConfig('FORBIDDEN'),
  },
  ERROR_NOT_FOUND: {
    name: 'Not Found Error',
    description: 'Resource not found error',
    config: createErrorConfig('NOT_FOUND'),
  },
  ERROR_SERVER: {
    name: 'Server Error',
    description: 'Internal server error',
    config: createErrorConfig('INTERNAL_SERVER_ERROR'),
  },
  ERROR_NETWORK: {
    name: 'Network Error',
    description: 'Network connectivity error',
    config: NETWORK_CONDITIONS.OFFLINE,
  },
  TIMEOUT: {
    name: 'Request Timeout',
    description: 'Request that never completes',
    config: NETWORK_CONDITIONS.TIMEOUT,
  },
}

/**
 * Create a mock API error response
 */
export const createMockError = (
  code: number = 500,
  message: string = 'Internal Server Error',
  details?: any
): MockApiError => ({
  code,
  message,
  details,
  timestamp: new Date().toISOString(),
})

/**
 * Apply delay to a response based on configuration
 */
export const applyDelay = async (config: MockConfig): Promise<void> => {
  if (config.delay === 'infinite') {
    // Never resolve - simulates timeout
    return new Promise(() => {})
  }
  if (typeof config.delay === 'number' && config.delay > 0) {
    await new Promise(resolve => setTimeout(resolve, config.delay))
  }
}

/**
 * Check if a request should fail based on configuration
 */
export const shouldFail = (config: MockConfig): boolean => {
  return config.shouldFail === true
}

/**
 * Generate a mock response with proper headers and status
 */
export const createMockResponse = <T>(
  data: T,
  config: MockConfig = DEFAULT_MOCK_CONFIG
) => {
  const status = shouldFail(config) ? config.errorCode! : 200
  const statusText = shouldFail(config) ? config.errorMessage! : 'OK'
  
  return {
    data: shouldFail(config) ? createMockError(config.errorCode!, config.errorMessage!) : data,
    status,
    statusText,
    headers: {
      'Content-Type': 'application/json',
      'X-Mock-Response': 'true',
      'X-Mock-Scenario': config.customResponse ? 'custom' : 'default',
    },
    _mockConfig: config,
  }
}

/**
 * Utility to randomly select a scenario (useful for chaos testing)
 */
export const getRandomScenario = (scenarios: MockScenario[]): MockScenario => {
  const index = Math.floor(Math.random() * scenarios.length)
  return scenarios[index]
}

/**
 * Utility to apply scenario configuration to handlers
 */
export const withScenario = (scenarioName: string, baseConfig: MockConfig = {}): MockConfig => {
  const scenario = COMMON_SCENARIOS[scenarioName]
  if (!scenario) {
    console.warn(`Unknown scenario: ${scenarioName}. Using default config.`)
    return baseConfig
  }
  
  return {
    ...baseConfig,
    ...scenario.config,
  }
}