import { useCallback, useEffect, useState } from 'react';
import axios from 'axios';
import { useTranslation } from 'react-i18next';
import { apiKeysService, type ApiKey } from '../../../services/api';

export type ExpirationOption = '7' | '30' | '90' | '180' | '365' | 'never';

export function extractErrorMessage(err: unknown, fallback: string): string {
  if (axios.isAxiosError(err) && err.response?.data?.error) return String(err.response.data.error);
  return fallback;
}

/** API key list with create and revoke. Create resolves to the one-time plaintext. */
export function useApiKeys() {
  const { t } = useTranslation();
  const [keys, setKeys] = useState<ApiKey[]>([]);
  const [isLoading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const response = await apiKeysService.list();
      setKeys(Array.isArray(response.data) ? response.data : []);
    } catch (err) {
      setError(extractErrorMessage(err, t('settings.apiKeys.loadFailed', 'Failed to load API keys.')));
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const create = async (name: string, expiration: ExpirationOption) => {
    try {
      const response = await apiKeysService.create({
        name,
        expires_in_days: expiration === 'never' ? undefined : Number(expiration),
      });
      void refresh();
      return { name: response.data.api_key.name, plaintext: response.data.plaintext };
    } catch (err) {
      throw new Error(extractErrorMessage(err, t('settings.apiKeys.createFailed', 'Failed to create API key.')));
    }
  };

  const revoke = async (key: ApiKey) => {
    try {
      await apiKeysService.revoke(key.id);
    } catch (err) {
      throw new Error(extractErrorMessage(err, t('settings.apiKeys.revokeFailed', 'Failed to revoke API key.')));
    }
    void refresh();
  };

  return { keys, isLoading, error, setError, create, revoke };
}
