import React, { useState, useEffect } from 'react';
import {
  Box,
  Typography,
  Paper,
  Chip,
  Stack,
  Button,
  Collapse,
  CircularProgress,
  Alert,
} from '@mui/material';
import {
  Timeline,
  TimelineItem,
  TimelineSeparator,
  TimelineConnector,
  TimelineContent,
  TimelineDot,
} from '@mui/lab';
import {
  Timeline as TimelineIcon,
  Upload as UploadIcon,
  Psychology as OcrIcon,
  CheckCircle as CheckIcon,
  Error as ErrorIcon,
  Refresh as RetryIcon,
  ExpandMore as ExpandIcon,
  Schedule as ScheduleIcon,
  Person as PersonIcon,
} from '@mui/icons-material';
import { useTheme } from '../contexts/ThemeContext';
import { useTheme as useMuiTheme } from '@mui/material/styles';
import { documentService } from '../services/api';

interface ProcessingTimelineProps {
  documentId: string;
  fileName: string;
  createdAt: string;
  updatedAt: string;
  userId: string;
  ocrStatus?: string;
  ocrCompletedAt?: string;
  ocrRetryCount?: number;
  ocrError?: string;
  compact?: boolean;
}

interface TimelineEvent {
  id: string;
  timestamp: string;
  type: 'upload' | 'ocr_start' | 'ocr_complete' | 'ocr_retry' | 'ocr_error' | 'update';
  title: string;
  description?: string;
  status: 'success' | 'error' | 'warning' | 'info';
  metadata?: Record<string, any>;
}

