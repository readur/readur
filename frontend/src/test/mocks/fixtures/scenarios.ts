/**
 * Predefined test scenarios for common testing situations
 * Each scenario provides a complete, consistent test environment
 */

import { MockState, MockScenario } from '../api/types'
import { generateScenarioDataset, createCompleteTestDataset } from '../factories'

/**
 * Complete test scenarios with all necessary data
 */
export const TEST_SCENARIOS: Record<string, MockState> = {
  // Empty system - for testing initial states
  EMPTY_SYSTEM: generateScenarioDataset('empty_system') as MockState,
  
  // New user setup - for onboarding tests
  NEW_USER_SETUP: generateScenarioDataset('new_user_setup') as MockState,
  
  // Active system - for normal operation tests
  ACTIVE_SYSTEM: generateScenarioDataset('active_system') as MockState,
  
  // System under load - for performance and stress tests
  SYSTEM_UNDER_LOAD: generateScenarioDataset('system_under_load') as MockState,
  
  // Multi-user system - for permission and isolation tests
  MULTI_USER_SYSTEM: generateScenarioDataset('multi_user_system') as MockState,
  
  // Problematic system - for error handling tests
  PROBLEMATIC_SYSTEM: generateScenarioDataset('problematic_system') as MockState,
}

/**
 * UI-specific test scenarios
 */
export const UI_TEST_SCENARIOS = {
  // Dashboard scenarios
  DASHBOARD_EMPTY: {
    name: 'Empty Dashboard',
    description: 'Dashboard with no documents or recent activity',
    data: TEST_SCENARIOS.EMPTY_SYSTEM,
  },
  
  DASHBOARD_WITH_RECENT_UPLOADS: {
    name: 'Dashboard with Recent Uploads',
    description: 'Dashboard showing recent document uploads and processing',
    data: TEST_SCENARIOS.ACTIVE_SYSTEM,
  },
  
  DASHBOARD_WITH_ERRORS: {
    name: 'Dashboard with Errors',
    description: 'Dashboard showing failed OCR and sync errors',
    data: TEST_SCENARIOS.PROBLEMATIC_SYSTEM,
  },

  // Search scenarios
  SEARCH_NO_RESULTS: {
    name: 'Search No Results',
    description: 'Search that returns no documents',
    searchQuery: 'nonexistent document',
    expectedResults: 0,
  },
  
  SEARCH_MANY_RESULTS: {
    name: 'Search Many Results',
    description: 'Search that returns many documents requiring pagination',
    searchQuery: 'document',
    expectedResults: 100,
  },
  
  SEARCH_WITH_FILTERS: {
    name: 'Search with Filters',
    description: 'Search with MIME type and tag filters applied',
    searchQuery: 'invoice',
    filters: {
      mime_types: ['application/pdf'],
      tags: ['important']
    },
    expectedResults: 5,
  },

  // Document management scenarios
  DOCUMENT_LIST_EMPTY: {
    name: 'Empty Document List',
    description: 'Document list with no documents',
    data: TEST_SCENARIOS.EMPTY_SYSTEM,
  },
  
  DOCUMENT_LIST_WITH_FAILED_OCR: {
    name: 'Document List with Failed OCR',
    description: 'Document list showing failed OCR documents',
    data: TEST_SCENARIOS.PROBLEMATIC_SYSTEM,
  },
  
  DOCUMENT_UPLOAD_IN_PROGRESS: {
    name: 'Document Upload in Progress',
    description: 'Documents being uploaded and processed',
    data: TEST_SCENARIOS.ACTIVE_SYSTEM,
  },

  // Source management scenarios
  SOURCES_MULTIPLE_TYPES: {
    name: 'Multiple Source Types',
    description: 'Sources including local, WebDAV, and S3 configurations',
    data: TEST_SCENARIOS.MULTI_USER_SYSTEM,
  },
  
  SOURCES_SYNC_IN_PROGRESS: {
    name: 'Sources Syncing',
    description: 'Sources actively syncing with progress indicators',
    data: TEST_SCENARIOS.ACTIVE_SYSTEM,
  },
  
  SOURCES_WITH_ERRORS: {
    name: 'Sources with Errors',
    description: 'Sources showing connection and sync errors',
    data: TEST_SCENARIOS.PROBLEMATIC_SYSTEM,
  },

  // Settings scenarios
  SETTINGS_DEFAULT: {
    name: 'Default Settings',
    description: 'Application with default settings configuration',
    settings: {
      ocr_enabled: true,
      auto_ocr: true,
      max_file_size_mb: 100,
      notification_enabled: true,
    },
  },
  
  SETTINGS_OCR_DISABLED: {
    name: 'OCR Disabled',
    description: 'Application with OCR processing disabled',
    settings: {
      ocr_enabled: false,
      auto_ocr: false,
      max_file_size_mb: 100,
      notification_enabled: true,
    },
  },
}

/**
 * Authentication test scenarios
 */
