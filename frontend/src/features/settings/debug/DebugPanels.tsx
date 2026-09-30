import { useEffect, useMemo, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Button, TextField } from '../../../ui';
import { BugReport, CloudUpload, Refresh, Search, Visibility } from '../../../ui/icons';
import { Notice } from '../shared/Notice';
import shared from '../shared/shared.module.css';
import type { DebugSession } from './useDebugSession';
import styles from './Debug.module.css';

const ACCEPT = '.pdf,.png,.jpg,.jpeg,.tiff,.bmp,.txt';

/** Upload a file and follow it through OCR. */
export function UploadPanel({ session: s, onShowResults }: { session: DebugSession; onShowResults: () => void }) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const inputRef = useRef<HTMLInputElement>(null);
  const failed = s.processingStatus.toLowerCase().includes('failed');
  const previewUrl = useMemo(
    () => (s.selectedFile?.type.startsWith('image/') && typeof URL.createObjectURL === 'function' ? URL.createObjectURL(s.selectedFile) : null),
    [s.selectedFile],
  );
  useEffect(() => () => {
    if (previewUrl && typeof URL.revokeObjectURL === 'function') URL.revokeObjectURL(previewUrl);
  }, [previewUrl]);

  return (
    <div className={shared.stack}>
      <div>
        <h3 className={styles.panelTitle}>{t('debug.upload.title')}</h3>
        <p className={styles.panelIntro}>{t('debug.upload.description')}</p>
      </div>
      <input
        ref={inputRef}
        type="file"
        accept={ACCEPT}
        className="visually-hidden"
        tabIndex={-1}
        aria-hidden="true"
        onChange={(e) => s.selectFile(e.target.files?.[0] ?? null)}
        data-testid="debug-file-input"
      />
      <div className={shared.row}>
        <Button icon={<CloudUpload fontSize="inherit" />} onPress={() => inputRef.current?.click()} isDisabled={s.uploading}>
          {t('debug.upload.selectFileButton')}
        </Button>
        {s.selectedFile ? (
          <Button variant="primary" icon={<CloudUpload fontSize="inherit" />} onPress={() => void s.upload()} isPending={s.uploading}>
            {s.uploading ? t('debug.upload.uploadingButton') : t('debug.upload.uploadDebugButton')}
          </Button>
        ) : null}
      </div>
      {s.selectedFile ? (
        <p className={shared.meta}>
          <strong>{t('debug.upload.selected')}</strong> {s.selectedFile.name}{' '}
          <span className={shared.mono}>({(s.selectedFile.size / 1024 / 1024).toFixed(2)} MB)</span>
        </p>
      ) : null}
      {s.uploading && s.uploadProgress > 0 ? (
        <div>
          <p className={shared.meta}>{t('debug.upload.uploadProgress', { percent: s.uploadProgress })}</p>
          <progress className={styles.progress} max={100} value={s.uploadProgress} aria-label={t('debug.upload.uploadingButton')} />
        </div>
      ) : null}
      {s.processingStatus ? (
        <Notice tone={failed ? 'danger' : 'info'}>
          {s.processingStatus}
          {s.monitoring ? <progress className={styles.progress} aria-label={s.processingStatus} /> : null}
        </Notice>
      ) : null}
      {s.uploadedDocumentId ? (
        <div className={shared.stack}>
          <p className={shared.meta}>
            <strong>{t('debug.upload.documentId')}</strong> <span className={shared.mono}>{s.uploadedDocumentId}</span>
          </p>
          <div className={shared.row}>
            <Button
              size="sm"
              variant={failed ? 'danger' : 'primary'}
              icon={<BugReport fontSize="inherit" />}
              onPress={() => {
                void s.fetchDebugInfo(s.uploadedDocumentId);
                onShowResults();
              }}
            >
              {failed ? t('debug.actions.showDebugDetails') : t('debug.actions.debugAnalysis')}
            </Button>
            <Button size="sm" icon={<Refresh fontSize="inherit" />} onPress={() => void s.fetchDebugInfo(s.uploadedDocumentId)}>
              {t('debug.actions.refreshStatus')}
            </Button>
            <Button
              size="sm"
              icon={<Visibility fontSize="inherit" />}
              onPress={() => navigate(`/documents/${s.uploadedDocumentId}`)}
            >
              {t('debug.actions.viewDocument')}
            </Button>
          </div>
        </div>
      ) : null}
      {previewUrl ? (
        <figure className={styles.figure}>
          <figcaption className={styles.panelTitle}>{t('debug.preview')}</figcaption>
          <img src={previewUrl} alt={s.selectedFile?.name ?? ''} className={styles.previewImage} />
        </figure>
      ) : null}
    </div>
  );
}

/** Look up diagnostics for an existing document by id. */
export function SearchPanel({ session: s }: { session: DebugSession }) {
  const { t } = useTranslation();
  return (
    <form
      className={shared.stack}
      onSubmit={(e) => {
        e.preventDefault();
        if (s.documentId.trim()) void s.fetchDebugInfo();
      }}
    >
      <div>
        <h3 className={styles.panelTitle}>{t('debug.search.title')}</h3>
        <p className={styles.panelIntro}>{t('debug.search.description')}</p>
      </div>
      <div className={styles.searchRow}>
        <TextField
          label={t('debug.search.documentIdLabel')}
          placeholder={t('debug.search.documentIdPlaceholder')}
          value={s.documentId}
          onChange={s.setDocumentId}
          className={styles.searchField}
        />
        <Button
          type="submit"
          variant="primary"
          icon={<Search fontSize="inherit" />}
          isDisabled={!s.documentId.trim()}
          isPending={s.loading}
        >
          {t('debug.search.debugButton')}
        </Button>
      </div>
    </form>
  );
}
