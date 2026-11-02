import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { queueService, type OcrStatusResponse } from '../../../services/api';
import { useToast } from '../../../ui';

export type OcrStatus = OcrStatusResponse;

/** Admin OCR pause/resume: status plus the two actions, with the previous page's messages. */
export function useOcrStatus(enabled: boolean) {
  const { t } = useTranslation();
  const toast = useToast();
  const [status, setStatus] = useState<OcrStatus | null>(null);
  const [busy, setBusy] = useState(false);

  const refresh = useCallback(async () => {
    try {
      const response = await queueService.getOcrStatus();
      setStatus(response.data);
    } catch {
      // The status endpoint is admin-only; failing quietly matches the previous page.
    }
  }, []);

  useEffect(() => {
    if (enabled) void refresh();
  }, [enabled, refresh]);

  const run = async (action: 'pause' | 'resume') => {
    setBusy(true);
    try {
      if (action === 'pause') await queueService.pauseOcr();
      else await queueService.resumeOcr();
      toast.show({
        title: t(action === 'pause' ? 'settings.messages.ocrPaused' : 'settings.messages.ocrResumed'),
        tone: 'success',
      });
      await refresh();
    } catch (error) {
      const forbidden = (error as { response?: { status?: number } })?.response?.status === 403;
      const key =
        action === 'pause'
          ? forbidden
            ? 'settings.messages.ocrPauseFailed'
            : 'settings.messages.ocrPauseFailedGeneric'
          : forbidden
            ? 'settings.messages.ocrResumeFailed'
            : 'settings.messages.ocrResumeFailedGeneric';
      toast.show({ title: t(key), tone: 'danger' });
    } finally {
      setBusy(false);
    }
  };

  return { status, busy, pause: () => run('pause'), resume: () => run('resume') };
}
