/**
 * MockApiProvider - Comprehensive API mocking context provider
 * Provides centralized mock API configuration for all child components
 */

import React, { createContext, useContext, useEffect, useState, useCallback, ReactNode } from 'react'
import { useMockApi, useMockTestUtils } from '../../utils/react-hooks'
import { MockConfig } from '../../api/types'
import { NETWORK_CONDITIONS } from '../../utils/config'

export interface MockApiContextType {
  // Core API management
  isSetup: boolean
  currentScenario: string | null
  
  // Scenario management
  applyScenario: (scenarioName: string) => void
  resetScenario: () => void
  
  // Network simulation
  setNetworkCondition: (condition: keyof typeof NETWORK_CONDITIONS | MockConfig) => void
  simulateNetworkError: () => void
  simulateSlowNetwork: (delay?: number) => void
  resetNetworkCondition: () => void
  
  // State management
  reset: () => void
  getCurrentState: () => any
  
  // Real-time updates
  enableRealTimeUpdates: () => void
  disableRealTimeUpdates: () => void
  
  // Performance monitoring
  startPerformanceMonitoring: () => void
  stopPerformanceMonitoring: () => void
  getPerformanceMetrics: () => any
}

const MockApiContext = createContext<MockApiContextType | null>(null)

export interface MockApiProviderProps {
  children: ReactNode
  scenario?: string
  networkCondition?: keyof typeof NETWORK_CONDITIONS
  enableWebSocket?: boolean
  enablePerformanceMonitoring?: boolean
  autoReset?: boolean
  defaultDelay?: number
}

export const MockApiProvider: React.FC<MockApiProviderProps> = ({
  children,
  scenario,
  networkCondition = 'fast',
  enableWebSocket = false,
  enablePerformanceMonitoring = false,
  autoReset = true,
  defaultDelay = 100,
}) => {
  const [performanceMonitoring, setPerformanceMonitoring] = useState(enablePerformanceMonitoring)
  const [realTimeUpdates, setRealTimeUpdates] = useState(enableWebSocket)
  const [performanceMetrics, setPerformanceMetrics] = useState<any>({})
  
  const mockApi = useMockApi({
    scenario,
    resetOnUnmount: autoReset,
    defaultDelay,
  })
  
  const testUtils = useMockTestUtils()

  // Apply initial network condition
  useEffect(() => {
    if (networkCondition && NETWORK_CONDITIONS[networkCondition]) {
      mockApi.setNetworkConfig(NETWORK_CONDITIONS[networkCondition])
    }
  }, [networkCondition, mockApi])

  // Performance monitoring setup
  useEffect(() => {
    if (performanceMonitoring) {
      const observer = new PerformanceObserver((list) => {
        const entries = list.getEntries()
        setPerformanceMetrics(prev => ({
          ...prev,
          navigationEntries: entries.filter(e => e.entryType === 'navigation'),
          resourceEntries: entries.filter(e => e.entryType === 'resource'),
          measureEntries: entries.filter(e => e.entryType === 'measure'),
        }))
      })
      
      observer.observe({ entryTypes: ['navigation', 'resource', 'measure'] })
      
      return () => observer.disconnect()
    }
  }, [performanceMonitoring])

  const setNetworkCondition = useCallback((condition: keyof typeof NETWORK_CONDITIONS | MockConfig) => {
    if (typeof condition === 'string' && NETWORK_CONDITIONS[condition]) {
      mockApi.setNetworkConfig(NETWORK_CONDITIONS[condition])
    } else if (typeof condition === 'object') {
      mockApi.setNetworkConfig(condition)
    }
  }, [mockApi])

  const resetNetworkCondition = useCallback(() => {
    mockApi.setNetworkConfig({ delay: defaultDelay, shouldFail: false })
  }, [mockApi, defaultDelay])

  const getCurrentState = useCallback(() => {
    return {
      scenario: mockApi.currentScenario,
      isSetup: mockApi.isSetup,
      realTimeUpdates,
      performanceMonitoring,
      performanceMetrics,
    }
  }, [mockApi.currentScenario, mockApi.isSetup, realTimeUpdates, performanceMonitoring, performanceMetrics])

  const startPerformanceMonitoring = useCallback(() => {
    setPerformanceMonitoring(true)
    performance.mark('mock-test-start')
  }, [])

  const stopPerformanceMonitoring = useCallback(() => {
    performance.mark('mock-test-end')
    performance.measure('mock-test-duration', 'mock-test-start', 'mock-test-end')
    setPerformanceMonitoring(false)
  }, [])

  const getPerformanceMetrics = useCallback(() => {
    const measures = performance.getEntriesByType('measure')
    const navigation = performance.getEntriesByType('navigation')
    
    return {
      ...performanceMetrics,
      testDuration: measures.find(m => m.name === 'mock-test-duration')?.duration || 0,
      navigationTiming: navigation[0] || null,
      timestamp: Date.now(),
    }
  }, [performanceMetrics])

  const contextValue: MockApiContextType = {
    isSetup: mockApi.isSetup,
    currentScenario: mockApi.currentScenario,
    applyScenario: mockApi.applyScenario,
    resetScenario: () => {
      mockApi.reset()
      testUtils.setupEmptyState()
    },
    setNetworkCondition,
    simulateNetworkError: mockApi.simulateNetworkError,
    simulateSlowNetwork: mockApi.simulateSlowNetwork,
    resetNetworkCondition,
    reset: mockApi.reset,
    getCurrentState,
    enableRealTimeUpdates: () => setRealTimeUpdates(true),
    disableRealTimeUpdates: () => setRealTimeUpdates(false),
    startPerformanceMonitoring,
    stopPerformanceMonitoring,
    getPerformanceMetrics,
  }

  return (
    <MockApiContext.Provider value={contextValue}>
      {children}
    </MockApiContext.Provider>
  )
}

export const useMockApiContext = (): MockApiContextType => {
  const context = useContext(MockApiContext)
  if (!context) {
    throw new Error('useMockApiContext must be used within a MockApiProvider')
  }
  return context
}

// High-order component for wrapping components with mock API
export const withMockApi = <P extends object>(
  Component: React.ComponentType<P>,
  providerProps?: Partial<MockApiProviderProps>
) => {
  const WrappedComponent = (props: P) => (
    <MockApiProvider {...providerProps}>
      <Component {...props} />
    </MockApiProvider>
  )
  
  WrappedComponent.displayName = `withMockApi(${Component.displayName || Component.name})`
  return WrappedComponent
}