const ProcessingTimeline: React.FC<ProcessingTimelineProps> = ({
  documentId,
  fileName,
  createdAt,
  updatedAt,
  userId,
  ocrStatus,
  ocrCompletedAt,
  ocrRetryCount = 0,
  ocrError,
  compact = false,
}) => {
  const [expanded, setExpanded] = useState(!compact);
  const [retryHistory, setRetryHistory] = useState<any[]>([]);
  const [loadingHistory, setLoadingHistory] = useState(false);
  const { modernTokens } = useTheme();
  const theme = useMuiTheme();

  const getStatusIcon = (type: string, status: string) => {
    switch (type) {
      case 'upload':
        return <UploadIcon />;
      case 'ocr_start':
      case 'ocr_complete':
        return <OcrIcon />;
      case 'ocr_retry':
        return <RetryIcon />;
      case 'ocr_error':
        return <ErrorIcon />;
      default:
        return <CheckIcon />;
    }
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'success':
        return theme.palette.success.main;
      case 'error':
        return theme.palette.error.main;
      case 'warning':
        return theme.palette.warning.main;
      default:
        return theme.palette.info.main;
    }
  };

  const generateTimelineEvents = (): TimelineEvent[] => {
    const events: TimelineEvent[] = [];

    // Upload event
    events.push({
      id: 'upload',
      timestamp: createdAt,
      type: 'upload',
      title: 'Document Uploaded',
      description: `File "${fileName}" was uploaded successfully`,
      status: 'success',
      metadata: { userId },
    });

    // OCR processing events
    if (ocrStatus) {
      if (ocrStatus === 'completed' && ocrCompletedAt) {
        events.push({
          id: 'ocr_complete',
          timestamp: ocrCompletedAt,
          type: 'ocr_complete',
          title: 'OCR Processing Completed',
          description: 'Text extraction finished successfully',
          status: 'success',
        });
      } else if (ocrStatus === 'failed' && ocrError) {
        events.push({
          id: 'ocr_error',
          timestamp: updatedAt,
          type: 'ocr_error',
          title: 'OCR Processing Failed',
          description: ocrError,
          status: 'error',
        });
      } else if (ocrStatus === 'processing') {
        events.push({
          id: 'ocr_start',
          timestamp: createdAt,
          type: 'ocr_start',
          title: 'OCR Processing Started',
          description: 'Text extraction is in progress',
          status: 'info',
        });
      }
    }

    // Retry events
    if (ocrRetryCount && ocrRetryCount > 0) {
      for (let i = 0; i < ocrRetryCount; i++) {
        events.push({
          id: `retry_${i}`,
          timestamp: updatedAt, // In real implementation, get actual retry timestamps
          type: 'ocr_retry',
          title: `OCR Retry Attempt ${i + 1}`,
          description: 'Attempting to reprocess document',
          status: 'warning',
        });
      }
    }

    // Sort by timestamp
    return events.sort((a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime());
  };

  const loadRetryHistory = async () => {
    if (loadingHistory) return;
    
    setLoadingHistory(true);
    try {
      // Note: This endpoint might not exist yet, it's for future implementation
      const response = await documentService.getDocumentRetryHistory(documentId);
      if (response?.data?.retry_history) {
        setRetryHistory(response.data.retry_history);
      }
    } catch (error) {
      console.error('Failed to load retry history:', error);
    } finally {
      setLoadingHistory(false);
    }
  };

  useEffect(() => {
    if (expanded && ocrRetryCount > 0) {
      loadRetryHistory();
    }
  }, [expanded, ocrRetryCount]);

  const formatTimestamp = (timestamp: string) => {
    return new Date(timestamp).toLocaleString('en-US', {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  const formatDuration = (start: string, end: string) => {
    const startTime = new Date(start).getTime();
    const endTime = new Date(end).getTime();
    const duration = endTime - startTime;
    
    if (duration < 1000) return `${duration}ms`;
    if (duration < 60000) return `${Math.round(duration / 1000)}s`;
    return `${Math.round(duration / 60000)}m`;
  };

  const events = generateTimelineEvents();

  if (compact) {
    return (
      <Paper 
        sx={{ 
          p: 2,
          backgroundColor: theme.palette.background.paper,
          border: `1px solid ${theme.palette.divider}`,
        }}
      >
        <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 2 }}>
          <Box sx={{ display: 'flex', alignItems: 'center' }}>
            <TimelineIcon 
              sx={{ 
                fontSize: 18, 
                mr: 1, 
                color: theme.palette.primary.main 
              }} 
            />
            <Typography variant="subtitle2" sx={{ fontWeight: 600 }}>
              Processing Timeline
            </Typography>
          </Box>
          <Typography variant="caption" color="text.secondary">
            {events.length} events
          </Typography>
        </Box>
        
        <Stack spacing={1}>
          {events.slice(-2).map((event, index) => (
            <Box key={event.id} sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
              <Box sx={{ display: 'flex', alignItems: 'center' }}>
                <Box
                  sx={{
                    width: 8,
                    height: 8,
                    borderRadius: '50%',
                    backgroundColor: getStatusColor(event.status),
                    mr: 1,
                  }}
                />
                <Typography variant="caption" sx={{ fontWeight: 500 }}>
                  {event.title}
                </Typography>
              </Box>
              <Typography variant="caption" color="text.secondary">
                {formatTimestamp(event.timestamp)}
              </Typography>
            </Box>
          ))}
        </Stack>
        
        <Button
          size="small"
          onClick={() => setExpanded(true)}
          sx={{ mt: 1, fontSize: '0.75rem' }}
        >
          View Full Timeline
        </Button>
      </Paper>
    );
  }

  return (
    <Paper 
      sx={{ 
        p: 3,
        backgroundColor: theme.palette.background.paper,
        border: `1px solid ${theme.palette.divider}`,
      }}
    >
      {/* Header */}
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 3 }}>
        <Box sx={{ display: 'flex', alignItems: 'center' }}>
          <TimelineIcon 
            sx={{ 
              fontSize: 24, 
              mr: 1.5, 
              color: modernTokens.colors.primary[500] 
            }} 
          />
          <Typography variant="h6" sx={{ fontWeight: 600 }}>
            Processing Timeline
          </Typography>
        </Box>
        
        <Stack direction="row" spacing={1}>
          <Chip 
            label={`${events.length} events`}
            size="small"
            sx={{ backgroundColor: theme.palette.action.hover }}
          />
          {ocrRetryCount > 0 && (
            <Chip 
              label={`${ocrRetryCount} retries`}
              size="small"
              color="warning"
            />
          )}
        </Stack>
      </Box>

      {/* Timeline */}
      <Timeline sx={{ p: 0 }}>
        {events.map((event, index) => (
          <TimelineItem key={event.id}>
            <TimelineSeparator>
              <TimelineDot 
                sx={{ 
                  backgroundColor: getStatusColor(event.status),
                  color: 'white',
                }}
              >
                {getStatusIcon(event.type, event.status)}
              </TimelineDot>
              {index < events.length - 1 && (
                <TimelineConnector 
                  sx={{ backgroundColor: theme.palette.action.selected }}
                />
              )}
            </TimelineSeparator>
            
            <TimelineContent sx={{ py: 1 }}>
              <Box sx={{ mb: 1 }}>
                <Typography variant="subtitle2" sx={{ fontWeight: 600 }}>
                  {event.title}
                </Typography>
                <Typography variant="caption" color="text.secondary">
                  {formatTimestamp(event.timestamp)}
                </Typography>
              </Box>
              
              {event.description && (
                <Typography variant="body2" color="text.secondary" sx={{ mb: 1 }}>
                  {event.description}
                </Typography>
              )}
              
              {event.metadata?.userId && (
                <Box sx={{ display: 'flex', alignItems: 'center', mt: 1 }}>
                  <PersonIcon sx={{ fontSize: 14, mr: 0.5, color: theme.palette.text.secondary }} />
                  <Typography variant="caption" color="text.secondary">
                    User: {event.metadata.userId.substring(0, 8)}...
                  </Typography>
                </Box>
              )}
              
              {index > 0 && events[index - 1] && (
                <Typography variant="caption" color="text.secondary" sx={{ fontStyle: 'italic' }}>
                  (+{formatDuration(events[index - 1].timestamp, event.timestamp)})
                </Typography>
              )}
            </TimelineContent>
          </TimelineItem>
        ))}
      </Timeline>

      {/* Retry History Section */}
      {ocrRetryCount > 0 && (
        <Box sx={{ mt: 3, pt: 2, borderTop: `1px solid ${theme.palette.divider}` }}>
          <Button
            onClick={() => setExpanded(!expanded)}
            endIcon={<ExpandIcon sx={{ transform: expanded ? 'rotate(180deg)' : 'none' }} />}
            sx={{ mb: 2 }}
          >
            Detailed Retry History
          </Button>
          
          <Collapse in={expanded}>
            {loadingHistory ? (
              <Box sx={{ display: 'flex', alignItems: 'center', py: 2 }}>
                <CircularProgress size={20} sx={{ mr: 1 }} />
                <Typography variant="body2" color="text.secondary">
                  Loading retry history...
                </Typography>
              </Box>
            ) : retryHistory.length > 0 ? (
              <Stack spacing={1}>
                {retryHistory.map((retry, index) => (
                  <Paper 
                    key={retry.id} 
                    sx={{ 
                      p: 2, 
                      backgroundColor: theme.palette.background.default,
                      border: `1px solid ${theme.palette.divider}`,
                    }}
                  >
                    <Typography variant="subtitle2">
                      Retry #{index + 1}
                    </Typography>
                    <Typography variant="caption" color="text.secondary">
                      {retry.retry_reason || 'Manual retry'}
                    </Typography>
                  </Paper>
                ))}
              </Stack>
            ) : (
              <Alert severity="info">
                Detailed retry history not available. Enable detailed logging for future retries.
              </Alert>
            )}
          </Collapse>
        </Box>
      )}
    </Paper>
  );
};

export default ProcessingTimeline;