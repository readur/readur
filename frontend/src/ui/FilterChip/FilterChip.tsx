import type { ReactNode } from 'react';
import { Button as RACButton, Dialog as RACDialog } from 'react-aria-components';
import { useTranslation } from 'react-i18next';
import { Close, ExpandMore } from '../icons';
import { Popover, PopoverTrigger } from '../Popover';
import { cx } from '../shared/FieldParts';
import styles from './FilterChip.module.css';

export interface FilterChipProps {
  label: string;
  /** Current value, shown in the data face after the label. */
  value?: ReactNode;
  isActive: boolean;
  onPress?: () => void;
  /** Shows a clear button when provided and the chip is active. */
  onClear?: () => void;
  /**
   * Popover content (for example a checkbox list). When given, the chip toggles a Popover
   * anchored to itself; the clear button stays outside the trigger so it never opens it.
   */
  popover?: ReactNode;
  className?: string;
}

/** Compact 28px filter trigger: condensed label, mono value, optional clear. */
export function FilterChip({ label, value, isActive, onPress, onClear, popover, className }: FilterChipProps) {
  const { t } = useTranslation();
  const hasValue = value !== undefined && value !== null && value !== '';

  const trigger = (
    <RACButton className={styles.trigger} onPress={onPress} aria-pressed={popover ? undefined : isActive}>
      <span className={styles.label}>{label}</span>
      {hasValue ? (
        <>
          {' '}
          <span className={styles.value}>{value}</span>
        </>
      ) : null}
      {popover ? (
        <span className={styles.chevron} aria-hidden="true">
          <ExpandMore fontSize="inherit" />
        </span>
      ) : null}
    </RACButton>
  );

  return (
    <span className={cx(styles.chip, isActive && styles.active, className)} data-active={isActive || undefined}>
      {popover ? (
        <PopoverTrigger>
          {trigger}
          <Popover placement="bottom start">
            <RACDialog className={styles.dialog} aria-label={label}>
              {popover}
            </RACDialog>
          </Popover>
        </PopoverTrigger>
      ) : (
        trigger
      )}
      {isActive && onClear ? (
        <RACButton
          className={styles.clear}
          onPress={onClear}
          aria-label={t('ui.filterChip.clear', { defaultValue: 'Clear {{label}} filter', label })}
        >
          <Close fontSize="inherit" />
        </RACButton>
      ) : null}
    </span>
  );
}
