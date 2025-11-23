/**
 * TestErrorBoundary - Error handling validation component
 * Provides comprehensive error boundary testing with realistic error scenarios
 */

import React, { Component, ErrorInfo, ReactNode } from 'react'

export interface ErrorBoundaryState {
  hasError: boolean
  error: Error | null
  errorInfo: ErrorInfo | null
  errorId: string | null
  retryCount: number
}

export interface TestErrorBoundaryProps {
  children: ReactNode
  fallbackComponent?: React.ComponentType<ErrorFallbackProps>
  onError?: (error: Error, errorInfo: ErrorInfo, errorId: string) => void
  enableRetry?: boolean
  maxRetries?: number
  resetKeys?: Array<string | number>
  resetOnPropsChange?: boolean
  isolateErrors?: boolean
  logErrors?: boolean
  enableErrorReporting?: boolean
  customErrorMessages?: Record<string, string>
}

export interface ErrorFallbackProps {
  error: Error | null
  errorInfo: ErrorInfo | null
  retry: () => void
  canRetry: boolean
  retryCount: number
  errorId: string | null
}

export interface ErrorSimulation {
  triggerRenderError: () => void
  triggerAsyncError: () => void
  triggerNetworkError: () => void
  triggerTypeError: () => void
  triggerChunkLoadError: () => void
  triggerCustomError: (message: string, type?: string) => void
  clearError: () => void
}

// Default fallback component with 2026 minimalistic design
const DefaultErrorFallback: React.FC<ErrorFallbackProps> = ({
  error,
  errorInfo,
  retry,
  canRetry,
  retryCount,
  errorId,
}) => (
  <div 
    style={{
      padding: '2rem',
      margin: '1rem',
      borderRadius: '12px',
      backgroundColor: '#fef2f2',
      border: '1px solid #fecaca',
      color: '#991b1b',
      fontFamily: 'Inter, system-ui, sans-serif',
    }}
    data-testid="error-boundary-fallback"
    role="alert"
  >
    <div style={{ display: 'flex', alignItems: 'center', marginBottom: '1rem' }}>
      <svg 
        width="20" 
        height="20" 
        fill="currentColor" 
        viewBox="0 0 20 20"
        style={{ marginRight: '0.5rem' }}
      >
        <path 
          fillRule="evenodd" 
          d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z" 
          clipRule="evenodd" 
        />
      </svg>
      <h3 style={{ margin: 0, fontSize: '1.125rem', fontWeight: '600' }}>
        Something went wrong
      </h3>
    </div>
    
    <div style={{ marginBottom: '1rem' }}>
      <p style={{ margin: '0 0 0.5rem 0', fontSize: '0.875rem', opacity: 0.8 }}>
        Error ID: {errorId}
      </p>
      <details style={{ fontSize: '0.875rem' }}>
        <summary style={{ cursor: 'pointer', marginBottom: '0.5rem' }}>
          Error Details
        </summary>
        <div 
          style={{ 
            background: '#fee2e2', 
            padding: '0.75rem', 
            borderRadius: '8px',
            fontFamily: 'monospace',
            fontSize: '0.75rem',
            whiteSpace: 'pre-wrap',
            overflow: 'auto',
            maxHeight: '200px'
          }}
        >
          <strong>Message:</strong> {error?.message || 'Unknown error'}
          {error?.stack && (
            <>
              <br /><br />
              <strong>Stack:</strong>
              <br />
              {error.stack}
            </>
          )}
          {errorInfo?.componentStack && (
            <>
              <br /><br />
              <strong>Component Stack:</strong>
              <br />
              {errorInfo.componentStack}
            </>
          )}
        </div>
      </details>
    </div>

    <div style={{ display: 'flex', gap: '0.75rem', alignItems: 'center' }}>
      {canRetry && (
        <button
          onClick={retry}
          style={{
            background: '#dc2626',
            color: 'white',
            border: 'none',
            padding: '0.5rem 1rem',
            borderRadius: '8px',
            fontSize: '0.875rem',
            fontWeight: '500',
            cursor: 'pointer',
            transition: 'background-color 0.2s',
          }}
          onMouseOver={(e) => e.currentTarget.style.background = '#b91c1c'}
          onMouseOut={(e) => e.currentTarget.style.background = '#dc2626'}
        >
          Retry {retryCount > 0 && `(${retryCount})`}
        </button>
      )}
      
      <button
        onClick={() => window.location.reload()}
        style={{
          background: 'transparent',
          color: '#dc2626',
          border: '1px solid #dc2626',
          padding: '0.5rem 1rem',
          borderRadius: '8px',
          fontSize: '0.875rem',
          fontWeight: '500',
          cursor: 'pointer',
        }}
      >
        Reload Page
      </button>
    </div>
  </div>
)

