import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button, Dialog, EmptyState, SearchField, Switch } from '../../../ui';
import { Add } from '../../../ui/icons';
import LabelCreateDialog from '../../labels/LabelCreateDialog';
import type { LabelData } from '../../labels/Label';
import { Notice } from '../shared/Notice';
import shared from '../shared/shared.module.css';
import { LabelCard } from './LabelCard';
import styles from './Labels.module.css';
import { useLabels } from './useLabels';

/** Label management: search, system-label filter, grouped cards with usage counts, create/edit/delete. */
export default function LabelsSection() {
  const { t } = useTranslation();
  const { labels, isLoading, error, setError, createLabel, updateLabel, deleteLabel } = useLabels();
  const [search, setSearch] = useState('');
  const [showSystem, setShowSystem] = useState(true);
  const [createOpen, setCreateOpen] = useState(false);
  const [editing, setEditing] = useState<LabelData | null>(null);
  const [toDelete, setToDelete] = useState<LabelData | null>(null);
  const [deleting, setDeleting] = useState(false);

  const q = search.toLowerCase();
  const filtered = labels.filter(
    (l) =>
      (l.name.toLowerCase().includes(q) || (l.description || '').toLowerCase().includes(q)) &&
      (showSystem || !l.is_system),
  );
  const systemLabels = filtered.filter((l) => l.is_system);
  const userLabels = filtered.filter((l) => !l.is_system);

  const confirmDelete = async () => {
    if (!toDelete) return;
    setDeleting(true);
    const done = await deleteLabel(toDelete.id);
    setDeleting(false);
    if (done) setToDelete(null);
  };

  if (isLoading && labels.length === 0 && !error) {
    return (
      <p className={shared.meta} role="status">
        {t('labels.loading')}
      </p>
    );
  }

  const createButton = (
    <Button variant="primary" icon={<Add fontSize="inherit" />} onPress={() => setCreateOpen(true)}>
      {t('labels.actions.createLabel')}
    </Button>
  );

  return (
    <div className={shared.stack}>
      <div className={shared.sectionHead}>
        <p className={shared.sectionIntro}>
          {t('settings.labels.intro', 'Labels organise documents and sources. Counts show how often each label is used.')}
        </p>
        {createButton}
      </div>

      {error && !toDelete ? (
        <Notice tone="danger" onDismiss={() => setError(null)}>
          {error}
        </Notice>
      ) : null}

      <div className={styles.filters}>
        <SearchField
          aria-label={t('labels.search.placeholder')}
          placeholder={t('labels.search.placeholder')}
          value={search}
          onChange={setSearch}
          className={styles.search}
        />
        <Switch label={t('labels.filters.systemLabels')} isSelected={showSystem} onChange={setShowSystem} />
      </div>

      {systemLabels.length > 0 ? (
        <section aria-labelledby="labels-system">
          <h3 id="labels-system" className={shared.subheading}>
            {t('labels.sections.systemLabels')}
          </h3>
          <ul className={styles.grid}>
            {systemLabels.map((label) => (
              <LabelCard key={label.id} label={label} />
            ))}
          </ul>
        </section>
      ) : null}

      {userLabels.length > 0 ? (
        <section aria-labelledby="labels-mine">
          <h3 id="labels-mine" className={shared.subheading}>
            {t('labels.sections.myLabels')}
          </h3>
          <ul className={styles.grid}>
            {userLabels.map((label) => (
              <LabelCard key={label.id} label={label} onEdit={setEditing} onDelete={setToDelete} />
            ))}
          </ul>
        </section>
      ) : null}

      {filtered.length === 0 ? (
        <EmptyState
          headingAs="h3"
          title={t('labels.empty.title')}
          description={search ? t('labels.empty.noMatch', { query: search }) : t('labels.empty.noLabels')}
          action={
            search ? undefined : (
              <Button variant="primary" icon={<Add fontSize="inherit" />} onPress={() => setCreateOpen(true)}>
                {t('labels.empty.createFirst')}
              </Button>
            )
          }
        />
      ) : null}

      <LabelCreateDialog
        open={createOpen || editing !== null}
        onClose={() => {
          setCreateOpen(false);
          setEditing(null);
        }}
        onSubmit={async (data) => {
          if (editing) {
            await updateLabel(editing.id, data);
            setEditing(null);
          } else {
            await createLabel(data);
          }
        }}
        editingLabel={editing ?? undefined}
      />

      <Dialog
        role="alertdialog"
        size="sm"
        isOpen={toDelete !== null}
        onOpenChange={(open) => !open && !deleting && setToDelete(null)}
        title={t('labels.dialogs.delete.title')}
        actions={
          <>
            <Button variant="ghost" onPress={() => setToDelete(null)} isDisabled={deleting}>
              {t('common.actions.cancel')}
            </Button>
            <Button variant="danger" onPress={() => void confirmDelete()} isPending={deleting}>
              {t('common.actions.delete')}
            </Button>
          </>
        }
      >
        <div className={shared.stack}>
          <p>
            {t('labels.dialogs.delete.message', { name: toDelete?.name })}
            {(toDelete?.document_count || 0) > 0
              ? t('labels.dialogs.delete.inUseWarning', { count: toDelete?.document_count })
              : null}
          </p>
          {error && toDelete ? <Notice tone="danger">{error}</Notice> : null}
        </div>
      </Dialog>
    </div>
  );
}
