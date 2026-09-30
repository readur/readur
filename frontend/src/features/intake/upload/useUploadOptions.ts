import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { api, ocrService } from '../../../services/api';
import type { LabelData } from '../../labels/Label';
import { ErrorCodes, hasCode, serverMessage } from '../shared/errors';

type NewLabel = Omit<LabelData, 'id' | 'is_system' | 'created_at' | 'updated_at' | 'document_count' | 'source_count'>;

/** Labels and OCR languages to apply to the next uploads. */
export function useUploadOptions() {
  const { t } = useTranslation();
  const [selectedLabels, setSelectedLabels] = useState<LabelData[]>([]);
  const [availableLabels, setAvailableLabels] = useState<LabelData[]>([]);
  const [labelsLoading, setLabelsLoading] = useState(true);
  const [languages, setLanguages] = useState<string[]>([]);
  const [primaryLanguage, setPrimaryLanguage] = useState('');

  useEffect(() => {
    let alive = true;
    api
      .get<LabelData[]>('/labels?include_counts=false')
      .then((res) => {
        if (alive && Array.isArray(res?.data)) setAvailableLabels(res.data);
      })
      .catch(() => {
        /* labels are optional for uploading */
      })
      .finally(() => alive && setLabelsLoading(false));
    ocrService
      .getAvailableLanguages()
      .then((res) => res?.data?.current_user_language || 'eng')
      .catch(() => 'eng')
      .then((lang) => {
        if (!alive) return;
        setLanguages([lang]);
        setPrimaryLanguage(lang);
      });
    return () => {
      alive = false;
    };
  }, []);

  const createLabel = async (label: NewLabel): Promise<LabelData> => {
    try {
      const res = await api.post<LabelData>('/labels', label);
      setAvailableLabels((prev) => [...prev, res.data]);
      return res.data;
    } catch (error) {
      if (hasCode(error, ErrorCodes.LABEL_DUPLICATE_NAME)) throw new Error(t('intake.upload.labelDuplicate', 'A label with this name already exists.'));
      if (hasCode(error, ErrorCodes.LABEL_INVALID_NAME)) throw new Error(t('intake.upload.labelInvalidName', 'That label name is not allowed.'));
      if (hasCode(error, ErrorCodes.LABEL_INVALID_COLOR)) throw new Error(t('intake.upload.labelInvalidColor', 'That colour is not valid.'));
      if (hasCode(error, ErrorCodes.LABEL_MAX_LABELS_REACHED)) throw new Error(t('intake.upload.labelMax', 'You have reached the label limit.'));
      throw new Error(serverMessage(error) ?? t('intake.upload.labelFailed', 'Could not create the label.'));
    }
  };

  const changeLanguages = (next: string[], primary?: string) => {
    setLanguages(next);
    setPrimaryLanguage(primary ?? next[0] ?? '');
  };

  return {
    selectedLabels,
    setSelectedLabels,
    availableLabels,
    labelsLoading,
    createLabel,
    languages,
    primaryLanguage,
    changeLanguages,
  };
}
