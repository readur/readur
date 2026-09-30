import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ErrorHelper, userWatchService, type UserWatchDirectoryResponse } from '../../../services/api';
import { useToast } from '../../../ui';
import type { UserRow } from './useUsers';

const statusOf = (e: unknown) => (e as { response?: { status?: number } })?.response?.status;

/** Per-user watch directory status and create/view/remove, with the previous page's messages. */
export function useWatchDirectories(users: UserRow[], enabled: boolean) {
  const { t } = useTranslation();
  const toast = useToast();
  const [dirs, setDirs] = useState<Map<string, UserWatchDirectoryResponse>>(new Map());
  const [busy, setBusy] = useState<Set<string>>(new Set());

  const setOne = (userId: string, info: UserWatchDirectoryResponse) =>
    setDirs((prev) => new Map(prev).set(userId, info));
  const markGone = (userId: string) =>
    setDirs((prev) => {
      const current = prev.get(userId);
      return current ? new Map(prev).set(userId, { ...current, exists: false, enabled: false }) : prev;
    });
  const setBusyFor = (userId: string, on: boolean) =>
    setBusy((prev) => {
      const next = new Set(prev);
      if (on) next.add(userId);
      else next.delete(userId);
      return next;
    });

  const load = useCallback(async () => {
    const map = new Map<string, UserWatchDirectoryResponse>();
    await Promise.all(
      users.map(async (user) => {
        try {
          const response = await userWatchService.getUserWatchDirectory(user.id);
          map.set(user.id, response.data);
        } catch (error) {
          if (statusOf(error) === 404) {
            map.set(user.id, {
              user_id: user.id,
              username: user.username,
              watch_directory_path: `./user_watch/${user.username}`,
              exists: false,
              enabled: false,
            });
          }
        }
      }),
    );
    setDirs(map);
  }, [users]);

  useEffect(() => {
    if (enabled && users.length > 0) void load();
  }, [enabled, users, load]);

  const create = async (userId: string) => {
    setBusyFor(userId, true);
    try {
      const response = await userWatchService.createUserWatchDirectory(userId);
      if (response.data.success) {
        toast.show({ title: t('settings.messages.watchDirectoryCreated'), tone: 'success' });
        try {
          setOne(userId, (await userWatchService.getUserWatchDirectory(userId)).data);
        } catch {
          /* keep the previous status; the next load refreshes it */
        }
      } else {
        toast.show({ title: response.data.message || t('settings.messages.watchDirectoryCreatedFailed'), tone: 'danger' });
      }
    } catch (error) {
      const status = statusOf(error);
      if (status === 403) toast.show({ title: t('settings.messages.permissionDenied'), tone: 'danger' });
      else if (status === 409) toast.show({ title: t('settings.messages.watchDirectoryAlreadyExists'), tone: 'info' });
      else
        toast.show({
          title: ErrorHelper.formatErrorForDisplay(error, true).message || t('settings.messages.watchDirectoryCreatedFailed'),
          tone: 'danger',
        });
    } finally {
      setBusyFor(userId, false);
    }
  };

  const view = (path: string) => toast.show({ title: t('settings.messages.watchDirectoryPath', { path }), tone: 'info' });

  const remove = async (userId: string) => {
    setBusyFor(userId, true);
    try {
      const response = await userWatchService.deleteUserWatchDirectory(userId);
      if (response.data.success) {
        toast.show({ title: t('settings.messages.watchDirectoryRemoved'), tone: 'success' });
        markGone(userId);
      } else {
        toast.show({ title: response.data.message || t('settings.messages.watchDirectoryRemoveFailed'), tone: 'danger' });
      }
    } catch (error) {
      const status = statusOf(error);
      if (status === 403) toast.show({ title: t('settings.messages.permissionDenied'), tone: 'danger' });
      else if (status === 404) {
        toast.show({ title: t('settings.messages.watchDirectoryNotFound'), tone: 'info' });
        markGone(userId);
      } else
        toast.show({
          title: ErrorHelper.formatErrorForDisplay(error, true).message || t('settings.messages.watchDirectoryRemoveFailed'),
          tone: 'danger',
        });
    } finally {
      setBusyFor(userId, false);
    }
  };

  return { dirs, busy, create, view, remove };
}
