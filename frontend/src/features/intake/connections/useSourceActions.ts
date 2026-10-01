import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useToast } from '../../../ui';
import { sourcesService, type SourceResponse } from '../../../services/api';
import { buildTestConnectionRequest } from '../../../services/sourceConnectionTest';
import type { TestConnectionRequest } from '../../../types/generated';
import { ErrorCodes, hasCode, pickMessage, serverMessage, statusOf } from '../shared/errors';

export type SourceAction = 'sync' | 'deepScan' | 'stop' | 'test' | 'validate' | 'toggle' | 'delete';

/**
 * Every action a connection offers, with its pending state and the same error wording the
 * old page used. `onChanged` runs after anything that changes the connection.
 */
export function useSourceActions(onChanged: () => void, onDeleted?: (id: string) => void) {
  const { t } = useTranslation();
  const toast = useToast();
  const [pending, setPending] = useState<SourceAction | null>(null);

  const run = async (action: SourceAction, work: () => Promise<void>) => {
    setPending(action);
    try {
      await work();
    } finally {
      setPending(null);
    }
  };

  const refreshSoon = () => window.setTimeout(onChanged, 1000);

  const sync = (source: SourceResponse) =>
    run('sync', async () => {
      try {
        await sourcesService.triggerSync(source.id);
        toast.show({ title: t('intake.detail.syncStarted', 'Sync started'), description: source.name, tone: 'success' });
        refreshSoon();
      } catch (error) {
        if (hasNotFound(error)) onChanged();
        toast.show({
          title: t('intake.detail.syncFailed', 'Could not start the sync'),
          description: pickMessage(
            error,
            [
              [ErrorCodes.SOURCE_SYNC_IN_PROGRESS, t('intake.detail.alreadySyncing', 'This connection is already syncing.')],
              [ErrorCodes.SOURCE_CONNECTION_FAILED, t('intake.detail.cannotConnect', 'Could not connect to the source. Check its settings.')],
              [ErrorCodes.SOURCE_AUTH_FAILED, t('intake.detail.authFailed', 'The source rejected the credentials.')],
              [ErrorCodes.SOURCE_NOT_FOUND, t('intake.detail.deleted', 'This connection no longer exists.')],
            ],
            source.name,
          ),
          tone: 'danger',
        });
      }
    });

  const deepScan = (source: SourceResponse) =>
    run('deepScan', async () => {
      try {
        await sourcesService.triggerDeepScan(source.id);
        toast.show({ title: t('intake.detail.deepScanStarted', 'Deep scan started'), description: source.name, tone: 'success' });
        refreshSoon();
      } catch (error) {
        const status = statusOf(error);
        const message = serverMessage(error) ?? '';
        toast.show({
          title:
            status === 409
              ? t('intake.detail.alreadySyncing', 'This connection is already syncing.')
              : status === 400 && message.includes('only supported for WebDAV')
                ? t('intake.detail.deepScanWebdavOnly', 'Deep scan is only available for WebDAV connections.')
                : t('intake.detail.deepScanFailed', 'Could not start the deep scan'),
          tone: status === 409 ? 'info' : 'danger',
        });
      }
    });

  const stop = (source: SourceResponse) =>
    run('stop', async () => {
      try {
        await sourcesService.stopSync(source.id);
        toast.show({ title: t('intake.detail.syncStopped', 'Sync stopped'), description: source.name, tone: 'success' });
        refreshSoon();
      } catch (error) {
        toast.show({
          title:
            statusOf(error) === 409
              ? t('intake.detail.notSyncing', 'This connection is not syncing.')
              : t('intake.detail.stopFailed', 'Could not stop the sync'),
          tone: statusOf(error) === 409 ? 'info' : 'danger',
        });
      }
    });

  /** Tests the saved configuration. */
  const test = (source: SourceResponse) =>
    run('test', async () => {
      try {
        // The saved config carries no secrets; the id lets the server test with the stored ones.
        const config = (source.config && typeof source.config === 'object' ? source.config : {}) as Record<string, unknown>;
        const res = await sourcesService.testConnection(
          buildTestConnectionRequest(source.source_type, config, source) as TestConnectionRequest,
        );
        const ok = Boolean(res.data?.success);
        toast.show({
          title: ok ? t('intake.form.test.ok', 'Connection successful') : t('intake.form.test.failed', 'Connection failed'),
          description: res.data?.message,
          tone: ok ? 'success' : 'danger',
        });
      } catch (error) {
        toast.show({
          title: t('intake.form.test.failed', 'Connection failed'),
          description: pickMessage(
            error,
            [
              [ErrorCodes.SOURCE_CONNECTION_FAILED, t('intake.form.test.unreachable', 'Could not reach the server. Check the URL and your network.')],
              [ErrorCodes.SOURCE_AUTH_FAILED, t('intake.form.test.auth', 'Sign-in failed. Check the username and password.')],
              [ErrorCodes.SOURCE_INVALID_PATH, t('intake.form.test.path', 'A folder path is invalid or not accessible.')],
              [ErrorCodes.SOURCE_CONFIG_INVALID, t('intake.form.test.config', 'Some settings are invalid. Check the values and try again.')],
              [ErrorCodes.SOURCE_NETWORK_TIMEOUT, t('intake.form.test.timeout', 'The connection timed out. The server may be slow or unreachable.')],
            ],
            t('intake.form.test.error', 'Could not test the connection'),
          ),
          tone: 'danger',
        });
      }
    });

  const validate = (source: SourceResponse) =>
    run('validate', async () => {
      try {
        const res = await sourcesService.validate(source.id);
        const ok = Boolean(res.data?.success);
        toast.show({
          title: ok ? t('intake.detail.healthStarted', 'Health check started') : t('intake.detail.healthFailed', 'Could not start the health check'),
          description: res.data?.message,
          tone: ok ? 'success' : 'danger',
        });
        if (ok) window.setTimeout(onChanged, 2000);
      } catch (error) {
        toast.show({
          title: t('intake.detail.healthFailed', 'Could not start the health check'),
          description: serverMessage(error),
          tone: 'danger',
        });
      }
    });

  const toggle = (source: SourceResponse) =>
    run('toggle', async () => {
      try {
        await sourcesService.update(source.id, { enabled: !source.enabled });
        toast.show({
          title: source.enabled ? t('intake.detail.disabled', 'Connection turned off') : t('intake.detail.enabled', 'Connection turned on'),
          description: source.name,
          tone: 'success',
        });
        onChanged();
      } catch (error) {
        toast.show({ title: t('intake.detail.toggleFailed', 'Could not change the connection'), description: serverMessage(error), tone: 'danger' });
      }
    });

  /** Resolves true when the connection is gone (deleted now or already). */
  const remove = async (source: SourceResponse): Promise<boolean> => {
    setPending('delete');
    try {
      await sourcesService.remove(source.id);
      toast.show({ title: t('intake.detail.deletedToast', 'Connection deleted'), description: source.name, tone: 'success' });
      onDeleted?.(source.id);
      onChanged();
      return true;
    } catch (error) {
      if (hasNotFound(error)) {
        toast.show({ title: t('intake.detail.deleted', 'This connection no longer exists.'), tone: 'info' });
        onDeleted?.(source.id);
        onChanged();
        return true;
      }
      toast.show({
        title: t('intake.detail.deleteFailed', 'Could not delete the connection'),
        description: pickMessage(
          error,
          [[ErrorCodes.SOURCE_SYNC_IN_PROGRESS, t('intake.detail.deleteWhileSyncing', 'Stop the sync before deleting this connection.')]],
          source.name,
        ),
        tone: 'danger',
      });
      return false;
    } finally {
      setPending(null);
    }
  };

  return { pending, sync, deepScan, stop, test, validate, toggle, remove };
}

const hasNotFound = (error: unknown) => hasCode(error, ErrorCodes.SOURCE_NOT_FOUND);
