import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { api, ErrorCodes, ErrorHelper, type ErrorCode } from '../../../services/api';
import type { LabelData, LabelDraft } from '../../labels';

export type LabelInput = LabelDraft;

/** Labels with usage counts, plus create/update (which throw display messages) and delete. */
export function useLabels() {
  const { t } = useTranslation();
  const [labels, setLabels] = useState<LabelData[]>([]);
  const [isLoading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchLabels = useCallback(async () => {
    try {
      setLoading(true);
      const response = await api.get('/labels?include_counts=true');
      if (response.status === 200 && Array.isArray(response.data)) {
        setLabels(response.data);
        setError(null);
      } else {
        setError(
          Array.isArray(response.data)
            ? t('settings.labels.unexpectedStatus', 'Server returned unexpected response ({{status}})', {
                status: response.status,
              })
            : t('settings.labels.invalidData', 'Received invalid data format from server'),
        );
        setLabels([]);
      }
    } catch (err) {
      const info = ErrorHelper.formatErrorForDisplay(err, true);
      const is = (code: ErrorCode) => ErrorHelper.isErrorCode(err, code);
      if (is(ErrorCodes.USER_SESSION_EXPIRED) || is(ErrorCodes.USER_TOKEN_EXPIRED)) setError(t('labels.errors.sessionExpired'));
      else if (is(ErrorCodes.USER_PERMISSION_DENIED)) setError(t('labels.errors.permissionDenied'));
      else if (info.category === 'server') setError(t('labels.errors.serverError'));
      else if (info.category === 'network') setError(t('labels.errors.networkError'));
      else setError(info.message || t('labels.errors.loadFailed'));
      setLabels([]);
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => {
    void fetchLabels();
  }, [fetchLabels]);

  const createLabel = async (data: LabelInput) => {
    try {
      await api.post('/labels', data);
    } catch (err) {
      const info = ErrorHelper.formatErrorForDisplay(err, true);
      const is = (code: ErrorCode) => ErrorHelper.isErrorCode(err, code);
      if (is(ErrorCodes.LABEL_DUPLICATE_NAME)) throw new Error(t('labels.errors.duplicateName'));
      if (is(ErrorCodes.LABEL_INVALID_NAME)) throw new Error(t('labels.errors.invalidName'));
      if (is(ErrorCodes.LABEL_INVALID_COLOR)) throw new Error(t('labels.errors.invalidColor'));
      if (is(ErrorCodes.LABEL_MAX_LABELS_REACHED)) throw new Error(t('labels.errors.maxLabelsReached'));
      throw new Error(info.message || t('settings.labels.createFailed', 'Failed to create label'));
    }
    await fetchLabels();
  };

  const updateLabel = async (id: string, data: LabelInput) => {
    try {
      await api.put(`/labels/${id}`, data);
    } catch (err) {
      const info = ErrorHelper.formatErrorForDisplay(err, true);
      const is = (code: ErrorCode) => ErrorHelper.isErrorCode(err, code);
      if (is(ErrorCodes.LABEL_NOT_FOUND)) throw new Error(t('labels.errors.notFound'));
      if (is(ErrorCodes.LABEL_DUPLICATE_NAME)) throw new Error(t('labels.errors.duplicateName'));
      if (is(ErrorCodes.LABEL_SYSTEM_MODIFICATION)) throw new Error(t('labels.errors.systemModification'));
      if (is(ErrorCodes.LABEL_INVALID_NAME)) throw new Error(t('labels.errors.invalidName'));
      if (is(ErrorCodes.LABEL_INVALID_COLOR)) throw new Error(t('labels.errors.invalidColor'));
      throw new Error(info.message || t('settings.labels.updateFailed', 'Failed to update label'));
    }
    await fetchLabels();
  };

  /** Resolves true when the dialog can close (deleted, or already gone). Errors land in `error`. */
  const deleteLabel = async (id: string): Promise<boolean> => {
    try {
      await api.delete(`/labels/${id}`);
      await fetchLabels();
      return true;
    } catch (err) {
      const info = ErrorHelper.formatErrorForDisplay(err, true);
      const is = (code: ErrorCode) => ErrorHelper.isErrorCode(err, code);
      if (is(ErrorCodes.LABEL_NOT_FOUND)) {
        // Refresh first: a successful fetch clears `error`, which would hide this message.
        await fetchLabels();
        setError(t('labels.errors.alreadyDeleted'));
        return true;
      }
      if (is(ErrorCodes.LABEL_IN_USE)) setError(t('labels.errors.inUse'));
      else if (is(ErrorCodes.LABEL_SYSTEM_MODIFICATION)) setError(t('labels.errors.systemDelete'));
      else setError(info.message || t('settings.labels.deleteFailed', 'Failed to delete label'));
      return false;
    }
  };

  return { labels, isLoading, error, setError, createLabel, updateLabel, deleteLabel };
}
