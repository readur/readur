import { useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { StatusMark, Tooltip, TooltipTrigger } from '../../ui';
import { useLit } from '../board/litStore';
import { DocumentThumbnail } from '../document/DocumentThumbnail';
import { Label } from '../labels';
import { displayName, type LibraryRow } from './data';
import { formatDateTime, formatRelative, ocrState } from './format';
import styles from './Library.module.css';

/** NEW / CHANGED tag for rows that changed since the user last opened them. */
export function ChangeTag({ id }: { id: string }) {
  const { t } = useTranslation();
  const { lit, reason } = useLit('document', id);
  if (!lit) return null;
  return (
    <span className={styles.changeTag}>
      {reason === 'new' ? t('library.tag.new', 'NEW') : t('library.tag.changed', 'CHANGED')}
    </span>
  );
}

export function NameCell({ row }: { row: LibraryRow }) {
  return (
    <span className={styles.nameCell}>
      <span className={styles.thumb}>
        <DocumentThumbnail documentId={row.id} mimeType={row.mime_type} size="row" lazy />
      </span>
      <span className={styles.name}>{displayName(row)}</span>
      <ChangeTag id={row.id} />
    </span>
  );
}

export function StatusCell({ row }: { row: LibraryRow }) {
  const progress =
    row.ocr_progress_total && row.ocr_progress_total > 0
      ? { current: row.ocr_progress_current ?? 0, total: row.ocr_progress_total }
      : undefined;
  return <StatusMark state={ocrState(row.ocr_status)} progress={progress} size="sm" />;
}

export function LabelsCell({ row }: { row: LibraryRow }) {
  const { t } = useTranslation();
  if (row.labels.length === 0) return <span className={styles.none}>—</span>;
  const extra = row.labels.length - 2;
  return (
    <span className={styles.labelsCell}>
      {row.labels.slice(0, 2).map((l) => (
        <Label key={l.id} label={l} size="small" />
      ))}
      {extra > 0 ? (
        <span className={styles.more} aria-label={t('library.labelsMore', { count: extra, defaultValue: '{{count}} more' })}>
          +{extra}
        </span>
      ) : null}
    </span>
  );
}

/** Relative time, with the full date in a tooltip on hover and as text for assistive tech. */
export function AddedCell({ value }: { value: string }) {
  const { i18n } = useTranslation();
  const ref = useRef<HTMLSpanElement>(null);
  const [open, setOpen] = useState(false);
  const absolute = formatDateTime(value, i18n.language);
  return (
    <TooltipTrigger isOpen={open} onOpenChange={setOpen}>
      <span ref={ref} className={styles.added} onMouseEnter={() => setOpen(true)} onMouseLeave={() => setOpen(false)}>
        <time dateTime={value} aria-hidden="true">
          {formatRelative(value, i18n.language)}
        </time>
        <span className={styles.srOnly}>{absolute}</span>
      </span>
      <Tooltip triggerRef={ref}>{absolute}</Tooltip>
    </TooltipTrigger>
  );
}