export class TestErrorBoundary extends Component<TestErrorBoundaryProps, ErrorBoundaryState> {
  private retryTimeoutId: number | null = null
  private errorReportingEndpoint = '/api/errors' // Mock endpoint

  constructor(props: TestErrorBoundaryProps) {
    super(props)
    
    this.state = {
      hasError: false,
      error: null,
      errorInfo: null,
      errorId: null,
      retryCount: 0,
    }
  }

  static getDerivedStateFromError(error: Error): Partial<ErrorBoundaryState> {
    const errorId = `error_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`
    
    return {
      hasError: true,
      error,
      errorId,
    }
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    const errorId = this.state.errorId || 'unknown'
    
    this.setState({
      error,
      errorInfo,
    })

    // Log error if enabled
    if (this.props.logErrors !== false) {
      console.group(`🔴 Error Boundary Caught Error [${errorId}]`)
      console.error('Error:', error)
      console.error('Error Info:', errorInfo)
      console.error('Component Stack:', errorInfo.componentStack)
      console.groupEnd()
    }

    // Call error callback
    if (this.props.onError) {
      this.props.onError(error, errorInfo, errorId)
    }

    // Report error if enabled
    if (this.props.enableErrorReporting) {
      this.reportError(error, errorInfo, errorId)
    }
  }

  componentDidUpdate(prevProps: TestErrorBoundaryProps) {
    const { resetKeys = [], resetOnPropsChange = false } = this.props
    const { hasError } = this.state

    if (hasError && resetOnPropsChange) {
      // Check if any reset keys have changed
      const hasResetKeyChanged = resetKeys.some((key, index) => 
        prevProps.resetKeys?.[index] !== key
      )

      if (hasResetKeyChanged) {
        this.resetError()
      }
    }
  }

  componentWillUnmount() {
    if (this.retryTimeoutId) {
      clearTimeout(this.retryTimeoutId)
    }
  }

  resetError = () => {
    this.setState({
      hasError: false,
      error: null,
      errorInfo: null,
      errorId: null,
      retryCount: 0,
    })
  }

  retry = () => {
    const { maxRetries = 3 } = this.props
    const { retryCount } = this.state

    if (retryCount >= maxRetries) {
      console.warn(`Max retries (${maxRetries}) reached`)
      return
    }

    this.setState(prevState => ({
      hasError: false,
      error: null,
      errorInfo: null,
      errorId: null,
      retryCount: prevState.retryCount + 1,
    }))
  }

  reportError = async (error: Error, errorInfo: ErrorInfo, errorId: string) => {
    try {
      const errorReport = {
        errorId,
        message: error.message,
        stack: error.stack,
        componentStack: errorInfo.componentStack,
        userAgent: navigator.userAgent,
        url: window.location.href,
        timestamp: new Date().toISOString(),
        retryCount: this.state.retryCount,
      }

      // In a real app, you'd send this to your error reporting service
      console.log('📊 Error Report:', errorReport)
      
      // Simulate API call
      await fetch(this.errorReportingEndpoint, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(errorReport),
      }).catch(() => {
        // Fail silently for error reporting
        console.warn('Failed to report error to server')
      })
    } catch (reportingError) {
      console.warn('Error in error reporting:', reportingError)
    }
  }

  getErrorMessage = (error: Error | null): string => {
    if (!error) return 'Unknown error occurred'
    
    const { customErrorMessages = {} } = this.props
    
    // Check for custom messages
    if (customErrorMessages[error.message]) {
      return customErrorMessages[error.message]
    }

    // Provide user-friendly messages for common errors
    if (error.message.includes('ChunkLoadError')) {
      return 'Failed to load application resources. Please refresh the page.'
    }
    
    if (error.message.includes('NetworkError')) {
      return 'Network connection lost. Please check your connection and try again.'
    }
    
    if (error.name === 'TypeError') {
      return 'A technical error occurred. Our team has been notified.'
    }
    
    return error.message
  }

  render() {
    const { 
      children, 
      fallbackComponent: FallbackComponent = DefaultErrorFallback,
      enableRetry = true,
      maxRetries = 3,
      isolateErrors = false 
    } = this.props
    
    const { hasError, error, errorInfo, errorId, retryCount } = this.state

    if (hasError) {
      const canRetry = enableRetry && retryCount < maxRetries
      
      return (
        <FallbackComponent
          error={error}
          errorInfo={errorInfo}
          retry={this.retry}
          canRetry={canRetry}
          retryCount={retryCount}
          errorId={errorId}
        />
      )
    }

    // If isolateErrors is enabled, wrap children in individual error boundaries
    if (isolateErrors) {
      return (
        <div data-testid="isolated-error-boundary">
          {React.Children.map(children, (child, index) => (
            <TestErrorBoundary
              key={index}
              {...this.props}
              isolateErrors={false} // Prevent infinite recursion
            >
              {child}
            </TestErrorBoundary>
          ))}
        </div>
      )
    }

    return <>{children}</>
  }
}

