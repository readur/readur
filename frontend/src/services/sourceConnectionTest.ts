/**
 * Request body for `POST /sources/test/connection`.
 *
 * Secrets of an existing source are never sent back to the browser, so the
 * edit form leaves them blank. When testing an existing source with a blank
 * secret, include its `source_id` so the server fills in the stored value.
 */

type SourceType = 'webdav' | 'local_folder' | 's3'

/** The source form fields that make up a source configuration. */
export interface SourceConfigForm {
  source_type: SourceType
  server_url: string
  username: string
  password: string
  server_type: string
  recursive: boolean
  follow_symlinks: boolean
  bucket_name: string
  region: string
  access_key_id: string
  secret_access_key: string
  endpoint_url: string
  force_path_style: 'auto' | 'path' | 'vhost'
  prefix: string
  watch_folders: string[]
  file_extensions: string[]
  auto_sync: boolean
  sync_interval_minutes: number
}

const SECRET_FIELDS: Partial<Record<SourceType, string>> = {
  webdav: 'password',
  s3: 'secret_access_key',
}

/**
 * The complete configuration object for a source type, as the server's typed
 * configs expect it. With `omitBlankSecrets` a blank secret is left out
 * entirely, which on update means "keep the stored value".
 */
export const buildSourceConfig = (
  form: SourceConfigForm,
  { omitBlankSecrets = false }: { omitBlankSecrets?: boolean } = {},
): Record<string, unknown> => {
  const common = {
    watch_folders: form.watch_folders,
    file_extensions: form.file_extensions,
    auto_sync: form.auto_sync,
    sync_interval_minutes: form.sync_interval_minutes,
  }
  const secret = (key: string, value: string): Record<string, string> =>
    omitBlankSecrets && !value ? {} : { [key]: value }
  switch (form.source_type) {
    case 'webdav':
      return {
        server_url: form.server_url,
        username: form.username,
        ...secret('password', form.password),
        server_type: form.server_type,
        ...common,
      }
    case 'local_folder':
      return { ...common, recursive: form.recursive, follow_symlinks: form.follow_symlinks }
    case 's3':
      return {
        bucket_name: form.bucket_name,
        region: form.region,
        access_key_id: form.access_key_id,
        ...secret('secret_access_key', form.secret_access_key),
        endpoint_url: form.endpoint_url,
        force_path_style:
          form.force_path_style === 'path' ? true : form.force_path_style === 'vhost' ? false : null,
        prefix: form.prefix,
        ...common,
      }
  }
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
