import { useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { BulkRetryModal } from '../../components/BulkRetryModal';
import { documentService } from '../../services/api';
import { labelService } from '../../services/api/labels';
import { BulkActionBar, Button, Dialog, useToast } from '../../ui';
import { Delete, Download, Label as LabelIcon, Refresh } from '../../ui/icons';
import { LabelSelector, toLabelData, type LabelData, type LabelDraft } from '../labels';
import { displayName, type LibraryRow } from './data';
import styles from './Library.module.css';

interface BulkActionsProps {
  selected: LibraryRow[];
  onClear: () => void;
  availableLabels: LabelData[];
  onLabelCreated: (label: LabelData) => void;
  /** Refetch the page after a change that affects many rows. */
  onChanged: () => void;
  /** Called with the ids that were deleted. */
  onDeleted?: (ids: string[]) => void;
}

type Open = 'label' | 'retry' | 'delete' | null;

/** The floating bar for the rows the user ticked, and the dialogs behind its actions. */
export function BulkActions({ selected, onClear, availableLabels, onLabelCreated, onChanged, onDeleted }: BulkActionsProps) {
  const { t } = useTranslation();
  const toast = useToast();
  const [open, setOpen] = useState<Open>(null);
  const [pickedLabels, setPickedLabels] = useState<LabelData[]>([]);
  const [busy, setBusy] = useState(false);
  const [downloading, setDownloading] = useState(false);
  const downloadingRef = useRef(false);
  const ids = useMemo(() => selected.map((r) => r.id), [selected]);
  const count = selected.length;

  const close = () => {
    if (busy) return;
    setOpen(null);
    setPickedLabels([]);
  };

  const addLabels = async () => {
    setBusy(true);
    try {
      await labelService.bulkAssign(ids, pickedLabels.map((l) => l.id), 'add');
      toast.show({ title: t('library.bulk.labelled', { count, defaultValue: 'Labels added to {{count}} documents', defaultValue_one: 'Labels added to 1 document' }), tone: 'success' });
      setOpen(null);
      setPickedLabels([]);
      onChanged();
    } catch {
      toast.show({ title: t('library.bulk.labelFailed', 'Could not add the labels'), tone: 'danger' });
    } finally {
      setBusy(false);
    }
  };

  const remove = async () => {
    setBusy(true);
    try {
      await documentService.bulkDelete(ids);
      toast.show({ title: t('library.bulk.deleted', { count, defaultValue: '{{count}} documents deleted', defaultValue_one: '1 document deleted' }), tone: 'success' });
      setOpen(null);
      onDeleted?.(ids);
      onClear();
      onChanged();
    } catch {
      toast.show({ title: t('library.bulk.deleteFailed', 'Could not delete the documents'), tone: 'danger' });
    } finally {
      setBusy(false);
    }
  };

  // The API has no archive download, so files download one after another. A second press while
  // a batch is running is ignored.
  const download = async () => {
    if (downloadingRef.current) return;
    downloadingRef.current = true;
    setDownloading(true);
    const rows = [...selected];
    let failed = 0;
    for (const row of rows) {
      try {
        await documentService.downloadFile(row.id, displayName(row));
      } catch {
        failed += 1;
      }
    }
    downloadingRef.current = false;
    setDownloading(false);
    const done = rows.length - failed;
    if (done > 0) {
      toast.show({
        title: t('library.bulk.downloaded', { count: done, defaultValue: '{{count}} documents downloaded', defaultValue_one: '1 document downloaded' }),
        tone: 'success',
      });
    }
    if (failed > 0) {
      toast.show({ title: t('library.bulk.downloadFailed', { count: failed, defaultValue: '{{count}} downloads failed', defaultValue_one: '1 download failed' }), tone: 'danger' });
    }
  };

  const createLabel = async (draft: LabelDraft) => {
    const res = await labelService.create(draft);
    const created = toLabelData(res.data);
    onLabelCreated(created);
    return created;
  };

  return (
    <>
      <BulkActionBar
        count={count}
        onClear={onClear}
        actions={[
          { id: 'label', label: t('library.bulk.addLabel', 'Add label'), icon: <LabelIcon fontSize="small" />, onPress: () => setOpen('label') },
          { id: 'retry', label: t('library.bulk.retry', 'Retry OCR'), icon: <Refresh fontSize="small" />, onPress: () => setOpen('retry') },
          {
            id: 'download',
            label: downloading ? t('library.bulk.downloading', 'Downloading…') : t('library.bulk.download', 'Download'),
            icon: <Download fontSize="small" />,
            isDisabled: downloading,
            onPress: () => void download(),
          },
          { id: 'delete', label: t('library.bulk.delete', 'Delete'), icon: <Delete fontSize="small" />, tone: 'danger', onPress: () => setOpen('delete') },
        ]}
      />

      <Dialog
        isOpen={open === 'label'}
        onOpenChange={(next) => (next ? undefined : close())}
        title={t('library.bulk.addLabelTitle', { count, defaultValue: 'Add labels to {{count}} documents', defaultValue_one: 'Add labels to 1 document' })}
        actions={
          <>
            <Button variant="ghost" onPress={close} isDisabled={busy}>
              {t('library.cancel', 'Cancel')}
            </Button>
            <Button variant="primary" isPending={busy} isDisabled={pickedLabels.length === 0} onPress={() => void addLabels()}>
              {t('library.bulk.apply', 'Add labels')}
            </Button>
          </>
        }
      >
        <div className={styles.dialogBody}>
          <LabelSelector
            label={t('library.columns.labels', 'Labels')}
            selectedLabels={pickedLabels}
            availableLabels={availableLabels}
            onLabelsChange={setPickedLabels}
            onCreateLabel={createLabel}
          />
        </div>
      </Dialog>

      <Dialog
        role="alertdialog"
        size="sm"
        isOpen={open === 'delete'}
        onOpenChange={(next) => (next ? undefined : close())}
        title={t('library.bulk.deleteTitle', { count, defaultValue: 'Delete {{count}} documents?', defaultValue_one: 'Delete 1 document?' })}
        actions={
          <>
            <Button variant="ghost" onPress={close} isDisabled={busy}>
              {t('library.cancel', 'Cancel')}
            </Button>
            <Button variant="danger" isPending={busy} onPress={() => void remove()}>
              {t('library.bulk.delete', 'Delete')}
            </Button>
          </>
        }
      >
        {t('library.bulk.deleteBody', 'The documents and their text will be removed. This cannot be undone.')}
      </Dialog>

      {open === 'retry' ? (
        <BulkRetryModal
          open
          onClose={() => setOpen(null)}
          selectedDocumentIds={ids}
          onSuccess={() => {
            setOpen(null);
            onChanged();
          }}
        />
      ) : null}
    </>
  );
}
