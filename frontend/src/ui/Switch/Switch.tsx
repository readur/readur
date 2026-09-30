import type { ReactNode } from 'react';
import { Switch as RACSwitch, type SwitchProps as RACSwitchProps } from 'react-aria-components';
import { cx } from '../shared/FieldParts';
import styles from './Switch.module.css';

export interface SwitchProps extends Omit<RACSwitchProps, 'children' | 'className'> {
  /** Visible label. If omitted, provide `aria-label`. */
  label?: ReactNode;
  description?: ReactNode;
  className?: string;
}

export function Switch({ label, description, className, ...rest }: SwitchProps) {
  return (
    <RACSwitch {...rest} className={cx(styles.switch, className)}>
      <span className={styles.track} aria-hidden="true">
        <span className={styles.thumb} />
      </span>
      {label || description ? (
        <span className={styles.text}>
          {label ? <span>{label}</span> : null}
          {description ? <span className={styles.description}>{description}</span> : null}
        </span>
      ) : null}
    </RACSwitch>
  );
}
