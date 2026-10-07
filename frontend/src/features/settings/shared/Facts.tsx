import type { ReactNode } from 'react';
import styles from './shared.module.css';

// Facts and YesNo were promoted to ui/Pass; re-exported here until callers migrate.
export { Facts, YesNo, type Fact } from '../../../ui/Pass';

/** Small tinted tag. `tone` changes border/text only; the word always carries the meaning. */
export function Tag({ children, tone = 'default' }: { children: ReactNode; tone?: 'default' | 'ok' | 'danger' }) {
  return (
    <span className={styles.tag} data-tone={tone}>
      {children}
    </span>
  );
}
