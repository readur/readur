/**
 * MockWebSocketProvider - WebSocket connection testing
 * Provides comprehensive WebSocket mocking with realistic connection states and message flows
 */

import React, { createContext, useContext, useState, useCallback, useEffect, useRef, ReactNode } from 'react'
import { useMockWebSocket } from '../../utils/react-hooks'
import { WebSocketTestUtils } from '../../utils/websocket'

export interface MockWebSocketContextType {
  // Connection state
  connectionState: 'disconnected' | 'connecting' | 'connected' | 'reconnecting' | 'error'
  lastMessage: any
  messageHistory: any[]
  connectionAttempts: number
  
  // Connection management
  connect: (url?: string) => void
  disconnect: () => void
  reconnect: () => void
  
  // Message handling
  sendMessage: (message: any) => void
  sendRaw: (data: string) => void
  clearMessageHistory: () => void
  
  // Testing utilities
  simulateConnectionLoss: () => void
  simulateReconnection: () => void
  simulateLatency: (ms: number) => void
  simulateMessageLoss: (percentage: number) => void
  
  // Progress simulation
  simulateSyncProgress: (sourceId: string, scenario?: string) => void
  simulateUploadProgress: (fileId: string, totalSize: number) => void
  simulateOcrProgress: (documentId: string) => void
  
  // Error simulation
  simulateConnectionError: (errorType?: 'timeout' | 'refused' | 'network') => void
  simulateProtocolError: () => void
  
  // Configuration
  setAutoReconnect: (enabled: boolean) => void
  setHeartbeatInterval: (ms: number) => void
  setConnectionTimeout: (ms: number) => void
  
  // State utilities
  reset: () => void
  getConnectionStats: () => ConnectionStats
}

export interface ConnectionStats {
  totalConnections: number
  totalDisconnections: number
  totalMessages: number
  totalErrors: number
  uptime: number
  averageLatency: number
  lastConnected: Date | null
  lastDisconnected: Date | null
}

const MockWebSocketContext = createContext<MockWebSocketContextType | null>(null)

export interface MockWebSocketProviderProps {
  children: ReactNode
  url?: string
  autoConnect?: boolean
  autoReconnect?: boolean
  reconnectDelay?: number
  maxReconnectAttempts?: number
  heartbeatInterval?: number
  connectionTimeout?: number
  enableLatencySimulation?: boolean
  defaultLatency?: number
}

