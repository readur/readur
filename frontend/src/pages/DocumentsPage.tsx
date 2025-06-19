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
} from '@mui/icons-material';
import { documentService } from '../services/api';
import DocumentThumbnail from '../components/DocumentThumbnail';
import Label, { type LabelData } from '../components/Labels/Label';
import LabelSelector from '../components/Labels/LabelSelector';
import { useApi } from '../hooks/useApi';

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

  useEffect(() => {
    fetchDocuments();
    fetchLabels();
  }, [pagination.limit, pagination.offset, ocrFilter]);

  const fetchDocuments = async (): Promise<void> => {
    try {
      setLoading(true);
      const response = await documentService.listWithPagination(
        pagination.limit, 
        pagination.offset, 
        ocrFilter || undefined
      );
      setDocuments(response.data.documents);
      setPagination(response.data.pagination);
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

  const filteredDocuments = documents.filter(doc =>
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

  const handlePageChange = (event: React.ChangeEvent<unknown>, page: number): void => {
    const newOffset = (page - 1) * pagination.limit;
    setPagination(prev => ({ ...prev, offset: newOffset }));
  };

  const handleOcrFilterChange = (event: React.ChangeEvent<HTMLInputElement>): void => {
    setOcrFilter(event.target.value);
    setPagination(prev => ({ ...prev, offset: 0 })); // Reset to first page when filtering
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
                  '&:hover': {
                    transform: 'translateY(-4px)',
                    boxShadow: (theme) => theme.shadows[4],
                  },
                }}
                onClick={() => navigate(`/documents/${doc.id}`)}
              >
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
                              sx={{ fontSize: '0.7rem', height: '20px' }}
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
                    
                    <IconButton
                      size="small"
                      onClick={(e) => {
                        e.stopPropagation();
                        handleDocMenuClick(e, doc);
                      }}
                    >
                      <MoreIcon />
                    </IconButton>
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
    </Box>
  );
};

export default DocumentsPage;