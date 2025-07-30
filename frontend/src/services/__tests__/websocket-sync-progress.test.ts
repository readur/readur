import { describe, test, expect, vi, beforeEach, afterEach } from 'vitest';

// Mock WebSocket globally
const mockWebSocket = vi.fn();
const mockWebSocketInstances: any[] = [];

mockWebSocket.mockImplementation((url: string) => {
  const instance = {
    url,
    readyState: WebSocket.CONNECTING,
    send: vi.fn(),
    close: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    onopen: null as any,
    onmessage: null as any,
    onerror: null as any,
    onclose: null as any,
    CONNECTING: 0,
    OPEN: 1,
    CLOSING: 2,
    CLOSED: 3,
  };
  
  mockWebSocketInstances.push(instance);
  
  // Simulate connection opening after a short delay
  setTimeout(() => {
    instance.readyState = WebSocket.OPEN;
    if (instance.onopen) {
      instance.onopen(new Event('open'));
    }
  }, 10);
  
  return instance;
});

// Replace global WebSocket
Object.defineProperty(global, 'WebSocket', {
  value: mockWebSocket,
  writable: true,
});

// Mock localStorage
const mockLocalStorage = {
  getItem: vi.fn(),
  setItem: vi.fn(),
  removeItem: vi.fn(),
  clear: vi.fn(),
};

Object.defineProperty(global, 'localStorage', {
  value: mockLocalStorage,
  writable: true,
});

// WebSocket service implementation
class WebSocketSyncProgressService {
  private ws: WebSocket | null = null;
  private sourceId: string;
  private onMessage: (data: any) => void;
  private onError: (error: Event) => void;
  private onConnectionChange: (status: 'connecting' | 'connected' | 'disconnected') => void;
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private reconnectDelay = 1000;

  constructor(
    sourceId: string,
    onMessage: (data: any) => void,
    onError: (error: Event) => void,
    onConnectionChange: (status: 'connecting' | 'connected' | 'disconnected') => void
  ) {
    this.sourceId = sourceId;
    this.onMessage = onMessage;
    this.onError = onError;
    this.onConnectionChange = onConnectionChange;
  }

  connect(): void {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      return; // Already connected
    }

    this.onConnectionChange('connecting');
    
    const token = localStorage.getItem('token');
    if (!token) {
      this.onError(new Event('auth-error'));
      return;
    }

    const wsUrl = `ws://localhost:8080/api/sources/${this.sourceId}/sync/progress/ws?token=${encodeURIComponent(token)}`;
    
    try {
      this.ws = new WebSocket(wsUrl);
      
      this.ws.onopen = (event) => {
        this.reconnectAttempts = 0;
        this.onConnectionChange('connected');
      };

      this.ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data);
          this.onMessage(data);
        } catch (error) {
          console.error('Failed to parse WebSocket message:', error);
          this.onError(new Event('parse-error'));
        }
      };

      this.ws.onerror = (event) => {
        console.error('WebSocket error:', event);
        this.onError(event);
      };

      this.ws.onclose = (event) => {
        this.onConnectionChange('disconnected');
        
        // Attempt to reconnect if not intentionally closed
        if (event.code !== 1000 && this.reconnectAttempts < this.maxReconnectAttempts) {
          setTimeout(() => {
            this.reconnectAttempts++;
            this.connect();
          }, this.reconnectDelay * Math.pow(2, this.reconnectAttempts));
        }
      };
    } catch (error) {
      console.error('Failed to create WebSocket connection:', error);
      this.onError(new Event('connection-error'));
    }
  }

  disconnect(): void {
    if (this.ws) {
      this.ws.close(1000, 'Client disconnect');
      this.ws = null;
    }
  }

  sendPing(): void {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send('ping');
    }
  }

  getConnectionState(): number {
    return this.ws ? this.ws.readyState : WebSocket.CLOSED;
  }
}