export const MockWebSocketProvider: React.FC<MockWebSocketProviderProps> = ({
  children,
  url = 'ws://localhost:8000/ws',
  autoConnect = true,
  autoReconnect = true,
  reconnectDelay = 3000,
  maxReconnectAttempts = 5,
  heartbeatInterval = 30000,
  connectionTimeout = 10000,
  enableLatencySimulation = false,
  defaultLatency = 100,
}) => {
  const [messageHistory, setMessageHistory] = useState<any[]>([])
  const [connectionAttempts, setConnectionAttempts] = useState(0)
  const [totalConnections, setTotalConnections] = useState(0)
  const [totalDisconnections, setTotalDisconnections] = useState(0)
  const [totalMessages, setTotalMessages] = useState(0)
  const [totalErrors, setTotalErrors] = useState(0)
  const [lastConnected, setLastConnected] = useState<Date | null>(null)
  const [lastDisconnected, setLastDisconnected] = useState<Date | null>(null)
  const [connectionStartTime, setConnectionStartTime] = useState<Date | null>(null)
  const [latencyMs, setLatencyMs] = useState(defaultLatency)
  const [messageLossPercentage, setMessageLossPercentage] = useState(0)
  const [autoReconnectEnabled, setAutoReconnectEnabled] = useState(autoReconnect)
  
  const reconnectTimeoutRef = useRef<NodeJS.Timeout | null>(null)
  const heartbeatIntervalRef = useRef<NodeJS.Timeout | null>(null)
  
  const { 
    webSocket, 
    connectionState, 
    lastMessage, 
    sendMessage: originalSendMessage,
    simulateProgress,
    simulateError 
  } = useMockWebSocket(autoConnect ? url : undefined)

  // Track message history
  useEffect(() => {
    if (lastMessage) {
      setMessageHistory(prev => [...prev, { ...lastMessage, timestamp: new Date() }])
      setTotalMessages(prev => prev + 1)
    }
  }, [lastMessage])

  // Track connection state changes
  useEffect(() => {
    if (connectionState === 'connected') {
      setTotalConnections(prev => prev + 1)
      setLastConnected(new Date())
      setConnectionStartTime(new Date())
      setConnectionAttempts(0)
      
      // Start heartbeat
      if (heartbeatInterval > 0) {
        heartbeatIntervalRef.current = setInterval(() => {
          sendMessage({ type: 'heartbeat', timestamp: Date.now() })
        }, heartbeatInterval)
      }
    } else if (connectionState === 'disconnected') {
      setTotalDisconnections(prev => prev + 1)
      setLastDisconnected(new Date())
      setConnectionStartTime(null)
      
      // Clear heartbeat
      if (heartbeatIntervalRef.current) {
        clearInterval(heartbeatIntervalRef.current)
      }
      
      // Auto-reconnect if enabled
      if (autoReconnectEnabled && connectionAttempts < maxReconnectAttempts) {
        reconnectTimeoutRef.current = setTimeout(() => {
          reconnect()
        }, reconnectDelay)
      }
    }
  }, [connectionState, autoReconnectEnabled, connectionAttempts, maxReconnectAttempts, reconnectDelay, heartbeatInterval])

  // Cleanup timeouts
  useEffect(() => {
    return () => {
      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current)
      }
      if (heartbeatIntervalRef.current) {
        clearInterval(heartbeatIntervalRef.current)
      }
    }
  }, [])

  const connect = useCallback((connectUrl?: string) => {
    const targetUrl = connectUrl || url
    setConnectionAttempts(prev => prev + 1)
    
    // Simulate connection timeout
    const timeout = setTimeout(() => {
      if (connectionState === 'connecting') {
        setTotalErrors(prev => prev + 1)
        simulateConnectionError('timeout')
      }
    }, connectionTimeout)

    // Clear timeout if connection succeeds
    if (connectionState === 'connected') {
      clearTimeout(timeout)
    }
  }, [url, connectionState, connectionTimeout])

  const disconnect = useCallback(() => {
    if (webSocket && connectionState !== 'disconnected') {
      webSocket.close()
    }
    
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current)
    }
  }, [webSocket, connectionState])

  const reconnect = useCallback(() => {
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current)
    }
    connect()
  }, [connect])

  const sendMessage = useCallback((message: any) => {
    // Simulate message loss
    if (messageLossPercentage > 0 && Math.random() * 100 < messageLossPercentage) {
      console.warn('📉 Message lost due to simulated network conditions')
      return
    }

    // Simulate latency
    if (enableLatencySimulation && latencyMs > 0) {
      setTimeout(() => {
        originalSendMessage(message)
      }, latencyMs)
    } else {
      originalSendMessage(message)
    }
  }, [originalSendMessage, enableLatencySimulation, latencyMs, messageLossPercentage])

  const sendRaw = useCallback((data: string) => {
    if (webSocket && connectionState === 'connected') {
      webSocket.send(data)
    }
  }, [webSocket, connectionState])

  const clearMessageHistory = useCallback(() => {
    setMessageHistory([])
  }, [])

  const simulateConnectionLoss = useCallback(() => {
    setTotalErrors(prev => prev + 1)
    if (webSocket) {
      webSocket.close()
    }
  }, [webSocket])

  const simulateReconnection = useCallback(() => {
    simulateConnectionLoss()
    setTimeout(() => {
      reconnect()
    }, 1000)
  }, [simulateConnectionLoss, reconnect])

  const simulateLatency = useCallback((ms: number) => {
    setLatencyMs(ms)
  }, [])

  const simulateMessageLoss = useCallback((percentage: number) => {
    setMessageLossPercentage(Math.max(0, Math.min(100, percentage)))
  }, [])

  const simulateSyncProgress = useCallback((sourceId: string, scenario: string = 'in_progress') => {
    simulateProgress(sourceId, scenario)
  }, [simulateProgress])

  const simulateUploadProgress = useCallback((fileId: string, totalSize: number) => {
    let uploaded = 0
    const chunkSize = Math.max(1000, totalSize * 0.05) // 5% chunks
    
    const uploadInterval = setInterval(() => {
      uploaded += chunkSize
      const percentage = Math.min(100, (uploaded / totalSize) * 100)
      
      sendMessage({
        type: 'upload_progress',
        file_id: fileId,
        uploaded,
        total: totalSize,
        percentage: Math.round(percentage),
        timestamp: Date.now(),
      })
      
      if (percentage >= 100) {
        clearInterval(uploadInterval)
        sendMessage({
          type: 'upload_complete',
          file_id: fileId,
          timestamp: Date.now(),
        })
      }
    }, 500)
  }, [sendMessage])

  const simulateOcrProgress = useCallback((documentId: string) => {
    const stages = [
      { stage: 'preprocessing', percentage: 10 },
      { stage: 'text_detection', percentage: 30 },
      { stage: 'text_recognition', percentage: 70 },
      { stage: 'post_processing', percentage: 90 },
      { stage: 'complete', percentage: 100 },
    ]
    
    stages.forEach((stage, index) => {
      setTimeout(() => {
        sendMessage({
          type: 'ocr_progress',
          document_id: documentId,
          stage: stage.stage,
          percentage: stage.percentage,
          timestamp: Date.now(),
        })
      }, index * 1000)
    })
  }, [sendMessage])

  const simulateConnectionError = useCallback((errorType: 'timeout' | 'refused' | 'network' = 'network') => {
    setTotalErrors(prev => prev + 1)
    simulateError(`Connection ${errorType}`)
  }, [simulateError])

  const simulateProtocolError = useCallback(() => {
    setTotalErrors(prev => prev + 1)
    sendMessage({
      type: 'error',
      error: 'Protocol violation',
      code: 1002,
      timestamp: Date.now(),
    })
  }, [sendMessage])

  const setAutoReconnect = useCallback((enabled: boolean) => {
    setAutoReconnectEnabled(enabled)
  }, [])

  const setHeartbeatInterval = useCallback((ms: number) => {
    // Implementation would update heartbeat interval
  }, [])

  const setConnectionTimeout = useCallback((ms: number) => {
    // Implementation would update connection timeout
  }, [])

  const reset = useCallback(() => {
    setMessageHistory([])
    setConnectionAttempts(0)
    setTotalConnections(0)
    setTotalDisconnections(0)
    setTotalMessages(0)
    setTotalErrors(0)
    setLastConnected(null)
    setLastDisconnected(null)
    setConnectionStartTime(null)
    setLatencyMs(defaultLatency)
    setMessageLossPercentage(0)
    setAutoReconnectEnabled(autoReconnect)
    
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current)
    }
    if (heartbeatIntervalRef.current) {
      clearInterval(heartbeatIntervalRef.current)
    }
  }, [defaultLatency, autoReconnect])

  const getConnectionStats = useCallback((): ConnectionStats => {
    const uptime = connectionStartTime ? Date.now() - connectionStartTime.getTime() : 0
    const latencyHistory = messageHistory
      .filter(m => m.type === 'pong' && m.latency)
      .map(m => m.latency)
    const averageLatency = latencyHistory.length > 0 
      ? latencyHistory.reduce((sum, lat) => sum + lat, 0) / latencyHistory.length
      : 0

    return {
      totalConnections,
      totalDisconnections,
      totalMessages,
      totalErrors,
      uptime,
      averageLatency,
      lastConnected,
      lastDisconnected,
    }
  }, [totalConnections, totalDisconnections, totalMessages, totalErrors, connectionStartTime, messageHistory, lastConnected, lastDisconnected])

  const contextValue: MockWebSocketContextType = {
    connectionState,
    lastMessage,
    messageHistory,
    connectionAttempts,
    connect,
    disconnect,
    reconnect,
    sendMessage,
    sendRaw,
    clearMessageHistory,
    simulateConnectionLoss,
    simulateReconnection,
    simulateLatency,
    simulateMessageLoss,
    simulateSyncProgress,
    simulateUploadProgress,
    simulateOcrProgress,
    simulateConnectionError,
    simulateProtocolError,
    setAutoReconnect,
    setHeartbeatInterval,
    setConnectionTimeout,
    reset,
    getConnectionStats,
  }

  return (
    <MockWebSocketContext.Provider value={contextValue}>
      {children}
      <WebSocketDebugPanel 
        connectionState={connectionState}
        messageHistory={messageHistory}
        stats={getConnectionStats()}
      />
    </MockWebSocketContext.Provider>
  )
}

