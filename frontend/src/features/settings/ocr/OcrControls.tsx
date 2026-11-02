import { useTranslation } from 'react-i18next';
import { Button, StatusMark } from '../../../ui';
import { Pause, PlayArrow } from '../../../ui/icons';
import { Notice } from '../shared/Notice';
import shared from '../shared/shared.module.css';
import formStyles from '../form/form.module.css';
import type { OcrStatus } from './useOcrStatus';

export interface OcrControlsProps {
  status: OcrStatus | null;
  busy: boolean;
  onPause: () => void;
  onResume: () => void;
}

/** Admin-only pause/resume of the OCR queue. */
export function OcrControls({ status, busy, onPause, onResume }: OcrControlsProps) {
  const { t } = useTranslation();
  const paused = Boolean(status?.is_paused);
  return (
    <div className={formStyles.form}>
      <p className={formStyles.help}>{t('settings.general.ocrControls.description')}</p>
      <div className={shared.row}>
        <Button
          variant={paused ? 'primary' : 'secondary'}
          icon={paused ? <PlayArrow fontSize="inherit" /> : <Pause fontSize="inherit" />}
          onPress={paused ? onResume : onPause}
          isPending={busy}
        >
          {paused ? t('settings.general.ocrControls.resumeOcr') : t('settings.general.ocrControls.pauseOcr')}
        </Button>
        {status ? (
          <span className={shared.row}>
            <StatusMark state={paused ? 'disabled' : 'healthy'} />
            <span className={shared.meta}>
              {t('settings.general.ocrControls.ocrStatusLabel', { status: status.status.toUpperCase() })}
            </span>
          </span>
        ) : null}
      </div>
      {status ? (
        <p className={formStyles.help}>
          {paused
            ? t('settings.general.ocrControls.ocrPausedMessage')
            : t('settings.general.ocrControls.ocrActiveMessage')}
        </p>
      ) : null}
      {paused ? (
        <Notice tone="warning" title={t('settings.general.ocrControls.pausedAlertTitle')}>
          {t('settings.general.ocrControls.pausedAlertMessage')}
        </Notice>
      ) : null}
    </div>
  );
}
