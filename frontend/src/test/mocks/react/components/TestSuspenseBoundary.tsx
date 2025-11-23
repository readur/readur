/**
 * TestSuspenseBoundary - Loading state testing component
 * Provides comprehensive Suspense testing with realistic loading scenarios
 */

import React, { Suspense, Component, ReactNode, useState, useEffect, useCallback } from 'react'

export interface TestSuspenseBoundaryProps {
  children: ReactNode
  fallback?: ReactNode
  enableDebug?: boolean
  onSuspense?: (componentName: string) => void
  onResolve?: (componentName: string, duration: number) => void
  timeout?: number
  showProgressBar?: boolean
  animateEntrance?: boolean
  customStyles?: React.CSSProperties
}

export interface SuspenseMetrics {
  suspenseCount: number
  totalSuspenseTime: number
  averageSuspenseTime: number
  longestSuspense: number
  componentsLoaded: string[]
  loadingHistory: Array<{
    component: string
    startTime: number
    endTime: number | null
    duration?: number
  }>
}

// Modern minimalistic loading component
const ModernLoadingFallback: React.FC<{ 
  showProgressBar?: boolean
  animateEntrance?: boolean
  customStyles?: React.CSSProperties
}> = ({ 
  showProgressBar = false, 
  animateEntrance = true,
  customStyles = {}
}) => {
  const [progress, setProgress] = useState(0)

  useEffect(() => {
    if (!showProgressBar) return

    const interval = setInterval(() => {
      setProgress(prev => {
        const increment = Math.random() * 20 + 5 // 5-25% increments
        const newProgress = Math.min(prev + increment, 90) // Stop at 90%
        return newProgress
      })
    }, 300)

    return () => clearInterval(interval)
  }, [showProgressBar])

  const baseStyles: React.CSSProperties = {
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    justifyContent: 'center',
    padding: '3rem 2rem',
    minHeight: '200px',
    backgroundColor: '#fafafa',
    borderRadius: '16px',
    margin: '1rem',
    opacity: animateEntrance ? 0 : 1,
    transform: animateEntrance ? 'translateY(8px)' : 'translateY(0)',
    animation: animateEntrance ? 'fadeInUp 0.3s ease-out forwards' : 'none',
    ...customStyles,
  }

  return (
    <>
      <style>
        {`
          @keyframes fadeInUp {
            to {
              opacity: 1;
              transform: translateY(0);
            }
          }
          
          @keyframes spin {
            to {
              transform: rotate(360deg);
            }
          }
          
          @keyframes pulse {
            0%, 100% {
              opacity: 1;
            }
            50% {
              opacity: 0.5;
            }
          }
        `}
      </style>
      
      <div style={baseStyles} data-testid="suspense-loading">
        {/* Modern loading spinner */}
        <div
          style={{
            width: '32px',
            height: '32px',
            border: '3px solid #e5e7eb',
            borderTop: '3px solid #3b82f6',
            borderRadius: '50%',
            animation: 'spin 1s linear infinite',
            marginBottom: '1.5rem',
          }}
        />
        
        <p style={{
          margin: '0 0 1rem 0',
          color: '#6b7280',
          fontSize: '0.875rem',
          fontWeight: '500',
          fontFamily: 'Inter, system-ui, sans-serif',
        }}>
          Loading...
        </p>

        {showProgressBar && (
          <div style={{
            width: '100%',
            maxWidth: '200px',
            height: '4px',
            backgroundColor: '#e5e7eb',
            borderRadius: '2px',
            overflow: 'hidden',
          }}>
            <div
              style={{
                width: `${progress}%`,
                height: '100%',
                backgroundColor: '#3b82f6',
                borderRadius: '2px',
                transition: 'width 0.3s ease-out',
              }}
            />
          </div>
        )}

        {/* Subtle pulsing dots */}
        <div style={{
          display: 'flex',
          gap: '0.25rem',
          marginTop: '1rem',
        }}>
          {[0, 1, 2].map(i => (
            <div
              key={i}
              style={{
                width: '6px',
                height: '6px',
                borderRadius: '50%',
                backgroundColor: '#9ca3af',
                animation: `pulse 1.5s ease-in-out infinite`,
                animationDelay: `${i * 0.2}s`,
              }}
            />
          ))}
        </div>
      </div>
    </>
  )
}

