import { useTranslation } from 'react-i18next';
import { Switch } from '../../../../ui';
import { IntervalField, ListField } from './fields';
import { errorText, type TypeFieldsProps } from './typeFields';
import { MAX_INTERVAL, MIN_INTERVAL, normalizeExtension, prefersAbsolutePath } from './sourceFormModel';
import { UrlPreview } from './UrlPreview';
import styles from './SourceForm.module.css';

/** Folders, file types, schedule and the enabled switch: shared by every connection type. */
export function CommonFields({ form, set, errors }: TypeFieldsProps) {
  const { t } = useTranslation();
  const type = form.source_type;

  const folderCopy =
    type === 's3'
      ? {
          label: t('intake.form.folders.s3', 'Object prefixes to monitor'),
          hint: t('intake.form.folders.s3Hint', 'Object prefixes (like folders) to scan for files.'),
          placeholder: 'documents/',
        }
      : type === 'local_folder'
        ? {
            label: t('intake.form.folders.local', 'Directories to monitor'),
            hint: t('intake.form.folders.localHint', 'Local directories to scan. Use absolute paths.'),
            placeholder: '/home/user/Documents',
          }
        : {
            label: t('intake.form.folders.webdav', 'Folders to monitor'),
            hint: t('intake.form.folders.webdavHint', 'Folders to scan for files. Use absolute paths starting with “/”.'),
            placeholder: '/Documents',
          };

  const autoHint =
    type === 's3'
      ? t('intake.form.schedule.autoHintS3', 'Check for new objects on a schedule')
      : type === 'local_folder'
        ? t('intake.form.schedule.autoHintLocal', 'Scan for new files on a schedule')
        : t('intake.form.schedule.autoHint', 'Sync files on a schedule');

  return (
    <>
      <fieldset className={styles.group}>
        <legend className={styles.legend}>{t('intake.form.folders.title', 'What to watch')}</legend>
        <ListField
          label={folderCopy.label}
          description={folderCopy.hint}
          placeholder={folderCopy.placeholder}
          items={form.watch_folders}
          onChange={(watch_folders) => set({ watch_folders })}
          advise={(value) =>
            prefersAbsolutePath(type) && !value.startsWith('/')
              ? t('intake.form.relativeWarning', '“{{value}}” is a relative path. Absolute paths starting with “/” are recommended.', { value })
              : null
          }
          listError={errorText(t, errors.watch_folders)}
        />
        <UrlPreview form={form} />
        <ListField
          label={t('intake.form.extensions.label', 'File extensions')}
          description={
            type === 'local_folder'
              ? t('intake.form.extensions.hintLocal', 'File types to monitor and process with OCR.')
              : t('intake.form.extensions.hint', 'File types to sync and process with OCR.')
          }
          placeholder="docx"
          items={form.file_extensions}
          onChange={(file_extensions) => set({ file_extensions })}
          normalize={normalizeExtension}
        />
      </fieldset>

      <fieldset className={styles.group}>
        <legend className={styles.legend}>{t('intake.form.schedule.title', 'Schedule')}</legend>
        <Switch
          label={t('intake.form.schedule.auto', 'Automatic sync')}
          description={autoHint}
          isSelected={form.auto_sync}
          onChange={(auto_sync) => set({ auto_sync })}
        />
        {form.auto_sync ? (
          <IntervalField
            label={t('intake.form.schedule.interval', 'Sync interval (minutes)')}
            description={t('intake.form.schedule.intervalHint', 'How often to check for new files ({{min}} min to 24 hours)', {
              min: MIN_INTERVAL,
              max: MAX_INTERVAL,
            })}
            value={form.sync_interval_minutes}
            onChange={(sync_interval_minutes) => set({ sync_interval_minutes })}
            errorMessage={errorText(t, errors.sync_interval_minutes)}
          />
        ) : null}
        <Switch
          label={t('intake.form.enabled', 'Connection enabled')}
          description={t('intake.form.enabledHint', 'Include this connection when syncing')}
          isSelected={form.enabled}
          onChange={(enabled) => set({ enabled })}
        />
      </fieldset>
    </>
  );
}
