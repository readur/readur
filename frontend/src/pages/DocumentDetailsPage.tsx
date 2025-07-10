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
  Container,
  Fade,
  Skeleton,
} from '@mui/material';
import Grid from '@mui/material/GridLegacy';
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
  Label as LabelIcon,
  Visibility as ViewIcon,
  Search as SearchIcon,
  Edit as EditIcon,
  PhotoFilter as ProcessedImageIcon,
  Source as SourceIcon,
  AccessTime as AccessTimeIcon,
  Create as CreateIcon,
  Info as InfoIcon,
  Refresh as RefreshIcon,
  History as HistoryIcon,
  Speed as SpeedIcon,
  MoreVert as MoreIcon,
} from '@mui/icons-material';
import { documentService, OcrResponse, type Document } from '../services/api';
import DocumentViewer from '../components/DocumentViewer';
import LabelSelector from '../components/Labels/LabelSelector';
import { type LabelData } from '../components/Labels/Label';
import MetadataDisplay from '../components/MetadataDisplay';
import MetadataParser from '../components/MetadataParser';
import FileIntegrityDisplay from '../components/FileIntegrityDisplay';
import ProcessingTimeline from '../components/ProcessingTimeline';
import { RetryHistoryModal } from '../components/RetryHistoryModal';
import { modernTokens, glassEffect } from '../theme';
import api from '../services/api';

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
  const [showProcessedImageDialog, setShowProcessedImageDialog] = useState<boolean>(false);
  const [processedImageUrl, setProcessedImageUrl] = useState<string | null>(null);
  const [processedImageLoading, setProcessedImageLoading] = useState<boolean>(false);
  const [thumbnailUrl, setThumbnailUrl] = useState<string | null>(null);
  const [documentLabels, setDocumentLabels] = useState<LabelData[]>([]);
  const [availableLabels, setAvailableLabels] = useState<LabelData[]>([]);
  const [showLabelDialog, setShowLabelDialog] = useState<boolean>(false);
  const [labelsLoading, setLabelsLoading] = useState<boolean>(false);
  
  // Retry functionality state
  const [retryingOcr, setRetryingOcr] = useState<boolean>(false);
  const [retryHistoryModalOpen, setRetryHistoryModalOpen] = useState<boolean>(false);

  // Retry handlers
  const handleRetryOcr = async () => {
    if (!document) return;
    
    setRetryingOcr(true);
    try {
      await documentService.bulkRetryOcr({
        mode: 'specific',
        document_ids: [document.id],
        priority_override: 15,
      });
      
      // Show success message and refresh document
      setTimeout(() => {
        fetchDocumentDetails();
      }, 1000);
    } catch (error) {
      console.error('Failed to retry OCR:', error);
    } finally {
      setRetryingOcr(false);
    }
  };

  const handleShowRetryHistory = () => {
    setRetryHistoryModalOpen(true);
  };

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
      fetchDocumentLabels();
    }
  }, [document]);

  useEffect(() => {
    fetchAvailableLabels();
  }, []);

  const fetchDocumentDetails = async (): Promise<void> => {
    if (!id) {
      setError('No document ID provided');
      setLoading(false);
      return;
    }

    try {
      setLoading(true);
      setError(null);
      
      const response = await documentService.getById(id);
      setDocument(response.data);
    } catch (err: any) {
      const errorMessage = err.message || 'Failed to load document details';
      setError(errorMessage);
      console.error('Failed to fetch document details:', err);
    } finally {
      setLoading(false);
    }
  };

  const handleDownload = async (): Promise<void> => {
    if (!document) return;
    
    try {
      const response = await documentService.download(document.id);
      const url = window.URL.createObjectURL(new Blob([response.data]));
      const link = window.document.createElement('a');
      link.href = url;
      link.setAttribute('download', document.original_filename);
      window.document.body.appendChild(link);
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

  const handleViewProcessedImage = async (): Promise<void> => {
    if (!document) return;
    
    setProcessedImageLoading(true);
    try {
      const response = await documentService.getProcessedImage(document.id);
      const url = window.URL.createObjectURL(new Blob([response.data], { type: 'image/png' }));
      setProcessedImageUrl(url);
      setShowProcessedImageDialog(true);
    } catch (err: any) {
      console.log('Processed image not available:', err);
      alert('No processed image available for this document. This feature requires "Save Processed Images" to be enabled in OCR settings.');
    } finally {
      setProcessedImageLoading(false);
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

  const fetchDocumentLabels = async (): Promise<void> => {
    if (!id) return;
    
    try {
      const response = await api.get(`/labels/documents/${id}`);
      if (response.status === 200 && Array.isArray(response.data)) {
        setDocumentLabels(response.data);
      }
    } catch (error) {
      console.error('Failed to fetch document labels:', error);
    }
  };

  const fetchAvailableLabels = async (): Promise<void> => {
    try {
      setLabelsLoading(true);
      const response = await api.get('/labels?include_counts=false');
      if (response.status === 200 && Array.isArray(response.data)) {
        setAvailableLabels(response.data);
      }
    } catch (error) {
      console.error('Failed to fetch available labels:', error);
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

  const handleSaveLabels = async (selectedLabels: LabelData[]): Promise<void> => {
    if (!id) return;
    
    try {
      const labelIds = selectedLabels.map(label => label.id);
      await api.put(`/labels/documents/${id}`, { label_ids: labelIds });
      setDocumentLabels(selectedLabels);
      setShowLabelDialog(false);
    } catch (error) {
      console.error('Failed to save labels:', error);
    }
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
    <Box 
      sx={{ 
        minHeight: '100vh',
        background: `linear-gradient(135deg, ${modernTokens.colors.primary[50]} 0%, ${modernTokens.colors.secondary[50]} 100%)`,
      }}
    >
      <Container maxWidth="xl" sx={{ py: 4 }}>
        {/* Modern Header */}
        <Fade in timeout={600}>
          <Box sx={{ mb: 6 }}>
            <Button
              startIcon={<BackIcon />}
              onClick={() => navigate('/documents')}
              sx={{ 
                mb: 3,
                color: modernTokens.colors.neutral[600],
                '&:hover': {
                  backgroundColor: modernTokens.colors.neutral[100],
                },
              }}
            >
              Back to Documents
            </Button>
            
            <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 2 }}>
              <Typography 
                variant="h2" 
                sx={{ 
                  fontWeight: 800,
                  background: `linear-gradient(135deg, ${modernTokens.colors.primary[600]} 0%, ${modernTokens.colors.secondary[600]} 100%)`,
                  backgroundClip: 'text',
                  WebkitBackgroundClip: 'text',
                  color: 'transparent',
                  letterSpacing: '-0.02em',
                }}
              >
                {document?.original_filename || 'Document Details'}
              </Typography>
              
              {/* Floating Action Menu */}
              <Box sx={{ display: 'flex', gap: 1 }}>
                <Tooltip title="Download">
                  <IconButton
                    onClick={handleDownload}
                    sx={{
                      ...glassEffect(0.2),
                      color: modernTokens.colors.primary[600],
                      '&:hover': {
                        transform: 'scale(1.05)',
                        backgroundColor: modernTokens.colors.primary[100],
                      },
                    }}
                  >
                    <DownloadIcon />
                  </IconButton>
                </Tooltip>
                
                <Tooltip title="View Document">
                  <IconButton
                    onClick={handleViewDocument}
                    sx={{
                      ...glassEffect(0.2),
                      color: modernTokens.colors.primary[600],
                      '&:hover': {
                        transform: 'scale(1.05)',
                        backgroundColor: modernTokens.colors.primary[100],
                      },
                    }}
                  >
                    <ViewIcon />
                  </IconButton>
                </Tooltip>
                
                {document?.has_ocr_text && (
                  <Tooltip title="View OCR Text">
                    <IconButton
                      onClick={handleViewOcr}
                      sx={{
                        ...glassEffect(0.2),
                        color: modernTokens.colors.secondary[600],
                        '&:hover': {
                          transform: 'scale(1.05)',
                          backgroundColor: modernTokens.colors.secondary[100],
                        },
                      }}
                    >
                      <SearchIcon />
                    </IconButton>
                  </Tooltip>
                )}
              </Box>
            </Box>
            
            <Typography variant="body1" color="text.secondary" sx={{ fontSize: '1.1rem' }}>
              Comprehensive document analysis and metadata viewer
            </Typography>
          </Box>
        </Fade>

        {/* Modern Content Layout */}
        <Fade in timeout={800}>
          <Grid container spacing={4}>
            {/* Hero Document Preview */}
            <Grid item xs={12} lg={5}>
              <Card 
                sx={{ 
                  ...glassEffect(0.3),
                  height: 'fit-content',
                  background: `linear-gradient(135deg, ${modernTokens.colors.neutral[0]} 0%, ${modernTokens.colors.primary[50]} 100%)`,
                }}
              >
                <CardContent sx={{ p: 4 }}>
                  {/* Document Preview */}
                  <Box
                    sx={{
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      mb: 4,
                      p: 4,
                      background: `linear-gradient(135deg, ${modernTokens.colors.primary[100]} 0%, ${modernTokens.colors.secondary[100]} 100%)`,
                      borderRadius: 3,
                      minHeight: 280,
                      position: 'relative',
                      overflow: 'hidden',
                      '&::before': {
                        content: '""',
                        position: 'absolute',
                        top: 0,
                        left: 0,
                        right: 0,
                        bottom: 0,
                        background: 'radial-gradient(circle at 30% 30%, rgba(255,255,255,0.3) 0%, transparent 50%)',
                        pointerEvents: 'none',
                      },
                    }}
                  >
                    {thumbnailUrl ? (
                      <img
                        src={thumbnailUrl}
                        alt={document.original_filename}
                        onClick={handleViewDocument}
                        style={{
                          maxWidth: '100%',
                          maxHeight: '250px',
                          borderRadius: '12px',
                          objectFit: 'contain',
                          cursor: 'pointer',
                          transition: 'all 0.3s cubic-bezier(0.4, 0, 0.2, 1)',
                          boxShadow: modernTokens.shadows.lg,
                        }}
                        onMouseEnter={(e) => {
                          e.currentTarget.style.transform = 'scale(1.05) rotateY(5deg)';
                          e.currentTarget.style.boxShadow = modernTokens.shadows.xl;
                        }}
                        onMouseLeave={(e) => {
                          e.currentTarget.style.transform = 'scale(1) rotateY(0deg)';
                          e.currentTarget.style.boxShadow = modernTokens.shadows.lg;
                        }}
                      />
                    ) : (
                      <Box
                        onClick={handleViewDocument}
                        sx={{
                          cursor: 'pointer',
                          transition: 'all 0.3s cubic-bezier(0.4, 0, 0.2, 1)',
                          '&:hover': {
                            transform: 'scale(1.1) rotateY(10deg)',
                          }
                        }}
                      >
                        <Box sx={{ fontSize: 120, color: modernTokens.colors.primary[400], display: 'flex' }}>
                          {getFileIcon(document.mime_type)}
                        </Box>
                      </Box>
                    )}
                  </Box>
                  
                  {/* File Type Badge */}
                  <Box sx={{ display: 'flex', justifyContent: 'center', mb: 3 }}>
                    <Chip 
                      label={document.mime_type}
                      sx={{
                        backgroundColor: modernTokens.colors.primary[100],
                        color: modernTokens.colors.primary[700],
                        fontWeight: 600,
                        border: `1px solid ${modernTokens.colors.primary[300]}`,
                      }}
                    />
                  </Box>
                  
                  {/* Quick Stats */}
                  <Stack spacing={2}>
                    <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                      <Typography variant="body2" color="text.secondary">
                        File Size
                      </Typography>
                      <Typography variant="body2" sx={{ fontWeight: 600 }}>
                        {formatFileSize(document.file_size)}
                      </Typography>
                    </Box>
                    
                    <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                      <Typography variant="body2" color="text.secondary">
                        Upload Date
                      </Typography>
                      <Typography variant="body2" sx={{ fontWeight: 600 }}>
                        {formatDate(document.created_at)}
                      </Typography>
                    </Box>
                    
                    {document.has_ocr_text && (
                      <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                        <Typography variant="body2" color="text.secondary">
                          OCR Status
                        </Typography>
                        <Chip 
                          label="Text Extracted" 
                          color="success"
                          size="small"
                          icon={<TextIcon sx={{ fontSize: 16 }} />}
                        />
                      </Box>
                    )}
                  </Stack>
                  
                  {/* Action Buttons */}
                  <Stack direction="row" spacing={1} sx={{ mt: 4 }} justifyContent="center">
                    {document.mime_type?.includes('image') && (
                      <Tooltip title="View Processed Image">
                        <IconButton
                          onClick={handleViewProcessedImage}
                          disabled={processedImageLoading}
                          sx={{
                            backgroundColor: modernTokens.colors.secondary[100],
                            color: modernTokens.colors.secondary[600],
                            '&:hover': {
                              backgroundColor: modernTokens.colors.secondary[200],
                              transform: 'scale(1.1)',
                            },
                          }}
                        >
                          {processedImageLoading ? (
                            <CircularProgress size={20} />
                          ) : (
                            <ProcessedImageIcon />
                          )}
                        </IconButton>
                      </Tooltip>
                    )}
                    
                    <Tooltip title="Retry OCR">
                      <IconButton
                        onClick={handleRetryOcr}
                        disabled={retryingOcr}
                        sx={{
                          backgroundColor: modernTokens.colors.warning[100],
                          color: modernTokens.colors.warning[600],
                          '&:hover': {
                            backgroundColor: modernTokens.colors.warning[200],
                            transform: 'scale(1.1)',
                          },
                        }}
                      >
                        {retryingOcr ? (
                          <CircularProgress size={20} />
                        ) : (
                          <RefreshIcon />
                        )}
                      </IconButton>
                    </Tooltip>
                    
                    <Tooltip title="Retry History">
                      <IconButton
                        onClick={handleShowRetryHistory}
                        sx={{
                          backgroundColor: modernTokens.colors.info[100],
                          color: modernTokens.colors.info[600],
                          '&:hover': {
                            backgroundColor: modernTokens.colors.info[200],
                            transform: 'scale(1.1)',
                          },
                        }}
                      >
                        <HistoryIcon />
                      </IconButton>
                    </Tooltip>
                  </Stack>
                </CardContent>
              </Card>
            </Grid>

            {/* Main Content Area */}
            <Grid item xs={12} lg={7}>
              <Stack spacing={4}>
                {/* File Integrity Display */}
                <FileIntegrityDisplay
                  fileHash={document.file_hash}
                  fileName={document.original_filename}
                  fileSize={document.file_size}
                  mimeType={document.mime_type}
                  createdAt={document.created_at}
                  updatedAt={document.updated_at}
                  userId={document.user_id}
                />
                
                {/* Processing Timeline */}
                <ProcessingTimeline
                  documentId={document.id}
                  fileName={document.original_filename}
                  createdAt={document.created_at}
                  updatedAt={document.updated_at}
                  userId={document.user_id}
                  ocrStatus={document.has_ocr_text ? 'completed' : 'pending'}
                  ocrCompletedAt={ocrData?.ocr_completed_at}
                  ocrError={ocrData?.ocr_error}
                />
                
                {/* Enhanced Metadata Display */}
                {document.source_metadata && Object.keys(document.source_metadata).length > 0 && (
                  <Card 
                    sx={{ 
                      ...glassEffect(0.2),
                      background: `linear-gradient(135deg, ${modernTokens.colors.neutral[0]} 0%, ${modernTokens.colors.info[50]} 100%)`,
                    }}
                  >
                    <CardContent sx={{ p: 4 }}>
                      <Typography variant="h5" sx={{ mb: 3, fontWeight: 700 }}>
                        📊 Rich Metadata Analysis
                      </Typography>
                      <MetadataParser 
                        metadata={document.source_metadata}
                        fileType={document.mime_type}
                      />
                    </CardContent>
                  </Card>
                )}
                
                {/* Tags and Labels */}
                <Card 
                  sx={{ 
                    ...glassEffect(0.2),
                    background: `linear-gradient(135deg, ${modernTokens.colors.neutral[0]} 0%, ${modernTokens.colors.secondary[50]} 100%)`,
                  }}
                >
                  <CardContent sx={{ p: 4 }}>
                    <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 3 }}>
                      <Typography variant="h5" sx={{ fontWeight: 700 }}>
                        🏷️ Tags & Labels
                      </Typography>
                      <Button
                        startIcon={<EditIcon />}
                        onClick={() => setShowLabelDialog(true)}
                        sx={{
                          backgroundColor: modernTokens.colors.secondary[100],
                          color: modernTokens.colors.secondary[700],
                          '&:hover': {
                            backgroundColor: modernTokens.colors.secondary[200],
                          },
                        }}
                      >
                        Edit Labels
                      </Button>
                    </Box>
                    
                    {/* Tags */}
                    {document.tags && document.tags.length > 0 && (
                      <Box sx={{ mb: 3 }}>
                        <Typography variant="subtitle1" sx={{ mb: 2, fontWeight: 600 }}>
                          Tags
                        </Typography>
                        <Stack direction="row" spacing={1} flexWrap="wrap" gap={1}>
                          {document.tags.map((tag, index) => (
                            <Chip 
                              key={index}
                              label={tag} 
                              sx={{
                                backgroundColor: modernTokens.colors.primary[100],
                                color: modernTokens.colors.primary[700],
                                border: `1px solid ${modernTokens.colors.primary[300]}`,
                                fontWeight: 500,
                              }}
                            />
                          ))}
                        </Stack>
                      </Box>
                    )}
                    
                    {/* Labels */}
                    <Box>
                      <Typography variant="subtitle1" sx={{ mb: 2, fontWeight: 600 }}>
                        Labels
                      </Typography>
                      {documentLabels.length > 0 ? (
                        <Stack direction="row" spacing={1} flexWrap="wrap" gap={1}>
                          {documentLabels.map((label) => (
                            <Chip
                              key={label.id}
                              label={label.name}
                              sx={{
                                backgroundColor: label.background_color || `${label.color}20`,
                                color: label.color,
                                border: `1px solid ${label.color}`,
                                fontWeight: 500,
                              }}
                            />
                          ))}
                        </Stack>
                      ) : (
                        <Typography variant="body2" color="text.secondary" sx={{ fontStyle: 'italic' }}>
                          No labels assigned to this document
                        </Typography>
                      )}
                    </Box>
                  </CardContent>
                </Card>
              </Stack>
            </Grid>
          </Grid>
        </Fade>

        {/* OCR Text Section */}
        {document.has_ocr_text && (
          <Fade in timeout={1000}>
            <Card 
              sx={{ 
                mt: 4,
                ...glassEffect(0.2),
                background: `linear-gradient(135deg, ${modernTokens.colors.neutral[0]} 0%, ${modernTokens.colors.success[50]} 100%)`,
              }}
            >
              <CardContent sx={{ p: 4 }}>
                <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 3 }}>
                  <Typography variant="h4" sx={{ fontWeight: 700 }}>
                    🔍 Extracted Text (OCR)
                  </Typography>
                  <Button
                    startIcon={<SpeedIcon />}
                    onClick={handleViewOcr}
                    sx={{
                      backgroundColor: modernTokens.colors.success[100],
                      color: modernTokens.colors.success[700],
                      '&:hover': {
                        backgroundColor: modernTokens.colors.success[200],
                      },
                    }}
                  >
                    View Full Text
                  </Button>
                </Box>
                
                {ocrLoading ? (
                  <Box sx={{ display: 'flex', justifyContent: 'center', alignItems: 'center', py: 6 }}>
                    <CircularProgress size={32} sx={{ mr: 2 }} />
                    <Typography variant="h6" color="text.secondary">
                      Loading OCR analysis...
                    </Typography>
                  </Box>
                ) : ocrData ? (
                  <>
                    {/* Enhanced OCR Stats */}
                    <Box sx={{ mb: 4, display: 'flex', gap: 2, flexWrap: 'wrap' }}>
                      {ocrData.ocr_confidence && (
                        <Box 
                          sx={{ 
                            p: 2, 
                            borderRadius: 2,
                            backgroundColor: modernTokens.colors.primary[100],
                            border: `1px solid ${modernTokens.colors.primary[300]}`,
                            textAlign: 'center',
                            minWidth: 120,
                          }}
                        >
                          <Typography variant="h5" sx={{ fontWeight: 700, color: modernTokens.colors.primary[700] }}>
                            {Math.round(ocrData.ocr_confidence)}%
                          </Typography>
                          <Typography variant="caption" color="text.secondary">
                            Confidence
                          </Typography>
                        </Box>
                      )}
                      {ocrData.ocr_word_count && (
                        <Box 
                          sx={{ 
                            p: 2, 
                            borderRadius: 2,
                            backgroundColor: modernTokens.colors.secondary[100],
                            border: `1px solid ${modernTokens.colors.secondary[300]}`,
                            textAlign: 'center',
                            minWidth: 120,
                          }}
                        >
                          <Typography variant="h5" sx={{ fontWeight: 700, color: modernTokens.colors.secondary[700] }}>
                            {ocrData.ocr_word_count.toLocaleString()}
                          </Typography>
                          <Typography variant="caption" color="text.secondary">
                            Words
                          </Typography>
                        </Box>
                      )}
                      {ocrData.ocr_processing_time_ms && (
                        <Box 
                          sx={{ 
                            p: 2, 
                            borderRadius: 2,
                            backgroundColor: modernTokens.colors.info[100],
                            border: `1px solid ${modernTokens.colors.info[300]}`,
                            textAlign: 'center',
                            minWidth: 120,
                          }}
                        >
                          <Typography variant="h5" sx={{ fontWeight: 700, color: modernTokens.colors.info[700] }}>
                            {ocrData.ocr_processing_time_ms}ms
                          </Typography>
                          <Typography variant="caption" color="text.secondary">
                            Processing Time
                          </Typography>
                        </Box>
                      )}
                    </Box>

                    {/* OCR Error Display */}
                    {ocrData.ocr_error && (
                      <Alert 
                        severity="error" 
                        sx={{ 
                          mb: 3,
                          borderRadius: 2,
                          backgroundColor: modernTokens.colors.error[50],
                          border: `1px solid ${modernTokens.colors.error[200]}`,
                        }}
                      >
                        <Typography variant="subtitle2" sx={{ fontWeight: 600 }}>
                          OCR Processing Error
                        </Typography>
                        <Typography variant="body2">{ocrData.ocr_error}</Typography>
                      </Alert>
                    )}

                    {/* OCR Text Preview */}
                    <Paper
                      sx={{
                        p: 4,
                        background: `linear-gradient(135deg, ${modernTokens.colors.neutral[50]} 0%, ${modernTokens.colors.neutral[100]} 100%)`,
                        border: `1px solid ${modernTokens.colors.neutral[300]}`,
                        borderRadius: 3,
                        maxHeight: 300,
                        overflow: 'auto',
                        position: 'relative',
                      }}
                    >
                      {ocrData.ocr_text ? (
                        <Typography
                          variant="body1"
                          sx={{
                            fontFamily: '"Inter", monospace',
                            whiteSpace: 'pre-wrap',
                            lineHeight: 1.8,
                            color: modernTokens.colors.neutral[800],
                            fontSize: '0.95rem',
                          }}
                        >
                          {ocrData.ocr_text.length > 500 
                            ? `${ocrData.ocr_text.substring(0, 500)}...` 
                            : ocrData.ocr_text
                          }
                        </Typography>
                      ) : (
                        <Typography variant="body1" color="text.secondary" sx={{ fontStyle: 'italic', textAlign: 'center', py: 4 }}>
                          No OCR text available for this document.
                        </Typography>
                      )}
                      
                      {ocrData.ocr_text && ocrData.ocr_text.length > 500 && (
                        <Box 
                          sx={{ 
                            position: 'absolute',
                            bottom: 0,
                            left: 0,
                            right: 0,
                            height: 60,
                            background: `linear-gradient(transparent, ${modernTokens.colors.neutral[100]})`,
                            display: 'flex',
                            alignItems: 'end',
                            justifyContent: 'center',
                            pb: 2,
                          }}
                        >
                          <Button 
                            onClick={handleViewOcr}
                            size="small"
                            sx={{ 
                              backgroundColor: modernTokens.colors.neutral[0],
                              boxShadow: modernTokens.shadows.sm,
                            }}
                          >
                            View Full Text
                          </Button>
                        </Box>
                      )}
                    </Paper>

                    {/* Processing Info */}
                    {ocrData.ocr_completed_at && (
                      <Box sx={{ mt: 3, pt: 3, borderTop: `1px solid ${modernTokens.colors.neutral[200]}` }}>
                        <Typography variant="body2" color="text.secondary">
                          ✅ Processing completed: {new Date(ocrData.ocr_completed_at).toLocaleString()}
                        </Typography>
                      </Box>
                    )}
                  </>
                ) : (
                  <Alert 
                    severity="info"
                    sx={{
                      borderRadius: 2,
                      backgroundColor: modernTokens.colors.info[50],
                      border: `1px solid ${modernTokens.colors.info[200]}`,
                    }}
                  >
                    OCR text is available but failed to load. Try clicking the "View Full Text" button above.
                  </Alert>
                )}
              </CardContent>
            </Card>
          </Fade>
        )}
      </Container>

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

      {/* Processed Image Dialog */}
      <Dialog
        open={showProcessedImageDialog}
        onClose={() => setShowProcessedImageDialog(false)}
        maxWidth="lg"
        fullWidth
      >
        <DialogTitle>
          Processed Image - OCR Enhancement Applied
        </DialogTitle>
        <DialogContent>
          {processedImageUrl ? (
            <Box sx={{ textAlign: 'center', py: 2 }}>
              <img 
                src={processedImageUrl}
                alt="Processed image that was fed to OCR"
                style={{ 
                  maxWidth: '100%', 
                  maxHeight: '70vh', 
                  objectFit: 'contain',
                  border: '1px solid #ddd',
                  borderRadius: '4px'
                }}
              />
              <Typography variant="body2" sx={{ mt: 2, color: 'text.secondary' }}>
                This is the enhanced image that was actually processed by the OCR engine.
                You can adjust OCR enhancement settings in the Settings page.
              </Typography>
            </Box>
          ) : (
            <Box sx={{ textAlign: 'center', py: 4 }}>
              <Typography>No processed image available</Typography>
            </Box>
          )}
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setShowProcessedImageDialog(false)}>
            Close
          </Button>
        </DialogActions>
      </Dialog>

      {/* Label Edit Dialog */}
      <Dialog
        open={showLabelDialog}
        onClose={() => setShowLabelDialog(false)}
        maxWidth="md"
        fullWidth
      >
        <DialogTitle>
          Edit Document Labels
        </DialogTitle>
        <DialogContent>
          <Box sx={{ mt: 2 }}>
            <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
              Select labels to assign to this document
            </Typography>
            <LabelSelector
              selectedLabels={documentLabels}
              availableLabels={availableLabels}
              onLabelsChange={setDocumentLabels}
              onCreateLabel={handleCreateLabel}
              placeholder="Choose labels for this document..."
              size="medium"
              disabled={labelsLoading}
            />
          </Box>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setShowLabelDialog(false)}>
            Cancel
          </Button>
          <Button 
            variant="contained" 
            onClick={() => handleSaveLabels(documentLabels)}
            sx={{ borderRadius: 2 }}
          >
            Save Labels
          </Button>
        </DialogActions>
      </Dialog>

      {/* Retry History Modal */}
      {document && (
        <RetryHistoryModal
          open={retryHistoryModalOpen}
          onClose={() => setRetryHistoryModalOpen(false)}
          documentId={document.id}
          documentName={document.original_filename}
        />
      )}
    </Box>
  );
};

export default DocumentDetailsPage;