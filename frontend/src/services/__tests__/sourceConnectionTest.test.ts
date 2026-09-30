import { describe, it, expect } from 'vitest'
import { buildSourceConfig, buildTestConnectionRequest, type SourceConfigForm } from '../sourceConnectionTest'

const webdavSource = { id: 'src-1', source_type: 'webdav' }

describe('buildTestConnectionRequest', () => {
  it('omits source_id for a new source', () => {
    const request = buildTestConnectionRequest('webdav', { password: '' }, null)
    expect(request).toEqual({ source_type: 'webdav', config: { password: '' } })
  })

  it('sends source_id when editing with a blank password', () => {
    const request = buildTestConnectionRequest('webdav', { password: '' }, webdavSource)
    expect(request.source_id).toBe('src-1')
  })

  it('omits source_id when a new password is entered', () => {
    const request = buildTestConnectionRequest('webdav', { password: 'new' }, webdavSource)
    expect(request.source_id).toBeUndefined()
  })

  it('sends source_id for S3 with a blank secret key', () => {
    const request = buildTestConnectionRequest(
      's3',
      { secret_access_key: '' },
      { id: 'src-2', source_type: 's3' },
    )
    expect(request.source_id).toBe('src-2')
  })

  it('omits source_id when the source type differs or has no secret', () => {
    expect(buildTestConnectionRequest('s3', {}, webdavSource).source_id).toBeUndefined()
    expect(
      buildTestConnectionRequest('local_folder', {}, { id: 'src-3', source_type: 'local_folder' }).source_id,
    ).toBeUndefined()
  })
})

const form = (overrides: Partial<SourceConfigForm> = {}): SourceConfigForm => ({
  source_type: 'webdav',
  server_url: 'https://dav.example',
  username: 'u',
  password: '',
  server_type: 'generic',
  recursive: true,
  follow_symlinks: false,
  bucket_name: 'b',
  region: 'us-east-1',
  access_key_id: 'AK',
  secret_access_key: '',
  endpoint_url: '',
  force_path_style: 'auto',
  prefix: '',
  watch_folders: ['/Documents'],
  file_extensions: ['pdf'],
  auto_sync: true,
  sync_interval_minutes: 30,
  ...overrides,
})

// Every field the server's typed source configs require.
const REQUIRED_FIELDS = {
  webdav: ['server_url', 'username', 'password', 'watch_folders', 'file_extensions', 'auto_sync', 'sync_interval_minutes'],
  local_folder: ['watch_folders', 'file_extensions', 'auto_sync', 'sync_interval_minutes', 'recursive', 'follow_symlinks'],
  s3: ['bucket_name', 'region', 'access_key_id', 'secret_access_key', 'watch_folders', 'file_extensions', 'auto_sync', 'sync_interval_minutes'],
} as const

describe('buildSourceConfig', () => {
  it.each(Object.entries(REQUIRED_FIELDS))('includes every required %s field', (sourceType, fields) => {
    const config = buildSourceConfig(form({ source_type: sourceType as SourceConfigForm['source_type'] }))
    for (const field of fields) {
      expect(config).toHaveProperty(field)
    }
  })

  it('carries sync settings from the form', () => {
    const config = buildSourceConfig(form({ source_type: 's3' }))
    expect(config.auto_sync).toBe(true)
    expect(config.sync_interval_minutes).toBe(30)
    expect(config.force_path_style).toBeNull()
  })

  it('maps the addressing style to force_path_style', () => {
    expect(buildSourceConfig(form({ source_type: 's3', force_path_style: 'path' })).force_path_style).toBe(true)
    expect(buildSourceConfig(form({ source_type: 's3', force_path_style: 'vhost' })).force_path_style).toBe(false)
  })

  it('omits blank secrets only when asked', () => {
    expect(buildSourceConfig(form(), { omitBlankSecrets: true })).not.toHaveProperty('password')
    expect(buildSourceConfig(form({ password: 'p' }), { omitBlankSecrets: true }).password).toBe('p')
    expect(
      buildSourceConfig(form({ source_type: 's3' }), { omitBlankSecrets: true }),
    ).not.toHaveProperty('secret_access_key')
  })

  it('does not leak fields from other source types', () => {
    const config = buildSourceConfig(form({ source_type: 'local_folder' }))
    expect(config).not.toHaveProperty('password')
    expect(config).not.toHaveProperty('secret_access_key')
  })
})
