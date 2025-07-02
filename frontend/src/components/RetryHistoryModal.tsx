import React, { useState, useEffect } from 'react';
import {
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  Button,
  Typography,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Paper,
  Alert,
  LinearProgress,
  Box,
  Chip,
  Tooltip,
  IconButton,
} from '@mui/material';
import {
  History as HistoryIcon,
  Close as CloseIcon,
  Refresh as RefreshIcon,
  Schedule as ScheduleIcon,
  PriorityHigh as PriorityIcon,
} from '@mui/icons-material';
import { documentService, DocumentRetryHistoryItem } from '../services/api';
import { format, formatDistanceToNow } from 'date-fns';

interface RetryHistoryModalProps {
  open: boolean;
  onClose: () => void;
  documentId: string;
  documentName?: string;
}

const RETRY_REASON_LABELS: Record<string, string> = {
  manual_retry: 'Manual Retry',
  bulk_retry_all: 'Bulk Retry (All)',
  bulk_retry_specific: 'Bulk Retry (Selected)',
  bulk_retry_filtered: 'Bulk Retry (Filtered)',
  scheduled_retry: 'Scheduled Retry',
  auto_retry: 'Automatic Retry',
};

const STATUS_COLORS: Record<string, 'default' | 'primary' | 'secondary' | 'error' | 'info' | 'success' | 'warning'> = {
  pending: 'info',
  processing: 'warning',
  completed: 'success',
  failed: 'error',
  cancelled: 'default',
};

