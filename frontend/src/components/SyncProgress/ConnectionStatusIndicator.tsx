import React from 'react';
import { Chip, IconButton, Tooltip } from '@mui/material';
import { Refresh as RefreshIcon } from '@mui/icons-material';
import { ConnectionStatus } from '../../services/syncProgress';

interface ConnectionStatusIndicatorProps {
  connectionStatus: ConnectionStatus;
  isActive?: boolean;
  onReconnect?: () => void;
}

export const ConnectionStatusIndicator: React.FC<ConnectionStatusIndicatorProps> = ({
  connectionStatus,
  isActive = false,
  onReconnect
}) => {
  const getStatusConfig = () => {
    switch (connectionStatus) {
      case 'connecting':
        return { label: 'Connecting...', color: 'warning' as const };
      case 'reconnecting':
        return { label: 'Reconnecting...', color: 'warning' as const };
      case 'connected':
        return isActive
          ? { label: 'Live', color: 'success' as const }
          : { label: 'Connected', color: 'info' as const };
      case 'disconnected':
      case 'error':
        return { label: 'Disconnected', color: 'error' as const };
      case 'failed':
        return { label: 'Connection Failed', color: 'error' as const };
      default:
        return { label: 'Unknown', color: 'default' as const };
    }
  };

  const { label, color } = getStatusConfig();
  const showReconnect = (connectionStatus === 'failed' || connectionStatus === 'error') && onReconnect;

  return (
    <>
      <Chip size="small" label={label} color={color} />
      {showReconnect && (
        <Tooltip title="Reconnect">
          <IconButton onClick={onReconnect} size="small" color="primary">
            <RefreshIcon />
          </IconButton>
        </Tooltip>
      )}
    </>
  );
};

export default ConnectionStatusIndicator;