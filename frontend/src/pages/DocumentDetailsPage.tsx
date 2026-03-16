import React, { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import {
  Box,
  Typography,
  Button,
  CircularProgress,
  Alert,
  Container,
  Snackbar,
  Dialog,
  DialogContent,
  DialogTitle,
  DialogActions,
  Tabs,
  Tab,
} from '@mui/material';
import {
  ArrowBack as BackIcon,
  Delete as DeleteIcon,
} from '@mui/icons-material';
import { documentService, OcrResponse, type Document } from '../services/api';
import LabelSelector from '../components/Labels/LabelSelector';
import { type LabelData } from '../components/Labels/Label';
import { RetryHistoryModal } from '../components/RetryHistoryModal';
import { useTheme as useMuiTheme } from '@mui/material/styles';
import api from '../services/api';
import DocumentDetailsHeader from '../components/DocumentDetails/DocumentDetailsHeader';
import PreviewTab from '../components/DocumentDetails/PreviewTab';
import OcrTextTab from '../components/DocumentDetails/OcrTextTab';
import DetailsTab from '../components/DocumentDetails/DetailsTab';
import ActivityTab from '../components/DocumentDetails/ActivityTab';

const DocumentDetailsPage: React.FC = () => {
  const { t } = useTranslation();
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const theme = useMuiTheme();

  // Tab state
  const [tabValue, setTabValue] = useState<number>(0);

  // Core data
  const [document, setDocument] = useState<Document | null>(null);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);
  const [ocrText, setOcrText] = useState<string>('');
  const [ocrData, setOcrData] = useState<OcrResponse | null>(null);
  const [ocrLoading, setOcrLoading] = useState<boolean>(false);

  // Processed image
  const [showProcessedImageDialog, setShowProcessedImageDialog] = useState<boolean>(false);
  const [processedImageUrl, setProcessedImageUrl] = useState<string | null>(null);
  const [processedImageLoading, setProcessedImageLoading] = useState<boolean>(false);

  // Labels
  const [documentLabels, setDocumentLabels] = useState<LabelData[]>([]);
  const [availableLabels, setAvailableLabels] = useState<LabelData[]>([]);
  const [showLabelDialog, setShowLabelDialog] = useState<boolean>(false);
  const [labelsLoading, setLabelsLoading] = useState<boolean>(false);

  // Retry
  const [retryingOcr, setRetryingOcr] = useState<boolean>(false);
  const [retryHistoryModalOpen, setRetryHistoryModalOpen] = useState<boolean>(false);

  // Delete
  const [deleting, setDeleting] = useState<boolean>(false);
  const [deleteConfirmOpen, setDeleteConfirmOpen] = useState<boolean>(false);

  // Snackbar
  const [snackbarOpen, setSnackbarOpen] = useState<boolean>(false);
  const [snackbarMessage, setSnackbarMessage] = useState<string>('');
  const [snackbarSeverity, setSnackbarSeverity] = useState<'success' | 'error'>('success');

  // --- Handlers ---

  const handleTabChange = (_event: React.SyntheticEvent, newValue: number) => {
    setTabValue(newValue);
  };

  const handleRetryOcr = async () => {
    if (!document) return;
    setRetryingOcr(true);
    try {
      await documentService.bulkRetryOcr({
        mode: 'specific',
        document_ids: [document.id],
        priority_override: 15,
      });
      setSnackbarMessage('OCR retry initiated successfully');
      setSnackbarSeverity('success');
      setSnackbarOpen(true);
      setTimeout(() => { fetchDocumentDetails(); }, 1000);
    } catch (error: any) {
      console.error('Failed to retry OCR:', error);
      setSnackbarMessage(`Failed to retry OCR: ${error.message || 'Unknown error'}`);
      setSnackbarSeverity('error');
      setSnackbarOpen(true);
    } finally {
      setRetryingOcr(false);
    }
  };

  const handleDeleteDocument = async () => {
    if (!document) return;
    setDeleting(true);
    try {
      await documentService.delete(document.id);
      navigate('/documents');
    } catch (error) {
      console.error('Failed to delete document:', error);
      setSnackbarMessage(t('common.status.error'));
      setSnackbarSeverity('error');
      setSnackbarOpen(true);
    } finally {
      setDeleting(false);
      setDeleteConfirmOpen(false);
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
      setSnackbarMessage(t('documentDetails.dialogs.processedImage.noImage'));
      setSnackbarSeverity('error');
      setSnackbarOpen(true);
    } finally {
      setProcessedImageLoading(false);
    }
  };

  // --- Data fetching ---

  const fetchDocumentDetails = async (): Promise<void> => {
    if (!id) {
      setError(t('documentDetails.errors.notFound'));
      setLoading(false);
      return;
    }
    try {
      setLoading(true);
      setError(null);
      const response = await documentService.getById(id);
      setDocument(response.data);
    } catch (err: any) {
      const errorMessage = err.message || t('common.status.error');
      setError(errorMessage);
      console.error('Failed to fetch document details:', err);
    } finally {
      setLoading(false);
    }
  };

  const fetchOcrText = async (): Promise<void> => {
    if (!document || !document.has_ocr_text) return;
    try {
      setOcrLoading(true);
      const response = await documentService.getOcrText(document.id);
      setOcrData(response.data);
      setOcrText(response.data.ocr_text || t('documentDetails.ocr.noText'));
    } catch (err) {
      console.error('Failed to fetch OCR text:', err);
      setOcrText(t('documentDetails.ocr.loadFailed'));
    } finally {
      setOcrLoading(false);
    }
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

  // --- Utilities ---

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

  // --- Effects ---

  useEffect(() => {
    if (id) { fetchDocumentDetails(); }
  }, [id]);

  useEffect(() => {
    if (document && document.has_ocr_text && !ocrData) {
      fetchOcrText();
    }
  }, [document]);

  useEffect(() => {
    if (document) {
      fetchDocumentLabels();
    }
  }, [document]);

  useEffect(() => {
    fetchAvailableLabels();
  }, []);

  // Auto-refresh during OCR processing
  useEffect(() => {
    if (!document) return;
    const isProcessing = document.ocr_status === 'pending' || document.ocr_status === 'processing' || retryingOcr;
    if (isProcessing) {
      const interval = setInterval(() => { fetchDocumentDetails(); }, 3000);
      return () => clearInterval(interval);
    }
  }, [document?.ocr_status, retryingOcr]);

  // --- Render ---

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
          {t('documentDetails.actions.backToDocuments')}
        </Button>
        <Alert severity="error">
          {error || t('documentDetails.errors.notFound')}
        </Alert>
      </Box>
    );
  }

  return (
    <Box sx={{ minHeight: '100vh', backgroundColor: theme.palette.background.default }}>
      <Container maxWidth="lg" sx={{ py: 3 }}>
        {/* Header */}
        <DocumentDetailsHeader
          document={document}
          documentLabels={documentLabels}
          deleting={deleting}
          retryingOcr={retryingOcr}
          onBack={() => navigate('/documents')}
          onDownload={handleDownload}
          onDelete={() => setDeleteConfirmOpen(true)}
          onRetryOcr={handleRetryOcr}
          onEditLabels={() => setShowLabelDialog(true)}
          formatFileSize={formatFileSize}
          formatDate={formatDate}
          t={t}
        />

        {/* Tabs */}
        <Box sx={{ borderBottom: 1, borderColor: 'divider', mb: 3 }}>
          <Tabs
            value={tabValue}
            onChange={handleTabChange}
            variant="scrollable"
            scrollButtons="auto"
            sx={{
              '& .MuiTab-root': {
                textTransform: 'none',
                fontWeight: 600,
                minWidth: 'auto',
                px: 2,
              },
            }}
          >
            <Tab label={t('documentDetails.tabs.ocrText')} />
            <Tab label={t('documentDetails.tabs.preview')} />
          </Tabs>
        </Box>

        {/* Tab content */}
        <Box>
          {tabValue === 0 && (
            <>
              <OcrTextTab
                ocrData={ocrData}
                ocrLoading={ocrLoading}
                ocrText={ocrText}
                t={t}
              />

              {/* Details & Activity below OCR text */}
              <Box sx={{ mt: 4, display: 'flex', gap: 3, flexDirection: { xs: 'column', md: 'row' } }}>
                <Box sx={{ flex: 1 }}>
                  <Typography variant="subtitle2" sx={{ fontWeight: 600, mb: 1.5, color: 'text.secondary', textTransform: 'uppercase', fontSize: '0.7rem', letterSpacing: '0.05em' }}>
                    {t('documentDetails.tabs.details')}
                  </Typography>
                  <Box sx={{ border: `1px solid ${theme.palette.divider}`, borderRadius: 1 }}>
                    <DetailsTab
                      document={document}
                      formatFileSize={formatFileSize}
                      formatDate={formatDate}
                    />
                  </Box>
                </Box>
                <Box sx={{ flex: 1 }}>
                  <Typography variant="subtitle2" sx={{ fontWeight: 600, mb: 1.5, color: 'text.secondary', textTransform: 'uppercase', fontSize: '0.7rem', letterSpacing: '0.05em' }}>
                    {t('documentDetails.tabs.activity')}
                  </Typography>
                  <Box sx={{ border: `1px solid ${theme.palette.divider}`, borderRadius: 1, p: 2 }}>
                    <ActivityTab
                      document={document}
                      ocrData={ocrData}
                      onShowRetryHistory={() => setRetryHistoryModalOpen(true)}
                    />
                  </Box>
                </Box>
              </Box>
            </>
          )}

          {tabValue === 1 && (
            <PreviewTab
              document={document}
              processedImageLoading={processedImageLoading}
              onViewProcessedImage={handleViewProcessedImage}
              t={t}
            />
          )}
        </Box>
      </Container>

      {/* Processed Image Dialog (kept) */}
      <Dialog
        open={showProcessedImageDialog}
        onClose={() => setShowProcessedImageDialog(false)}
        maxWidth="lg"
        fullWidth
      >
        <DialogTitle>
          {t('documentDetails.dialogs.processedImage.title')}
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
                  borderRadius: '4px',
                }}
              />
              <Typography variant="body2" sx={{ mt: 2, color: 'text.secondary' }}>
                {t('documentDetails.dialogs.processedImage.description')}
              </Typography>
            </Box>
          ) : (
            <Box sx={{ textAlign: 'center', py: 4 }}>
              <Typography>{t('documentDetails.dialogs.processedImage.noImage')}</Typography>
            </Box>
          )}
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setShowProcessedImageDialog(false)}>
            {t('common.actions.close')}
          </Button>
        </DialogActions>
      </Dialog>

      {/* Label Edit Dialog (kept) */}
      <Dialog
        open={showLabelDialog}
        onClose={() => setShowLabelDialog(false)}
        maxWidth="md"
        fullWidth
      >
        <DialogTitle>
          {t('documentDetails.dialogs.editLabels.title')}
        </DialogTitle>
        <DialogContent>
          <Box sx={{ mt: 2 }}>
            <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
              {t('documentDetails.dialogs.editLabels.description')}
            </Typography>
            <LabelSelector
              selectedLabels={documentLabels}
              availableLabels={availableLabels}
              onLabelsChange={setDocumentLabels}
              onCreateLabel={handleCreateLabel}
              placeholder={t('documentDetails.dialogs.editLabels.placeholder')}
              size="medium"
              disabled={labelsLoading}
            />
          </Box>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setShowLabelDialog(false)}>
            {t('common.actions.cancel')}
          </Button>
          <Button
            variant="contained"
            onClick={() => handleSaveLabels(documentLabels)}
          >
            {t('documentDetails.dialogs.editLabels.saveLabels')}
          </Button>
        </DialogActions>
      </Dialog>

      {/* Retry History Modal (kept) */}
      {document && (
        <RetryHistoryModal
          open={retryHistoryModalOpen}
          onClose={() => setRetryHistoryModalOpen(false)}
          documentId={document.id}
          documentName={document.original_filename}
        />
      )}

      {/* Delete Confirmation Dialog (kept) */}
      <Dialog
        open={deleteConfirmOpen}
        onClose={() => setDeleteConfirmOpen(false)}
        maxWidth="sm"
        fullWidth
      >
        <DialogTitle>
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
            <DeleteIcon color="error" />
            <Typography variant="h6" sx={{ fontWeight: 600 }}>
              {t('documentDetails.dialogs.delete.title')}
            </Typography>
          </Box>
        </DialogTitle>
        <DialogContent>
          <Alert severity="warning" sx={{ mb: 2 }}>
            {t('documentDetails.dialogs.delete.warning')}
          </Alert>
          <Typography variant="body1" dangerouslySetInnerHTML={{ __html: t('documentDetails.dialogs.delete.message', { filename: document?.original_filename }) }} />
          <Typography variant="body2" color="text.secondary" sx={{ mt: 1 }}>
            {t('documentDetails.dialogs.delete.details')}
          </Typography>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setDeleteConfirmOpen(false)} disabled={deleting}>
            {t('common.actions.cancel')}
          </Button>
          <Button
            variant="contained"
            color="error"
            onClick={handleDeleteDocument}
            disabled={deleting}
            startIcon={deleting ? <CircularProgress size={16} /> : <DeleteIcon />}
          >
            {deleting ? t('documentDetails.dialogs.delete.deleting') : t('documentDetails.dialogs.delete.delete')}
          </Button>
        </DialogActions>
      </Dialog>

      {/* Snackbar */}
      <Snackbar
        open={snackbarOpen}
        autoHideDuration={6000}
        onClose={() => setSnackbarOpen(false)}
        anchorOrigin={{ vertical: 'bottom', horizontal: 'right' }}
      >
        <Alert
          onClose={() => setSnackbarOpen(false)}
          severity={snackbarSeverity}
          sx={{ width: '100%' }}
        >
          {snackbarMessage}
        </Alert>
      </Snackbar>
    </Box>
  );
};

export default DocumentDetailsPage;
