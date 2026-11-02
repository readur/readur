import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import Label from '../../labels/Label';
import LabelSelector from '../../labels/LabelSelector';
import type { LabelData } from '../../labels';
import { Button } from '../../../ui';
import { Add, Edit } from '../../../ui/icons';
import type { UseDocumentLabelsResult } from '../hooks/useDocumentLabels';
import styles from './DocumentDrawer.module.css';

export interface DrawerLabelsProps {
  labels: UseDocumentLabelsResult;
  tags: string[];
  /** Saves `next`, reporting a failure; the chips update at once. */
  onSave: (next: LabelData[]) => void;
}

/** Tags and labels as chips (× removes a label), and an editor that saves every change at once. */
export function DrawerLabels({ labels, tags, onSave }: DrawerLabelsProps) {
  const { t } = useTranslation();
  const [editing, setEditing] = useState(false);
  const empty = labels.labels.length === 0 && tags.length === 0;

  return (
    <div className={styles.labels}>
      <div className={styles.chips} role="group" aria-label={t('document.pass.labels', 'Labels')}>
        {tags.map((tag) => (
          <span key={`tag-${tag}`} className={styles.tag}>
            {tag}
          </span>
        ))}
        {labels.labels.map((label) => (
          <Label
            key={label.id}
            label={label}
            size="medium"
            deletable={!editing}
            onDelete={(id) => onSave(labels.labels.filter((l) => l.id !== id))}
          />
        ))}
        <Button
          variant="ghost"
          size="sm"
          icon={empty ? <Add fontSize="inherit" /> : <Edit fontSize="inherit" />}
          aria-expanded={editing}
          onPress={() => setEditing((open) => !open)}
        >
          {editing
            ? t('document.labels.done', 'Done')
            : empty
              ? t('document.labels.add', 'Add label')
              : t('document.pass.editLabels', 'Edit labels')}
        </Button>
      </div>
      {editing ? (
        <section className={styles.labelEditor} aria-label={t('document.labels.editor', 'Edit labels')}>
          <LabelSelector
            selectedLabels={labels.labels}
            availableLabels={labels.available}
            onLabelsChange={onSave}
            onCreateLabel={labels.create}
            placeholder={t('document.labels.placeholder', 'Search or create labels…')}
            size="small"
            disabled={labels.isLoading}
          />
        </section>
      ) : null}
    </div>
  );
}