// Hook for error simulation in tests
export const useErrorSimulation = (): ErrorSimulation => {
  const triggerRenderError = () => {
    throw new Error('Test render error: Component failed to render')
  }

  const triggerAsyncError = () => {
    setTimeout(() => {
      throw new Error('Test async error: Unhandled promise rejection')
    }, 100)
  }

  const triggerNetworkError = () => {
    throw new Error('NetworkError: Failed to fetch data from server')
  }

  const triggerTypeError = () => {
    // @ts-ignore - Intentional type error for testing
    null.someMethod()
  }

  const triggerChunkLoadError = () => {
    throw new Error('ChunkLoadError: Failed to load chunk from CDN')
  }

  const triggerCustomError = (message: string, type = 'Error') => {
    const error = new Error(message)
    error.name = type
    throw error
  }

  const clearError = () => {
    // This would be used to clear errors in the calling component
    console.log('Clearing error state')
  }

  return {
    triggerRenderError,
    triggerAsyncError,
    triggerNetworkError,
    triggerTypeError,
    triggerChunkLoadError,
    triggerCustomError,
    clearError,
  }
}

// Error testing component for development
export const ErrorTestPanel: React.FC = () => {
  const errorSim = useErrorSimulation()

  if (process.env.NODE_ENV === 'production') {
    return null
  }

  return (
    <div 
      style={{
        position: 'fixed',
        bottom: '16px',
        right: '16px',
        background: 'white',
        border: '1px solid #e5e7eb',
        borderRadius: '12px',
        padding: '1rem',
        boxShadow: '0 4px 12px rgba(0, 0, 0, 0.15)',
        zIndex: 9999,
        minWidth: '250px',
      }}
      data-testid="error-test-panel"
    >
      <h4 style={{ margin: '0 0 1rem 0', fontSize: '0.875rem', fontWeight: '600' }}>
        Error Testing
      </h4>
      
      <div style={{ display: 'grid', gap: '0.5rem' }}>
        {[
          { label: 'Render Error', action: errorSim.triggerRenderError },
          { label: 'Async Error', action: errorSim.triggerAsyncError },
          { label: 'Network Error', action: errorSim.triggerNetworkError },
          { label: 'Type Error', action: errorSim.triggerTypeError },
          { label: 'Chunk Error', action: errorSim.triggerChunkLoadError },
        ].map(({ label, action }) => (
          <button
            key={label}
            onClick={action}
            style={{
              background: '#fee2e2',
              color: '#991b1b',
              border: '1px solid #fecaca',
              padding: '0.5rem',
              borderRadius: '6px',
              fontSize: '0.75rem',
              cursor: 'pointer',
              transition: 'all 0.2s',
            }}
            onMouseOver={(e) => {
              e.currentTarget.style.background = '#fecaca'
            }}
            onMouseOut={(e) => {
              e.currentTarget.style.background = '#fee2e2'
            }}
          >
            {label}
          </button>
        ))}
        
        <button
          onClick={() => errorSim.triggerCustomError('Custom test error', 'CustomError')}
          style={{
            background: '#ddd6fe',
            color: '#6b21a8',
            border: '1px solid #c4b5fd',
            padding: '0.5rem',
            borderRadius: '6px',
            fontSize: '0.75rem',
            cursor: 'pointer',
          }}
        >
          Custom Error
        </button>
      </div>
    </div>
  )
}

// Higher-order component for easy error boundary wrapping
export const withErrorBoundary = <P extends object>(
  Component: React.ComponentType<P>,
  errorBoundaryProps?: Partial<TestErrorBoundaryProps>
) => {
  const WrappedComponent = (props: P) => (
    <TestErrorBoundary {...errorBoundaryProps}>
      <Component {...props} />
    </TestErrorBoundary>
  )
  
  WrappedComponent.displayName = `withErrorBoundary(${Component.displayName || Component.name})`
  return WrappedComponent
}