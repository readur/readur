import React, { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import {
  Box,
  Typography,
  Card,
  CardContent,
  Button,
  Chip,
  Stack,
  Grid,
  Divider,
  IconButton,
  Paper,
  Alert,
  CircularProgress,
  Tooltip,
  Dialog,
  DialogContent,
  DialogTitle,
  DialogActions,
} from '@mui/material';
import {
  ArrowBack as BackIcon,
  Download as DownloadIcon,
  PictureAsPdf as PdfIcon,
  Image as ImageIcon,
  Description as DocIcon,
  TextSnippet as TextIcon,
  CalendarToday as DateIcon,
  Storage as SizeIcon,
  Tag as TagIcon,
  Visibility as ViewIcon,
  Search as SearchIcon,
  Edit as EditIcon,
} from '@mui/icons-material';
import { documentService, OcrResponse } from '../services/api';
import DocumentViewer from '../components/DocumentViewer';

interface Document {
  id: string;
  original_filename: string;
  filename?: string;
  file_size: number;
  mime_type: string;
  created_at: string;
  has_ocr_text?: boolean;
  tags?: string[];
}

const DocumentDetailsPage: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [document, setDocument] = useState<Document | null>(null);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);
  const [ocrText, setOcrText] = useState<string>('');
  const [ocrData, setOcrData] = useState<OcrResponse | null>(null);
  const [showOcrDialog, setShowOcrDialog] = useState<boolean>(false);
  const [ocrLoading, setOcrLoading] = useState<boolean>(false);
  const [showViewDialog, setShowViewDialog] = useState<boolean>(false);
  const [thumbnailUrl, setThumbnailUrl] = useState<string | null>(null);

  useEffect(() => {
    if (id) {
      fetchDocumentDetails();
    }
  }, [id]);

  useEffect(() => {
    if (document && document.has_ocr_text && !ocrData) {
      fetchOcrText();
    }
  }, [document]);

  useEffect(() => {
    if (document) {
      loadThumbnail();
    }
  }, [document]);

  const fetchDocumentDetails = async (): Promise<void> => {
    try {
      setLoading(true);
      setError(null);
      
      // Since we don't have a direct document details endpoint, 
      // we'll fetch the document from the list and find the matching one
      const response = await documentService.list(1000, 0);
      const foundDoc = response.data.find(doc => doc.id === id);
      
      if (foundDoc) {
        setDocument(foundDoc);
      } else {
        setError('Document not found');
      }
    } catch (err) {
      setError('Failed to load document details');
      console.error(err);
    } finally {
      setLoading(false);
    }
  };

  const handleDownload = async (): Promise<void> => {
    if (!document) return;
    
    try {
      const response = await documentService.download(document.id);
      const url = window.URL.createObjectURL(new Blob([response.data]));
      const link = document.createElement('a');
      link.href = url;
      link.setAttribute('download', document.original_filename);
      document.body.appendChild(link);
      link.click();
      link.remove();
      window.URL.revokeObjectURL(url);
    } catch (err) {
      console.error('Download failed:', err);
    }
  };

  const fetchOcrText = async (): Promise<void> => {
    if (!document || !document.has_ocr_text) return;
    
    try {
      setOcrLoading(true);
      const response = await documentService.getOcrText(document.id);
      setOcrData(response.data);
      setOcrText(response.data.ocr_text || 'No OCR text available');
    } catch (err) {
      console.error('Failed to fetch OCR text:', err);
      setOcrText('Failed to load OCR text. Please try again.');
    } finally {
      setOcrLoading(false);
    }
  };

  const handleViewOcr = (): void => {
    setShowOcrDialog(true);
    if (!ocrData) {
      fetchOcrText();
    }
  };

  const loadThumbnail = async (): Promise<void> => {
    if (!document) return;
    
    try {
      const response = await documentService.getThumbnail(document.id);
      const url = window.URL.createObjectURL(new Blob([response.data]));
      setThumbnailUrl(url);
    } catch (err) {
      console.log('Thumbnail not available:', err);
      // Thumbnail not available, use fallback icon
    }
  };

  const handleViewDocument = (): void => {
    setShowViewDialog(true);
  };

  const getFileIcon = (mimeType?: string): React.ReactElement => {
    if (mimeType?.includes('pdf')) return <PdfIcon color="error" sx={{ fontSize: 64 }} />;
    if (mimeType?.includes('image')) return <ImageIcon color="primary" sx={{ fontSize: 64 }} />;
    if (mimeType?.includes('text')) return <TextIcon color="info" sx={{ fontSize: 64 }} />;
    return <DocIcon color="secondary" sx={{ fontSize: 64 }} />;
  };

  const formatFileSize = (bytes: number): string => {
    const sizes = ['Bytes', 'KB', 'MB', 'GB'];
    if (bytes === 0) return '0 Bytes';
    const i = Math.floor(Math.log(bytes) / Math.log(1024));
    return Math.round(bytes / Math.pow(1024, i) * 100) / 100 + ' ' + sizes[i];
  };

  const formatDate = (dateString: string): string => {
    return new Date(dateString).toLocaleString('en-US', {
      year: 'numeric',
      month: 'long',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  if (loading) {
    return (
      <Box display="flex" justifyContent="center" alignItems="center" minHeight="400px">
        <CircularProgress />
      </Box>
    );
  }

  if (error || !document) {
    return (
      <Box sx={{ p: 3 }}>
        <Button
          startIcon={<BackIcon />}
          onClick={() => navigate('/documents')}
          sx={{ mb: 3 }}
        >
          Back to Documents
        </Button>
        <Alert severity="error">
          {error || 'Document not found'}
        </Alert>
      </Box>
    );
  }

  return (
    <Box sx={{ p: 3 }}>
      {/* Header */}
      <Box sx={{ mb: 4 }}>
        <Button
          startIcon={<BackIcon />}
          onClick={() => navigate('/documents')}
          sx={{ mb: 2 }}
        >
          Back to Documents
        </Button>
        
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
          Document Details
        </Typography>
        <Typography variant="body1" color="text.secondary">
          View and manage document information
        </Typography>
      </Box>

      <Grid container spacing={3}>
        {/* Document Preview */}
        <Grid item xs={12} md={4}>
          <Card sx={{ height: 'fit-content' }}>
            <CardContent sx={{ textAlign: 'center', py: 4 }}>
              <Box
                sx={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  mb: 3,
                  p: 3,
                  background: 'linear-gradient(135deg, rgba(99, 102, 241, 0.1) 0%, rgba(139, 92, 246, 0.1) 100%)',
                  borderRadius: 2,
                  minHeight: 200,
                }}
              >
                {thumbnailUrl ? (
                  <img
                    src={thumbnailUrl}
                    alt={document.original_filename}
                    style={{
                      maxWidth: '100%',
                      maxHeight: '200px',
                      borderRadius: '8px',
                      objectFit: 'contain',
                    }}
                  />
                ) : (
                  getFileIcon(document.mime_type)
                )}
              </Box>
              
              <Typography variant="h6" sx={{ mb: 2, fontWeight: 600 }}>
                {document.original_filename}
              </Typography>
              
              <Stack direction="row" spacing={1} justifyContent="center" sx={{ mb: 3, flexWrap: 'wrap' }}>
                <Button
                  variant="contained"
                  startIcon={<ViewIcon />}
                  onClick={handleViewDocument}
                  sx={{ borderRadius: 2 }}
                >
                  View
                </Button>
                <Button
                  variant="outlined"
                  startIcon={<DownloadIcon />}
                  onClick={handleDownload}
                  sx={{ borderRadius: 2 }}
                >
                  Download
                </Button>
                {document.has_ocr_text && (
                  <Button
                    variant="outlined"
                    startIcon={<SearchIcon />}
                    onClick={handleViewOcr}
                    sx={{ borderRadius: 2 }}
                  >
                    OCR Text
                  </Button>
                )}
              </Stack>
              
              {document.has_ocr_text && (
                <Chip 
                  label="OCR Processed" 
                  color="success"
                  variant="outlined"
                  icon={<TextIcon />}
                />
              )}
            </CardContent>
          </Card>
        </Grid>

        {/* Document Information */}
        <Grid item xs={12} md={8}>
          <Card>
            <CardContent>
              <Typography variant="h6" sx={{ mb: 3, fontWeight: 600 }}>
                Document Information
              </Typography>

              <Grid container spacing={3}>
                <Grid item xs={12} sm={6}>
                  <Paper sx={{ p: 2, height: '100%' }}>
                    <Box sx={{ display: 'flex', alignItems: 'center', mb: 1 }}>
                      <DocIcon color="primary" sx={{ mr: 1 }} />
                      <Typography variant="subtitle2" color="text.secondary">
                        Filename
                      </Typography>
                    </Box>
                    <Typography variant="body1" sx={{ fontWeight: 500 }}>
                      {document.original_filename}
                    </Typography>
                  </Paper>
                </Grid>

                <Grid item xs={12} sm={6}>
                  <Paper sx={{ p: 2, height: '100%' }}>
                    <Box sx={{ display: 'flex', alignItems: 'center', mb: 1 }}>
                      <SizeIcon color="primary" sx={{ mr: 1 }} />
                      <Typography variant="subtitle2" color="text.secondary">
                        File Size
                      </Typography>
                    </Box>
                    <Typography variant="body1" sx={{ fontWeight: 500 }}>
                      {formatFileSize(document.file_size)}
                    </Typography>
                  </Paper>
                </Grid>

                <Grid item xs={12} sm={6}>
                  <Paper sx={{ p: 2, height: '100%' }}>
                    <Box sx={{ display: 'flex', alignItems: 'center', mb: 1 }}>
                      <DateIcon color="primary" sx={{ mr: 1 }} />
                      <Typography variant="subtitle2" color="text.secondary">
                        Upload Date
                      </Typography>
                    </Box>
                    <Typography variant="body1" sx={{ fontWeight: 500 }}>
                      {formatDate(document.created_at)}
                    </Typography>
                  </Paper>
                </Grid>

                <Grid item xs={12} sm={6}>
                  <Paper sx={{ p: 2, height: '100%' }}>
                    <Box sx={{ display: 'flex', alignItems: 'center', mb: 1 }}>
                      <ViewIcon color="primary" sx={{ mr: 1 }} />
                      <Typography variant="subtitle2" color="text.secondary">
                        File Type
                      </Typography>
                    </Box>
                    <Typography variant="body1" sx={{ fontWeight: 500 }}>
                      {document.mime_type}
                    </Typography>
                  </Paper>
                </Grid>

                {document.tags && document.tags.length > 0 && (
                  <Grid item xs={12}>
                    <Paper sx={{ p: 2 }}>
                      <Box sx={{ display: 'flex', alignItems: 'center', mb: 2 }}>
                        <TagIcon color="primary" sx={{ mr: 1 }} />
                        <Typography variant="subtitle2" color="text.secondary">
                          Tags
                        </Typography>
                      </Box>
                      <Stack direction="row" spacing={1} flexWrap="wrap" gap={1}>
                        {document.tags.map((tag, index) => (
                          <Chip 
                            key={index}
                            label={tag} 
                            color="primary"
                            variant="outlined"
                          />
                        ))}
                      </Stack>
                    </Paper>
                  </Grid>
                )}
              </Grid>

              <Divider sx={{ my: 3 }} />

              <Typography variant="h6" sx={{ mb: 2, fontWeight: 600 }}>
                Processing Status
              </Typography>
              
              <Grid container spacing={2}>
                <Grid item xs={12} sm={6}>
                  <Box sx={{ display: 'flex', alignItems: 'center' }}>
                    <Box
                      sx={{
                        width: 12,
                        height: 12,
                        borderRadius: '50%',
                        backgroundColor: 'success.main',
                        mr: 2,
                      }}
                    />
                    <Typography variant="body2">
                      Document uploaded successfully
                    </Typography>
                  </Box>
                </Grid>
                <Grid item xs={12} sm={6}>
                  <Box sx={{ display: 'flex', alignItems: 'center' }}>
                    <Box
                      sx={{
                        width: 12,
                        height: 12,
                        borderRadius: '50%',
                        backgroundColor: document.has_ocr_text ? 'success.main' : 'warning.main',
                        mr: 2,
                      }}
                    />
                    <Typography variant="body2">
                      {document.has_ocr_text ? 'OCR processing completed' : 'OCR processing pending'}
                    </Typography>
                  </Box>
                </Grid>
              </Grid>
            </CardContent>
          </Card>
        </Grid>

        {/* OCR Text Section */}
        {document.has_ocr_text && (
          <Grid item xs={12}>
            <Card>
              <CardContent>
                <Typography variant="h6" sx={{ mb: 3, fontWeight: 600 }}>
                  Extracted Text (OCR)
                </Typography>
                
                {ocrLoading ? (
                  <Box sx={{ display: 'flex', justifyContent: 'center', alignItems: 'center', py: 4 }}>
                    <CircularProgress size={24} sx={{ mr: 2 }} />
                    <Typography variant="body2" color="text.secondary">
                      Loading OCR text...
                    </Typography>
                  </Box>
                ) : ocrData ? (
                  <>
                    {/* OCR Stats */}
                    <Box sx={{ mb: 3, display: 'flex', gap: 1, flexWrap: 'wrap' }}>
                      {ocrData.ocr_confidence && (
                        <Chip 
                          label={`${Math.round(ocrData.ocr_confidence)}% confidence`} 
                          color="primary" 
                          size="small" 
                        />
                      )}
                      {ocrData.ocr_word_count && (
                        <Chip 
                          label={`${ocrData.ocr_word_count} words`} 
                          color="secondary" 
                          size="small" 
                        />
                      )}
                      {ocrData.ocr_processing_time_ms && (
                        <Chip 
                          label={`${ocrData.ocr_processing_time_ms}ms processing`} 
                          color="info" 
                          size="small" 
                        />
                      )}
                    </Box>

                    {/* OCR Error Display */}
                    {ocrData.ocr_error && (
                      <Alert severity="error" sx={{ mb: 3 }}>
                        OCR Error: {ocrData.ocr_error}
                      </Alert>
                    )}

                    {/* OCR Text Content */}
                    <Paper
                      sx={{
                        p: 3,
                        backgroundColor: (theme) => theme.palette.mode === 'light' ? 'grey.50' : 'grey.900',
                        border: '1px solid',
                        borderColor: 'divider',
                        maxHeight: 400,
                        overflow: 'auto',
                        position: 'relative',
                      }}
                    >
                      {ocrData.ocr_text ? (
                        <Typography
                          variant="body2"
                          sx={{
                            fontFamily: 'monospace',
                            whiteSpace: 'pre-wrap',
                            lineHeight: 1.6,
                            color: 'text.primary',
                          }}
                        >
                          {ocrData.ocr_text}
                        </Typography>
                      ) : (
                        <Typography variant="body2" color="text.secondary" sx={{ fontStyle: 'italic' }}>
                          No OCR text available for this document.
                        </Typography>
                      )}
                    </Paper>

                    {/* Processing Info */}
                    {ocrData.ocr_completed_at && (
                      <Box sx={{ mt: 2, pt: 2, borderTop: '1px solid', borderColor: 'divider' }}>
                        <Typography variant="caption" color="text.secondary">
                          Processing completed: {new Date(ocrData.ocr_completed_at).toLocaleString()}
                        </Typography>
                      </Box>
                    )}
                  </>
                ) : (
                  <Alert severity="info">
                    OCR text is available but failed to load. Try clicking the "View OCR" button above.
                  </Alert>
                )}
              </CardContent>
            </Card>
          </Grid>
        )}
      </Grid>

      {/* OCR Text Dialog */}
      <Dialog
        open={showOcrDialog}
        onClose={() => setShowOcrDialog(false)}
        maxWidth="md"
        fullWidth
      >
        <DialogTitle>
          <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <Typography variant="h6" sx={{ fontWeight: 600 }}>
              Extracted Text (OCR)
            </Typography>
            {ocrData && (
              <Stack direction="row" spacing={1}>
                {ocrData.ocr_confidence && (
                  <Chip 
                    label={`${Math.round(ocrData.ocr_confidence)}% confidence`} 
                    color="primary" 
                    size="small" 
                  />
                )}
                {ocrData.ocr_word_count && (
                  <Chip 
                    label={`${ocrData.ocr_word_count} words`} 
                    color="secondary" 
                    size="small" 
                  />
                )}
              </Stack>
            )}
          </Box>
        </DialogTitle>
        <DialogContent>
          {ocrLoading ? (
            <Box sx={{ display: 'flex', justifyContent: 'center', alignItems: 'center', py: 4 }}>
              <CircularProgress />
              <Typography variant="body2" sx={{ ml: 2 }}>
                Loading OCR text...
              </Typography>
            </Box>
          ) : (
            <>
              {ocrData && ocrData.ocr_error && (
                <Alert severity="error" sx={{ mb: 2 }}>
                  OCR Error: {ocrData.ocr_error}
                </Alert>
              )}
              <Paper
                sx={{
                  p: 2,
                  backgroundColor: 'grey.50',
                  border: '1px solid',
                  borderColor: 'grey.200',
                  maxHeight: 400,
                  overflow: 'auto',
                }}
              >
                <Typography
                  variant="body2"
                  sx={{
                    fontFamily: 'monospace',
                    whiteSpace: 'pre-wrap',
                    color: ocrText ? 'text.primary' : 'text.secondary',
                    lineHeight: 1.6,
                  }}
                >
                  {ocrText || 'No OCR text available for this document.'}
                </Typography>
              </Paper>
              {ocrData && (ocrData.ocr_processing_time_ms || ocrData.ocr_completed_at) && (
                <Box sx={{ mt: 2, pt: 2, borderTop: '1px solid', borderColor: 'grey.200' }}>
                  <Typography variant="caption" color="text.secondary">
                    {ocrData.ocr_processing_time_ms && `Processing time: ${ocrData.ocr_processing_time_ms}ms`}
                    {ocrData.ocr_processing_time_ms && ocrData.ocr_completed_at && ' • '}
                    {ocrData.ocr_completed_at && `Completed: ${new Date(ocrData.ocr_completed_at).toLocaleString()}`}
                  </Typography>
                </Box>
              )}
            </>
          )}
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setShowOcrDialog(false)}>
            Close
          </Button>
        </DialogActions>
      </Dialog>

      {/* Document View Dialog */}
      <Dialog
        open={showViewDialog}
        onClose={() => setShowViewDialog(false)}
        maxWidth="lg"
        fullWidth
        PaperProps={{
          sx: { height: '90vh' }
        }}
      >
        <DialogTitle>
          <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <Typography variant="h6" sx={{ fontWeight: 600 }}>
              {document?.original_filename}
            </Typography>
            <Box>
              <Button
                startIcon={<DownloadIcon />}
                onClick={handleDownload}
                size="small"
                sx={{ mr: 1 }}
              >
                Download
              </Button>
            </Box>
          </Box>
        </DialogTitle>
        <DialogContent sx={{ p: 0, display: 'flex', flexDirection: 'column' }}>
          {document && (
            <DocumentViewer
              documentId={document.id}
              filename={document.original_filename}
              mimeType={document.mime_type}
            />
          )}
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setShowViewDialog(false)}>
            Close
          </Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
};

export default DocumentDetailsPage;