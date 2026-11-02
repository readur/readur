import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { TFunction } from 'i18next';
import { api, ErrorCodes, ErrorHelper, type ErrorCode } from '../../../services/api';
import { useToast } from '../../../ui';

export interface UserRow {
  id: string;
  username: string;
  email: string;
  role?: string;
  /** False while an account is disabled or awaiting approval. */
  is_active?: boolean;
  created_at?: string;
}

export interface UserForm {
  username: string;
  email: string;
  password: string;
}

/** Maps a failed create/update/delete to the previous page's messages. */
export function userErrorMessage(error: unknown, t: TFunction): { message: string; tone: 'danger' | 'info' } {
  const info = ErrorHelper.formatErrorForDisplay(error, true);
  const is = (code: ErrorCode) => ErrorHelper.isErrorCode(error, code);
  if (is(ErrorCodes.USER_DUPLICATE_USERNAME)) return { message: t('settings.messages.duplicateUsername'), tone: 'danger' };
  if (is(ErrorCodes.USER_DUPLICATE_EMAIL)) return { message: t('settings.messages.duplicateEmail'), tone: 'danger' };
  if (is(ErrorCodes.USER_INVALID_PASSWORD)) return { message: t('settings.messages.invalidPassword'), tone: 'danger' };
  if (is(ErrorCodes.USER_INVALID_EMAIL)) return { message: t('settings.messages.invalidEmail'), tone: 'danger' };
  if (is(ErrorCodes.USER_INVALID_USERNAME)) return { message: t('settings.messages.invalidUsername'), tone: 'danger' };
  if (is(ErrorCodes.USER_PERMISSION_DENIED)) return { message: t('settings.messages.permissionDenied'), tone: 'danger' };
  if (is(ErrorCodes.USER_DELETE_RESTRICTED)) return { message: t('settings.messages.cannotDeleteUser'), tone: 'danger' };
  if (is(ErrorCodes.USER_NOT_FOUND)) return { message: t('settings.messages.userNotFound'), tone: 'info' };
  return { message: info.message || t('settings.messages.settingsUpdateFailed'), tone: 'danger' };
}

/** `/users` list plus create, update and delete, each toasting like the previous page. */
export function useUsers() {
  const { t } = useTranslation();
  const toast = useToast();
  const [users, setUsers] = useState<UserRow[]>([]);
  const [isLoading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const response = await api.get<UserRow[]>('/users');
      setUsers(Array.isArray(response.data) ? response.data : []);
      setLoadError(null);
    } catch (error) {
      const status = (error as { response?: { status?: number } })?.response?.status;
      if (status !== 404) setLoadError(t('settings.users.loadFailed', 'Failed to load users'));
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  /** Resolves on success; rejects with the display message so the dialog can show it inline. */
  const saveUser = async (form: UserForm, existing: UserRow | null) => {
    try {
      if (existing) {
        const { password, ...rest } = form;
        await api.put(`/users/${existing.id}`, password ? { ...rest, password } : rest);
        toast.show({ title: t('settings.messages.userUpdated'), tone: 'success' });
      } else {
        await api.post('/users', form);
        toast.show({ title: t('settings.messages.userCreated'), tone: 'success' });
      }
    } catch (error) {
      const { message } = userErrorMessage(error, t);
      toast.show({ title: message, tone: 'danger' });
      throw new Error(message);
    }
    void refresh();
  };

  const deleteUser = async (user: UserRow) => {
    try {
      await api.delete(`/users/${user.id}`);
      toast.show({ title: t('settings.messages.userDeleted'), tone: 'success' });
      void refresh();
    } catch (error) {
      const { message, tone } = userErrorMessage(error, t);
      toast.show({ title: message, tone });
      if (ErrorHelper.isErrorCode(error, ErrorCodes.USER_NOT_FOUND)) void refresh();
    }
  };

  /** Enables or disables an account; disabling also signs that user out everywhere. */
  const setUserActive = async (user: UserRow, active: boolean) => {
    try {
      await api.put(`/users/${user.id}`, { is_active: active });
      setUsers((prev) => prev.map((u) => (u.id === user.id ? { ...u, is_active: active } : u)));
      toast.show({
        title: active
          ? t('settings.messages.userEnabled', { username: user.username, defaultValue: '{{username}} can now sign in' })
          : t('settings.messages.userDisabled', {
              username: user.username,
              defaultValue: '{{username}} has been disabled and signed out',
            }),
        tone: 'success',
      });
    } catch (error) {
      toast.show({ title: userErrorMessage(error, t).message, tone: 'danger' });
    }
  };

  return { users, isLoading, loadError, refresh, saveUser, deleteUser, setUserActive };
}
