/**
 * Enhanced Render Utilities - Comprehensive test rendering with mock integration
 * Provides modern, minimalistic testing utilities with full mock API integration
 */

import React, { ReactElement, ReactNode, Suspense } from 'react'
import { render, RenderOptions, RenderResult } from '@testing-library/react'
import { BrowserRouter, MemoryRouter } from 'react-router-dom'
// import { QueryClient, QueryClientProvider } from '@tanstack/react-query' // Not used in this project
import { ComprehensiveMockProvider } from './providers'
import { TestErrorBoundary, TestSuspenseBoundary, TestingPlayground } from './components'
import { useMockTestingKit } from './hooks'
import type { 
  MockApiProviderProps, 
  MockAuthProviderProps, 
  MockNotificationProviderProps, 
  MockWebSocketProviderProps 
} from './providers'

// Re-export everything from @testing-library/react for convenience
export * from '@testing-library/react'

export interface EnhancedRenderOptions extends Omit<RenderOptions, 'wrapper'> {
  // Mock provider configurations
  mockApi?: Partial<MockApiProviderProps>
  mockAuth?: Partial<MockAuthProviderProps>
  mockNotifications?: Partial<MockNotificationProviderProps>
  mockWebSocket?: Partial<MockWebSocketProviderProps>

  // Router configuration
  router?: {
    type?: 'browser' | 'memory'
    initialEntries?: string[]
    initialIndex?: number
    routerProps?: any
  }

  // React Query configuration
  reactQuery?: {
    enabled?: boolean
    // client?: QueryClient // Not used
    defaultOptions?: any
  }

  // Testing environment
  testing?: {
    enableErrorBoundary?: boolean
    enableSuspenseBoundary?: boolean
    enableNotifications?: boolean
    enableDebugPanels?: boolean
    enablePerformanceMonitoring?: boolean
    isolateComponent?: boolean
  }

  // Accessibility testing
  accessibility?: {
    enabled?: boolean
    checkFocus?: boolean
    checkAriaLabels?: boolean
    checkColorContrast?: boolean
  }

  // Custom wrapper components
  wrappers?: React.ComponentType<{ children: ReactNode }>[]
}

export interface EnhancedRenderResult extends RenderResult {
  // Additional testing utilities
  mockUtils: ReturnType<typeof useMockTestingKit>
  // queryClient?: QueryClient // Not used
  
  // Enhanced queries with better error messages
  getByTestId: (testId: string) => HTMLElement
  findByTestId: (testId: string) => Promise<HTMLElement>
  queryByTestId: (testId: string) => HTMLElement | null
  
  // Accessibility helpers
  getAccessibilityReport: () => AccessibilityReport
  
  // Performance helpers
  getPerformanceMetrics: () => PerformanceMetrics
  
  // Mock state helpers
  getCurrentMockState: () => MockState
  resetAllMocks: () => void
}

export interface AccessibilityReport {
  issues: string[]
  warnings: string[]
  score: number
  focusableElements: number
  missingLabels: number
}

export interface PerformanceMetrics {
  renderTime: number
  componentCount: number
  rerenderCount: number
  memoryUsage?: number
}

export interface MockState {
  api: any
  auth: any
  notifications: any
  webSocket: any
  documents: any
  search: any
  upload: any
}

/**
 * Enhanced render function with comprehensive mock integration
 */
