import React, { useState, useEffect } from 'react';
import {
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  Button,
  FormControl,
  FormLabel,
  RadioGroup,
  FormControlLabel,
  Radio,
  TextField,
  Chip,
  Box,
  Typography,
  Alert,
  LinearProgress,
  Accordion,
  AccordionSummary,
  AccordionDetails,
  Checkbox,
  Slider,
  Stack,
  Card,
  CardContent,
  Divider,
} from '@mui/material';
import {
  ExpandMore as ExpandMoreIcon,
  Schedule as ScheduleIcon,
  Assessment as AssessmentIcon,
  Refresh as RefreshIcon,
} from '@mui/icons-material';
import { documentService, BulkOcrRetryRequest, OcrRetryFilter, BulkOcrRetryResponse, ErrorHelper, ErrorCodes } from '../services/api';

interface BulkRetryModalProps {
  open: boolean;
  onClose: () => void;
  onSuccess: (result: BulkOcrRetryResponse) => void;
  selectedDocumentIds?: string[];
}

const COMMON_MIME_TYPES = [
  { value: 'application/pdf', label: 'PDF' },
  { value: 'image/png', label: 'PNG' },
  { value: 'image/jpeg', label: 'JPEG' },
  { value: 'image/tiff', label: 'TIFF' },
  { value: 'text/plain', label: 'Text' },
];

const COMMON_FAILURE_REASONS = [
  { value: 'pdf_font_encoding', label: 'Font Encoding Issues' },
  { value: 'ocr_timeout', label: 'Processing Timeout' },
  { value: 'pdf_corruption', label: 'File Corruption' },
  { value: 'low_ocr_confidence', label: 'Low Confidence' },
  { value: 'no_extractable_text', label: 'No Text Found' },
  { value: 'ocr_memory_limit', label: 'Memory Limit' },
];

const FILE_SIZE_PRESETS = [
  { label: '< 1MB', value: 1024 * 1024 },
  { label: '< 5MB', value: 5 * 1024 * 1024 },
  { label: '< 10MB', value: 10 * 1024 * 1024 },
  { label: '< 50MB', value: 50 * 1024 * 1024 },
];

