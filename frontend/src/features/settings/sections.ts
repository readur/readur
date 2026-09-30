export type SectionId = 'general' | 'account' | 'ocr' | 'users' | 'server' | 'api-keys' | 'labels' | 'debug' | 'appearance';

/** A searchable entry: a group title or a single setting, pointing at the group that holds it. */
export interface SearchEntry {
  group: string;
  label: string;
  fallback: string;
}

export interface SectionDef {
  id: SectionId;
  label: string;
  fallback: string;
  adminOnly?: boolean;
  entries: SearchEntry[];
}

const e = (group: string, label: string, fallback: string): SearchEntry => ({ group, label, fallback });

export const SECTIONS: readonly SectionDef[] = [
  {
    id: 'general',
    label: 'settings.sections.general',
    fallback: 'General',
    entries: [
      e('file-processing', 'settings.general.fileProcessing.title', 'File Processing'),
      e('file-processing', 'settings.general.fileProcessing.maxFileSize', 'Max File Size (MB)'),
      e('file-processing', 'settings.general.fileProcessing.memoryLimit', 'Memory Limit (MB)'),
      e('file-processing', 'settings.general.fileProcessing.autoRotateImages', 'Auto-rotate Images'),
      e('file-processing', 'settings.general.fileProcessing.enableImagePreprocessing', 'Enable Image Preprocessing'),
      e('file-processing', 'settings.general.fileProcessing.enableBackgroundOcr', 'Enable Background OCR'),
      e('search', 'settings.general.searchConfiguration.title', 'Search Configuration'),
      e('search', 'settings.general.searchConfiguration.resultsPerPage', 'Results Per Page'),
      e('search', 'settings.general.searchConfiguration.snippetLength', 'Snippet Length'),
      e('search', 'settings.general.searchConfiguration.fuzzySearchThreshold', 'Fuzzy Search Threshold'),
      e('storage', 'settings.general.storageManagement.title', 'Storage Management'),
      e('storage', 'settings.general.storageManagement.retentionDays', 'Retention Days'),
      e('storage', 'settings.general.storageManagement.enableAutoCleanup', 'Enable Auto Cleanup'),
      e('storage', 'settings.general.storageManagement.enableCompression', 'Enable Compression'),
    ],
  },
  {
    id: 'account',
    label: 'settings.sections.account',
    fallback: 'Account',
    entries: [
      e('account', 'settings.account.changePassword', 'Change password'),
      e('account', 'settings.account.currentPassword', 'Current password'),
    ],
  },
  {
    id: 'ocr',
    label: 'settings.sections.ocr',
    fallback: 'OCR',
    entries: [
      e('ocr-languages', 'settings.groups.languages', 'Languages'),
      e('ocr-languages', 'settings.general.ocrConfiguration.autoDetectLanguageCombination', 'Auto-detect language combinations'),
      e('ocr-processing', 'settings.groups.processing', 'Processing'),
      e('ocr-processing', 'settings.general.ocrConfiguration.concurrentOcrJobs', 'Concurrent OCR Jobs'),
      e('ocr-processing', 'settings.general.ocrConfiguration.ocrTimeout', 'OCR Timeout (seconds)'),
      e('ocr-processing', 'settings.general.ocrConfiguration.cpuPriority', 'CPU Priority'),
      e('ocr-controls', 'settings.groups.ocrControls', 'Processing controls'),
      e('ocr-enhancement', 'settings.ocrSettings.enhancementControls.title', 'Enhancement Controls'),
      e('ocr-enhancement', 'settings.ocrSettings.enhancementControls.skipEnhancement', 'Skip All Image Enhancement'),
      e('ocr-enhancement', 'settings.ocrSettings.enhancementControls.brightnessBoost', 'Brightness Boost'),
      e('ocr-enhancement', 'settings.ocrSettings.enhancementControls.contrastMultiplier', 'Contrast Multiplier'),
      e('ocr-enhancement', 'settings.ocrSettings.enhancementControls.noiseReductionLevel', 'Noise Reduction Level'),
      e('ocr-enhancement', 'settings.ocrSettings.enhancementControls.sharpeningStrength', 'Sharpening Strength'),
      e('ocr-thresholds', 'settings.groups.qualityThresholds', 'Quality thresholds'),
      e('ocr-thresholds', 'settings.ocrSettings.qualityThresholds.brightnessThreshold', 'Brightness Threshold'),
      e('ocr-thresholds', 'settings.ocrSettings.qualityThresholds.contrastThreshold', 'Contrast Threshold'),
      e('ocr-thresholds', 'settings.ocrSettings.qualityThresholds.noiseThreshold', 'Noise Threshold'),
      e('ocr-thresholds', 'settings.ocrSettings.qualityThresholds.sharpnessThreshold', 'Sharpness Threshold'),
      e('ocr-advanced', 'settings.ocrSettings.advancedProcessing.title', 'Advanced Processing Options'),
      e('ocr-advanced', 'settings.ocrSettings.advancedProcessing.morphologicalOperations', 'Morphological Operations'),
      e('ocr-advanced', 'settings.ocrSettings.advancedProcessing.histogramEqualization', 'Histogram Equalization'),
      e('ocr-advanced', 'settings.ocrSettings.advancedProcessing.saveProcessedImages', 'Save Processed Images for Review'),
      e('ocr-advanced', 'settings.ocrSettings.advancedProcessing.adaptiveThresholdWindowSize', 'Adaptive Threshold Window Size'),
      e('ocr-image-size', 'settings.ocrSettings.imageSizeScaling.title', 'Image Size and Scaling'),
      e('ocr-image-size', 'settings.ocrSettings.imageSizeScaling.maxImageWidth', 'Max Image Width'),
      e('ocr-image-size', 'settings.ocrSettings.imageSizeScaling.maxImageHeight', 'Max Image Height'),
      e('ocr-image-size', 'settings.ocrSettings.imageSizeScaling.upscaleFactor', 'Upscale Factor'),
    ],
  },
  {
    id: 'users',
    label: 'settings.sections.users',
    fallback: 'Users',
    adminOnly: true,
    entries: [
      e('users', 'settings.userManagement.addUser', 'Add User'),
      e('users', 'settings.userManagement.tableHeaders.watchDirectory', 'Watch Directory'),
    ],
  },
  {
    id: 'server',
    label: 'settings.sections.server',
    fallback: 'Server',
    adminOnly: true,
    entries: [
      e('server-upload', 'settings.serverConfiguration.fileUpload.title', 'File Upload Configuration'),
      e('server-ocr', 'settings.serverConfiguration.ocrProcessing.title', 'OCR Processing Configuration'),
      e('server-info', 'settings.serverConfiguration.serverInformation.title', 'Server Information'),
      e('server-info', 'settings.serverConfiguration.serverInformation.version', 'Version'),
      e('server-watch', 'settings.serverConfiguration.watchFolderConfiguration.title', 'Watch Folder Configuration'),
    ],
  },
  {
    id: 'api-keys',
    label: 'settings.sections.apiKeys',
    fallback: 'API keys',
    entries: [e('api-keys', 'settings.apiKeys.create', 'Create API key')],
  },
  {
    id: 'labels',
    label: 'settings.sections.labels',
    fallback: 'Labels',
    entries: [e('labels', 'labels.actions.createLabel', 'Create Label')],
  },
  {
    id: 'debug',
    label: 'settings.sections.debug',
    fallback: 'Debug',
    adminOnly: true,
    entries: [
      e('debug', 'debug.tabs.uploadAndDebug', 'Upload & Debug'),
      e('debug', 'debug.tabs.searchExisting', 'Search Existing'),
    ],
  },
  {
    id: 'appearance',
    label: 'settings.sections.appearance',
    fallback: 'Appearance',
    entries: [
      e('appearance-theme', 'settings.appearance.theme', 'Theme'),
      e('appearance-language', 'settings.appearance.language', 'Interface language'),
    ],
  },
];

export const DEFAULT_SECTION: SectionId = 'general';

export function findSection(id: string | undefined): SectionDef | undefined {
  return SECTIONS.find((s) => s.id === id);
}

export function sectionPath(id: SectionId): string {
  return id === DEFAULT_SECTION ? '/settings' : `/settings/${id}`;
}
