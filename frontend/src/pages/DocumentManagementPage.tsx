import React, { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Box,
  Typography,
  Card,
  CardContent,
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
  Tabs,
  Tab,
  TextField,
  MenuItem,
  useTheme,
  Divider,
  InputAdornment,
} from '@mui/material';
import Grid from '@mui/material/GridLegacy';
import {
  Refresh as RefreshIcon,
  Error as ErrorIcon,
  Info as InfoIcon,
  ExpandMore as ExpandMoreIcon,
  ExpandLess as ExpandLessIcon,
  Schedule as ScheduleIcon,
  Visibility as VisibilityIcon,
  Download as DownloadIcon,
  FileCopy as FileCopyIcon,
  Delete as DeleteIcon,
  FindInPage as FindInPageIcon,
  OpenInNew as OpenInNewIcon,
  Warning as WarningIcon,
  Block as BlockIcon,
} from '@mui/icons-material';
import { format } from 'date-fns';
import { api, documentService, queueService } from '../services/api';
import DocumentViewer from '../components/DocumentViewer';
import FailedDocumentViewer from '../components/FailedDocumentViewer';

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
  ocr_confidence?: number;
  ocr_word_count?: number;
  failure_reason: string;
  error_message?: string;
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
    total_pages: number;
  };
  statistics: {
    total_failed: number;
    by_reason: Record<string, number>;
    by_stage: Record<string, number>;
  };
}

interface RetryResponse {
  success: boolean;
  message: string;
  queue_id?: string;
  estimated_wait_minutes?: number;
}

interface DuplicateDocument {
  id: string;
  filename: string;
  original_filename: string;
  file_size: number;
  mime_type: string;
  created_at: string;
  user_id: string;
}

interface DuplicateGroup {
  file_hash: string;
  duplicate_count: number;
  first_uploaded: string;
  last_uploaded: string;
  documents: DuplicateDocument[];
}

interface DuplicatesResponse {
  duplicates: DuplicateGroup[];
  pagination: {
    total: number;
    limit: number;
    offset: number;
    has_more: boolean;
  };
  statistics: {
    total_duplicate_groups: number;
  };
}

interface IgnoredFile {
  id: string;
  file_hash: string;
  filename: string;
  original_filename: string;
  file_path: string;
  file_size: number;
  mime_type: string;
  source_type?: string;
  source_path?: string;
  source_identifier?: string;
  ignored_at: string;
  ignored_by: string;
  ignored_by_username?: string;
  reason?: string;
  created_at: string;
}

interface IgnoredFilesStats {
  total_ignored_files: number;
  by_source_type: Array<{
    source_type?: string;
    count: number;
    total_size_bytes: number;
  }>;
  total_size_bytes: number;
  most_recent_ignored_at?: string;
}

