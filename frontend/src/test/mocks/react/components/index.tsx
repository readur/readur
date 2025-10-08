/**
 * React Components - Mock testing components
 * Centralized exports for all mock testing components
 */

export {
  TestErrorBoundary,
  useErrorSimulation,
  ErrorTestPanel,
  withErrorBoundary,
  type TestErrorBoundaryProps,
  type ErrorBoundaryState,
  type ErrorFallbackProps,
  type ErrorSimulation,
} from './TestErrorBoundary'

export {
  TestSuspenseBoundary,
  useSuspenseSimulation,
  SuspenseTestPanel,
  withSuspenseBoundary,
  type TestSuspenseBoundaryProps,
  type SuspenseMetrics,
} from './TestSuspenseBoundary'

export {
  MockFileDropzone,
  type MockFileDropzoneProps,
  type FilePreview,
} from './MockFileDropzone'

export {
  MockSearchResults,
  type MockSearchResultsProps,
} from './MockSearchResults'

export {
  MockNotificationStack,
  type MockNotificationStackProps,
} from './MockNotificationStack'

// Combined testing component that includes all utilities
import React, { useState, useRef } from 'react'
import { TestErrorBoundary, ErrorTestPanel } from './TestErrorBoundary'
import { TestSuspenseBoundary, SuspenseTestPanel } from './TestSuspenseBoundary'
import { MockNotificationStack } from './MockNotificationStack'
import { ComprehensiveMockProvider } from '../providers'

export interface TestingPlaygroundProps {
  children: React.ReactNode
  enableErrorTesting?: boolean
  enableSuspenseTesting?: boolean
  enableNotifications?: boolean
  showDebugPanels?: boolean
  errorBoundaryProps?: Partial<React.ComponentProps<typeof TestErrorBoundary>>
  suspenseProps?: Partial<React.ComponentProps<typeof TestSuspenseBoundary>>
  notificationProps?: Partial<React.ComponentProps<typeof MockNotificationStack>>
}

/**
 * TestingPlayground - Comprehensive testing environment
 * Wraps children with all testing utilities and debug panels
 */
export const TestingPlayground: React.FC<TestingPlaygroundProps> = ({
  children,
  enableErrorTesting = true,
  enableSuspenseTesting = true,
  enableNotifications = true,
  showDebugPanels = process.env.NODE_ENV === 'development',
  errorBoundaryProps = {},
  suspenseProps = {},
  notificationProps = {},
}) => {
  const suspenseBoundaryRef = useRef<TestSuspenseBoundary>(null)

  let wrappedChildren = children

  // Wrap with Suspense boundary
  if (enableSuspenseTesting) {
    wrappedChildren = (
      <TestSuspenseBoundary
        ref={suspenseBoundaryRef}
        enableDebug={showDebugPanels}
        {...suspenseProps}
      >
        {wrappedChildren}
      </TestSuspenseBoundary>
    )
  }

  // Wrap with Error boundary
  if (enableErrorTesting) {
    wrappedChildren = (
      <TestErrorBoundary
        enableErrorReporting={showDebugPanels}
        logErrors={showDebugPanels}
        {...errorBoundaryProps}
      >
        {wrappedChildren}
      </TestErrorBoundary>
    )
  }

  return (
    <ComprehensiveMockProvider>
      {wrappedChildren}
      
      {/* Notification stack */}
      {enableNotifications && (
        <MockNotificationStack
          position="top-right"
          enableActions={true}
          showProgress={true}
          {...notificationProps}
        />
      )}
      
      {/* Debug panels */}
      {showDebugPanels && (
        <>
          {enableErrorTesting && <ErrorTestPanel />}
          {enableSuspenseTesting && (
            <SuspenseTestPanel boundary={suspenseBoundaryRef} />
          )}
        </>
      )}
    </ComprehensiveMockProvider>
  )
}

// Quick setup components for common testing scenarios
export const ErrorTestingEnvironment: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <TestingPlayground
    enableSuspenseTesting={false}
    enableNotifications={false}
    errorBoundaryProps={{
      enableRetry: true,
      maxRetries: 3,
      resetOnPropsChange: true,
    }}
  >
    {children}
  </TestingPlayground>
)

export const SuspenseTestingEnvironment: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <TestingPlayground
    enableErrorTesting={false}
    enableNotifications={false}
    suspenseProps={{
      showProgressBar: true,
      animateEntrance: true,
      timeout: 10000,
    }}
  >
    {children}
  </TestingPlayground>
)

export const NotificationTestingEnvironment: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <TestingPlayground
    enableErrorTesting={false}
    enableSuspenseTesting={false}
    notificationProps={{
      position: 'top-right',
      maxVisible: 10,
      enableGrouping: true,
      enableActions: true,
    }}
  >
    {children}
  </TestingPlayground>
)

