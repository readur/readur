import type { CSSProperties, ReactNode } from 'react';
import { Button as RACButton } from 'react-aria-components';
import { Close } from '../icons';
import { cx } from '../shared/FieldParts';
import styles from './Label.module.css';

export interface LabelChipProps {
  name: string;
  /** Any CSS colour for the dot; omitted or invalid values give a neutral dot. */
  color?: string;
  icon?: ReactNode;
  /** Shown as `(n)` after the name when above zero. */
  count?: number;
  size?: 'small' | 'medium' | 'large';
  variant?: 'filled' | 'outlined';
  /** Makes the chip a button. */
  onPress?: () => void;
  /** Shows a remove button named by `removeLabel`. */
  onRemove?: () => void;
  removeLabel?: string;
  isDisabled?: boolean;
  className?: string;
  /** Rendered as `data-label` so callers can find the chip for a label id. */
  dataId?: string;
}

const HEX = /^#(?:[0-9a-f]{3,4}|[0-9a-f]{6}|[0-9a-f]{8})$/i;

/** True for a value the browser accepts as a colour (hex only where CSS.supports is unavailable). */
function isColor(value: string): boolean {
  const v = value.trim();
  if (!v) return false;
  if (typeof CSS !== 'undefined' && typeof CSS.supports === 'function') return CSS.supports('color', v);
  return HEX.test(v);
}

/**
 * A label: a colour dot plus the name on a neutral pill. The name is always shown, so the
 * colour is never the only cue, and text never sits on the label's own colour.
 */
export function LabelChip({
  name,
  color,
  icon,
  count,
  size = 'medium',
  variant = 'filled',
  onPress,
  onRemove,
  removeLabel,
  isDisabled = false,
  className,
  dataId,
}: LabelChipProps) {
  // An invalid value would make the custom property invalid and hide the dot, so fall back to neutral.
  const style = color && isColor(color) ? ({ '--label-color': color.trim() } as CSSProperties) : undefined;
  const content: ReactNode = (
    <>
      <span className={styles.swatch} data-swatch="" aria-hidden="true" />
      {icon ? (
        <span className={styles.tagIcon} aria-hidden="true">
          {icon}
        </span>
      ) : null}
      <span className={styles.tagName}>{name}</span>
      {count !== undefined && count > 0 ? <span className={styles.tagCount}>({count})</span> : null}
    </>
  );

  return (
    <span
      className={cx(styles.tag, styles[size], styles[variant], isDisabled && styles.disabled, className)}
      style={style}
      data-label={dataId}
      data-disabled={isDisabled || undefined}
    >
      {onPress ? (
        <RACButton className={styles.tagButton} isDisabled={isDisabled} onPress={onPress}>
          {content}
        </RACButton>
      ) : (
        <span className={styles.tagBody}>{content}</span>
      )}
      {onRemove ? (
        <RACButton className={styles.tagRemove} isDisabled={isDisabled} aria-label={removeLabel} onPress={onRemove}>
          <Close fontSize="inherit" />
        </RACButton>
      ) : null}
    </span>
  );
}
