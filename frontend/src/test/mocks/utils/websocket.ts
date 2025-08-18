/**
 * WebSocket mocking utilities for testing real-time features
 * Provides comprehensive WebSocket simulation for sync progress and other real-time data
 */

import { MockSyncProgress, MockWebSocketMessage, MockConfig } from '../api/types'
import { createMockSyncProgressWithScenario } from '../factories'
import { applyDelay } from './config'

interface MockWebSocketOptions {
  autoConnect?: boolean
  messageDelay?: number
  heartbeatInterval?: number
  simulateReconnects?: boolean
  maxReconnects?: number
}

/**
 * Mock WebSocket class that simulates WebSocket behavior for testing
 */
export class MockWebSocket extends EventTarget {
  public readyState: number = WebSocket.CONNECTING
  public url: string
  public protocol: string
  public binaryType: BinaryType = 'blob'

  private _options: MockWebSocketOptions
  private _messageQueue: MockWebSocketMessage[] = []
  private _heartbeatInterval?: NodeJS.Timeout
  private _simulationInterval?: NodeJS.Timeout
  private _isSimulating = false
  private _reconnectCount = 0

  // Mock static constants
  static readonly CONNECTING = 0
  static readonly OPEN = 1
  static readonly CLOSING = 2
  static readonly CLOSED = 3

  public onopen: ((event: Event) => void) | null = null
  public onclose: ((event: CloseEvent) => void) | null = null
  public onmessage: ((event: MessageEvent) => void) | null = null
  public onerror: ((event: Event) => void) | null = null

  constructor(url: string, protocols?: string | string[], options: MockWebSocketOptions = {}) {
    super()
    
    this.url = url
    this.protocol = Array.isArray(protocols) ? protocols[0] || '' : protocols || ''
    this._options = {
      autoConnect: true,
      messageDelay: 100,
      heartbeatInterval: 30000,
      simulateReconnects: false,
      maxReconnects: 3,
      ...options
    }

    if (this._options.autoConnect) {
      // Simulate connection delay
      setTimeout(() => this._connect(), 10)
    }
  }

  private _connect() {
    this.readyState = WebSocket.OPEN
    
    const event = new Event('open')
    this.onopen?.(event)
    this.dispatchEvent(event)

    // Start heartbeat if configured
    if (this._options.heartbeatInterval) {
      this._startHeartbeat()
    }

    // Send connection confirmed message
    this._sendMessage({
      type: 'connection_confirmed',
      data: { connected_at: new Date().toISOString() }
    })
  }

  private _startHeartbeat() {
    this._heartbeatInterval = setInterval(() => {
      if (this.readyState === WebSocket.OPEN) {
        this._sendMessage({
          type: 'heartbeat',
          data: { timestamp: new Date().toISOString() }
        })
      }
    }, this._options.heartbeatInterval)
  }

  private _sendMessage(message: MockWebSocketMessage) {
    if (this.readyState !== WebSocket.OPEN) return

    setTimeout(() => {
      const messageEvent = new MessageEvent('message', {
        data: JSON.stringify(message)
      })
      this.onmessage?.(messageEvent)
      this.dispatchEvent(messageEvent)
    }, this._options.messageDelay)
  }

  send(data: string | ArrayBufferLike | Blob | ArrayBufferView): void {
    if (this.readyState !== WebSocket.OPEN) {
      throw new Error('WebSocket is not open')
    }

    // Echo message back for testing
    this._sendMessage({
      type: 'progress',
      data: { echo: data }
    })
  }

  close(code?: number, reason?: string): void {
    if (this.readyState === WebSocket.CLOSED || this.readyState === WebSocket.CLOSING) {
      return
    }

    this.readyState = WebSocket.CLOSING

    // Clear intervals
    if (this._heartbeatInterval) {
      clearInterval(this._heartbeatInterval)
    }
    if (this._simulationInterval) {
      clearInterval(this._simulationInterval)
    }

    setTimeout(() => {
      this.readyState = WebSocket.CLOSED
      
      const closeEvent = new CloseEvent('close', {
        code: code || 1000,
        reason: reason || 'Normal closure',
        wasClean: true
      })
      
      this.onclose?.(closeEvent)
      this.dispatchEvent(closeEvent)
    }, 10)
  }

  /**
   * Start simulating sync progress messages
   */
  startSyncProgressSimulation(sourceId: string, scenario: string = 'in_progress') {
    if (this._isSimulating) return

    this._isSimulating = true
    let currentProgress = createMockSyncProgressWithScenario(scenario)
    currentProgress.source_id = sourceId

    this._simulationInterval = setInterval(() => {
      if (this.readyState === WebSocket.OPEN) {
        // Simulate progress updates
        if (currentProgress.phase === 'processing' && currentProgress.files_progress_percent < 100) {
          currentProgress.files_processed += Math.floor(Math.random() * 5) + 1
          currentProgress.files_progress_percent = Math.min(100, 
            (currentProgress.files_processed / currentProgress.files_found) * 100
          )
          currentProgress.elapsed_time_secs += 5
        }

        this._sendMessage({
          type: 'progress',
          data: currentProgress
        })

        // Complete the sync when reaching 100%
        if (currentProgress.files_progress_percent >= 100) {
          currentProgress = createMockSyncProgressWithScenario('completed')
          currentProgress.source_id = sourceId
          this.stopSyncProgressSimulation()
        }
      }
    }, 1000) // Update every second
  }

  /**
   * Stop sync progress simulation
   */
  stopSyncProgressSimulation() {
    this._isSimulating = false
    if (this._simulationInterval) {
      clearInterval(this._simulationInterval)
      this._simulationInterval = undefined
    }
  }

