import type { ReactNode } from 'react';
import {
  Checkbox as RACCheckbox,
  CheckboxGroup,
  Label,
  Text,
  type CheckboxGroupProps,
  type CheckboxProps,
} from 'react-aria-components';
import { cx } from '../shared/FieldParts';
import styles from './ChoiceTile.module.css';

export interface ChoiceGroupProps extends Omit<CheckboxGroupProps, 'children' | 'className'> {
  label: ReactNode;
  description?: ReactNode;
  children: ReactNode;
  className?: string;
}

/** A labelled set of ChoiceTiles (multi-select). */
export function ChoiceGroup({ label, description, children, className, ...rest }: ChoiceGroupProps) {
  return (
    <CheckboxGroup {...rest} className={cx(styles.group, className)}>
      <Label className={styles.groupLabel}>{label}</Label>
      {description ? (
        <Text slot="description" className={styles.groupDescription}>
          {description}
        </Text>
      ) : null}
      <div className={styles.tiles}>{children}</div>
    </CheckboxGroup>
  );
}

export interface ChoiceTileProps extends Omit<CheckboxProps, 'children' | 'className'> {
  label: ReactNode;
  description?: ReactNode;
  className?: string;
}

/** A checkbox rendered as a selectable tile: accent border and tint when on. */
export function ChoiceTile({ label, description, className, ...rest }: ChoiceTileProps) {
  return (
    <RACCheckbox {...rest} className={cx(styles.tile, className)}>
      <span className={styles.box} aria-hidden="true">
        <svg viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="2">
          <polyline points="2,6.5 5,9.5 10,3" />
        </svg>
      </span>
      <span className={styles.text}>
        <span className={styles.label}>{label}</span>
        {description ? <span className={styles.description}>{description}</span> : null}
      </span>
    </RACCheckbox>
  );
}
