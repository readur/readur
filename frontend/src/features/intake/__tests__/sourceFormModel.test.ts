import { describe, expect, it } from 'vitest';
import {
  buildConfig,
  buildTestRequest,
  canTestConnection,
  defaultFolders,
  emptyForm,
  formFromSource,
  isValidServerUrl,
  isValidInterval,
  normalizeExtension,
  storedSecrets,
  validateForm,
} from '../connections/form/sourceFormModel';
import { source } from './intakeTestUtils';
import type { SourceResponse } from '../../../services/api';

const asSource = (o: Record<string, unknown>) => o as unknown as SourceResponse;

describe('test connection payload (ported from SourcesPage.simple)', () => {
  it('uses the unified test endpoint shape for every type and sends WebDAV folders and extensions', () => {
    const form = { ...emptyForm('webdav'), server_url: 'https://example.com', username: 'user', password: 'pass', file_extensions: ['pdf', 'jpg'] };
    const req = buildTestRequest(form);
    expect(req.source_type).toBe('webdav');
    expect(req.config).toMatchObject({
      server_url: 'https://example.com',
      username: 'user',
      password: 'pass',
      server_type: 'generic',
      watch_folders: ['/Documents'],
      file_extensions: ['pdf', 'jpg'],
    });
    expect(buildTestRequest(emptyForm('local_folder')).source_type).toBe('local_folder');
    expect(buildTestRequest(emptyForm('s3')).source_type).toBe('s3');
  });

  it('requires at least one watch folder before a connection can be saved', () => {
    const valid = { ...emptyForm('webdav'), name: 'x', server_url: 'https://webdav.example.com', username: 'u' };
    expect(validateForm(valid)).toEqual({});
    expect(validateForm({ ...valid, watch_folders: [] }).watch_folders).toBe('folderRequired');
  });

  it('keeps the watch folders and extensions of an existing source when editing', () => {
    const existing = asSource(
      source('s1', {
        config: {
          server_url: 'https://existing.webdav.com',
          username: 'existing',
          password: 'pass',
          server_type: 'nextcloud',
          watch_folders: ['/Documents', '/Photos'],
          file_extensions: ['pdf', 'jpg', 'png'],
        },
      }),
    );
    const form = formFromSource(existing);
    expect(form.watch_folders).toEqual(['/Documents', '/Photos']);
    expect(form.file_extensions).toEqual(['pdf', 'jpg', 'png']);
    expect(form.server_type).toBe('nextcloud');
    // Secrets never come back from the API; a blank field keeps the stored one.
    expect(form.password).toBe('');
  });
});

describe('stored secrets of a saved connection', () => {
  const webdav = asSource(
    source('s1', { config: { server_url: 'https://dav.example', username: 'u', has_password: true, watch_folders: ['/D'] } }),
  );
  const s3 = asSource(
    source('s2', {
      source_type: 's3',
      config: { bucket_name: 'b', access_key_id: 'AK', has_secret_access_key: true, watch_folders: ['docs/'] },
    }),
  );

  it('reads the flags the server reports instead of the secrets', () => {
    expect(storedSecrets(webdav)).toEqual({ password: true, secretAccessKey: false });
    expect(storedSecrets(s3)).toEqual({ password: false, secretAccessKey: true });
    expect(storedSecrets(null)).toEqual({ password: false, secretAccessKey: false });
  });

  it('tests an edited connection with the stored secret by sending its id', () => {
    const req = buildTestRequest({ ...formFromSource(webdav), password: '' }, webdav);
    expect(req.source_id).toBe('s1');
    expect((req.config as Record<string, unknown>).password).toBe('');
  });

  it('tests with a newly typed secret without the id', () => {
    const req = buildTestRequest({ ...formFromSource(webdav), password: 'new' }, webdav);
    expect(req.source_id).toBeUndefined();
    expect((req.config as Record<string, unknown>).password).toBe('new');
  });

  it('never saves a blank secret for an existing connection', () => {
    expect(buildConfig(formFromSource(webdav), { omitBlankSecrets: true })).not.toHaveProperty('password');
    expect(buildConfig(formFromSource(s3), { omitBlankSecrets: true })).not.toHaveProperty('secret_access_key');
    expect(buildConfig({ ...formFromSource(webdav), password: 'p' }, { omitBlankSecrets: true }).password).toBe('p');
  });

  it('does not require the S3 secret again when one is stored', () => {
    const form = { ...formFromSource(s3), name: 'x' };
    expect(validateForm(form).secret_access_key).toBe('required');
    expect(validateForm(form, storedSecrets(s3)).secret_access_key).toBeUndefined();
    expect(canTestConnection(form)).toBe(false);
    expect(canTestConnection(form, storedSecrets(s3))).toBe(true);
  });
});

