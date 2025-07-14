import React, { useState } from 'react';
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
import { ocrService } from '../../services/api';

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

  const handleLanguagesChange = (languages: string[], primary?: string) => {
    setSelectedLanguages(languages);
    setPrimaryLanguage(primary || languages[0] || '');
  };

  // Simple language code to name mapping for display
  const getLanguageDisplayName = (langCode: string): string => {
    const languageNames: Record<string, string> = {
      'eng': 'English',
      'spa': 'Spanish',  
      'fra': 'French',
      'deu': 'German',
      'ita': 'Italian',
      'por': 'Portuguese',
      'rus': 'Russian',
      'jpn': 'Japanese',
      'chi_sim': 'Chinese (Simplified)',
      'chi_tra': 'Chinese (Traditional)',
      'kor': 'Korean',
      'ara': 'Arabic',
      'hin': 'Hindi',
      'tha': 'Thai',
      'vie': 'Vietnamese',
      'pol': 'Polish',
      'nld': 'Dutch',
      'dan': 'Danish',
      'nor': 'Norwegian',
      'swe': 'Swedish',
      'fin': 'Finnish',
      'ces': 'Czech',
      'hun': 'Hungarian',
      'tur': 'Turkish',
      'heb': 'Hebrew',
      'ukr': 'Ukrainian',
      'bul': 'Bulgarian',
      'ron': 'Romanian',
      'hrv': 'Croatian',
      'slk': 'Slovak',
      'slv': 'Slovenian',
      'est': 'Estonian',
      'lav': 'Latvian',
      'lit': 'Lithuanian',
      'ell': 'Greek',
      'cat': 'Catalan',
      'eus': 'Basque',
      'gla': 'Scottish Gaelic',
      'gle': 'Irish',
      'cym': 'Welsh',
      'isl': 'Icelandic',
      'mlt': 'Maltese',
      'afr': 'Afrikaans',
    };
    return languageNames[langCode] || langCode;
  };

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
      onRetryError(
        error.response?.data?.message || 'Failed to retry OCR processing'
      );
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