import React, { useState } from 'react';
import {
  Box,
  Typography,
  Button,
  IconButton,
  Divider,
  Chip,
  Card,
  CardContent,
  Collapse,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  TextField,
  FormControlLabel,
  Switch,
  Alert,
  Stack,
  Tooltip,
  Paper,
} from '@mui/material';
import Grid from '@mui/material/GridLegacy';
import {
  ContentCopy as CopyIcon,
  ExpandMore as ExpandMoreIcon,
  ExpandLess as ExpandLessIcon,
  Refresh as RefreshIcon,
  Block as BlockIcon,
  Schedule as ScheduleIcon,
  Speed as SpeedIcon,
  Folder as FolderIcon,
  CloudOff as CloudOffIcon,
  Timer as TimerIcon,
  Info as InfoIcon,
  Warning as WarningIcon,
} from '@mui/icons-material';
import { alpha } from '@mui/material/styles';

import { WebDAVScanFailure } from '../../services/api';
import { modernTokens } from '../../theme';
import { useNotification } from '../../contexts/NotificationContext';

interface FailureDetailsPanelProps {
  failure: WebDAVScanFailure;
  onRetry: (failure: WebDAVScanFailure, notes?: string) => Promise<void>;
  onExclude: (failure: WebDAVScanFailure, notes?: string, permanent?: boolean) => Promise<void>;
  isRetrying?: boolean;
  isExcluding?: boolean;
}

interface ConfirmationDialogProps {
  open: boolean;
  onClose: () => void;
  onConfirm: (notes?: string, permanent?: boolean) => void;
  title: string;
  description: string;
  confirmText: string;
  confirmColor?: 'primary' | 'error' | 'warning';
  showPermanentOption?: boolean;
  isLoading?: boolean;
}

const ConfirmationDialog: React.FC<ConfirmationDialogProps> = ({
  open,
  onClose,
  onConfirm,
  title,
  description,
  confirmText,
  confirmColor = 'primary',
  showPermanentOption = false,
  isLoading = false,
}) => {
  const [notes, setNotes] = useState('');
  const [permanent, setPermanent] = useState(true);

  const handleConfirm = () => {
    onConfirm(notes || undefined, showPermanentOption ? permanent : undefined);
    setNotes('');
    setPermanent(true);
  };

  const handleClose = () => {
    setNotes('');
    setPermanent(true);
    onClose();
  };

  return (
    <Dialog open={open} onClose={handleClose} maxWidth="sm" fullWidth>
      <DialogTitle>{title}</DialogTitle>
      <DialogContent>
        <Typography variant="body1" sx={{ mb: 3 }}>
          {description}
        </Typography>
        
        <TextField
          fullWidth
          label="Notes (optional)"
          value={notes}
          onChange={(e) => setNotes(e.target.value)}
          multiline
          rows={3}
          sx={{ mb: 2 }}
        />

        {showPermanentOption && (
          <FormControlLabel
            control={
              <Switch
                checked={permanent}
                onChange={(e) => setPermanent(e.target.checked)}
              />
            }
            label="Permanently exclude (recommended)"
            sx={{ mt: 1 }}
          />
        )}
      </DialogContent>
      <DialogActions>
        <Button onClick={handleClose} disabled={isLoading}>
          Cancel
        </Button>
        <Button
          onClick={handleConfirm}
          color={confirmColor}
          variant="contained"
          disabled={isLoading}
        >
          {confirmText}
        </Button>
      </DialogActions>
    </Dialog>
  );
};

