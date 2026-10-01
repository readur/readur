import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import Label from '../labels/Label';
import type { LabelData } from '../labels/Label';
import { Button } from '../../ui';
import { Add, Edit, ExpandLess, ExpandMore } from '../../ui/icons';
import styles from './DocumentToolbar.module.css';

export interface DocumentToolbarProps {
  labels: LabelData[];
  tags: string[];
  isEditingLabels: boolean;
  onEditLabels: () => void;
  detailsId: string;
  isDetailsOpen: boolean;
  onToggleDetails: () => void;
  /** The Document | Side by side | Text switch. */
  viewSwitch: ReactNode;
}

/** Labels as colour chips, the Details disclosure and the view switch, on one row. */
export function DocumentToolbar({
  labels,
  tags,
  isEditingLabels,
  onEditLabels,
  detailsId,
  isDetailsOpen,
  onToggleDetails,
  viewSwitch,
}: DocumentToolbarProps) {
  const { t } = useTranslation();
  const empty = labels.length === 0 && tags.length === 0;

  return (
    <div className={styles.toolbar}>
      <div className={styles.labels} role="group" aria-label={t('document.pass.labels', 'Labels')}>
        {tags.map((tag) => (
          <span key={`tag-${tag}`} className={styles.tag}>
            {tag}
          </span>
        ))}
        {labels.map((label) => (
          <Label key={label.id} label={label} size="medium" />
        ))}
        <Button
          variant="ghost"
          size="sm"
          icon={empty ? <Add fontSize="inherit" /> : <Edit fontSize="inherit" />}
          aria-expanded={isEditingLabels}
          onPress={onEditLabels}
        >
          {empty ? t('document.labels.add', 'Add label') : t('document.pass.editLabels', 'Edit labels')}
        </Button>
      </div>
      <div className={styles.end}>
        <Button
          variant="ghost"
          size="sm"
          aria-expanded={isDetailsOpen}
          aria-controls={isDetailsOpen ? detailsId : undefined}
          icon={isDetailsOpen ? <ExpandLess fontSize="inherit" /> : <ExpandMore fontSize="inherit" />}
          onPress={onToggleDetails}
        >
          {t('document.details.title', 'Details')}
        </Button>
        {viewSwitch}
      </div>
    </div>
  );
}
