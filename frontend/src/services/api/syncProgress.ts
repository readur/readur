import type { WebSocketMessage } from './types'

export interface SyncProgressInfo {
  source_id: string
  phase: string
  phase_description: string
  elapsed_time_secs: number
  directories_found: number
  directories_processed: number
  files_found: number
  files_processed: number
  bytes_processed: number
  processing_rate_files_per_sec: number
  files_progress_percent: number
  estimated_time_remaining_secs?: number
  current_directory: string
  current_file?: string
  errors: number
  warnings: number
  is_active: boolean
}

export class SyncProgressWebSocket {
  private ws: WebSocket | null = null;
  private sourceId: string;
  private url: string;
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private reconnectDelay = 1000;
  private isManuallyClosing = false;
  private listeners: { [key: string]: ((data: any) => void)[] } = {};

  constructor(sourceId: string) {
    this.sourceId = sourceId;
    this.url = this.buildWebSocketUrl(sourceId);
  }

  private buildWebSocketUrl(sourceId: string): string {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    
    // In development, frontend (5173) and backend (8000) run on different ports
    // WebSocket connections can't use Vite proxy, so we need to connect directly to backend
    const isDevelopment = import.meta.env.DEV || window.location.port === '5173';
    const host = isDevelopment ? 'localhost:8000' : window.location.host;
    
    return `${protocol}//${host}/api/sources/${sourceId}/sync/progress/ws`;
  }
  
  private getAuthProtocol(): string | undefined {
    const token = localStorage.getItem('token');
    return token ? `bearer.${token}` : undefined;
  }

  connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      try {
        // Create WebSocket connection with secure authentication via protocol header
        const authProtocol = this.getAuthProtocol();
        this.ws = authProtocol 
          ? new WebSocket(this.url, [authProtocol])
          : new WebSocket(this.url);

        this.ws.onopen = () => {
          console.log(`WebSocket connected to sync progress for source: ${this.sourceId}`);
          this.reconnectAttempts = 0;
          this.emit('connectionStatus', 'connected');
          resolve();
        };

        this.ws.onmessage = (event) => {
          try {
            const message: WebSocketMessage = JSON.parse(event.data);
            
            switch (message.type) {
              case 'progress':
                this.emit('progress', message.data);
                break;
              case 'heartbeat':
                this.emit('heartbeat', message.data);
                break;
              case 'error':
                this.emit('error', message.data);
                console.error('WebSocket error from server:', message.data);
                break;
              case 'connection_confirmed':
                this.emit('connectionConfirmed', message.data);
                break;
              case 'connection_closing':
                this.emit('connectionClosing', message.data);
                console.log('Server is closing connection:', message.data);
                break;
              default:
                console.warn('Unknown WebSocket message type:', message.type);
            }
          } catch (error) {
            console.error('Failed to parse WebSocket message:', error);
            this.emit('error', { error: 'Failed to parse message' });
          }
        };

        this.ws.onclose = (event) => {
          console.log(`WebSocket closed for source ${this.sourceId}:`, event.code, event.reason);
          this.emit('connectionStatus', 'disconnected');
          
          if (!this.isManuallyClosing && this.shouldReconnect(event.code)) {
            this.scheduleReconnect();
          }
        };

        this.ws.onerror = (error) => {
          console.error('WebSocket error:', error);
          this.emit('connectionStatus', 'error');
          this.emit('error', { error: 'WebSocket connection error' });
          reject(error);
        };

      } catch (error) {
        console.error('Failed to create WebSocket:', error);
        this.emit('connectionStatus', 'error');
        reject(error);
      }
    });
  }

  private shouldReconnect(code: number): boolean {
    // Don't reconnect on normal closure, authentication failure, or when max attempts reached
    // WebSocket close codes: 1000 = normal, 1001 = going away, 1003 = unsupported data, 1008 = policy violation (auth)
    const noReconnectCodes = [1000, 1001, 1003, 1008];
    return !noReconnectCodes.includes(code) && this.reconnectAttempts < this.maxReconnectAttempts;
  }

  private scheduleReconnect(): void {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      console.error('Max reconnection attempts reached for WebSocket');
      this.emit('connectionStatus', 'failed');
      return;
    }

    this.reconnectAttempts++;
    const delay = Math.min(this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1), 30000);
    
    console.log(`Attempting to reconnect WebSocket in ${delay}ms (attempt ${this.reconnectAttempts}/${this.maxReconnectAttempts})`);
    this.emit('connectionStatus', 'reconnecting');

    setTimeout(() => {
      if (!this.isManuallyClosing) {
        this.connect().catch(error => {
          console.error('Reconnection failed:', error);
        });
      }
    }, delay);
  }

  addEventListener(eventType: string, callback: (data: any) => void): void {
    if (!this.listeners[eventType]) {
      this.listeners[eventType] = [];
    }
    this.listeners[eventType].push(callback);
  }

  removeEventListener(eventType: string, callback: (data: any) => void): void {
    if (this.listeners[eventType]) {
      this.listeners[eventType] = this.listeners[eventType].filter(cb => cb !== callback);
    }
  }

  private emit(eventType: string, data: any): void {
    if (this.listeners[eventType]) {
      this.listeners[eventType].forEach(callback => callback(data));
    }
  }

  close(): void {
    this.isManuallyClosing = true;
    if (this.ws) {
      this.ws.close(1000, 'Client requested closure');
      this.ws = null;
    }
    this.listeners = {};
  }

  getReadyState(): number {
    return this.ws?.readyState ?? WebSocket.CLOSED;
  }

  isConnected(): boolean {
    return this.ws?.readyState === WebSocket.OPEN;
  }
}
