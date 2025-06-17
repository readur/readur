import React, { useState, useEffect } from 'react';
import {
  Box,
  Typography,
  Card,
  CardContent,
  Grid,
  Button,
  Chip,
  Alert,
  AlertTitle,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Paper,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  Pagination,
  CircularProgress,
  Tooltip,
  IconButton,
  Collapse,
  LinearProgress,
  Snackbar,
} from '@mui/material';
import {
  Refresh as RefreshIcon,
  Error as ErrorIcon,
  Info as InfoIcon,
  ExpandMore as ExpandMoreIcon,
  ExpandLess as ExpandLessIcon,
  Schedule as ScheduleIcon,
  Visibility as VisibilityIcon,
  Download as DownloadIcon,
} from '@mui/icons-material';
import { format } from 'date-fns';
import { api, documentService } from '../services/api';

interface FailedDocument {
  id: string;
  filename: string;
  original_filename: string;
  file_size: number;
  mime_type: string;
  created_at: string;
  updated_at: string;
  tags: string[];
  ocr_status: string;
  ocr_error: string;
  ocr_failure_reason: string;
  ocr_completed_at?: string;
  retry_count: number;
  last_attempt_at?: string;
  can_retry: boolean;
  failure_category: string;
}

interface FailureCategory {
  reason: string;
  display_name: string;
  count: number;
}

interface FailedOcrResponse {
  documents: FailedDocument[];
  pagination: {
    total: number;
    limit: number;
    offset: number;
    has_more: boolean;
  };
  statistics: {
    total_failed: number;
    failure_categories: FailureCategory[];
  };
}

interface RetryResponse {
  success: boolean;
  message: string;
  queue_id?: string;
  estimated_wait_minutes?: number;
}

