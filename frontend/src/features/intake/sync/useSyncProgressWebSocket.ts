import { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import { SyncProgressWebSocket, SyncProgressInfo, sourcesService } from '../services/api';

export type ConnectionStatus = 'disconnected' | 'connecting' | 'connected' | 'reconnecting' | 'error' | 'failed';

export interface UseSyncProgressWebSocketOptions {
  sourceId: string;
  enabled?: boolean;
  onError?: (error: any) => void;
  onConnectionStatusChange?: (status: ConnectionStatus) => void;
}

export interface UseSyncProgressWebSocketReturn {
  progressInfo: SyncProgressInfo | null;
  connectionStatus: ConnectionStatus;
  isConnected: boolean;
  reconnect: () => void;
  disconnect: () => void;
}

// Connection state management with proper synchronization
interface ConnectionState {
  status: ConnectionStatus;
  progressInfo: SyncProgressInfo | null;
  lastUpdate: number;
}

/**
 * Custom React hook for managing WebSocket connections to sync progress streams
 * Provides automatic connection management, reconnection logic, and progress data handling
 */
export const useSyncProgressWebSocket = ({
  sourceId,
  enabled = true,
  onError,
  onConnectionStatusChange,
}: UseSyncProgressWebSocketOptions): UseSyncProgressWebSocketReturn => {
  // Use a single state object to prevent race conditions
  const [connectionState, setConnectionState] = useState<ConnectionState>({
    status: 'disconnected',
    progressInfo: null,
    lastUpdate: Date.now(),
  });
  
  const wsRef = useRef<SyncProgressWebSocket | null>(null);
  const mountedRef = useRef(true);
  const stateUpdateTimeoutRef = useRef<NodeJS.Timeout | null>(null);

  // Atomic state update function to prevent race conditions
  const updateConnectionState = useCallback((updates: Partial<ConnectionState>) => {
    if (!mountedRef.current) return;
    
    // Clear any pending state updates to prevent race conditions
    if (stateUpdateTimeoutRef.current) {
      clearTimeout(stateUpdateTimeoutRef.current);
    }
    
    // Use functional update to ensure consistency
    setConnectionState(prevState => {
      const newState = {
        ...prevState,
        ...updates,
        lastUpdate: Date.now(),
      };
      
      // Only notify if status actually changed
      if (updates.status && updates.status !== prevState.status) {
        // Schedule callback on next tick to avoid synchronous state updates
        stateUpdateTimeoutRef.current = setTimeout(() => {
          if (mountedRef.current) {
            onConnectionStatusChange?.(updates.status!);
          }
        }, 0);
      }
      
      return newState;
    });
  }, [onConnectionStatusChange]);

  // Handle progress updates from WebSocket
  const handleProgress = useCallback((data: SyncProgressInfo) => {
    if (!mountedRef.current) return;
    
    console.log('Received sync progress update:', data);
    updateConnectionState({ progressInfo: data });
  }, [updateConnectionState]);

  // Handle heartbeat messages from WebSocket
  const handleHeartbeat = useCallback((data: any) => {
    if (!mountedRef.current) return;
    
    console.log('Received heartbeat:', data);
    
    // Clear progress info if sync is not active
    if (data && !data.is_active) {
      updateConnectionState({ progressInfo: null });
    }
  }, [updateConnectionState]);

  // Handle WebSocket errors
  const handleError = useCallback((error: any) => {
    if (!mountedRef.current) return;
    
    console.error('WebSocket error:', error);
    onError?.(error);
  }, [onError]);

  // Handle connection status changes from WebSocket
  const handleConnectionStatus = useCallback((status: ConnectionStatus) => {
    updateConnectionState({ status });
  }, [updateConnectionState]);

  // Connect to WebSocket
  const connect = useCallback(async () => {
    if (!enabled || !sourceId || !mountedRef.current) {
      return;
    }

    // Cleanup existing connection
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }

    try {
      updateConnectionState({ status: 'connecting' });
      
      const ws = sourcesService.createSyncProgressWebSocket(sourceId);
      wsRef.current = ws;

      // Set up event listeners
      ws.addEventListener('progress', handleProgress);
      ws.addEventListener('heartbeat', handleHeartbeat);
      ws.addEventListener('error', handleError);
      ws.addEventListener('connectionStatus', handleConnectionStatus);

      // Attempt connection
      await ws.connect();
      
      if (mountedRef.current) {
        console.log(`Successfully connected to sync progress WebSocket for source: ${sourceId}`);
      }
    } catch (error) {
      console.error('Failed to connect to sync progress WebSocket:', error);
      if (mountedRef.current) {
        updateConnectionState({ status: 'error' });
        onError?.(error);
      }
    }
  }, [enabled, sourceId, handleProgress, handleHeartbeat, handleError, handleConnectionStatus, updateConnectionState, onError]);

  // Disconnect from WebSocket
  const disconnect = useCallback(() => {
    if (wsRef.current) {
      console.log(`Disconnecting from sync progress WebSocket for source: ${sourceId}`);
      wsRef.current.close();
      wsRef.current = null;
    }
    
    if (mountedRef.current) {
      updateConnectionState({ 
        status: 'disconnected', 
        progressInfo: null 
      });
    }
  }, [sourceId, updateConnectionState]);

  // Reconnect to WebSocket
  const reconnect = useCallback(() => {
    console.log(`Manually reconnecting to sync progress WebSocket for source: ${sourceId}`);
    disconnect();
    
    // Use setTimeout to ensure cleanup is complete before reconnecting
    setTimeout(() => {
      if (mountedRef.current) {
        connect();
      }
    }, 100);
  }, [sourceId, disconnect, connect]);

  // Effect to manage WebSocket connection lifecycle
  useEffect(() => {
    mountedRef.current = true;

    if (enabled && sourceId) {
      connect();
    } else {
      disconnect();
    }

    // Cleanup function
    return () => {
      mountedRef.current = false;
      if (stateUpdateTimeoutRef.current) {
        clearTimeout(stateUpdateTimeoutRef.current);
      }
      disconnect();
    };
  }, [enabled, sourceId, connect, disconnect]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      mountedRef.current = false;
      if (stateUpdateTimeoutRef.current) {
        clearTimeout(stateUpdateTimeoutRef.current);
      }
      if (wsRef.current) {
        wsRef.current.close();
        wsRef.current = null;
      }
    };
  }, []);
  
  // Memoize return values to prevent unnecessary re-renders
  const returnValue = useMemo(() => ({
    progressInfo: connectionState.progressInfo,
    connectionStatus: connectionState.status,
    isConnected: connectionState.status === 'connected',
    reconnect,
    disconnect,
  }), [connectionState.progressInfo, connectionState.status, reconnect, disconnect]);

  return returnValue;
};

export default useSyncProgressWebSocket;