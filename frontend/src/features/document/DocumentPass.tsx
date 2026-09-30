import { useTranslation } from 'react-i18next';
import type { LabelData } from '../labels';
import type { Document, OcrResponse } from '../../services/api';
import { IconButton, Pass, PassCell, StatusMark } from '../../ui';
import { Edit } from '../../ui/icons';
import { typeCodeOf } from '../../lib/fileType';
import { formatBytes, formatStamp, ocrState, sourceLabel, type OcrExtras } from './format';
import styles from './DocumentPass.module.css';

export interface DocumentPassProps {
  document: Document;
  ocr: OcrResponse | null;
  labels: LabelData[];
  isEditingLabels: boolean;
  onEditLabels: () => void;
}

/** The document's key facts as one segmented strip under the page title. */
export function DocumentPass({ document: doc, ocr, labels, isEditingLabels, onEditLabels }: DocumentPassProps) {
  const { t } = useTranslation();
  const extras = (ocr ?? {}) as OcrExtras;
  const state = ocrState(doc.ocr_status);
  const total = doc.ocr_progress_total ?? 0;
  const current = doc.ocr_progress_current ?? 0;
  const hasProgress = state === 'processing' && total > 0;
  const percent = hasProgress ? Math.round((Math.min(current, total) / total) * 100) : 0;

  const type = typeCodeOf(doc.mime_type, doc.original_filename || doc.filename);
  const pages = extras.pages_processed ?? (total > 0 ? total : null);
  const confidence = ocr?.ocr_confidence ?? doc.ocr_confidence;
  const source = sourceLabel(doc.source_type, (key, fallback) => t(key, fallback));

  return (
    <Pass variant="header" aria-label={t('document.pass.label', 'Document summary')} className={styles.strip}>
      <PassCell label={t('document.pass.status', 'Status')}>
        <span className={styles.status}>
          <StatusMark state={state} progress={hasProgress ? { current, total } : undefined} />
          {hasProgress ? (
            <span
              className={styles.progress}
              role="progressbar"
              aria-label={t('document.pass.progress', 'OCR progress')}
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={percent}
            >
              <span className={styles.progressFill} style={{ width: `${percent}%` }} />
            </span>
          ) : null}
        </span>
      </PassCell>
      <PassCell label={t('document.pass.type', 'Type')} mono>{type}</PassCell>
      <PassCell label={t('document.pass.pages', 'Pages')} mono>{pages ?? '—'}</PassCell>
      <PassCell label={t('document.pass.size', 'Size')} mono>{formatBytes(doc.file_size)}</PassCell>
      <PassCell label={t('document.pass.source', 'Source')} mono>{source}</PassCell>
      <PassCell label={t('document.pass.added', 'Added')} mono>{formatStamp(doc.created_at)}</PassCell>
      <PassCell label={t('document.pass.language', 'Language')} mono>
        {extras.detected_language ? extras.detected_language.toUpperCase() : '—'}
      </PassCell>
      <PassCell label={t('document.pass.confidence', 'Confidence')} mono>
        {confidence != null ? `${Math.round(confidence)}%` : '—'}
      </PassCell>
      <PassCell label={t('document.pass.labels', 'Labels')} span={2}>
        <span className={styles.labels}>
          {labels.length === 0 && (doc.tags ?? []).length === 0 ? (
            <span className={styles.none}>{t('document.pass.noLabels', 'None')}</span>
          ) : null}
          {(doc.tags ?? []).map((tag) => (
            <span key={`tag-${tag}`} className={styles.tag}>
              {tag}
            </span>
          ))}
          {labels.map((label) => (
            <span key={label.id} className={styles.tag}>
              {label.name}
            </span>
          ))}
          <IconButton
            size="sm"
            label={t('document.pass.editLabels', 'Edit labels')}
            icon={<Edit fontSize="inherit" />}
            aria-expanded={isEditingLabels}
            onPress={onEditLabels}
          />
        </span>
      </PassCell>
    </Pass>
  );
}