export const renderWithMocks = (
  ui: ReactElement,
  options: EnhancedRenderOptions = {}
): EnhancedRenderResult => {
  const {
    mockApi = {},
    mockAuth = {},
    mockNotifications = {},
    mockWebSocket = {},
    router = { type: 'memory', initialEntries: ['/'] },
    reactQuery = { enabled: true },
    testing = {
      enableErrorBoundary: true,
      enableSuspenseBoundary: true,
      enableNotifications: true,
      enableDebugPanels: process.env.NODE_ENV === 'development',
      enablePerformanceMonitoring: false,
      isolateComponent: false,
    },
    accessibility = { enabled: process.env.NODE_ENV === 'development' },
    wrappers = [],
    ...renderOptions
  } = options

  // Create QueryClient if React Query is enabled (currently disabled - not installed)
  const queryClient = undefined // React Query not used in this project

  // Performance monitoring
  const startTime = performance.now()
  let renderCount = 0

  // Create wrapper component
  const TestWrapper: React.FC<{ children: ReactNode }> = ({ children }) => {
    renderCount++
    
    // Mock testing utilities hook
    const mockUtils = useMockTestingKit()

    // Build wrapper chain
    let wrappedChildren: ReactNode = children

    // Apply custom wrappers in reverse order
    wrappers.reverse().forEach(Wrapper => {
      wrappedChildren = <Wrapper>{wrappedChildren}</Wrapper>
    })

    // React Query wrapper (disabled - not installed)
    // if (queryClient) {
    //   wrappedChildren = (
    //     <QueryClientProvider client={queryClient}>
    //       {wrappedChildren}
    //     </QueryClientProvider>
    //   )
    // }

    // Router wrapper
    const RouterComponent = router.type === 'browser' ? BrowserRouter : MemoryRouter
    const routerProps = router.type === 'memory' 
      ? { 
          initialEntries: router.initialEntries || ['/'],
          initialIndex: router.initialIndex,
          ...router.routerProps,
        }
      : router.routerProps || {}

    wrappedChildren = (
      <RouterComponent {...routerProps}>
        {wrappedChildren}
      </RouterComponent>
    )

    // Testing environment wrapper
    if (testing.isolateComponent) {
      wrappedChildren = (
        <TestingPlayground
          enableErrorTesting={testing.enableErrorBoundary}
          enableSuspenseTesting={testing.enableSuspenseBoundary}
          enableNotifications={testing.enableNotifications}
          showDebugPanels={testing.enableDebugPanels}
        >
          {wrappedChildren}
        </TestingPlayground>
      )
    } else {
      // Individual boundary wrapping
      if (testing.enableSuspenseBoundary) {
        wrappedChildren = (
          <TestSuspenseBoundary
            enableDebug={testing.enableDebugPanels}
            showProgressBar={true}
            animateEntrance={false} // Disable animations in tests
          >
            {wrappedChildren}
          </TestSuspenseBoundary>
        )
      }

      if (testing.enableErrorBoundary) {
        wrappedChildren = (
          <TestErrorBoundary
            enableRetry={true}
            maxRetries={3}
            logErrors={testing.enableDebugPanels}
            enableErrorReporting={false} // Disable in tests
          >
            {wrappedChildren}
          </TestErrorBoundary>
        )
      }
    }

    // Accessibility wrapper
    if (accessibility.enabled) {
      wrappedChildren = (
        <AccessibilityTestWrapper {...accessibility}>
          {wrappedChildren}
        </AccessibilityTestWrapper>
      )
    }

    // Performance wrapper
    if (testing.enablePerformanceMonitoring) {
      wrappedChildren = (
        <PerformanceTestWrapper name="TestComponent">
          {wrappedChildren}
        </PerformanceTestWrapper>
      )
    }

    // Mock providers wrapper
    wrappedChildren = (
      <ComprehensiveMockProvider
        api={mockApi}
        auth={mockAuth}
        notifications={mockNotifications}
        websocket={mockWebSocket}
      >
        {wrappedChildren}
      </ComprehensiveMockProvider>
    )

    return <>{wrappedChildren}</>
  }

  // Render with wrapper
  const result = render(ui, {
    wrapper: TestWrapper,
    ...renderOptions,
  })

  // Calculate performance metrics
  const renderTime = performance.now() - startTime

  // Enhanced result with additional utilities
  const enhancedResult: EnhancedRenderResult = {
    ...result,
    mockUtils: {} as any, // Will be populated by the hook
    queryClient,

    // Enhanced queries with better error messages
    getByTestId: (testId: string) => {
      try {
        return result.getByTestId(testId)
      } catch (error) {
        throw new Error(
          `Unable to find element with test ID "${testId}". ` +
          `Available test IDs: ${getAllTestIds(result.container).join(', ')}`
        )
      }
    },

    findByTestId: async (testId: string) => {
      try {
        return await result.findByTestId(testId)
      } catch (error) {
        throw new Error(
          `Unable to find element with test ID "${testId}" within timeout. ` +
          `Available test IDs: ${getAllTestIds(result.container).join(', ')}`
        )
      }
    },

    queryByTestId: (testId: string) => {
      return result.queryByTestId(testId)
    },

    // Accessibility helpers
    getAccessibilityReport: (): AccessibilityReport => {
      return getAccessibilityReport(result.container)
    },

    // Performance helpers
    getPerformanceMetrics: (): PerformanceMetrics => ({
      renderTime,
      componentCount: result.container.querySelectorAll('*').length,
      rerenderCount: renderCount,
      memoryUsage: (performance as any).memory?.usedJSHeapSize,
    }),

    // Mock state helpers
    getCurrentMockState: (): MockState => {
      // This would be populated by the mock utils hook
      return {
        api: {},
        auth: {},
        notifications: {},
        webSocket: {},
        documents: {},
        search: {},
        upload: {},
      }
    },

    resetAllMocks: () => {
      // Reset all mock providers
      // This would be implemented to reset all mock state
    },
  }

  return enhancedResult
}

