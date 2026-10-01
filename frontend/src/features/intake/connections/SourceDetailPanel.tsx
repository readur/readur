import { useId, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Button, Pass, PassCell, SlideOver, StatusMark } from '../../../ui';
import type { SourceResponse } from '../../../services/api';
import { formatBytes, formatCount, formatRelative } from '../shared/format';
import { HumanReason } from '../shared/HumanReason';
import { ConfirmDialog, Notice, sharedStyles } from '../shared/parts';
import { ConnectionDot } from './ConnectionDot';
import { humanizeAdvice } from './connectionFailure';
import { ignoredFilesHref, sourceTypeLabel } from '../shared/sourceTypes';
import { SyncProgressDisplay } from '../sync/SyncProgressDisplay';
import { RecentErrors } from './RecentErrors';
import { listOf, nextSyncAt, sourceAuth, sourceLocation, sourceState, syncIntervalMinutes, validationIssues } from './sourceModel';
import type { useSourceActions } from './useSourceActions';
import styles from './Connections.module.css';

export interface SourceDetailPanelProps {
  source: SourceResponse | null;
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
  onEdit: (source: SourceResponse) => void;
  actions: ReturnType<typeof useSourceActions>;
}

/** Everything about one connection plus its actions, in a panel beside the board. */
export function SourceDetailPanel({ source, isOpen, onOpenChange, onEdit, actions }: SourceDetailPanelProps) {
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const errorsHeadingId = useId();
  const [confirmDelete, setConfirmDelete] = useState(false);

  if (!source) return null;
  const lng = i18n.language;
  const syncing = source.status === 'syncing';
  const isWebdav = source.source_type === 'webdav';
  const interval = syncIntervalMinutes(source);
  const next = nextSyncAt(source);
  const folders = listOf(source, 'watch_folders');
  const extensions = listOf(source, 'file_extensions');
  const busy = actions.pending !== null;
  const issues = validationIssues(source);

  const footer = (
    <div className={styles.actions}>
      {syncing ? (
        <Button variant="primary" onPress={() => actions.stop(source)} isPending={actions.pending === 'stop'}>
          {t('intake.detail.stop', 'Stop sync')}
        </Button>
      ) : (
        <Button
          variant="primary"
          onPress={() => actions.sync(source)}
          isPending={actions.pending === 'sync'}
          isDisabled={!source.enabled || (busy && actions.pending !== 'sync')}
        >
          {t('intake.detail.sync', 'Sync now')}
        </Button>
      )}
      <Button
        onPress={() => actions.deepScan(source)}
        isPending={actions.pending === 'deepScan'}
        isDisabled={!isWebdav || syncing || !source.enabled}
        aria-describedby={isWebdav ? undefined : `${errorsHeadingId}-deep`}
      >
        {t('intake.detail.deepScan', 'Deep scan')}
      </Button>
      <Button onPress={() => actions.test(source)} isPending={actions.pending === 'test'}>
        {t('intake.form.test.button', 'Test connection')}
      </Button>
      <Button
        onPress={() => actions.validate(source)}
        isPending={actions.pending === 'validate'}
        isDisabled={syncing || !source.enabled}
      >
        {t('intake.detail.health', 'Check health')}
      </Button>
      <Button onPress={() => onEdit(source)}>{t('intake.detail.edit', 'Edit')}</Button>
      <Button onPress={() => actions.toggle(source)} isPending={actions.pending === 'toggle'}>
        {source.enabled ? t('intake.detail.disable', 'Disable') : t('intake.detail.enable', 'Enable')}
      </Button>
      <Button variant="ghost" onPress={() => navigate(ignoredFilesHref(source))}>
        {t('intake.detail.ignored', 'Ignored files')}
      </Button>
      <Button variant="danger" onPress={() => setConfirmDelete(true)}>
        {t('intake.detail.delete', 'Delete')}
      </Button>
    </div>
  );

  return (
    <>
      <SlideOver title={source.name} isOpen={isOpen} onOpenChange={onOpenChange} footer={footer}>
        <div className={sharedStyles.stack}>
          <div className={styles.detailStatus}>
            <StatusMark state={sourceState(source)} />
            <ConnectionDot id={source.id} type={source.source_type} />
            <span className={sharedStyles.meta}>{sourceTypeLabel(t, source.source_type)}</span>
          </div>

          <Pass aria-label={t('intake.detail.connection', 'Connection')}>
            <PassCell label={t('intake.detail.location', 'Location')} mono span={2}>
              {sourceLocation(source)}
            </PassCell>
            <PassCell label={t('intake.detail.auth', 'Sign-in')} mono>
              {sourceAuth(source, t('intake.detail.passwordSet', 'password set'))}
            </PassCell>
          </Pass>
          <Pass aria-label={t('intake.detail.schedule', 'Schedule')}>
            <PassCell label={t('intake.detail.interval', 'Interval')} mono>
              {interval ? t('intake.detail.every', 'every {{minutes}} min', { minutes: interval }) : t('intake.detail.manual', 'manual')}
            </PassCell>
            <PassCell label={t('intake.detail.lastSync', 'Last sync')} mono>
              {source.last_sync_at ? formatRelative(source.last_sync_at, lng) : t('intake.detail.never', 'never')}
            </PassCell>
            <PassCell label={t('intake.detail.next', 'Next')} mono>
              {next ? formatRelative(next, lng) : '—'}
            </PassCell>
          </Pass>
          <Pass aria-label={t('intake.detail.counts', 'Counts')}>
            <PassCell label={t('intake.detail.documents', 'Documents')} mono>
              {formatCount(source.total_documents, lng)}
            </PassCell>
            <PassCell label={t('intake.detail.ocrDone', 'OCR done')} mono>
              {formatCount(source.total_documents_ocr, lng)}
            </PassCell>
            <PassCell label={t('intake.detail.pending', 'Pending')} mono>
              {formatCount(source.total_files_pending, lng)}
            </PassCell>
            <PassCell label={t('intake.detail.size', 'Size')} mono>
              {formatBytes(source.total_size_bytes)}
            </PassCell>
          </Pass>
          <Pass aria-label={t('intake.detail.scope', 'Scope')}>
            <PassCell label={t('intake.detail.folders', 'Folders')} mono>
              {folders.length > 0 ? folders.join(', ') : '—'}
            </PassCell>
            <PassCell label={t('intake.detail.extensions', 'File types')} mono>
              {extensions.length > 0 ? extensions.join(' ') : '—'}
            </PassCell>
            <PassCell label={t('intake.detail.healthLabel', 'Health')} mono>
              {source.validation_status
                ? source.validation_score !== null && source.validation_score !== undefined
                  ? `${source.validation_status} · ${source.validation_score}`
                  : source.validation_status
                : '—'}
            </PassCell>
          </Pass>

          {!isWebdav ? (
            <p id={`${errorsHeadingId}-deep`} className={sharedStyles.meta}>
              {t('intake.detail.deepScanHint', 'Deep scan is only available for WebDAV connections; use Sync now instead.')}
            </p>
          ) : null}

          {source.last_error ? (
            <Notice tone="danger" title={t('intake.detail.lastError', 'Last sync failed')}>
              <HumanReason kind="connection" raw={source.last_error} />
              {source.last_error_at ? (
                <span className={sharedStyles.meta}> · {formatRelative(source.last_error_at, lng)}</span>
              ) : null}
            </Notice>
          ) : null}

          {issues.length > 0 ? (
            <section className={sharedStyles.stack} aria-labelledby={`${errorsHeadingId}-health`}>
              <h3 id={`${errorsHeadingId}-health`} className={sharedStyles.heading}>
                {t('intake.detail.healthFindings', 'Health check')}
              </h3>
              <ul className={styles.issueList}>
                {issues.map((issue, i) => (
                  <li key={i} className={styles.issue}>
                    <HumanReason kind="connection" raw={issue.message} />
                    {issue.recommendation ? <span className={styles.issueFix}>{humanizeAdvice(issue.recommendation)}</span> : null}
                  </li>
                ))}
              </ul>
            </section>
          ) : null}

          <SyncProgressDisplay sourceId={source.id} sourceName={source.name} isVisible={syncing} />

          <section className={sharedStyles.stack} aria-labelledby={errorsHeadingId}>
            <h3 id={errorsHeadingId} className={sharedStyles.heading}>
              {t('intake.detail.recentErrors', 'Recent errors')}
            </h3>
            <RecentErrors sourceId={source.id} headingId={errorsHeadingId} />
          </section>
        </div>
      </SlideOver>
      <ConfirmDialog
        isOpen={confirmDelete}
        onOpenChange={setConfirmDelete}
        title={t('intake.detail.deleteTitle', 'Delete “{{name}}”?', { name: source.name })}
        confirmLabel={t('intake.detail.deleteConfirm', 'Delete connection')}
        isPending={actions.pending === 'delete'}
        onConfirm={async () => {
          const gone = await actions.remove(source);
          if (gone) {
            setConfirmDelete(false);
            onOpenChange(false);
          }
        }}
      >
        <p>
          {t(
            'intake.detail.deleteBody',
            'This cannot be undone. The connection settings and all of its sync history are removed permanently.',
          )}
        </p>
      </ConfirmDialog>
    </>
  );
}
