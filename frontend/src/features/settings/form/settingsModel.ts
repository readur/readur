import type { SettingsResponse } from '../../../types/generated';

/** The subset of `/api/settings` this UI edits, keyed exactly as the API sends and accepts them. */
export type SettingsValues = Pick<
  SettingsResponse,
  | 'ocr_language'
  | 'preferred_languages'
  | 'primary_language'
  | 'ocr_timeout_seconds'
  | 'max_file_size_mb'
  | 'auto_rotate_images'
  | 'enable_image_preprocessing'
  | 'fuzzy_search_threshold'
  | 'enable_background_ocr'
  | 'ocr_brightness_boost'
  | 'ocr_contrast_multiplier'
  | 'ocr_noise_reduction_level'
  | 'ocr_sharpening_strength'
  | 'ocr_morphological_operations'
  | 'ocr_adaptive_threshold_window_size'
  | 'ocr_histogram_equalization'
  | 'ocr_upscale_factor'
  | 'ocr_max_image_width'
  | 'ocr_max_image_height'
  | 'save_processed_images'
  | 'ocr_quality_threshold_brightness'
  | 'ocr_quality_threshold_contrast'
  | 'ocr_quality_threshold_noise'
  | 'ocr_quality_threshold_sharpness'
  | 'ocr_skip_enhancement'
>;

export type SettingKey = keyof SettingsValues;
export type SettingsPatch = Partial<SettingsValues>;

/** Defaults used when the server omits a value (the same defaults the previous settings page used). */
export const DEFAULT_SETTINGS: SettingsValues = {
  ocr_language: 'eng',
  preferred_languages: ['eng'],
  primary_language: 'eng',
  ocr_timeout_seconds: 300,
  max_file_size_mb: null,
  auto_rotate_images: true,
  enable_image_preprocessing: false,
  fuzzy_search_threshold: 0.5,
  enable_background_ocr: true,
  ocr_brightness_boost: 0,
  ocr_contrast_multiplier: 1,
  ocr_noise_reduction_level: 1,
  ocr_sharpening_strength: 0,
  ocr_morphological_operations: true,
  ocr_adaptive_threshold_window_size: 15,
  ocr_histogram_equalization: false,
  ocr_upscale_factor: 1,
  ocr_max_image_width: 10000,
  ocr_max_image_height: 10000,
  save_processed_images: false,
  ocr_quality_threshold_brightness: 40,
  ocr_quality_threshold_contrast: 0.15,
  ocr_quality_threshold_noise: 0.3,
  ocr_quality_threshold_sharpness: 0.15,
  ocr_skip_enhancement: false,
};

/**
 * Merge a server response over the defaults. Missing or null values fall back to the default,
 * except `max_file_size_mb`, where null means "use the server's limit".
 */
export function normalizeSettings(
  data: Partial<Record<string, unknown>> | null | undefined,
): SettingsValues {
  const out = { ...DEFAULT_SETTINGS } as Record<string, unknown>;
  if (!data || typeof data !== 'object') return out as SettingsValues;
  for (const key of Object.keys(DEFAULT_SETTINGS) as SettingKey[]) {
    const v = data[key];
    if (key === 'max_file_size_mb') {
      out[key] = typeof v === 'number' ? v : null;
    } else if (key === 'preferred_languages') {
      out[key] = Array.isArray(v) ? v : DEFAULT_SETTINGS.preferred_languages;
    } else if (typeof v === 'string' ? v !== '' : v !== undefined && v !== null) {
      out[key] = v;
    }
  }
  return out as SettingsValues;
}