// Convenience render functions for specific scenarios
export const renderWithAuth = (
  ui: ReactElement,
  authConfig: Partial<MockAuthProviderProps> = {},
  options: EnhancedRenderOptions = {}
): EnhancedRenderResult => {
  return renderWithMocks(ui, {
    ...options,
    mockAuth: { autoLogin: true, initialUser: 'user', ...authConfig },
  })
}

export const renderWithAdminAuth = (
  ui: ReactElement,
  options: EnhancedRenderOptions = {}
): EnhancedRenderResult => {
  return renderWithAuth(ui, { initialUser: 'admin' }, options)
}

export const renderWithScenario = (
  ui: ReactElement,
  scenario: string,
  options: EnhancedRenderOptions = {}
): EnhancedRenderResult => {
  return renderWithMocks(ui, {
    ...options,
    mockApi: { scenario, ...options.mockApi },
  })
}

export const renderWithNetworkError = (
  ui: ReactElement,
  options: EnhancedRenderOptions = {}
): EnhancedRenderResult => {
  return renderWithMocks(ui, {
    ...options,
    mockApi: { networkCondition: 'offline', ...options.mockApi },
  })
}

export const renderWithWebSocket = (
  ui: ReactElement,
  webSocketConfig: Partial<MockWebSocketProviderProps> = {},
  options: EnhancedRenderOptions = {}
): EnhancedRenderResult => {
  return renderWithMocks(ui, {
    ...options,
    mockWebSocket: { autoConnect: true, ...webSocketConfig },
  })
}

export const renderWithNotifications = (
  ui: ReactElement,
  options: EnhancedRenderOptions = {}
): EnhancedRenderResult => {
  return renderWithMocks(ui, {
    ...options,
    testing: { 
      ...options.testing, 
      enableNotifications: true 
    },
    mockNotifications: { 
      position: 'top-right', 
      maxNotifications: 5,
      ...options.mockNotifications 
    },
  })
}

export const renderForPerformanceTesting = (
  ui: ReactElement,
  options: EnhancedRenderOptions = {}
): EnhancedRenderResult => {
  return renderWithMocks(ui, {
    ...options,
    testing: {
      ...options.testing,
      enablePerformanceMonitoring: true,
      enableDebugPanels: false,
    },
  })
}

