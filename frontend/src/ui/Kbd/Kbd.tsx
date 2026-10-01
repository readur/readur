import type { HTMLAttributes } from 'react';
import { cx } from '../shared/FieldParts';
import styles from './Kbd.module.css';

export type KbdProps = HTMLAttributes<HTMLElement>;

/** Mono key cap, e.g. `<Kbd>Esc</Kbd>`. */
export function Kbd({ className, ...rest }: KbdProps) {
  return <kbd {...rest} className={cx(styles.kbd, className)} />;
}
