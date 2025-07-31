import { SyncProgressManager, ConnectionStatus } from './SyncProgressManager';
import { SyncProgressWebSocket, SyncProgressInfo, sourcesService } from '../api';

/**
 * WebSocket-based implementation of SyncProgressManager
 * Handles real-time sync progress updates via WebSocket connection
 */
export class WebSocketSyncProgressManager extends SyncProgressManager {
  private ws: SyncProgressWebSocket | null = null;
  private sourceId: string | null = null;
  private reconnectTimeout: NodeJS.Timeout | null = null;
  private reconnectAttempts = 0;
  private readonly maxReconnectAttempts = 5;
  private readonly reconnectDelay = 1000; // Start with 1 second

  async connect(sourceId: string): Promise<void> {
    // Clean up any existing connection
    this.cleanup();

    this.sourceId = sourceId;
    this.reconnectAttempts = 0;

    try {
      this.updateState({ connectionStatus: 'connecting' });

      // Create WebSocket connection
      this.ws = sourcesService.createSyncProgressWebSocket(sourceId);

      // Set up event listeners
      this.setupEventListeners();

      // Connect
      await this.ws.connect();

      // Connection successful
      this.updateState({ connectionStatus: 'connected' });
      console.log(`Successfully connected to sync progress for source: ${sourceId}`);
    } catch (error) {
      this.handleConnectionError(error);
      throw error;
    }
  }

  disconnect(): void {
    this.cleanup();
    this.updateState({
      connectionStatus: 'disconnected',
      progressInfo: null
    });
  }

  reconnect(): void {
    if (!this.sourceId) {
      console.warn('Cannot reconnect: no source ID available');
      return;
    }

    console.log(`Reconnecting to sync progress for source: ${this.sourceId}`);
    this.disconnect();

    // Use setTimeout to ensure cleanup is complete
    setTimeout(() => {
      if (this.sourceId) {
        this.connect(this.sourceId).catch(error => {
          console.error('Reconnection failed:', error);
        });
      }
    }, 100);
  }

  private setupEventListeners(): void {
    if (!this.ws) return;

    // Progress updates
    this.ws.addEventListener('progress', (data: SyncProgressInfo) => {
      console.log('Received sync progress update:', data);
      this.updateState({ progressInfo: data });
    });

    // Heartbeat messages
    this.ws.addEventListener('heartbeat', (data: any) => {
      console.log('Received heartbeat:', data);
      
      // Clear progress info if sync is not active
      if (data && !data.is_active) {
        this.updateState({ progressInfo: null });
      }
    });

    // Connection status changes
    this.ws.addEventListener('connectionStatus', (status: ConnectionStatus) => {
      console.log(`WebSocket connection status changed to: ${status}`);
      this.updateState({ connectionStatus: status });

      // Handle automatic reconnection for certain statuses
      if (status === 'disconnected' || status === 'error') {
        this.scheduleReconnect();
      }
    });

    // Errors
    this.ws.addEventListener('error', (error: any) => {
      this.handleError(new Error(error.message || 'WebSocket error'));
    });
  }

  private handleConnectionError(error: any): void {
    console.error('WebSocket connection error:', error);
    this.updateState({ connectionStatus: 'error' });
    this.handleError(error instanceof Error ? error : new Error(String(error)));
    this.scheduleReconnect();
  }

  private scheduleReconnect(): void {
    // Don't reconnect if we've exceeded max attempts
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      this.updateState({ connectionStatus: 'failed' });
      return;
    }

    // Don't schedule if already scheduled
    if (this.reconnectTimeout) return;

    this.reconnectAttempts++;
    const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1); // Exponential backoff

    console.log(`Scheduling reconnect attempt ${this.reconnectAttempts} in ${delay}ms`);
    this.updateState({ connectionStatus: 'reconnecting' });

    this.reconnectTimeout = setTimeout(() => {
      this.reconnectTimeout = null;
      this.reconnect();
    }, delay);
  }

  private cleanup(): void {
    // Clear reconnect timeout
    if (this.reconnectTimeout) {
      clearTimeout(this.reconnectTimeout);
      this.reconnectTimeout = null;
    }

    // Close WebSocket
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
  }

  destroy(): void {
    this.cleanup();
    super.destroy();
  }
}