// Utility component for testing component isolation
export const IsolatedComponentTest: React.FC<{
  children: React.ReactNode
  componentName?: string
  enableAllBoundaries?: boolean
}> = ({ children, componentName = 'TestComponent', enableAllBoundaries = true }) => {
  const [hasError, setHasError] = useState(false)
  const [isLoading, setIsLoading] = useState(false)

  return (
    <div
      style={{
        border: '2px dashed #e5e7eb',
        borderRadius: '8px',
        padding: '1rem',
        margin: '1rem',
        backgroundColor: '#fafafa',
      }}
      data-testid={`isolated-test-${componentName.toLowerCase()}`}
    >
      <div style={{
        fontSize: '0.75rem',
        color: '#6b7280',
        marginBottom: '0.5rem',
        fontFamily: 'monospace',
      }}>
        🧪 Testing: {componentName}
        {hasError && ' (Error State)'}
        {isLoading && ' (Loading State)'}
      </div>
      
      {enableAllBoundaries ? (
        <TestingPlayground
          showDebugPanels={false}
          errorBoundaryProps={{
            onError: () => setHasError(true),
            enableRetry: true,
          }}
          suspenseProps={{
            onSuspense: () => setIsLoading(true),
            onResolve: () => setIsLoading(false),
          }}
        >
          {children}
        </TestingPlayground>
      ) : (
        children
      )}
    </div>
  )
}

// Performance testing wrapper
export const PerformanceTestWrapper: React.FC<{
  children: React.ReactNode
  name?: string
  onMetrics?: (metrics: any) => void
}> = ({ children, name = 'Component', onMetrics }) => {
  const [renderCount, setRenderCount] = useState(0)
  const [lastRenderTime, setLastRenderTime] = useState<number | null>(null)
  const startTimeRef = useRef<number>(performance.now())

  React.useEffect(() => {
    const renderTime = performance.now() - startTimeRef.current
    setRenderCount(prev => prev + 1)
    setLastRenderTime(renderTime)
    
    if (onMetrics) {
      onMetrics({
        name,
        renderCount: renderCount + 1,
        renderTime,
        timestamp: Date.now(),
      })
    }
    
    startTimeRef.current = performance.now()
  })

  return (
    <div data-testid={`performance-wrapper-${name.toLowerCase()}`}>
      {process.env.NODE_ENV === 'development' && (
        <div style={{
          fontSize: '0.625rem',
          color: '#9ca3af',
          padding: '0.25rem 0.5rem',
          backgroundColor: '#f3f4f6',
          borderRadius: '4px',
          marginBottom: '0.5rem',
          fontFamily: 'monospace',
        }}>
          📊 {name}: {renderCount} renders
          {lastRenderTime && ` | Last: ${lastRenderTime.toFixed(2)}ms`}
        </div>
      )}
      {children}
    </div>
  )
}

// Accessibility testing wrapper
export const AccessibilityTestWrapper: React.FC<{
  children: React.ReactNode
  checkFocus?: boolean
  checkAriaLabels?: boolean
  checkColorContrast?: boolean
}> = ({ children, checkFocus = true, checkAriaLabels = true, checkColorContrast = true }) => {
  const wrapperRef = useRef<HTMLDivElement>(null)
  const [a11yIssues, setA11yIssues] = useState<string[]>([])

  React.useEffect(() => {
    if (process.env.NODE_ENV !== 'development') return

    const issues: string[] = []
    const wrapper = wrapperRef.current
    if (!wrapper) return

    // Check for missing aria-labels
    if (checkAriaLabels) {
      const interactiveElements = wrapper.querySelectorAll('button, input, select, textarea, [role="button"]')
      interactiveElements.forEach(element => {
        const hasLabel = element.getAttribute('aria-label') || 
                        element.getAttribute('aria-labelledby') ||
                        element.getAttribute('title') ||
                        element.textContent?.trim()
        
        if (!hasLabel) {
          issues.push(`Interactive element missing accessible label: ${element.tagName}`)
        }
      })
    }

    // Check for focus management
    if (checkFocus) {
      const focusableElements = wrapper.querySelectorAll('button, input, select, textarea, a[href], [tabindex]:not([tabindex="-1"])')
      if (focusableElements.length === 0) {
        issues.push('No focusable elements found')
      }
    }

    setA11yIssues(issues)
  }, [children, checkFocus, checkAriaLabels, checkColorContrast])

  return (
    <div ref={wrapperRef} data-testid="accessibility-test-wrapper">
      {process.env.NODE_ENV === 'development' && a11yIssues.length > 0 && (
        <div style={{
          backgroundColor: '#fef2f2',
          border: '1px solid #fecaca',
          borderRadius: '6px',
          padding: '0.75rem',
          marginBottom: '1rem',
        }}>
          <h4 style={{ 
            margin: '0 0 0.5rem 0', 
            fontSize: '0.875rem', 
            color: '#dc2626',
            fontWeight: '600',
          }}>
            ♿ Accessibility Issues:
          </h4>
          <ul style={{ 
            margin: 0, 
            paddingLeft: '1.5rem', 
            fontSize: '0.75rem',
            color: '#dc2626',
          }}>
            {a11yIssues.map((issue, index) => (
              <li key={index}>{issue}</li>
            ))}
          </ul>
        </div>
      )}
      {children}
    </div>
  )
}