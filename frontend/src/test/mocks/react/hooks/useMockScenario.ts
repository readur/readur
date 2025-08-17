/**
 * useMockScenario - Dynamic test scenario management
 * Provides easy switching between test scenarios with state management
 */

import { useState, useCallback, useEffect, useRef } from 'react'
import { useMockApiContext } from '../providers/MockApiProvider'
import { useMockAuthContext } from '../providers/MockAuthProvider'
import { useMockNotificationContext } from '../providers/MockNotificationProvider'
import { useMockWebSocketContext } from '../providers/MockWebSocketProvider'
import { TEST_SCENARIOS, getTestScenario } from '../../fixtures/scenarios'
import type { MockState } from '../../api/types'

export interface ScenarioConfig {
  includeAuth?: boolean
  includeNotifications?: boolean
  includeWebSocket?: boolean
  resetOnChange?: boolean
  persistState?: boolean
  onScenarioChange?: (scenarioName: string, data: MockState) => void
}

export interface UseTestScenarioReturn {
  // Current state
  currentScenario: string | null
  scenarioData: MockState | null
  availableScenarios: string[]
  isTransitioning: boolean
  
  // Scenario management
  loadScenario: (scenarioName: string) => Promise<void>
  resetScenario: () => void
  createCustomScenario: (name: string, data: Partial<MockState>) => void
  
  // Quick scenario switches
  switchToEmpty: () => Promise<void>
  switchToActive: () => Promise<void>
  switchToProblematic: () => Promise<void>
  switchToMultiUser: () => Promise<void>
  
  // State utilities
  getScenarioPreview: (scenarioName: string) => ScenarioPreview
  saveCurrentState: () => string
  restoreState: (stateId: string) => void
  
  // Transition management
  queueScenarioTransition: (scenarios: string[], interval?: number) => void
  stopTransitions: () => void
}

export interface ScenarioPreview {
  name: string
  description: string
  documentCount: number
  userCount: number
  hasErrors: boolean
  estimatedSetupTime: number
}

const customScenarios = new Map<string, MockState>()
const savedStates = new Map<string, MockState>()

