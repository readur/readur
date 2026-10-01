import { useId, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { Button, ChangeTag } from '../../ui';
import { cx } from '../../ui/shared/FieldParts';
import { litTagOf } from './litTag';
import styles from './Home.module.css';

export interface RegionProps {
  title: ReactNode;
  /** Right side of the header: a link or a control. */
  headerAction?: ReactNode;
  className?: string;
  children: ReactNode;
}

/** A titled, raised panel on Home. */
export function Region({ title, headerAction, className, children }: RegionProps) {
  const headingId = useId();
  return (
    <section aria-labelledby={headingId} className={cx(styles.panel, className)}>
      <header className={styles.panelHead}>
        <h2 id={headingId} className={styles.panelTitle}>
          {title}
        </h2>
        {headerAction ? <div className={styles.panelAction}>{headerAction}</div> : null}
      </header>
      {children}
    </section>
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
    <div role="alert" className={styles.error}>
      <span className={styles.errorMark} aria-hidden="true">
        ▲
      </span>
      <span className={styles.errorText}>{message}</span>
      <Button size="sm" variant="secondary" onPress={onRetry}>
        {t('board.retry', 'Retry')}
      </Button>
    </div>
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
