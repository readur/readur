/**
 * Request body for `POST /sources/test/connection`.
 *
 * Secrets of an existing source are never sent back to the browser, so the
 * edit form leaves them blank. When testing an existing source with a blank
 * secret, include its `source_id` so the server fills in the stored value.
 */

type SourceType = 'webdav' | 'local_folder' | 's3'

const SECRET_FIELDS: Partial<Record<SourceType, string>> = {
  webdav: 'password',
  s3: 'secret_access_key',
}

export interface TestConnectionRequest {
  source_type: SourceType
  config: Record<string, unknown>
  source_id?: string
}

export const buildTestConnectionRequest = (
  sourceType: SourceType,
  config: Record<string, unknown>,
  existingSource?: { id: string; source_type: string } | null,
): TestConnectionRequest => {
  const request: TestConnectionRequest = { source_type: sourceType, config }
  const secretField = SECRET_FIELDS[sourceType]
  if (
    existingSource &&
    existingSource.source_type === sourceType &&
    secretField &&
    !config[secretField]
  ) {
    request.source_id = existingSource.id
  }
  return request
}
