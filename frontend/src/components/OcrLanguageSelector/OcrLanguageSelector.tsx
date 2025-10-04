import React, { useState, useEffect } from 'react';
import {
  FormControl,
  InputLabel,
  Select,
  MenuItem,
  Typography,
  Box,
  Chip,
  CircularProgress,
  Alert,
  SelectChangeEvent,
} from '@mui/material';
import { Language as LanguageIcon } from '@mui/icons-material';
import { useTranslation } from 'react-i18next';
import { ocrService, LanguageInfo } from '../../services/api';

interface OcrLanguageSelectorProps {
  value?: string;
  onChange: (language: string) => void;
  label?: string;
  size?: 'small' | 'medium';
  fullWidth?: boolean;
  disabled?: boolean;
  showCurrentIndicator?: boolean;
  required?: boolean;
  helperText?: string;
}

const OcrLanguageSelector: React.FC<OcrLanguageSelectorProps> = ({
  value = '',
  onChange,
  label = 'OCR Language',
  size = 'medium',
  fullWidth = true,
  disabled = false,
  showCurrentIndicator = true,
  required = false,
  helperText,
}) => {
  const { t } = useTranslation();
  const [languages, setLanguages] = useState<LanguageInfo[]>([]);
  const [currentUserLanguage, setCurrentUserLanguage] = useState<string>('eng');
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string>('');

  useEffect(() => {
    fetchLanguages();
  }, []);

  const fetchLanguages = async () => {
    try {
      setLoading(true);
      setError('');
      const response = await ocrService.getAvailableLanguages();
      setLanguages(response.data.available_languages);
      setCurrentUserLanguage(response.data.current_user_language);
      
      // If no value is set, default to user's current language
      if (!value) {
        onChange(response.data.current_user_language);
      }
    } catch (err: any) {
      setError(err.response?.data?.message || 'Failed to load OCR languages');
      // Fallback to English if API fails
      setLanguages([
        { code: 'eng', name: 'English', installed: true }
      ]);
      if (!value) {
        onChange('eng');
      }
    } finally {
      setLoading(false);
    }
  };

  const handleChange = (event: SelectChangeEvent) => {
    onChange(event.target.value);
  };

  const getLanguageDisplay = (langCode: string) => {
    const language = languages.find(lang => lang.code === langCode);
    return language ? language.name : langCode;
  };

  if (loading) {
    return (
      <FormControl fullWidth={fullWidth} size={size}>
        <InputLabel>{label}</InputLabel>
        <Box sx={{ display: 'flex', alignItems: 'center', p: 2 }}>
          <CircularProgress size={20} sx={{ mr: 1 }} />
          <Typography variant="body2" color="text.secondary">
            {t('ocr.languageSelector.loading')}
          </Typography>
        </Box>
      </FormControl>
    );
  }

  if (error) {
    return (
      <Box>
        <Alert
          severity="warning"
          sx={{ mb: 1 }}
          action={
            <Typography
              variant="button"
              onClick={fetchLanguages}
              sx={{ cursor: 'pointer', textDecoration: 'underline' }}
            >
              {t('ocr.languageSelector.retry')}
            </Typography>
          }
        >
          {error}
        </Alert>
        <FormControl fullWidth={fullWidth} size={size} disabled>
          <InputLabel>{label}</InputLabel>
          <Select value="eng">
            <MenuItem value="eng">{t('ocr.languageSelector.fallback')}</MenuItem>
          </Select>
        </FormControl>
      </Box>
    );
  }

  return (
    <Box>
      <FormControl fullWidth={fullWidth} size={size} disabled={disabled} required={required}>
        <InputLabel id="ocr-language-label">{label}</InputLabel>
        <Select
          labelId="ocr-language-label"
          value={value || currentUserLanguage}
          onChange={handleChange}
          label={label}
          startAdornment={<LanguageIcon sx={{ mr: 1, color: 'text.secondary' }} />}
        >
          {languages.map((language) => (
            <MenuItem key={language.code} value={language.code}>
              <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', width: '100%' }}>
                <Typography>{language.name}</Typography>
                <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                  <Typography variant="caption" color="text.secondary">
                    {language.code}
                  </Typography>
                  {showCurrentIndicator && language.code === currentUserLanguage && (
                    <Chip
                      label={t('ocr.languageSelector.current')}
                      size="small"
                      color="primary"
                      variant="outlined"
                      sx={{ fontSize: '0.7rem', height: '20px' }}
                    />
                  )}
                </Box>
              </Box>
            </MenuItem>
          ))}
        </Select>
        {helperText && (
          <Typography variant="caption" color="text.secondary" sx={{ mt: 0.5, ml: 1.5 }}>
            {helperText}
          </Typography>
        )}
      </FormControl>


      {showCurrentIndicator && languages.length > 0 && (
        <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mt: 1 }}>
          {t('ocr.languageSelector.languagesAvailable', {
            count: languages.length,
            plural: languages.length !== 1 ? 's' : ''
          })}
          {value && value !== currentUserLanguage && (
            <span> • {t('ocr.languageSelector.selectingWillUpdate', { language: getLanguageDisplay(value) })}</span>
          )}
        </Typography>
      )}
    </Box>
  );
};

export default OcrLanguageSelector;