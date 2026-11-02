import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Label, Slider, SliderOutput, SliderThumb, SliderTrack } from 'react-aria-components';
import { BoardTable, Button, StatusMark, useToast, type BoardColumn } from '../../../ui';
import { documentService } from '../../../services/api';
import { ocrState } from '../../library/format';
import { formatBytes, formatDate } from '../shared/format';
import { ConfirmDialog, Notice, sharedStyles } from '../shared/parts';
import { confidenceText } from './failureLabels';
import { PREVIEW_ROWS, type CleanupDocument, type CleanupResponse } from './cleanupTypes';
import styles from './Attention.module.css';

export const DEFAULT_THRESHOLD = 30;

/**
 * Documents whose OCR confidence is below a threshold. Preview first; deleting is only possible
 * for the threshold that was previewed, and asks for confirmation.
 */
export function LowConfidencePanel() {
  const { t, i18n } = useTranslation();
  const toast = useToast();
  const [threshold, setThreshold] = useState(DEFAULT_THRESHOLD);
  const [preview, setPreview] = useState<(CleanupResponse & { threshold: number }) | null>(null);
  const [loading, setLoading] = useState(false);
  const [confirm, setConfirm] = useState(false);

  const runPreview = async () => {
    setLoading(true);
    try {
      const res = await documentService.deleteLowConfidence(threshold, true);
      setPreview({ ...(res.data as CleanupResponse), threshold });
    } catch {
      toast.show({ title: t('intake.lowConfidence.previewFailed', 'Could not preview low-confidence documents'), tone: 'danger' });
    } finally {
      setLoading(false);
    }
  };

  const remove = async () => {
    if (!preview || preview.matched_count === 0) return;
    setLoading(true);
    try {
      const res = await documentService.deleteLowConfidence(preview.threshold, false);
      toast.show({ title: (res.data as CleanupResponse)?.message || t('intake.lowConfidence.deleted', 'Documents deleted'), tone: 'success' });
      setPreview(null);
      setConfirm(false);
    } catch {
      toast.show({ title: t('intake.lowConfidence.deleteFailed', 'Could not delete the documents'), tone: 'danger' });
    } finally {
      setLoading(false);
    }
  };

  const docs = preview?.documents ?? [];
  const columns: BoardColumn<CleanupDocument>[] = [
    { id: 'name', label: t('intake.lowConfidence.col.name', 'Name'), render: (d) => d.original_filename || d.filename },
    { id: 'size', hideOnNarrow: true, label: t('intake.lowConfidence.col.size', 'Size'), align: 'end', width: 100, render: (d) => formatBytes(d.file_size, 2) },
    {
      id: 'confidence',
      label: t('intake.lowConfidence.col.confidence', 'Confidence'),
      align: 'end',
      width: 120,
      render: (d) => confidenceText(d.ocr_confidence) ?? t('intake.lowConfidence.none', 'n/a'),
    },
    {
      id: 'status',
      hideOnNarrow: true,
      label: t('intake.lowConfidence.col.status', 'Status'),
      width: 120,
      render: (d) => <StatusMark state={ocrState(d.ocr_status)} size="sm" />,
    },
    { id: 'added', hideOnNarrow: true, label: t('intake.lowConfidence.col.added', 'Added'), mono: true, width: 130, render: (d) => formatDate(d.created_at, i18n.language) },
  ];

  return (
    <div className={sharedStyles.section}>
      <p className={sharedStyles.lead}>
        {t(
          'intake.lowConfidence.lead',
          'Find documents whose text recognition confidence is below a threshold. Preview the matches before deleting anything.',
        )}
      </p>
      <div className={styles.thresholdRow}>
        <Slider
          className={styles.slider}
          minValue={0}
          maxValue={100}
          step={1}
          value={threshold}
          onChange={(v) => setThreshold(v as number)}
        >
          <div className={styles.sliderHead}>
            <Label className={sharedStyles.heading}>{t('intake.lowConfidence.threshold', 'Confidence threshold')}</Label>
            <SliderOutput className={sharedStyles.mono}>{({ state }) => `${state.getThumbValue(0)}%`}</SliderOutput>
          </div>
          <SliderTrack className={styles.track}>
            {({ state }) => (
              <>
                <div className={styles.trackFill} style={{ width: `${state.getThumbPercent(0) * 100}%` }} />
                <SliderThumb className={styles.thumb} />
              </>
            )}
          </SliderTrack>
        </Slider>
        <div className={sharedStyles.toolbar}>
          <Button onPress={runPreview} isPending={loading && !confirm}>
            {t('intake.lowConfidence.preview', 'Preview matches')}
          </Button>
          <Button
            variant="danger"
            onPress={() => setConfirm(true)}
            isDisabled={!preview || preview.matched_count === 0 || preview.threshold !== threshold || loading}
          >
            {t('intake.lowConfidence.delete', 'Delete matches')}
          </Button>
        </div>
      </div>
      {preview && preview.threshold !== threshold ? (
        <p className={sharedStyles.meta}>{t('intake.lowConfidence.stale', 'The threshold changed. Preview again before deleting.')}</p>
      ) : null}

      {preview ? (
        <section className={sharedStyles.stack} aria-label={t('intake.lowConfidence.results', 'Preview results')}>
          <Notice tone={preview.matched_count > 0 ? 'info' : 'ok'} live="status" title={preview.message} />
          {docs.length > 0 ? (
            <>
              <BoardTable
                aria-label={t('intake.lowConfidence.results', 'Preview results')}
                columns={columns}
                rows={docs.slice(0, PREVIEW_ROWS)}
                getRowId={(d) => d.id}
                density="compact"
              />
              {docs.length > PREVIEW_ROWS ? (
                <p className={sharedStyles.meta}>
                  {t('intake.lowConfidence.more', 'and {{count}} more', { count: docs.length - PREVIEW_ROWS })}
                </p>
              ) : null}
            </>
          ) : null}
        </section>
      ) : null}

      <ConfirmDialog
        isOpen={confirm}
        onOpenChange={setConfirm}
        title={t('intake.lowConfidence.confirmTitle', 'Delete {{count}} documents below {{threshold}}%?', {
          count: preview?.matched_count ?? 0,
          threshold: preview?.threshold ?? threshold,
        })}
        confirmLabel={t('intake.lowConfidence.confirm', 'Delete documents')}
        isPending={loading}
        onConfirm={remove}
      >
        <p>{t('intake.attention.deleteBody', 'This cannot be undone. The documents and their files are deleted permanently.')}</p>
      </ConfirmDialog>
    </div>
  );
}