describe('field validation (ported from WebDAVTab "WebDAV Data Validation")', () => {
  it('accepts http(s) URLs and bare hosts like the backend, and rejects anything else', () => {
    for (const url of ['https://cloud.example.com', 'http://localhost:8080', 'https://subdomain.example.com/path', 'cloud.example.com', 'nas.local:8443/dav']) {
      expect(isValidServerUrl(url)).toBe(true);
    }
    for (const url of ['ftp://example.com', '', '   ', '/relative/path', 'https://', 'https:///dav', 'http://a://b', 'has space.com']) {
      expect(isValidServerUrl(url)).toBe(false);
    }
  });

  it('recommends absolute folder paths for WebDAV and local folders only', async () => {
    const { prefersAbsolutePath } = await import('../connections/form/sourceFormModel');
    expect(prefersAbsolutePath('webdav')).toBe(true);
    expect(prefersAbsolutePath('local_folder')).toBe(true);
    expect(prefersAbsolutePath('s3')).toBe(false);
  });

  it('normalizes extensions by trimming and dropping a leading dot', () => {
    expect(normalizeExtension(' .pdf ')).toBe('pdf');
    expect(normalizeExtension('docx')).toBe('docx');
    expect(normalizeExtension('  ')).toBe('');
  });

  it('accepts sync intervals from 15 to 1440 whole minutes', () => {
    for (const n of [15, 30, 60, 120, 1440]) expect(isValidInterval(n)).toBe(true);
    for (const n of [5, 10, 2000, 30.5, NaN]) expect(isValidInterval(n)).toBe(false);
  });
});

describe('source form model', () => {
  it('uses type-specific default folders for new connections', () => {
    expect(defaultFolders('webdav')).toEqual(['/Documents']);
    expect(defaultFolders('local_folder')).toEqual(['/home/user/Documents']);
    expect(defaultFolders('s3')).toEqual(['documents/']);
    expect(emptyForm('s3').watch_folders).toEqual(['documents/']);
  });

  it('builds the saved WebDAV config with every field', () => {
    const form = { ...emptyForm('webdav'), server_url: 'https://dav', username: 'u', password: 'p', server_type: 'owncloud' as const, auto_sync: true, sync_interval_minutes: 30 };
    expect(buildConfig(form)).toEqual({
      server_url: 'https://dav',
      username: 'u',
      password: 'p',
      watch_folders: ['/Documents'],
      file_extensions: ['pdf', 'png', 'jpg', 'jpeg', 'tiff', 'bmp', 'txt'],
      auto_sync: true,
      sync_interval_minutes: 30,
      server_type: 'owncloud',
    });
  });

  it('builds the saved local-folder config with recursive and symlink options', () => {
    const form = { ...emptyForm('local_folder'), recursive: false, follow_symlinks: true };
    expect(buildConfig(form)).toMatchObject({ recursive: false, follow_symlinks: true, watch_folders: ['/home/user/Documents'] });
    expect(buildConfig(form)).not.toHaveProperty('server_url');
  });

  it.each([
    ['path', true],
    ['vhost', false],
    ['auto', null],
  ] as const)('maps S3 addressing style %s to force_path_style %s in config and test payload', (style, value) => {
    const form = { ...emptyForm('s3'), bucket_name: 'b', access_key_id: 'k', secret_access_key: 's', force_path_style: style };
    expect(buildConfig(form).force_path_style).toBe(value);
    expect((buildTestRequest(form).config as Record<string, unknown>).force_path_style).toBe(value);
  });

  it('reads force_path_style back into the addressing style when editing', () => {
    const s3 = (v: unknown) => formFromSource(asSource(source('s', { source_type: 's3', config: { bucket_name: 'b', force_path_style: v } })));
    expect(s3(true).force_path_style).toBe('path');
    expect(s3(false).force_path_style).toBe('vhost');
    expect(s3(null).force_path_style).toBe('auto');
  });

  it('enables Test connection only with the minimum for each type', () => {
    expect(canTestConnection(emptyForm('webdav'))).toBe(false);
    expect(canTestConnection({ ...emptyForm('webdav'), server_url: 'https://x', username: 'u' })).toBe(true);
    expect(canTestConnection({ ...emptyForm('local_folder'), watch_folders: [] })).toBe(false);
    expect(canTestConnection(emptyForm('local_folder'))).toBe(true);
    expect(canTestConnection({ ...emptyForm('s3'), bucket_name: 'b', access_key_id: 'k' })).toBe(false);
    expect(canTestConnection({ ...emptyForm('s3'), bucket_name: 'b', access_key_id: 'k', secret_access_key: 's' })).toBe(true);
  });

  it('reports every missing required field per type', () => {
    expect(validateForm(emptyForm('webdav'))).toEqual({ name: 'required', server_url: 'required', username: 'required' });
    expect(validateForm({ ...emptyForm('s3'), name: 'x' })).toEqual({
      bucket_name: 'required',
      access_key_id: 'required',
      secret_access_key: 'required',
    });
    expect(validateForm({ ...emptyForm('local_folder'), name: 'x' })).toEqual({});
  });

  it('never saves a non-number interval: it falls back to 60 minutes', () => {
    const form = { ...emptyForm('local_folder'), sync_interval_minutes: Number.NaN };
    expect(buildConfig(form).sync_interval_minutes).toBe(60);
    expect(buildConfig({ ...form, sync_interval_minutes: 30 }).sync_interval_minutes).toBe(30);
  });

  it('only checks the interval when automatic sync is on', () => {
    const base = { ...emptyForm('local_folder'), name: 'x', sync_interval_minutes: 5 };
    expect(validateForm(base)).toEqual({});
    expect(validateForm({ ...base, auto_sync: true }).sync_interval_minutes).toBe('interval');
  });
});
