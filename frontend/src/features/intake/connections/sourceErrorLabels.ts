/**
 * Labels and marks for the generated source-error enums. They are PascalCase on the wire
 * ("Timeout", "High", "WebDAV"), so every value is mapped explicitly here.
 */
import type { TFunction } from 'i18next';
import type { StatusState } from '../../../ui';
import type { ErrorSourceType, SourceErrorSeverity, SourceErrorType, SourceType } from '../../../types/generated';

export const SOURCE_ERROR_TYPES: readonly SourceErrorType[] = [
  'Timeout',
  'PermissionDenied',
  'NetworkError',
  'ServerError',
  'PathTooLong',
  'InvalidCharacters',
  'TooManyItems',
  'DepthLimit',
  'SizeLimit',
  'XmlParseError',
  'JsonParseError',
  'QuotaExceeded',
  'RateLimited',
  'NotFound',
  'Conflict',
  'UnsupportedOperation',
  'Unknown',
];

export const SOURCE_ERROR_SEVERITIES: readonly SourceErrorSeverity[] = ['Low', 'Medium', 'High', 'Critical'];

export function errorTypeLabel(t: TFunction, type: SourceErrorType | string): string {
  switch (type) {
    case 'Timeout':
      return t('intake.sourceErrors.type.Timeout', 'Timeout');
    case 'PermissionDenied':
      return t('intake.sourceErrors.type.PermissionDenied', 'Permission denied');
    case 'NetworkError':
      return t('intake.sourceErrors.type.NetworkError', 'Network error');
    case 'ServerError':
      return t('intake.sourceErrors.type.ServerError', 'Server error');
    case 'PathTooLong':
      return t('intake.sourceErrors.type.PathTooLong', 'Path too long');
    case 'InvalidCharacters':
      return t('intake.sourceErrors.type.InvalidCharacters', 'Invalid characters');
    case 'TooManyItems':
      return t('intake.sourceErrors.type.TooManyItems', 'Too many items');
    case 'DepthLimit':
      return t('intake.sourceErrors.type.DepthLimit', 'Folder depth limit');
    case 'SizeLimit':
      return t('intake.sourceErrors.type.SizeLimit', 'Size limit');
    case 'XmlParseError':
      return t('intake.sourceErrors.type.XmlParseError', 'Unreadable XML response');
    case 'JsonParseError':
      return t('intake.sourceErrors.type.JsonParseError', 'Unreadable JSON response');
    case 'QuotaExceeded':
      return t('intake.sourceErrors.type.QuotaExceeded', 'Quota exceeded');
    case 'RateLimited':
      return t('intake.sourceErrors.type.RateLimited', 'Rate limited');
    case 'NotFound':
      return t('intake.sourceErrors.type.NotFound', 'Not found');
    case 'Conflict':
      return t('intake.sourceErrors.type.Conflict', 'Conflict');
    case 'UnsupportedOperation':
      return t('intake.sourceErrors.type.UnsupportedOperation', 'Unsupported operation');
    case 'Unknown':
      return t('intake.sourceErrors.type.Unknown', 'Unknown error');
    default:
      return String(type);
  }
}

export function severityLabel(t: TFunction, severity: SourceErrorSeverity | string): string {
  switch (severity) {
    case 'Low':
      return t('intake.sourceErrors.severity.Low', 'Low');
    case 'Medium':
      return t('intake.sourceErrors.severity.Medium', 'Medium');
    case 'High':
      return t('intake.sourceErrors.severity.High', 'High');
    case 'Critical':
      return t('intake.sourceErrors.severity.Critical', 'Critical');
    default:
      return String(severity);
  }
}

/** High and Critical failures show as ERROR (▲); Low and Medium as CHECK (◆). */
export function severityState(severity: SourceErrorSeverity | string): StatusState {
  return severity === 'High' || severity === 'Critical' ? 'error' : 'warning';
}

/** The error tracker's source type for a connection type. */
export function errorSourceTypeOf(type: SourceType): ErrorSourceType {
  switch (type) {
    case 'webdav':
      return 'WebDAV';
    case 's3':
      return 'S3';
    case 'local_folder':
    default:
      return 'Local';
  }
}
