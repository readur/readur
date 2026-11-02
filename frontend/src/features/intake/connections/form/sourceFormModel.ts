/**
 * Source form state and the pure rules around it: defaults per type, loading an existing
 * source, the config/test payloads the backend expects, validation and the sync URL preview.
 */
import type { JsonValue, SourceResponse, SourceType, TestConnectionRequest } from '../../../../types/generated';
import { buildSourceConfig, buildTestConnectionRequest } from '../../../../services/sourceConnectionTest';

export type ServerType = 'nextcloud' | 'owncloud' | 'generic';
export type AddressingStyle = 'auto' | 'path' | 'vhost';

export interface SourceFormData {
  name: string;
  source_type: SourceType;
  enabled: boolean;
  // WebDAV
  server_url: string;
  username: string;
  password: string;
  server_type: ServerType;
  // Local folder
  recursive: boolean;
  follow_symlinks: boolean;
  // S3
  bucket_name: string;
  region: string;
  access_key_id: string;
  secret_access_key: string;
  endpoint_url: string;
  force_path_style: AddressingStyle;
  prefix: string;
  // Common
  watch_folders: string[];
  file_extensions: string[];
  auto_sync: boolean;
  sync_interval_minutes: number;
}

export const DEFAULT_EXTENSIONS = ['pdf', 'png', 'jpg', 'jpeg', 'tiff', 'bmp', 'txt'];
export const MIN_INTERVAL = 15;
export const MAX_INTERVAL = 1440;
export const DEFAULT_INTERVAL = 60;

/** Default watch folders for a new source of each type. */
export function defaultFolders(type: SourceType): string[] {
  switch (type) {
    case 'local_folder':
      return ['/home/user/Documents'];
    case 's3':
      return ['documents/'];
    case 'webdav':
    default:
      return ['/Documents'];
  }
}

export function emptyForm(type: SourceType = 'webdav'): SourceFormData {
  return {
    name: '',
    source_type: type,
    enabled: true,
    server_url: '',
    username: '',
    password: '',
    server_type: 'generic',
    recursive: true,
    follow_symlinks: false,
    bucket_name: '',
    region: 'us-east-1',
    access_key_id: '',
    secret_access_key: '',
    endpoint_url: '',
    force_path_style: 'auto',
    prefix: '',
    watch_folders: defaultFolders(type),
    file_extensions: [...DEFAULT_EXTENSIONS],
    auto_sync: false,
    sync_interval_minutes: DEFAULT_INTERVAL,
  };
}

type Config = Record<string, unknown>;
const str = (v: unknown, fallback = ''): string => (typeof v === 'string' ? v : fallback);
const list = (v: unknown, fallback: string[]): string[] =>
  Array.isArray(v) ? v.filter((x): x is string => typeof x === 'string') : fallback;

/** Form state for editing an existing source; every config value is carried over. */
export function formFromSource(source: SourceResponse): SourceFormData {
  const c = (source.config && typeof source.config === 'object' ? source.config : {}) as Config;
  const serverType = str(c.server_type, 'generic');
  return {
    name: source.name,
    source_type: source.source_type,
    enabled: source.enabled,
    server_url: str(c.server_url),
    username: str(c.username),
    // Secrets are never returned by the API; blank keeps the stored value.
    password: '',
    server_type: (['nextcloud', 'owncloud', 'generic'].includes(serverType) ? serverType : 'generic') as ServerType,
    recursive: c.recursive !== undefined ? Boolean(c.recursive) : true,
    follow_symlinks: Boolean(c.follow_symlinks),
    bucket_name: str(c.bucket_name),
    region: str(c.region, 'us-east-1') || 'us-east-1',
    access_key_id: str(c.access_key_id),
    secret_access_key: '',
    endpoint_url: str(c.endpoint_url),
    force_path_style: c.force_path_style === true ? 'path' : c.force_path_style === false ? 'vhost' : 'auto',
    prefix: str(c.prefix),
    watch_folders: list(c.watch_folders, ['/Documents']),
    file_extensions: list(c.file_extensions, [...DEFAULT_EXTENSIONS]),
    auto_sync: Boolean(c.auto_sync),
    sync_interval_minutes: typeof c.sync_interval_minutes === 'number' && c.sync_interval_minutes > 0
      ? c.sync_interval_minutes
      : DEFAULT_INTERVAL,
  };
}

