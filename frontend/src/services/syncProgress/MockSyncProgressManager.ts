import { SyncProgressManager, ConnectionStatus } from './SyncProgressManager';
import { SyncProgressInfo } from '../api';

/**
 * Mock implementation of SyncProgressManager for testing
 * Provides controllable sync progress updates without WebSocket dependencies
 */
export class MockSyncProgressManager extends SyncProgressManager {
  private connected = false;
  private sourceId: string | null = null;

  async connect(sourceId: string): Promise<void> {
    this.sourceId = sourceId;
    this.updateState({ connectionStatus: 'connecting' });

    // Simulate async connection
    await new Promise(resolve => setTimeout(resolve, 10));

    this.connected = true;
    this.updateState({ connectionStatus: 'connected' });
  }

  disconnect(): void {
    this.connected = false;
    this.sourceId = null;
    this.updateState({
      connectionStatus: 'disconnected',
      progressInfo: null
    });
  }

  reconnect(): void {
    if (!this.sourceId) return;
    
    const sourceId = this.sourceId;
    this.disconnect();
    
    setTimeout(() => {
      this.connect(sourceId).catch(console.error);
    }, 10);
  }

  // Test helper methods

  /**
   * Simulate a progress update
   */
  simulateProgress(progressInfo: SyncProgressInfo): void {
    if (!this.connected) {
      console.warn('Cannot simulate progress: not connected');
      return;
    }
    this.updateState({ progressInfo });
  }

  /**
   * Simulate a connection status change
   */
  simulateConnectionStatus(status: ConnectionStatus): void {
    this.updateState({ connectionStatus: status });
    if (status === 'connected') {
      this.connected = true;
    } else if (status === 'disconnected' || status === 'failed') {
      this.connected = false;
    }
  }

  /**
   * Simulate a heartbeat
   */
  simulateHeartbeat(data: { source_id: string; is_active: boolean; timestamp: number }): void {
    if (!this.connected) return;

    if (!data.is_active) {
      this.updateState({ progressInfo: null });
    }
  }

  /**
   * Simulate an error
   */
  simulateError(error: Error): void {
    this.handleError(error);
  }
}