// Debug panel component for development
const WebSocketDebugPanel: React.FC<{
  connectionState: string
  messageHistory: any[]
  stats: ConnectionStats
}> = ({ connectionState, messageHistory, stats }) => {
  const [isVisible, setIsVisible] = useState(false)

  if (process.env.NODE_ENV === 'production') {
    return null
  }

  return (
    <>
      <button
        onClick={() => setIsVisible(!isVisible)}
        style={{
          position: 'fixed',
          bottom: '16px',
          left: '16px',
          zIndex: 9998,
          background: '#333',
          color: 'white',
          border: 'none',
          padding: '8px 12px',
          borderRadius: '4px',
          fontSize: '12px',
          cursor: 'pointer',
        }}
        data-testid="websocket-debug-toggle"
      >
        WS: {connectionState}
      </button>
      
      {isVisible && (
        <div
          style={{
            position: 'fixed',
            bottom: '60px',
            left: '16px',
            width: '300px',
            maxHeight: '400px',
            background: 'white',
            border: '1px solid #ccc',
            borderRadius: '8px',
            boxShadow: '0 4px 12px rgba(0,0,0,0.15)',
            zIndex: 9999,
            overflow: 'hidden',
          }}
          data-testid="websocket-debug-panel"
        >
          <div style={{ padding: '12px', borderBottom: '1px solid #eee', background: '#f8f9fa' }}>
            <strong>WebSocket Debug</strong>
          </div>
          
          <div style={{ padding: '12px', fontSize: '12px' }}>
            <div><strong>State:</strong> {connectionState}</div>
            <div><strong>Messages:</strong> {stats.totalMessages}</div>
            <div><strong>Errors:</strong> {stats.totalErrors}</div>
            <div><strong>Uptime:</strong> {Math.round(stats.uptime / 1000)}s</div>
          </div>
          
          <div style={{ maxHeight: '200px', overflow: 'auto', fontSize: '11px' }}>
            {messageHistory.slice(-10).map((msg, index) => (
              <div key={index} style={{ padding: '4px 12px', borderBottom: '1px solid #f0f0f0' }}>
                <div style={{ color: '#666' }}>{msg.timestamp?.toLocaleTimeString()}</div>
                <div>{JSON.stringify(msg, null, 1).slice(0, 100)}...</div>
              </div>
            ))}
          </div>
        </div>
      )}
    </>
  )
}

export const useMockWebSocketContext = (): MockWebSocketContextType => {
  const context = useContext(MockWebSocketContext)
  if (!context) {
    throw new Error('useMockWebSocketContext must be used within a MockWebSocketProvider')
  }
  return context
}

// High-order component for wrapping components with mock WebSocket
export const withMockWebSocket = <P extends object>(
  Component: React.ComponentType<P>,
  providerProps?: Partial<MockWebSocketProviderProps>
) => {
  const WrappedComponent = (props: P) => (
    <MockWebSocketProvider {...providerProps}>
      <Component {...props} />
    </MockWebSocketProvider>
  )
  
  WrappedComponent.displayName = `withMockWebSocket(${Component.displayName || Component.name})`
  return WrappedComponent
}