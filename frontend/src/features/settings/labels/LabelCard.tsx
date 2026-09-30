import { useTranslation } from 'react-i18next';
import { IconButton } from '../../../ui';
import { Delete, Edit } from '../../../ui/icons';
import Label, { type LabelData } from '../../labels/Label';
import { Tag } from '../shared/Facts';
import styles from './Labels.module.css';

export interface LabelCardProps {
  label: LabelData;
  /** Only user labels can be edited or deleted. */
  onEdit?: (label: LabelData) => void;
  onDelete?: (label: LabelData) => void;
}

export function LabelCard({ label, onEdit, onDelete }: LabelCardProps) {
  const { t } = useTranslation();
  return (
    <li className={styles.card}>
      <div className={styles.cardHead}>
        <Label label={label} showCount />
        {label.is_system ? (
          <Tag>{t('labels.badge.system')}</Tag>
        ) : (
          <span className={styles.cardActions}>
            {onEdit ? (
              <IconButton
                size="sm"
                label={`${t('labels.actions.editLabel')} ${label.name}`}
                icon={<Edit fontSize="inherit" />}
                onPress={() => onEdit(label)}
              />
            ) : null}
            {onDelete ? (
              <IconButton
                size="sm"
                variant="ghost"
                label={`${t('labels.actions.deleteLabel')} ${label.name}`}
                icon={<Delete fontSize="inherit" />}
                onPress={() => onDelete(label)}
              />
            ) : null}
          </span>
        )}
      </div>
      {label.description ? <p className={styles.description}>{label.description}</p> : null}
      <p className={styles.counts}>
        <span>{t('labels.stats.documents', { count: label.document_count || 0 })}</span>
        <span>{t('labels.stats.sources', { count: label.source_count || 0 })}</span>
      </p>
    </li>
  );
}
