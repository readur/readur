import type { ReactNode } from 'react';
import { ToggleButton, ToggleButtonGroup } from 'react-aria-components';
import styles from './Segmented.module.css';

export interface SegmentedItem<T extends string> {
  id: T;
  label: string;
  icon?: ReactNode;
}

export interface SegmentedProps<T extends string> {
  /** Names the group for assistive tech. */
  label: string;
  items: readonly SegmentedItem<T>[];
  value: T;
  onChange: (value: T) => void;
  /** Show only the icons (the label stays as the accessible name). */
  iconOnly?: boolean;
}

/** A two-or-three-way switch, such as Grid | Table or Newest | Best match. Exactly one is on. */
export function Segmented<T extends string>({ label, items, value, onChange, iconOnly = false }: SegmentedProps<T>) {
  return (
    <ToggleButtonGroup
      className={styles.group}
      aria-label={label}
      selectionMode="single"
      disallowEmptySelection
      selectedKeys={[value]}
      onSelectionChange={(keys) => {
        const next = [...keys][0];
        if (next !== undefined) onChange(String(next) as T);
      }}
    >
      {items.map((item) => (
        <ToggleButton
          key={item.id}
          id={item.id}
          className={styles.item}
          aria-label={iconOnly ? item.label : undefined}
        >
          {item.icon ? <span className={styles.icon} aria-hidden="true">{item.icon}</span> : null}
          {iconOnly ? null : <span>{item.label}</span>}
        </ToggleButton>
      ))}
    </ToggleButtonGroup>
  );
}
