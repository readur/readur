import { useId, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { Button, Card, ChangeTag, Notice } from '../../ui';
import { cx } from '../../ui/shared/FieldParts';
import { litTagOf } from './litTag';
import styles from './Home.module.css';

export interface RegionProps {
  title: ReactNode;
  /** A figure beside the title, e.g. how many items the region holds. */
  count?: ReactNode;
  /** Right side of the header: a link or a control. */
  headerAction?: ReactNode;
  /** `card` (default) puts the region on a Card; `plain` leaves the heading on the page background. */
  surface?: 'card' | 'plain';
  className?: string;
  children: ReactNode;
}

/** A titled region on Home: on a Card, or a heading above loose content (cards, notices). */
export function Region({ title, count, headerAction, surface = 'card', className, children }: RegionProps) {
  const headingId = useId();
  const head = (
    <header className={styles.regionHead}>
      <span className={styles.regionTitleRow}>
        <h2 id={headingId} className={styles.regionTitle}>
          {title}
        </h2>
        {count !== undefined ? <span className={styles.regionCount}>{count}</span> : null}
      </span>
      {headerAction ? <div className={styles.regionAction}>{headerAction}</div> : null}
    </header>
  );
  if (surface === 'plain') {
    return (
      <section aria-labelledby={headingId} className={cx(styles.region, className)}>
        {head}
        {children}
      </section>
    );
  }
  return (
    <Card as="section" aria-labelledby={headingId} className={cx(styles.region, className)}>
      {head}
      {children}
    </Card>
  );
}

export interface RegionErrorProps {
  message: ReactNode;
  onRetry: () => void;
}

/** Inline failure notice with a retry, so one failing call never blanks the page. */
export function RegionError({ message, onRetry }: RegionErrorProps) {
  const { t } = useTranslation();
  return (
    <Notice
      tone="danger"
      title={message}
      action={
        <Button size="sm" variant="ghost" onPress={onRetry}>
          {t('board.retry', 'Retry')}
        </Button>
      }
    />
  );
}

/** The New / Changed tag drawn beside an item that changed (see litTag for the vocabulary). */
export function ChangedTag({ reason, className }: { reason?: string; className?: string }) {
  const { t } = useTranslation();
  return (
    <ChangeTag className={className}>
      {litTagOf(reason) === 'new' ? t('board.tag.new', 'New') : t('board.tag.changed', 'Changed')}
    </ChangeTag>
  );
}
