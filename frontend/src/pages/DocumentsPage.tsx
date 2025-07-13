import React, { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Box,
  Typography,
  Card,
  CardContent,
  CardActions,
  Button,
  Chip,
  IconButton,
  ToggleButton,
  ToggleButtonGroup,
  TextField,
  InputAdornment,
  Stack,
  Menu,
  MenuItem,
  ListItemIcon,
  ListItemText,
  Divider,
  CircularProgress,
  Alert,
  Pagination,
  FormControl,
  InputLabel,
  Select,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  Checkbox,
  Fab,
  Tooltip,
} from '@mui/material';
import Grid from '@mui/material/GridLegacy';
import {
  GridView as GridViewIcon,
  ViewList as ListViewIcon,
  Search as SearchIcon,
  FilterList as FilterIcon,
  Sort as SortIcon,
  Download as DownloadIcon,
  PictureAsPdf as PdfIcon,
  Image as ImageIcon,
  Description as DocIcon,
  TextSnippet as TextIcon,
  MoreVert as MoreIcon,
  CalendarToday as DateIcon,
  Storage as SizeIcon,
  Visibility as ViewIcon,
  ChevronLeft as ChevronLeftIcon,
  ChevronRight as ChevronRightIcon,
  Edit as EditIcon,
  Delete as DeleteIcon,
  CheckBoxOutlineBlank as CheckBoxOutlineBlankIcon,
  CheckBox as CheckBoxIcon,
  SelectAll as SelectAllIcon,
  Close as CloseIcon,
  Refresh as RefreshIcon,
  History as HistoryIcon,
} from '@mui/icons-material';
import { documentService } from '../services/api';
import DocumentThumbnail from '../components/DocumentThumbnail';
import Label, { type LabelData } from '../components/Labels/Label';
import LabelSelector from '../components/Labels/LabelSelector';
import { useApi } from '../hooks/useApi';
import { RetryHistoryModal } from '../components/RetryHistoryModal';

interface Document {
  id: string;
  original_filename: string;
  filename?: string;
  file_size: number;
  mime_type: string;
  created_at: string;
  has_ocr_text?: boolean;
  ocr_status?: string;
  ocr_confidence?: number;
  tags: string[];
  labels?: LabelData[];
}

interface PaginationInfo {
  total: number;
  limit: number;
  offset: number;
  has_more: boolean;
}

interface DocumentsResponse {
  documents: Document[];
  pagination: PaginationInfo;
}

type ViewMode = 'grid' | 'list';
type SortField = 'created_at' | 'original_filename' | 'file_size';
type SortOrder = 'asc' | 'desc';

