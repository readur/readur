import { describe, expect, it } from 'vitest';
import {
  buildConfig,
  buildTestRequest,
  canTestConnection,
  defaultFolders,
  emptyForm,
  formFromSource,
  isHttpUrl,
  isValidInterval,
  normalizeExtension,
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
    expect(form.password).toBe('pass');
  });
});

describe('field validation (ported from WebDAVTab "WebDAV Data Validation")', () => {
  it('accepts http(s) server URLs and rejects anything else', () => {
    for (const url of ['https://cloud.example.com', 'http://localhost:8080', 'https://subdomain.example.com/path']) {
      expect(isHttpUrl(url)).toBe(true);
    }
    for (const url of ['not-a-url', 'ftp://example.com', '']) {
      expect(isHttpUrl(url)).toBe(false);
    }
  });

  it('requires absolute folder paths for WebDAV and local folders only', async () => {
    const { needsAbsolutePath } = await import('../connections/form/sourceFormModel');
    expect(needsAbsolutePath('webdav')).toBe(true);
    expect(needsAbsolutePath('local_folder')).toBe(true);
    expect(needsAbsolutePath('s3')).toBe(false);
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

  it('only checks the interval when automatic sync is on', () => {
    const base = { ...emptyForm('local_folder'), name: 'x', sync_interval_minutes: 5 };
    expect(validateForm(base)).toEqual({});
    expect(validateForm({ ...base, auto_sync: true }).sync_interval_minutes).toBe('interval');
  });
});
