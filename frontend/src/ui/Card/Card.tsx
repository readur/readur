import type { HTMLAttributes } from 'react';
import { cx } from '../shared/FieldParts';
import styles from './Card.module.css';

export interface CardProps extends HTMLAttributes<HTMLElement> {
  as?: 'div' | 'section' | 'article' | 'li';
  padding?: 'none' | 'sm' | 'md';
  interactive?: boolean;
}

/** The Studio surface: a panel with 14px corners and a soft shadow. */
export function Card({ as: El = 'div', padding = 'md', interactive = false, className, ...rest }: CardProps) {
  return <El {...rest} data-padding={padding} data-interactive={interactive || undefined} className={cx(styles.card, className)} />;
}
