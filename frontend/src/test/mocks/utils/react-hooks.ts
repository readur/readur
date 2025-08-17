/**
 * React testing hooks for easy mock API setup and configuration
 * Provides hooks for managing mock state and configuration in React components
 */

import { useEffect, useState, useCallback, useRef } from 'react'
import { server, resetMockApi, useMockHandlers } from '../api/server'
import { MockConfig, MockState, UseMockApiOptions } from '../api/types'
import { TEST_SCENARIOS, getTestScenario } from '../fixtures/scenarios'
import { enableWebSocketMocking, disableWebSocketMocking, WebSocketTestUtils } from './websocket'
import { 
  setDocumentMockConfig, 
  setMockDocuments, 
  resetMockDocuments 
} from '../handlers/documents'
import { 
  setAuthMockConfig, 
  setCurrentUser, 
  resetAuthState 
} from '../handlers/auth'
import { 
  setSearchMockConfig, 
  resetSearchState 
} from '../handlers/search'

/**
 * Hook for managing mock API configuration
 */
export const useMockApi = (options: UseMockApiOptions = {}) => {
  const {
    scenario,
    customHandlers = [],
    resetOnUnmount = true,
    defaultDelay = 100,
  } = options

  const [isSetup, setIsSetup] = useState(false)
  const [currentScenario, setCurrentScenario] = useState<string | null>(scenario || null)
  const originalHandlers = useRef<any[]>([])

  // Setup mock API on mount
  useEffect(() => {
    if (!isSetup) {
      // Start MSW server if not already running
      if (!server.listenerCount('request')) {
        server.listen({ onUnhandledRequest: 'warn' })
      }

      // Enable WebSocket mocking
      enableWebSocketMocking({ autoConnect: true, messageDelay: 50 })

      // Apply default configuration
      const defaultConfig = { delay: defaultDelay, shouldFail: false }
      setDocumentMockConfig(defaultConfig)
      setAuthMockConfig(defaultConfig)
      setSearchMockConfig(defaultConfig)

      setIsSetup(true)
    }

    // Apply custom handlers
    if (customHandlers.length > 0) {
      useMockHandlers(...customHandlers)
      originalHandlers.current = customHandlers
    }

    // Apply scenario if specified
    if (scenario) {
      applyScenario(scenario)
    }

    return () => {
      if (resetOnUnmount) {
        resetMockApi()
        if (originalHandlers.current.length > 0) {
          // Remove custom handlers
          originalHandlers.current = []
        }
      }
    }
  }, [scenario, customHandlers, resetOnUnmount, defaultDelay, isSetup])

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (resetOnUnmount) {
        disableWebSocketMocking()
        WebSocketTestUtils.closeAllWebSockets()
      }
    }
  }, [resetOnUnmount])

  const applyScenario = useCallback((scenarioName: string) => {
    const scenarioData = getTestScenario(scenarioName)
    
    // Apply documents
    if (scenarioData.documents) {
      setMockDocuments(scenarioData.documents)
    }

    // Apply users and auth state
    if (scenarioData.users && scenarioData.users.length > 0) {
      setCurrentUser(scenarioData.users[0]) // Set first user as current
    }

    setCurrentScenario(scenarioName)
  }, [])

  const setNetworkConfig = useCallback((config: Partial<MockConfig>) => {
    setDocumentMockConfig(config)
    setAuthMockConfig(config)
    setSearchMockConfig(config)
  }, [])

  const simulateNetworkError = useCallback(() => {
    setNetworkConfig({
      shouldFail: true,
      errorCode: 500,
      errorMessage: 'Network Error',
    })
  }, [setNetworkConfig])

  const simulateSlowNetwork = useCallback((delay: number = 2000) => {
    setNetworkConfig({
      delay,
      shouldFail: false,
    })
  }, [setNetworkConfig])

  const reset = useCallback(() => {
    resetMockApi()
    resetMockDocuments()
    resetAuthState()
    resetSearchState()
    setCurrentScenario(null)
  }, [])

  return {
    isSetup,
    currentScenario,
    applyScenario,
    setNetworkConfig,
    simulateNetworkError,
    simulateSlowNetwork,
    reset,
  }
}

/**
 * Hook for managing mock documents
 */
export const useMockDocuments = () => {
  const [documents, setDocuments] = useState<any[]>([])

  const addDocument = useCallback((document: any) => {
    setDocuments(prev => [document, ...prev])
    setMockDocuments([document, ...documents])
  }, [documents])

  const removeDocument = useCallback((documentId: string) => {
    const filtered = documents.filter(doc => doc.id !== documentId)
    setDocuments(filtered)
    setMockDocuments(filtered)
  }, [documents])

  const updateDocument = useCallback((documentId: string, updates: any) => {
    const updated = documents.map(doc => 
      doc.id === documentId ? { ...doc, ...updates } : doc
    )
    setDocuments(updated)
    setMockDocuments(updated)
  }, [documents])

  const clearDocuments = useCallback(() => {
    setDocuments([])
    setMockDocuments([])
  }, [])

  return {
    documents,
    addDocument,
    removeDocument,
    updateDocument,
    clearDocuments,
    setDocuments: (docs: any[]) => {
      setDocuments(docs)
      setMockDocuments(docs)
    },
  }
}

/**
 * Hook for managing mock authentication state
 */
export const useMockAuth = () => {
  const [isAuthenticated, setIsAuthenticated] = useState(false)
  const [currentUser, setCurrentUserState] = useState<any>(null)

  const login = useCallback((user: any) => {
    setCurrentUser(user)
    setCurrentUserState(user)
    setIsAuthenticated(true)
  }, [])

  const logout = useCallback(() => {
    setCurrentUser(null)
    setCurrentUserState(null)
    setIsAuthenticated(false)
  }, [])

  const setUser = useCallback((user: any) => {
    setCurrentUser(user)
    setCurrentUserState(user)
    setIsAuthenticated(!!user)
  }, [])

  return {
    isAuthenticated,
    currentUser,
    login,
    logout,
    setUser,
  }
}

