import React, { useEffect, useState } from 'react';
import { Alert, Box, Button, Card, CardContent, Chip, CircularProgress, Divider, Typography } from '@mui/material';
import Grid from '@mui/material/GridLegacy';
import { PlayArrow as PlayArrowIcon, Pause as PauseIcon, Stop as StopIcon } from '@mui/icons-material';
import { useTranslation } from 'react-i18next';
import { queueService } from '../../services/api';

type Severity = 'success' | 'error' | 'warning' | 'info';

interface OcrQueueControlsProps {
  disabled?: boolean;
  onMessage: (message: string, severity: Severity) => void;
}

/** Pause/resume controls for the OCR queue (administrators only). */
const OcrQueueControls: React.FC<OcrQueueControlsProps> = ({ disabled = false, onMessage }) => {
  const { t } = useTranslation();
  const [ocrStatus, setOcrStatus] = useState<{ is_paused: boolean; status: 'paused' | 'running' } | null>(null);
  const [ocrActionLoading, setOcrActionLoading] = useState(false);

  const fetchOcrStatus = async (): Promise<void> => {
    try {
      const response = await queueService.getOcrStatus();
      setOcrStatus(response.data);
    } catch (error: any) {
      console.error('Error fetching OCR status:', error);
    }
  };

  useEffect(() => {
    fetchOcrStatus();
  }, []);

  const runAction = async (action: 'pause' | 'resume'): Promise<void> => {
    setOcrActionLoading(true);
    try {
      if (action === 'pause') {
        await queueService.pauseOcr();
        onMessage(t('settings.messages.ocrPaused'), 'success');
      } else {
        await queueService.resumeOcr();
        onMessage(t('settings.messages.ocrResumed'), 'success');
      }
      fetchOcrStatus(); // Refresh status
    } catch (error: any) {
      console.error(`Error trying to ${action} OCR:`, error);
      const forbidden = error.response?.status === 403;
      if (action === 'pause') {
        onMessage(t(forbidden ? 'settings.messages.ocrPauseFailed' : 'settings.messages.ocrPauseFailedGeneric'), 'error');
      } else {
        onMessage(t(forbidden ? 'settings.messages.ocrResumeFailed' : 'settings.messages.ocrResumeFailedGeneric'), 'error');
      }
    } finally {
      setOcrActionLoading(false);
    }
  };

  return (
    <Card sx={{ mb: 3 }}>
      <CardContent>
        <Typography variant="subtitle1" sx={{ mb: 2 }}>
          <StopIcon sx={{ mr: 1, verticalAlign: 'middle' }} />
          {t('settings.general.ocrControls.title')}
        </Typography>
        <Divider sx={{ mb: 2 }} />

        <Typography variant="body2" sx={{ mb: 2, color: 'text.secondary' }}>
          {t('settings.general.ocrControls.description')}
        </Typography>

        <Grid container spacing={2} alignItems="center">
          <Grid item xs={12} md={6}>
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 2 }}>
              <Button
                variant={ocrStatus?.is_paused ? 'outlined' : 'contained'}
                color={ocrStatus?.is_paused ? 'success' : 'warning'}
                startIcon={ocrActionLoading ? <CircularProgress size={16} /> :
                  (ocrStatus?.is_paused ? <PlayArrowIcon /> : <PauseIcon />)}
                onClick={() => runAction(ocrStatus?.is_paused ? 'resume' : 'pause')}
                disabled={ocrActionLoading || disabled}
                size="large"
              >
                {ocrActionLoading ? t('common.status.processing') :
                  ocrStatus?.is_paused ? t('settings.general.ocrControls.resumeOcr') : t('settings.general.ocrControls.pauseOcr')}
              </Button>
            </Box>
          </Grid>

          <Grid item xs={12} md={6}>
            {ocrStatus && (
              <Box>
                <Chip
                  label={t('settings.general.ocrControls.ocrStatusLabel', { status: ocrStatus.status.toUpperCase() })}
                  color={ocrStatus.is_paused ? 'warning' : 'success'}
                  variant="outlined"
                  icon={ocrStatus.is_paused ? <PauseIcon /> : <PlayArrowIcon />}
                  size="medium"
                />
                <Typography variant="caption" sx={{ display: 'block', mt: 1, color: 'text.secondary' }}>
                  {ocrStatus.is_paused
                    ? t('settings.general.ocrControls.ocrPausedMessage')
                    : t('settings.general.ocrControls.ocrActiveMessage')}
                </Typography>
              </Box>
            )}
          </Grid>
        </Grid>

        {ocrStatus?.is_paused && (
          <Alert severity="warning" sx={{ mt: 2 }}>
            <Typography variant="body2">
              <strong>{t('settings.general.ocrControls.pausedAlertTitle')}</strong><br />
              {t('settings.general.ocrControls.pausedAlertMessage')}
            </Typography>
          </Alert>
        )}
      </CardContent>
    </Card>
  );
};

export default OcrQueueControls;