export const BulkRetryModal: React.FC<BulkRetryModalProps> = ({
  open,
  onClose,
  onSuccess,
  selectedDocumentIds = [],
}) => {
  const [mode, setMode] = useState<'all' | 'specific' | 'filter'>('all');
  const [filter, setFilter] = useState<OcrRetryFilter>({});
  const [priorityOverride, setPriorityOverride] = useState<number>(10);
  const [usePriorityOverride, setUsePriorityOverride] = useState(false);
  const [previewOnly, setPreviewOnly] = useState(true);
  const [loading, setLoading] = useState(false);
  const [previewResult, setPreviewResult] = useState<BulkOcrRetryResponse | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Initialize mode based on selected documents
  useEffect(() => {
    if (selectedDocumentIds.length > 0) {
      setMode('specific');
    }
  }, [selectedDocumentIds]);

  const handleModeChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    setMode(event.target.value as 'all' | 'specific' | 'filter');
    setPreviewResult(null);
    setError(null);
  };

  const handleFilterChange = (key: keyof OcrRetryFilter, value: any) => {
    setFilter(prev => ({
      ...prev,
      [key]: value,
    }));
    setPreviewResult(null);
  };

  const handleMimeTypeToggle = (mimeType: string) => {
    const current = filter.mime_types || [];
    if (current.includes(mimeType)) {
      handleFilterChange('mime_types', current.filter(t => t !== mimeType));
    } else {
      handleFilterChange('mime_types', [...current, mimeType]);
    }
  };

  const handleFailureReasonToggle = (reason: string) => {
    const current = filter.failure_reasons || [];
    if (current.includes(reason)) {
      handleFilterChange('failure_reasons', current.filter(r => r !== reason));
    } else {
      handleFilterChange('failure_reasons', [...current, reason]);
    }
  };

  const buildRequest = (preview: boolean): BulkOcrRetryRequest => {
    const request: BulkOcrRetryRequest = {
      mode,
      preview_only: preview,
    };

    if (mode === 'specific') {
      request.document_ids = selectedDocumentIds;
    } else if (mode === 'filter') {
      request.filter = filter;
    }

    if (usePriorityOverride) {
      request.priority_override = priorityOverride;
    }

    return request;
  };

  const handlePreview = async () => {
    setLoading(true);
    setError(null);
    try {
      const request = buildRequest(true);
      const response = await documentService.bulkRetryOcr(request);
      setPreviewResult(response.data);
    } catch (err: any) {
      const errorInfo = ErrorHelper.formatErrorForDisplay(err, true);
      let errorMessage = 'Failed to preview retry operation';
      
      // Handle specific bulk retry preview errors
      if (ErrorHelper.isErrorCode(err, ErrorCodes.USER_SESSION_EXPIRED) || 
          ErrorHelper.isErrorCode(err, ErrorCodes.USER_TOKEN_EXPIRED)) {
        errorMessage = 'Your session has expired. Please refresh the page and log in again.';
      } else if (ErrorHelper.isErrorCode(err, ErrorCodes.USER_PERMISSION_DENIED)) {
        errorMessage = 'You do not have permission to preview retry operations.';
      } else if (ErrorHelper.isErrorCode(err, ErrorCodes.DOCUMENT_NOT_FOUND)) {
        errorMessage = 'No documents found matching the specified criteria.';
      } else if (errorInfo.category === 'server') {
        errorMessage = 'Server error. Please try again later.';
      } else if (errorInfo.category === 'network') {
        errorMessage = 'Network error. Please check your connection and try again.';
      } else {
        errorMessage = errorInfo.message || 'Failed to preview retry operation';
      }
      
      setError(errorMessage);
      setPreviewResult(null);
    } finally {
      setLoading(false);
    }
  };

  const handleExecute = async () => {
    setLoading(true);
    setError(null);
    try {
      const request = buildRequest(false);
      const response = await documentService.bulkRetryOcr(request);
      onSuccess(response.data);
      onClose();
    } catch (err: any) {
      const errorInfo = ErrorHelper.formatErrorForDisplay(err, true);
      let errorMessage = 'Failed to execute retry operation';
      
      // Handle specific bulk retry execution errors
      if (ErrorHelper.isErrorCode(err, ErrorCodes.USER_SESSION_EXPIRED) || 
          ErrorHelper.isErrorCode(err, ErrorCodes.USER_TOKEN_EXPIRED)) {
        errorMessage = 'Your session has expired. Please refresh the page and log in again.';
      } else if (ErrorHelper.isErrorCode(err, ErrorCodes.USER_PERMISSION_DENIED)) {
        errorMessage = 'You do not have permission to execute retry operations.';
      } else if (ErrorHelper.isErrorCode(err, ErrorCodes.DOCUMENT_NOT_FOUND)) {
        errorMessage = 'No documents found matching the specified criteria.';
      } else if (ErrorHelper.isErrorCode(err, ErrorCodes.DOCUMENT_PROCESSING_FAILED)) {
        errorMessage = 'Some documents cannot be retried due to processing issues.';
      } else if (errorInfo.category === 'server') {
        errorMessage = 'Server error. Please try again later or contact support.';
      } else if (errorInfo.category === 'network') {
        errorMessage = 'Network error. Please check your connection and try again.';
      } else {
        errorMessage = errorInfo.message || 'Failed to execute retry operation';
      }
      
      setError(errorMessage);
    } finally {
      setLoading(false);
    }
  };

  const formatFileSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
  };

  const formatDuration = (minutes: number) => {
    if (minutes < 1) return `${Math.round(minutes * 60)} seconds`;
    if (minutes < 60) return `${Math.round(minutes)} minutes`;
    return `${Math.round(minutes / 60)} hours`;
  };

  return (
    <Dialog open={open} onClose={onClose} maxWidth="md" fullWidth>
      <DialogTitle>
        <Box display="flex" alignItems="center" gap={1}>
          <RefreshIcon />
          Bulk OCR Retry
        </Box>
      </DialogTitle>

      <DialogContent>
        <Stack spacing={3}>
          {error && (
            <Alert severity="error">{error}</Alert>
          )}

          {/* Selection Mode */}
          <FormControl component="fieldset">
            <FormLabel component="legend">Retry Mode</FormLabel>
            <RadioGroup value={mode} onChange={handleModeChange}>
              <FormControlLabel
                value="all"
                control={<Radio />}
                label="Retry all failed OCR documents"
              />
              <FormControlLabel
                value="specific"
                control={<Radio />}
                label={`Retry selected documents (${selectedDocumentIds.length} selected)`}
                disabled={selectedDocumentIds.length === 0}
              />
              <FormControlLabel
                value="filter"
                control={<Radio />}
                label="Retry documents matching criteria"
              />
            </RadioGroup>
          </FormControl>

          {/* Filter Options */}
          {mode === 'filter' && (
            <Accordion>
              <AccordionSummary expandIcon={<ExpandMoreIcon />}>
                <Typography variant="h6">Filter Criteria</Typography>
              </AccordionSummary>
              <AccordionDetails>
                <Stack spacing={3}>
                  {/* MIME Types */}
                  <Box>
                    <Typography variant="subtitle1" gutterBottom>
                      File Types
                    </Typography>
                    <Box display="flex" flexWrap="wrap" gap={1}>
                      {COMMON_MIME_TYPES.map(({ value, label }) => (
                        <Chip
                          key={value}
                          label={label}
                          variant={filter.mime_types?.includes(value) ? 'filled' : 'outlined'}
                          onClick={() => handleMimeTypeToggle(value)}
                          clickable
                        />
                      ))}
                    </Box>
                  </Box>

                  {/* Failure Reasons */}
                  <Box>
                    <Typography variant="subtitle1" gutterBottom>
                      Failure Reasons
                    </Typography>
                    <Box display="flex" flexWrap="wrap" gap={1}>
                      {COMMON_FAILURE_REASONS.map(({ value, label }) => (
                        <Chip
                          key={value}
                          label={label}
                          variant={filter.failure_reasons?.includes(value) ? 'filled' : 'outlined'}
                          onClick={() => handleFailureReasonToggle(value)}
                          clickable
                          color="secondary"
                        />
                      ))}
                    </Box>
                  </Box>

                  {/* File Size */}
                  <Box>
                    <Typography variant="subtitle1" gutterBottom>
                      Maximum File Size
                    </Typography>
                    <Box display="flex" flexWrap="wrap" gap={1} mb={2}>
                      {FILE_SIZE_PRESETS.map(({ label, value }) => (
                        <Chip
                          key={value}
                          label={label}
                          variant={filter.max_file_size === value ? 'filled' : 'outlined'}
                          onClick={() => handleFilterChange('max_file_size', 
                            filter.max_file_size === value ? undefined : value)}
                          clickable
                          color="primary"
                        />
                      ))}
                    </Box>
                    {filter.max_file_size && (
                      <Typography variant="body2" color="text.secondary">
                        Max file size: {formatFileSize(filter.max_file_size)}
                      </Typography>
                    )}
                  </Box>

                  {/* Limit */}
                  <TextField
                    label="Maximum Documents to Retry"
                    type="number"
                    value={filter.limit || ''}
                    onChange={(e) => handleFilterChange('limit', 
                      e.target.value ? parseInt(e.target.value) : undefined)}
                    InputProps={{
                      inputProps: { min: 1, max: 1000 }
                    }}
                    helperText="Leave empty for no limit"
                  />
                </Stack>
              </AccordionDetails>
            </Accordion>
          )}

          {/* Priority Override */}
          <Accordion>
            <AccordionSummary expandIcon={<ExpandMoreIcon />}>
              <Typography variant="h6">Advanced Options</Typography>
            </AccordionSummary>
            <AccordionDetails>
              <Stack spacing={2}>
                <FormControlLabel
                  control={
                    <Checkbox
                      checked={usePriorityOverride}
                      onChange={(e) => setUsePriorityOverride(e.target.checked)}
                    />
                  }
                  label="Override processing priority"
                />
                {usePriorityOverride && (
                  <Box>
                    <Typography gutterBottom>
                      Priority: {priorityOverride} (Higher = More Urgent)
                    </Typography>
                    <Slider
                      value={priorityOverride}
                      onChange={(_, value) => setPriorityOverride(value as number)}
                      min={1}
                      max={20}
                      marks={[
                        { value: 1, label: 'Low' },
                        { value: 10, label: 'Normal' },
                        { value: 20, label: 'High' },
                      ]}
                      valueLabelDisplay="auto"
                    />
                  </Box>
                )}
              </Stack>
            </AccordionDetails>
          </Accordion>

          {/* Preview Results */}
          {previewResult && (
            <Card>
              <CardContent>
                <Typography variant="h6" gutterBottom>
                  <AssessmentIcon sx={{ mr: 1, verticalAlign: 'middle' }} />
                  Preview Results
                </Typography>
                <Stack spacing={2}>
                  <Box display="flex" justifyContent="space-between">
                    <Typography>Documents matched:</Typography>
                    <Typography fontWeight="bold">{previewResult.matched_count}</Typography>
                  </Box>
                  <Box display="flex" justifyContent="space-between">
                    <Typography>Estimated processing time:</Typography>
                    <Typography fontWeight="bold">
                      <ScheduleIcon sx={{ mr: 0.5, verticalAlign: 'middle', fontSize: 'small' }} />
                      {formatDuration(previewResult.estimated_total_time_minutes)}
                    </Typography>
                  </Box>
                  {previewResult.documents && previewResult.documents.length > 0 && (
                    <Box>
                      <Typography variant="subtitle2" gutterBottom>
                        Sample Documents:
                      </Typography>
                      <Box maxHeight={200} overflow="auto">
                        {(previewResult.documents || []).slice(0, 10).map((doc) => (
                          <Box key={doc.id} py={0.5}>
                            <Typography variant="body2">
                              {doc.filename} ({formatFileSize(doc.file_size)})
                              {doc.ocr_failure_reason && (
                                <Chip 
                                  size="small" 
                                  label={doc.ocr_failure_reason} 
                                  sx={{ ml: 1, fontSize: '0.7rem' }}
                                />
                              )}
                            </Typography>
                          </Box>
                        ))}
                        {previewResult.documents && previewResult.documents.length > 10 && (
                          <Typography variant="body2" color="text.secondary" mt={1}>
                            ... and {previewResult.documents.length - 10} more documents
                          </Typography>
                        )}
                      </Box>
                    </Box>
                  )}
                </Stack>
              </CardContent>
            </Card>
          )}

          {loading && <LinearProgress />}
        </Stack>
      </DialogContent>

      <DialogActions>
        <Button onClick={onClose} disabled={loading}>
          Cancel
        </Button>
        <Button 
          onClick={handlePreview} 
          disabled={loading}
          variant="outlined"
        >
          Preview
        </Button>
        <Button
          onClick={handleExecute}
          disabled={loading || !previewResult || previewResult.matched_count === 0}
          variant="contained"
          color="primary"
        >
          {loading ? 'Processing...' : `Retry ${previewResult?.matched_count || 0} Documents`}
        </Button>
      </DialogActions>
    </Dialog>
  );
};