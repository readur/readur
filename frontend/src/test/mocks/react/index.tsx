/**
 * React Mock Testing Framework - Main Export
 * Comprehensive React-specific mock utilities and testing components for modern testing
 */

// Core providers for mock setup
export * from './providers'

// Custom hooks for test scenarios
export * from './hooks'

// Test components and utilities
export * from './components'

// Enhanced render utilities
export * from './render'

// Testing patterns and orchestrators
export * from './patterns'

// Quick setup utilities and convenience functions
import { renderWithMocks, type EnhancedRenderOptions } from './render'
import { ComprehensiveMockProvider } from './providers'
import { TestingPlayground } from './components'
import { useMockTestingKit } from './hooks'
import { testSuites, TestPatternBuilder } from './patterns'

/**
 * Quick Setup Functions - Common testing scenarios made easy
 */

// Basic component testing
export const quickRender = (ui: React.ReactElement, options: Partial<EnhancedRenderOptions> = {}) =>
  renderWithMocks(ui, {
    testing: { enableDebugPanels: false },
    ...options,
  })

// Component testing with full mock environment
export const fullMockRender = (ui: React.ReactElement, options: Partial<EnhancedRenderOptions> = {}) =>
  renderWithMocks(ui, {
    testing: {
      enableErrorBoundary: true,
      enableSuspenseBoundary: true,
      enableNotifications: true,
      isolateComponent: true,
    },
    ...options,
  })

// Testing with authentication
export const authenticatedRender = (ui: React.ReactElement, userType: 'user' | 'admin' = 'user') =>
  renderWithMocks(ui, {
    mockAuth: { autoLogin: true, initialUser: userType },
    testing: { enableDebugPanels: false },
  })

// Performance-focused testing
export const performanceRender = (ui: React.ReactElement) =>
  renderWithMocks(ui, {
    testing: {
      enablePerformanceMonitoring: true,
      enableDebugPanels: false,
      enableErrorBoundary: false,
      enableSuspenseBoundary: false,
    },
  })

// Error testing setup
export const errorTestRender = (ui: React.ReactElement) =>
  renderWithMocks(ui, {
    testing: {
      enableErrorBoundary: true,
      enableDebugPanels: true,
      isolateComponent: true,
    },
  })

/**
 * Test Environment Providers - Wrap entire test suites
 */
export const TestEnvironment: React.FC<{
  children: React.ReactNode
  scenario?: string
  enableDebug?: boolean
}> = ({ children, scenario = 'ACTIVE_SYSTEM', enableDebug = process.env.NODE_ENV === 'development' }) => (
  <ComprehensiveMockProvider
    api={{ scenario }}
    auth={{ autoLogin: true, initialUser: 'user' }}
  >
    <TestingPlayground
      enableErrorTesting={true}
      enableSuspenseTesting={true}
      enableNotifications={true}
      showDebugPanels={enableDebug}
    >
      {children}
    </TestingPlayground>
  </ComprehensiveMockProvider>
)

/**
 * Common Testing Workflows
 */
export const commonWorkflows = {
  // Quick component validation
  validateComponent: async (ui: React.ReactElement) => {
    const result = quickRender(ui)
    const accessibilityReport = result.getAccessibilityReport()
    const performanceMetrics = result.getPerformanceMetrics()
    
    return {
      rendered: true,
      accessibilityScore: accessibilityReport.score,
      renderTime: performanceMetrics.renderTime,
      issues: accessibilityReport.issues,
    }
  },

  // Form testing workflow
  testForm: async (ui: React.ReactElement, formConfig: any) => {
    const result = fullMockRender(ui)
    return await testSuites.form(result, formConfig)
  },

  // Interactive component workflow
  testInteractiveComponent: async (ui: React.ReactElement) => {
    const result = fullMockRender(ui)
    return await testSuites.interactive(result)
  },

  // Performance validation workflow
  validatePerformance: async (ui: React.ReactElement) => {
    const result = performanceRender(ui)
    return await testSuites.basic(result)
  },

  // Critical component validation (comprehensive)
  validateCriticalComponent: async (ui: React.ReactElement) => {
    const result = fullMockRender(ui)
    return await testSuites.critical(result)
  },
}

