import React from 'react';
import {
  Box,
  Typography,
  Button,
  Stack,
  Chip,
} from '@mui/material';
import {
  Upload as UploadIcon,
  Psychology as OcrIcon,
  CheckCircle as CheckIcon,
  Error as ErrorIcon,
  Refresh as RetryIcon,
  History as HistoryIcon,
} from '@mui/icons-material';
import { useTheme as useMuiTheme } from '@mui/material/styles';
import { type Document, type OcrResponse } from '../../services/api';

interface ActivityTabProps {
  document: Document;
  ocrData: OcrResponse | null;
  onShowRetryHistory: () => void;
}

interface TimelineEvent {
  id: string;
  timestamp: string;
  type: 'upload' | 'ocr_complete' | 'ocr_error' | 'ocr_processing';
  title: string;
  description?: string;
  status: 'success' | 'error' | 'info';
}

const ActivityTab: React.FC<ActivityTabProps> = ({
  document,
  ocrData,
  onShowRetryHistory,
}) => {
  const theme = useMuiTheme();

  const getStatusIcon = (type: string) => {
    switch (type) {
      case 'upload': return <UploadIcon sx={{ fontSize: 18 }} />;
      case 'ocr_complete': return <CheckIcon sx={{ fontSize: 18 }} />;
      case 'ocr_error': return <ErrorIcon sx={{ fontSize: 18 }} />;
      case 'ocr_processing': return <OcrIcon sx={{ fontSize: 18 }} />;
      default: return <CheckIcon sx={{ fontSize: 18 }} />;
    }
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'success': return theme.palette.success.main;
      case 'error': return theme.palette.error.main;
      default: return theme.palette.info.main;
    }
  };

  const formatTimestamp = (timestamp: string) => {
    return new Date(timestamp).toLocaleString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  const events: TimelineEvent[] = [];

  // Upload event
  events.push({
    id: 'upload',
    timestamp: document.created_at,
    type: 'upload',
    title: 'Document Uploaded',
    description: document.username ? `by ${document.username}` : undefined,
    status: 'success',
  });

  // OCR events
  const ocrStatus = document.ocr_status;
  if (ocrStatus === 'completed' && ocrData?.ocr_completed_at) {
    events.push({
      id: 'ocr_complete',
      timestamp: ocrData.ocr_completed_at,
      type: 'ocr_complete',
      title: 'OCR Processing Completed',
      description: ocrData.ocr_word_count != null
        ? `${ocrData.ocr_word_count.toLocaleString()} words extracted`
        : 'Text extraction finished',
      status: 'success',
    });
  } else if (ocrStatus === 'failed') {
    events.push({
      id: 'ocr_error',
      timestamp: document.updated_at,
      type: 'ocr_error',
      title: 'OCR Processing Failed',
      description: document.ocr_error || document.ocr_failure_reason || 'Processing encountered an error',
      status: 'error',
    });
  } else if (ocrStatus === 'processing') {
    events.push({
      id: 'ocr_processing',
      timestamp: document.created_at,
      type: 'ocr_processing',
      title: 'OCR Processing In Progress',
      description: 'Text extraction is currently running',
      status: 'info',
    });
  }

  // Sort chronologically
  events.sort((a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime());

  return (
    <Box>
      <Stack spacing={0}>
        {events.map((event, index) => (
          <Box
            key={event.id}
            sx={{
              display: 'flex',
              gap: 2,
              py: 2,
              borderBottom: index < events.length - 1 ? `1px solid ${theme.palette.divider}` : 'none',
            }}
          >
            {/* Status dot + connector */}
            <Box sx={{ display: 'flex', flexDirection: 'column', alignItems: 'center', pt: 0.25 }}>
              <Box
                sx={{
                  width: 32,
                  height: 32,
                  borderRadius: '50%',
                  backgroundColor: `${getStatusColor(event.status)}15`,
                  border: `2px solid ${getStatusColor(event.status)}`,
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  color: getStatusColor(event.status),
                }}
              >
                {getStatusIcon(event.type)}
              </Box>
            </Box>

            {/* Content */}
            <Box sx={{ flex: 1 }}>
              <Typography variant="subtitle2" sx={{ fontWeight: 600 }}>
                {event.title}
              </Typography>
              {event.description && (
                <Typography variant="body2" color="text.secondary" sx={{ mt: 0.25 }}>
                  {event.description}
                </Typography>
              )}
              <Typography variant="caption" color="text.secondary">
                {formatTimestamp(event.timestamp)}
              </Typography>
            </Box>
          </Box>
        ))}
      </Stack>

      {/* Retry history button */}
      {document.ocr_retry_count != null && document.ocr_retry_count > 0 && (
        <Box sx={{ mt: 3, pt: 2, borderTop: `1px solid ${theme.palette.divider}` }}>
          <Button
            variant="outlined"
            size="small"
            startIcon={<HistoryIcon />}
            onClick={onShowRetryHistory}
            sx={{ textTransform: 'none' }}
          >
            View retry history ({document.ocr_retry_count} retries)
          </Button>
        </Box>
      )}
    </Box>
  );
};

export default ActivityTab;