/** Which secrets the server holds for a saved connection (it reports only that one is set). */
export interface StoredSecrets {
  password: boolean;
  secretAccessKey: boolean;
}

export const NO_STORED_SECRETS: StoredSecrets = { password: false, secretAccessKey: false };

export function storedSecrets(source: SourceResponse | null | undefined): StoredSecrets {
  if (!source) return NO_STORED_SECRETS;
  const c = (source.config && typeof source.config === 'object' ? source.config : {}) as Config;
  const isSet = (flag: unknown, value: unknown) => flag === true || (typeof value === 'string' && value.length > 0);
  return {
    password: source.source_type === 'webdav' && isSet(c.has_password, c.password),
    secretAccessKey: source.source_type === 's3' && isSet(c.has_secret_access_key, c.secret_access_key),
  };
}

const sanitizedInterval = (minutes: number): number =>
  Number.isFinite(minutes) && minutes > 0 ? minutes : DEFAULT_INTERVAL;

type ConfigOut = { [key: string]: JsonValue };

/**
 * The `config` object saved for the selected type. With `omitBlankSecrets` (editing) a blank
 * secret is left out, which the server reads as "keep the stored value".
 */
export function buildConfig(f: SourceFormData, { omitBlankSecrets = false }: { omitBlankSecrets?: boolean } = {}): ConfigOut {
  return buildSourceConfig(
    { ...f, sync_interval_minutes: sanitizedInterval(f.sync_interval_minutes) },
    { omitBlankSecrets },
  ) as ConfigOut;
}

/**
 * Body of POST /sources/test/connection for the current form. When editing with a blank secret
 * the saved connection's id goes along, so the server tests with the stored secret.
 */
export function buildTestRequest(
  f: SourceFormData,
  existing?: Pick<SourceResponse, 'id' | 'source_type'> | null,
): TestConnectionRequest {
  const config = buildConfig(f);
  return buildTestConnectionRequest(f.source_type, config, existing) as TestConnectionRequest;
}

/** Test connection needs the minimum to reach the server: the same rule the old form used. */
export function canTestConnection(f: SourceFormData, stored: StoredSecrets = NO_STORED_SECRETS): boolean {
  switch (f.source_type) {
    case 'webdav':
      return Boolean(f.server_url && f.username);
    case 'local_folder':
      return f.watch_folders.length > 0;
    case 's3':
      return Boolean(f.bucket_name && f.access_key_id && (f.secret_access_key || stored.secretAccessKey));
    default:
      return false;
  }
}

export type FieldKey =
  | 'name'
  | 'server_url'
  | 'username'
  | 'bucket_name'
  | 'access_key_id'
  | 'secret_access_key'
  | 'watch_folders'
  | 'sync_interval_minutes';

export type FormErrorCode =
  | 'required'
  | 'url'
  | 'folderRequired'
  | 'interval';

export type FormErrors = Partial<Record<FieldKey, FormErrorCode>>;

/**
 * Mirrors the backend WebDAV URL check (src/services/webdav/config.rs, normalize_server_url):
 * a bare host is fine (https:// is assumed), but after dropping an http(s):// scheme there must
 * be a host, no second "://" (so ftp://… is rejected), and no leading "/".
 */
