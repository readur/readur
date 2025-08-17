/**
 * useMockNetworkCondition - Network state simulation for testing
 * Provides comprehensive network condition simulation with realistic scenarios
 */

import { useState, useCallback, useEffect, useRef } from 'react'
import { useMockApiContext } from '../providers/MockApiProvider'
import { NETWORK_CONDITIONS } from '../../utils/config'
import type { MockConfig } from '../../api/types'

export interface NetworkProfile {
  name: string
  delay: number
  jitter?: number
  packetLoss?: number
  bandwidth?: number
  description: string
}

export interface UseMockNetworkConditionReturn {
  // Current state
  currentCondition: string
  activeProfile: NetworkProfile | null
  isOffline: boolean
  connectionQuality: 'excellent' | 'good' | 'poor' | 'disconnected'
  
  // Network conditions
  setCondition: (condition: keyof typeof NETWORK_CONDITIONS) => void
  setCustomCondition: (config: MockConfig) => void
  goOffline: () => void
  goOnline: () => void
  
  // Dynamic conditions
  simulateIntermittentConnection: (interval?: number) => void
  simulateGradualDegradation: (duration?: number) => void
  simulateNetworkRecovery: (duration?: number) => void
  simulateSpeedTest: () => Promise<NetworkTestResult>
  
  // Real-world scenarios
  simulateMobileConnection: (type?: 'edge' | '3g' | '4g' | '5g') => void
  simulateWifiConnection: (quality?: 'poor' | 'good' | 'excellent') => void
  simulateServerLoad: (level?: 'low' | 'medium' | 'high') => void
  
  // Monitoring
  startLatencyMonitoring: () => void
  stopLatencyMonitoring: () => void
  getNetworkStats: () => NetworkStats
  
  // Reset
  reset: () => void
}

export interface NetworkTestResult {
  latency: number
  downloadSpeed: number
  uploadSpeed: number
  packetLoss: number
  jitter: number
  quality: 'excellent' | 'good' | 'poor' | 'disconnected'
}

export interface NetworkStats {
  averageLatency: number
  maxLatency: number
  minLatency: number
  requestCount: number
  timeoutCount: number
  errorCount: number
  uptime: number
  lastTest: NetworkTestResult | null
}

const NETWORK_PROFILES: Record<string, NetworkProfile> = {
  'fiber': {
    name: 'Fiber Broadband',
    delay: 10,
    jitter: 2,
    packetLoss: 0.1,
    bandwidth: 1000,
    description: 'High-speed fiber connection',
  },
  'cable': {
    name: 'Cable Internet',
    delay: 25,
    jitter: 5,
    packetLoss: 0.2,
    bandwidth: 100,
    description: 'Standard cable broadband',
  },
  'dsl': {
    name: 'DSL',
    delay: 50,
    jitter: 10,
    packetLoss: 0.5,
    bandwidth: 25,
    description: 'DSL connection',
  },
  'wifi_good': {
    name: 'Good WiFi',
    delay: 30,
    jitter: 15,
    packetLoss: 0.3,
    bandwidth: 50,
    description: 'Stable WiFi connection',
  },
  'wifi_poor': {
    name: 'Poor WiFi',
    delay: 150,
    jitter: 50,
    packetLoss: 2,
    bandwidth: 5,
    description: 'Unstable WiFi with interference',
  },
  '5g': {
    name: '5G Mobile',
    delay: 20,
    jitter: 8,
    packetLoss: 0.2,
    bandwidth: 200,
    description: '5G mobile connection',
  },
  '4g': {
    name: '4G/LTE',
    delay: 50,
    jitter: 20,
    packetLoss: 0.5,
    bandwidth: 50,
    description: '4G LTE mobile connection',
  },
  '3g': {
    name: '3G Mobile',
    delay: 200,
    jitter: 100,
    packetLoss: 2,
    bandwidth: 3,
    description: '3G mobile connection',
  },
  'edge': {
    name: 'EDGE/2G',
    delay: 800,
    jitter: 400,
    packetLoss: 5,
    bandwidth: 0.3,
    description: 'Slow mobile connection',
  },
  'satellite': {
    name: 'Satellite',
    delay: 600,
    jitter: 100,
    packetLoss: 1,
    bandwidth: 25,
    description: 'Satellite internet connection',
  },
}