export const renderForAccessibilityTesting = (
  ui: ReactElement,
  options: EnhancedRenderOptions = {}
): EnhancedRenderResult => {
  return renderWithMocks(ui, {
    ...options,
    accessibility: {
      enabled: true,
      checkFocus: true,
      checkAriaLabels: true,
      checkColorContrast: true,
      ...options.accessibility,
    },
  })
}

export const renderIsolated = (
  ui: ReactElement,
  options: EnhancedRenderOptions = {}
): EnhancedRenderResult => {
  return renderWithMocks(ui, {
    ...options,
    testing: {
      ...options.testing,
      isolateComponent: true,
      enableErrorBoundary: true,
      enableSuspenseBoundary: true,
    },
  })
}

// Helper functions
function getAllTestIds(container: HTMLElement): string[] {
  const elements = container.querySelectorAll('[data-testid]')
  return Array.from(elements)
    .map(el => el.getAttribute('data-testid'))
    .filter(Boolean) as string[]
}

function getAccessibilityReport(container: HTMLElement): AccessibilityReport {
  const issues: string[] = []
  const warnings: string[] = []

  // Check for missing alt text on images
  const images = container.querySelectorAll('img')
  images.forEach(img => {
    if (!img.getAttribute('alt') && !img.getAttribute('aria-label')) {
      issues.push('Image missing alt text')
    }
  })

  // Check for missing labels on form elements
  const formElements = container.querySelectorAll('input, select, textarea')
  let missingLabels = 0
  formElements.forEach(element => {
    const hasLabel = element.getAttribute('aria-label') ||
                    element.getAttribute('aria-labelledby') ||
                    container.querySelector(`label[for="${element.id}"]`)
    
    if (!hasLabel) {
      missingLabels++
      issues.push(`Form element missing label: ${element.tagName}`)
    }
  })

  // Check for focusable elements
  const focusableElements = container.querySelectorAll(
    'button, input, select, textarea, a[href], [tabindex]:not([tabindex="-1"])'
  )

  // Check for missing button text
  const buttons = container.querySelectorAll('button')
  buttons.forEach(button => {
    if (!button.textContent?.trim() && !button.getAttribute('aria-label')) {
      issues.push('Button missing accessible text')
    }
  })

  // Calculate accessibility score
  const totalChecks = images.length + formElements.length + buttons.length
  const issueCount = issues.length
  const score = totalChecks > 0 ? Math.max(0, (totalChecks - issueCount) / totalChecks * 100) : 100

  return {
    issues,
    warnings,
    score: Math.round(score),
    focusableElements: focusableElements.length,
    missingLabels,
  }
}

// Import helper components for use
import { AccessibilityTestWrapper, PerformanceTestWrapper } from './components'

// Async rendering utilities for Suspense and async components
export const renderAsync = async (
  ui: ReactElement,
  options: EnhancedRenderOptions = {}
): Promise<EnhancedRenderResult> => {
  const result = renderWithMocks(ui, options)
  
  // Wait for any Suspense boundaries to resolve
  await new Promise(resolve => setTimeout(resolve, 100))
  
  return result
}

// Batch rendering for testing multiple scenarios
export const renderBatch = (
  scenarios: Array<{
    name: string
    ui: ReactElement
    options?: EnhancedRenderOptions
  }>
): Array<{ name: string; result: EnhancedRenderResult }> => {
  return scenarios.map(({ name, ui, options = {} }) => ({
    name,
    result: renderWithMocks(ui, options),
  }))
}

// Comparative rendering for A/B testing
export const renderComparative = (
  variants: Array<{
    name: string
    ui: ReactElement
    options?: EnhancedRenderOptions
  }>
): {
  variants: Array<{ name: string; result: EnhancedRenderResult }>
  compare: (selector: string) => Array<{ name: string; element: HTMLElement | null }>
} => {
  const results = renderBatch(variants)
  
  return {
    variants: results,
    compare: (selector: string) => {
      return results.map(({ name, result }) => ({
        name,
        element: result.container.querySelector(selector),
      }))
    },
  }
}