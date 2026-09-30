import type { ReactNode } from 'react';
import { Checkbox as RACCheckbox, type CheckboxProps as RACCheckboxProps } from 'react-aria-components';
import { cx } from '../shared/FieldParts';
import styles from './Checkbox.module.css';

export interface CheckboxProps extends Omit<RACCheckboxProps, 'children' | 'className'> {
  /** Visible label. If omitted, provide `aria-label`. */
  label?: ReactNode;
  description?: ReactNode;
  errorMessage?: ReactNode;
  className?: string;
}

export function Checkbox({ label, description, errorMessage, className, ...rest }: CheckboxProps) {
  return (
    <RACCheckbox {...rest} className={cx(styles.checkbox, className)}>
      <span className={styles.box} aria-hidden="true">
        <svg className={cx(styles.glyph, styles.check)} viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="2">
          <polyline points="2,6.5 5,9.5 10,3" />
        </svg>
        <svg className={cx(styles.glyph, styles.dash)} viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="2">
          <line x1="2" y1="6" x2="10" y2="6" />
        </svg>
      </span>
      {label || description || errorMessage ? (
        <span className={styles.text}>
          {label ? <span>{label}</span> : null}
          {description ? <span className={styles.description}>{description}</span> : null}
          {errorMessage && rest.isInvalid ? <span className={styles.error}>{errorMessage}</span> : null}
        </span>
      ) : null}
    </RACCheckbox>
  );
}
