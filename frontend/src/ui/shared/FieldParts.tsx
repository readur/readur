import type { ReactNode } from 'react';
import { FieldError, Label, Text, type ValidationResult } from 'react-aria-components';
import styles from './field.module.css';

export type ErrorMessage = ReactNode | ((v: ValidationResult) => ReactNode);

export interface FieldChromeProps {
  label?: ReactNode;
  description?: ReactNode;
  errorMessage?: ErrorMessage;
}

export const cx = (...parts: Array<string | false | null | undefined>) => parts.filter(Boolean).join(' ');

/** Visible label rendered above the control. */
export function FieldLabel({ label }: { label?: ReactNode }) {
  return label ? <Label className={styles.label}>{label}</Label> : null;
}

/** Description + error slots shared by all text-like fields. */
export function FieldHelp({ description, errorMessage }: Omit<FieldChromeProps, 'label'>) {
  return (
    <>
      {description ? (
        <Text slot="description" className={styles.description}>
          {description}
        </Text>
      ) : null}
      <FieldError className={styles.error}>{errorMessage}</FieldError>
    </>
  );
}
