import type { ElementType, ReactNode } from 'react';
import { Plus } from 'lucide-react';
import styles from './EmptyState.module.css';

export interface EmptyStateProps {
  title: ReactNode;
  description?: ReactNode;
  action?: ReactNode;
  icon?: ReactNode;
  /** Shows the stacked-pages illustration (first-run screens) instead of `icon`. */
  illustration?: boolean;
  /** Heading element for the title. Default `h2`; pick the level that fits the page outline. */
  headingAs?: ElementType;
}

export function EmptyState({ title, description, action, icon, illustration = false, headingAs: Heading = 'h2' }: EmptyStateProps) {
  return (
    <div className={styles.empty}>
      {illustration ? (
        <span className={styles.art} data-illustration="" aria-hidden="true">
          <span className={styles.p1} />
          <span className={styles.p2} />
          <span className={styles.p3}>
            <i />
            <i />
            <i />
            <i />
          </span>
          <span className={styles.badge}>
            <Plus width={14} height={14} strokeWidth={2.5} />
          </span>
        </span>
      ) : icon ? (
        <span className={styles.icon} aria-hidden="true">
          {icon}
        </span>
      ) : null}
      <Heading className={styles.title}>{title}</Heading>
      {description ? <p className={styles.description}>{description}</p> : null}
      {action ? <div className={styles.action}>{action}</div> : null}
    </div>
  );
}
