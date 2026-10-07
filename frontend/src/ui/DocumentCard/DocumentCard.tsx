import type { ReactNode } from 'react';
import { Button as RACButton, Link } from 'react-aria-components';
import { Checkbox } from '../Checkbox';
import styles from './DocumentCard.module.css';

export interface DocumentCardProps {
  title: string;
  /** The page preview, typically `DocumentThumbnail size="fill"`. */
  thumbnail: ReactNode;
  /** Mono line: type, pages, size, date. */
  meta?: ReactNode;
  status?: ReactNode;
  labels?: ReactNode;
  source?: ReactNode;
  /** Marks such as a ChangeTag, shown at the end of the status row (never over the preview). */
  flags?: ReactNode;
  /** Opens as a link when set; otherwise the title is a button calling `onOpen`. */
  href?: string;
  onOpen?: () => void;
  isSelected?: boolean;
  /** With `selectLabel`, adds a selection checkbox that appears on hover, focus or selection. */
  onSelectionChange?: (selected: boolean) => void;
  selectLabel?: string;
  /** Rings the card with the new-signal colour. */
  isChanged?: boolean;
  /** Raised icon buttons shown on hover / focus. */
  quickActions?: ReactNode;
  as?: 'li' | 'div' | 'article';
}

/** A document in a grid: tall page preview on a tinted well, then name, meta, status, labels and source. */
export function DocumentCard({
  title,
  thumbnail,
  meta,
  status,
  labels,
  source,
  flags,
  href,
  onOpen,
  isSelected,
  onSelectionChange,
  selectLabel,
  isChanged,
  quickActions,
  as: El = 'li',
}: DocumentCardProps) {
  const body = (
    <>
      <span className={styles.thumb} data-thumb="">
        {thumbnail}
      </span>
      <span className={styles.name}>{title}</span>
    </>
  );
  return (
    <El className={styles.card} data-selected={isSelected || undefined} data-changed={isChanged || undefined}>
      {href ? (
        <Link href={href} className={styles.open} onPress={onOpen}>
          {body}
        </Link>
      ) : (
        <RACButton className={styles.open} onPress={onOpen}>
          {body}
        </RACButton>
      )}
      <span className={styles.details} data-details="">
        {meta ? <span className={styles.meta}>{meta}</span> : null}
        {status || flags ? (
          <span className={styles.statusRow}>
            {status}
            {flags ? <span className={styles.flags}>{flags}</span> : null}
          </span>
        ) : null}
        {labels ? <span className={styles.row}>{labels}</span> : null}
        {source ? <span className={styles.row}>{source}</span> : null}
      </span>
      {onSelectionChange && selectLabel ? (
        <span className={styles.check} data-card-check="">
          <Checkbox aria-label={selectLabel} isSelected={!!isSelected} onChange={onSelectionChange} />
        </span>
      ) : null}
      {quickActions ? <span className={styles.actions}>{quickActions}</span> : null}
    </El>
  );
}