export const AUTH_TEST_SCENARIOS = {
  LOGGED_OUT: {
    name: 'Logged Out User',
    description: 'User not authenticated, should see login screen',
    authenticated: false,
    user: null,
  },
  
  REGULAR_USER: {
    name: 'Regular User',
    description: 'Normal user with standard permissions',
    authenticated: true,
    user: {
      id: 'user-1',
      username: 'testuser',
      email: 'test@example.com',
      role: 'user',
    },
  },
  
  ADMIN_USER: {
    name: 'Admin User',
    description: 'Administrator with elevated permissions',
    authenticated: true,
    user: {
      id: 'admin-1',
      username: 'admin',
      email: 'admin@example.com',
      role: 'admin',
    },
  },
  
  OIDC_USER: {
    name: 'OIDC User',
    description: 'User authenticated via OpenID Connect',
    authenticated: true,
    user: {
      id: 'oidc-1',
      username: 'oidc_user',
      email: 'oidc@example.com',
      role: 'user',
      oidc_sub: 'oidc|123456789',
    },
  },
}

/**
 * Performance test scenarios
 */
export const PERFORMANCE_TEST_SCENARIOS = {
  LARGE_DOCUMENT_SET: {
    name: 'Large Document Set',
    description: 'System with thousands of documents for performance testing',
    data: TEST_SCENARIOS.SYSTEM_UNDER_LOAD,
    metrics: {
      document_count: 1000,
      expected_search_time_ms: 500,
      expected_list_time_ms: 200,
    },
  },
  
  HEAVY_OCR_QUEUE: {
    name: 'Heavy OCR Queue',
    description: 'System with large OCR processing queue',
    data: TEST_SCENARIOS.SYSTEM_UNDER_LOAD,
    metrics: {
      queue_size: 200,
      processing_rate: 5, // docs per minute
    },
  },
  
  MULTIPLE_ACTIVE_SYNCS: {
    name: 'Multiple Active Syncs',
    description: 'Multiple sources syncing simultaneously',
    data: TEST_SCENARIOS.MULTI_USER_SYSTEM,
    metrics: {
      active_syncs: 5,
      sync_progress_update_rate: 1000, // ms
    },
  },
}

/**
 * Error condition test scenarios
 */
export const ERROR_TEST_SCENARIOS = {
  NETWORK_OFFLINE: {
    name: 'Network Offline',
    description: 'All API requests fail with network errors',
    networkCondition: 'offline',
    expectedBehavior: 'show_offline_message',
  },
  
  SERVER_ERROR: {
    name: 'Server Error',
    description: 'API returns 500 internal server errors',
    networkCondition: 'server_error',
    expectedBehavior: 'show_error_message',
  },
  
  AUTHENTICATION_EXPIRED: {
    name: 'Authentication Expired',
    description: 'User session has expired, requires re-authentication',
    networkCondition: 'auth_expired',
    expectedBehavior: 'redirect_to_login',
  },
  
  RATE_LIMITED: {
    name: 'Rate Limited',
    description: 'API requests are being rate limited',
    networkCondition: 'rate_limited',
    expectedBehavior: 'show_rate_limit_message',
  },
}

/**
 * Integration test scenarios
 */
export const INTEGRATION_TEST_SCENARIOS = {
  FULL_DOCUMENT_LIFECYCLE: {
    name: 'Full Document Lifecycle',
    description: 'Complete flow from upload to OCR to search',
    steps: [
      'upload_document',
      'wait_for_ocr',
      'verify_searchable',
      'apply_labels',
      'search_and_find',
    ],
  },
  
  SOURCE_SYNC_WORKFLOW: {
    name: 'Source Sync Workflow',
    description: 'Complete source creation and sync process',
    steps: [
      'create_source',
      'test_connection',
      'trigger_sync',
      'monitor_progress',
      'verify_documents_synced',
    ],
  },
  
  USER_ONBOARDING: {
    name: 'User Onboarding',
    description: 'New user registration and setup process',
    steps: [
      'register_user',
      'verify_email',
      'setup_profile',
      'create_first_source',
      'upload_first_document',
    ],
  },
}

/**
 * Get a specific test scenario by name
 */
export const getTestScenario = (scenarioName: string): MockState => {
  const scenario = TEST_SCENARIOS[scenarioName]
  if (!scenario) {
    console.warn(`Unknown test scenario: ${scenarioName}`)
    return createCompleteTestDataset()
  }
  return scenario
}

/**
 * Get UI test scenario by name
 */
export const getUITestScenario = (scenarioName: string) => {
  const scenario = UI_TEST_SCENARIOS[scenarioName]
  if (!scenario) {
    console.warn(`Unknown UI test scenario: ${scenarioName}`)
    return UI_TEST_SCENARIOS.DASHBOARD_EMPTY
  }
  return scenario
}

/**
 * List all available test scenarios
 */
export const listTestScenarios = () => ({
  data_scenarios: Object.keys(TEST_SCENARIOS),
  ui_scenarios: Object.keys(UI_TEST_SCENARIOS),
  auth_scenarios: Object.keys(AUTH_TEST_SCENARIOS),
  performance_scenarios: Object.keys(PERFORMANCE_TEST_SCENARIOS),
  error_scenarios: Object.keys(ERROR_TEST_SCENARIOS),
  integration_scenarios: Object.keys(INTEGRATION_TEST_SCENARIOS),
})