import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { TFunction } from 'i18next';
import { api, ErrorCodes, ErrorHelper } from '../../../services/api';
import { useToast } from '../../../ui';
import { DEFAULT_SETTINGS, normalizeSettings, type SettingsPatch, type SettingsValues } from './settingsModel';

/** Turns a failed `PUT /settings` into the message the previous settings page showed. */
export function settingsErrorMessage(error: unknown, t: TFunction): string {
  const info = ErrorHelper.formatErrorForDisplay(error, true);
  if (ErrorHelper.isErrorCode(error, ErrorCodes.SETTINGS_INVALID_LANGUAGE)) {
    return t('settings.messages.invalidLanguage');
  }
  if (ErrorHelper.isErrorCode(error, ErrorCodes.SETTINGS_VALUE_OUT_OF_RANGE)) {
    return t('settings.messages.valueOutOfRange', {
      message: info.message,
      suggestedAction: info.suggestedAction || '',
    });
  }
  if (ErrorHelper.isErrorCode(error, ErrorCodes.SETTINGS_CONFLICTING_SETTINGS)) {
    return t('settings.messages.conflictingSettings');
  }
  return info.message || t('settings.messages.settingsUpdateFailed');
}

export interface SettingsData {
  values: SettingsValues;
  isLoading: boolean;
  loadError: string | null;
  /** Sends only `patch` to `PUT /settings`, merges it on success and toasts. Rejects with a message. */
  save: (patch: SettingsPatch) => Promise<void>;
  reload: () => Promise<void>;
}

/** Loads `GET /settings` once and exposes a patch-saving function. */
export function useSettingsData(): SettingsData {
  const { t } = useTranslation();
  const toast = useToast();
  const [values, setValues] = useState<SettingsValues>(DEFAULT_SETTINGS);
  const [isLoading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const alive = useRef(true);

  const reload = useCallback(async () => {
    setLoading(true);
    try {
      const response = await api.get('/settings');
      if (!alive.current) return;
      setValues(normalizeSettings(response?.data));
      setLoadError(null);
    } catch (error: unknown) {
      if (!alive.current) return;
      const status = (error as { response?: { status?: number } })?.response?.status;
      // A missing settings row (404) is not an error: the defaults apply.
      if (status !== 404) setLoadError(t('settings.messages.settingsLoadFailed', 'Failed to load settings'));
    } finally {
      if (alive.current) setLoading(false);
    }
  }, [t]);

  useEffect(() => {
    alive.current = true;
    void reload();
    return () => {
      alive.current = false;
    };
  }, [reload]);

  const save = useCallback(
    async (patch: SettingsPatch) => {
      try {
        await api.put('/settings', patch);
      } catch (error) {
        const message = settingsErrorMessage(error, t);
        const warning = ErrorHelper.isErrorCode(error, ErrorCodes.SETTINGS_CONFLICTING_SETTINGS);
        toast.show({ title: message, tone: warning ? 'info' : 'danger' });
        throw new Error(message);
      }
      if (alive.current) setValues((prev) => ({ ...prev, ...patch }));
      toast.show({ title: t('settings.messages.settingsUpdated'), tone: 'success' });
    },
    [t, toast],
  );

  return { values, isLoading, loadError, save, reload };
}