export const RetryHistoryModal: React.FC<RetryHistoryModalProps> = ({
  open,
  onClose,
  documentId,
  documentName,
}) => {
  const [history, setHistory] = useState<DocumentRetryHistoryItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [totalRetries, setTotalRetries] = useState(0);

  const loadRetryHistory = async () => {
    if (!documentId) return;
    
    setLoading(true);
    setError(null);
    try {
      const response = await documentService.getDocumentRetryHistory(documentId);
      setHistory(response.data.retry_history);
      setTotalRetries(response.data.total_retries);
    } catch (err: any) {
      setError(err.response?.data?.message || 'Failed to load retry history');
      setHistory([]);
      setTotalRetries(0);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (open && documentId) {
      loadRetryHistory();
    }
  }, [open, documentId]);

  const formatRetryReason = (reason: string) => {
    return RETRY_REASON_LABELS[reason] || reason.replace(/_/g, ' ');
  };

  const getPriorityLabel = (priority: number) => {
    if (priority >= 15) return 'Very High';
    if (priority >= 12) return 'High';
    if (priority >= 8) return 'Medium';
    if (priority >= 5) return 'Low';
    return 'Very Low';
  };

  const getPriorityColor = (priority: number): 'default' | 'primary' | 'secondary' | 'error' | 'info' | 'success' | 'warning' => {
    if (priority >= 15) return 'error';
    if (priority >= 12) return 'warning';
    if (priority >= 8) return 'primary';
    if (priority >= 5) return 'info';
    return 'default';
  };

  return (
    <Dialog open={open} onClose={onClose} maxWidth="lg" fullWidth>
      <DialogTitle>
        <Box display="flex" alignItems="center" justifyContent="space-between">
          <Box display="flex" alignItems="center" gap={1}>
            <HistoryIcon />
            <Box>
              <Typography variant="h6">OCR Retry History</Typography>
              {documentName && (
                <Typography variant="body2" color="text.secondary">
                  {documentName}
                </Typography>
              )}
            </Box>
          </Box>
          <IconButton onClick={onClose} size="small">
            <CloseIcon />
          </IconButton>
        </Box>
      </DialogTitle>

      <DialogContent>
        {error && (
          <Alert severity="error" sx={{ mb: 2 }}>
            {error}
          </Alert>
        )}

        {loading ? (
          <Box>
            <LinearProgress />
            <Typography variant="body2" color="text.secondary" mt={1} textAlign="center">
              Loading retry history...
            </Typography>
          </Box>
        ) : history.length === 0 ? (
          <Alert severity="info">
            <Typography variant="body1">
              No retry attempts found for this document.
            </Typography>
            <Typography variant="body2" color="text.secondary" mt={1}>
              This document hasn't been retried yet, or retry history is not available.
            </Typography>
          </Alert>
        ) : (
          <Box>
            {/* Summary */}
            <Alert severity="info" sx={{ mb: 3 }}>
              <Typography variant="body1">
                <strong>{totalRetries}</strong> retry attempts found for this document.
              </Typography>
              <Typography variant="body2" color="text.secondary">
                Most recent attempt: {formatDistanceToNow(new Date(history[0].created_at))} ago
              </Typography>
            </Alert>

            {/* History Table */}
            <TableContainer component={Paper}>
              <Table>
                <TableHead>
                  <TableRow>
                    <TableCell>Date & Time</TableCell>
                    <TableCell>Retry Reason</TableCell>
                    <TableCell>Previous Status</TableCell>
                    <TableCell>Priority</TableCell>
                    <TableCell>Queue Status</TableCell>
                  </TableRow>
                </TableHead>
                <TableBody>
                  {history.map((item, index) => (
                    <TableRow key={item.id} hover>
                      <TableCell>
                        <Box>
                          <Typography variant="body2">
                            {format(new Date(item.created_at), 'MMM dd, yyyy')}
                          </Typography>
                          <Typography variant="body2" color="text.secondary">
                            {format(new Date(item.created_at), 'h:mm a')}
                          </Typography>
                          <Typography variant="caption" color="text.secondary">
                            ({formatDistanceToNow(new Date(item.created_at))} ago)
                          </Typography>
                        </Box>
                      </TableCell>
                      
                      <TableCell>
                        <Chip
                          label={formatRetryReason(item.retry_reason)}
                          size="small"
                          variant="outlined"
                        />
                      </TableCell>
                      
                      <TableCell>
                        <Box>
                          {item.previous_status && (
                            <Chip
                              label={item.previous_status}
                              size="small"
                              color={STATUS_COLORS[item.previous_status] || 'default'}
                              sx={{ mb: 0.5 }}
                            />
                          )}
                          {item.previous_failure_reason && (
                            <Typography variant="caption" display="block" color="text.secondary">
                              {item.previous_failure_reason.replace(/_/g, ' ')}
                            </Typography>
                          )}
                          {item.previous_error && (
                            <Tooltip title={item.previous_error}>
                              <Typography variant="caption" display="block" color="error.main" sx={{ 
                                maxWidth: 200,
                                overflow: 'hidden',
                                textOverflow: 'ellipsis',
                                whiteSpace: 'nowrap',
                                cursor: 'help'
                              }}>
                                {item.previous_error}
                              </Typography>
                            </Tooltip>
                          )}
                        </Box>
                      </TableCell>
                      
                      <TableCell>
                        <Tooltip title={`Priority: ${item.priority}/20`}>
                          <Chip
                            icon={<PriorityIcon fontSize="small" />}
                            label={`${getPriorityLabel(item.priority)} (${item.priority})`}
                            size="small"
                            color={getPriorityColor(item.priority)}
                          />
                        </Tooltip>
                      </TableCell>
                      
                      <TableCell>
                        {item.queue_id ? (
                          <Box>
                            <Typography variant="body2" color="success.main">
                              ✓ Queued
                            </Typography>
                            <Typography variant="caption" color="text.secondary">
                              ID: {item.queue_id.slice(0, 8)}...
                            </Typography>
                          </Box>
                        ) : (
                          <Typography variant="body2" color="warning.main">
                            ⚠ Not queued
                          </Typography>
                        )}
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </TableContainer>

            {/* Legend */}
            <Box mt={2} p={2} bgcolor="grey.50" borderRadius={1}>
              <Typography variant="caption" color="text.secondary" paragraph>
                <strong>Priority Levels:</strong> Very High (15-20), High (12-14), Medium (8-11), Low (5-7), Very Low (1-4)
              </Typography>
              <Typography variant="caption" color="text.secondary">
                <strong>Retry Reasons:</strong> Manual (user-initiated), Bulk (batch operations), Scheduled (automatic), Auto (system-triggered)
              </Typography>
            </Box>
          </Box>
        )}
      </DialogContent>

      <DialogActions>
        <Button
          startIcon={<RefreshIcon />}
          onClick={loadRetryHistory}
          disabled={loading}
        >
          Refresh
        </Button>
        <Button onClick={onClose} variant="contained">
          Close
        </Button>
      </DialogActions>
    </Dialog>
  );
};