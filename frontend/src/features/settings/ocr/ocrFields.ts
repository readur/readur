import type { FieldSpec } from '../form/fields';

const OC = 'settings.general.ocrConfiguration';
const EC = 'settings.ocrSettings.enhancementControls';
const QT = 'settings.ocrSettings.qualityThresholds';
const AP = 'settings.ocrSettings.advancedProcessing';
const IS = 'settings.ocrSettings.imageSizeScaling';

export const PROCESSING_FIELDS: FieldSpec[] = [
  { kind: 'number', key: 'concurrent_ocr_jobs', label: `${OC}.concurrentOcrJobs`, help: `${OC}.concurrentOcrJobsHelper`, min: 1, max: 16, integer: true },
  { kind: 'number', key: 'ocr_timeout_seconds', label: `${OC}.ocrTimeout`, help: `${OC}.ocrTimeoutHelper`, min: 30, max: 3600, integer: true },
  {
    kind: 'select',
    key: 'cpu_priority',
    label: `${OC}.cpuPriority`,
    options: [
      { value: 'low', label: `${OC}.cpuPriorityLow`, fallback: 'Low' },
      { value: 'normal', label: `${OC}.cpuPriorityNormal`, fallback: 'Normal' },
      { value: 'high', label: `${OC}.cpuPriorityHigh`, fallback: 'High' },
    ],
  },
];

export const NOISE_OPTIONS = [
  { value: 0, label: `${EC}.noiseReductionNone`, fallback: 'None' },
  { value: 1, label: `${EC}.noiseReductionLight`, fallback: 'Light' },
  { value: 2, label: `${EC}.noiseReductionModerate`, fallback: 'Moderate' },
  { value: 3, label: `${EC}.noiseReductionHeavy`, fallback: 'Heavy' },
];

export const ENHANCEMENT_FIELDS: FieldSpec[] = [
  { kind: 'number', key: 'ocr_brightness_boost', label: `${EC}.brightnessBoost`, help: `${EC}.brightnessBoostHelper`, min: 0, max: 100, step: 0.1 },
  { kind: 'number', key: 'ocr_contrast_multiplier', label: `${EC}.contrastMultiplier`, help: `${EC}.contrastMultiplierHelper`, min: 0.1, max: 5, step: 0.1 },
  { kind: 'select', key: 'ocr_noise_reduction_level', label: `${EC}.noiseReductionLevel`, options: NOISE_OPTIONS },
  { kind: 'number', key: 'ocr_sharpening_strength', label: `${EC}.sharpeningStrength`, help: `${EC}.sharpeningStrengthHelper`, min: 0, max: 2, step: 0.1 },
];

export const THRESHOLD_FIELDS: FieldSpec[] = [
  { kind: 'number', key: 'ocr_quality_threshold_brightness', label: `${QT}.brightnessThreshold`, help: `${QT}.brightnessThresholdHelper`, min: 0, max: 255, step: 1 },
  { kind: 'number', key: 'ocr_quality_threshold_contrast', label: `${QT}.contrastThreshold`, help: `${QT}.contrastThresholdHelper`, min: 0, max: 1, step: 0.01 },
  { kind: 'number', key: 'ocr_quality_threshold_noise', label: `${QT}.noiseThreshold`, help: `${QT}.noiseThresholdHelper`, min: 0, max: 1, step: 0.01 },
  { kind: 'number', key: 'ocr_quality_threshold_sharpness', label: `${QT}.sharpnessThreshold`, help: `${QT}.sharpnessThresholdHelper`, min: 0, max: 1, step: 0.01 },
];

export const ADVANCED_FIELDS: FieldSpec[] = [
  {
    kind: 'number',
    key: 'ocr_adaptive_threshold_window_size',
    label: `${AP}.adaptiveThresholdWindowSize`,
    help: `${AP}.adaptiveThresholdWindowSizeHelper`,
    min: 3,
    max: 101,
    step: 2,
    odd: true,
  },
];

export const IMAGE_SIZE_FIELDS: FieldSpec[] = [
  { kind: 'number', key: 'ocr_max_image_width', label: `${IS}.maxImageWidth`, help: `${IS}.maxImageWidthHelper`, min: 100, max: 50000, step: 100, integer: true },
  { kind: 'number', key: 'ocr_max_image_height', label: `${IS}.maxImageHeight`, help: `${IS}.maxImageHeightHelper`, min: 100, max: 50000, step: 100, integer: true },
  { kind: 'number', key: 'ocr_upscale_factor', label: `${IS}.upscaleFactor`, help: `${IS}.upscaleFactorHelper`, min: 0.1, max: 5, step: 0.1 },
];

export const PREFIX = { OC, EC, QT, AP, IS };