export function isValidServerUrl(value: string): boolean {
  const trimmed = value.trim();
  if (!trimmed || /\s/.test(trimmed)) return false;
  const rest = trimmed.replace(/^https?:\/\//i, '');
  if (!rest) return false;
  if (rest.includes('://')) return false;
  if (rest.startsWith('/')) return false;
  return true;
}

export function isValidInterval(minutes: number): boolean {
  return Number.isInteger(minutes) && minutes >= MIN_INTERVAL && minutes <= MAX_INTERVAL;
}

/** Everything that must hold before saving. Empty object means valid. */
export function validateForm(f: SourceFormData, stored: StoredSecrets = NO_STORED_SECRETS): FormErrors {
  const errors: FormErrors = {};
  if (!f.name.trim()) errors.name = 'required';
  if (f.source_type === 'webdav') {
    if (!f.server_url.trim()) errors.server_url = 'required';
    else if (!isValidServerUrl(f.server_url)) errors.server_url = 'url';
    if (!f.username.trim()) errors.username = 'required';
  }
  if (f.source_type === 's3') {
    if (!f.bucket_name.trim()) errors.bucket_name = 'required';
    if (!f.access_key_id.trim()) errors.access_key_id = 'required';
    if (!f.secret_access_key && !stored.secretAccessKey) errors.secret_access_key = 'required';
  }
  if (f.watch_folders.length === 0) errors.watch_folders = 'folderRequired';
  if (f.auto_sync && !isValidInterval(f.sync_interval_minutes)) errors.sync_interval_minutes = 'interval';
  return errors;
}

/**
 * Absolute folder paths are recommended for WebDAV and local folders (the backend accepts
 * relative ones, so the form only advises); S3 prefixes are free-form.
 */
export function prefersAbsolutePath(type: SourceType): boolean {
  return type === 'webdav' || type === 'local_folder';
}

/** Trim, and for extensions drop a leading dot. Returns '' when nothing is left. */
export function normalizeExtension(value: string): string {
  return value.trim().replace(/^\.+/, '');
}

export interface UrlPart {
  text: string;
  type: 'server' | 'path' | 'folder' | 'file';
}

export type UrlPreviewInput = Pick<
  SourceFormData,
  'source_type' | 'server_url' | 'username' | 'server_type' | 'watch_folders' | 'bucket_name' | 'region' | 'endpoint_url' | 'prefix'
>;

/**
 * Example URL of the first file a sync would fetch, split into typed parts. Mirrors the URL the
 * backend builds (src/services/webdav/config.rs). Returns null until there is enough to show.
 */
export function buildExampleSyncUrl(formData: UrlPreviewInput): { parts: UrlPart[] } | null {
  const exampleFile = 'document1.pdf';
  const firstFolder = formData.watch_folders.length > 0 ? formData.watch_folders[0] : '/Documents';

  if (formData.source_type === 'webdav') {
    if (!formData.server_url) return null;

    let serverUrl = formData.server_url.trim();
    if (!serverUrl.startsWith('http://') && !serverUrl.startsWith('https://')) {
      serverUrl = `https://${serverUrl}`;
    }
    serverUrl = serverUrl.replace(/\/+$/, '');

    let webdavPath = '';
    if (formData.server_type === 'nextcloud') {
      if (!serverUrl.includes('/remote.php/dav/files/')) {
        webdavPath = `/remote.php/dav/files/${formData.username || 'username'}`;
      }
    } else if (formData.server_type === 'owncloud') {
      if (!serverUrl.includes('/remote.php/webdav')) {
        webdavPath = '/remote.php/webdav';
      }
    }

    const cleanFolder = firstFolder.replace(/^\/+/, '');

    return {
      parts: [
        { text: serverUrl, type: 'server' },
        { text: webdavPath, type: 'path' },
        { text: `/${cleanFolder}`, type: 'folder' },
        { text: `/${exampleFile}`, type: 'file' },
      ],
    };
  } else if (formData.source_type === 's3') {
    if (!formData.bucket_name) return null;

    const endpoint = formData.endpoint_url?.trim() || `https://s3.${formData.region || 'us-east-1'}.amazonaws.com`;
    const cleanEndpoint = endpoint.replace(/\/+$/, '');
    const prefix = formData.prefix?.trim().replace(/^\/+|\/+$/g, '') || '';
    const cleanFolder = firstFolder.replace(/^\/+|\/+$/, '');

    const parts: UrlPart[] = [
      { text: cleanEndpoint, type: 'server' },
      { text: `/${formData.bucket_name}`, type: 'path' },
      { text: `/${cleanFolder}`, type: 'folder' },
      { text: `/${exampleFile}`, type: 'file' },
    ];
    if (prefix) {
      parts.splice(2, 0, { text: `/${prefix}`, type: 'path' });
    }

    return { parts };
  } else if (formData.source_type === 'local_folder') {
    if (formData.watch_folders.length === 0) return null;

    return {
      parts: [
        { text: firstFolder, type: 'folder' },
        { text: `/${exampleFile}`, type: 'file' },
      ],
    };
  }

  return null;
}