export const useMockScenario = (config: ScenarioConfig = {}): UseTestScenarioReturn => {
  const {
    includeAuth = true,
    includeNotifications = true,
    includeWebSocket = true,
    resetOnChange = true,
    persistState = false,
    onScenarioChange,
  } = config

  const [currentScenario, setCurrentScenario] = useState<string | null>(null)
  const [scenarioData, setScenarioData] = useState<MockState | null>(null)
  const [isTransitioning, setIsTransitioning] = useState(false)
  const [transitionQueue, setTransitionQueue] = useState<string[]>([])
  
  const transitionIntervalRef = useRef<NodeJS.Timeout | null>(null)
  
  // Context hooks (with fallbacks if providers not available)
  const apiContext = useMockApiContext?.() || null
  const authContext = useMockAuthContext?.() || null
  const notificationContext = useMockNotificationContext?.() || null
  const webSocketContext = useMockWebSocketContext?.() || null

  const availableScenarios = [
    ...Object.keys(TEST_SCENARIOS),
    ...Array.from(customScenarios.keys()),
  ]

  const loadScenario = useCallback(async (scenarioName: string) => {
    setIsTransitioning(true)
    
    try {
      // Reset current state if configured
      if (resetOnChange) {
        apiContext?.reset?.()
        authContext?.reset?.()
        notificationContext?.reset?.()
        webSocketContext?.reset?.()
      }

      // Load scenario data
      let data: MockState
      if (customScenarios.has(scenarioName)) {
        data = customScenarios.get(scenarioName)!
      } else {
        data = getTestScenario(scenarioName)
      }

      // Apply scenario to different contexts
      if (apiContext && data) {
        apiContext.applyScenario(scenarioName)
      }

      if (authContext && includeAuth && data.users?.length) {
        await authContext.login(data.users[0])
      }

      if (notificationContext && includeNotifications) {
        // Clear existing notifications
        notificationContext.clearNotifications()
        
        // Add scenario-specific notifications
        if (scenarioName.includes('PROBLEMATIC')) {
          notificationContext.simulateNotificationFlow('error_cascade')
        } else if (scenarioName.includes('ACTIVE')) {
          notificationContext.simulateSystemNotification('upload_complete')
        }
      }

      if (webSocketContext && includeWebSocket) {
        if (scenarioName.includes('SYNC') || scenarioName.includes('PROGRESS')) {
          webSocketContext.simulateSyncProgress('test-source-1')
        }
      }

      setCurrentScenario(scenarioName)
      setScenarioData(data)
      
      // Save state if persistence enabled
      if (persistState) {
        savedStates.set(`auto_${Date.now()}`, data)
      }

      // Callback notification
      onScenarioChange?.(scenarioName, data)
      
    } catch (error) {
      console.error('Failed to load scenario:', error)
      throw error
    } finally {
      setIsTransitioning(false)
    }
  }, [
    resetOnChange, 
    includeAuth, 
    includeNotifications, 
    includeWebSocket, 
    persistState,
    onScenarioChange,
    apiContext,
    authContext,
    notificationContext,
    webSocketContext
  ])

  const resetScenario = useCallback(() => {
    apiContext?.reset?.()
    authContext?.reset?.()
    notificationContext?.reset?.()
    webSocketContext?.reset?.()
    
    setCurrentScenario(null)
    setScenarioData(null)
  }, [apiContext, authContext, notificationContext, webSocketContext])

  const createCustomScenario = useCallback((name: string, data: Partial<MockState>) => {
    const fullData: MockState = {
      documents: [],
      users: [],
      sources: [],
      labels: [],
      queueStats: null,
      searchResults: [],
      ...data,
    }
    
    customScenarios.set(name, fullData)
  }, [])

  // Quick scenario switches
  const switchToEmpty = useCallback(() => loadScenario('EMPTY_SYSTEM'), [loadScenario])
  const switchToActive = useCallback(() => loadScenario('ACTIVE_SYSTEM'), [loadScenario])
  const switchToProblematic = useCallback(() => loadScenario('PROBLEMATIC_SYSTEM'), [loadScenario])
  const switchToMultiUser = useCallback(() => loadScenario('MULTI_USER_SYSTEM'), [loadScenario])

  const getScenarioPreview = useCallback((scenarioName: string): ScenarioPreview => {
    let data: MockState
    
    if (customScenarios.has(scenarioName)) {
      data = customScenarios.get(scenarioName)!
    } else {
      data = getTestScenario(scenarioName)
    }

    const documentCount = data.documents?.length || 0
    const userCount = data.users?.length || 0
    const hasErrors = scenarioName.includes('PROBLEMATIC') || scenarioName.includes('ERROR')
    
    // Estimate setup time based on complexity
    let estimatedSetupTime = 100 // Base time
    estimatedSetupTime += documentCount * 5 // 5ms per document
    estimatedSetupTime += userCount * 10 // 10ms per user
    if (hasErrors) estimatedSetupTime += 200 // Error scenarios take longer

    return {
      name: scenarioName,
      description: getScenarioDescription(scenarioName),
      documentCount,
      userCount,
      hasErrors,
      estimatedSetupTime,
    }
  }, [])

  const saveCurrentState = useCallback((): string => {
    if (!scenarioData) return ''
    
    const stateId = `manual_${Date.now()}`
    savedStates.set(stateId, scenarioData)
    return stateId
  }, [scenarioData])

  const restoreState = useCallback((stateId: string) => {
    const state = savedStates.get(stateId)
    if (state) {
      setScenarioData(state)
      setCurrentScenario(`restored_${stateId}`)
    }
  }, [])

  const queueScenarioTransition = useCallback((scenarios: string[], interval: number = 3000) => {
    setTransitionQueue(scenarios)
    
    let currentIndex = 0
    transitionIntervalRef.current = setInterval(async () => {
      if (currentIndex < scenarios.length) {
        await loadScenario(scenarios[currentIndex])
        currentIndex++
      } else {
        // Loop back to start
        currentIndex = 0
      }
    }, interval)
  }, [loadScenario])

  const stopTransitions = useCallback(() => {
    if (transitionIntervalRef.current) {
      clearInterval(transitionIntervalRef.current)
      transitionIntervalRef.current = null
    }
    setTransitionQueue([])
  }, [])

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (transitionIntervalRef.current) {
        clearInterval(transitionIntervalRef.current)
      }
    }
  }, [])

  return {
    currentScenario,
    scenarioData,
    availableScenarios,
    isTransitioning,
    loadScenario,
    resetScenario,
    createCustomScenario,
    switchToEmpty,
    switchToActive,
    switchToProblematic,
    switchToMultiUser,
    getScenarioPreview,
    saveCurrentState,
    restoreState,
    queueScenarioTransition,
    stopTransitions,
  }
}

function getScenarioDescription(scenarioName: string): string {
  const descriptions: Record<string, string> = {
    EMPTY_SYSTEM: 'Clean system with no documents or users',
    NEW_USER_SETUP: 'Fresh user account with minimal data',
    ACTIVE_SYSTEM: 'Normal operation with realistic data',
    SYSTEM_UNDER_LOAD: 'Heavy usage with large datasets',
    MULTI_USER_SYSTEM: 'Multiple users with shared documents',
    PROBLEMATIC_SYSTEM: 'System with various error conditions',
  }
  
  return descriptions[scenarioName] || 'Custom test scenario'
}