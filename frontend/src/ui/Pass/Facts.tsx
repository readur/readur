import type { ReactNode } from 'react';
import { cx } from '../shared/FieldParts';
import styles from './Facts.module.css';

export interface Fact {
  label: ReactNode;
  value: ReactNode;
  /** Render in the data face (numbers, sizes, ids, paths). */
  mono?: boolean;
}

/** Read-only label/value list on a card: label column, hairline rows. */
export function Facts({ items, title, className }: { items: Fact[]; title?: ReactNode; className?: string }) {
  return (
    <div className={cx(styles.box, className)}>
      {title ? <p className={styles.title}>{title}</p> : null}
      <dl className={styles.list}>
        {items.map((item, i) => (
          <div key={i} className={styles.fact}>
            <dt>{item.label}</dt>
            <dd data-mono={item.mono || undefined}>{item.value}</dd>
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
