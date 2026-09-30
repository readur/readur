import type { ReactNode } from 'react';
import styles from './PageHeader.module.css';

export interface PageHeaderProps {
  /** The page's single h1. */
  title: ReactNode;
  /** Small condensed label above the title. */
  kicker?: ReactNode;
  /** Controls aligned to the right of the title. */
  actions?: ReactNode;
  /** Secondary line under the title (counts, dates, status). */
  meta?: ReactNode;
  /** Id for the h1, so regions can point `aria-labelledby` at it. */
  headingId?: string;
  className?: string;
}

/** Page title block: condensed display h1 with a hairline rule under it and actions on the right. */
export function PageHeader({ title, kicker, actions, meta, headingId, className }: PageHeaderProps) {
  return (
    <div className={className ? `${styles.header} ${className}` : styles.header}>
      <div className={styles.titles}>
        {kicker ? <p className={styles.kicker}>{kicker}</p> : null}
        <h1 id={headingId} className={styles.title}>
          {title}
        </h1>
        {meta ? <div className={styles.meta}>{meta}</div> : null}
      </div>
      {actions ? <div className={styles.actions}>{actions}</div> : null}
    </div>
  );
}
