import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button, Dialog, TextField, useToast } from '../../../../ui';
import { sourcesService, type SourceResponse } from '../../../../services/api';
import type { SourceType } from '../../../../types/generated';
import { ErrorCodes, pickMessage } from '../../shared/errors';
import { Notice } from '../../shared/parts';
import { CommonFields } from './CommonFields';
import { CrawlEstimate } from './CrawlEstimate';
import { ChoiceField } from './fields';
import {
  buildConfig,
  buildTestRequest,
  canTestConnection,
  defaultFolders,
  emptyForm,
  formFromSource,
  storedSecrets,
  validateForm,
  type SourceFormData,
} from './sourceFormModel';
import { errorText, LocalFolderFields, S3Fields, WebDAVFields } from './typeFields';
import styles from './SourceForm.module.css';

export interface SourceFormProps {
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
  /** The connection to edit; omit to create one. */
  source?: SourceResponse | null;
  onSaved: (source?: SourceResponse) => void;
}

type TestResult = { ok: boolean; message: string } | null;

/** Create or edit a connection: type, per-type fields, folders, schedule; test before saving. */
export function SourceForm({ isOpen, onOpenChange, source, onSaved }: SourceFormProps) {
  const { t } = useTranslation();
  const toast = useToast();
  const isEditing = Boolean(source);
  const [form, setForm] = useState<SourceFormData>(() => (source ? formFromSource(source) : emptyForm()));
  const [showErrors, setShowErrors] = useState(false);
  const [testing, setTesting] = useState(false);
  const [saving, setSaving] = useState(false);
  const [testResult, setTestResult] = useState<TestResult>(null);

  // Reset whenever the dialog opens for a different connection.
  useEffect(() => {
    if (!isOpen) return;
    setForm(source ? formFromSource(source) : emptyForm());
    setShowErrors(false);
    setTestResult(null);
    // Keyed on the id: a background refresh of the list must not wipe edits in progress.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isOpen, source?.id]);

  // Secrets the server already holds for this connection: a blank field keeps them.
  const stored = useMemo(
    () => storedSecrets(source && source.source_type === form.source_type ? source : null),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [source?.id, source?.config, form.source_type],
  );
  const errors = useMemo(() => validateForm(form, stored), [form, stored]);
  const shownErrors = showErrors ? errors : {};

  const set = (patch: Partial<SourceFormData>) => {
    setForm((prev) => ({ ...prev, ...patch }));
    setTestResult(null);
  };

  const changeType = (source_type: SourceType) => {
    // New connections get the default folders of the chosen type.
    set({ source_type, watch_folders: defaultFolders(source_type) });
  };

  const testConnection = async () => {
    setTesting(true);
    setTestResult(null);
    try {
      const res = await sourcesService.testConnection(buildTestRequest(form, source));
      if (res.data?.success) {
        setTestResult({ ok: true, message: res.data.message || t('intake.form.test.ok', 'Connection successful') });
      } else {
        setTestResult({ ok: false, message: res.data?.message || t('intake.form.test.failed', 'Connection failed') });
      }
    } catch (error) {
      setTestResult({
        ok: false,
        message: pickMessage(
          error,
          [
            [ErrorCodes.SOURCE_CONNECTION_FAILED, t('intake.form.test.unreachable', 'Could not reach the server. Check the URL and your network.')],
            [ErrorCodes.SOURCE_AUTH_FAILED, t('intake.form.test.auth', 'Sign-in failed. Check the username and password.')],
            [ErrorCodes.SOURCE_INVALID_PATH, t('intake.form.test.path', 'A folder path is invalid or not accessible.')],
            [ErrorCodes.SOURCE_CONFIG_INVALID, t('intake.form.test.config', 'Some settings are invalid. Check the values and try again.')],
            [ErrorCodes.SOURCE_NETWORK_TIMEOUT, t('intake.form.test.timeout', 'The connection timed out. The server may be slow or unreachable.')],
          ],
          t('intake.form.test.error', 'Could not test the connection'),
        ),
      });
    } finally {
      setTesting(false);
    }
  };

  const save = async () => {
    setShowErrors(true);
    if (Object.keys(errors).length > 0) return;
    setSaving(true);
    try {
      const config = buildConfig(form, { omitBlankSecrets: isEditing });
      const res = source
        ? await sourcesService.update(source.id, { name: form.name.trim(), enabled: form.enabled, config })
        : await sourcesService.create({ name: form.name.trim(), source_type: form.source_type, enabled: form.enabled, config });
      toast.show({
        title: source ? t('intake.form.saved', 'Connection updated') : t('intake.form.created', 'Connection added'),
        tone: 'success',
      });
      onSaved(res?.data);
      onOpenChange(false);
    } catch (error) {
      toast.show({
        title: t('intake.form.saveFailed', 'Could not save the connection'),
        description: pickMessage(
          error,
          [
            [ErrorCodes.SOURCE_DUPLICATE_NAME, t('intake.form.errors.duplicateName', 'A connection with this name already exists.')],
            [ErrorCodes.SOURCE_CONFIG_INVALID, t('intake.form.test.config', 'Some settings are invalid. Check the values and try again.')],
            [ErrorCodes.SOURCE_AUTH_FAILED, t('intake.form.test.auth', 'Sign-in failed. Check the username and password.')],
            [ErrorCodes.SOURCE_CONNECTION_FAILED, t('intake.form.test.unreachable', 'Could not reach the server. Check the URL and your network.')],
            [ErrorCodes.SOURCE_INVALID_PATH, t('intake.form.test.path', 'A folder path is invalid or not accessible.')],
          ],
          t('intake.form.saveFailedHint', 'Check the settings and try again.'),
        ),
        tone: 'danger',
      });
    } finally {
      setSaving(false);
    }
  };

  const errorCount = Object.keys(shownErrors).length;
  const TypeFields = form.source_type === 'webdav' ? WebDAVFields : form.source_type === 's3' ? S3Fields : LocalFolderFields;

  return (
    <Dialog
      isOpen={isOpen}
      onOpenChange={onOpenChange}
      size="lg"
      isDismissable={false}
      title={isEditing ? t('intake.form.editTitle', 'Edit connection') : t('intake.form.createTitle', 'Add connection')}
      actions={
        <>
          <Button variant="ghost" onPress={() => onOpenChange(false)} isDisabled={saving}>
            {t('intake.actions.cancel', 'Cancel')}
          </Button>
          <Button onPress={testConnection} isPending={testing} isDisabled={!canTestConnection(form, stored)}>
            {t('intake.form.test.button', 'Test connection')}
          </Button>
          <Button variant="primary" onPress={save} isPending={saving}>
            {isEditing ? t('intake.form.saveChanges', 'Save changes') : t('intake.form.create', 'Add connection')}
          </Button>
        </>
      }
    >
      <div className={styles.form}>
        {errorCount > 0 ? (
          <Notice tone="danger" live="alert" title={t('intake.form.errors.summary', 'Fix the highlighted fields before saving.')} />
        ) : null}
        <TextField
          label={t('intake.form.name', 'Name')}
          value={form.name}
          onChange={(name) => set({ name })}
          placeholder={t('intake.form.namePlaceholder', 'My document server')}
          isRequired
          autoFocus
          isInvalid={Boolean(shownErrors.name)}
          errorMessage={errorText(t, shownErrors.name)}
        />
        {!isEditing ? (
          <ChoiceField<SourceType>
            label={t('intake.form.type', 'Connection type')}
            value={form.source_type}
            onChange={changeType}
            options={[
              { value: 'webdav', label: 'WebDAV', description: t('intake.form.typeWebdav', 'Nextcloud, ownCloud and other WebDAV servers') },
              { value: 'local_folder', label: t('intake.sourceType.local_folder', 'Local folder'), description: t('intake.form.typeLocal', 'Folders on the Readur server') },
              { value: 's3', label: t('intake.form.typeS3Label', 'S3-compatible'), description: t('intake.form.typeS3', 'AWS S3, MinIO and other S3-compatible storage') },
            ]}
          />
        ) : null}
        <TypeFields form={form} set={set} errors={shownErrors} stored={isEditing ? stored : undefined} />
        <CommonFields form={form} set={set} errors={shownErrors} />
        {source && form.source_type === 'webdav' && form.server_url && form.username && form.watch_folders.length > 0 ? (
          <CrawlEstimate sourceId={source.id} />
        ) : null}
        {testResult ? (
          <Notice tone={testResult.ok ? 'ok' : 'danger'} live={testResult.ok ? 'status' : 'alert'} title={testResult.message} />
        ) : null}
      </div>
    </Dialog>
  );
}
