import { useContext, useId, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { BoardTable, Button, Pass, PassCell, StatusMark, useToast, type BoardColumn, type StatusState } from '../../../ui';
import { Refresh } from '../../../ui/icons';
import { queueService, userWatchService } from '../../../services/api';
import { useAuth } from '../../../contexts/AuthContext';
import { isAdmin } from '../../../auth/roles';
import { FeatureFlagsContext } from '../../../contexts/FeatureFlagsContext';
import { formatCount, formatMinutes } from '../shared/format';
import { ConfirmDialog, Notice, sharedStyles } from '../shared/parts';
import { useInterval } from '../shared/useInterval';
import { useLoader } from '../shared/useLoader';
import { SYSTEM_WATCH } from './watchConfig';
import styles from './Watch.module.css';

export const QUEUE_POLL_MS = 30_000;

interface WatchedLocation {
  id: string;
  scope: string;
  path: string;
  state: StatusState;
  detail: string;
}

/** The server's watch folder, the user's own watch folder (when enabled) and the OCR queue. */
export function WatchSection() {
  const { t, i18n } = useTranslation();
  const toast = useToast();
  const { user } = useAuth();
  const flags = useContext(FeatureFlagsContext)?.flags;
  const perUser = Boolean(user && flags?.enablePerUserWatch);
  const queueId = useId();
  const locationsId = useId();
  const [confirmRequeue, setConfirmRequeue] = useState(false);
  const [requeuing, setRequeuing] = useState(false);
  const [creating, setCreating] = useState(false);

  const queue = useLoader(async () => (await queueService.getStats()).data, []);
  const personal = useLoader(async () => (await userWatchService.getUserWatchDirectory(user!.id)).data, [user?.id], perUser);

  // Keep the queue figures fresh while the section is open.
  useInterval(() => void queue.reload(), QUEUE_POLL_MS);

  const createDirectory = async () => {
    if (!user) return;
    setCreating(true);
    try {
      const res = await userWatchService.createUserWatchDirectory(user.id);
      toast.show({
        title: res.data?.success ? t('intake.watch.created', 'Watch folder created') : t('intake.watch.createFailed', 'Could not create the watch folder'),
        description: res.data?.message,
        tone: res.data?.success ? 'success' : 'danger',
      });
      await personal.reload();
    } catch {
      toast.show({ title: t('intake.watch.createFailed', 'Could not create the watch folder'), tone: 'danger' });
    } finally {
      setCreating(false);
    }
  };

  const requeue = async () => {
    setRequeuing(true);
    try {
      const res = await queueService.requeueFailed();
      const count = (res.data as { requeued_count?: number } | undefined)?.requeued_count ?? 0;
      toast.show({
        title: count > 0 ? t('intake.watch.requeued', '{{count}} jobs queued again', { count }) : t('intake.watch.nothingToRequeue', 'No failed jobs to retry'),
        tone: count > 0 ? 'success' : 'info',
      });
      setConfirmRequeue(false);
      await queue.reload();
    } catch {
      toast.show({ title: t('intake.watch.requeueFailed', 'Could not retry the failed jobs'), tone: 'danger' });
    } finally {
      setRequeuing(false);
    }
  };

  const locations: WatchedLocation[] = [
    {
      id: 'system',
      scope: t('intake.watch.system', 'Server'),
      path: SYSTEM_WATCH.folder,
      state: 'healthy',
      detail: t('intake.watch.systemDetail', 'every {{seconds}} s · files up to {{hours}} h old', {
        seconds: SYSTEM_WATCH.intervalSeconds,
        hours: SYSTEM_WATCH.maxFileAgeHours,
      }),
    },
  ];
  if (perUser && personal.data) {
    locations.push({
      id: 'personal',
      scope: t('intake.watch.personal', 'Yours'),
      path: personal.data.watch_directory_path,
      state: !personal.data.exists ? 'error' : personal.data.enabled ? 'healthy' : 'disabled',
      detail: personal.data.exists
        ? t('intake.watch.exists', 'Folder exists')
        : t('intake.watch.missing', 'Folder does not exist yet'),
    });
  }

  const columns: BoardColumn<WatchedLocation>[] = [
    { id: 'path', label: t('intake.watch.col.path', 'Folder'), mono: true, render: (l) => l.path },
    { id: 'scope', label: t('intake.watch.col.scope', 'Scope'), width: 120, render: (l) => l.scope },
    { id: 'status', label: t('intake.watch.col.status', 'Status'), width: 120, render: (l) => <StatusMark state={l.state} size="sm" /> },
    { id: 'detail', label: t('intake.watch.col.detail', 'Details'), render: (l) => l.detail },
  ];

  const stats = queue.data;
  const lng = i18n.language;

  return (
    <div className={sharedStyles.section}>
      <div className={sharedStyles.toolbar}>
        <Button
          icon={<Refresh fontSize="inherit" />}
          onPress={() => {
            void queue.reload();
            if (perUser) void personal.reload();
          }}
          isPending={queue.isLoading && Boolean(stats)}
        >
          {t('intake.actions.refresh', 'Refresh')}
        </Button>
        {stats && stats.failed_count > 0 ? (
          <Button onPress={() => setConfirmRequeue(true)}>
            {t('intake.watch.requeue', 'Retry {{count}} failed jobs', { count: stats.failed_count })}
          </Button>
        ) : null}
      </div>

      <section className={sharedStyles.stack} aria-labelledby={queueId}>
        <h2 id={queueId} className={sharedStyles.heading}>{t('intake.watch.queue', 'Processing queue')}</h2>
        {queue.error && !stats ? (
          <Notice tone="danger" live="alert" title={t('intake.watch.queueFailed', 'Could not load the processing queue.')} />
        ) : stats ? (
          <Pass aria-label={t('intake.watch.queue', 'Processing queue')}>
            <PassCell label={t('intake.watch.pending', 'Waiting')} mono>{formatCount(stats.pending_count, lng)}</PassCell>
            <PassCell label={t('intake.watch.processing', 'Processing')} mono>{formatCount(stats.processing_count, lng)}</PassCell>
            <PassCell label={t('intake.watch.failed', 'Failed')} mono>{formatCount(stats.failed_count, lng)}</PassCell>
            <PassCell label={t('intake.watch.today', 'Done today')} mono>{formatCount(stats.completed_today, lng)}</PassCell>
            <PassCell label={t('intake.watch.avgWait', 'Average wait')} mono>{formatMinutes(stats.avg_wait_time_minutes)}</PassCell>
            <PassCell label={t('intake.watch.oldest', 'Oldest waiting')} mono>{formatMinutes(stats.oldest_pending_minutes)}</PassCell>
          </Pass>
        ) : (
          <p className={sharedStyles.meta}>{t('intake.watch.loading', 'Loading…')}</p>
        )}
      </section>

      <section className={sharedStyles.stack} aria-labelledby={locationsId}>
        <h2 id={locationsId} className={sharedStyles.heading}>{t('intake.watch.locations', 'Watched folders')}</h2>
        {!isAdmin(user) ? (
          <p className={sharedStyles.lead}>
            {t('intake.watch.systemInfo', 'The server folder is set by your administrator and applies to everyone.')}
          </p>
        ) : null}
        <BoardTable
          aria-labelledby={locationsId}
          columns={columns}
          rows={locations}
          getRowId={(l) => l.id}
          density="compact"
        />
        {perUser && personal.error ? (
          <Notice tone="danger" title={t('intake.watch.personalFailed', 'Could not load your watch folder.')} />
        ) : null}
        {perUser && personal.data && !personal.data.exists ? (
          <div className={styles.createRow}>
            <p className={sharedStyles.lead}>
              {t('intake.watch.createHint', 'Your personal watch folder does not exist yet. Create it to drop files there.')}
            </p>
            <Button variant="primary" onPress={createDirectory} isPending={creating}>
              {t('intake.watch.create', 'Create my watch folder')}
            </Button>
          </div>
        ) : null}
        <div className={styles.types}>
          <span className={sharedStyles.heading}>{t('intake.watch.types', 'File types')}</span>
          <span className={sharedStyles.mono}>{SYSTEM_WATCH.allowedTypes.map((type) => `.${type}`).join(' ')}</span>
        </div>
      </section>

      <section className={sharedStyles.stack} aria-label={t('intake.watch.howTitle', 'How the watch folder works')}>
        <h2 className={sharedStyles.heading}>{t('intake.watch.howTitle', 'How the watch folder works')}</h2>
        <ol className={styles.steps}>
          <li>{t('intake.watch.step.detect', 'New files are detected in the folder.')}</li>
          <li>{t('intake.watch.step.validate', 'Type and size are checked.')}</li>
          <li>{t('intake.watch.step.dedupe', 'Files already in the library are skipped.')}</li>
          <li>{t('intake.watch.step.store', 'The file is stored.')}</li>
          <li>{t('intake.watch.step.queue', 'It joins the OCR queue.')}</li>
        </ol>
        <p className={sharedStyles.meta}>
          {t('intake.watch.strategy', 'Strategy: {{strategy}} (file events plus periodic scans)', { strategy: SYSTEM_WATCH.strategy })}
        </p>
      </section>

      <ConfirmDialog
        isOpen={confirmRequeue}
        onOpenChange={setConfirmRequeue}
        tone="primary"
        title={t('intake.watch.requeueTitle', 'Retry failed jobs?')}
        confirmLabel={t('intake.watch.requeueConfirm', 'Retry jobs')}
        isPending={requeuing}
        onConfirm={requeue}
      >
        <p>{t('intake.watch.requeueBody', 'Every failed OCR job goes back into the queue.')}</p>
      </ConfirmDialog>
    </div>
  );
}