export const useMockNetworkCondition = (): UseMockNetworkConditionReturn => {
  const [currentCondition, setCurrentCondition] = useState('fast')
  const [activeProfile, setActiveProfile] = useState<NetworkProfile | null>(null)
  const [isOffline, setIsOffline] = useState(false)
  const [connectionQuality, setConnectionQuality] = useState<'excellent' | 'good' | 'poor' | 'disconnected'>('excellent')
  const [latencyHistory, setLatencyHistory] = useState<number[]>([])
  const [networkStats, setNetworkStats] = useState<NetworkStats>({
    averageLatency: 0,
    maxLatency: 0,
    minLatency: 0,
    requestCount: 0,
    timeoutCount: 0,
    errorCount: 0,
    uptime: Date.now(),
    lastTest: null,
  })
  
  const apiContext = useMockApiContext?.()
  const intermittentIntervalRef = useRef<NodeJS.Timeout | null>(null)
  const latencyMonitorRef = useRef<NodeJS.Timeout | null>(null)
  const degradationIntervalRef = useRef<NodeJS.Timeout | null>(null)

  const updateConnectionQuality = useCallback((delay: number) => {
    if (delay < 50) {
      setConnectionQuality('excellent')
    } else if (delay < 150) {
      setConnectionQuality('good')
    } else if (delay < 500) {
      setConnectionQuality('poor')
    } else {
      setConnectionQuality('disconnected')
    }
  }, [])

  const setCondition = useCallback((condition: keyof typeof NETWORK_CONDITIONS) => {
    const config = NETWORK_CONDITIONS[condition]
    if (!config) return

    setCurrentCondition(condition)
    setIsOffline(condition === 'offline')
    
    if (condition === 'offline') {
      setConnectionQuality('disconnected')
      setActiveProfile(null)
    } else {
      updateConnectionQuality(config.delay || 0)
      
      // Try to match with a network profile
      const matchingProfile = Object.values(NETWORK_PROFILES).find(profile => 
        Math.abs(profile.delay - (config.delay || 0)) < 20
      )
      setActiveProfile(matchingProfile || null)
    }

    apiContext?.setNetworkCondition?.(config)
  }, [apiContext, updateConnectionQuality])

  const setCustomCondition = useCallback((config: MockConfig) => {
    setCurrentCondition('custom')
    setIsOffline(config.shouldFail === true && config.errorCode === 0)
    updateConnectionQuality(config.delay as number || 0)
    setActiveProfile(null)
    
    apiContext?.setNetworkCondition?.(config)
  }, [apiContext, updateConnectionQuality])

  const goOffline = useCallback(() => {
    setCondition('offline')
  }, [setCondition])

  const goOnline = useCallback(() => {
    setCondition('fast')
  }, [setCondition])

  const simulateIntermittentConnection = useCallback((interval: number = 5000) => {
    let isCurrentlyOnline = true
    
    intermittentIntervalRef.current = setInterval(() => {
      if (isCurrentlyOnline) {
        goOffline()
      } else {
        goOnline()
      }
      isCurrentlyOnline = !isCurrentlyOnline
    }, interval)
  }, [goOffline, goOnline])

  const simulateGradualDegradation = useCallback((duration: number = 10000) => {
    const steps = 10
    const stepDuration = duration / steps
    const initialDelay = 50
    const finalDelay = 2000
    const delayIncrement = (finalDelay - initialDelay) / steps
    
    let currentStep = 0
    
    degradationIntervalRef.current = setInterval(() => {
      const currentDelay = initialDelay + (delayIncrement * currentStep)
      setCustomCondition({ delay: currentDelay, shouldFail: false })
      
      currentStep++
      if (currentStep >= steps) {
        if (degradationIntervalRef.current) {
          clearInterval(degradationIntervalRef.current)
        }
      }
    }, stepDuration)
  }, [setCustomCondition])

  const simulateNetworkRecovery = useCallback((duration: number = 5000) => {
    const steps = 8
    const stepDuration = duration / steps
    const initialDelay = 2000
    const finalDelay = 50
    const delayDecrement = (initialDelay - finalDelay) / steps
    
    let currentStep = 0
    
    degradationIntervalRef.current = setInterval(() => {
      const currentDelay = initialDelay - (delayDecrement * currentStep)
      setCustomCondition({ delay: Math.max(currentDelay, finalDelay), shouldFail: false })
      
      currentStep++
      if (currentStep >= steps) {
        if (degradationIntervalRef.current) {
          clearInterval(degradationIntervalRef.current)
        }
      }
    }, stepDuration)
  }, [setCustomCondition])

  const simulateSpeedTest = useCallback(async (): Promise<NetworkTestResult> => {
    const startTime = Date.now()
    
    // Simulate network test
    await new Promise(resolve => setTimeout(resolve, 100))
    
    const latency = activeProfile?.delay || 50
    const jitter = activeProfile?.jitter || 10
    const packetLoss = activeProfile?.packetLoss || 0.1
    const bandwidth = activeProfile?.bandwidth || 100
    
    // Calculate simulated speeds (simplified)
    const downloadSpeed = bandwidth * (1 - packetLoss / 100) * Math.random() * 0.8 + 0.2
    const uploadSpeed = downloadSpeed * 0.8 // Upload typically slower
    
    const result: NetworkTestResult = {
      latency: latency + (Math.random() - 0.5) * jitter,
      downloadSpeed,
      uploadSpeed,
      packetLoss,
      jitter,
      quality: connectionQuality,
    }
    
    setNetworkStats(prev => ({ ...prev, lastTest: result }))
    return result
  }, [activeProfile, connectionQuality])

  const simulateMobileConnection = useCallback((type: 'edge' | '3g' | '4g' | '5g' = '4g') => {
    const profile = NETWORK_PROFILES[type]
    if (profile) {
      setActiveProfile(profile)
      setCustomCondition({
        delay: profile.delay + Math.random() * (profile.jitter || 0),
        shouldFail: Math.random() * 100 < (profile.packetLoss || 0),
      })
      setCurrentCondition(`mobile_${type}`)
    }
  }, [setCustomCondition])

  const simulateWifiConnection = useCallback((quality: 'poor' | 'good' | 'excellent' = 'good') => {
    const profileKey = quality === 'poor' ? 'wifi_poor' : 'wifi_good'
    const profile = NETWORK_PROFILES[profileKey]
    
    if (profile) {
      setActiveProfile(profile)
      setCustomCondition({
        delay: profile.delay + Math.random() * (profile.jitter || 0),
        shouldFail: Math.random() * 100 < (profile.packetLoss || 0),
      })
      setCurrentCondition(`wifi_${quality}`)
    }
  }, [setCustomCondition])

  const simulateServerLoad = useCallback((level: 'low' | 'medium' | 'high' = 'medium') => {
    const delays = { low: 50, medium: 200, high: 800 }
    const delay = delays[level]
    
    setCustomCondition({
      delay: delay + Math.random() * 100,
      shouldFail: level === 'high' && Math.random() < 0.1, // 10% failure on high load
    })
    setCurrentCondition(`server_load_${level}`)
  }, [setCustomCondition])

  const startLatencyMonitoring = useCallback(() => {
    latencyMonitorRef.current = setInterval(() => {
      const currentLatency = activeProfile?.delay || 50
      const jitter = activeProfile?.jitter || 0
      const measuredLatency = currentLatency + (Math.random() - 0.5) * jitter
      
      setLatencyHistory(prev => {
        const newHistory = [...prev, measuredLatency].slice(-50) // Keep last 50 measurements
        
        // Update stats
        setNetworkStats(prevStats => ({
          ...prevStats,
          averageLatency: newHistory.reduce((sum, lat) => sum + lat, 0) / newHistory.length,
          maxLatency: Math.max(...newHistory),
          minLatency: Math.min(...newHistory),
          requestCount: prevStats.requestCount + 1,
        }))
        
        return newHistory
      })
    }, 1000)
  }, [activeProfile])

  const stopLatencyMonitoring = useCallback(() => {
    if (latencyMonitorRef.current) {
      clearInterval(latencyMonitorRef.current)
      latencyMonitorRef.current = null
    }
  }, [])

  const getNetworkStats = useCallback((): NetworkStats => {
    return {
      ...networkStats,
      uptime: Date.now() - networkStats.uptime,
    }
  }, [networkStats])

  const reset = useCallback(() => {
    setCurrentCondition('fast')
    setActiveProfile(null)
    setIsOffline(false)
    setConnectionQuality('excellent')
    setLatencyHistory([])
    setNetworkStats({
      averageLatency: 0,
      maxLatency: 0,
      minLatency: 0,
      requestCount: 0,
      timeoutCount: 0,
      errorCount: 0,
      uptime: Date.now(),
      lastTest: null,
    })
    
    // Clear all intervals
    if (intermittentIntervalRef.current) {
      clearInterval(intermittentIntervalRef.current)
    }
    if (latencyMonitorRef.current) {
      clearInterval(latencyMonitorRef.current)
    }
    if (degradationIntervalRef.current) {
      clearInterval(degradationIntervalRef.current)
    }
    
    apiContext?.resetNetworkCondition?.()
  }, [apiContext])

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (intermittentIntervalRef.current) {
        clearInterval(intermittentIntervalRef.current)
      }
      if (latencyMonitorRef.current) {
        clearInterval(latencyMonitorRef.current)
      }
      if (degradationIntervalRef.current) {
        clearInterval(degradationIntervalRef.current)
      }
    }
  }, [])

  return {
    currentCondition,
    activeProfile,
    isOffline,
    connectionQuality,
    setCondition,
    setCustomCondition,
    goOffline,
    goOnline,
    simulateIntermittentConnection,
    simulateGradualDegradation,
    simulateNetworkRecovery,
    simulateSpeedTest,
    simulateMobileConnection,
    simulateWifiConnection,
    simulateServerLoad,
    startLatencyMonitoring,
    stopLatencyMonitoring,
    getNetworkStats,
    reset,
  }
}