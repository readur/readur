import { useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import type { SourceResponse } from '../../services/api';
import type { SyncProgressManager } from '../../services/syncProgress';
import { useToast } from '../../ui';
import { useSyncProgress } from '../intake/sync/useSyncProgress';

interface SyncToastsProps {
  sources: SourceResponse[] | null;
  /** Inject a progress manager (tests); each sync opens its own WebSocket otherwise. */
  manager?: SyncProgressManager;
}

/** One progress toast per syncing source, wherever the person is in the app. */
export function SyncToasts({ sources, manager }: SyncToastsProps) {
  const syncing = (sources ?? []).filter((s) => s.status === 'syncing');
  return (
    <>
      {syncing.map((s) => (
        <SyncToast key={s.id} source={s} manager={manager} />
      ))}
    </>
  );
}

function SyncToast({ source, manager }: { source: SourceResponse; manager?: SyncProgressManager }) {
  const { t, i18n } = useTranslation();
  const toast = useToast();
  const { progressInfo: info } = useSyncProgress({ sourceId: source.id, manager });
  const id = `sync-${source.id}`;

  useEffect(() => {
    if (info && !info.is_active) {
      toast.dismiss(id);
      return;
    }
    const n = (v: number) => new Intl.NumberFormat(i18n.language).format(v);
    const counted = info && info.files_found > 0;
    toast.progress(id, {
      title: t('shell.sync.title', 'Syncing {{name}}', { name: source.name }),
      description: counted
        ? t('shell.sync.files', '{{processed}} of {{total}} files', { processed: n(info.files_processed), total: n(info.files_found) })
        : info?.phase_description,
      value: counted ? info.files_progress_percent : undefined,
    });
  }, [info, id, source.name, t, i18n.language, toast]);

  // The sync ended (the source stopped reporting syncing) or the shell went away.
  useEffect(() => () => toast.dismiss(id), [id, toast]);
  return null;
}
