import type { TFunction } from 'i18next';
import { useTranslation } from 'react-i18next';
import { Select, SelectItem, Switch, TextField } from '../../../../ui';
import { Notice } from '../../shared/parts';
import { ChoiceField } from './fields';
import {
  MAX_INTERVAL,
  MIN_INTERVAL,
  type AddressingStyle,
  type FieldKey,
  type FormErrorCode,
  type FormErrors,
  type ServerType,
  type SourceFormData,
  type StoredSecrets,
} from './sourceFormModel';
import styles from './SourceForm.module.css';

export interface TypeFieldsProps {
  form: SourceFormData;
  set: (patch: Partial<SourceFormData>) => void;
  errors: FormErrors;
  /** Set when editing a saved connection: which secrets the server already holds. */
  stored?: StoredSecrets;
}

/** Placeholder and hint for a secret field of a saved connection, whose value is never shown. */
function useSecretHints(stored: StoredSecrets | undefined, isStored: boolean, storedText: string) {
  const { t } = useTranslation();
  if (!stored) return {};
  return {
    placeholder: t('sources.form.keepCurrentSecret', 'Leave blank to keep current'),
    description: isStored ? storedText : undefined,
  };
}

export function errorText(t: TFunction, code?: FormErrorCode): string | undefined {
  switch (code) {
    case 'required':
      return t('intake.form.errors.required', 'Required');
    case 'url':
      return t('intake.form.errors.url', 'Enter a server address such as cloud.example.com or https://cloud.example.com');
    case 'folderRequired':
      return t('intake.form.errors.folderRequired', 'Add at least one folder');
    case 'interval':
      return t('intake.form.errors.interval', 'Enter whole minutes between {{min}} and {{max}}', {
        min: MIN_INTERVAL,
        max: MAX_INTERVAL,
      });
    default:
      return undefined;
  }
}

function useFieldError(errors: FormErrors) {
  const { t } = useTranslation();
  return (key: FieldKey) => {
    const message = errorText(t, errors[key]);
    return { isInvalid: Boolean(message), errorMessage: message };
  };
}

const SERVER_URL_PLACEHOLDER: Record<ServerType, string> = {
  nextcloud: 'https://nextcloud.example.com/',
  owncloud: 'https://owncloud.example.com/remote.php/dav/files/username/',
  generic: 'https://webdav.example.com/dav/',
};

export function WebDAVFields({ form, set, errors, stored }: TypeFieldsProps) {
  const { t } = useTranslation();
  const err = useFieldError(errors);
  const secretHints = useSecretHints(
    stored,
    Boolean(stored?.password),
    t('sources.form.passwordStored', 'A password is stored for this source'),
  );
  return (
    <fieldset className={styles.group}>
      <legend className={styles.legend}>{t('intake.form.webdav.title', 'WebDAV server')}</legend>
      <ChoiceField<ServerType>
        label={t('intake.form.webdav.serverType', 'Server type')}
        value={form.server_type}
        onChange={(server_type) => set({ server_type })}
        options={[
          { value: 'nextcloud', label: 'Nextcloud', description: t('intake.form.webdav.nextcloud', 'Optimized for Nextcloud servers') },
          { value: 'owncloud', label: 'ownCloud', description: t('intake.form.webdav.owncloud', 'Optimized for ownCloud servers') },
          { value: 'generic', label: t('intake.form.webdav.generic', 'Generic WebDAV'), description: t('intake.form.webdav.genericHint', 'Standard WebDAV protocol') },
        ]}
      />
      <TextField
        label={t('intake.form.webdav.serverUrl', 'Server URL')}
        value={form.server_url}
        onChange={(server_url) => set({ server_url })}
        placeholder={SERVER_URL_PLACEHOLDER[form.server_type]}
        type="url"
        isRequired
        {...err('server_url')}
      />
      <div className={styles.pair}>
        <TextField
          label={t('intake.form.webdav.username', 'Username')}
          value={form.username}
          onChange={(username) => set({ username })}
          autoComplete="username"
          isRequired
          {...err('username')}
        />
        <TextField
          label={t('intake.form.webdav.password', 'Password')}
          type="password"
          value={form.password}
          onChange={(password) => set({ password })}
          autoComplete="new-password"
          {...secretHints}
        />
      </div>
    </fieldset>
  );
}

