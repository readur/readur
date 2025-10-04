import React, { useState, useCallback, useEffect } from 'react';
import {
  Box,
  Card,
  CardContent,
  Typography,
  TextField,
  Button,
  Paper,
  Stepper,
  Step,
  StepLabel,
  StepContent,
  Alert,
  Chip,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Accordion,
  AccordionSummary,
  AccordionDetails,
  CircularProgress,
  Container,
  Tabs,
  Tab,
  LinearProgress,
  Divider,
} from '@mui/material';
import Grid from '@mui/material/GridLegacy';
import {
  ExpandMore as ExpandMoreIcon,
  BugReport as BugReportIcon,
  CheckCircle as CheckCircleIcon,
  Error as ErrorIcon,
  Warning as WarningIcon,
  Pending as PendingIcon,
  PlayArrow as PlayArrowIcon,
  CloudUpload as UploadIcon,
  Search as SearchIcon,
  Refresh as RefreshIcon,
  Visibility as PreviewIcon,
} from '@mui/icons-material';
import { useTranslation } from 'react-i18next';
import { api } from '../services/api';

interface DebugStep {
  step: number;
  name: string;
  status: string;
  details: any;
  success: boolean;
  error?: string;
}

interface DebugInfo {
  document_id: string;
  filename: string;
  overall_status: string;
  pipeline_steps: DebugStep[];
  failed_document_info?: any;
  user_settings: any;
  debug_timestamp: string;
  detailed_processing_logs?: any[];
  file_analysis?: {
    file_size: number;
    mime_type: string;
    is_text_file: boolean;
    is_image_file: boolean;
    character_count: number;
    word_count: number;
    estimated_processing_time: number;
    complexity_score: number;
    [key: string]: any;
  };
}

