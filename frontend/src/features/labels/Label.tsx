import { useTranslation } from 'react-i18next';
import { LabelChip } from '../../ui/Label';
import { isHexColor, labelIcon, type LabelData } from './labelData';

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

/**
 * A label tag for a LabelData record: the ui LabelChip (colour dot plus name) with the label's
 * colour, icon, count and remove action wired up. System labels are never removable.
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
  const canDelete = deletable && !label.is_system;
  return (
    <LabelChip
      name={label.name}
      color={label.color && isHexColor(label.color) ? label.color.trim() : undefined}
      icon={Icon ? <Icon fontSize="inherit" /> : undefined}
      count={showCount ? (label.document_count ?? 0) : undefined}
      size={size}
      variant={variant}
      onPress={onClick ? () => onClick(label.id) : undefined}
      onRemove={canDelete ? () => onDelete?.(label.id) : undefined}
      removeLabel={t('labels.tag.remove', { name: label.name, defaultValue: 'Remove {{name}}' })}
      isDisabled={disabled}
      className={className}
      dataId={label.id}
    />
  );
}

export { Label };
export default Label;
