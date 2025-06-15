import React, { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Box,
  Typography,
  Grid,
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
} from '@mui/material';
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
} from '@mui/icons-material';
import { documentService } from '../services/api';

interface Document {
  id: string;
  original_filename: string;
  filename?: string;
  file_size: number;
  mime_type: string;
  created_at: string;
  has_ocr_text?: boolean;
  tags: string[];
}

type ViewMode = 'grid' | 'list';
type SortField = 'created_at' | 'original_filename' | 'file_size';
type SortOrder = 'asc' | 'desc';

const DocumentsPage: React.FC = () => {
  const navigate = useNavigate();
  const [documents, setDocuments] = useState<Document[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);
  const [viewMode, setViewMode] = useState<ViewMode>('grid');
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [sortBy, setSortBy] = useState<SortField>('created_at');
  const [sortOrder, setSortOrder] = useState<SortOrder>('desc');
  
  // Menu states
  const [sortMenuAnchor, setSortMenuAnchor] = useState<null | HTMLElement>(null);
  const [docMenuAnchor, setDocMenuAnchor] = useState<null | HTMLElement>(null);
  const [selectedDoc, setSelectedDoc] = useState<Document | null>(null);

  useEffect(() => {
    fetchDocuments();
  }, []);

  const fetchDocuments = async (): Promise<void> => {
    try {
      setLoading(true);
      const response = await documentService.list(100, 0);
      setDocuments(response.data);
    } catch (err) {
      setError('Failed to load documents');
      console.error(err);
    } finally {
      setLoading(false);
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
                    <Box sx={{ fontSize: '3rem' }}>
                      {getFileIcon(doc.mime_type)}
                    </Box>
                  </Box>
                )}
                
                <CardContent sx={{ flexGrow: 1, pb: 1 }}>
                  <Box sx={{ display: 'flex', alignItems: 'flex-start', gap: 1 }}>
                    {viewMode === 'list' && (
                      <Box sx={{ mr: 1, mt: 0.5 }}>
                        {getFileIcon(doc.mime_type)}
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
                        {doc.has_ocr_text && (
                          <Chip 
                            label="OCR" 
                            size="small" 
                            color="success"
                            variant="outlined"
                          />
                        )}
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

      {/* Results count */}
      <Box sx={{ mt: 3, textAlign: 'center' }}>
        <Typography variant="body2" color="text.secondary">
          Showing {sortedDocuments.length} of {documents.length} documents
          {searchQuery && ` matching "${searchQuery}"`}
        </Typography>
      </Box>
    </Box>
  );
};

export default DocumentsPage;