export function LocalFolderFields({ form, set }: TypeFieldsProps) {
  const { t } = useTranslation();
  return (
    <fieldset className={styles.group}>
      <legend className={styles.legend}>{t('intake.form.local.title', 'Local folder')}</legend>
      <Notice>
        {t(
          'intake.form.local.hint',
          'Readur watches folders on the server it runs on. Make sure it can read the paths you add.',
        )}
      </Notice>
      <Switch
        label={t('intake.form.local.recursive', 'Scan subfolders')}
        description={t('intake.form.local.recursiveHint', 'Scan subdirectories recursively')}
        isSelected={form.recursive}
        onChange={(recursive) => set({ recursive })}
      />
      <Switch
        label={t('intake.form.local.symlinks', 'Follow symbolic links')}
        description={t('intake.form.local.symlinksHint', 'Follow symlinks when scanning directories')}
        isSelected={form.follow_symlinks}
        onChange={(follow_symlinks) => set({ follow_symlinks })}
      />
    </fieldset>
  );
}

export function S3Fields({ form, set, errors, stored }: TypeFieldsProps) {
  const { t } = useTranslation();
  const err = useFieldError(errors);
  const secretHints = useSecretHints(
    stored,
    Boolean(stored?.secretAccessKey),
    t('sources.form.secretStored', 'A secret is stored for this source'),
  );
  return (
    <fieldset className={styles.group}>
      <legend className={styles.legend}>{t('intake.form.s3.title', 'S3-compatible storage')}</legend>
      <Notice>
        {t(
          'intake.form.s3.hint',
          'Connect to AWS S3, MinIO or any S3-compatible service. For MinIO, give the endpoint URL of your server.',
        )}
      </Notice>
      <div className={styles.pair}>
        <TextField
          label={t('intake.form.s3.bucket', 'Bucket name')}
          value={form.bucket_name}
          onChange={(bucket_name) => set({ bucket_name })}
          placeholder="my-documents-bucket"
          isRequired
          {...err('bucket_name')}
        />
        <TextField
          label={t('intake.form.s3.region', 'Region')}
          value={form.region}
          onChange={(region) => set({ region })}
          placeholder="us-east-1"
        />
      </div>
      <div className={styles.pair}>
        <TextField
          label={t('intake.form.s3.accessKey', 'Access key ID')}
          value={form.access_key_id}
          onChange={(access_key_id) => set({ access_key_id })}
          autoComplete="off"
          isRequired
          {...err('access_key_id')}
        />
        <TextField
          label={t('intake.form.s3.secretKey', 'Secret access key')}
          type="password"
          value={form.secret_access_key}
          onChange={(secret_access_key) => set({ secret_access_key })}
          autoComplete="new-password"
          isRequired={!stored?.secretAccessKey}
          {...secretHints}
          {...err('secret_access_key')}
        />
      </div>
      <TextField
        label={t('intake.form.s3.endpoint', 'Endpoint URL (optional)')}
        value={form.endpoint_url}
        onChange={(endpoint_url) => set({ endpoint_url })}
        placeholder="https://minio.example.com"
        description={t(
          'intake.form.s3.endpointHint',
          'Leave empty for AWS S3, or give a custom endpoint for MinIO and other S3-compatible storage.',
        )}
      />
      <Select
        label={t('intake.form.s3.addressing', 'Addressing style')}
        selectedKey={form.force_path_style}
        onSelectionChange={(key) => set({ force_path_style: key as AddressingStyle })}
        description={t(
          'intake.form.s3.addressingHint',
          'MinIO, RustFS and most self-hosted S3 services need path-style. Auto-detect tries both.',
        )}
      >
        <SelectItem id="auto">{t('intake.form.s3.auto', 'Auto-detect')}</SelectItem>
        <SelectItem id="path">{t('intake.form.s3.path', 'Path-style (endpoint/bucket/key)')}</SelectItem>
        <SelectItem id="vhost">{t('intake.form.s3.vhost', 'Virtual-hosted (bucket.endpoint/key)')}</SelectItem>
      </Select>
      <TextField
        label={t('intake.form.s3.prefix', 'Object key prefix (optional)')}
        value={form.prefix}
        onChange={(prefix) => set({ prefix })}
        placeholder="documents/"
        description={t('intake.form.s3.prefixHint', 'Only scan object keys that start with this prefix.')}
      />
    </fieldset>
  );
}
