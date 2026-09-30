import { useState, useEffect, useRef, useCallback } from 'react';
import { SyncProgressManager, SyncProgressState, ConnectionStatus, WebSocketSyncProgressManager } from '../services/syncProgress';
import { SyncProgressInfo } from '../services/api';

export interface UseSyncProgressOptions {
  sourceId: string;
  enabled?: boolean;
  onError?: (error: Error) => void;
  onConnectionStatusChange?: (status: ConnectionStatus) => void;
  // Allow injecting a custom manager for testing
  manager?: SyncProgressManager;
}

export interface UseSyncProgressReturn {
  progressInfo: SyncProgressInfo | null;
  connectionStatus: ConnectionStatus;
  isConnected: boolean;
  reconnect: () => void;
  disconnect: () => void;
}

/**
 * React hook for managing sync progress state
 * Uses the SyncProgressManager abstraction for clean separation of concerns
 */
export const useSyncProgress = ({
  sourceId,
  enabled = true,
  onError,
  onConnectionStatusChange,
  manager: injectedManager
}: UseSyncProgressOptions): UseSyncProgressReturn => {
  const [state, setState] = useState<SyncProgressState>({
    progressInfo: null,
    connectionStatus: 'disconnected',
    lastUpdate: Date.now()
  });

  const managerRef = useRef<SyncProgressManager | null>(null);
  const mountedRef = useRef(true);

  // Create or use injected manager
  useEffect(() => {
    if (!managerRef.current) {
      managerRef.current = injectedManager || new WebSocketSyncProgressManager();
    }
    return () => {
      if (!injectedManager && managerRef.current) {
        managerRef.current.destroy();
        managerRef.current = null;
      }
    };
  }, [injectedManager]);

  // Set up event listeners
  useEffect(() => {
    const manager = managerRef.current;
    if (!manager) return;

    const handleStateChange = (newState: SyncProgressState) => {
      if (mountedRef.current) {
        setState(newState);
      }
    };

    const handleConnectionStatusChange = (status: ConnectionStatus) => {
      if (mountedRef.current) {
        onConnectionStatusChange?.(status);
      }
    };

    const handleError = (error: Error) => {
      if (mountedRef.current) {
        onError?.(error);
      }
    };

    manager.on('stateChange', handleStateChange);
    manager.on('connectionStatusChange', handleConnectionStatusChange);
    manager.on('error', handleError);

    // Set initial state
    setState(manager.getState());

    return () => {
      manager.off('stateChange', handleStateChange);
      manager.off('connectionStatusChange', handleConnectionStatusChange);
      manager.off('error', handleError);
    };
  }, [onConnectionStatusChange, onError]);

  // Handle connection lifecycle
  useEffect(() => {
    const manager = managerRef.current;
    if (!manager) return;

    if (enabled && sourceId) {
      manager.connect(sourceId).catch(error => {
        console.error('Failed to connect to sync progress:', error);
      });
    } else {
      manager.disconnect();
    }

    return () => {
      manager.disconnect();
    };
  }, [enabled, sourceId]);

  // Track mounted state
  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  // Callbacks
  const reconnect = useCallback(() => {
    managerRef.current?.reconnect();
  }, []);

  const disconnect = useCallback(() => {
    managerRef.current?.disconnect();
  }, []);

  return {
    progressInfo: state.progressInfo,
    connectionStatus: state.connectionStatus,
    isConnected: state.connectionStatus === 'connected',
    reconnect,
    disconnect
  };
};

export default useSyncProgress;