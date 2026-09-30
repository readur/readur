import { useId, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '../../ui';
import { cx } from '../../ui/shared/FieldParts';
import styles from './Board.module.css';

export interface RegionProps {
  title: ReactNode;
  /** Right side of the header: a link or a control. */
  headerAction?: ReactNode;
  className?: string;
  children: ReactNode;
}

/** A titled module on the Board grid. */
export function Region({ title, headerAction, className, children }: RegionProps) {
  const headingId = useId();
  return (
    <section aria-labelledby={headingId} className={cx(styles.region, className)}>
      <header className={styles.regionHead}>
        <h2 id={headingId} className={styles.regionTitle}>
          {title}
        </h2>
        {headerAction ? <div className={styles.regionAction}>{headerAction}</div> : null}
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

/** The NEW / CHANGED tag drawn beside a changed row's name. */
export function ChangedTag({ reason }: { reason?: string }) {
  const { t } = useTranslation();
  return (
    <span className={styles.tag}>
      {reason === 'new' ? t('board.tag.new', 'New') : t('board.tag.changed', 'Changed')}
    </span>
  );
}
