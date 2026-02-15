import React, { useState, useEffect, useCallback } from 'react';
import {
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  Button,
  Box,
  Typography,
  CircularProgress,
  Alert,
  Divider,
} from '@mui/material';
import { Refresh as RefreshIcon, Language as LanguageIcon } from '@mui/icons-material';
import OcrLanguageSelector from '../OcrLanguageSelector';
import LanguageSelector from '../LanguageSelector';
import { ocrService, ErrorHelper, ErrorCodes, LanguageInfo } from '../../services/api';

interface OcrRetryDialogProps {
  open: boolean;
  onClose: () => void;
  document: {
    id: string;
    filename: string;
    original_filename: string;
    failure_category: string;
    ocr_error: string;
    retry_count: number;
  } | null;
  onRetrySuccess: (message: string) => void;
  onRetryError: (message: string) => void;
}

const OcrRetryDialog: React.FC<OcrRetryDialogProps> = ({
  open,
  onClose,
  document,
  onRetrySuccess,
  onRetryError,
}) => {
  const [selectedLanguage, setSelectedLanguage] = useState<string>('');
  const [selectedLanguages, setSelectedLanguages] = useState<string[]>([]);
  const [primaryLanguage, setPrimaryLanguage] = useState<string>('');
  const [useMultiLanguage, setUseMultiLanguage] = useState<boolean>(false);
  const [retrying, setRetrying] = useState<boolean>(false);
  const [languageMap, setLanguageMap] = useState<Record<string, string>>({});

  useEffect(() => {
    ocrService.getAvailableLanguages()
      .then((response) => {
        const map: Record<string, string> = {};
        response.data.available_languages.forEach((lang: LanguageInfo) => {
          map[lang.code] = lang.name;
        });
        setLanguageMap(map);
      })
      .catch(() => {
        // Fallback handled by getLanguageDisplayName returning the code
      });
  }, []);

  const handleLanguagesChange = (languages: string[], primary?: string) => {
    setSelectedLanguages(languages);
    setPrimaryLanguage(primary || languages[0] || '');
  };

  const getLanguageDisplayName = useCallback((langCode: string): string => {
    return languageMap[langCode] || langCode;
  }, [languageMap]);

  const handleRetry = async () => {
    if (!document) return;

    try {
      setRetrying(true);
      
      // Use multi-language if enabled and languages are selected, otherwise use single language
      const languagesToUse = useMultiLanguage && selectedLanguages.length > 0 
        ? selectedLanguages 
        : undefined;
      const singleLanguageToUse = !useMultiLanguage && selectedLanguage 
        ? selectedLanguage 
        : undefined;
      
      const response = await ocrService.retryWithLanguage(
        document.id, 
        singleLanguageToUse,
        languagesToUse
      );
      
      if (response.data.success) {
        const waitTime = response.data.estimated_wait_minutes || 'Unknown';
        let languageInfo = '';
        
        if (languagesToUse && languagesToUse.length > 0) {
          const langNames = languagesToUse.map(lang => getLanguageDisplayName(lang));
          languageInfo = ` with languages: ${langNames.join(', ')} (Primary: ${getLanguageDisplayName(primaryLanguage)})`;
        } else if (singleLanguageToUse) {
          languageInfo = ` with language "${getLanguageDisplayName(singleLanguageToUse)}"`;
        }
        
        onRetrySuccess(
          `OCR retry queued for "${document.filename}"${languageInfo}. Estimated wait time: ${waitTime} minutes.`
        );
        onClose();
      } else {
        onRetryError(response.data.message || 'Failed to retry OCR processing');
      }
    } catch (error: any) {
      console.error('Failed to retry OCR:', error);
      
      const errorInfo = ErrorHelper.formatErrorForDisplay(error, true);
      let errorMessage = 'Failed to retry OCR processing';
      
      // Handle specific OCR retry errors
      if (ErrorHelper.isErrorCode(error, ErrorCodes.DOCUMENT_NOT_FOUND)) {
        errorMessage = 'Document not found. It may have been deleted or processed already.';
      } else if (ErrorHelper.isErrorCode(error, ErrorCodes.DOCUMENT_OCR_FAILED)) {
        errorMessage = 'Document cannot be retried due to processing issues. Please check the document format.';
      } else if (ErrorHelper.isErrorCode(error, ErrorCodes.OCR_NOT_INSTALLED)) {
        errorMessage = 'OCR engine is not installed or configured. Please contact your administrator.';
      } else if (ErrorHelper.isErrorCode(error, ErrorCodes.OCR_LANG_MISSING)) {
        errorMessage = 'Selected OCR language is not available. Please choose a different language or contact your administrator.';
      } else if (ErrorHelper.isErrorCode(error, ErrorCodes.OCR_TIMEOUT)) {
        errorMessage = 'OCR processing timed out. The document may be too large or complex.';
      } else if (ErrorHelper.isErrorCode(error, ErrorCodes.USER_SESSION_EXPIRED) || 
                 ErrorHelper.isErrorCode(error, ErrorCodes.USER_TOKEN_EXPIRED)) {
        errorMessage = 'Your session has expired. Please refresh the page and log in again.';
      } else if (ErrorHelper.isErrorCode(error, ErrorCodes.USER_PERMISSION_DENIED)) {
        errorMessage = 'You do not have permission to retry OCR processing.';
      } else if (errorInfo.category === 'server') {
        errorMessage = 'Server error. Please try again later or contact support.';
      } else if (errorInfo.category === 'network') {
        errorMessage = 'Network error. Please check your connection and try again.';
      } else {
        errorMessage = errorInfo.message || 'Failed to retry OCR processing';
      }
      
      onRetryError(errorMessage);
    } finally {
      setRetrying(false);
    }
  };

  const handleClose = () => {
    if (!retrying) {
      setSelectedLanguage('');
      setSelectedLanguages([]);
      setPrimaryLanguage('');
      setUseMultiLanguage(false);
      onClose();
    }
  };

  if (!document) return null;

  return (
    <Dialog open={open} onClose={handleClose} maxWidth="sm" fullWidth>
      <DialogTitle>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          <RefreshIcon />
          <Typography variant="h6">Retry OCR Processing</Typography>
        </Box>
      </DialogTitle>
      
      <DialogContent>
        <Box sx={{ mb: 3 }}>
          <Typography variant="subtitle1" sx={{ fontWeight: 600, mb: 1 }}>
            Document: {document.original_filename}
          </Typography>
          <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
            Previous attempts: {document.retry_count}
          </Typography>
          
          {document.failure_category && (
            <Alert severity="warning" sx={{ mb: 2 }}>
              <Typography variant="body2">
                <strong>Previous failure:</strong> {document.failure_category}
              </Typography>
              {document.ocr_error && (
                <Typography variant="caption" sx={{ display: 'block', mt: 1 }}>
                  {document.ocr_error}
                </Typography>
              )}
            </Alert>
          )}
        </Box>

        <Divider sx={{ my: 2 }} />

        <Box sx={{ mb: 3 }}>
          <Typography variant="subtitle2" sx={{ fontWeight: 600, mb: 2, display: 'flex', alignItems: 'center', gap: 1 }}>
            <LanguageIcon fontSize="small" />
            OCR Language Selection
          </Typography>
          <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
            Choose a different language or language combination if the previous OCR attempt failed due to incorrect language settings.
          </Typography>

          {/* Toggle between single and multi-language */}
          <Box sx={{ mb: 2 }}>
            <Button
              variant={!useMultiLanguage ? "contained" : "outlined"}
              size="small"
              onClick={() => setUseMultiLanguage(false)}
              sx={{ mr: 1 }}
            >
              Single Language
            </Button>
            <Button
              variant={useMultiLanguage ? "contained" : "outlined"}
              size="small"
              onClick={() => setUseMultiLanguage(true)}
            >
              Multiple Languages
            </Button>
          </Box>

          {/* Single Language Selector */}
          {!useMultiLanguage && (
            <OcrLanguageSelector
              value={selectedLanguage}
              onChange={setSelectedLanguage}
              label="OCR Language (Optional)"
              size="medium"
              helperText="Leave empty to use your default language setting"
              showCurrentIndicator={true}
            />
          )}

          {/* Multi-Language Selector */}
          {useMultiLanguage && (
            <LanguageSelector
              selectedLanguages={selectedLanguages}
              primaryLanguage={primaryLanguage}
              onLanguagesChange={handleLanguagesChange}
              disabled={retrying}
              showPrimarySelector={true}
              className="w-full"
            />
          )}
        </Box>

        <Alert severity="info" sx={{ mt: 2 }}>
          <Typography variant="body2">
            The retry will use enhanced OCR processing and may take several minutes depending on document size and complexity.
          </Typography>
        </Alert>
      </DialogContent>
      
      <DialogActions sx={{ px: 3, pb: 3 }}>
        <Button onClick={handleClose} disabled={retrying}>
          Cancel
        </Button>
        <Button
          onClick={handleRetry}
          variant="contained"
          disabled={retrying}
          startIcon={retrying ? <CircularProgress size={20} /> : <RefreshIcon />}
        >
          {retrying ? 'Retrying...' : 'Retry OCR'}
        </Button>
      </DialogActions>
    </Dialog>
  );
};

export default OcrRetryDialog;