/**
 * Developer Experience Utilities
 */
export const devUtils = {
  // Component inspector for debugging
  inspectComponent: (ui: React.ReactElement) => {
    const result = renderWithMocks(ui, {
      testing: { 
        enableDebugPanels: true,
        enablePerformanceMonitoring: true,
        isolateComponent: true,
      },
    })

    return {
      result,
      debug: () => {
        console.group('🔍 Component Inspection')
        console.log('Accessibility Report:', result.getAccessibilityReport())
        console.log('Performance Metrics:', result.getPerformanceMetrics())
        console.log('Mock State:', result.getCurrentMockState())
        console.groupEnd()
      },
      performance: () => result.getPerformanceMetrics(),
      accessibility: () => result.getAccessibilityReport(),
      mockState: () => result.getCurrentMockState(),
    }
  },

  // Rapid prototyping with mock data
  prototypeWithMocks: (ui: React.ReactElement, scenario: string = 'ACTIVE_SYSTEM') =>
    renderWithMocks(ui, {
      mockApi: { scenario },
      mockAuth: { autoLogin: true, initialUser: 'user' },
      testing: { enableDebugPanels: true },
    }),

  // A/B testing setup
  compareVariants: (variants: Array<{ name: string; component: React.ReactElement }>) => {
    const results = variants.map(({ name, component }) => ({
      name,
      result: quickRender(component),
    }))

    return {
      results,
      compare: (selector: string) =>
        results.map(({ name, result }) => ({
          name,
          element: result.container.querySelector(selector),
        })),
      performance: () =>
        results.map(({ name, result }) => ({
          name,
          metrics: result.getPerformanceMetrics(),
        })),
    }
  },
}

/**
 * Testing Utilities Export
 */
export const testUtils = {
  // Pattern builders
  createTestPattern: () => TestPatternBuilder.create(),
  
  // Quick patterns
  formPattern: (fields: any[]) => TestPatternBuilder.create().withForm({
    fields,
    enableAccessibilityChecks: true,
  }),
  
  asyncPattern: (configs: any[]) => TestPatternBuilder.create().withAsync(configs),
  
  errorPattern: (configs: any[]) => TestPatternBuilder.create().withError(configs),
  
  // Hook for accessing mock utilities in tests
  useMockUtils: useMockTestingKit,
  
  // Test suites
  suites: testSuites,
  
  // Common workflows
  workflows: commonWorkflows,
}

/**
 * TypeScript Support Exports
 */
export type {
  // Render types
  EnhancedRenderOptions,
  EnhancedRenderResult,
  
  // Provider types
  MockApiProviderProps,
  MockAuthProviderProps,
  MockNotificationProviderProps,
  MockWebSocketProviderProps,
  
  // Hook types
  UseMockScenarioReturn,
  UseMockNetworkConditionReturn,
  UseMockUserReturn,
  UseMockDocumentsReturn,
  UseMockSearchReturn,
  UseMockUploadReturn,
  
  // Component types
  TestErrorBoundaryProps,
  TestSuspenseBoundaryProps,
  MockFileDropzoneProps,
  MockSearchResultsProps,
  MockNotificationStackProps,
  
  // Pattern types
  FormTestConfig,
  AsyncOperationConfig,
  ErrorTestConfig,
  PerformanceTestConfig,
  ComprehensiveTestConfig,
} from './providers'

// Re-export testing library utilities for convenience
export { 
  screen, 
  fireEvent, 
  waitFor, 
  act,
  within,
  createEvent,
  getByRole,
  queryByRole,
  findByRole,
} from '@testing-library/react'

export { default as userEvent } from '@testing-library/user-event'

/**
 * Default export for easy importing
 */
const ReactMockFramework = {
  // Core functions
  render: renderWithMocks,
  quickRender,
  fullMockRender,
  authenticatedRender,
  performanceRender,
  errorTestRender,
  
  // Providers
  TestEnvironment,
  ComprehensiveMockProvider,
  TestingPlayground,
  
  // Utilities
  testUtils,
  devUtils,
  commonWorkflows,
  
  // Test suites
  testSuites,
  
  // Pattern builder
  TestPatternBuilder,
}

export default ReactMockFramework