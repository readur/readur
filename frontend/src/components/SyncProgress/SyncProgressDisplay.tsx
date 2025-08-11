import React, { useState, useCallback } from 'react';
import {
  Box,
  Typography,
  Collapse,
  IconButton,
  Tooltip,
  useTheme,
  alpha,
  Card,
  CardContent,
  Stack,
  Alert,
} from '@mui/material';
import {
  ExpandMore as ExpandMoreIcon,
  ExpandLess as ExpandLessIcon,
  Speed as SpeedIcon,
  Folder as FolderIcon,
  TextSnippet as FileIcon,
  Storage as StorageIcon,
  CheckCircle as CheckCircleIcon,
  Error as ErrorIcon,
} from '@mui/icons-material';
import { SyncProgressInfo } from '../../services/api';
import { useSyncProgress } from '../../hooks/useSyncProgress';
import { ConnectionStatus } from '../../services/syncProgress';
import { ConnectionStatusIndicator } from './ConnectionStatusIndicator';
import { ProgressStatistics } from './ProgressStatistics';
import { SyncProgressManager } from '../../services/syncProgress';

interface SyncProgressDisplayProps {
  sourceId: string;
  sourceName: string;
  isVisible: boolean;
  onClose?: () => void;
  manager?: SyncProgressManager;
}

export const SyncProgressDisplay: React.FC<SyncProgressDisplayProps> = ({
  sourceId,
  sourceName,
  isVisible,
  onClose,
  manager,
}) => {
  const theme = useTheme();
  const [isExpanded, setIsExpanded] = useState(true);

  // Handle WebSocket connection errors
  const handleWebSocketError = useCallback((error: Error) => {
    console.error('WebSocket connection error in SyncProgressDisplay:', error);
  }, []);

  // Handle connection status changes
  const handleConnectionStatusChange = useCallback((status: ConnectionStatus) => {
    console.log(`Connection status changed to: ${status}`);
  }, []);

  // Use the sync progress hook
  const {
    progressInfo,
    connectionStatus,
    isConnected,
    reconnect,
    disconnect,
  } = useSyncProgress({
    sourceId,
    enabled: isVisible && !!sourceId,
    onError: handleWebSocketError,
    onConnectionStatusChange: handleConnectionStatusChange,
    manager,
  });

  const getPhaseColor = (phase: string) => {
    switch (phase) {
      case 'initializing':
      case 'evaluating':
        return theme.palette.info.main;
      case 'discovering_directories':
      case 'discovering_files':
        return theme.palette.warning.main;
      case 'processing_files':
        return theme.palette.primary.main;
      case 'saving_metadata':
        return theme.palette.secondary.main;
      case 'completed':
        return theme.palette.success.main;
      case 'failed':
        return theme.palette.error.main;
      default:
        return theme.palette.grey[500];
    }
  };

  const getPhaseIcon = (phase: string) => {
    switch (phase) {
      case 'discovering_directories':
        return <FolderIcon />;
      case 'discovering_files':
      case 'processing_files':
        return <FileIcon />;
      case 'saving_metadata':
        return <StorageIcon />;
      case 'completed':
        return <CheckCircleIcon />;
      case 'failed':
        return <ErrorIcon />;
      default:
        return <SpeedIcon />;
    }
  };

  if (!isVisible || (!progressInfo && connectionStatus === 'disconnected' && !isConnected)) {
    return null;
  }

  const phaseColor = progressInfo ? getPhaseColor(progressInfo.phase) : theme.palette.grey[500];

  return (
    <Card
      data-testid="sync-progress"
      sx={{
        mb: 2,
        border: progressInfo?.is_active ? `2px solid ${phaseColor}` : '1px solid',
        borderColor: progressInfo?.is_active ? phaseColor : theme.palette.divider,
        backgroundColor: progressInfo?.is_active 
          ? alpha(phaseColor, 0.05)
          : theme.palette.background.paper,
      }}
    >
      <CardContent sx={{ pb: isExpanded ? 2 : '16px !important' }}>
        <Box display="flex" alignItems="center" justifyContent="space-between" mb={isExpanded ? 2 : 0}>
          <Box display="flex" alignItems="center" gap={2}>
            {progressInfo && (
              <Box
                sx={{
                  color: phaseColor,
                  display: 'flex',
                  alignItems: 'center',
                }}
              >
                {getPhaseIcon(progressInfo.phase)}
              </Box>
            )}
            <Box>
              <Typography variant="h6" component="div">
                {sourceName} - Sync Progress
              </Typography>
              {progressInfo && (
                <Typography variant="body2" color="text.secondary">
                  {progressInfo.phase_description}
                </Typography>
              )}
            </Box>
          </Box>
          <Box display="flex" alignItems="center" gap={1}>
            <ConnectionStatusIndicator
              connectionStatus={connectionStatus}
              isActive={progressInfo?.is_active}
              onReconnect={reconnect}
            />
            
            <Tooltip title={isExpanded ? "Collapse" : "Expand"}>
              <IconButton 
                onClick={() => setIsExpanded(!isExpanded)}
                size="small"
              >
                {isExpanded ? <ExpandLessIcon /> : <ExpandMoreIcon />}
              </IconButton>
            </Tooltip>
          </Box>
        </Box>

        <Collapse in={isExpanded}>
          {progressInfo ? (
            <Stack spacing={2}>
              <ProgressStatistics 
                progressInfo={progressInfo} 
                phaseColor={phaseColor}
              />
            </Stack>
          ) : (
            <Alert severity="info">
              Waiting for sync progress information...
            </Alert>
          )}
        </Collapse>
      </CardContent>
    </Card>
  );
};

export default SyncProgressDisplay;