// Suspense boundary with metrics tracking
export class TestSuspenseBoundary extends Component<
  TestSuspenseBoundaryProps, 
  { metrics: SuspenseMetrics; currentlySuspended: Set<string> }
> {
  private suspenseStartTimes = new Map<string, number>()

  constructor(props: TestSuspenseBoundaryProps) {
    super(props)
    
    this.state = {
      metrics: {
        suspenseCount: 0,
        totalSuspenseTime: 0,
        averageSuspenseTime: 0,
        longestSuspense: 0,
        componentsLoaded: [],
        loadingHistory: [],
      },
      currentlySuspended: new Set(),
    }
  }

  componentDidCatch(error: any, errorInfo: any) {
    // Handle Suspense promises
    if (error && typeof error.then === 'function') {
      const componentName = this.extractComponentName(errorInfo)
      this.handleSuspenseStart(componentName)
      
      error.then(() => {
        this.handleSuspenseEnd(componentName)
      })
    }
  }

  private extractComponentName(errorInfo: any): string {
    // Extract component name from error info
    const stack = errorInfo?.componentStack || ''
    const match = stack.match(/\s+in (\w+)/)
    return match ? match[1] : 'UnknownComponent'
  }

  private handleSuspenseStart = (componentName: string) => {
    const startTime = performance.now()
    this.suspenseStartTimes.set(componentName, startTime)
    
    this.setState(prevState => ({
      metrics: {
        ...prevState.metrics,
        suspenseCount: prevState.metrics.suspenseCount + 1,
        loadingHistory: [
          ...prevState.metrics.loadingHistory,
          {
            component: componentName,
            startTime,
            endTime: null,
          }
        ],
      },
      currentlySuspended: new Set([...prevState.currentlySuspended, componentName]),
    }))

    if (this.props.onSuspense) {
      this.props.onSuspense(componentName)
    }

    if (this.props.enableDebug) {
      console.log(`🔄 Suspense started for: ${componentName}`)
    }
  }

  private handleSuspenseEnd = (componentName: string) => {
    const endTime = performance.now()
    const startTime = this.suspenseStartTimes.get(componentName)
    
    if (startTime) {
      const duration = endTime - startTime
      
      this.setState(prevState => {
        const newTotalTime = prevState.metrics.totalSuspenseTime + duration
        const newCount = prevState.metrics.suspenseCount
        const newAverage = newTotalTime / newCount
        const newLongest = Math.max(prevState.metrics.longestSuspense, duration)
        
        const updatedHistory = prevState.metrics.loadingHistory.map(item =>
          item.component === componentName && item.endTime === null
            ? { ...item, endTime, duration }
            : item
        )

        const updatedSuspended = new Set(prevState.currentlySuspended)
        updatedSuspended.delete(componentName)

        return {
          metrics: {
            ...prevState.metrics,
            totalSuspenseTime: newTotalTime,
            averageSuspenseTime: newAverage,
            longestSuspense: newLongest,
            componentsLoaded: [...prevState.metrics.componentsLoaded, componentName],
            loadingHistory: updatedHistory,
          },
          currentlySuspended: updatedSuspended,
        }
      })

      if (this.props.onResolve) {
        this.props.onResolve(componentName, duration)
      }

      if (this.props.enableDebug) {
        console.log(`✅ Suspense resolved for: ${componentName} (${duration.toFixed(2)}ms)`)
      }

      this.suspenseStartTimes.delete(componentName)
    }
  }

  getMetrics = (): SuspenseMetrics => {
    return this.state.metrics
  }

  resetMetrics = () => {
    this.setState({
      metrics: {
        suspenseCount: 0,
        totalSuspenseTime: 0,
        averageSuspenseTime: 0,
        longestSuspense: 0,
        componentsLoaded: [],
        loadingHistory: [],
      },
      currentlySuspended: new Set(),
    })
    this.suspenseStartTimes.clear()
  }

  render() {
    const { 
      children, 
      fallback, 
      showProgressBar = false,
      animateEntrance = true,
      customStyles = {},
      timeout 
    } = this.props

    const defaultFallback = (
      <ModernLoadingFallback 
        showProgressBar={showProgressBar}
        animateEntrance={animateEntrance}
        customStyles={customStyles}
      />
    )

    let suspenseElement = (
      <Suspense fallback={fallback || defaultFallback}>
        {children}
      </Suspense>
    )

    // Add timeout if specified
    if (timeout) {
      suspenseElement = (
        <SuspenseWithTimeout timeout={timeout}>
          {suspenseElement}
        </SuspenseWithTimeout>
      )
    }

    return suspenseElement
  }
}