const FailedOcrPage: React.FC = () => {
  const [documents, setDocuments] = useState<FailedDocument[]>([]);
  const [loading, setLoading] = useState(true);
  const [retrying, setRetrying] = useState<string | null>(null);
  const [statistics, setStatistics] = useState<FailedOcrResponse['statistics'] | null>(null);
  const [pagination, setPagination] = useState({ page: 1, limit: 25 });
  const [totalPages, setTotalPages] = useState(0);
  const [selectedDocument, setSelectedDocument] = useState<FailedDocument | null>(null);
  const [detailsOpen, setDetailsOpen] = useState(false);
  const [expandedRows, setExpandedRows] = useState<Set<string>>(new Set());
  const [snackbar, setSnackbar] = useState<{ open: boolean; message: string; severity: 'success' | 'error' }>({
    open: false,
    message: '',
    severity: 'success'
  });

  const fetchFailedDocuments = async () => {
    try {
      setLoading(true);
      const offset = (pagination.page - 1) * pagination.limit;
      const response = await documentService.getFailedOcrDocuments(pagination.limit, offset);
      
      setDocuments(response.data.documents);
      setStatistics(response.data.statistics);
      setTotalPages(Math.ceil(response.data.pagination.total / pagination.limit));
    } catch (error) {
      console.error('Failed to fetch failed OCR documents:', error);
      setSnackbar({
        open: true,
        message: 'Failed to load failed OCR documents',
        severity: 'error'
      });
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchFailedDocuments();
  }, [pagination.page]);

  const handleRetryOcr = async (document: FailedDocument) => {
    try {
      setRetrying(document.id);
      const response = await documentService.retryOcr(document.id);
      
      if (response.data.success) {
        setSnackbar({
          open: true,
          message: `OCR retry queued for "${document.filename}". Estimated wait time: ${response.data.estimated_wait_minutes || 'Unknown'} minutes.`,
          severity: 'success'
        });
        
        // Refresh the list to update retry counts and status
        await fetchFailedDocuments();
      } else {
        setSnackbar({
          open: true,
          message: response.data.message || 'Failed to retry OCR',
          severity: 'error'
        });
      }
    } catch (error) {
      console.error('Failed to retry OCR:', error);
      setSnackbar({
        open: true,
        message: 'Failed to retry OCR processing',
        severity: 'error'
      });
    } finally {
      setRetrying(null);
    }
  };

  const formatFileSize = (bytes: number): string => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  };

  const getFailureCategoryColor = (category: string): "error" | "warning" | "info" | "default" => {
    switch (category) {
      case 'PDF Font Issues':
      case 'PDF Corruption':
      case 'PDF Parsing Error':
        return 'warning';
      case 'Timeout':
      case 'Memory Limit':
        return 'error';
      case 'Unknown Error':
        return 'info';
      default:
        return 'default';
    }
  };

  const toggleRowExpansion = (documentId: string) => {
    const newExpanded = new Set(expandedRows);
    if (newExpanded.has(documentId)) {
      newExpanded.delete(documentId);
    } else {
      newExpanded.add(documentId);
    }
    setExpandedRows(newExpanded);
  };

  const showDocumentDetails = (document: FailedDocument) => {
    setSelectedDocument(document);
    setDetailsOpen(true);
  };

  if (loading && documents.length === 0) {
    return (
      <Box display="flex" justifyContent="center" alignItems="center" minHeight="400px">
        <CircularProgress />
      </Box>
    );
  }

  return (
    <Box sx={{ p: 3 }}>
      <Box display="flex" justifyContent="space-between" alignItems="center" mb={3}>
        <Typography variant="h4" component="h1">
          Failed OCR Documents
        </Typography>
        <Button
          variant="outlined"
          startIcon={<RefreshIcon />}
          onClick={fetchFailedDocuments}
          disabled={loading}
        >
          Refresh
        </Button>
      </Box>

      {/* Statistics Overview */}
      {statistics && (
        <Grid container spacing={3} mb={3}>
          <Grid item xs={12} md={4}>
            <Card>
              <CardContent>
                <Typography variant="h6" color="error">
                  <ErrorIcon sx={{ mr: 1, verticalAlign: 'middle' }} />
                  Total Failed
                </Typography>
                <Typography variant="h3" color="error.main">
                  {statistics.total_failed}
                </Typography>
              </CardContent>
            </Card>
          </Grid>
          <Grid item xs={12} md={8}>
            <Card>
              <CardContent>
                <Typography variant="h6" mb={2}>
                  Failure Categories
                </Typography>
                <Box display="flex" flexWrap="wrap" gap={1}>
                  {statistics.failure_categories.map((category) => (
                    <Chip
                      key={category.reason}
                      label={`${category.display_name}: ${category.count}`}
                      color={getFailureCategoryColor(category.display_name)}
                      variant="outlined"
                      size="small"
                    />
                  ))}
                </Box>
              </CardContent>
            </Card>
          </Grid>
        </Grid>
      )}

      {documents.length === 0 ? (
        <Alert severity="success" sx={{ mt: 2 }}>
          <AlertTitle>Great news!</AlertTitle>
          No documents have failed OCR processing. All your documents are processing successfully.
        </Alert>
      ) : (
        <>
          <Alert severity="info" sx={{ mb: 2 }}>
            <AlertTitle>OCR Failures</AlertTitle>
            These documents failed OCR processing. You can retry OCR with detailed output to understand why failures occurred.
            Common causes include corrupted PDFs, unsupported fonts, or memory limitations.
          </Alert>

          <TableContainer component={Paper}>
            <Table>
              <TableHead>
                <TableRow>
                  <TableCell />
                  <TableCell>Document</TableCell>
                  <TableCell>Failure Type</TableCell>
                  <TableCell>Retry Count</TableCell>
                  <TableCell>Last Failed</TableCell>
                  <TableCell>Actions</TableCell>
                </TableRow>
              </TableHead>
              <TableBody>
                {documents.map((document) => (
                  <React.Fragment key={document.id}>
                    <TableRow>
                      <TableCell>
                        <IconButton
                          size="small"
                          onClick={() => toggleRowExpansion(document.id)}
                        >
                          {expandedRows.has(document.id) ? <ExpandLessIcon /> : <ExpandMoreIcon />}
                        </IconButton>
                      </TableCell>
                      <TableCell>
                        <Box>
                          <Typography variant="body2" fontWeight="bold">
                            {document.filename}
                          </Typography>
                          <Typography variant="caption" color="text.secondary">
                            {formatFileSize(document.file_size)} • {document.mime_type}
                          </Typography>
                        </Box>
                      </TableCell>
                      <TableCell>
                        <Chip
                          label={document.failure_category}
                          color={getFailureCategoryColor(document.failure_category)}
                          size="small"
                        />
                      </TableCell>
                      <TableCell>
                        <Typography variant="body2">
                          {document.retry_count} attempts
                        </Typography>
                      </TableCell>
                      <TableCell>
                        <Typography variant="body2">
                          {document.updated_at ? format(new Date(document.updated_at), 'MMM dd, yyyy HH:mm') : 'Unknown'}
                        </Typography>
                      </TableCell>
                      <TableCell>
                        <Box display="flex" gap={1}>
                          <Tooltip title="Retry OCR">
                            <IconButton
                              size="small"
                              onClick={() => handleRetryOcr(document)}
                              disabled={retrying === document.id || !document.can_retry}
                            >
                              {retrying === document.id ? (
                                <CircularProgress size={16} />
                              ) : (
                                <RefreshIcon />
                              )}
                            </IconButton>
                          </Tooltip>
                          <Tooltip title="View Details">
                            <IconButton
                              size="small"
                              onClick={() => showDocumentDetails(document)}
                            >
                              <VisibilityIcon />
                            </IconButton>
                          </Tooltip>
                          <Tooltip title="Download Document">
                            <IconButton
                              size="small"
                              onClick={() => window.open(`/api/documents/${document.id}/download`, '_blank')}
                            >
                              <DownloadIcon />
                            </IconButton>
                          </Tooltip>
                        </Box>
                      </TableCell>
                    </TableRow>
                    <TableRow>
                      <TableCell sx={{ paddingBottom: 0, paddingTop: 0 }} colSpan={6}>
                        <Collapse in={expandedRows.has(document.id)} timeout="auto" unmountOnExit>
                          <Box sx={{ margin: 1, p: 2, bgcolor: 'grey.50' }}>
                            <Typography variant="h6" gutterBottom>
                              Error Details
                            </Typography>
                            <Grid container spacing={2}>
                              <Grid item xs={12} md={6}>
                                <Typography variant="body2" color="text.secondary">
                                  <strong>Failure Reason:</strong>
                                </Typography>
                                <Typography variant="body2" sx={{ mb: 1 }}>
                                  {document.ocr_failure_reason || 'Not specified'}
                                </Typography>
                                
                                <Typography variant="body2" color="text.secondary">
                                  <strong>Error Message:</strong>
                                </Typography>
                                <Typography
                                  variant="body2"
                                  sx={{
                                    fontFamily: 'monospace',
                                    bgcolor: 'grey.100',
                                    p: 1,
                                    borderRadius: 1,
                                    fontSize: '0.75rem',
                                    wordBreak: 'break-word'
                                  }}
                                >
                                  {document.ocr_error || 'No error message available'}
                                </Typography>
                              </Grid>
                              <Grid item xs={12} md={6}>
                                <Typography variant="body2" color="text.secondary">
                                  <strong>Last Attempt:</strong>
                                </Typography>
                                <Typography variant="body2" sx={{ mb: 1 }}>
                                  {document.last_attempt_at
                                    ? format(new Date(document.last_attempt_at), 'PPpp')
                                    : 'No previous attempts'}
                                </Typography>
                                
                                <Typography variant="body2" color="text.secondary">
                                  <strong>File Created:</strong>
                                </Typography>
                                <Typography variant="body2">
                                  {format(new Date(document.created_at), 'PPpp')}
                                </Typography>
                              </Grid>
                            </Grid>
                          </Box>
                        </Collapse>
                      </TableCell>
                    </TableRow>
                  </React.Fragment>
                ))}
              </TableBody>
            </Table>
          </TableContainer>

          {/* Pagination */}
          {totalPages > 1 && (
            <Box display="flex" justifyContent="center" mt={3}>
              <Pagination
                count={totalPages}
                page={pagination.page}
                onChange={(_, page) => setPagination(prev => ({ ...prev, page }))}
                color="primary"
              />
            </Box>
          )}
        </>
      )}

      {/* Document Details Dialog */}
      <Dialog
        open={detailsOpen}
        onClose={() => setDetailsOpen(false)}
        maxWidth="md"
        fullWidth
      >
        <DialogTitle>
          Document Details: {selectedDocument?.filename}
        </DialogTitle>
        <DialogContent>
          {selectedDocument && (
            <Grid container spacing={2}>
              <Grid item xs={12} md={6}>
                <Typography variant="body2" color="text.secondary">
                  <strong>Original Filename:</strong>
                </Typography>
                <Typography variant="body2" sx={{ mb: 2 }}>
                  {selectedDocument.original_filename}
                </Typography>

                <Typography variant="body2" color="text.secondary">
                  <strong>File Size:</strong>
                </Typography>
                <Typography variant="body2" sx={{ mb: 2 }}>
                  {formatFileSize(selectedDocument.file_size)}
                </Typography>

                <Typography variant="body2" color="text.secondary">
                  <strong>MIME Type:</strong>
                </Typography>
                <Typography variant="body2" sx={{ mb: 2 }}>
                  {selectedDocument.mime_type}
                </Typography>

                <Typography variant="body2" color="text.secondary">
                  <strong>Tags:</strong>
                </Typography>
                <Box sx={{ mb: 2 }}>
                  {selectedDocument.tags.length > 0 ? (
                    selectedDocument.tags.map((tag) => (
                      <Chip key={tag} label={tag} size="small" sx={{ mr: 1, mb: 1 }} />
                    ))
                  ) : (
                    <Typography variant="body2" color="text.secondary">No tags</Typography>
                  )}
                </Box>
              </Grid>
              <Grid item xs={12} md={6}>
                <Typography variant="body2" color="text.secondary">
                  <strong>Failure Category:</strong>
                </Typography>
                <Chip
                  label={selectedDocument.failure_category}
                  color={getFailureCategoryColor(selectedDocument.failure_category)}
                  sx={{ mb: 2 }}
                />

                <Typography variant="body2" color="text.secondary">
                  <strong>Retry Count:</strong>
                </Typography>
                <Typography variant="body2" sx={{ mb: 2 }}>
                  {selectedDocument.retry_count} attempts
                </Typography>

                <Typography variant="body2" color="text.secondary">
                  <strong>Created:</strong>
                </Typography>
                <Typography variant="body2" sx={{ mb: 2 }}>
                  {format(new Date(selectedDocument.created_at), 'PPpp')}
                </Typography>

                <Typography variant="body2" color="text.secondary">
                  <strong>Last Updated:</strong>
                </Typography>
                <Typography variant="body2">
                  {format(new Date(selectedDocument.updated_at), 'PPpp')}
                </Typography>
              </Grid>
              <Grid item xs={12}>
                <Typography variant="body2" color="text.secondary" sx={{ mb: 1 }}>
                  <strong>Full Error Message:</strong>
                </Typography>
                <Paper sx={{ p: 2, bgcolor: 'grey.50' }}>
                  <Typography
                    variant="body2"
                    sx={{
                      fontFamily: 'monospace',
                      fontSize: '0.875rem',
                      wordBreak: 'break-word',
                      whiteSpace: 'pre-wrap'
                    }}
                  >
                    {selectedDocument.ocr_error || 'No error message available'}
                  </Typography>
                </Paper>
              </Grid>
            </Grid>
          )}
        </DialogContent>
        <DialogActions>
          {selectedDocument?.can_retry && (
            <Button
              onClick={() => {
                setDetailsOpen(false);
                if (selectedDocument) {
                  handleRetryOcr(selectedDocument);
                }
              }}
              startIcon={<RefreshIcon />}
              disabled={retrying === selectedDocument?.id}
            >
              Retry OCR
            </Button>
          )}
          <Button onClick={() => setDetailsOpen(false)}>Close</Button>
        </DialogActions>
      </Dialog>

      {/* Success/Error Snackbar */}
      <Snackbar
        open={snackbar.open}
        autoHideDuration={6000}
        onClose={() => setSnackbar(prev => ({ ...prev, open: false }))}
      >
        <Alert
          onClose={() => setSnackbar(prev => ({ ...prev, open: false }))}
          severity={snackbar.severity}
          sx={{ width: '100%' }}
        >
          {snackbar.message}
        </Alert>
      </Snackbar>
    </Box>
  );
};

export default FailedOcrPage;