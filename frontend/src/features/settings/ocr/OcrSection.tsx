import { useTranslation } from 'react-i18next';
import { Skeleton } from '../../../ui';
import { SettingGroup } from '../fold/SettingGroup';
import { SettingsForm } from '../form/SettingsForm';
import { SettingSwitch } from '../form/SettingSwitch';
import { useSettingsData } from '../form/useSettingsData';
import formStyles from '../form/form.module.css';
import { Notice } from '../shared/Notice';
import shared from '../shared/shared.module.css';
import { useIsAdmin } from '../shared/useIsAdmin';
import { LanguagesForm } from './LanguagesForm';
import { OcrControls } from './OcrControls';
import {
  ADVANCED_FIELDS,
  ENHANCEMENT_FIELDS,
  IMAGE_SIZE_FIELDS,
  NOISE_OPTIONS,
  PREFIX,
  PROCESSING_FIELDS,
  THRESHOLD_FIELDS,
} from './ocrFields';
import { useOcrStatus } from './useOcrStatus';

const { OC, EC, QT, AP, IS } = PREFIX;

/** OCR languages, processing, admin pause/resume and every image-processing setting. */
export default function OcrSection() {
  const { t } = useTranslation();
  const isAdmin = useIsAdmin();
  const { values: v, isLoading, loadError, save } = useSettingsData();
  const ocr = useOcrStatus(isAdmin);
  const onOff = (b: boolean) => (b ? t('settings.summary.on', 'on') : t('settings.summary.off', 'off'));
  const noise = NOISE_OPTIONS.find((o) => o.value === v.ocr_noise_reduction_level);

  if (isLoading) return <Skeleton lines={4} height={48} label={t('common.status.loading', 'Loading...')} />;

  return (
    <div className={shared.stack}>
      {loadError ? <Notice tone="danger">{loadError}</Notice> : null}

      <SettingGroup
        id="ocr-languages"
        title={t('settings.groups.languages', 'Languages')}
        summary={t('settings.summary.languages', '{{languages}} · auto-detect {{auto}}', {
          languages: v.preferred_languages.length
            ? v.preferred_languages.map((l) => l.toUpperCase()).join(', ')
            : t('settings.summary.defaultLanguage', 'default'),
          auto: onOff(v.auto_detect_language_combination),
        })}
      >
        <div className={formStyles.form}>
          <LanguagesForm values={v} onSave={save} />
          <SettingSwitch
            settingKey="auto_detect_language_combination"
            label={t(`${OC}.autoDetectLanguageCombination`)}
            description={t(`${OC}.autoDetectLanguageCombinationHelper`)}
            isSelected={v.auto_detect_language_combination}
            onSave={save}
          />
        </div>
      </SettingGroup>

      <SettingGroup
        id="ocr-processing"
        title={t('settings.groups.processing', 'Processing')}
        summary={t('settings.summary.processing', '{{jobs}} jobs · {{timeout}}s · {{priority}} priority', {
          jobs: v.concurrent_ocr_jobs,
          timeout: v.ocr_timeout_seconds,
          priority: v.cpu_priority,
        })}
      >
        <SettingsForm fields={PROCESSING_FIELDS} values={v} onSave={save} />
      </SettingGroup>

      {isAdmin ? (
        <SettingGroup
          id="ocr-controls"
          title={t('settings.groups.ocrControls', 'Processing controls')}
          summary={
            ocr.status
              ? ocr.status.is_paused
                ? t('settings.summary.paused', 'paused')
                : t('settings.summary.running', 'running')
              : '—'
          }
        >
          <OcrControls status={ocr.status} busy={ocr.busy} onPause={ocr.pause} onResume={ocr.resume} />
        </SettingGroup>
      ) : null}

      <SettingGroup
        id="ocr-enhancement"
        title={t(`${EC}.title`)}
        summary={
          v.ocr_skip_enhancement
            ? t('settings.summary.enhancementSkipped', 'skipped · original images only')
            : t('settings.summary.enhancement', 'brightness {{b}} · contrast {{c}}× · noise {{n}} · sharpen {{s}}', {
                b: v.ocr_brightness_boost,
                c: v.ocr_contrast_multiplier,
                n: noise ? t(noise.label, noise.fallback).toLowerCase() : v.ocr_noise_reduction_level,
                s: v.ocr_sharpening_strength,
              })
        }
      >
        <SettingsForm
          fields={ENHANCEMENT_FIELDS}
          values={v}
          onSave={save}
          before={
            <SettingSwitch
              settingKey="ocr_skip_enhancement"
              label={t(`${EC}.skipEnhancement`)}
              isSelected={v.ocr_skip_enhancement}
              onSave={save}
            />
          }
        />
      </SettingGroup>

      <SettingGroup
        id="ocr-thresholds"
        title={t('settings.groups.qualityThresholds', 'Quality thresholds')}
        summary={t('settings.summary.thresholds', 'brightness {{b}} · contrast {{c}} · noise {{n}} · sharpness {{s}}', {
          b: v.ocr_quality_threshold_brightness,
          c: v.ocr_quality_threshold_contrast,
          n: v.ocr_quality_threshold_noise,
          s: v.ocr_quality_threshold_sharpness,
        })}
      >
        <p className={formStyles.help}>{t(`${QT}.title`)}</p>
        <SettingsForm fields={THRESHOLD_FIELDS} values={v} onSave={save} />
      </SettingGroup>

      <SettingGroup
        id="ocr-advanced"
        title={t(`${AP}.title`)}
        summary={t('settings.summary.advanced', 'window {{w}} · morphology {{m}} · equalization {{h}} · save images {{s}}', {
          w: v.ocr_adaptive_threshold_window_size,
          m: onOff(v.ocr_morphological_operations),
          h: onOff(v.ocr_histogram_equalization),
          s: onOff(v.save_processed_images),
        })}
      >
        <SettingsForm
          fields={ADVANCED_FIELDS}
          values={v}
          onSave={save}
          before={
            <>
              <SettingSwitch
                settingKey="ocr_morphological_operations"
                label={t(`${AP}.morphologicalOperations`)}
                isSelected={v.ocr_morphological_operations}
                onSave={save}
              />
              <SettingSwitch
                settingKey="ocr_histogram_equalization"
                label={t(`${AP}.histogramEqualization`)}
                isSelected={v.ocr_histogram_equalization}
                onSave={save}
              />
              <SettingSwitch
                settingKey="save_processed_images"
                label={t(`${AP}.saveProcessedImages`)}
                isSelected={v.save_processed_images}
                onSave={save}
              />
            </>
          }
        />
      </SettingGroup>

      <SettingGroup
        id="ocr-image-size"
        title={t(`${IS}.title`)}
        summary={t('settings.summary.imageSize', '{{w}} × {{h}} px · upscale {{u}}×', {
          w: v.ocr_max_image_width,
          h: v.ocr_max_image_height,
          u: v.ocr_upscale_factor,
        })}
      >
        <SettingsForm fields={IMAGE_SIZE_FIELDS} values={v} onSave={save} />
      </SettingGroup>
    </div>
  );
}