/**
 * Hook for managing WebSocket testing
 */
export const useMockWebSocket = (url?: string) => {
  const [webSocket, setWebSocket] = useState<any>(null)
  const [connectionState, setConnectionState] = useState<'disconnected' | 'connecting' | 'connected'>('disconnected')
  const [lastMessage, setLastMessage] = useState<any>(null)

  useEffect(() => {
    if (url) {
      const ws = WebSocketTestUtils.createWebSocket(url)
      
      ws.addEventListener('open', () => setConnectionState('connected'))
      ws.addEventListener('close', () => setConnectionState('disconnected'))
      ws.addEventListener('message', (event) => {
        try {
          const message = JSON.parse(event.data)
          setLastMessage(message)
        } catch (e) {
          setLastMessage({ type: 'raw', data: event.data })
        }
      })

      setWebSocket(ws)
      setConnectionState('connecting')

      return () => {
        ws.close()
      }
    }
  }, [url])

  const sendMessage = useCallback((message: any) => {
    if (webSocket && connectionState === 'connected') {
      webSocket.send(JSON.stringify(message))
    }
  }, [webSocket, connectionState])

  const simulateProgress = useCallback((sourceId: string, scenario: string = 'in_progress') => {
    if (webSocket && connectionState === 'connected') {
      webSocket.startSyncProgressSimulation(sourceId, scenario)
    }
  }, [webSocket, connectionState])

  const simulateError = useCallback((error: string) => {
    if (webSocket && connectionState === 'connected') {
      webSocket.simulateError(error)
    }
  }, [webSocket, connectionState])

  return {
    webSocket,
    connectionState,
    lastMessage,
    sendMessage,
    simulateProgress,
    simulateError,
  }
}

/**
 * Hook for managing test scenarios
 */
export const useTestScenario = (initialScenario?: string) => {
  const [currentScenario, setCurrentScenario] = useState<string | null>(initialScenario || null)
  const [scenarioData, setScenarioData] = useState<MockState | null>(null)

  const loadScenario = useCallback((scenarioName: string) => {
    const data = getTestScenario(scenarioName)
    setScenarioData(data)
    setCurrentScenario(scenarioName)

    // Apply the scenario to mock handlers
    if (data.documents) setMockDocuments(data.documents)
    if (data.users && data.users.length > 0) setCurrentUser(data.users[0])
  }, [])

  const listAvailableScenarios = useCallback(() => {
    return Object.keys(TEST_SCENARIOS)
  }, [])

  return {
    currentScenario,
    scenarioData,
    loadScenario,
    listAvailableScenarios,
  }
}

/**
 * Hook for testing error conditions
 */
export const useMockErrors = () => {
  const [errorConfig, setErrorConfig] = useState<MockConfig | null>(null)

  const simulateError = useCallback((errorType: 'network' | 'server' | 'auth' | 'timeout', delay?: number) => {
    let config: MockConfig

    switch (errorType) {
      case 'network':
        config = { shouldFail: true, errorCode: 0, errorMessage: 'Network Error', delay: delay || 0 }
        break
      case 'server':
        config = { shouldFail: true, errorCode: 500, errorMessage: 'Internal Server Error', delay: delay || 100 }
        break
      case 'auth':
        config = { shouldFail: true, errorCode: 401, errorMessage: 'Unauthorized', delay: delay || 100 }
        break
      case 'timeout':
        config = { delay: 'infinite', shouldFail: false }
        break
      default:
        config = { shouldFail: true, errorCode: 500, errorMessage: 'Unknown Error', delay: delay || 100 }
    }

    setErrorConfig(config)
    setDocumentMockConfig(config)
    setAuthMockConfig(config)
    setSearchMockConfig(config)
  }, [])

  const clearErrors = useCallback(() => {
    const defaultConfig = { delay: 100, shouldFail: false }
    setErrorConfig(null)
    setDocumentMockConfig(defaultConfig)
    setAuthMockConfig(defaultConfig)
    setSearchMockConfig(defaultConfig)
  }, [])

  return {
    errorConfig,
    simulateError,
    clearErrors,
  }
}

/**
 * Utility hook for common test operations
 */
export const useMockTestUtils = () => {
  const documents = useMockDocuments()
  const auth = useMockAuth()
  const errors = useMockErrors()
  const api = useMockApi()

  const setupEmptyState = useCallback(() => {
    documents.clearDocuments()
    auth.logout()
    errors.clearErrors()
    api.reset()
  }, [documents, auth, errors, api])

  const setupTypicalUser = useCallback(() => {
    const user = {
      id: 'test-user-1',
      username: 'testuser',
      email: 'test@example.com',
      role: 'user',
    }
    auth.login(user)
    api.applyScenario('ACTIVE_SYSTEM')
  }, [auth, api])

  const setupAdminUser = useCallback(() => {
    const admin = {
      id: 'admin-1',
      username: 'admin',
      email: 'admin@example.com',
      role: 'admin',
    }
    auth.login(admin)
    api.applyScenario('ACTIVE_SYSTEM')
  }, [auth, api])

  const waitForApiCall = useCallback(async (timeout = 5000) => {
    return new Promise((resolve) => {
      setTimeout(resolve, Math.min(api.currentScenario ? 100 : 50, timeout))
    })
  }, [api.currentScenario])

  return {
    setupEmptyState,
    setupTypicalUser,
    setupAdminUser,
    waitForApiCall,
    ...documents,
    ...auth,
    ...errors,
    ...api,
  }
}