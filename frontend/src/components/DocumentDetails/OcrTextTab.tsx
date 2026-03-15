import React, { useState, useMemo } from 'react';
import {
  Box,
  Typography,
  TextField,
  InputAdornment,
  IconButton,
  Paper,
  CircularProgress,
  Alert,
  FormControlLabel,
  Checkbox,
} from '@mui/material';
import {
  Search as SearchIcon,
  Close as CloseIcon,
} from '@mui/icons-material';
import { useTheme as useMuiTheme } from '@mui/material/styles';
import { type OcrResponse } from '../../services/api';

interface OcrTextTabProps {
  ocrData: OcrResponse | null;
  ocrLoading: boolean;
  ocrText: string;
  t: (key: string, options?: any) => string;
}

const OcrTextTab: React.FC<OcrTextTabProps> = ({
  ocrData,
  ocrLoading,
  ocrText,
  t,
}) => {
  const theme = useMuiTheme();
  const [searchTerm, setSearchTerm] = useState('');
  const [showLineNumbers, setShowLineNumbers] = useState(false);

  const matchCount = useMemo(() => {
    if (!searchTerm || !ocrData?.ocr_text) return 0;
    return ocrData.ocr_text.toLowerCase().split(searchTerm.toLowerCase()).length - 1;
  }, [searchTerm, ocrData?.ocr_text]);

  const renderedHtml = useMemo(() => {
    if (!ocrData?.ocr_text) return '';

    let text = ocrData.ocr_text;

    // Apply search highlighting first (on raw text)
    if (searchTerm) {
      text = text.replace(
        new RegExp(`(${searchTerm.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`, 'gi'),
        '<mark style="background-color: #ffeb3b; color: #000; padding: 1px 2px; border-radius: 2px;">$1</mark>'
      );
    }

    if (showLineNumbers) {
      const lines = text.split('\n');
      const gutterWidth = String(lines.length).length;
      return lines
        .map((line, i) => {
          const num = String(i + 1).padStart(gutterWidth, ' ');
          return `<span style="color: ${theme.palette.text.disabled}; user-select: none; display: inline-block; width: ${gutterWidth + 1}ch; text-align: right; margin-right: 1.5ch; flex-shrink: 0;">${num}</span>${line}`;
        })
        .join('\n');
    }

    return text;
  }, [searchTerm, ocrData?.ocr_text, showLineNumbers, theme.palette.text.disabled]);

  if (ocrLoading) {
    return (
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', py: 8 }}>
        <CircularProgress size={24} sx={{ mr: 2 }} />
        <Typography variant="body2" color="text.secondary">
          {t('documentDetails.ocr.loading')}
        </Typography>
      </Box>
    );
  }

  if (!ocrData) {
    return (
      <Alert severity="info">
        {t('documentDetails.ocr.loadFailed')}
      </Alert>
    );
  }

  // Metadata line
  const metaParts: string[] = [];
  if (ocrData.ocr_confidence) {
    metaParts.push(`${Math.round(ocrData.ocr_confidence)}% confidence`);
  }
  if (ocrData.ocr_word_count != null) {
    metaParts.push(`${ocrData.ocr_word_count.toLocaleString()} words`);
  }
  if (ocrData.ocr_processing_time_ms) {
    metaParts.push(`${ocrData.ocr_processing_time_ms}ms`);
  }
  if (ocrData.ocr_completed_at) {
    metaParts.push(`Completed ${new Date(ocrData.ocr_completed_at).toLocaleDateString('en-US', { month: 'short', day: 'numeric' })}`);
  }

  return (
    <Box>
      {/* Search bar + options row */}
      <Box sx={{ display: 'flex', gap: 1.5, alignItems: 'center', mb: 1.5 }}>
        <TextField
          fullWidth
          size="small"
          variant="outlined"
          placeholder={t('documentDetails.dialogs.ocrExpanded.searchPlaceholder')}
          value={searchTerm}
          onChange={(e) => setSearchTerm(e.target.value)}
          InputProps={{
            startAdornment: (
              <InputAdornment position="start">
                <SearchIcon color="action" sx={{ fontSize: 20 }} />
              </InputAdornment>
            ),
            endAdornment: searchTerm ? (
              <InputAdornment position="end">
                <IconButton size="small" onClick={() => setSearchTerm('')}>
                  <CloseIcon fontSize="small" />
                </IconButton>
              </InputAdornment>
            ) : undefined,
          }}
        />
        <FormControlLabel
          control={
            <Checkbox
              size="small"
              checked={showLineNumbers}
              onChange={(e) => setShowLineNumbers(e.target.checked)}
            />
          }
          label={
            <Typography variant="body2" sx={{ whiteSpace: 'nowrap' }}>
              Line numbers
            </Typography>
          }
          sx={{ flexShrink: 0, mr: 0 }}
        />
      </Box>

      {/* Search results count + OCR metadata */}
      <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 2 }}>
        {searchTerm
          ? matchCount > 0
            ? `${matchCount} match${matchCount === 1 ? '' : 'es'} found`
            : 'No matches found'
          : metaParts.join(' · ')
        }
      </Typography>

      {/* OCR error */}
      {ocrData.ocr_error && (
        <Alert severity="error" sx={{ mb: 2 }}>
          <Typography variant="body2">{ocrData.ocr_error}</Typography>
        </Alert>
      )}

      {/* OCR text */}
      {ocrData.ocr_text ? (
        <Paper
          sx={{
            p: 3,
            backgroundColor: theme.palette.background.default,
            border: `1px solid ${theme.palette.divider}`,
            borderRadius: 1,
            maxHeight: 'calc(100vh - 380px)',
            minHeight: 300,
            overflow: 'auto',
            scrollbarWidth: 'thin',
          }}
        >
          <Typography
            component="pre"
            sx={{
              fontFamily: showLineNumbers ? 'monospace' : '"Inter", sans-serif',
              whiteSpace: 'pre-wrap',
              lineHeight: 1.8,
              fontSize: '0.95rem',
              margin: 0,
            }}
            dangerouslySetInnerHTML={{ __html: renderedHtml }}
          />
        </Paper>
      ) : (
        <Typography variant="body2" color="text.secondary" sx={{ fontStyle: 'italic', py: 4, textAlign: 'center' }}>
          {t('documentDetails.ocr.noText')}
        </Typography>
      )}
    </Box>
  );
};

export default OcrTextTab;
