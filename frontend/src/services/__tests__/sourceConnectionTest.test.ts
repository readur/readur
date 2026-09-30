import { describe, it, expect } from 'vitest'
import { buildTestConnectionRequest } from '../sourceConnectionTest'

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
