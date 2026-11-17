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

export const FILE_FIELDS: FieldSpec[] = [
  {
    kind: 'number',
    key: 'max_file_size_mb',
    label: `${FP}.maxFileSize`,
    help: `${FP}.maxFileSizeHelper`,
    min: 1,
    max: 500,
    integer: true,
    nullable: true,
  },
];

export const SEARCH_FIELDS: FieldSpec[] = [
  {
    kind: 'number',
    key: 'fuzzy_search_threshold',
    label: `${SC}.fuzzySearchThreshold`,
    help: `${SC}.fuzzySearchThresholdHelper`,
    min: 0,
    max: 1,
    step: 0.1,
  },
];

/** File processing and search settings (the old General tab, minus OCR). */
export default function GeneralSection() {
  const { t } = useTranslation();
  const { values: v, isLoading, loadError, save } = useSettingsData();
  const onOff = (b: boolean) =>
    b ? t('settings.summary.on', 'on') : t('settings.summary.off', 'off');

  if (isLoading)
    return <Skeleton lines={3} height={48} label={t('common.status.loading', 'Loading...')} />;

  return (
    <div className={shared.stack}>
      {loadError ? <Notice tone="danger">{loadError}</Notice> : null}

      <SettingGroup
        id="file-processing"
        title={t(`${FP}.title`)}
        summary={t('settings.summary.fileProcessing', '{{size}} · rotate {{rotate}}', {
          size:
            v.max_file_size_mb === null
              ? t('settings.summary.serverLimit', 'server limit')
              : t('settings.summary.maxSize', '{{size}} MB max', { size: v.max_file_size_mb }),
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
        summary={t('settings.summary.search', 'fuzzy {{fuzzy}}', {
          fuzzy: v.fuzzy_search_threshold,
        })}
      >
        <SettingsForm fields={SEARCH_FIELDS} values={v} onSave={save} />
      </SettingGroup>
    </div>
  );
}
