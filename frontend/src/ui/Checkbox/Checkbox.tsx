import { useId, type ReactNode } from 'react';
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
  const uid = useId();
  const descId = description ? `${uid}-desc` : undefined;
  const showError = Boolean(errorMessage && rest.isInvalid);
  const errId = showError ? `${uid}-err` : undefined;
  const describedBy = [rest['aria-describedby'], descId, errId].filter(Boolean).join(' ') || undefined;
  return (
    <div className={styles.wrapper}>
      <RACCheckbox {...rest} aria-describedby={describedBy} className={cx(styles.checkbox, className)}>
        <span className={styles.box} aria-hidden="true">
          <svg className={cx(styles.glyph, styles.check)} viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="2">
            <polyline points="2,6.5 5,9.5 10,3" />
          </svg>
          <svg className={cx(styles.glyph, styles.dash)} viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="2">
            <line x1="2" y1="6" x2="10" y2="6" />
          </svg>
        </span>
        {label ? <span className={styles.text}>{label}</span> : null}
      </RACCheckbox>
      {description ? (
        <span id={descId} className={styles.description}>
          {description}
        </span>
      ) : null}
      {showError ? (
        <span id={errId} className={styles.error}>
          {errorMessage}
        </span>
      ) : null}
    </div>
  );
}
