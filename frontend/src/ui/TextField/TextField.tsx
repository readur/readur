import type { HTMLInputTypeAttribute } from 'react';
import {
  Input,
  TextArea,
  TextField as RACTextField,
  type TextFieldProps as RACTextFieldProps,
} from 'react-aria-components';
import { cx, FieldHelp, FieldLabel, type FieldChromeProps } from '../shared/FieldParts';
import styles from '../shared/field.module.css';

export interface TextFieldProps extends Omit<RACTextFieldProps, 'children' | 'className'>, FieldChromeProps {
  placeholder?: string;
  /** Render a multi-line textarea instead of an input. */
  multiline?: boolean;
  rows?: number;
  type?: HTMLInputTypeAttribute;
  className?: string;
}

export function TextField({
  label,
  description,
  errorMessage,
  placeholder,
  multiline,
  rows,
  className,
  ...rest
}: TextFieldProps) {
  return (
    <RACTextField {...rest} className={cx(styles.field, className)}>
      <FieldLabel label={label} />
      {multiline ? (
        <TextArea className={styles.control} placeholder={placeholder} rows={rows} />
      ) : (
        <Input className={styles.control} placeholder={placeholder} />
      )}
      <FieldHelp description={description} errorMessage={errorMessage} />
    </RACTextField>
  );
}