  /**
   * Simulate an error
   */
  simulateError(error: string) {
    this._sendMessage({
      type: 'error',
      data: { error, timestamp: new Date().toISOString() }
    })
  }

  /**
   * Simulate connection closing
   */
  simulateConnectionClosing(reason?: string) {
    this._sendMessage({
      type: 'connection_closing',
      data: { reason: reason || 'Server shutdown' }
    })
    
    setTimeout(() => this.close(1001, reason), 1000)
  }
}

/**
 * Global WebSocket mock for replacing native WebSocket in tests
 */
let originalWebSocket: typeof WebSocket

export const enableWebSocketMocking = (options: MockWebSocketOptions = {}) => {
  if (typeof globalThis !== 'undefined' && globalThis.WebSocket) {
    originalWebSocket = globalThis.WebSocket
    // Use Object.defineProperty to override read-only property
    try {
      Object.defineProperty(globalThis, 'WebSocket', {
        value: class extends MockWebSocket {
          constructor(url: string, protocols?: string | string[]) {
            super(url, protocols, options)
          }
        },
        writable: true,
        configurable: true
      })
    } catch (e) {
      // Fallback if defineProperty fails
      // @ts-ignore
      globalThis.WebSocket = class extends MockWebSocket {
        constructor(url: string, protocols?: string | string[]) {
          super(url, protocols, options)
        }
      }
    }
  }
}

export const disableWebSocketMocking = () => {
  if (originalWebSocket && typeof globalThis !== 'undefined') {
    try {
      Object.defineProperty(globalThis, 'WebSocket', {
        value: originalWebSocket,
        writable: true,
        configurable: true
      })
    } catch (e) {
      // Fallback if defineProperty fails
      // @ts-ignore
      globalThis.WebSocket = originalWebSocket
    }
  }
}

/**
 * Create a mock WebSocket instance for testing
 */
export const createMockWebSocket = (
  url: string, 
  protocols?: string | string[], 
  options: MockWebSocketOptions = {}
): MockWebSocket => {
  return new MockWebSocket(url, protocols, options)
}

/**
 * WebSocket testing utilities
 */
export class WebSocketTestUtils {
  private static instances: MockWebSocket[] = []

  /**
   * Create and track a WebSocket instance for testing
   */
  static createWebSocket(
    url: string, 
    protocols?: string | string[], 
    options: MockWebSocketOptions = {}
  ): MockWebSocket {
    const ws = new MockWebSocket(url, protocols, options)
    this.instances.push(ws)
    return ws
  }

  /**
   * Close all tracked WebSocket instances
   */
  static closeAllWebSockets() {
    this.instances.forEach(ws => {
      if (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING) {
        ws.close()
      }
    })
    this.instances = []
  }

  /**
   * Wait for WebSocket to reach specific state
   */
  static async waitForState(ws: MockWebSocket, state: number, timeout = 5000): Promise<void> {
    return new Promise((resolve, reject) => {
      const checkState = () => {
        if (ws.readyState === state) {
          resolve()
          return
        }
        
        setTimeout(checkState, 50)
      }

      setTimeout(() => reject(new Error(`WebSocket did not reach state ${state} within ${timeout}ms`)), timeout)
      checkState()
    })
  }

  /**
   * Wait for specific message type
   */
  static async waitForMessage(
    ws: MockWebSocket, 
    messageType: string, 
    timeout = 5000
  ): Promise<MockWebSocketMessage> {
    return new Promise((resolve, reject) => {
      const messageHandler = (event: MessageEvent) => {
        try {
          const message = JSON.parse(event.data)
          if (message.type === messageType) {
            ws.removeEventListener('message', messageHandler)
            resolve(message)
          }
        } catch (e) {
          // Ignore parsing errors
        }
      }

      ws.addEventListener('message', messageHandler)
      
      setTimeout(() => {
        ws.removeEventListener('message', messageHandler)
        reject(new Error(`Did not receive message type '${messageType}' within ${timeout}ms`))
      }, timeout)
    })
  }

  /**
   * Simulate realistic sync progress scenario
   */
  static simulateRealisticSyncProgress(ws: MockWebSocket, sourceId: string, durationMs = 10000) {
    const phases = ['discovery', 'processing', 'cleanup', 'completed']
    let currentPhaseIndex = 0
    let elapsedTime = 0
    const updateInterval = 500

    const simulation = setInterval(() => {
      const phase = phases[currentPhaseIndex]
      const progress = Math.min(100, (elapsedTime / durationMs) * 100)
      
      const progressData = {
        source_id: sourceId,
        phase,
        phase_description: `${phase.charAt(0).toUpperCase() + phase.slice(1)} in progress...`,
        elapsed_time_secs: Math.floor(elapsedTime / 1000),
        files_progress_percent: progress,
        is_active: progress < 100,
        current_directory: `/documents/batch-${Math.floor(elapsedTime / 2000)}`,
        files_found: 100,
        files_processed: Math.floor(progress),
        errors: Math.floor(Math.random() * 2),
        warnings: Math.floor(Math.random() * 5),
      }

      if (ws.readyState === WebSocket.OPEN) {
        ws._sendMessage({
          type: 'progress',
          data: progressData
        })
      }

      elapsedTime += updateInterval

      // Move to next phase
      if (elapsedTime >= (durationMs / phases.length) * (currentPhaseIndex + 1)) {
        currentPhaseIndex = Math.min(currentPhaseIndex + 1, phases.length - 1)
      }

      // Complete simulation
      if (elapsedTime >= durationMs) {
        clearInterval(simulation)
      }
    }, updateInterval)
  }
}

// Extend the MockWebSocket prototype with private method access for testing
declare module './websocket' {
  interface MockWebSocket {
    _sendMessage(message: MockWebSocketMessage): void
  }
}