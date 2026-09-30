import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button, useToast } from '../../../ui';
import { Pause, PlayArrow } from '../../../ui/icons';
import { queueService } from '../../../services/api';
import { useLoader } from '../shared/useLoader';

/** Admin-only switch that pauses or resumes OCR processing for every user. */
export function OcrControls() {
  const { t } = useTranslation();
  const toast = useToast();
  const [pending, setPending] = useState(false);
  const status = useLoader(async () => (await queueService.getOcrStatus()).data, []);

  if (!status.data) return null;
  const paused = status.data.is_paused;

  const toggle = async () => {
    setPending(true);
    try {
      if (paused) await queueService.resumeOcr();
      else await queueService.pauseOcr();
      await status.reload();
      toast.show({
        title: paused ? t('intake.ocr.resumed', 'OCR resumed') : t('intake.ocr.paused', 'OCR paused'),
        tone: 'success',
      });
    } catch {
      toast.show({
        title: paused ? t('intake.ocr.resumeFailed', 'Could not resume OCR') : t('intake.ocr.pauseFailed', 'Could not pause OCR'),
        tone: 'danger',
      });
    } finally {
      setPending(false);
    }
  };

  return (
    <Button
      onPress={toggle}
      isPending={pending}
      icon={paused ? <PlayArrow fontSize="inherit" /> : <Pause fontSize="inherit" />}
    >
      {paused ? t('intake.ocr.resume', 'Resume OCR') : t('intake.ocr.pause', 'Pause OCR')}
    </Button>
  );
}