describe('WebSocket Sync Progress Service', () => {
  let service: WebSocketSyncProgressService;
  let mockOnMessage: any;
  let mockOnError: any;
  let mockOnConnectionChange: any;
  let sourceId: string;

  beforeEach(() => {
    vi.clearAllMocks();
    mockWebSocketInstances.length = 0;
    
    sourceId = 'test-source-123';
    mockOnMessage = vi.fn();
    mockOnError = vi.fn();
    mockOnConnectionChange = vi.fn();
    
    mockLocalStorage.getItem.mockReturnValue('mock-jwt-token');
    
    service = new WebSocketSyncProgressService(
      sourceId,
      mockOnMessage,
      mockOnError,
      mockOnConnectionChange
    );
  });

  afterEach(() => {
    if (service) {
      service.disconnect();
    }
  });

  test('should create WebSocket connection with correct URL and token', () => {
    service.connect();
    
    expect(mockWebSocket).toHaveBeenCalledWith(
      `ws://localhost:8080/api/sources/${sourceId}/sync/progress/ws?token=mock-jwt-token`
    );
    expect(mockOnConnectionChange).toHaveBeenCalledWith('connecting');
  });

  test('should handle connection success', async () => {
    service.connect();
    
    const wsInstance = mockWebSocketInstances[0];
    expect(wsInstance).toBeDefined();
    
    // Wait for simulated connection
    await new Promise(resolve => setTimeout(resolve, 20));
    
    expect(mockOnConnectionChange).toHaveBeenCalledWith('connected');
  });

  test('should handle authentication error when no token', () => {
    mockLocalStorage.getItem.mockReturnValue(null);
    
    service.connect();
    
    expect(mockWebSocket).not.toHaveBeenCalled();
    expect(mockOnError).toHaveBeenCalledWith(expect.any(Event));
  });

  test('should parse and handle WebSocket messages', () => {
    service.connect();
    
    const wsInstance = mockWebSocketInstances[0];
    const testData = {
      type: 'progress',
      data: {
        source_id: sourceId,
        phase: 'processing_files',
        files_processed: 10,
        files_found: 50,
        is_active: true
      }
    };
    
    // Simulate message reception
    if (wsInstance.onmessage) {
      wsInstance.onmessage({
        data: JSON.stringify(testData)
      });
    }
    
    expect(mockOnMessage).toHaveBeenCalledWith(testData);
  });

  test('should handle heartbeat messages', () => {
    service.connect();
    
    const wsInstance = mockWebSocketInstances[0];
    const heartbeatData = {
      type: 'heartbeat',
      data: {
        source_id: sourceId,
        is_active: false,
        timestamp: Date.now()
      }
    };
    
    if (wsInstance.onmessage) {
      wsInstance.onmessage({
        data: JSON.stringify(heartbeatData)
      });
    }
    
    expect(mockOnMessage).toHaveBeenCalledWith(heartbeatData);
  });

  test('should handle connection confirmation messages', () => {
    service.connect();
    
    const wsInstance = mockWebSocketInstances[0];
    const connectionData = {
      type: 'connected',
      source_id: sourceId,
      timestamp: Date.now()
    };
    
    if (wsInstance.onmessage) {
      wsInstance.onmessage({
        data: JSON.stringify(connectionData)
      });
    }
    
    expect(mockOnMessage).toHaveBeenCalledWith(connectionData);
  });

  test('should handle malformed JSON messages', () => {
    service.connect();
    
    const wsInstance = mockWebSocketInstances[0];
    
    if (wsInstance.onmessage) {
      wsInstance.onmessage({
        data: 'invalid json {'
      });
    }
    
    expect(mockOnError).toHaveBeenCalledWith(expect.any(Event));
    expect(mockOnMessage).not.toHaveBeenCalled();
  });

  test('should handle WebSocket errors', () => {
    service.connect();
    
    const wsInstance = mockWebSocketInstances[0];
    const errorEvent = new Event('error');
    
    if (wsInstance.onerror) {
      wsInstance.onerror(errorEvent);
    }
    
    expect(mockOnError).toHaveBeenCalledWith(errorEvent);
  });

  test('should attempt reconnection on unexpected disconnection', () => {
    vi.useFakeTimers();
    
    service.connect();
    
    const wsInstance = mockWebSocketInstances[0];
    
    // Simulate unexpected disconnection (not code 1000)
    if (wsInstance.onclose) {
      wsInstance.onclose({
        code: 1006, // Abnormal closure
        reason: 'Connection lost'
      });
    }
    
    expect(mockOnConnectionChange).toHaveBeenCalledWith('disconnected');
    
    // Fast-forward time to trigger reconnection
    vi.advanceTimersByTime(1000);
    
    // Should attempt to reconnect
    expect(mockWebSocket).toHaveBeenCalledTimes(2);
    
    vi.useRealTimers();
  });

  test('should not reconnect on intentional disconnection', () => {
    vi.useFakeTimers();
    
    service.connect();
    
    const wsInstance = mockWebSocketInstances[0];
    
    // Simulate intentional disconnection (code 1000)
    if (wsInstance.onclose) {
      wsInstance.onclose({
        code: 1000, // Normal closure
        reason: 'Client disconnect'
      });
    }
    
    expect(mockOnConnectionChange).toHaveBeenCalledWith('disconnected');
    
    // Fast-forward time
    vi.advanceTimersByTime(5000);
    
    // Should not attempt to reconnect
    expect(mockWebSocket).toHaveBeenCalledTimes(1);
    
    vi.useRealTimers();
  });

  test('should limit reconnection attempts', () => {
    vi.useFakeTimers();
    
    service.connect();
    
    // Simulate multiple disconnections
    for (let i = 0; i < 6; i++) {
      const wsInstance = mockWebSocketInstances[mockWebSocketInstances.length - 1];
      
      if (wsInstance.onclose) {
        wsInstance.onclose({
          code: 1006,
          reason: 'Connection lost'
        });
      }
      
      // Fast-forward to trigger reconnection
      vi.advanceTimersByTime(10000);
    }
    
    // Should stop reconnecting after max attempts
    expect(mockWebSocket).toHaveBeenCalledTimes(6); // Initial + 5 reconnections
    
    vi.useRealTimers();
  });

  test('should send ping messages', () => {
    service.connect();
    
    const wsInstance = mockWebSocketInstances[0];
    wsInstance.readyState = WebSocket.OPEN;
    
    service.sendPing();
    
    expect(wsInstance.send).toHaveBeenCalledWith('ping');
  });

  test('should not send ping when not connected', () => {
    service.connect();
    
    const wsInstance = mockWebSocketInstances[0];
    wsInstance.readyState = WebSocket.CLOSED;
    
    service.sendPing();
    
    expect(wsInstance.send).not.toHaveBeenCalled();
  });

  test('should disconnect properly', () => {
    service.connect();
    
    const wsInstance = mockWebSocketInstances[0];
    
    service.disconnect();
    
    expect(wsInstance.close).toHaveBeenCalledWith(1000, 'Client disconnect');
  });

  test('should return correct connection state', () => {
    expect(service.getConnectionState()).toBe(WebSocket.CLOSED);
    
    service.connect();
    
    const wsInstance = mockWebSocketInstances[0];
    wsInstance.readyState = WebSocket.CONNECTING;
    
    expect(service.getConnectionState()).toBe(WebSocket.CONNECTING);
    
    wsInstance.readyState = WebSocket.OPEN;
    expect(service.getConnectionState()).toBe(WebSocket.OPEN);
  });

  test('should not create multiple connections when already connected', () => {
    service.connect();
    
    const wsInstance = mockWebSocketInstances[0];
    wsInstance.readyState = WebSocket.OPEN;
    
    // Try to connect again
    service.connect();
    
    // Should not create a new WebSocket
    expect(mockWebSocket).toHaveBeenCalledTimes(1);
  });

  test('should handle progressive backoff for reconnections', () => {
    vi.useFakeTimers();
    
    service.connect();
    
    const initialCallCount = mockWebSocket.mock.calls.length;
    
    // First reconnection
    const wsInstance1 = mockWebSocketInstances[0];
    if (wsInstance1.onclose) {
      wsInstance1.onclose({ code: 1006, reason: 'Connection lost' });
    }
    
    vi.advanceTimersByTime(1000); // 1s delay
    expect(mockWebSocket).toHaveBeenCalledTimes(initialCallCount + 1);
    
    // Second reconnection
    const wsInstance2 = mockWebSocketInstances[1];
    if (wsInstance2.onclose) {
      wsInstance2.onclose({ code: 1006, reason: 'Connection lost' });
    }
    
    vi.advanceTimersByTime(2000); // 2s delay (exponential backoff)
    expect(mockWebSocket).toHaveBeenCalledTimes(initialCallCount + 2);
    
    vi.useRealTimers();
  });
});

