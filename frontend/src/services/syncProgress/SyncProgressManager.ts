import { EventEmitter } from 'events';
import { SyncProgressInfo } from '../api';

export type ConnectionStatus = 'disconnected' | 'connecting' | 'connected' | 'reconnecting' | 'error' | 'failed';

export interface SyncProgressState {
  progressInfo: SyncProgressInfo | null;
  connectionStatus: ConnectionStatus;
  lastUpdate: number;
}

export interface SyncProgressEvents {
  'stateChange': (state: SyncProgressState) => void;
  'progressUpdate': (progressInfo: SyncProgressInfo) => void;
  'connectionStatusChange': (status: ConnectionStatus) => void;
  'error': (error: Error) => void;
}

/**
 * Abstract base class for sync progress management
 * Provides a clean interface for components to consume sync progress data
 * without being coupled to WebSocket implementation details
 */
export abstract class SyncProgressManager extends EventEmitter {
  protected state: SyncProgressState = {
    progressInfo: null,
    connectionStatus: 'disconnected',
    lastUpdate: Date.now()
  };

  constructor() {
    super();
    this.setMaxListeners(20); // Prevent memory leak warnings
  }

  /**
   * Get current state
   */
  getState(): SyncProgressState {
    return { ...this.state };
  }

  /**
   * Connect to sync progress source
   */
  abstract connect(sourceId: string): Promise<void>;

  /**
   * Disconnect from sync progress source
   */
  abstract disconnect(): void;

  /**
   * Reconnect to sync progress source
   */
  abstract reconnect(): void;

  /**
   * Check if currently connected
   */
  isConnected(): boolean {
    return this.state.connectionStatus === 'connected';
  }

  /**
   * Update state and emit events
   */
  protected updateState(updates: Partial<SyncProgressState>): void {
    const prevState = this.state;
    this.state = {
      ...this.state,
      ...updates,
      lastUpdate: Date.now()
    };

    // Emit state change event
    this.emit('stateChange', this.getState());

    // Emit specific events for what changed
    if (updates.progressInfo !== undefined && updates.progressInfo !== prevState.progressInfo) {
      if (updates.progressInfo) {
        this.emit('progressUpdate', updates.progressInfo);
      }
    }

    if (updates.connectionStatus !== undefined && updates.connectionStatus !== prevState.connectionStatus) {
      this.emit('connectionStatusChange', updates.connectionStatus);
    }
  }

  /**
   * Handle errors
   */
  protected handleError(error: Error): void {
    console.error('SyncProgressManager error:', error);
    this.emit('error', error);
    this.updateState({ connectionStatus: 'error' });
  }

  /**
   * Clean up resources
   */
  destroy(): void {
    this.disconnect();
    this.removeAllListeners();
  }
}