const DocumentManagementPage: React.FC = () => {
  const theme = useTheme();
  const navigate = useNavigate();
  const [currentTab, setCurrentTab] = useState(0);
  const [documents, setDocuments] = useState<FailedDocument[]>([]);
  const [duplicates, setDuplicates] = useState<DuplicateGroup[]>([]);
  const [loading, setLoading] = useState(true);
  const [duplicatesLoading, setDuplicatesLoading] = useState(false);
  const [failedDocumentsFilters, setFailedDocumentsFilters] = useState<{ stage?: string; reason?: string }>({});
  const [selectedFailedDocument, setSelectedFailedDocument] = useState<any>(null);
  const [retrying, setRetrying] = useState<string | null>(null);
  const [retryingAll, setRetryingAll] = useState(false);
  const [statistics, setStatistics] = useState<FailedOcrResponse['statistics'] | null>(null);
  const [duplicateStatistics, setDuplicateStatistics] = useState<DuplicatesResponse['statistics'] | null>(null);
  const [pagination, setPagination] = useState({ page: 1, limit: 25 });
  const [duplicatesPagination, setDuplicatesPagination] = useState({ page: 1, limit: 25 });
  const [totalPages, setTotalPages] = useState(0);
  const [duplicatesTotalPages, setDuplicatesTotalPages] = useState(0);
  const [selectedDocument, setSelectedDocument] = useState<FailedDocument | null>(null);
  const [detailsOpen, setDetailsOpen] = useState(false);
  const [expandedRows, setExpandedRows] = useState<Set<string>>(new Set());
  const [expandedDuplicateGroups, setExpandedDuplicateGroups] = useState<Set<string>>(new Set());
  const [snackbar, setSnackbar] = useState<{ open: boolean; message: string; severity: 'success' | 'error' | 'info' | 'warning' }>({
    open: false,
    message: '',
    severity: 'success'
  });
  
  // Low confidence documents state
  const [confidenceThreshold, setConfidenceThreshold] = useState<number>(30);
  const [lowConfidenceLoading, setLowConfidenceLoading] = useState(false);
  const [previewData, setPreviewData] = useState<any>(null);
  const [confirmDeleteOpen, setConfirmDeleteOpen] = useState(false);

  // Failed documents deletion state
  const [failedDocsLoading, setFailedDocsLoading] = useState(false);
  const [failedPreviewData, setFailedPreviewData] = useState<any>(null);
  const [confirmDeleteFailedOpen, setConfirmDeleteFailedOpen] = useState(false);
  
  // Ignored files state
  const [ignoredFiles, setIgnoredFiles] = useState<IgnoredFile[]>([]);
  const [ignoredFilesStats, setIgnoredFilesStats] = useState<IgnoredFilesStats | null>(null);
  const [ignoredFilesLoading, setIgnoredFilesLoading] = useState(false);
  const [ignoredFilesPagination, setIgnoredFilesPagination] = useState({ page: 1, limit: 25 });
  const [ignoredFilesTotalPages, setIgnoredFilesTotalPages] = useState(0);
  const [ignoredFilesSearchTerm, setIgnoredFilesSearchTerm] = useState('');
  const [ignoredFilesSourceTypeFilter, setIgnoredFilesSourceTypeFilter] = useState('');
  const [selectedIgnoredFiles, setSelectedIgnoredFiles] = useState<Set<string>>(new Set());
  const [bulkDeleteIgnoredDialog, setBulkDeleteIgnoredDialog] = useState(false);
  const [deletingIgnoredFiles, setDeletingIgnoredFiles] = useState(false);

  const fetchFailedDocuments = async () => {
    try {
      setLoading(true);
      const offset = (pagination.page - 1) * pagination.limit;
      // Use the comprehensive API that supports filtering
      const response = await documentService.getFailedDocuments(
        pagination.limit, 
        offset,
        failedDocumentsFilters.stage,
        failedDocumentsFilters.reason
      );
      
      if (response?.data) {
        setDocuments(response.data.documents || []);
        setStatistics(response.data.statistics || null);
        if (response.data.pagination) {
          setTotalPages(Math.ceil(response.data.pagination.total / pagination.limit));
        }
      }
    } catch (error) {
      console.error('Failed to fetch failed documents:', error);
      setSnackbar({
        open: true,
        message: 'Failed to load failed documents',
        severity: 'error'
      });
    } finally {
      setLoading(false);
    }
  };

  const fetchDuplicates = async () => {
    try {
      setDuplicatesLoading(true);
      const offset = (duplicatesPagination.page - 1) * duplicatesPagination.limit;
      const response = await documentService.getDuplicates(duplicatesPagination.limit, offset);
      
      if (response?.data) {
        setDuplicates(response.data.duplicates || []);
        setDuplicateStatistics(response.data.statistics || null);
        if (response.data.pagination) {
          setDuplicatesTotalPages(Math.ceil(response.data.pagination.total / duplicatesPagination.limit));
        }
      }
    } catch (error) {
      console.error('Failed to fetch duplicates:', error);
      setSnackbar({
        open: true,
        message: 'Failed to load duplicate documents',
        severity: 'error'
      });
    } finally {
      setDuplicatesLoading(false);
    }
  };

  useEffect(() => {
    fetchFailedDocuments();
    // Also fetch ignored files stats for the tab label
    fetchIgnoredFilesStats();
  }, [pagination.page, failedDocumentsFilters]);

  useEffect(() => {
    if (currentTab === 2) {
      fetchDuplicates();
    } else if (currentTab === 4) {
      fetchIgnoredFiles();
    }
  }, [currentTab, duplicatesPagination.page, ignoredFilesPagination.page, ignoredFilesSearchTerm, ignoredFilesSourceTypeFilter]);


  const getFailureReasonColor = (reason: string): "error" | "warning" | "info" | "default" => {
    switch (reason) {
      case 'low_ocr_confidence':
      case 'ocr_timeout':
      case 'ocr_memory_limit':
      case 'pdf_parsing_error':
        return 'error';
      case 'duplicate_content':
      case 'unsupported_format':
      case 'file_too_large':
        return 'warning';
      case 'file_corrupted':
      case 'access_denied':
      case 'permission_denied':
        return 'error';
      default:
        return 'default';
    }
  };

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

  const handleRetryAllFailed = async () => {
    try {
      setRetryingAll(true);
      const response = await queueService.requeueFailed();
      
      if (response.data.requeued_count > 0) {
        setSnackbar({
          open: true,
          message: `Successfully queued ${response.data.requeued_count} failed documents for OCR retry. Check the queue stats for progress.`,
          severity: 'success'
        });
        
        // Refresh the list to update status
        await fetchFailedDocuments();
      } else {
        setSnackbar({
          open: true,
          message: 'No failed documents found to retry',
          severity: 'info'
        });
      }
    } catch (error) {
      console.error('Failed to retry all failed OCR:', error);
      setSnackbar({
        open: true,
        message: 'Failed to retry all failed OCR documents',
        severity: 'error'
      });
    } finally {
      setRetryingAll(false);
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
      case 'Low OCR Confidence':
        return 'warning';
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
  
  // Ignored Files functions
  const fetchIgnoredFiles = async () => {
    try {
      setIgnoredFilesLoading(true);
      const offset = (ignoredFilesPagination.page - 1) * ignoredFilesPagination.limit;
      
      const params = new URLSearchParams({
        limit: ignoredFilesPagination.limit.toString(),
        offset: offset.toString(),
      });
      
      if (ignoredFilesSearchTerm) {
        params.append('filename', ignoredFilesSearchTerm);
      }
      
      if (ignoredFilesSourceTypeFilter) {
        params.append('source_type', ignoredFilesSourceTypeFilter);
      }
      
      const response = await api.get(`/ignored-files?${params}`);
      
      if (response?.data) {
        setIgnoredFiles(response.data.ignored_files || []);
        setIgnoredFilesTotalPages(Math.ceil(response.data.total / ignoredFilesPagination.limit));
      }
    } catch (error) {
      console.error('Failed to fetch ignored files:', error);
      setSnackbar({
        open: true,
        message: 'Failed to load ignored files',
        severity: 'error'
      });
    } finally {
      setIgnoredFilesLoading(false);
    }
  };
  
  const fetchIgnoredFilesStats = async () => {
    try {
      const response = await api.get('/ignored-files/stats');
      if (response?.data) {
        setIgnoredFilesStats(response.data);
      }
    } catch (error) {
      console.error('Failed to fetch ignored files stats:', error);
    }
  };
  
  const handleIgnoredFileSelect = (fileId: string) => {
    const newSelected = new Set(selectedIgnoredFiles);
    if (newSelected.has(fileId)) {
      newSelected.delete(fileId);
    } else {
      newSelected.add(fileId);
    }
    setSelectedIgnoredFiles(newSelected);
  };
  
  const handleIgnoredFilesSelectAll = () => {
    if (selectedIgnoredFiles.size === ignoredFiles.length) {
      setSelectedIgnoredFiles(new Set());
    } else {
      setSelectedIgnoredFiles(new Set(ignoredFiles.map(file => file.id)));
    }
  };
  
  const handleDeleteSelectedIgnoredFiles = async () => {
    if (selectedIgnoredFiles.size === 0) return;
    
    setDeletingIgnoredFiles(true);
    try {
      const response = await api.delete('/ignored-files/bulk-delete', {
        data: {
          ignored_file_ids: Array.from(selectedIgnoredFiles)
        }
      });
      
      setSnackbar({
        open: true,
        message: response.data.message || 'Files removed from ignored list',
        severity: 'success'
      });
      setSelectedIgnoredFiles(new Set());
      setBulkDeleteIgnoredDialog(false);
      fetchIgnoredFiles();
      fetchIgnoredFilesStats();
    } catch (error: any) {
      setSnackbar({
        open: true,
        message: error.response?.data?.message || 'Failed to delete ignored files',
        severity: 'error'
      });
    } finally {
      setDeletingIgnoredFiles(false);
    }
  };
  
  const handleDeleteSingleIgnoredFile = async (fileId: string) => {
    try {
      const response = await api.delete(`/ignored-files/${fileId}`);
      setSnackbar({
        open: true,
        message: response.data.message || 'File removed from ignored list',
        severity: 'success'
      });
      fetchIgnoredFiles();
      fetchIgnoredFilesStats();
    } catch (error: any) {
      setSnackbar({
        open: true,
        message: error.response?.data?.message || 'Failed to delete ignored file',
        severity: 'error'
      });
    }
  };
  
  const getSourceIcon = (sourceType?: string) => {
    switch (sourceType) {
      case 'webdav':
        return <OpenInNewIcon fontSize="small" />;
      case 'local_folder':
        return <FileCopyIcon fontSize="small" />;
      case 's3':
        return <DownloadIcon fontSize="small" />;
      default:
        return <FileCopyIcon fontSize="small" />;
    }
  };
  
  const getSourceTypeDisplay = (sourceType?: string) => {
    switch (sourceType) {
      case 'webdav':
        return 'WebDAV';
      case 'local_folder':
        return 'Local Folder';
      case 's3':
        return 'S3';
      default:
        return sourceType || 'Unknown';
    }
  };

  const toggleDuplicateGroupExpansion = (groupHash: string) => {
    const newExpanded = new Set(expandedDuplicateGroups);
    if (newExpanded.has(groupHash)) {
      newExpanded.delete(groupHash);
    } else {
      newExpanded.add(groupHash);
    }
    setExpandedDuplicateGroups(newExpanded);
  };

  const handleTabChange = (event: React.SyntheticEvent, newValue: number) => {
    setCurrentTab(newValue);
  };

  const refreshCurrentTab = () => {
    if (currentTab === 0) {
      fetchFailedDocuments();
    } else if (currentTab === 1) {
      handlePreviewLowConfidence();
    } else if (currentTab === 2) {
      fetchDuplicates();
    } else if (currentTab === 3) {
      handlePreviewFailedDocuments();
    } else if (currentTab === 4) {
      fetchIgnoredFiles();
      fetchIgnoredFilesStats();
    }
  };

  // Low confidence document handlers
  const handlePreviewLowConfidence = async () => {
    try {
      setLowConfidenceLoading(true);
      const response = await documentService.deleteLowConfidence(confidenceThreshold, true);
      setPreviewData(response.data);
      setSnackbar({
        open: true,
        message: response.data.message,
        severity: 'info'
      });
    } catch (error) {
      setSnackbar({
        open: true,
        message: 'Failed to preview low confidence documents',
        severity: 'error'
      });
    } finally {
      setLowConfidenceLoading(false);
    }
  };

  const handleDeleteLowConfidence = async () => {
    if (!previewData || previewData.matched_count === 0) {
      setSnackbar({
        open: true,
        message: 'No documents to delete',
        severity: 'warning'
      });
      return;
    }

    try {
      setLowConfidenceLoading(true);
      const response = await documentService.deleteLowConfidence(confidenceThreshold, false);
      setSnackbar({
        open: true,
        message: response.data.message,
        severity: 'success'
      });
      setPreviewData(null);
      setConfirmDeleteOpen(false);
      
      // Refresh other tabs if they have data affected
      if (currentTab === 0) {
        fetchFailedDocuments();
      }
    } catch (error) {
      setSnackbar({
        open: true,
        message: 'Failed to delete low confidence documents',
        severity: 'error'
      });
    } finally {
      setLowConfidenceLoading(false);
    }
  };

  // Failed documents handlers
  const handlePreviewFailedDocuments = async () => {
    try {
      setFailedDocsLoading(true);
      const response = await documentService.deleteFailedOcr(true);
      setFailedPreviewData(response.data);
    } catch (error) {
      setSnackbar({
        open: true,
        message: 'Failed to preview failed documents',
        severity: 'error'
      });
    } finally {
      setFailedDocsLoading(false);
    }
  };

  const handleDeleteFailedDocuments = async () => {
    try {
      setFailedDocsLoading(true);
      const response = await documentService.deleteFailedOcr(false);
      
      setSnackbar({
        open: true,
        message: response.data.message,
        severity: 'success'
      });
      setFailedPreviewData(null);
      setConfirmDeleteFailedOpen(false);
      
      // Refresh failed OCR tab if currently viewing it
      if (currentTab === 0) {
        fetchFailedDocuments();
      }
    } catch (error) {
      setSnackbar({
        open: true,
        message: 'Failed to delete failed documents',
        severity: 'error'
      });
    } finally {
      setFailedDocsLoading(false);
    }
  };

  if (loading && (!documents || documents.length === 0)) {
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
          Document Management
        </Typography>
        <Button
          variant="outlined"
          startIcon={<RefreshIcon />}
          onClick={refreshCurrentTab}
          disabled={loading || duplicatesLoading || retryingAll}
        >
          Refresh
        </Button>
      </Box>

      <Paper sx={{ mb: 3, borderRadius: 2, overflow: 'hidden' }}>
        <Tabs 
          value={currentTab} 
          onChange={handleTabChange} 
          aria-label="document management tabs"
          variant="scrollable"
          scrollButtons="auto"
          sx={{
            '& .MuiTabs-flexContainer': {
              gap: 1,
            },
            '& .MuiTab-root': {
              minHeight: 64,
              py: 2,
              px: 3,
              textTransform: 'none',
              fontWeight: 500,
              '&.Mui-selected': {
                backgroundColor: 'action.selected',
              },
            },
          }}
        >
          <Tooltip title="View and manage documents that failed during processing (OCR, ingestion, validation, etc.)">
            <Tab
              icon={<ErrorIcon />}
              label={`Failed Documents${statistics ? ` (${statistics.total_failed})` : ''}`}
              iconPosition="start"
            />
          </Tooltip>
          <Tooltip title="Manage documents with low OCR confidence scores - preview and delete documents below a confidence threshold">
            <Tab
              icon={<FindInPageIcon />}
              label={`Low Quality Manager${previewData ? ` (${previewData.matched_count})` : ''}`}
              iconPosition="start"
            />
          </Tooltip>
          <Tooltip title="View and manage duplicate document groups - documents with identical content">
            <Tab
              icon={<FileCopyIcon />}
              label={`Duplicate Files${duplicateStatistics ? ` (${duplicateStatistics.total_duplicate_groups})` : ''}`}
              iconPosition="start"
            />
          </Tooltip>
          <Tooltip title="Bulk operations for document cleanup and maintenance">
            <Tab
              icon={<DeleteIcon />}
              label="Bulk Cleanup"
              iconPosition="start"
            />
          </Tooltip>
          <Tooltip title="Manage files that have been ignored during sync operations">
            <Tab
              icon={<BlockIcon />}
              label={`Ignored Files${ignoredFilesStats ? ` (${ignoredFilesStats.total_ignored_files})` : ''}`}
              iconPosition="start"
            />
          </Tooltip>
        </Tabs>
      </Paper>

      {/* Failed OCR Tab Content */}
      {currentTab === 0 && (
        <>
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
                <Box sx={{ mt: 2 }}>
                  <Button
                    variant="contained"
                    color="warning"
                    startIcon={retryingAll ? <CircularProgress size={20} /> : <RefreshIcon />}
                    onClick={handleRetryAllFailed}
                    disabled={retryingAll || statistics.total_failed === 0}
                    size="small"
                    fullWidth
                  >
                    {retryingAll ? 'Retrying All...' : 'Retry All Failed OCR'}
                  </Button>
                </Box>
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
                  {statistics?.by_reason ? Object.entries(statistics.by_reason).map(([reason, count]) => (
                    <Chip
                      key={reason}
                      label={`${reason}: ${count}`}
                      color={getFailureCategoryColor(reason)}
                      variant="outlined"
                      size="small"
                    />
                  )) : (
                    <Typography variant="body2" color="text.secondary">
                      No failure data available
                    </Typography>
                  )}
                </Box>
              </CardContent>
            </Card>
          </Grid>
        </Grid>
      )}

      {/* Filter Controls */}
      <Card sx={{ mb: 3 }}>
        <CardContent>
          <Typography variant="h6" mb={2}>Filter Options</Typography>
          <Grid container spacing={3} alignItems="center">
            <Grid item xs={12} md={4}>
              <TextField
                label="Filter by Stage"
                select
                value={failedDocumentsFilters.stage || ''}
                onChange={(e) => setFailedDocumentsFilters(prev => ({ ...prev, stage: e.target.value || undefined }))}
                displayEmpty
                fullWidth
              >
                <MenuItem value="">All Stages</MenuItem>
                <MenuItem value="ocr">OCR Processing</MenuItem>
                <MenuItem value="ingestion">Document Ingestion</MenuItem>
                <MenuItem value="validation">Validation</MenuItem>
                <MenuItem value="storage">File Storage</MenuItem>
                <MenuItem value="processing">Processing</MenuItem>
                <MenuItem value="sync">Synchronization</MenuItem>
              </TextField>
            </Grid>
            <Grid item xs={12} md={4}>
              <TextField
                label="Filter by Reason"
                select
                value={failedDocumentsFilters.reason || ''}
                onChange={(e) => setFailedDocumentsFilters(prev => ({ ...prev, reason: e.target.value || undefined }))}
                displayEmpty
                fullWidth
              >
                <MenuItem value="">All Reasons</MenuItem>
                <MenuItem value="duplicate_content">Duplicate Content</MenuItem>
                <MenuItem value="low_ocr_confidence">Low OCR Confidence</MenuItem>
                <MenuItem value="unsupported_format">Unsupported Format</MenuItem>
                <MenuItem value="file_too_large">File Too Large</MenuItem>
                <MenuItem value="file_corrupted">File Corrupted</MenuItem>
                <MenuItem value="ocr_timeout">OCR Timeout</MenuItem>
                <MenuItem value="pdf_parsing_error">PDF Parsing Error</MenuItem>
                <MenuItem value="other">Other</MenuItem>
              </TextField>
            </Grid>
            <Grid item xs={12} md={4}>
              <Button
                variant="outlined"
                onClick={() => setFailedDocumentsFilters({})}
                disabled={!failedDocumentsFilters.stage && !failedDocumentsFilters.reason}
                fullWidth
              >
                Clear Filters
              </Button>
            </Grid>
          </Grid>
        </CardContent>
      </Card>

      {(!documents || documents.length === 0) ? (
        <Alert severity="success" sx={{ mt: 2 }}>
          <AlertTitle>Great news!</AlertTitle>
          No documents have failed OCR processing. All your documents are processing successfully.
        </Alert>
      ) : (
        <>
          <Alert severity="info" sx={{ mb: 2 }}>
            <AlertTitle>Failed Documents Overview</AlertTitle>
            These documents failed at various stages of processing: ingestion, validation, OCR, storage, etc.
            Use the filters above to narrow down by failure stage or specific reason. You can retry processing for recoverable failures.
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
                {(documents || []).map((document) => (
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
                              onClick={async () => {
                                try {
                                  await documentService.downloadFile(document.id, document.original_filename || document.filename);
                                } catch (error) {
                                  console.error('Download failed:', error);
                                }
                              }}
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
                          <Box sx={{ 
                            margin: 1, 
                            p: 2, 
                            bgcolor: (theme) => theme.palette.mode === 'dark' ? 'grey.900' : 'grey.50',
                            borderRadius: 1
                          }}>
                            <Typography variant="h6" gutterBottom>
                              Error Details
                            </Typography>
                            <Grid container spacing={2}>
                              <Grid item xs={12} md={6}>
                                <Typography variant="body2" color="text.secondary">
                                  <strong>Failure Reason:</strong>
                                </Typography>
                                <Typography variant="body2" sx={{ mb: 1 }}>
                                  {document.failure_reason || document.ocr_failure_reason || 'Not specified'}
                                </Typography>
                                
                                {/* Show OCR confidence and word count for low confidence failures */}
                                {(document.failure_reason === 'low_ocr_confidence' || document.ocr_failure_reason === 'low_ocr_confidence') && (
                                  <>
                                    <Typography variant="body2" color="text.secondary">
                                      <strong>OCR Results:</strong>
                                    </Typography>
                                    <Box sx={{ mb: 1, display: 'flex', gap: 2, flexWrap: 'wrap' }}>
                                      {document.ocr_confidence !== undefined && (
                                        <Chip
                                          size="small"
                                          icon={<WarningIcon />}
                                          label={`${document.ocr_confidence.toFixed(1)}% confidence`}
                                          color="warning"
                                          variant="outlined"
                                        />
                                      )}
                                      {document.ocr_word_count !== undefined && (
                                        <Chip
                                          size="small"
                                          icon={<FindInPageIcon />}
                                          label={`${document.ocr_word_count} words found`}
                                          color="info"
                                          variant="outlined"
                                        />
                                      )}
                                    </Box>
                                  </>
                                )}
                                
                                <Typography variant="body2" color="text.secondary">
                                  <strong>Error Message:</strong>
                                </Typography>
                                <Typography
                                  variant="body2"
                                  sx={{
                                    fontFamily: 'monospace',
                                    bgcolor: (theme) => theme.palette.mode === 'dark' ? 'grey.800' : 'grey.100',
                                    p: 1,
                                    borderRadius: 1,
                                    fontSize: '0.75rem',
                                    wordBreak: 'break-word'
                                  }}
                                >
                                  {document.error_message || document.ocr_error || 'No error message available'}
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
        </>
      )}

      {/* Duplicate Files Tab Content */}
      {currentTab === 2 && (
        <>
          {/* Duplicate Statistics Overview */}
          {duplicateStatistics && (
            <Grid container spacing={3} mb={3}>
              <Grid item xs={12} md={6}>
                <Card>
                  <CardContent>
                    <Typography variant="h6" color="warning.main">
                      <FileCopyIcon sx={{ mr: 1, verticalAlign: 'middle' }} />
                      Total Duplicate Groups
                    </Typography>
                    <Typography variant="h3" color="warning.main">
                      {duplicateStatistics.total_duplicate_groups}
                    </Typography>
                  </CardContent>
                </Card>
              </Grid>
            </Grid>
          )}

          {duplicatesLoading ? (
            <Box display="flex" justifyContent="center" alignItems="center" minHeight="400px">
              <CircularProgress />
            </Box>
          ) : duplicates.length === 0 ? (
            <Alert severity="success" sx={{ mt: 2 }}>
              <AlertTitle>No duplicates found!</AlertTitle>
              You don't have any duplicate documents. All your files have unique content.
            </Alert>
          ) : (
            <>
              <Alert severity="info" sx={{ mb: 2 }}>
                <AlertTitle>Duplicate Documents Found</AlertTitle>
                These documents have identical content but may have different filenames. 
                You can expand each group to see all files with the same content and choose which ones to keep.
              </Alert>

              <Alert severity="warning" sx={{ mb: 2 }}>
                <AlertTitle>What should you do?</AlertTitle>
                <Typography variant="body2" component="div" sx={{ mt: 1, mb: 0 }}>
                  <Box component="ul" sx={{ pl: 2, mt: 0, mb: 0 }}>
                    <li><strong>Review each group:</strong> Click to expand and see all duplicate files</li>
                    <li><strong>Keep the best version:</strong> Choose the file with the most descriptive name</li>
                    <li><strong>Check content:</strong> Use View/Download to verify files are truly identical</li>
                    <li><strong>Note for admin:</strong> Consider implementing bulk delete functionality for duplicates</li>
                  </Box>
                </Typography>
              </Alert>

              <TableContainer component={Paper}>
                <Table>
                  <TableHead>
                    <TableRow>
                      <TableCell />
                      <TableCell>Content Hash</TableCell>
                      <TableCell>Duplicate Count</TableCell>
                      <TableCell>First Uploaded</TableCell>
                      <TableCell>Last Uploaded</TableCell>
                      <TableCell>Actions</TableCell>
                    </TableRow>
                  </TableHead>
                  <TableBody>
                    {duplicates.map((group) => (
                      <React.Fragment key={group.file_hash}>
                        <TableRow>
                          <TableCell>
                            <IconButton
                              size="small"
                              onClick={() => toggleDuplicateGroupExpansion(group.file_hash)}
                            >
                              {expandedDuplicateGroups.has(group.file_hash) ? <ExpandLessIcon /> : <ExpandMoreIcon />}
                            </IconButton>
                          </TableCell>
                          <TableCell>
                            <Typography variant="body2" fontFamily="monospace">
                              {group.file_hash.substring(0, 16)}...
                            </Typography>
                          </TableCell>
                          <TableCell>
                            <Chip
                              label={`${group.duplicate_count} files`}
                              color="warning"
                              size="small"
                            />
                          </TableCell>
                          <TableCell>
                            <Typography variant="body2">
                              {format(new Date(group.first_uploaded), 'MMM dd, yyyy')}
                            </Typography>
                          </TableCell>
                          <TableCell>
                            <Typography variant="body2">
                              {format(new Date(group.last_uploaded), 'MMM dd, yyyy')}
                            </Typography>
                          </TableCell>
                          <TableCell>
                            <Typography variant="body2" color="text.secondary">
                              View files below
                            </Typography>
                          </TableCell>
                        </TableRow>
                        <TableRow>
                          <TableCell sx={{ paddingBottom: 0, paddingTop: 0 }} colSpan={6}>
                            <Collapse in={expandedDuplicateGroups.has(group.file_hash)} timeout="auto" unmountOnExit>
                              <Box 
                                sx={{ 
                                  margin: 1, 
                                  p: 3,
                                  background: theme.palette.mode === 'light' 
                                    ? 'rgba(248, 250, 252, 0.8)' 
                                    : 'rgba(30, 30, 30, 0.8)',
                                  backdropFilter: 'blur(10px)',
                                  borderRadius: 2,
                                  border: `1px solid ${theme.palette.divider}`,
                                }}
                              >
                                <Typography variant="h6" gutterBottom sx={{ 
                                  color: theme.palette.primary.main,
                                  display: 'flex',
                                  alignItems: 'center',
                                  gap: 1
                                }}>
                                  <FileCopyIcon />
                                  Duplicate Files ({group.duplicate_count} total)
                                </Typography>
                                
                                <Alert severity="info" sx={{ mb: 2, fontSize: '0.875rem' }}>
                                  <strong>Storage Impact:</strong> These {group.duplicate_count} files contain identical content. 
                                  Consider keeping only the best-named version to save space.
                                </Alert>

                                <Grid container spacing={2}>
                                  {group.documents.map((doc, index) => (
                                    <Grid item xs={12} md={6} lg={4} key={doc.id}>
                                      <Card 
                                        variant="outlined"
                                        sx={{
                                          background: theme.palette.mode === 'light'
                                            ? 'rgba(255, 255, 255, 0.9)'
                                            : 'rgba(40, 40, 40, 0.9)',
                                          backdropFilter: 'blur(5px)',
                                          border: `1px solid ${theme.palette.divider}`,
                                          transition: 'all 0.2s ease',
                                          '&:hover': {
                                            transform: 'translateY(-2px)',
                                            boxShadow: theme.shadows[4],
                                          }
                                        }}
                                      >
                                        <CardContent>
                                          <Box display="flex" justifyContent="space-between" alignItems="flex-start" mb={1}>
                                            <Typography variant="body2" fontWeight="bold" sx={{ 
                                              color: theme.palette.text.primary,
                                              wordBreak: 'break-word',
                                              flex: 1,
                                              mr: 1
                                            }}>
                                              {doc.filename}
                                            </Typography>
                                            {index === 0 && (
                                              <Chip 
                                                label="First" 
                                                size="small" 
                                                color="primary" 
                                                variant="outlined"
                                              />
                                            )}
                                          </Box>
                                          
                                          {doc.original_filename !== doc.filename && (
                                            <Typography variant="caption" color="text.secondary" display="block">
                                              Original: {doc.original_filename}
                                            </Typography>
                                          )}
                                          
                                          <Typography variant="caption" display="block" color="text.secondary" sx={{ mb: 1 }}>
                                            {formatFileSize(doc.file_size)} • {doc.mime_type}
                                          </Typography>
                                          
                                          <Typography variant="caption" display="block" color="text.secondary" sx={{ mb: 2 }}>
                                            Uploaded: {format(new Date(doc.created_at), 'MMM dd, yyyy HH:mm')}
                                          </Typography>
                                          
                                          <Box display="flex" justifyContent="space-between" alignItems="center">
                                            <Box>
                                              <Tooltip title="View Document">
                                                <IconButton
                                                  size="small"
                                                  onClick={() => window.open(`/api/documents/${doc.id}/view`, '_blank')}
                                                  sx={{ color: theme.palette.primary.main }}
                                                >
                                                  <VisibilityIcon />
                                                </IconButton>
                                              </Tooltip>
                                              <Tooltip title="Download Document">
                                                <IconButton
                                                  size="small"
                                                  onClick={async () => {
                                                    try {
                                                      await documentService.downloadFile(doc.id, doc.original_filename || doc.filename);
                                                    } catch (error) {
                                                      console.error('Download failed:', error);
                                                    }
                                                  }}
                                                  sx={{ color: theme.palette.secondary.main }}
                                                >
                                                  <DownloadIcon />
                                                </IconButton>
                                              </Tooltip>
                                            </Box>
                                            
                                            <Tooltip title="Get document details and duplicate information">
                                              <Button
                                                size="small"
                                                variant="outlined"
                                                color="info"
                                                startIcon={<FindInPageIcon />}
                                                sx={{ fontSize: '0.75rem' }}
                                                onClick={() => {
                                                  setSnackbar({
                                                    open: true,
                                                    message: `Document "${doc.filename}" has ${group.duplicate_count - 1} duplicate(s). Content hash: ${group.file_hash.substring(0, 16)}...`,
                                                    severity: 'info'
                                                  });
                                                }}
                                              >
                                                Info
                                              </Button>
                                            </Tooltip>
                                          </Box>
                                        </CardContent>
                                      </Card>
                                    </Grid>
                                  ))}
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

              {/* Duplicates Pagination */}
              {duplicatesTotalPages > 1 && (
                <Box display="flex" justifyContent="center" mt={3}>
                  <Pagination
                    count={duplicatesTotalPages}
                    page={duplicatesPagination.page}
                    onChange={(_, page) => setDuplicatesPagination(prev => ({ ...prev, page }))}
                    color="primary"
                  />
                </Box>
              )}
            </>
          )}
        </>
      )}

      {/* Low Quality Manager Tab Content */}
      {currentTab === 1 && (
        <>
          <Alert severity="info" sx={{ mb: 3 }}>
            <AlertTitle>Low Confidence Document Deletion</AlertTitle>
            <Typography>
              This tool allows you to delete documents with OCR confidence below a specified threshold. 
              Use the preview feature first to see what documents would be affected before deleting.
            </Typography>
          </Alert>

          <Card sx={{ mb: 3 }}>
            <CardContent>
              <Grid container spacing={3} alignItems="center">
                <Grid item xs={12} md={4}>
                  <TextField
                    label="Confidence Threshold (%)"
                    type="number"
                    value={confidenceThreshold}
                    onChange={(e) => setConfidenceThreshold(Math.max(0, Math.min(100, Number(e.target.value))))}
                    fullWidth
                    inputProps={{ min: 0, max: 100, step: 1 }}
                    helperText="Documents with confidence below this value will be deleted"
                  />
                </Grid>
                <Grid item xs={12} md={4}>
                  <Button
                    variant="outlined"
                    onClick={handlePreviewLowConfidence}
                    disabled={lowConfidenceLoading}
                    startIcon={lowConfidenceLoading ? <CircularProgress size={20} /> : <FindInPageIcon />}
                    fullWidth
                  >
                    Preview Documents
                  </Button>
                </Grid>
                <Grid item xs={12} md={4}>
                  <Button
                    variant="contained"
                    color="warning"
                    onClick={() => setConfirmDeleteOpen(true)}
                    disabled={!previewData || previewData.matched_count === 0 || lowConfidenceLoading}
                    startIcon={<DeleteIcon />}
                    fullWidth
                  >
                    Delete Low Confidence Documents
                  </Button>
                </Grid>
              </Grid>
            </CardContent>
          </Card>

          {/* Preview Results */}
          {previewData && (
            <Card sx={{ mb: 3 }}>
              <CardContent>
                <Typography variant="h6" gutterBottom>
                  Preview Results
                </Typography>
                <Typography color={previewData.matched_count > 0 ? 'warning.main' : 'success.main'}>
                  {previewData.message}
                </Typography>
                {previewData.matched_count > 0 && previewData.documents && (
                  <Box sx={{ mt: 2 }}>
                    <Typography variant="body2" color="text.secondary" gutterBottom>
                      Documents that would be deleted:
                    </Typography>
                    <TableContainer component={Paper} variant="outlined" sx={{ mt: 2 }}>
                      <Table size="small">
                        <TableHead>
                          <TableRow>
                            <TableCell>Filename</TableCell>
                            <TableCell>Size</TableCell>
                            <TableCell>OCR Confidence</TableCell>
                            <TableCell>Status</TableCell>
                            <TableCell>Date</TableCell>
                          </TableRow>
                        </TableHead>
                        <TableBody>
                          {previewData.documents.slice(0, 20).map((doc: any) => (
                            <TableRow key={doc.id}>
                              <TableCell>
                                <Typography variant="body2" noWrap>
                                  {doc.original_filename || doc.filename}
                                </Typography>
                              </TableCell>
                              <TableCell>
                                <Typography variant="body2">
                                  {formatFileSize(doc.file_size)}
                                </Typography>
                              </TableCell>
                              <TableCell>
                                <Typography variant="body2" color={doc.ocr_confidence ? 'warning.main' : 'error.main'}>
                                  {doc.ocr_confidence ? `${doc.ocr_confidence.toFixed(1)}%` : 'N/A'}
                                </Typography>
                              </TableCell>
                              <TableCell>
                                <Chip
                                  size="small"
                                  label={doc.ocr_status || 'Unknown'}
                                  color={doc.ocr_status === 'failed' ? 'error' : 'default'}
                                />
                              </TableCell>
                              <TableCell>
                                <Typography variant="body2">
                                  {new Date(doc.created_at).toLocaleDateString()}
                                </Typography>
                              </TableCell>
                            </TableRow>
                          ))}
                        </TableBody>
                      </Table>
                    </TableContainer>
                    {previewData.documents.length > 20 && (
                      <Typography variant="body2" color="text.secondary" sx={{ mt: 1 }}>
                        ... and {previewData.documents.length - 20} more documents
                      </Typography>
                    )}
                  </Box>
                )}
              </CardContent>
            </Card>
          )}

          {/* Loading State */}
          {lowConfidenceLoading && !previewData && (
            <Box display="flex" justifyContent="center" alignItems="center" minHeight="200px">
              <CircularProgress />
              <Typography sx={{ ml: 2 }}>Processing request...</Typography>
            </Box>
          )}
        </>
      )}

      {/* Delete Failed Documents Tab Content */}
      {currentTab === 3 && (
        <>
          <Alert severity="warning" sx={{ mb: 3 }}>
            <AlertTitle>Delete Failed OCR Documents</AlertTitle>
            <Typography>
              This tool allows you to delete all documents where OCR processing failed completely. 
              This includes documents with NULL confidence values or explicit failure status.
              Use the preview feature first to see what documents would be affected before deleting.
            </Typography>
          </Alert>

          <Card sx={{ mb: 3 }}>
            <CardContent>
              <Grid container spacing={3} alignItems="center">
                <Grid item xs={12} md={6}>
                  <Button
                    variant="outlined"
                    onClick={handlePreviewFailedDocuments}
                    disabled={failedDocsLoading}
                    startIcon={failedDocsLoading ? <CircularProgress size={20} /> : <FindInPageIcon />}
                    fullWidth
                  >
                    Preview Failed Documents
                  </Button>
                </Grid>
                <Grid item xs={12} md={6}>
                  <Button
                    variant="contained"
                    color="error"
                    onClick={() => setConfirmDeleteFailedOpen(true)}
                    disabled={!failedPreviewData || failedPreviewData.matched_count === 0 || failedDocsLoading}
                    startIcon={<DeleteIcon />}
                    fullWidth
                  >
                    Delete Failed Documents
                  </Button>
                </Grid>
              </Grid>
            </CardContent>
          </Card>

          {/* Preview Results */}
          {failedPreviewData && (
            <Card sx={{ mb: 3 }}>
              <CardContent>
                <Typography variant="h6" gutterBottom>
                  Preview Results
                </Typography>
                <Typography color={failedPreviewData.matched_count > 0 ? 'error.main' : 'success.main'}>
                  {failedPreviewData.message}
                </Typography>
                {failedPreviewData.matched_count > 0 && (
                  <Box sx={{ mt: 2 }}>
                    <Typography variant="body2" color="text.secondary">
                      Document IDs that would be deleted:
                    </Typography>
                    <Typography variant="body2" sx={{ fontFamily: 'monospace', wordBreak: 'break-all' }}>
                      {failedPreviewData.document_ids.slice(0, 10).join(', ')}
                      {failedPreviewData.document_ids.length > 10 && ` ... and ${failedPreviewData.document_ids.length - 10} more`}
                    </Typography>
                  </Box>
                )}
              </CardContent>
            </Card>
          )}

          {/* Loading State */}
          {failedDocsLoading && !failedPreviewData && (
            <Box display="flex" justifyContent="center" alignItems="center" minHeight="200px">
              <CircularProgress />
              <Typography sx={{ ml: 2 }}>Processing request...</Typography>
            </Box>
          )}
        </>
      )}

      {/* Ignored Files Tab Content */}
      {currentTab === 4 && (
        <>
          <Alert severity="info" sx={{ mb: 3 }}>
            <AlertTitle>Ignored Files Management</AlertTitle>
            <Typography>
              Files that have been marked as ignored during sync operations from various sources. 
              You can remove files from the ignored list to allow them to be synced again.
            </Typography>
          </Alert>

          {/* Statistics Cards */}
          {ignoredFilesStats && (
            <Grid container spacing={3} mb={3}>
              <Grid item xs={12} md={4}>
                <Card>
                  <CardContent>
                    <Typography variant="h6" color="primary">
                      <BlockIcon sx={{ mr: 1, verticalAlign: 'middle' }} />
                      Total Ignored
                    </Typography>
                    <Typography variant="h3" color="primary.main">
                      {ignoredFilesStats.total_ignored_files}
                    </Typography>
                  </CardContent>
                </Card>
              </Grid>
              <Grid item xs={12} md={4}>
                <Card>
                  <CardContent>
                    <Typography variant="h6" color="primary">
                      Total Size
                    </Typography>
                    <Typography variant="h3" color="primary.main">
                      {formatFileSize(ignoredFilesStats.total_size_bytes)}
                    </Typography>
                  </CardContent>
                </Card>
              </Grid>
              {ignoredFilesStats.most_recent_ignored_at && (
                <Grid item xs={12} md={4}>
                  <Card>
                    <CardContent>
                      <Typography variant="h6" color="primary">
                        Most Recent
                      </Typography>
                      <Typography variant="body1" color="primary.main">
                        {format(new Date(ignoredFilesStats.most_recent_ignored_at), 'MMM dd, yyyy')}
                      </Typography>
                    </CardContent>
                  </Card>
                </Grid>
              )}
            </Grid>
          )}

          {/* Filters and Search */}
          <Card variant="outlined" sx={{ mb: 3 }}>
            <CardContent>
              <Box display="flex" gap={2} alignItems="center" flexWrap="wrap">
                <TextField
                  placeholder="Search filenames..."
                  variant="outlined"
                  size="small"
                  value={ignoredFilesSearchTerm}
                  onChange={(e) => {
                    setIgnoredFilesSearchTerm(e.target.value);
                    setIgnoredFilesPagination(prev => ({ ...prev, page: 1 }));
                  }}
                  InputProps={{
                    startAdornment: (
                      <InputAdornment position="start">
                        <FindInPageIcon />
                      </InputAdornment>
                    ),
                  }}
                  sx={{ flexGrow: 1, minWidth: '200px' }}
                />
                
                <TextField
                  select
                  label="Source Type"
                  size="small"
                  value={ignoredFilesSourceTypeFilter}
                  onChange={(e) => {
                    setIgnoredFilesSourceTypeFilter(e.target.value);
                    setIgnoredFilesPagination(prev => ({ ...prev, page: 1 }));
                  }}
                  sx={{ minWidth: '150px' }}
                >
                  <MenuItem value="">All Sources</MenuItem>
                  <MenuItem value="webdav">WebDAV</MenuItem>
                  <MenuItem value="local_folder">Local Folder</MenuItem>
                  <MenuItem value="s3">S3</MenuItem>
                </TextField>

                <Button
                  variant="outlined"
                  startIcon={<RefreshIcon />}
                  onClick={() => {
                    fetchIgnoredFiles();
                    fetchIgnoredFilesStats();
                  }}
                >
                  Refresh
                </Button>
              </Box>
            </CardContent>
          </Card>

          {/* Bulk Actions */}
          {selectedIgnoredFiles.size > 0 && (
            <Card variant="outlined" sx={{ mb: 2, bgcolor: 'action.selected' }}>
              <CardContent>
                <Box display="flex" justifyContent="space-between" alignItems="center">
                  <Typography variant="body2">
                    {selectedIgnoredFiles.size} file{selectedIgnoredFiles.size !== 1 ? 's' : ''} selected
                  </Typography>
                  <Button
                    variant="contained"
                    color="success"
                    startIcon={<RefreshIcon />}
                    onClick={() => setBulkDeleteIgnoredDialog(true)}
                    size="small"
                  >
                    Remove from Ignored List
                  </Button>
                </Box>
              </CardContent>
            </Card>
          )}

          {ignoredFilesLoading ? (
            <Box display="flex" justifyContent="center" alignItems="center" minHeight="400px">
              <CircularProgress />
            </Box>
          ) : ignoredFiles.length === 0 ? (
            <Alert severity="success" sx={{ mt: 2 }}>
              <AlertTitle>No ignored files found!</AlertTitle>
              You don't have any files in the ignored list. All your files are being processed normally.
            </Alert>
          ) : (
            <>
              <TableContainer component={Paper}>
                <Table>
                  <TableHead>
                    <TableRow>
                      <TableCell padding="checkbox">
                        <Checkbox
                          indeterminate={selectedIgnoredFiles.size > 0 && selectedIgnoredFiles.size < ignoredFiles.length}
                          checked={ignoredFiles.length > 0 && selectedIgnoredFiles.size === ignoredFiles.length}
                          onChange={handleIgnoredFilesSelectAll}
                        />
                      </TableCell>
                      <TableCell>Filename</TableCell>
                      <TableCell>Source</TableCell>
                      <TableCell>Size</TableCell>
                      <TableCell>Ignored Date</TableCell>
                      <TableCell>Reason</TableCell>
                      <TableCell>Actions</TableCell>
                    </TableRow>
                  </TableHead>
                  <TableBody>
                    {ignoredFiles.map((file) => (
                      <TableRow key={file.id} hover>
                        <TableCell padding="checkbox">
                          <Checkbox
                            checked={selectedIgnoredFiles.has(file.id)}
                            onChange={() => handleIgnoredFileSelect(file.id)}
                          />
                        </TableCell>
                        <TableCell>
                          <Box>
                            <Typography variant="body2" fontWeight="medium">
                              {file.filename}
                            </Typography>
                            {file.filename !== file.original_filename && (
                              <Typography variant="caption" color="text.secondary">
                                Original: {file.original_filename}
                              </Typography>
                            )}
                            <Typography variant="caption" color="text.secondary" display="block">
                              {file.mime_type}
                            </Typography>
                          </Box>
                        </TableCell>
                        <TableCell>
                          <Box display="flex" alignItems="center" gap={1}>
                            {getSourceIcon(file.source_type)}
                            <Box>
                              <Typography variant="body2">
                                {getSourceTypeDisplay(file.source_type)}
                              </Typography>
                              {file.source_path && (
                                <Typography variant="caption" color="text.secondary">
                                  {file.source_path}
                                </Typography>
                              )}
                            </Box>
                          </Box>
                        </TableCell>
                        <TableCell>
                          <Typography variant="body2">
                            {formatFileSize(file.file_size)}
                          </Typography>
                        </TableCell>
                        <TableCell>
                          <Typography variant="body2">
                            {format(new Date(file.ignored_at), 'MMM dd, yyyy')}
                          </Typography>
                          <Typography variant="caption" color="text.secondary">
                            {format(new Date(file.ignored_at), 'HH:mm')}
                          </Typography>
                        </TableCell>
                        <TableCell>
                          <Typography variant="body2">
                            {file.reason || 'No reason provided'}
                          </Typography>
                        </TableCell>
                        <TableCell>
                          <Tooltip title="Remove from ignored list (allow re-syncing)">
                            <IconButton
                              size="small"
                              onClick={() => handleDeleteSingleIgnoredFile(file.id)}
                              color="success"
                            >
                              <RefreshIcon fontSize="small" />
                            </IconButton>
                          </Tooltip>
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </TableContainer>

              {/* Pagination */}
              {ignoredFilesTotalPages > 1 && (
                <Box display="flex" justifyContent="center" mt={3}>
                  <Pagination
                    count={ignoredFilesTotalPages}
                    page={ignoredFilesPagination.page}
                    onChange={(_, page) => setIgnoredFilesPagination(prev => ({ ...prev, page }))}
                    color="primary"
                  />
                </Box>
              )}
            </>
          )}
        </>
      )}

      {/* Confirmation Dialog */}
      <Dialog
        open={confirmDeleteOpen}
        onClose={() => setConfirmDeleteOpen(false)}
        maxWidth="sm"
        fullWidth
      >
        <DialogTitle color="warning.main">
          <DeleteIcon sx={{ mr: 1, verticalAlign: 'middle' }} />
          Confirm Low Confidence Document Deletion
        </DialogTitle>
        <DialogContent>
          <Typography>
            Are you sure you want to delete {previewData?.matched_count || 0} documents with OCR confidence below {confidenceThreshold}%?
          </Typography>
          <Alert severity="warning" sx={{ mt: 2 }}>
            This action cannot be undone. The documents and their files will be permanently deleted.
          </Alert>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setConfirmDeleteOpen(false)}>
            Cancel
          </Button>
          <Button
            onClick={handleDeleteLowConfidence}
            color="warning"
            variant="contained"
            disabled={lowConfidenceLoading}
            startIcon={lowConfidenceLoading ? <CircularProgress size={20} /> : <DeleteIcon />}
          >
            {lowConfidenceLoading ? 'Deleting...' : 'Delete Documents'}
          </Button>
        </DialogActions>
      </Dialog>

      {/* Confirmation Dialog for Failed Documents */}
      <Dialog
        open={confirmDeleteFailedOpen}
        onClose={() => setConfirmDeleteFailedOpen(false)}
        maxWidth="sm"
        fullWidth
      >
        <DialogTitle color="error.main">
          <DeleteIcon sx={{ mr: 1, verticalAlign: 'middle' }} />
          Confirm Failed Document Deletion
        </DialogTitle>
        <DialogContent>
          <Typography>
            Are you sure you want to delete {failedPreviewData?.matched_count || 0} documents with failed OCR processing?
          </Typography>
          <Alert severity="error" sx={{ mt: 2 }}>
            This action cannot be undone. The documents and their files will be permanently deleted.
          </Alert>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setConfirmDeleteFailedOpen(false)}>
            Cancel
          </Button>
          <Button
            onClick={handleDeleteFailedDocuments}
            color="error"
            variant="contained"
            disabled={failedDocsLoading}
            startIcon={failedDocsLoading ? <CircularProgress size={20} /> : <DeleteIcon />}
          >
            {failedDocsLoading ? 'Deleting...' : 'Delete Failed Documents'}
          </Button>
        </DialogActions>
      </Dialog>

      {/* Document Details Dialog */}
      <Dialog
        open={detailsOpen}
        onClose={() => setDetailsOpen(false)}
        maxWidth="lg"
        fullWidth
      >
        <DialogTitle>
          Document Details: {selectedDocument?.filename}
        </DialogTitle>
        <DialogContent>
          {selectedDocument && (
            <Grid container spacing={3}>
              {/* File Preview Section */}
              <Grid item xs={12} md={6}>
                <Typography variant="h6" sx={{ mb: 2 }}>
                  File Preview
                </Typography>
                <Box
                  onClick={() => {
                    if (selectedDocument) {
                      navigate(`/documents/${selectedDocument.id}`);
                    }
                  }}
                  sx={{
                    cursor: 'pointer',
                    border: '2px dashed',
                    borderColor: 'primary.main',
                    borderRadius: 2,
                    p: 1,
                    transition: 'all 0.2s ease-in-out',
                    '&:hover': {
                      borderColor: 'primary.dark',
                      boxShadow: 2,
                    },
                  }}
                >
                  <FailedDocumentViewer
                    failedDocumentId={selectedDocument.id}
                    filename={selectedDocument.original_filename}
                    mimeType={selectedDocument.mime_type}
                  />
                  <Box sx={{ mt: 1, textAlign: 'center' }}>
                    <Typography variant="caption" color="primary.main">
                      Click to open full document details page
                    </Typography>
                  </Box>
                </Box>
              </Grid>

              {/* Document Information Section */}
              <Grid item xs={12} md={6}>
                <Typography variant="h6" sx={{ mb: 2 }}>
                  Document Information
                </Typography>
                <Box>
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
                    <strong>Failure Category:</strong>
                  </Typography>
                  <Chip
                    label={selectedDocument.failure_category}
                    color={getFailureCategoryColor(selectedDocument.failure_category)}
                    sx={{ mb: 2 }}
                  />

                  <Typography variant="body2" color="text.secondary" sx={{ mt: 2 }}>
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

                  <Typography variant="body2" color="text.secondary" sx={{ mt: 2 }}>
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
                </Box>
              </Grid>

              {/* Error Details Section */}
              <Grid item xs={12}>
                <Divider sx={{ my: 2 }} />
                <Typography variant="h6" sx={{ mb: 2 }}>
                  Error Details
                </Typography>
                <Typography variant="body2" color="text.secondary" sx={{ mb: 1 }}>
                  <strong>Full Error Message:</strong>
                </Typography>
                <Paper sx={{ 
                  p: 2, 
                  bgcolor: (theme) => theme.palette.mode === 'dark' ? 'grey.800' : 'grey.50',
                  borderRadius: 1
                }}>
                  <Typography
                    variant="body2"
                    sx={{
                      fontFamily: 'monospace',
                      fontSize: '0.875rem',
                      wordBreak: 'break-word',
                      whiteSpace: 'pre-wrap'
                    }}
                  >
                    {selectedDocument.error_message || selectedDocument.ocr_error || 'No error message available'}
                  </Typography>
                </Paper>
              </Grid>
            </Grid>
          )}
        </DialogContent>
        <DialogActions>
          <Button
            onClick={() => {
              if (selectedDocument) {
                // For failed documents, we need to create a special route or disable this feature
                // since the document might not exist in the main documents table
                setSnackbar({
                  open: true,
                  message: 'Failed documents are only available in this management view. The document may not exist in the main documents table.',
                  severity: 'info'
                });
              }
            }}
            startIcon={<InfoIcon />}
            color="secondary"
            disabled
          >
            Document Info Only
          </Button>
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

      {/* Bulk Delete Ignored Files Confirmation Dialog */}
      <Dialog open={bulkDeleteIgnoredDialog} onClose={() => setBulkDeleteIgnoredDialog(false)}>
        <DialogTitle>Confirm Bulk Delete</DialogTitle>
        <DialogContent>
          <Typography>
            Are you sure you want to remove {selectedIgnoredFiles.size} file{selectedIgnoredFiles.size !== 1 ? 's' : ''} from the ignored list?
          </Typography>
          <Typography variant="body2" color="text.secondary" sx={{ mt: 1 }}>
            These files will be eligible for syncing again if encountered from their sources. This action allows them to be re-imported during future syncs.
          </Typography>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setBulkDeleteIgnoredDialog(false)}>Cancel</Button>
          <Button
            onClick={handleDeleteSelectedIgnoredFiles}
            color="success"
            variant="contained"
            disabled={deletingIgnoredFiles}
            startIcon={deletingIgnoredFiles ? <CircularProgress size={16} /> : <RefreshIcon />}
          >
            Remove from Ignored List
          </Button>
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

export default DocumentManagementPage;