describe('WebSocket Message Types', () => {
  test('should handle progress messages with all fields', () => {
    const mockOnMessage = vi.fn();
    const service = new WebSocketSyncProgressService(
      'test-source',
      mockOnMessage,
      vi.fn(),
      vi.fn()
    );
    
    service.connect();
    
    const wsInstance = mockWebSocketInstances[0];
    const progressMessage = {
      type: 'progress',
      data: {
        source_id: 'test-source',
        phase: 'processing_files',
        phase_description: 'Downloading and processing files',
        elapsed_time_secs: 120,
        directories_found: 10,
        directories_processed: 7,
        files_found: 50,
        files_processed: 30,
        bytes_processed: 1024000,
        processing_rate_files_per_sec: 2.5,
        files_progress_percent: 60.0,
        estimated_time_remaining_secs: 80,
        current_directory: '/Documents/Projects',
        current_file: 'important-document.pdf',
        errors: 0,
        warnings: 1,
        is_active: true
      }
    };
    
    if (wsInstance.onmessage) {
      wsInstance.onmessage({
        data: JSON.stringify(progressMessage)
      });
    }
    
    expect(mockOnMessage).toHaveBeenCalledWith(progressMessage);
    
    const receivedData = mockOnMessage.mock.calls[0][0];
    expect(receivedData.type).toBe('progress');
    expect(receivedData.data.files_progress_percent).toBe(60.0);
    expect(receivedData.data.current_file).toBe('important-document.pdf');
  });

  test('should handle error messages', () => {
    const mockOnMessage = vi.fn();
    const service = new WebSocketSyncProgressService(
      'test-source',
      mockOnMessage,
      vi.fn(),
      vi.fn()
    );
    
    service.connect();
    
    const wsInstance = mockWebSocketInstances[0];
    const errorMessage = {
      type: 'error',
      data: {
        message: 'Failed to serialize progress data'
      }
    };
    
    if (wsInstance.onmessage) {
      wsInstance.onmessage({
        data: JSON.stringify(errorMessage)
      });
    }
    
    expect(mockOnMessage).toHaveBeenCalledWith(errorMessage);
  });

  test('should handle different sync phases', () => {
    const mockOnMessage = vi.fn();
    const service = new WebSocketSyncProgressService(
      'test-source',
      mockOnMessage,
      vi.fn(),
      vi.fn()
    );
    
    service.connect();
    
    const wsInstance = mockWebSocketInstances[0];
    const phases = [
      'initializing',
      'evaluating', 
      'discovering_directories',
      'discovering_files',
      'processing_files',
      'saving_metadata',
      'completed',
      'failed'
    ];
    
    phases.forEach((phase, index) => {
      const progressMessage = {
        type: 'progress',
        data: {
          source_id: 'test-source',
          phase: phase,
          phase_description: `Phase ${phase}`,
          is_active: phase !== 'completed' && phase !== 'failed'
        }
      };
      
      if (wsInstance.onmessage) {
        wsInstance.onmessage({
          data: JSON.stringify(progressMessage)
        });
      }
    });
    
    expect(mockOnMessage).toHaveBeenCalledTimes(phases.length);
    
    // Check specific phases
    const completedCall = mockOnMessage.mock.calls.find(call => 
      call[0].data.phase === 'completed'
    );
    expect(completedCall[0].data.is_active).toBe(false);
    
    const failedCall = mockOnMessage.mock.calls.find(call => 
      call[0].data.phase === 'failed'
    );
    expect(failedCall[0].data.is_active).toBe(false);
  });
});