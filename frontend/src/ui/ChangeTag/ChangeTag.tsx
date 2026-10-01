import type { ReactNode } from 'react';
import styles from './ChangeTag.module.css';

export interface ChangeTagProps {
  /** The translated word ("New", "Changed"). */
  children: ReactNode;
  /** Extra class for layout by the surface that places the tag. */
  className?: string;
}

/** The filled tag beside a row that changed since the user last looked. One style everywhere. */
export function ChangeTag({ children, className }: ChangeTagProps) {
  return <span className={className ? `${styles.tag} ${className}` : styles.tag}>{children}</span>;
}