const DebugPage: React.FC = () => {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<number>(0);
  const [documentId, setDocumentId] = useState<string>('');
  const [debugInfo, setDebugInfo] = useState<DebugInfo | null>(null);
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<string>('');

  // Upload functionality
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [uploading, setUploading] = useState<boolean>(false);
  const [uploadProgress, setUploadProgress] = useState<number>(0);
  const [uploadedDocumentId, setUploadedDocumentId] = useState<string>('');
  const [monitoringInterval, setMonitoringInterval] = useState<NodeJS.Timeout | null>(null);
  const [processingStatus, setProcessingStatus] = useState<string>('');

  // Auto-switch to debug results tab when debugInfo is available
  useEffect(() => {
    if (debugInfo && activeTab !== 2) {
      setActiveTab(2);
    }
  }, [debugInfo]);

  // Reset activeTab when debugInfo is cleared
  useEffect(() => {
    if (!debugInfo && activeTab === 2) {
      setActiveTab(0);
    }
  }, [debugInfo, activeTab]);

  const getStepIcon = (status: string, success: boolean) => {
    if (status === 'processing') return <CircularProgress size={20} />;
    if (success || status === 'completed' || status === 'passed') return <CheckCircleIcon color="success" />;
    if (status === 'failed' || status === 'error') return <ErrorIcon color="error" />;
    if (status === 'pending' || status === 'not_reached') return <PendingIcon color="disabled" />;
    if (status === 'not_queued' || status === 'ocr_disabled') return <WarningIcon color="warning" />;
    return <PlayArrowIcon color="primary" />;
  };

  const getStatusColor = (status: string, success: boolean): "default" | "primary" | "secondary" | "error" | "info" | "success" | "warning" => {
    if (status === 'processing') return 'info';
    if (success || status === 'completed' || status === 'passed') return 'success';
    if (status === 'failed' || status === 'error') return 'error';
    if (status === 'pending' || status === 'not_reached') return 'default';
    if (status === 'not_queued' || status === 'ocr_disabled') return 'warning';
    return 'primary';
  };

  const fetchDebugInfo = useCallback(async (docId?: string, retryCount = 0) => {
    const targetDocId = docId || documentId;
    if (!targetDocId.trim()) {
      setError(t('debug.errors.enterDocumentId'));
      return;
    }

    setLoading(true);
    if (retryCount === 0) {
      setError(''); // Only clear error on first attempt
    }

    try {
      const response = await api.get(`/documents/${targetDocId}/debug`);
      setDebugInfo(response.data);
      setError(''); // Clear any previous errors
    } catch (err: any) {
      console.error('Debug fetch error:', err);

      // If it's a 404 and we haven't retried much, try again after a short delay
      if (err.response?.status === 404 && retryCount < 3) {
        console.log(`Document not found, retrying in ${(retryCount + 1) * 1000}ms... (attempt ${retryCount + 1})`);
        setTimeout(() => {
          fetchDebugInfo(docId, retryCount + 1);
        }, (retryCount + 1) * 1000);
        return;
      }

      const errorMessage = err.response?.status === 404
        ? t('debug.errors.documentNotFound', { documentId: targetDocId })
        : err.response?.data?.message || t('debug.errors.fetchFailed', { message: err.message });
      setError(errorMessage);
      setDebugInfo(null);
    } finally {
      if (retryCount === 0) {
        setLoading(false);
      }
    }
  }, [documentId, t]);

  const handleFileSelect = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (file) {
      setSelectedFile(file);
      setError('');
    }
  };

  const uploadDocument = useCallback(async () => {
    if (!selectedFile) {
      setError(t('debug.upload.selectFile'));
      return;
    }

    setUploading(true);
    setUploadProgress(0);
    setError('');
    setProcessingStatus(t('debug.upload.uploading'));

    try {
      const formData = new FormData();
      formData.append('file', selectedFile);

      const response = await api.post('/documents', formData, {
        headers: {
          'Content-Type': 'multipart/form-data',
        },
        onUploadProgress: (progressEvent) => {
          const progress = progressEvent.total
            ? Math.round((progressEvent.loaded * 100) / progressEvent.total)
            : 0;
          setUploadProgress(progress);
        },
      });

      const uploadedDoc = response.data;
      setUploadedDocumentId(uploadedDoc.id);
      setDocumentId(uploadedDoc.id);
      setProcessingStatus(t('debug.upload.uploadedStartingOcr'));

      // Start monitoring the processing
      startProcessingMonitor(uploadedDoc.id);
    } catch (err: any) {
      setError(err.response?.data?.message || t('debug.upload.uploadFailed'));
      setProcessingStatus(t('debug.upload.uploadFailedStatus'));
    } finally {
      setUploading(false);
      setUploadProgress(0);
    }
  }, [selectedFile, t]);

  const startProcessingMonitor = useCallback((docId: string) => {
    // Clear any existing interval
    if (monitoringInterval) {
      clearInterval(monitoringInterval);
    }

    const interval = setInterval(async () => {
      try {
        const response = await api.get(`/documents/${docId}`);
        const doc = response.data;

        if (doc.ocr_status === 'completed' || doc.ocr_status === 'failed') {
          setProcessingStatus(t('debug.monitoring.processingComplete', { status: doc.ocr_status }));
          clearInterval(interval);
          setMonitoringInterval(null);

          // Auto-fetch debug info when processing is complete OR failed (but don't switch tabs)
          setTimeout(() => {
            fetchDebugInfo(docId);
            // Don't auto-switch tabs - let user decide when to view debug info
          }, 2000); // Give it a bit more time to ensure document is saved
        } else if (doc.ocr_status === 'processing') {
          setProcessingStatus(t('debug.monitoring.ocrInProgress'));
        } else if (doc.ocr_status === 'pending') {
          setProcessingStatus(t('debug.monitoring.queuedForOcr'));
        } else {
          setProcessingStatus(t('debug.monitoring.checkingStatus'));
        }
      } catch (err) {
        console.error('Error monitoring processing:', err);
      }
    }, 2000); // Check every 2 seconds

    setMonitoringInterval(interval);

    // Auto-clear monitoring after 5 minutes
    setTimeout(() => {
      clearInterval(interval);
      setMonitoringInterval(null);
      setProcessingStatus(t('debug.monitoring.monitoringTimeout'));
    }, 300000);
  }, [monitoringInterval, fetchDebugInfo, t]);

  // Cleanup interval on unmount
  useEffect(() => {
    return () => {
      if (monitoringInterval) {
        clearInterval(monitoringInterval);
      }
    };
  }, [monitoringInterval]);

  const renderStepDetails = (step: DebugStep) => {
    const details = step.details;
    
    return (
      <Box sx={{ mt: 2 }}>
        {step.error && (
          <Alert severity="error" sx={{ mb: 2 }}>
            {step.error}
          </Alert>
        )}
        
        {step.step === 1 && ( // File Upload & Ingestion
          <Box>
            <Grid container spacing={2}>
              <Grid item xs={12} md={6}>
                <Paper sx={{ p: 2 }}>
                  <Typography variant="h6" gutterBottom>{t('debug.steps.fileInformation.title')}</Typography>
                  <Typography><strong>{t('debug.steps.fileInformation.filename')}</strong> {details.filename}</Typography>
                  <Typography><strong>{t('debug.steps.fileInformation.original')}</strong> {details.original_filename}</Typography>
                  <Typography><strong>{t('debug.steps.fileInformation.size')}</strong> {(details.file_size / 1024 / 1024).toFixed(2)} MB</Typography>
                  <Typography><strong>{t('debug.steps.fileInformation.mimeType')}</strong> {details.mime_type}</Typography>
                  <Typography><strong>{t('debug.steps.fileInformation.fileExists')}</strong> <Chip
                    label={details.file_exists ? t('debug.steps.fileInformation.yes') : t('debug.steps.fileInformation.no')}
                    color={details.file_exists ? 'success' : 'error'}
                    size="small"
                  /></Typography>
                </Paper>
              </Grid>
              <Grid item xs={12} md={6}>
                <Paper sx={{ p: 2 }}>
                  <Typography variant="h6" gutterBottom>{t('debug.steps.fileMetadata.title')}</Typography>
                  {details.file_metadata ? (
                    <>
                      <Typography><strong>{t('debug.steps.fileMetadata.actualSize')}</strong> {(details.file_metadata.size / 1024 / 1024).toFixed(2)} MB</Typography>
                      <Typography><strong>{t('debug.steps.fileMetadata.isFile')}</strong> {details.file_metadata.is_file ? t('debug.steps.fileInformation.yes') : t('debug.steps.fileInformation.no')}</Typography>
                      <Typography><strong>{t('debug.steps.fileMetadata.modified')}</strong> {details.file_metadata.modified ? new Date(details.file_metadata.modified.secs_since_epoch * 1000).toLocaleString() : t('debug.steps.fileMetadata.unknown')}</Typography>
                    </>
                  ) : (
                    <Typography color="text.secondary">{t('debug.steps.fileMetadata.notAvailable')}</Typography>
                  )}
                  <Typography><strong>{t('debug.steps.fileMetadata.created')}</strong> {new Date(details.created_at).toLocaleString()}</Typography>
                </Paper>
              </Grid>
            </Grid>
            
            {details.file_analysis && (
              <Box sx={{ mt: 2 }}>
                <Typography variant="h6" gutterBottom>Detailed File Analysis</Typography>
                <Grid container spacing={2}>
                  <Grid item xs={12} md={6}>
                    <Paper sx={{ p: 2 }}>
                      <Typography variant="subtitle1" gutterBottom>Basic Analysis</Typography>
                      <Typography><strong>File Type:</strong> {details.file_analysis.file_type}</Typography>
                      <Typography><strong>Size:</strong> {(details.file_analysis.file_size_bytes / 1024 / 1024).toFixed(2)} MB</Typography>
                      <Typography><strong>Readable:</strong> <Chip 
                        label={details.file_analysis.is_readable ? 'Yes' : 'No'} 
                        color={details.file_analysis.is_readable ? 'success' : 'error'} 
                        size="small" 
                      /></Typography>
                      {details.file_analysis.error_details && (
                        <Alert severity="error" sx={{ mt: 1 }}>
                          <strong>File Error:</strong> {details.file_analysis.error_details}
                        </Alert>
                      )}
                    </Paper>
                  </Grid>
                  <Grid item xs={12} md={6}>
                    {details.file_analysis.pdf_info ? (
                      <Paper sx={{ p: 2 }}>
                        <Typography variant="subtitle1" gutterBottom>PDF Analysis</Typography>
                        <Typography><strong>Valid PDF:</strong> <Chip 
                          label={details.file_analysis.pdf_info.is_valid_pdf ? 'Yes' : 'No'} 
                          color={details.file_analysis.pdf_info.is_valid_pdf ? 'success' : 'error'} 
                          size="small" 
                        /></Typography>
                        <Typography><strong>PDF Version:</strong> {details.file_analysis.pdf_info.pdf_version || 'Unknown'}</Typography>
                        <Typography><strong>Pages:</strong> {details.file_analysis.pdf_info.page_count || 'Unknown'}</Typography>
                        <Typography><strong>Has Text:</strong> <Chip 
                          label={details.file_analysis.pdf_info.has_text_content ? 'Yes' : 'No'} 
                          color={details.file_analysis.pdf_info.has_text_content ? 'success' : 'warning'} 
                          size="small" 
                        /></Typography>
                        <Typography><strong>Has Images:</strong> <Chip 
                          label={details.file_analysis.pdf_info.has_images ? 'Yes' : 'No'} 
                          color={details.file_analysis.pdf_info.has_images ? 'info' : 'default'} 
                          size="small" 
                        /></Typography>
                        <Typography><strong>Encrypted:</strong> <Chip 
                          label={details.file_analysis.pdf_info.is_encrypted ? 'Yes' : 'No'} 
                          color={details.file_analysis.pdf_info.is_encrypted ? 'error' : 'success'} 
                          size="small" 
                        /></Typography>
                        <Typography><strong>Font Count:</strong> {details.file_analysis.pdf_info.font_count}</Typography>
                        <Typography><strong>Text Length:</strong> {details.file_analysis.pdf_info.estimated_text_length} chars</Typography>
                        {details.file_analysis.pdf_info.text_extraction_error && (
                          <Alert severity="error" sx={{ mt: 1 }}>
                            <strong>PDF Text Extraction Error:</strong> {details.file_analysis.pdf_info.text_extraction_error}
                          </Alert>
                        )}
                      </Paper>
                    ) : details.file_analysis.text_preview ? (
                      <Paper sx={{ p: 2 }}>
                        <Typography variant="subtitle1" gutterBottom>Text Preview</Typography>
                        <Typography variant="body2" sx={{ 
                          backgroundColor: 'grey.100', 
                          p: 1, 
                          borderRadius: 1,
                          fontFamily: 'monospace',
                          fontSize: '0.85em'
                        }}>
                          {details.file_analysis.text_preview}
                        </Typography>
                      </Paper>
                    ) : (
                      <Paper sx={{ p: 2 }}>
                        <Typography variant="subtitle1" gutterBottom>File Content</Typography>
                        <Typography color="text.secondary">No preview available for this file type</Typography>
                      </Paper>
                    )}
                  </Grid>
                </Grid>
              </Box>
            )}
          </Box>
        )}

        {step.step === 2 && ( // OCR Queue Enrollment
          <Box>
            <Grid container spacing={2}>
              <Grid item xs={12} md={6}>
                <Paper sx={{ p: 2 }}>
                  <Typography variant="h6" gutterBottom>Queue Status</Typography>
                  <Typography><strong>User OCR Enabled:</strong> <Chip 
                    label={details.user_ocr_enabled ? 'Yes' : 'No'} 
                    color={details.user_ocr_enabled ? 'success' : 'warning'} 
                    size="small" 
                  /></Typography>
                  <Typography sx={{ mt: 1 }}><strong>Queue Entries:</strong> {details.queue_entries_count}</Typography>
                </Paper>
              </Grid>
            </Grid>
            
            {details.queue_history && details.queue_history.length > 0 && (
              <Box sx={{ mt: 2 }}>
                <Typography variant="h6" gutterBottom>Queue History</Typography>
                <TableContainer component={Paper}>
                  <Table size="small">
                    <TableHead>
                      <TableRow>
                        <TableCell>Status</TableCell>
                        <TableCell>Priority</TableCell>
                        <TableCell>Created</TableCell>
                        <TableCell>Started</TableCell>
                        <TableCell>Completed</TableCell>
                        <TableCell>Attempts</TableCell>
                        <TableCell>Worker</TableCell>
                      </TableRow>
                    </TableHead>
                    <TableBody>
                      {(details.queue_history || []).map((entry: any, index: number) => (
                        <TableRow key={index}>
                          <TableCell>
                            <Chip 
                              label={entry.status} 
                              color={entry.status === 'completed' ? 'success' : entry.status === 'failed' ? 'error' : 'default'} 
                              size="small" 
                            />
                          </TableCell>
                          <TableCell>{entry.priority}</TableCell>
                          <TableCell>{new Date(entry.created_at).toLocaleString()}</TableCell>
                          <TableCell>{entry.started_at ? new Date(entry.started_at).toLocaleString() : '-'}</TableCell>
                          <TableCell>{entry.completed_at ? new Date(entry.completed_at).toLocaleString() : '-'}</TableCell>
                          <TableCell>{entry.attempts}</TableCell>
                          <TableCell>{entry.worker_id || '-'}</TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                </TableContainer>
              </Box>
            )}
          </Box>
        )}

        {step.step === 3 && ( // OCR Processing
          <Grid container spacing={2}>
            <Grid item xs={12} md={6}>
              <Paper sx={{ p: 2 }}>
                <Typography variant="h6" gutterBottom>OCR Results</Typography>
                <Typography><strong>Text Length:</strong> {details.ocr_text_length} characters</Typography>
                <Typography><strong>Confidence:</strong> {details.ocr_confidence ? `${details.ocr_confidence.toFixed(1)}%` : 'N/A'}</Typography>
                <Typography><strong>Word Count:</strong> {details.ocr_word_count || 0}</Typography>
                <Typography><strong>Processing Time:</strong> {details.ocr_processing_time_ms ? `${details.ocr_processing_time_ms}ms` : 'N/A'}</Typography>
                <Typography><strong>Completed:</strong> {details.ocr_completed_at ? new Date(details.ocr_completed_at).toLocaleString() : 'Not completed'}</Typography>
              </Paper>
            </Grid>
            <Grid item xs={12} md={6}>
              <Paper sx={{ p: 2 }}>
                <Typography variant="h6" gutterBottom>Processing Details</Typography>
                <Typography><strong>Has Processed Image:</strong> <Chip 
                  label={details.has_processed_image ? 'Yes' : 'No'} 
                  color={details.has_processed_image ? 'success' : 'default'} 
                  size="small" 
                /></Typography>
                {details.processed_image_info && (
                  <>
                    <Typography sx={{ mt: 1 }}><strong>Image Size:</strong> {details.processed_image_info.image_width}x{details.processed_image_info.image_height}</Typography>
                    <Typography><strong>File Size:</strong> {(details.processed_image_info.file_size / 1024).toFixed(1)} KB</Typography>
                    <Typography><strong>Processing Steps:</strong> {details.processed_image_info.processing_steps?.join(', ') || 'None'}</Typography>
                    {details.processed_image_info.processing_parameters && (
                      <Typography><strong>Processing Parameters:</strong> {JSON.stringify(details.processed_image_info.processing_parameters)}</Typography>
                    )}
                  </>
                )}
              </Paper>
            </Grid>
          </Grid>
        )}

        {step.step === 4 && ( // Quality Validation
          <Box>
            <Grid container spacing={2}>
              <Grid item xs={12} md={6}>
                <Paper sx={{ p: 2 }}>
                  <Typography variant="h6" gutterBottom>Quality Thresholds</Typography>
                  <Typography><strong>Min Confidence:</strong> {details.quality_thresholds.min_confidence}%</Typography>
                  <Typography><strong>Brightness:</strong> {details.quality_thresholds.brightness_threshold}</Typography>
                  <Typography><strong>Contrast:</strong> {details.quality_thresholds.contrast_threshold}</Typography>
                  <Typography><strong>Noise:</strong> {details.quality_thresholds.noise_threshold}</Typography>
                  <Typography><strong>Sharpness:</strong> {details.quality_thresholds.sharpness_threshold}</Typography>
                </Paper>
              </Grid>
              <Grid item xs={12} md={6}>
                <Paper sx={{ p: 2 }}>
                  <Typography variant="h6" gutterBottom>Actual Values</Typography>
                  <Typography><strong>Confidence:</strong> {details.actual_values.confidence ? `${details.actual_values.confidence.toFixed(1)}%` : 'N/A'}</Typography>
                  <Typography><strong>Word Count:</strong> {details.actual_values.word_count || 0}</Typography>
                  <Typography><strong>Processed Image Available:</strong> <Chip 
                    label={details.actual_values.processed_image_available ? 'Yes' : 'No'} 
                    color={details.actual_values.processed_image_available ? 'success' : 'default'} 
                    size="small" 
                  /></Typography>
                  {details.actual_values.processing_parameters && (
                    <Typography><strong>Processing Parameters:</strong> {JSON.stringify(details.actual_values.processing_parameters)}</Typography>
                  )}
                </Paper>
              </Grid>
            </Grid>
            
            <Box sx={{ mt: 2 }}>
              <Typography variant="h6" gutterBottom>Quality Checks</Typography>
              <Grid container spacing={1}>
                {Object.entries(details.quality_checks || {}).map(([check, passed]: [string, any]) => (
                  <Grid item key={check}>
                    <Chip 
                      label={check.replace('_check', '').replace('_', ' ')} 
                      color={passed === true ? 'success' : passed === false ? 'error' : 'default'}
                      size="small"
                      icon={passed === true ? <CheckCircleIcon /> : passed === false ? <ErrorIcon /> : <WarningIcon />}
                    />
                  </Grid>
                ))}
              </Grid>
            </Box>
          </Box>
        )}
      </Box>
    );
  };

  const renderUploadTab = () => (
    <Box>
      <Card sx={{ mb: 4 }}>
        <CardContent>
          <Typography variant="h6" gutterBottom>
            {t('debug.upload.title')}
          </Typography>
          <Typography variant="body2" color="text.secondary" sx={{ mb: 3 }}>
            {t('debug.upload.description')}
          </Typography>

          <Box sx={{ mb: 3 }}>
            <input
              accept=".pdf,.png,.jpg,.jpeg,.tiff,.bmp,.txt"
              style={{ display: 'none' }}
              id="debug-file-upload"
              type="file"
              onChange={handleFileSelect}
            />
            <label htmlFor="debug-file-upload">
              <Button
                variant="outlined"
                component="span"
                startIcon={<UploadIcon />}
                disabled={uploading}
                sx={{ mr: 2 }}
              >
                {t('debug.upload.selectFileButton')}
              </Button>
            </label>

            {selectedFile && (
              <Box sx={{ mt: 2 }}>
                <Typography variant="body2">
                  <strong>{t('debug.upload.selected')}</strong> {selectedFile.name} ({(selectedFile.size / 1024 / 1024).toFixed(2)} MB)
                </Typography>
              </Box>
            )}

            {selectedFile && (
              <Button
                variant="contained"
                onClick={uploadDocument}
                disabled={uploading}
                startIcon={uploading ? <CircularProgress size={20} /> : <UploadIcon />}
                sx={{ mt: 2 }}
              >
                {uploading ? t('debug.upload.uploadingButton') : t('debug.upload.uploadDebugButton')}
              </Button>
            )}
          </Box>

          {uploading && uploadProgress > 0 && (
            <Box sx={{ mb: 2 }}>
              <Typography variant="body2" gutterBottom>
                {t('debug.upload.uploadProgress', { percent: uploadProgress })}
              </Typography>
              <LinearProgress variant="determinate" value={uploadProgress} />
            </Box>
          )}
          
          {processingStatus && (
            <Alert 
              severity={processingStatus.includes('failed') ? 'error' : 
                       processingStatus.includes('completed') ? 'success' : 'info'}
              sx={{ mb: 2 }}
            >
              {processingStatus}
              {monitoringInterval && (
                <Box sx={{ mt: 1 }}>
                  <LinearProgress />
                </Box>
              )}
            </Alert>
          )}
          
          {uploadedDocumentId && (
            <Box sx={{ mt: 2 }}>
              <Typography variant="body2">
                <strong>{t('debug.upload.documentId')}</strong> {uploadedDocumentId}
              </Typography>
              <Box sx={{ mt: 2 }}>
                <Button
                  variant="contained"
                  size="small"
                  onClick={() => {
                    fetchDebugInfo(uploadedDocumentId);
                    setActiveTab(2); // Switch to debug results tab
                  }}
                  startIcon={<BugReportIcon />}
                  sx={{ mr: 1 }}
                  color={processingStatus.includes('failed') ? 'error' : 'primary'}
                >
                  {processingStatus.includes('failed') ? t('debug.actions.showDebugDetails') : t('debug.actions.debugAnalysis')}
                </Button>
                <Button
                  variant="outlined"
                  size="small"
                  onClick={() => fetchDebugInfo(uploadedDocumentId)}
                  startIcon={<RefreshIcon />}
                  sx={{ mr: 1 }}
                >
                  {t('debug.actions.refreshStatus')}
                </Button>
                <Button
                  variant="outlined"
                  size="small"
                  onClick={() => window.open(`/api/documents/${uploadedDocumentId}/view`, '_blank')}
                  startIcon={<PreviewIcon />}
                >
                  {t('debug.actions.viewDocument')}
                </Button>
              </Box>
            </Box>
          )}
          
          {selectedFile && selectedFile.type.startsWith('image/') && (
            <Box sx={{ mt: 3 }}>
              <Typography variant="h6" gutterBottom>{t('debug.preview')}</Typography>
              <Box
                component="img"
                src={URL.createObjectURL(selectedFile)}
                alt="Document preview"
                sx={{
                  maxWidth: '100%',
                  maxHeight: '400px',
                  objectFit: 'contain',
                  border: '1px solid',
                  borderColor: 'divider',
                  borderRadius: 1
                }}
              />
            </Box>
          )}
        </CardContent>
      </Card>
    </Box>
  );

  const renderSearchTab = () => (
    <Box>
      <Card sx={{ mb: 4 }}>
        <CardContent>
          <Typography variant="h6" gutterBottom>
            {t('debug.search.title')}
          </Typography>
          <Typography variant="body2" color="text.secondary" sx={{ mb: 3 }}>
            {t('debug.search.description')}
          </Typography>

          <Box sx={{ display: 'flex', gap: 2, alignItems: 'center' }}>
            <TextField
              label={t('debug.search.documentIdLabel')}
              value={documentId}
              onChange={(e) => setDocumentId(e.target.value)}
              placeholder={t('debug.search.documentIdPlaceholder')}
              fullWidth
              size="small"
            />
            <Button
              variant="contained"
              onClick={() => fetchDebugInfo()}
              disabled={loading || !documentId.trim()}
              startIcon={loading ? <CircularProgress size={20} /> : <SearchIcon />}
            >
              {t('debug.search.debugButton')}
            </Button>
          </Box>

          {error && (
            <Alert severity="error" sx={{ mt: 2 }}>
              {error}
            </Alert>
          )}
        </CardContent>
      </Card>
    </Box>
  );

  return (
    <Container maxWidth="xl">
      <Box sx={{ mb: 4 }}>
        <Typography variant="h4" component="h1" gutterBottom>
          <BugReportIcon sx={{ mr: 1, verticalAlign: 'middle' }} />
          {t('debug.title')}
        </Typography>
        <Typography variant="body1" color="text.secondary">
          {t('debug.subtitle')}
        </Typography>
      </Box>

      <Card sx={{ mb: 4 }}>
        <Box sx={{ borderBottom: 1, borderColor: 'divider' }}>
          <Tabs value={activeTab} onChange={(_, newValue) => setActiveTab(newValue)}>
            <Tab
              label={t('debug.tabs.uploadAndDebug')}
              icon={<UploadIcon />}
              iconPosition="start"
            />
            <Tab
              label={t('debug.tabs.searchExisting')}
              icon={<SearchIcon />}
              iconPosition="start"
            />
            {debugInfo && (
              <Tab
                label={t('debug.tabs.debugResults')}
                icon={<PreviewIcon />}
                iconPosition="start"
              />
            )}
          </Tabs>
        </Box>

        <CardContent>
          {activeTab === 0 && renderUploadTab()}
          {activeTab === 1 && renderSearchTab()}
        </CardContent>
      </Card>

      {error && (
        <Alert severity="error" sx={{ mb: 4 }}>
          <Typography variant="h6">{t('debug.errors.debugError')}</Typography>
          {error}
        </Alert>
      )}

      {debugInfo && activeTab === 2 && (
        <Box>
          <Card sx={{ mb: 4 }}>
            <CardContent>
              <Typography variant="h6" gutterBottom>
                {t('debug.document.title', { filename: debugInfo.filename })}
              </Typography>
              <Box sx={{ display: 'flex', gap: 2, alignItems: 'center', mb: 2 }}>
                <Chip
                  label={t('debug.document.status', { status: debugInfo.overall_status })}
                  color={getStatusColor(debugInfo.overall_status, debugInfo.overall_status === 'success')}
                />
                <Typography variant="body2" color="text.secondary">
                  {t('debug.document.debugRunAt', { timestamp: new Date(debugInfo.debug_timestamp).toLocaleString() })}
                </Typography>
              </Box>
            </CardContent>
          </Card>

          <Card sx={{ mb: 4 }}>
            <CardContent>
              <Typography variant="h6" gutterBottom>
                {t('debug.pipeline.title')}
              </Typography>
              <Stepper orientation="vertical">
                {(debugInfo.pipeline_steps || []).map((step) => (
                  <Step key={step.step} active={true}>
                    <StepLabel 
                      icon={getStepIcon(step.status, step.success)}
                      StepIconProps={{ 
                        style: { color: step.success ? '#4caf50' : step.status === 'failed' ? '#f44336' : '#ff9800' }
                      }}
                    >
                      <span style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                        <Typography variant="subtitle1" component="span">{step.name}</Typography>
                        <Chip 
                          label={step.status} 
                          size="small" 
                          color={getStatusColor(step.status, step.success)}
                        />
                      </span>
                    </StepLabel>
                    <StepContent>
                      {renderStepDetails(step)}
                    </StepContent>
                  </Step>
                ))}
              </Stepper>
            </CardContent>
          </Card>

          {debugInfo.failed_document_info && (
            <Card sx={{ mb: 4 }}>
              <CardContent>
                <Typography variant="h6" gutterBottom color="error">
                  Failed Document Information
                </Typography>
                <Grid container spacing={2}>
                  <Grid item xs={12} md={6}>
                    <Paper sx={{ p: 2 }}>
                      <Typography variant="subtitle1" gutterBottom>Failure Details</Typography>
                      <Typography><strong>Failure Reason:</strong> {debugInfo.failed_document_info.failure_reason}</Typography>
                      <Typography><strong>Failure Stage:</strong> {debugInfo.failed_document_info.failure_stage}</Typography>
                      <Typography><strong>Retry Count:</strong> {debugInfo.failed_document_info.retry_count || 0}</Typography>
                      <Typography><strong>Created:</strong> {new Date(debugInfo.failed_document_info.created_at).toLocaleString()}</Typography>
                      {debugInfo.failed_document_info.last_retry_at && (
                        <Typography><strong>Last Retry:</strong> {new Date(debugInfo.failed_document_info.last_retry_at).toLocaleString()}</Typography>
                      )}
                    </Paper>
                  </Grid>
                  <Grid item xs={12} md={6}>
                    <Paper sx={{ p: 2 }}>
                      <Typography variant="subtitle1" gutterBottom>Failed OCR Results</Typography>
                      {debugInfo.failed_document_info.failed_ocr_text ? (
                        <>
                          <Typography><strong>OCR Text Length:</strong> {debugInfo.failed_document_info.failed_ocr_text.length} chars</Typography>
                          <Typography><strong>OCR Confidence:</strong> {debugInfo.failed_document_info.failed_ocr_confidence?.toFixed(1)}%</Typography>
                          <Typography><strong>Word Count:</strong> {debugInfo.failed_document_info.failed_ocr_word_count || 0}</Typography>
                          <Typography><strong>Processing Time:</strong> {debugInfo.failed_document_info.failed_ocr_processing_time_ms || 0}ms</Typography>
                        </>
                      ) : (
                        <Typography color="text.secondary">No OCR results available</Typography>
                      )}
                    </Paper>
                  </Grid>
                  {debugInfo.failed_document_info.error_message && (
                    <Grid item xs={12}>
                      <Alert severity="error">
                        <strong>Error Message:</strong> {debugInfo.failed_document_info.error_message}
                      </Alert>
                    </Grid>
                  )}
                  {debugInfo.failed_document_info.content_preview && (
                    <Grid item xs={12}>
                      <Paper sx={{ p: 2 }}>
                        <Typography variant="subtitle1" gutterBottom>Content Preview</Typography>
                        <Typography variant="body2" sx={{ 
                          backgroundColor: 'grey.100', 
                          p: 1, 
                          borderRadius: 1,
                          fontFamily: 'monospace',
                          fontSize: '0.85em'
                        }}>
                          {debugInfo.failed_document_info.content_preview}
                        </Typography>
                      </Paper>
                    </Grid>
                  )}
                </Grid>
              </CardContent>
            </Card>
          )}

          {debugInfo.detailed_processing_logs && debugInfo.detailed_processing_logs.length > 0 && (
            <Card sx={{ mb: 4 }}>
              <CardContent>
                <Typography variant="h6" gutterBottom>
                  Detailed Processing Logs
                </Typography>
                <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
                  Complete history of all OCR processing attempts for this document.
                </Typography>
                <TableContainer component={Paper}>
                  <Table size="small">
                    <TableHead>
                      <TableRow>
                        <TableCell>Attempt</TableCell>
                        <TableCell>Status</TableCell>
                        <TableCell>Priority</TableCell>
                        <TableCell>Created</TableCell>
                        <TableCell>Started</TableCell>
                        <TableCell>Completed</TableCell>
                        <TableCell>Duration</TableCell>
                        <TableCell>Wait Time</TableCell>
                        <TableCell>Attempts</TableCell>
                        <TableCell>Worker</TableCell>
                        <TableCell>Error</TableCell>
                      </TableRow>
                    </TableHead>
                    <TableBody>
                      {(debugInfo.detailed_processing_logs || []).map((log: any, index: number) => (
                        <TableRow key={log.id}>
                          <TableCell>{index + 1}</TableCell>
                          <TableCell>
                            <Chip 
                              label={log.status} 
                              color={log.status === 'completed' ? 'success' : log.status === 'failed' ? 'error' : 'default'} 
                              size="small" 
                            />
                          </TableCell>
                          <TableCell>{log.priority}</TableCell>
                          <TableCell>{new Date(log.created_at).toLocaleString()}</TableCell>
                          <TableCell>{log.started_at ? new Date(log.started_at).toLocaleString() : '-'}</TableCell>
                          <TableCell>{log.completed_at ? new Date(log.completed_at).toLocaleString() : '-'}</TableCell>
                          <TableCell>{log.processing_duration_ms ? `${log.processing_duration_ms}ms` : '-'}</TableCell>
                          <TableCell>{log.queue_wait_time_ms ? `${log.queue_wait_time_ms}ms` : '-'}</TableCell>
                          <TableCell>{log.attempts || 0}</TableCell>
                          <TableCell>{log.worker_id || '-'}</TableCell>
                          <TableCell>
                            {log.error_message ? (
                              <Alert severity="error" sx={{ minWidth: '200px' }}>
                                {log.error_message}
                              </Alert>
                            ) : '-'}
                          </TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                </TableContainer>
              </CardContent>
            </Card>
          )}

          {debugInfo.file_analysis && (
            <Card sx={{ mb: 4 }}>
              <CardContent>
                <Typography variant="h6" gutterBottom>
                  File Analysis Summary
                </Typography>
                <Grid container spacing={2}>
                  <Grid item xs={12} md={6}>
                    <Paper sx={{ p: 2 }}>
                      <Typography variant="subtitle1" gutterBottom>File Properties</Typography>
                      <Typography><strong>File Type:</strong> {debugInfo.file_analysis.file_type}</Typography>
                      <Typography><strong>Size:</strong> {(debugInfo.file_analysis.file_size_bytes / 1024 / 1024).toFixed(2)} MB</Typography>
                      <Typography><strong>Readable:</strong> <Chip 
                        label={debugInfo.file_analysis.is_readable ? 'Yes' : 'No'} 
                        color={debugInfo.file_analysis.is_readable ? 'success' : 'error'} 
                        size="small" 
                      /></Typography>
                    </Paper>
                  </Grid>
                  <Grid item xs={12} md={6}>
                    {debugInfo.file_analysis.pdf_info && (
                      <Paper sx={{ p: 2 }}>
                        <Typography variant="subtitle1" gutterBottom>PDF Properties</Typography>
                        <Typography><strong>Valid PDF:</strong> <Chip 
                          label={debugInfo.file_analysis.pdf_info.is_valid_pdf ? 'Yes' : 'No'} 
                          color={debugInfo.file_analysis.pdf_info.is_valid_pdf ? 'success' : 'error'} 
                          size="small" 
                        /></Typography>
                        <Typography><strong>Has Text Content:</strong> <Chip 
                          label={debugInfo.file_analysis.pdf_info.has_text_content ? 'Yes' : 'No'} 
                          color={debugInfo.file_analysis.pdf_info.has_text_content ? 'success' : 'warning'} 
                          size="small" 
                        /></Typography>
                        <Typography><strong>Text Length:</strong> {debugInfo.file_analysis.pdf_info.estimated_text_length} chars</Typography>
                        <Typography><strong>Page Count:</strong> {debugInfo.file_analysis.pdf_info.page_count || 'Unknown'}</Typography>
                        <Typography><strong>Encrypted:</strong> <Chip 
                          label={debugInfo.file_analysis.pdf_info.is_encrypted ? 'Yes' : 'No'} 
                          color={debugInfo.file_analysis.pdf_info.is_encrypted ? 'error' : 'success'} 
                          size="small" 
                        /></Typography>
                      </Paper>
                    )}
                  </Grid>
                  {debugInfo.file_analysis.pdf_info?.text_extraction_error && (
                    <Grid item xs={12}>
                      <Alert severity="error">
                        <strong>PDF Text Extraction Issue:</strong> {debugInfo.file_analysis.pdf_info.text_extraction_error}
                      </Alert>
                    </Grid>
                  )}
                </Grid>
              </CardContent>
            </Card>
          )}

          {(debugInfo.pipeline_steps || []).some(step => step.step === 3 && step.details?.has_processed_image) && (
            <Card sx={{ mb: 4 }}>
              <CardContent>
                <Typography variant="h6" gutterBottom>
                  Processed Images
                </Typography>
                <Grid container spacing={2}>
                  <Grid item xs={12} md={6}>
                    <Paper sx={{ p: 2 }}>
                      <Typography variant="subtitle1" gutterBottom>Original Document</Typography>
                      <Box 
                        component="iframe"
                        src={`/api/documents/${debugInfo.document_id}/view`}
                        sx={{
                          width: '100%',
                          height: '300px',
                          border: '1px solid',
                          borderColor: 'divider',
                          borderRadius: 1
                        }}
                      />
                    </Paper>
                  </Grid>
                  <Grid item xs={12} md={6}>
                    <Paper sx={{ p: 2 }}>
                      <Typography variant="subtitle1" gutterBottom>Processed Image (OCR Input)</Typography>
                      <Box 
                        component="img"
                        src={`/api/documents/${debugInfo.document_id}/processed-image`}
                        alt="Processed image for OCR"
                        onError={(e) => {
                          (e.target as HTMLImageElement).style.display = 'none';
                          (e.target as HTMLImageElement).parentNode?.appendChild(
                            document.createTextNode('Processed image not available')
                          );
                        }}
                        sx={{
                          maxWidth: '100%',
                          maxHeight: '300px',
                          objectFit: 'contain',
                          border: '1px solid',
                          borderColor: 'divider',
                          borderRadius: 1
                        }}
                      />
                    </Paper>
                  </Grid>
                </Grid>
              </CardContent>
            </Card>
          )}

          <Card>
            <CardContent>
              <Accordion>
                <AccordionSummary expandIcon={<ExpandMoreIcon />}>
                  <Typography variant="h6">User Settings</Typography>
                </AccordionSummary>
                <AccordionDetails>
                  <Grid container spacing={2}>
                    <Grid item xs={12} md={6}>
                      <Paper sx={{ p: 2 }}>
                        <Typography variant="subtitle1" gutterBottom>OCR Settings</Typography>
                        <Typography><strong>Background OCR:</strong> {debugInfo.user_settings?.enable_background_ocr ? 'Enabled' : 'Disabled'}</Typography>
                        <Typography><strong>Min Confidence:</strong> {debugInfo.user_settings?.ocr_min_confidence || 'N/A'}%</Typography>
                        <Typography><strong>Max File Size:</strong> {debugInfo.user_settings?.max_file_size_mb || 'N/A'} MB</Typography>
                      </Paper>
                    </Grid>
                    <Grid item xs={12} md={6}>
                      <Paper sx={{ p: 2 }}>
                        <Typography variant="subtitle1" gutterBottom>Quality Thresholds</Typography>
                        <Typography><strong>Brightness:</strong> {debugInfo.user_settings?.ocr_quality_threshold_brightness || 'N/A'}</Typography>
                        <Typography><strong>Contrast:</strong> {debugInfo.user_settings?.ocr_quality_threshold_contrast || 'N/A'}</Typography>
                        <Typography><strong>Noise:</strong> {debugInfo.user_settings?.ocr_quality_threshold_noise || 'N/A'}</Typography>
                        <Typography><strong>Sharpness:</strong> {debugInfo.user_settings?.ocr_quality_threshold_sharpness || 'N/A'}</Typography>
                      </Paper>
                    </Grid>
                  </Grid>
                </AccordionDetails>
              </Accordion>
            </CardContent>
          </Card>
        </Box>
      )}
    </Container>
  );
};

export default DebugPage;