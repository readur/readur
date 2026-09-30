import { useId } from 'react';
import { useDropzone } from 'react-dropzone';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { BoardTable, Button, EmptyState, StatusMark, type BoardColumn, type StatusState } from '../../../ui';
import LabelSelector from '../../labels/LabelSelector';
import LanguageSelector from '../../../components/LanguageSelector';
import { isLit, useLitCount } from '../../board/litStore';
import { formatBytes } from '../shared/format';
import { NameCell, Notice, ProgressCell, sharedStyles } from '../shared/parts';
import { ACCEPT, ACCEPTED_EXTENSIONS, MAX_FILE_SIZE, MAX_FILE_SIZE_MB } from './uploadConfig';
import { useUploadOptions } from './useUploadOptions';
import { useUploadQueue, type UploadItem, type UploadStatus } from './useUploadQueue';
import styles from './Upload.module.css';

const STATE: Record<UploadStatus, StatusState> = {
  pending: 'pending',
  uploading: 'syncing',
  // Uploaded files go straight into the OCR queue.
  success: 'processing',
  error: 'failed',
};

/** Drop area, upload options and the board of files being added. */
export function UploadSection() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const optionsId = useId();
  const options = useUploadOptions();
  useLitCount('document');
  const queue = useUploadQueue(() => ({
    labelIds: options.selectedLabels.map((l) => l.id),
    languages: options.languages,
  }));

  const { getRootProps, getInputProps, isDragActive, open } = useDropzone({
    onDrop: queue.add,
    accept: ACCEPT,
    maxSize: MAX_FILE_SIZE,
    multiple: true,
    noClick: true,
    noKeyboard: true,
  });

  const waiting = queue.items.filter((i) => i.status === 'pending' || i.status === 'error').length;
  const done = queue.items.filter((i) => i.status === 'success').length;

  const columns: BoardColumn<UploadItem>[] = [
    {
      id: 'name',
      label: t('intake.upload.col.name', 'Name'),
      render: (i) => (
        <NameCell
          name={i.file.name}
          tag={i.status === 'success' && i.documentId && isLit('document', i.documentId) ? 'new' : null}
        />
      ),
    },
    { id: 'size', label: t('intake.upload.col.size', 'Size'), align: 'end', width: 100, render: (i) => formatBytes(i.file.size) },
    {
      id: 'progress',
      label: t('intake.upload.col.progress', 'Progress'),
      width: 180,
      render: (i) => (
        <ProgressCell value={i.progress} label={t('intake.upload.progressOf', 'Upload progress for {{name}}', { name: i.file.name })} />
      ),
    },
    { id: 'status', label: t('intake.upload.col.status', 'Status'), width: 120, render: (i) => <StatusMark state={STATE[i.status]} size="sm" /> },
    {
      id: 'actions',
      label: <span className={sharedStyles.visuallyHidden}>{t('intake.upload.col.actions', 'Actions')}</span>,
      textValue: t('intake.upload.col.actions', 'Actions'),
      width: 180,
      align: 'end',
      render: (i) => (
        <span className={sharedStyles.rowActions}>
          {i.status === 'error' ? (
            <Button size="sm" onPress={() => queue.retry(i.id)} aria-label={t('intake.upload.retryFile', 'Retry {{name}}', { name: i.file.name })}>
              {t('intake.actions.retry', 'Retry')}
            </Button>
          ) : null}
          {i.status === 'pending' || i.status === 'error' ? (
            <Button size="sm" variant="ghost" onPress={() => queue.remove(i.id)} aria-label={t('intake.upload.removeFile', 'Remove {{name}}', { name: i.file.name })}>
              {t('intake.upload.remove', 'Remove')}
            </Button>
          ) : null}
        </span>
      ),
    },
  ];

  return (
    <div className={sharedStyles.section}>
      <div
        {...getRootProps({ className: styles.drop })}
        data-dragging={isDragActive ? 'true' : undefined}
        role="group"
        aria-label={t('intake.upload.dropLabel', 'Drop files to add')}
      >
        <input {...getInputProps({ 'aria-label': t('intake.upload.chooseFiles', 'Choose files') })} />
        <p className={styles.dropTitle}>
          {isDragActive ? t('intake.upload.dropNow', 'Release to add') : t('intake.upload.dropTitle', 'Drop files to add')}
        </p>
        <Button variant="primary" onPress={open}>
          {t('intake.upload.chooseFiles', 'Choose files')}
        </Button>
        <p className={styles.accepted}>
          {ACCEPTED_EXTENSIONS.join(' · ')}
          <span aria-hidden="true"> · </span>
          <span>{t('intake.upload.maxSize', 'max {{size}} MB per file', { size: MAX_FILE_SIZE_MB })}</span>
        </p>
      </div>

      {queue.rejected ? (
        <Notice tone="danger" live="alert" title={t('intake.upload.rejected', 'Some files were not added')}>
          {queue.rejected}
        </Notice>
      ) : null}

      <section className={styles.options} aria-labelledby={optionsId}>
        <h2 id={optionsId} className={sharedStyles.heading}>
          {t('intake.upload.options', 'Apply to these uploads')}
        </h2>
        <div className={styles.optionGrid}>
          <div className={sharedStyles.stack}>
            <span className={sharedStyles.heading}>{t('intake.upload.languages', 'OCR languages')}</span>
            <LanguageSelector
              selectedLanguages={options.languages}
              primaryLanguage={options.primaryLanguage}
              onLanguagesChange={options.changeLanguages}
              disabled={queue.isUploading}
            />
          </div>
          <div className={sharedStyles.stack}>
            <span className={sharedStyles.heading}>{t('intake.upload.labels', 'Labels')}</span>
            <LabelSelector
              selectedLabels={options.selectedLabels}
              availableLabels={options.availableLabels}
              onLabelsChange={options.setSelectedLabels}
              onCreateLabel={options.createLabel}
              placeholder={t('intake.upload.labelsPlaceholder', 'Add labels to every uploaded file')}
              size="medium"
              disabled={options.labelsLoading}
            />
          </div>
        </div>
      </section>

      <div className={sharedStyles.toolbar}>
        <Button variant="primary" onPress={() => void queue.uploadAll()} isDisabled={waiting === 0 || queue.isUploading}>
          {queue.isUploading
            ? t('intake.upload.uploading', 'Uploading {{done}} of {{total}}', { done: queue.batch.done, total: queue.batch.total })
            : t('intake.upload.uploadAll', 'Upload all ({{count}})', { count: waiting })}
        </Button>
        <Button variant="ghost" onPress={queue.clearCompleted} isDisabled={done === 0}>
          {t('intake.upload.clearDone', 'Clear finished')}
        </Button>
      </div>

      <BoardTable
        aria-label={t('intake.upload.boardLabel', 'Files to upload')}
        columns={columns}
        rows={queue.items}
        getRowId={(i) => i.id}
        isRowLit={(i) => i.status === 'success' && Boolean(i.documentId) && isLit('document', i.documentId!)}
        onRowAction={(id) => {
          const item = queue.items.find((i) => i.id === id);
          if (item?.status === 'success' && item.documentId) navigate(`/documents/${item.documentId}`);
        }}
        renderRowDetail={(i) => (i.error ? <span className={sharedStyles.dangerText}>{i.error}</span> : null)}
        emptyState={
          <EmptyState
            headingAs="h3"
            title={t('intake.upload.emptyTitle', 'No files yet')}
            description={t('intake.upload.emptyBody', 'Drop files above or choose them from your computer.')}
          />
        }
      />
    </div>
  );
}