const DocumentsPage: React.FC = () => {
  const navigate = useNavigate();
  const api = useApi();
  const [documents, setDocuments] = useState<Document[]>([]);
  const [pagination, setPagination] = useState<PaginationInfo>({ total: 0, limit: 20, offset: 0, has_more: false });
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);
  const [viewMode, setViewMode] = useState<ViewMode>('grid');
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [sortBy, setSortBy] = useState<SortField>('created_at');
  const [sortOrder, setSortOrder] = useState<SortOrder>('desc');
  const [ocrFilter, setOcrFilter] = useState<string>('');
  
  // Labels state
  const [availableLabels, setAvailableLabels] = useState<LabelData[]>([]);
  const [labelsLoading, setLabelsLoading] = useState<boolean>(false);
  const [labelEditDialogOpen, setLabelEditDialogOpen] = useState<boolean>(false);
  const [editingDocumentId, setEditingDocumentId] = useState<string | null>(null);
  const [editingDocumentLabels, setEditingDocumentLabels] = useState<LabelData[]>([]);
  
  // Menu states
  const [sortMenuAnchor, setSortMenuAnchor] = useState<null | HTMLElement>(null);
  const [docMenuAnchor, setDocMenuAnchor] = useState<null | HTMLElement>(null);
  const [selectedDoc, setSelectedDoc] = useState<Document | null>(null);
  
  // Delete confirmation dialog state
  const [deleteDialogOpen, setDeleteDialogOpen] = useState<boolean>(false);
  const [documentToDelete, setDocumentToDelete] = useState<Document | null>(null);
  const [deleteLoading, setDeleteLoading] = useState<boolean>(false);

  // Mass selection state
  const [selectionMode, setSelectionMode] = useState<boolean>(false);
  const [selectedDocuments, setSelectedDocuments] = useState<Set<string>>(new Set());
  const [bulkDeleteDialogOpen, setBulkDeleteDialogOpen] = useState<boolean>(false);
  const [bulkDeleteLoading, setBulkDeleteLoading] = useState<boolean>(false);

  // Retry functionality state
  const [retryingDocument, setRetryingDocument] = useState<string | null>(null);
  const [retryHistoryModalOpen, setRetryHistoryModalOpen] = useState<boolean>(false);
  const [selectedDocumentForHistory, setSelectedDocumentForHistory] = useState<string | null>(null);

  useEffect(() => {
    fetchDocuments();
    fetchLabels();
  }, [pagination?.limit, pagination?.offset, ocrFilter]);

  const fetchDocuments = async (): Promise<void> => {
    if (!pagination) return;
    
    try {
      setLoading(true);
      const response = await documentService.listWithPagination(
        pagination.limit, 
        pagination.offset, 
        ocrFilter || undefined
      );
      // Backend returns wrapped object with documents and pagination
      setDocuments(response.data.documents || []);
      setPagination(response.data.pagination || { total: 0, limit: 20, offset: 0, has_more: false });
    } catch (err) {
      setError('Failed to load documents');
      console.error(err);
    } finally {
      setLoading(false);
    }
  };

  const fetchLabels = async (): Promise<void> => {
    try {
      setLabelsLoading(true);
      const response = await api.get('/labels?include_counts=false');
      
      if (response.status === 200 && Array.isArray(response.data)) {
        setAvailableLabels(response.data);
      } else {
        console.error('Failed to fetch labels:', response);
      }
    } catch (error) {
      console.error('Failed to fetch labels:', error);
    } finally {
      setLabelsLoading(false);
    }
  };

  const handleCreateLabel = async (labelData: Omit<LabelData, 'id' | 'is_system' | 'created_at' | 'updated_at' | 'document_count' | 'source_count'>) => {
    try {
      const response = await api.post('/labels', labelData);
      const newLabel = response.data;
      setAvailableLabels(prev => [...prev, newLabel]);
      return newLabel;
    } catch (error) {
      console.error('Failed to create label:', error);
      throw error;
    }
  };

  const handleEditDocumentLabels = (doc: Document) => {
    setEditingDocumentId(doc.id);
    setEditingDocumentLabels(doc.labels || []);
    setLabelEditDialogOpen(true);
  };

  const handleSaveDocumentLabels = async () => {
    if (!editingDocumentId) return;

    try {
      const labelIds = editingDocumentLabels.map(label => label.id);
      await api.put(`/labels/documents/${editingDocumentId}`, { label_ids: labelIds });
      
      // Update the document in the local state
      setDocuments(prev => prev.map(doc => 
        doc.id === editingDocumentId 
          ? { ...doc, labels: editingDocumentLabels }
          : doc
      ));
      
      setLabelEditDialogOpen(false);
      setEditingDocumentId(null);
      setEditingDocumentLabels([]);
    } catch (error) {
      console.error('Failed to update document labels:', error);
    }
  };

  const handleDownload = async (doc: Document): Promise<void> => {
    try {
      const response = await documentService.download(doc.id);
      const url = window.URL.createObjectURL(new Blob([response.data]));
      const link = document.createElement('a');
      link.href = url;
      link.setAttribute('download', doc.original_filename);
      document.body.appendChild(link);
      link.click();
      link.remove();
      window.URL.revokeObjectURL(url);
    } catch (err) {
      console.error('Download failed:', err);
    }
  };

  const getFileIcon = (mimeType: string): React.ReactElement => {
    if (mimeType.includes('pdf')) return <PdfIcon color="error" />;
    if (mimeType.includes('image')) return <ImageIcon color="primary" />;
    if (mimeType.includes('text')) return <TextIcon color="info" />;
    return <DocIcon color="secondary" />;
  };

  const formatFileSize = (bytes: number): string => {
    const sizes = ['Bytes', 'KB', 'MB', 'GB'];
    if (bytes === 0) return '0 Bytes';
    const i = Math.floor(Math.log(bytes) / Math.log(1024));
    return Math.round(bytes / Math.pow(1024, i) * 100) / 100 + ' ' + sizes[i];
  };

  const formatDate = (dateString: string): string => {
    return new Date(dateString).toLocaleDateString('en-US', {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  const filteredDocuments = (documents || []).filter(doc =>
    doc.original_filename.toLowerCase().includes(searchQuery.toLowerCase()) ||
    doc.tags.some(tag => tag.toLowerCase().includes(searchQuery.toLowerCase()))
  );

  const sortedDocuments = [...filteredDocuments].sort((a, b) => {
    let aValue: any = a[sortBy];
    let bValue: any = b[sortBy];
    
    if (sortBy === 'created_at') {
      aValue = new Date(aValue);
      bValue = new Date(bValue);
    }
    
    if (sortOrder === 'asc') {
      return aValue > bValue ? 1 : -1;
    } else {
      return aValue < bValue ? 1 : -1;
    }
  });

  const handleViewModeChange = (event: React.MouseEvent<HTMLElement>, newView: ViewMode | null): void => {
    if (newView) {
      setViewMode(newView);
    }
  };

  const handleSortMenuClick = (event: React.MouseEvent<HTMLElement>): void => {
    setSortMenuAnchor(event.currentTarget);
  };

  const handleDocMenuClick = (event: React.MouseEvent<HTMLElement>, doc: Document): void => {
    setSelectedDoc(doc);
    setDocMenuAnchor(event.currentTarget);
  };

  const handleSortMenuClose = (): void => {
    setSortMenuAnchor(null);
  };

  const handleDocMenuClose = (): void => {
    setDocMenuAnchor(null);
  };

  const handleSortChange = (field: SortField, order: SortOrder): void => {
    setSortBy(field);
    setSortOrder(order);
    handleSortMenuClose();
  };

  const handleDeleteClick = (doc: Document): void => {
    setDocumentToDelete(doc);
    setDeleteDialogOpen(true);
    handleDocMenuClose();
  };

  const handleDeleteConfirm = async (): Promise<void> => {
    if (!documentToDelete) return;

    try {
      setDeleteLoading(true);
      await documentService.delete(documentToDelete.id);
      
      setDocuments(prev => prev.filter(doc => doc.id !== documentToDelete.id));
      setPagination(prev => ({ ...prev, total: prev.total - 1 }));
      
      setDeleteDialogOpen(false);
      setDocumentToDelete(null);
    } catch (error) {
      console.error('Failed to delete document:', error);
      setError('Failed to delete document');
    } finally {
      setDeleteLoading(false);
    }
  };

  const handleDeleteCancel = (): void => {
    setDeleteDialogOpen(false);
    setDocumentToDelete(null);
  };

  // Retry functionality handlers
  const handleRetryOcr = async (doc: Document): Promise<void> => {
    try {
      setRetryingDocument(doc.id);
      await documentService.bulkRetryOcr({
        mode: 'specific',
        document_ids: [doc.id],
        priority_override: 15,
      });
      
      // Refresh the document list to get updated status
      await fetchDocuments();
      
      setError(null);
    } catch (error) {
      console.error('Failed to retry OCR:', error);
      setError('Failed to retry OCR processing');
    } finally {
      setRetryingDocument(null);
      handleDocMenuClose();
    }
  };

  const handleShowRetryHistory = (docId: string): void => {
    setSelectedDocumentForHistory(docId);
    setRetryHistoryModalOpen(true);
    handleDocMenuClose();
  };

  const handlePageChange = (event: React.ChangeEvent<unknown>, page: number): void => {
    const newOffset = (page - 1) * pagination.limit;
    setPagination(prev => ({ ...prev, offset: newOffset }));
  };

  const handleOcrFilterChange = (event: React.ChangeEvent<HTMLInputElement>): void => {
    setOcrFilter(event.target.value);
    setPagination(prev => ({ ...prev, offset: 0 })); // Reset to first page when filtering
  };

  // Mass selection handlers
  const handleToggleSelectionMode = (): void => {
    setSelectionMode(!selectionMode);
    setSelectedDocuments(new Set());
  };

  const handleDocumentSelect = (documentId: string, isSelected: boolean): void => {
    const newSelection = new Set(selectedDocuments);
    if (isSelected) {
      newSelection.add(documentId);
    } else {
      newSelection.delete(documentId);
    }
    setSelectedDocuments(newSelection);
  };

  const handleSelectAll = (): void => {
    if (selectedDocuments.size === sortedDocuments.length) {
      setSelectedDocuments(new Set());
    } else {
      setSelectedDocuments(new Set(sortedDocuments.map(doc => doc.id)));
    }
  };

  const handleBulkDelete = (): void => {
    if (selectedDocuments.size === 0) return;
    setBulkDeleteDialogOpen(true);
  };

  const handleBulkDeleteConfirm = async (): Promise<void> => {
    if (selectedDocuments.size === 0) return;

    try {
      setBulkDeleteLoading(true);
      const documentIds = Array.from(selectedDocuments);
      await documentService.bulkDelete(documentIds);
      
      setDocuments(prev => prev.filter(doc => !selectedDocuments.has(doc.id)));
      setPagination(prev => ({ ...prev, total: prev.total - selectedDocuments.size }));
      
      setSelectedDocuments(new Set());
      setSelectionMode(false);
      setBulkDeleteDialogOpen(false);
    } catch (error) {
      console.error('Failed to delete documents:', error);
      setError('Failed to delete selected documents');
    } finally {
      setBulkDeleteLoading(false);
    }
  };

  const handleBulkDeleteCancel = (): void => {
    setBulkDeleteDialogOpen(false);
  };

  const getOcrStatusChip = (doc: Document) => {
    if (!doc.ocr_status) return null;
    
    const statusConfig = {
      'completed': { color: 'success' as const, label: doc.ocr_confidence ? `OCR ${Math.round(doc.ocr_confidence)}%` : 'OCR Done' },
      'processing': { color: 'warning' as const, label: 'Processing...' },
      'failed': { color: 'error' as const, label: 'OCR Failed' },
      'pending': { color: 'default' as const, label: 'Pending' },
    };
    
    const config = statusConfig[doc.ocr_status as keyof typeof statusConfig];
    if (!config) return null;
    
    return (
      <Chip 
        label={config.label}
        size="small" 
        color={config.color}
        variant="outlined"
      />
    );
  };

  if (loading) {
    return (
      <Box display="flex" justifyContent="center" alignItems="center" minHeight="400px">
        <CircularProgress />
      </Box>
    );
  }

  if (error) {
    return (
      <Alert severity="error" sx={{ m: 2 }}>
        {error}
      </Alert>
    );
  }

  return (
    <Box sx={{ p: 3 }}>
      {/* Header */}
      <Box sx={{ mb: 4 }}>
        <Typography 
          variant="h4" 
          sx={{ 
            fontWeight: 800,
            background: 'linear-gradient(135deg, #6366f1 0%, #8b5cf6 100%)',
            backgroundClip: 'text',
            WebkitBackgroundClip: 'text',
            color: 'transparent',
            mb: 1,
          }}
        >
          Documents
        </Typography>
        <Typography variant="body1" color="text.secondary">
          Manage and explore your document library
        </Typography>
      </Box>

      {/* Toolbar */}
      <Box sx={{ 
        mb: 3, 
        display: 'flex', 
        gap: 2, 
        alignItems: 'center',
        flexWrap: 'wrap',
      }}>
        {/* Search */}
        <TextField
          placeholder="Search documents..."
          variant="outlined"
          size="small"
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          InputProps={{
            startAdornment: (
              <InputAdornment position="start">
                <SearchIcon color="action" />
              </InputAdornment>
            ),
          }}
          sx={{ minWidth: 300, flexGrow: 1 }}
        />

        {/* View Toggle */}
        <ToggleButtonGroup
          value={viewMode}
          exclusive
          onChange={handleViewModeChange}
          size="small"
        >
          <ToggleButton value="grid">
            <GridViewIcon />
          </ToggleButton>
          <ToggleButton value="list">
            <ListViewIcon />
          </ToggleButton>
        </ToggleButtonGroup>

        {/* Selection Mode Toggle */}
        <Button
          variant={selectionMode ? "contained" : "outlined"}
          startIcon={selectionMode ? <CloseIcon /> : <CheckBoxOutlineBlankIcon />}
          onClick={handleToggleSelectionMode}
          size="small"
          color={selectionMode ? "secondary" : "primary"}
        >
          {selectionMode ? 'Cancel' : 'Select'}
        </Button>

        {/* OCR Filter */}
        <FormControl size="small" sx={{ minWidth: 120 }}>
          <InputLabel>OCR Status</InputLabel>
          <Select
            value={ocrFilter}
            label="OCR Status"
            onChange={handleOcrFilterChange}
          >
            <MenuItem value="">All</MenuItem>
            <MenuItem value="completed">Completed</MenuItem>
            <MenuItem value="processing">Processing</MenuItem>
            <MenuItem value="failed">Failed</MenuItem>
            <MenuItem value="pending">Pending</MenuItem>
          </Select>
        </FormControl>

        {/* Sort Button */}
        <Button
          variant="outlined"
          startIcon={<SortIcon />}
          onClick={handleSortMenuClick}
          size="small"
        >
          Sort
        </Button>
      </Box>

      {/* Selection Toolbar */}
      {selectionMode && (
        <Box sx={{ 
          mb: 2, 
          p: 2, 
          bgcolor: 'primary.light',
          borderRadius: 1,
          display: 'flex',
          alignItems: 'center',
          gap: 2,
          color: 'primary.contrastText'
        }}>
          <Typography variant="body2" sx={{ flexGrow: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
            {selectedDocuments.size > 999 ? `${Math.floor(selectedDocuments.size/1000)}K` : selectedDocuments.size} of {sortedDocuments.length > 999 ? `${Math.floor(sortedDocuments.length/1000)}K` : sortedDocuments.length} documents selected
          </Typography>
          <Button
            variant="text"
            startIcon={selectedDocuments.size === sortedDocuments.length ? <CheckBoxIcon /> : <CheckBoxOutlineBlankIcon />}
            onClick={handleSelectAll}
            size="small"
            sx={{ color: 'primary.contrastText' }}
          >
            {selectedDocuments.size === sortedDocuments.length ? 'Deselect All' : 'Select All'}
          </Button>
          <Button
            variant="contained"
            startIcon={<DeleteIcon />}
            onClick={handleBulkDelete}
            disabled={selectedDocuments.size === 0}
            size="small"
            color="error"
          >
            Delete Selected ({selectedDocuments.size > 999 ? `${Math.floor(selectedDocuments.size/1000)}K` : selectedDocuments.size})
          </Button>
        </Box>
      )}

      {/* Sort Menu */}
      <Menu
        anchorEl={sortMenuAnchor}
        open={Boolean(sortMenuAnchor)}
        onClose={handleSortMenuClose}
      >
        <MenuItem onClick={() => handleSortChange('created_at', 'desc')}>
          <ListItemIcon><DateIcon fontSize="small" /></ListItemIcon>
          <ListItemText>Newest First</ListItemText>
        </MenuItem>
        <MenuItem onClick={() => handleSortChange('created_at', 'asc')}>
          <ListItemIcon><DateIcon fontSize="small" /></ListItemIcon>
          <ListItemText>Oldest First</ListItemText>
        </MenuItem>
        <MenuItem onClick={() => handleSortChange('original_filename', 'asc')}>
          <ListItemIcon><TextIcon fontSize="small" /></ListItemIcon>
          <ListItemText>Name A-Z</ListItemText>
        </MenuItem>
        <MenuItem onClick={() => handleSortChange('original_filename', 'desc')}>
          <ListItemIcon><TextIcon fontSize="small" /></ListItemIcon>
          <ListItemText>Name Z-A</ListItemText>
        </MenuItem>
        <MenuItem onClick={() => handleSortChange('file_size', 'desc')}>
          <ListItemIcon><SizeIcon fontSize="small" /></ListItemIcon>
          <ListItemText>Largest First</ListItemText>
        </MenuItem>
        <MenuItem onClick={() => handleSortChange('file_size', 'asc')}>
          <ListItemIcon><SizeIcon fontSize="small" /></ListItemIcon>
          <ListItemText>Smallest First</ListItemText>
        </MenuItem>
      </Menu>

      {/* Document Menu */}
      <Menu
        anchorEl={docMenuAnchor}
        open={Boolean(docMenuAnchor)}
        onClose={handleDocMenuClose}
      >
        <MenuItem onClick={() => { 
          if (selectedDoc) handleDownload(selectedDoc); 
          handleDocMenuClose(); 
        }}>
          <ListItemIcon><DownloadIcon fontSize="small" /></ListItemIcon>
          <ListItemText>Download</ListItemText>
        </MenuItem>
        <MenuItem onClick={() => { 
          if (selectedDoc) navigate(`/documents/${selectedDoc.id}`); 
          handleDocMenuClose(); 
        }}>
          <ListItemIcon><ViewIcon fontSize="small" /></ListItemIcon>
          <ListItemText>View Details</ListItemText>
        </MenuItem>
        <MenuItem onClick={() => { 
          if (selectedDoc) handleEditDocumentLabels(selectedDoc); 
          handleDocMenuClose(); 
        }}>
          <ListItemIcon><EditIcon fontSize="small" /></ListItemIcon>
          <ListItemText>Edit Labels</ListItemText>
        </MenuItem>
        <Divider />
        <MenuItem onClick={() => { 
          if (selectedDoc) handleRetryOcr(selectedDoc); 
        }} disabled={retryingDocument === selectedDoc?.id}>
          <ListItemIcon>
            {retryingDocument === selectedDoc?.id ? (
              <CircularProgress size={16} />
            ) : (
              <RefreshIcon fontSize="small" />
            )}
          </ListItemIcon>
          <ListItemText>
            {retryingDocument === selectedDoc?.id ? 'Retrying OCR...' : 'Retry OCR'}
          </ListItemText>
        </MenuItem>
        <MenuItem onClick={() => { 
          if (selectedDoc) handleShowRetryHistory(selectedDoc.id); 
        }}>
          <ListItemIcon><HistoryIcon fontSize="small" /></ListItemIcon>
          <ListItemText>Retry History</ListItemText>
        </MenuItem>
        <Divider />
        <MenuItem onClick={() => { 
          if (selectedDoc) handleDeleteClick(selectedDoc); 
        }}>
          <ListItemIcon><DeleteIcon fontSize="small" color="error" /></ListItemIcon>
          <ListItemText>Delete</ListItemText>
        </MenuItem>
      </Menu>

      {/* Documents Grid/List */}
      {sortedDocuments.length === 0 ? (
        <Box 
          sx={{ 
            textAlign: 'center', 
            py: 8,
            background: 'linear-gradient(135deg, rgba(99, 102, 241, 0.05) 0%, rgba(139, 92, 246, 0.05) 100%)',
            borderRadius: 2,
            border: '1px dashed',
            borderColor: 'primary.main',
          }}
        >
          <Typography variant="h6" color="text.secondary" gutterBottom>
            No documents found
          </Typography>
          <Typography variant="body2" color="text.secondary">
            {searchQuery ? 'Try adjusting your search terms' : 'Upload your first document to get started'}
          </Typography>
        </Box>
      ) : (
        <Grid container spacing={viewMode === 'grid' ? 3 : 1}>
          {sortedDocuments.map((doc) => (
            <Grid 
              item 
              xs={12} 
              sm={viewMode === 'grid' ? 6 : 12} 
              md={viewMode === 'grid' ? 4 : 12} 
              lg={viewMode === 'grid' ? 3 : 12}
              key={doc.id}
            >
              <Card 
                sx={{ 
                  height: '100%',
                  display: 'flex',
                  flexDirection: viewMode === 'list' ? 'row' : 'column',
                  transition: 'all 0.2s ease-in-out',
                  cursor: 'pointer',
                  position: 'relative',
                  '&:hover': {
                    transform: 'translateY(-4px)',
                    boxShadow: (theme) => theme.shadows[4],
                  },
                  ...(selectionMode && selectedDocuments.has(doc.id) && {
                    boxShadow: (theme) => `0 0 0 2px ${theme.palette.primary.main}`,
                    bgcolor: 'primary.light',
                  }),
                }}
                onClick={(e) => {
                  if (selectionMode) {
                    e.stopPropagation();
                    handleDocumentSelect(doc.id, !selectedDocuments.has(doc.id));
                  } else {
                    navigate(`/documents/${doc.id}`);
                  }
                }}
              >
                {/* Selection checkbox */}
                {selectionMode && (
                  <Box
                    sx={{
                      position: 'absolute',
                      top: 8,
                      right: 8,
                      zIndex: 1,
                      bgcolor: 'background.paper',
                      borderRadius: '50%',
                      boxShadow: 1,
                    }}
                    onClick={(e) => {
                      e.stopPropagation();
                      handleDocumentSelect(doc.id, !selectedDocuments.has(doc.id));
                    }}
                  >
                    <Checkbox
                      checked={selectedDocuments.has(doc.id)}
                      size="small"
                      color="primary"
                    />
                  </Box>
                )}

                {viewMode === 'grid' && (
                  <Box
                    sx={{
                      height: 120,
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      background: 'linear-gradient(135deg, rgba(99, 102, 241, 0.1) 0%, rgba(139, 92, 246, 0.1) 100%)',
                    }}
                  >
                    <DocumentThumbnail
                      documentId={doc.id}
                      mimeType={doc.mime_type}
                      size="large"
                    />
                  </Box>
                )}
                
                <CardContent sx={{ flexGrow: 1, pb: 1 }}>
                  <Box sx={{ display: 'flex', alignItems: 'flex-start', gap: 1 }}>
                    {viewMode === 'list' && (
                      <Box sx={{ mr: 1, mt: 0.5 }}>
                        <DocumentThumbnail
                          documentId={doc.id}
                          mimeType={doc.mime_type}
                          size="small"
                        />
                      </Box>
                    )}
                    
                    <Box sx={{ flexGrow: 1, minWidth: 0 }}>
                      <Typography 
                        variant="h6" 
                        sx={{ 
                          fontSize: '1rem',
                          fontWeight: 600,
                          mb: 1,
                          overflow: 'hidden',
                          textOverflow: 'ellipsis',
                          whiteSpace: 'nowrap',
                        }}
                        title={doc.original_filename}
                      >
                        {doc.original_filename}
                      </Typography>
                      
                      <Stack direction="row" spacing={1} sx={{ mb: 1, flexWrap: 'wrap', gap: 0.5 }}>
                        <Chip 
                          label={formatFileSize(doc.file_size)} 
                          size="small" 
                          variant="outlined"
                        />
                        {getOcrStatusChip(doc)}
                      </Stack>
                      
                      {doc.tags.length > 0 && (
                        <Stack direction="row" spacing={0.5} sx={{ mb: 1, flexWrap: 'wrap' }}>
                          {doc.tags.slice(0, 3).map((tag, index) => (
                            <Chip 
                              key={index}
                              label={tag} 
                              size="small" 
                              color="primary"
                              variant="outlined"
                              sx={{ 
                                fontSize: '0.7rem', 
                                height: '20px',
                                maxWidth: '120px',
                                '& .MuiChip-label': { 
                                  overflow: 'hidden', 
                                  textOverflow: 'ellipsis',
                                  whiteSpace: 'nowrap'
                                }
                              }}
                            />
                          ))}
                          {doc.tags.length > 3 && (
                            <Chip 
                              label={`+${doc.tags.length - 3}`}
                              size="small" 
                              variant="outlined"
                              sx={{ fontSize: '0.7rem', height: '20px' }}
                            />
                          )}
                        </Stack>
                      )}
                      
                      {doc.labels && doc.labels.length > 0 && (
                        <Stack direction="row" spacing={0.5} sx={{ mb: 1, flexWrap: 'wrap' }}>
                          {doc.labels.slice(0, 3).map((label) => (
                            <Label
                              key={label.id}
                              label={label}
                              size="small"
                              variant="filled"
                            />
                          ))}
                          {doc.labels.length > 3 && (
                            <Chip 
                              label={`+${doc.labels.length - 3}`}
                              size="small" 
                              variant="outlined"
                              sx={{ fontSize: '0.7rem', height: '20px' }}
                            />
                          )}
                        </Stack>
                      )}
                      
                      <Typography variant="caption" color="text.secondary">
                        {formatDate(doc.created_at)}
                      </Typography>
                    </Box>
                    
                    {!selectionMode && (
                      <IconButton
                        size="small"
                        onClick={(e) => {
                          e.stopPropagation();
                          handleDocMenuClick(e, doc);
                        }}
                      >
                        <MoreIcon />
                      </IconButton>
                    )}
                  </Box>
                </CardContent>
                
                {viewMode === 'grid' && (
                  <CardActions sx={{ pt: 0 }}>
                    <Button 
                      size="small" 
                      startIcon={<DownloadIcon />}
                      onClick={(e) => {
                        e.stopPropagation();
                        handleDownload(doc);
                      }}
                      fullWidth
                    >
                      Download
                    </Button>
                  </CardActions>
                )}
              </Card>
            </Grid>
          ))}
        </Grid>
      )}

      {/* Label Edit Dialog */}
      <Dialog open={labelEditDialogOpen} onClose={() => setLabelEditDialogOpen(false)} maxWidth="sm" fullWidth>
        <DialogTitle>Edit Document Labels</DialogTitle>
        <DialogContent>
          <Box sx={{ pt: 2 }}>
            <LabelSelector
              selectedLabels={editingDocumentLabels}
              availableLabels={availableLabels}
              onLabelsChange={setEditingDocumentLabels}
              onCreateLabel={handleCreateLabel}
              placeholder="Select labels for this document..."
              size="medium"
              disabled={labelsLoading}
            />
          </Box>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setLabelEditDialogOpen(false)}>Cancel</Button>
          <Button onClick={handleSaveDocumentLabels} variant="contained">Save</Button>
        </DialogActions>
      </Dialog>

      {/* Delete Confirmation Dialog */}
      <Dialog open={deleteDialogOpen} onClose={handleDeleteCancel} maxWidth="sm">
        <DialogTitle>Delete Document</DialogTitle>
        <DialogContent>
          <Typography>
            Are you sure you want to delete "{documentToDelete?.original_filename}"?
          </Typography>
          <Typography variant="body2" color="text.secondary" sx={{ mt: 1 }}>
            This action cannot be undone. The document file and all associated data will be permanently removed.
          </Typography>
        </DialogContent>
        <DialogActions>
          <Button onClick={handleDeleteCancel} disabled={deleteLoading}>
            Cancel
          </Button>
          <Button 
            onClick={handleDeleteConfirm} 
            color="error" 
            variant="contained"
            disabled={deleteLoading}
            startIcon={deleteLoading ? <CircularProgress size={16} color="inherit" /> : <DeleteIcon />}
          >
            {deleteLoading ? 'Deleting...' : 'Delete'}
          </Button>
        </DialogActions>
      </Dialog>

      {/* Bulk Delete Confirmation Dialog */}
      <Dialog open={bulkDeleteDialogOpen} onClose={handleBulkDeleteCancel} maxWidth="sm">
        <DialogTitle>Delete Multiple Documents</DialogTitle>
        <DialogContent>
          <Typography gutterBottom>
            Are you sure you want to delete {selectedDocuments.size} selected document{selectedDocuments.size !== 1 ? 's' : ''}?
          </Typography>
          <Typography variant="body2" color="text.secondary" sx={{ mt: 1 }}>
            This action cannot be undone. All selected documents and their associated data will be permanently removed.
          </Typography>
          {selectedDocuments.size > 0 && (
            <Box sx={{ mt: 2, maxHeight: 200, overflow: 'auto' }}>
              <Typography variant="subtitle2" gutterBottom>
                Documents to be deleted:
              </Typography>
              {Array.from(selectedDocuments).slice(0, 10).map(docId => {
                const doc = documents.find(d => d.id === docId);
                return doc ? (
                  <Typography key={docId} variant="body2" sx={{ pl: 1 }}>
                    • {doc.original_filename}
                  </Typography>
                ) : null;
              })}
              {selectedDocuments.size > 10 && (
                <Typography variant="body2" sx={{ pl: 1, fontStyle: 'italic' }}>
                  ... and {selectedDocuments.size - 10} more
                </Typography>
              )}
            </Box>
          )}
        </DialogContent>
        <DialogActions>
          <Button onClick={handleBulkDeleteCancel} disabled={bulkDeleteLoading}>
            Cancel
          </Button>
          <Button 
            onClick={handleBulkDeleteConfirm} 
            color="error" 
            variant="contained"
            disabled={bulkDeleteLoading}
            startIcon={bulkDeleteLoading ? <CircularProgress size={16} color="inherit" /> : <DeleteIcon />}
          >
            {bulkDeleteLoading ? 'Deleting...' : `Delete ${selectedDocuments.size} Document${selectedDocuments.size !== 1 ? 's' : ''}`}
          </Button>
        </DialogActions>
      </Dialog>

      {/* Results count and pagination */}
      <Box sx={{ mt: 3 }}>
        <Box sx={{ textAlign: 'center', mb: 2 }}>
          <Typography variant="body2" color="text.secondary">
            Showing {pagination.offset + 1}-{Math.min(pagination.offset + pagination.limit, pagination.total)} of {pagination.total} documents
            {ocrFilter && ` with OCR status: ${ocrFilter}`}
            {searchQuery && ` matching "${searchQuery}"`}
          </Typography>
        </Box>
        
        {pagination.total > pagination.limit && (
          <Box sx={{ display: 'flex', justifyContent: 'center' }}>
            <Pagination
              count={Math.ceil(pagination.total / pagination.limit)}
              page={Math.floor(pagination.offset / pagination.limit) + 1}
              onChange={handlePageChange}
              color="primary"
              size="large"
              showFirstButton
              showLastButton
            />
          </Box>
        )}
      </Box>

      {/* Retry History Modal */}
      <RetryHistoryModal
        open={retryHistoryModalOpen}
        onClose={() => setRetryHistoryModalOpen(false)}
        documentId={selectedDocumentForHistory || ''}
        documentName={selectedDocumentForHistory ? 
          documents.find(d => d.id === selectedDocumentForHistory)?.original_filename : undefined}
      />
    </Box>
  );
};

export default DocumentsPage;