const FailureDetailsPanel: React.FC<FailureDetailsPanelProps> = ({
  failure,
  onRetry,
  onExclude,
  isRetrying = false,
  isExcluding = false,
}) => {
  const [showDiagnostics, setShowDiagnostics] = useState(false);
  const [retryDialogOpen, setRetryDialogOpen] = useState(false);
  const [excludeDialogOpen, setExcludeDialogOpen] = useState(false);

  const { showNotification } = useNotification();

  // Handle copy to clipboard
  const handleCopy = async (text: string, label: string) => {
    try {
      await navigator.clipboard.writeText(text);
      showNotification({
        type: 'success',
        message: `${label} copied to clipboard`,
      });
    } catch (error) {
      showNotification({
        type: 'error',
        message: `Failed to copy ${label}`,
      });
    }
  };

  // Format bytes
  const formatBytes = (bytes?: number) => {
    if (!bytes) return 'N/A';
    const sizes = ['Bytes', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(1024));
    return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${sizes[i]}`;
  };

  // Format duration
  const formatDuration = (ms?: number) => {
    if (!ms) return 'N/A';
    if (ms < 1000) return `${ms}ms`;
    const seconds = Math.floor(ms / 1000);
    if (seconds < 60) return `${seconds}s`;
    const minutes = Math.floor(seconds / 60);
    return `${minutes}m ${seconds % 60}s`;
  };

  // Get recommendation color and icon
  const getRecommendationStyle = () => {
    if (failure.diagnostic_summary.user_action_required) {
      return {
        color: modernTokens.colors.warning[600],
        bgColor: modernTokens.colors.warning[50],
        icon: WarningIcon,
      };
    }
    return {
      color: modernTokens.colors.info[600],
      bgColor: modernTokens.colors.info[50],
      icon: InfoIcon,
    };
  };

  const recommendationStyle = getRecommendationStyle();
  const RecommendationIcon = recommendationStyle.icon;

  return (
    <Box>
      {/* Error Message */}
      {failure.error_message && (
        <Alert 
          severity="error" 
          sx={{ 
            mb: 3,
            borderRadius: 2,
          }}
          action={
            <IconButton
              size="small"
              onClick={() => handleCopy(failure.error_message!, 'Error message')}
            >
              <CopyIcon fontSize="small" />
            </IconButton>
          }
        >
          <Typography variant="body2" sx={{ fontFamily: 'monospace', wordBreak: 'break-all' }}>
            {failure.error_message}
          </Typography>
        </Alert>
      )}

      {/* Basic Information */}
      <Grid container spacing={3} sx={{ mb: 3 }}>
        <Grid item xs={12} md={6}>
          <Card 
            variant="outlined"
            sx={{ 
              height: '100%',
              backgroundColor: modernTokens.colors.neutral[50],
            }}
          >
            <CardContent>
              <Typography 
                variant="h6" 
                sx={{ 
                  fontWeight: 600,
                  mb: 2,
                  color: modernTokens.colors.neutral[900],
                }}
              >
                Directory Information
              </Typography>
              
              <Stack spacing={2}>
                <Box>
                  <Typography variant="body2" color="text.secondary" sx={{ mb: 0.5 }}>
                    Path
                  </Typography>
                  <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                    <Typography 
                      variant="body1" 
                      sx={{ 
                        fontFamily: 'monospace',
                        fontSize: '0.875rem',
                        wordBreak: 'break-all',
                        flex: 1,
                      }}
                    >
                      {failure.directory_path}
                    </Typography>
                    <Tooltip title="Copy path">
                      <IconButton 
                        size="small"
                        onClick={() => handleCopy(failure.directory_path, 'Directory path')}
                      >
                        <CopyIcon fontSize="small" />
                      </IconButton>
                    </Tooltip>
                  </Box>
                </Box>

                <Divider />

                <Box>
                  <Typography variant="body2" color="text.secondary" sx={{ mb: 0.5 }}>
                    Failure Count
                  </Typography>
                  <Typography variant="body1">
                    {failure.failure_count} total • {failure.consecutive_failures} consecutive
                  </Typography>
                </Box>

                <Box>
                  <Typography variant="body2" color="text.secondary" sx={{ mb: 0.5 }}>
                    Timeline
                  </Typography>
                  <Typography variant="body2" sx={{ mb: 0.5 }}>
                    <strong>First failure:</strong> {new Date(failure.first_failure_at).toLocaleString()}
                  </Typography>
                  <Typography variant="body2" sx={{ mb: 0.5 }}>
                    <strong>Last failure:</strong> {new Date(failure.last_failure_at).toLocaleString()}
                  </Typography>
                  {failure.next_retry_at && (
                    <Typography variant="body2">
                      <strong>Next retry:</strong> {new Date(failure.next_retry_at).toLocaleString()}
                    </Typography>
                  )}
                </Box>

                {failure.http_status_code && (
                  <Box>
                    <Typography variant="body2" color="text.secondary" sx={{ mb: 0.5 }}>
                      HTTP Status
                    </Typography>
                    <Chip
                      label={`${failure.http_status_code}`}
                      size="small"
                      color={failure.http_status_code < 400 ? 'success' : 'error'}
                    />
                  </Box>
                )}
              </Stack>
            </CardContent>
          </Card>
        </Grid>

        <Grid item xs={12} md={6}>
          <Card 
            variant="outlined"
            sx={{ 
              height: '100%',
              backgroundColor: recommendationStyle.bgColor,
              border: `1px solid ${recommendationStyle.color}20`,
            }}
          >
            <CardContent>
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 2 }}>
                <RecommendationIcon 
                  sx={{ 
                    color: recommendationStyle.color,
                    fontSize: 20,
                  }} 
                />
                <Typography 
                  variant="h6" 
                  sx={{ 
                    fontWeight: 600,
                    color: recommendationStyle.color,
                  }}
                >
                  Recommended Action
                </Typography>
              </Box>
              
              <Typography 
                variant="body1" 
                sx={{ 
                  color: modernTokens.colors.neutral[800],
                  lineHeight: 1.6,
                  mb: 3,
                }}
              >
                {failure.diagnostic_summary.recommended_action}
              </Typography>

              <Stack direction="row" spacing={1} flexWrap="wrap">
                {failure.diagnostic_summary.can_retry && (
                  <Chip
                    icon={<RefreshIcon />}
                    label="Can retry"
                    size="small"
                    sx={{
                      backgroundColor: modernTokens.colors.success[100],
                      color: modernTokens.colors.success[700],
                    }}
                  />
                )}
                {failure.diagnostic_summary.user_action_required && (
                  <Chip
                    icon={<WarningIcon />}
                    label="Action required"
                    size="small"
                    sx={{
                      backgroundColor: modernTokens.colors.warning[100],
                      color: modernTokens.colors.warning[700],
                    }}
                  />
                )}
              </Stack>
            </CardContent>
          </Card>
        </Grid>
      </Grid>

      {/* Diagnostic Information (Collapsible) */}
      <Card variant="outlined" sx={{ mb: 3 }}>
        <CardContent>
          <Button
            fullWidth
            onClick={() => setShowDiagnostics(!showDiagnostics)}
            endIcon={showDiagnostics ? <ExpandLessIcon /> : <ExpandMoreIcon />}
            sx={{
              justifyContent: 'space-between',
              textAlign: 'left',
              p: 0,
              textTransform: 'none',
              color: modernTokens.colors.neutral[700],
            }}
          >
            <Typography variant="h6" sx={{ fontWeight: 600 }}>
              Diagnostic Details
            </Typography>
          </Button>

          <Collapse in={showDiagnostics}>
            <Box sx={{ mt: 2 }}>
              <Grid container spacing={2}>
                {failure.diagnostic_summary.path_length && (
                  <Grid item xs={6} md={3}>
                    <Paper 
                      variant="outlined" 
                      sx={{ 
                        p: 2, 
                        textAlign: 'center',
                        backgroundColor: modernTokens.colors.neutral[50],
                      }}
                    >
                      <FolderIcon sx={{ color: modernTokens.colors.primary[500], mb: 1 }} />
                      <Typography variant="h6" sx={{ fontWeight: 600 }}>
                        {failure.diagnostic_summary.path_length}
                      </Typography>
                      <Typography variant="caption" color="text.secondary">
                        Path Length (chars)
                      </Typography>
                    </Paper>
                  </Grid>
                )}

                {failure.diagnostic_summary.directory_depth && (
                  <Grid item xs={6} md={3}>
                    <Paper 
                      variant="outlined" 
                      sx={{ 
                        p: 2, 
                        textAlign: 'center',
                        backgroundColor: modernTokens.colors.neutral[50],
                      }}
                    >
                      <FolderIcon sx={{ color: modernTokens.colors.info[500], mb: 1 }} />
                      <Typography variant="h6" sx={{ fontWeight: 600 }}>
                        {failure.diagnostic_summary.directory_depth}
                      </Typography>
                      <Typography variant="caption" color="text.secondary">
                        Directory Depth
                      </Typography>
                    </Paper>
                  </Grid>
                )}

                {failure.diagnostic_summary.estimated_item_count && (
                  <Grid item xs={6} md={3}>
                    <Paper 
                      variant="outlined" 
                      sx={{ 
                        p: 2, 
                        textAlign: 'center',
                        backgroundColor: modernTokens.colors.neutral[50],
                      }}
                    >
                      <CloudOffIcon sx={{ color: modernTokens.colors.warning[500], mb: 1 }} />
                      <Typography variant="h6" sx={{ fontWeight: 600 }}>
                        {failure.diagnostic_summary.estimated_item_count.toLocaleString()}
                      </Typography>
                      <Typography variant="caption" color="text.secondary">
                        Estimated Items
                      </Typography>
                    </Paper>
                  </Grid>
                )}

                {failure.diagnostic_summary.response_time_ms && (
                  <Grid item xs={6} md={3}>
                    <Paper 
                      variant="outlined" 
                      sx={{ 
                        p: 2, 
                        textAlign: 'center',
                        backgroundColor: modernTokens.colors.neutral[50],
                      }}
                    >
                      <TimerIcon sx={{ color: modernTokens.colors.error[500], mb: 1 }} />
                      <Typography variant="h6" sx={{ fontWeight: 600 }}>
                        {formatDuration(failure.diagnostic_summary.response_time_ms)}
                      </Typography>
                      <Typography variant="caption" color="text.secondary">
                        Response Time
                      </Typography>
                    </Paper>
                  </Grid>
                )}

                {failure.diagnostic_summary.response_size_mb && (
                  <Grid item xs={6} md={3}>
                    <Paper 
                      variant="outlined" 
                      sx={{ 
                        p: 2, 
                        textAlign: 'center',
                        backgroundColor: modernTokens.colors.neutral[50],
                      }}
                    >
                      <SpeedIcon sx={{ color: modernTokens.colors.secondary[500], mb: 1 }} />
                      <Typography variant="h6" sx={{ fontWeight: 600 }}>
                        {failure.diagnostic_summary.response_size_mb.toFixed(1)} MB
                      </Typography>
                      <Typography variant="caption" color="text.secondary">
                        Response Size
                      </Typography>
                    </Paper>
                  </Grid>
                )}

                {failure.diagnostic_summary.server_type && (
                  <Grid item xs={12} md={6}>
                    <Paper 
                      variant="outlined" 
                      sx={{ 
                        p: 2,
                        backgroundColor: modernTokens.colors.neutral[50],
                      }}
                    >
                      <Typography variant="body2" color="text.secondary" sx={{ mb: 0.5 }}>
                        Server Type
                      </Typography>
                      <Typography variant="body1" sx={{ fontFamily: 'monospace' }}>
                        {failure.diagnostic_summary.server_type}
                      </Typography>
                    </Paper>
                  </Grid>
                )}
              </Grid>
            </Box>
          </Collapse>
        </CardContent>
      </Card>

      {/* User Notes */}
      {failure.user_notes && (
        <Alert 
          severity="info" 
          sx={{ 
            mb: 3,
            borderRadius: 2,
          }}
        >
          <Typography variant="body2">
            <strong>User Notes:</strong> {failure.user_notes}
          </Typography>
        </Alert>
      )}

      {/* Action Buttons */}
      {!failure.resolved && !failure.user_excluded && (
        <Stack direction="row" spacing={2} justifyContent="flex-end">
          <Button
            variant="outlined"
            startIcon={<BlockIcon />}
            onClick={() => setExcludeDialogOpen(true)}
            disabled={isExcluding}
            color="warning"
          >
            Exclude Directory
          </Button>

          {failure.diagnostic_summary.can_retry && (
            <Button
              variant="contained"
              startIcon={<RefreshIcon />}
              onClick={() => setRetryDialogOpen(true)}
              disabled={isRetrying}
            >
              Retry Scan
            </Button>
          )}
        </Stack>
      )}

      {/* Confirmation Dialogs */}
      <ConfirmationDialog
        open={retryDialogOpen}
        onClose={() => setRetryDialogOpen(false)}
        onConfirm={(notes) => {
          onRetry(failure, notes);
          setRetryDialogOpen(false);
        }}
        title="Retry WebDAV Scan"
        description={`This will attempt to scan "${failure.directory_path}" again. The failure will be reset and moved to the retry queue.`}
        confirmText="Retry Now"
        confirmColor="primary"
        isLoading={isRetrying}
      />

      <ConfirmationDialog
        open={excludeDialogOpen}
        onClose={() => setExcludeDialogOpen(false)}
        onConfirm={(notes, permanent) => {
          onExclude(failure, notes, permanent);
          setExcludeDialogOpen(false);
        }}
        title="Exclude Directory from Scanning"
        description={`This will prevent "${failure.directory_path}" from being scanned in future synchronizations.`}
        confirmText="Exclude Directory"
        confirmColor="warning"
        showPermanentOption
        isLoading={isExcluding}
      />
    </Box>
  );
};

export default FailureDetailsPanel;