// Suspense with timeout functionality
const SuspenseWithTimeout: React.FC<{ children: ReactNode; timeout: number }> = ({ 
  children, 
  timeout 
}) => {
  const [hasTimedOut, setHasTimedOut] = useState(false)

  useEffect(() => {
    const timer = setTimeout(() => {
      setHasTimedOut(true)
    }, timeout)

    return () => clearTimeout(timer)
  }, [timeout])

  if (hasTimedOut) {
    return (
      <div 
        style={{
          padding: '2rem',
          textAlign: 'center',
          color: '#ef4444',
          backgroundColor: '#fef2f2',
          border: '1px solid #fecaca',
          borderRadius: '12px',
          margin: '1rem',
        }}
        data-testid="suspense-timeout"
      >
        <h3 style={{ margin: '0 0 0.5rem 0', fontSize: '1rem' }}>
          Loading Timeout
        </h3>
        <p style={{ margin: 0, fontSize: '0.875rem', opacity: 0.8 }}>
          Component failed to load within {timeout}ms
        </p>
      </div>
    )
  }

  return <>{children}</>
}

// Hook for creating suspense-ready components
export const useSuspenseSimulation = () => {
  const createSuspenseComponent = useCallback((
    delay: number = 2000,
    componentName: string = 'TestComponent'
  ) => {
    const SuspenseComponent: React.FC = () => {
      throw new Promise(resolve => {
        setTimeout(resolve, delay)
      })
    }
    
    SuspenseComponent.displayName = componentName
    return SuspenseComponent
  }, [])

  const createFailingSuspenseComponent = useCallback((
    delay: number = 1000,
    componentName: string = 'FailingComponent'
  ) => {
    const FailingComponent: React.FC = () => {
      throw new Promise((_, reject) => {
        setTimeout(() => reject(new Error('Simulated loading failure')), delay)
      })
    }
    
    FailingComponent.displayName = componentName
    return FailingComponent
  }, [])

  const createDynamicImportSimulation = useCallback((delay: number = 1500) => {
    return () => new Promise<{ default: React.ComponentType }>(resolve => {
      setTimeout(() => {
        resolve({
          default: () => (
            <div data-testid="dynamic-component">
              Dynamically loaded component
            </div>
          ),
        })
      }, delay)
    })
  }, [])

  return {
    createSuspenseComponent,
    createFailingSuspenseComponent,
    createDynamicImportSimulation,
  }
}

