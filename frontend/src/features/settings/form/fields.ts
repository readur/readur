import type { TFunction } from 'i18next';
import type { SettingKey, SettingsValues } from './settingsModel';

interface BaseField {
  key: SettingKey;
  /** i18n key of the visible label. */
  label: string;
  /** i18n key of the helper text, if any. */
  help?: string;
}

export interface NumberField extends BaseField {
  kind: 'number';
  min: number;
  /** Omitted means no upper bound. */
  max?: number;
  step?: number;
  integer?: boolean;
  /** Must be an odd integer (adaptive threshold window). */
  odd?: boolean;
  /** Empty input is allowed and saved as null. */
  nullable?: boolean;
}

export interface SelectField extends BaseField {
  kind: 'select';
  options: { value: string | number; label: string; fallback: string }[];
}

export type FieldSpec = NumberField | SelectField;

/** Draft value as typed. Numbers are kept as strings until saved. */
export function formatValue(field: FieldSpec, value: SettingsValues[SettingKey]): string {
  if (value === null || value === undefined) return '';
  return String(value);
}

/** Returns an error message, or null when the draft is valid. */
export function validateField(field: FieldSpec, raw: string, t: TFunction): string | null {
  if (field.kind === 'select') {
    return field.options.some((o) => String(o.value) === raw)
      ? null
      : t('settings.validation.choose', 'Choose one of the options.');
  }
  const text = raw.trim();
  if (text === '') {
    return field.nullable ? null : t('settings.validation.required', 'Enter a value.');
  }
  const n = Number(text);
  if (!Number.isFinite(n)) return t('settings.validation.number', 'Enter a number.');
  if ((field.integer || field.odd) && !Number.isInteger(n)) {
    return t('settings.validation.integer', 'Enter a whole number.');
  }
  if (field.max === undefined && n < field.min) {
    return t('settings.validation.min', 'Enter {{min}} or more.', { min: field.min });
  }
  if (field.max !== undefined && (n < field.min || n > field.max)) {
    return t('settings.validation.range', 'Enter a value from {{min}} to {{max}}.', {
      min: field.min,
      max: field.max,
    });
  }
  if (field.odd && n % 2 === 0) return t('settings.validation.odd', 'Enter an odd number.');
  return null;
}

/** Converts a valid draft string into the API value. */
export function parseField(field: FieldSpec, raw: string): SettingsValues[SettingKey] {
  if (field.kind === 'select') {
    const option = field.options.find((o) => String(o.value) === raw);
    return (option ? option.value : raw) as SettingsValues[SettingKey];
  }
  const text = raw.trim();
  if (text === '' && field.nullable) return null;
  return Number(text);
}
