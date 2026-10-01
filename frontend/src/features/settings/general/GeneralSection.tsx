import { useTranslation } from 'react-i18next';
import { Skeleton } from '../../../ui';
import { SettingGroup } from '../fold/SettingGroup';
import type { FieldSpec } from '../form/fields';
import { SettingsForm } from '../form/SettingsForm';
import { SettingSwitch } from '../form/SettingSwitch';
import { useSettingsData } from '../form/useSettingsData';
import formStyles from '../form/form.module.css';
import { Notice } from '../shared/Notice';
import shared from '../shared/shared.module.css';

const FP = 'settings.general.fileProcessing';
const SC = 'settings.general.searchConfiguration';
const SM = 'settings.general.storageManagement';

export const FILE_FIELDS: FieldSpec[] = [
  { kind: 'number', key: 'max_file_size_mb', label: `${FP}.maxFileSize`, help: `${FP}.maxFileSizeHelper`, min: 1, max: 500, integer: true },
  { kind: 'number', key: 'memory_limit_mb', label: `${FP}.memoryLimit`, help: `${FP}.memoryLimitHelper`, min: 128, max: 4096, integer: true },
];

export const SEARCH_FIELDS: FieldSpec[] = [
  {
    kind: 'select',
    key: 'search_results_per_page',
    label: `${SC}.resultsPerPage`,
    options: [10, 25, 50, 100].map((n) => ({ value: n, label: String(n), fallback: String(n) })),
  },
  { kind: 'number', key: 'search_snippet_length', label: `${SC}.snippetLength`, help: `${SC}.snippetLengthHelper`, min: 50, max: 500, integer: true },
  { kind: 'number', key: 'fuzzy_search_threshold', label: `${SC}.fuzzySearchThreshold`, help: `${SC}.fuzzySearchThresholdHelper`, min: 0, max: 1, step: 0.1 },
];

export const STORAGE_FIELDS: FieldSpec[] = [
  { kind: 'number', key: 'retention_days', label: `${SM}.retentionDays`, help: `${SM}.retentionDaysHelper`, min: 1, integer: true, nullable: true },
];

/** File processing, search and storage settings (the old General tab, minus OCR). */
export default function GeneralSection() {
  const { t } = useTranslation();
  const { values: v, isLoading, loadError, save } = useSettingsData();
  const onOff = (b: boolean) => (b ? t('settings.summary.on', 'on') : t('settings.summary.off', 'off'));

  if (isLoading) return <Skeleton lines={3} height={48} label={t('common.status.loading', 'Loading...')} />;

  return (
    <div className={shared.stack}>
      {loadError ? <Notice tone="danger">{loadError}</Notice> : null}

      <SettingGroup
        id="file-processing"
        title={t(`${FP}.title`)}
        summary={t('settings.summary.fileProcessing', '{{size}} MB max · {{memory}} MB memory · rotate {{rotate}}', {
          size: v.max_file_size_mb,
          memory: v.memory_limit_mb,
          rotate: onOff(v.auto_rotate_images),
        })}
      >
        <SettingsForm
          fields={FILE_FIELDS}
          values={v}
          onSave={save}
          before={
            <>
              <SettingSwitch
                settingKey="auto_rotate_images"
                label={t(`${FP}.autoRotateImages`)}
                description={t(`${FP}.autoRotateImagesHelper`)}
                isSelected={v.auto_rotate_images}
                onSave={save}
              />
              <SettingSwitch
                settingKey="enable_image_preprocessing"
                label={t(`${FP}.enableImagePreprocessing`)}
                description={t(`${FP}.enableImagePreprocessingHelper`)}
                isSelected={v.enable_image_preprocessing}
                onSave={save}
              />
              <p className={formStyles.warning}>{t(`${FP}.preprocessingWarning`)}</p>
              <SettingSwitch
                settingKey="enable_background_ocr"
                label={t(`${FP}.enableBackgroundOcr`)}
                description={t(`${FP}.enableBackgroundOcrHelper`)}
                isSelected={v.enable_background_ocr}
                onSave={save}
              />
            </>
          }
        />
      </SettingGroup>

      <SettingGroup
        id="search"
        title={t(`${SC}.title`)}
        summary={t('settings.summary.search', '{{perPage}} per page · {{snippet}} chars · fuzzy {{fuzzy}}', {
          perPage: v.search_results_per_page,
          snippet: v.search_snippet_length,
          fuzzy: v.fuzzy_search_threshold,
        })}
      >
        <SettingsForm fields={SEARCH_FIELDS} values={v} onSave={save} />
      </SettingGroup>

      <SettingGroup
        id="storage"
        title={t(`${SM}.title`)}
        summary={t('settings.summary.storage', 'keep {{retention}} · cleanup {{cleanup}} · compression {{compression}}', {
          retention:
            v.retention_days === null
              ? t('settings.summary.forever', 'forever')
              : t('settings.summary.days', '{{count}} days', { count: v.retention_days }),
          cleanup: onOff(v.enable_auto_cleanup),
          compression: onOff(v.enable_compression),
        })}
      >
        <SettingsForm
          fields={STORAGE_FIELDS}
          values={v}
          onSave={save}
          after={
            <div className={formStyles.switches}>
              <SettingSwitch
                settingKey="enable_auto_cleanup"
                label={t(`${SM}.enableAutoCleanup`)}
                description={t(`${SM}.enableAutoCleanupHelper`)}
                isSelected={v.enable_auto_cleanup}
                onSave={save}
              />
              <SettingSwitch
                settingKey="enable_compression"
                label={t(`${SM}.enableCompression`)}
                description={t(`${SM}.enableCompressionHelper`)}
                isSelected={v.enable_compression}
                onSave={save}
              />
            </div>
          }
        />
      </SettingGroup>
    </div>
  );
}
