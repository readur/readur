import type { CSSProperties, ReactNode } from 'react';
import { Button as RACButton } from 'react-aria-components';
import { useTranslation } from 'react-i18next';
import { Close } from '../../ui/icons';
import { labelIcon, swatchStyle, type LabelData } from './labelData';
import styles from './Labels.module.css';

export type { LabelData, LabelDraft } from './labelData';

export interface LabelProps {
  label: LabelData;
  size?: 'small' | 'medium' | 'large';
  variant?: 'filled' | 'outlined';
  /** Show the document count after the name when it is above zero. */
  showCount?: boolean;
  /** Show a remove button (never for system labels). */
  deletable?: boolean;
  onDelete?: (labelId: string) => void;
  /** Makes the tag a button. */
  onClick?: (labelId: string) => void;
  disabled?: boolean;
  className?: string;
}

const cx = (...parts: Array<string | false | undefined>) => parts.filter(Boolean).join(' ');

/**
 * A label tag: a colour swatch plus the name. The name is always shown, so the colour is never
 * the only cue; text sits on the surface colour, never on the label's own colour.
 */
function Label({
  label,
  size = 'medium',
  variant = 'filled',
  showCount = false,
  deletable = false,
  onDelete,
  onClick,
  disabled = false,
  className,
}: LabelProps) {
  const { t } = useTranslation();
  const Icon = labelIcon(label.icon);
  const count = label.document_count ?? 0;
  const canDelete = deletable && !label.is_system;

  const content: ReactNode = (
    <>
      <span className={styles.swatch} style={swatchStyle(label.color) as CSSProperties} data-swatch="" aria-hidden="true" />
      {Icon ? (
        <span className={styles.tagIcon} aria-hidden="true">
          <Icon fontSize="inherit" />
        </span>
      ) : null}
      <span className={styles.tagName}>{label.name}</span>
      {showCount && count > 0 ? <span className={styles.tagCount}>({count})</span> : null}
    </>
  );

  return (
    <span
      className={cx(styles.tag, styles[size], styles[variant], disabled && styles.disabled, className)}
      data-label={label.id}
      data-disabled={disabled || undefined}
    >
      {onClick ? (
        <RACButton className={styles.tagButton} isDisabled={disabled} onPress={() => onClick(label.id)}>
          {content}
        </RACButton>
      ) : (
        <span className={styles.tagBody}>{content}</span>
      )}
      {canDelete ? (
        <RACButton
          className={styles.tagRemove}
          isDisabled={disabled}
          aria-label={t('labels.tag.remove', { name: label.name, defaultValue: 'Remove {{name}}' })}
          onPress={() => onDelete?.(label.id)}
        >
          <Close fontSize="inherit" />
        </RACButton>
      ) : null}
    </span>
  );
}

export { Label };
export default Label;
