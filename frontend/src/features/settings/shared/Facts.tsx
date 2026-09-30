import type { ReactNode } from 'react';
import { cx } from '../../../ui/shared/FieldParts';
import styles from './shared.module.css';

export interface Fact {
  label: ReactNode;
  value: ReactNode;
  /** Render in the data face (numbers, sizes, ids, paths). */
  mono?: boolean;
}

/** Compact label/value list for read-only diagnostics. */
export function Facts({ items, title, className }: { items: Fact[]; title?: ReactNode; className?: string }) {
  return (
    <div className={cx(styles.factsBox, className)}>
      {title ? <p className={styles.factsTitle}>{title}</p> : null}
      <dl className={styles.facts}>
        {items.map((item, i) => (
          <div key={i} className={styles.fact}>
            <dt>{item.label}</dt>
            <dd className={item.mono ? styles.mono : undefined}>{item.value}</dd>
          </div>
        ))}
      </dl>
    </div>
  );
}

/** A yes/no value shown as a word plus shape (■ yes, ○ no), never colour alone. */
export function YesNo({ value, yes, no }: { value: boolean | null | undefined; yes: string; no: string }) {
  return (
    <span className={styles.yesNo} data-value={value ? 'yes' : 'no'}>
      <span aria-hidden="true">{value ? '■' : '○'}</span> {value ? yes : no}
    </span>
  );
}

/** Small condensed uppercase tag. `tone` changes border/text only; the word always carries the meaning. */
export function Tag({ children, tone = 'default' }: { children: ReactNode; tone?: 'default' | 'ok' | 'danger' }) {
  return (
    <span className={styles.tag} data-tone={tone}>
      {children}
    </span>
  );
}