// Development panel for testing suspense scenarios
export const SuspenseTestPanel: React.FC<{
  boundary?: React.RefObject<TestSuspenseBoundary>
}> = ({ boundary }) => {
  const [isVisible, setIsVisible] = useState(false)
  const [metrics, setMetrics] = useState<SuspenseMetrics | null>(null)
  const { createSuspenseComponent } = useSuspenseSimulation()

  const updateMetrics = () => {
    if (boundary?.current) {
      setMetrics(boundary.current.getMetrics())
    }
  }

  const resetMetrics = () => {
    if (boundary?.current) {
      boundary.current.resetMetrics()
      setMetrics(null)
    }
  }

  useEffect(() => {
    const interval = setInterval(updateMetrics, 1000)
    return () => clearInterval(interval)
  }, [boundary])

  if (process.env.NODE_ENV === 'production') {
    return null
  }

  return (
    <>
      <button
        onClick={() => setIsVisible(!isVisible)}
        style={{
          position: 'fixed',
          bottom: '70px',
          left: '16px',
          zIndex: 9998,
          background: '#3b82f6',
          color: 'white',
          border: 'none',
          padding: '8px 12px',
          borderRadius: '6px',
          fontSize: '12px',
          cursor: 'pointer',
        }}
        data-testid="suspense-debug-toggle"
      >
        Suspense Debug
      </button>
      
      {isVisible && (
        <div
          style={{
            position: 'fixed',
            bottom: '120px',
            left: '16px',
            width: '300px',
            background: 'white',
            border: '1px solid #e5e7eb',
            borderRadius: '12px',
            boxShadow: '0 4px 12px rgba(0,0,0,0.15)',
            zIndex: 9999,
            maxHeight: '400px',
            overflow: 'auto',
          }}
          data-testid="suspense-debug-panel"
        >
          <div style={{ 
            padding: '12px', 
            borderBottom: '1px solid #e5e7eb', 
            background: '#f8f9fa',
            borderRadius: '12px 12px 0 0',
          }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <strong style={{ fontSize: '0.875rem' }}>Suspense Metrics</strong>
              <button
                onClick={resetMetrics}
                style={{
                  background: '#ef4444',
                  color: 'white',
                  border: 'none',
                  padding: '4px 8px',
                  borderRadius: '4px',
                  fontSize: '10px',
                  cursor: 'pointer',
                }}
              >
                Reset
              </button>
            </div>
          </div>
          
          <div style={{ padding: '12px', fontSize: '12px' }}>
            {metrics ? (
              <div style={{ display: 'grid', gap: '8px' }}>
                <div><strong>Total Suspense Events:</strong> {metrics.suspenseCount}</div>
                <div><strong>Average Time:</strong> {metrics.averageSuspenseTime.toFixed(2)}ms</div>
                <div><strong>Longest Suspense:</strong> {metrics.longestSuspense.toFixed(2)}ms</div>
                <div><strong>Components Loaded:</strong> {metrics.componentsLoaded.length}</div>
                
                {metrics.loadingHistory.length > 0 && (
                  <details style={{ marginTop: '8px' }}>
                    <summary style={{ cursor: 'pointer', fontWeight: '600' }}>
                      Loading History
                    </summary>
                    <div style={{ marginTop: '8px', maxHeight: '150px', overflow: 'auto' }}>
                      {metrics.loadingHistory.slice(-5).map((item, index) => (
                        <div key={index} style={{ 
                          padding: '4px', 
                          background: '#f3f4f6', 
                          marginBottom: '4px',
                          borderRadius: '4px',
                        }}>
                          <div><strong>{item.component}</strong></div>
                          <div>Duration: {item.duration?.toFixed(2) || 'Loading...'}ms</div>
                        </div>
                      ))}
                    </div>
                  </details>
                )}
              </div>
            ) : (
              <div style={{ color: '#6b7280' }}>No suspense events recorded</div>
            )}
          </div>
        </div>
      )}
    </>
  )
}

// Higher-order component for easy suspense boundary wrapping
export const withSuspenseBoundary = <P extends object>(
  Component: React.ComponentType<P>,
  suspenseProps?: Partial<TestSuspenseBoundaryProps>
) => {
  const WrappedComponent = (props: P) => (
    <TestSuspenseBoundary {...suspenseProps}>
      <Component {...props} />
    </TestSuspenseBoundary>
  )
  
  WrappedComponent.displayName = `withSuspenseBoundary(${Component.displayName || Component.name})`
  return WrappedComponent
}