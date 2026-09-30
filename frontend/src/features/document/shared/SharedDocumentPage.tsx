import { useEffect, useState, type FormEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { useParams } from 'react-router-dom';
import { sharedLinksPublicService, type SharedDocumentMetadata } from '../../../services/api';
import { Button, EmptyState, Pass, PassCell, Skeleton, TextField } from '../../../ui';
import { Download, Lock, Visibility } from '../../../ui/icons';
import { filenameFromDisposition, saveBlob } from '../download';
import { apiErrorMessage, fileTypeCode, formatBytes, formatDateTime, httpStatus } from '../format';
import { DocumentViewer } from '../reading/DocumentViewer';
import styles from './SharedDocumentPage.module.css';

type ErrorKind = 'missing' | 'expired' | 'invalid' | 'other';
type PageState =
  | { status: 'loading' }
  | { status: 'password' }
  | { status: 'ready' }
  | { status: 'error'; kind: ErrorKind; message?: string };

/** The metadata endpoint may add an expiry; show it when it does. */
type Metadata = SharedDocumentMetadata & { expires_at?: string | null };

function errorKind(err: unknown): ErrorKind {
  const status = httpStatus(err);
  if (status === 404) return 'missing';
  if (status === 410) return 'expired';
  return 'other';
}

/** Public page for a share link: no app chrome, optional password, preview and download. */
export function SharedDocumentPage() {
  const { t, i18n } = useTranslation();
  const { token } = useParams<{ token: string }>();
  const [state, setState] = useState<PageState>({ status: 'loading' });
  const [meta, setMeta] = useState<Metadata | null>(null);
  const [password, setPassword] = useState('');
  const [verified, setVerified] = useState<string | undefined>(undefined);
  const [verifying, setVerifying] = useState(false);
  const [passwordError, setPasswordError] = useState<string | null>(null);
  const [downloading, setDownloading] = useState(false);
  const [downloadError, setDownloadError] = useState<string | null>(null);
  const [showPreview, setShowPreview] = useState(false);

  useEffect(() => {
    if (!token) {
      setState({ status: 'error', kind: 'invalid' });
      return;
    }
    let cancelled = false;
    setState({ status: 'loading' });
    sharedLinksPublicService
      .getMetadata(token)
      .then((res) => {
        if (cancelled) return;
        setMeta(res.data);
        setState({ status: res.data.requires_password ? 'password' : 'ready' });
      })
      .catch((err) => {
        if (!cancelled) setState({ status: 'error', kind: errorKind(err), message: apiErrorMessage(err) ?? undefined });
      });
    return () => {
      cancelled = true;
    };
  }, [token]);

  const submitPassword = async (e: FormEvent) => {
    e.preventDefault();
    const value = password.trim();
    if (!token || !value) return;
    setVerifying(true);
    setPasswordError(null);
    try {
      const res = await sharedLinksPublicService.verifyPassword(token, value);
      if (res.data.valid) {
        setVerified(value);
        setState({ status: 'ready' });
      } else {
        setPasswordError(t('document.shared.wrongPassword', 'That password is not right. Try again.'));
      }
    } catch (err) {
      if (httpStatus(err) === 410) {
        setState({ status: 'error', kind: 'expired' });
      } else {
        setPasswordError(apiErrorMessage(err) ?? t('document.shared.verifyFailed', "Couldn't check the password. Try again."));
      }
    } finally {
      setVerifying(false);
    }
  };

  const download = async () => {
    if (!token) return;
    setDownloading(true);
    setDownloadError(null);
    try {
      const res = await sharedLinksPublicService.downloadDocument(token, verified);
      const name = filenameFromDisposition(res.headers?.['content-disposition']) ?? meta?.original_filename ?? 'download';
      saveBlob(res.data, name);
    } catch (err) {
      if (httpStatus(err) === 410) setState({ status: 'error', kind: 'expired' });
      else setDownloadError(t('document.shared.downloadFailed', "Couldn't download the file. Try again."));
    } finally {
      setDownloading(false);
    }
  };

  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <span className={styles.wordmark}>
          <img src="/readur-64.png" alt="" width={20} height={20} />
          READUR
        </span>
        <span className={styles.kicker}>{t('document.shared.kicker', 'Shared document')}</span>
      </header>

      <main className={styles.main}>
        {state.status === 'loading' ? (
          <div className={styles.card} role="status" aria-label={t('document.shared.loading', 'Loading shared document')}>
            <Skeleton width="60%" height={28} />
            <Skeleton height={72} />
          </div>
        ) : null}

        {state.status === 'error' ? (
          <div className={styles.card}>
            <EmptyState
              headingAs="h1"
              title={
                state.kind === 'expired'
                  ? t('document.shared.expiredTitle', 'This link has expired')
                  : t('document.shared.unavailableTitle', 'This link is not available')
              }
              description={
                state.kind === 'expired'
                  ? t('document.shared.expired', 'It has passed its expiry date, reached its view limit or was revoked. Ask the sender for a new link.')
                  : state.kind === 'missing'
                    ? t('document.shared.missing', 'It does not exist or has been removed.')
                    : state.kind === 'invalid'
                      ? t('document.shared.invalid', 'The link is incomplete.')
                      : state.message || t('document.shared.error', "The shared document couldn't be loaded. Try again later.")
              }
            />
          </div>
        ) : null}

        {state.status === 'password' ? (
          <form className={styles.card} onSubmit={submitPassword}>
            <span className={styles.lock} aria-hidden="true">
              <Lock fontSize="inherit" />
            </span>
            <h1 className={styles.title}>{t('document.shared.passwordTitle', 'Password required')}</h1>
            <p className={styles.hint}>
              {t('document.shared.passwordHint', 'This document is protected. Enter the password you were given.')}
            </p>
            <TextField
              label={t('document.shared.password', 'Password')}
              type="password"
              autoFocus
              value={password}
              onChange={(v) => {
                setPassword(v);
                setPasswordError(null);
              }}
              isDisabled={verifying}
              isInvalid={Boolean(passwordError)}
              errorMessage={passwordError ?? undefined}
            />
            <Button type="submit" variant="primary" isPending={verifying} isDisabled={!password.trim()}>
              {t('document.shared.submit', 'Open document')}
            </Button>
          </form>
        ) : null}

        {state.status === 'ready' && meta && token ? (
          <div className={styles.ready}>
            <h1 className={styles.title}>{meta.original_filename}</h1>
            <Pass aria-label={t('document.pass.label', 'Document summary')}>
              <PassCell label={t('document.shared.file', 'File')} span={2}>{meta.original_filename}</PassCell>
              <PassCell label={t('document.pass.type', 'Type')} mono>{fileTypeCode(meta.mime_type) ?? meta.mime_type}</PassCell>
              <PassCell label={t('document.pass.size', 'Size')} mono>{formatBytes(meta.file_size)}</PassCell>
              {meta.expires_at !== undefined ? (
                <PassCell label={t('document.shared.expires', 'Expires')} mono>
                  {meta.expires_at ? formatDateTime(meta.expires_at, i18n.language) : t('document.share.never', 'Never')}
                </PassCell>
              ) : null}
            </Pass>
            {downloadError ? (
              <p className={styles.error} role="alert">
                {downloadError}
              </p>
            ) : null}
            <div className={styles.actions}>
              <Button variant="primary" icon={<Download fontSize="inherit" />} isPending={downloading} onPress={download}>
                {t('document.actions.download', 'Download')}
              </Button>
              {!showPreview ? (
                <Button icon={<Visibility fontSize="inherit" />} onPress={() => setShowPreview(true)}>
                  {t('document.shared.preview', 'Show preview')}
                </Button>
              ) : null}
            </div>
            {showPreview ? (
              <DocumentViewer
                documentId={token}
                filename={meta.original_filename}
                mimeType={meta.mime_type}
                load={() => sharedLinksPublicService.viewDocument(token, verified)}
              />
            ) : (
              <p className={styles.hint}>
                {t('document.shared.previewHint', 'Opening the preview counts as a view of this link.')}
              </p>
            )}
          </div>
        ) : null}
      </main>
    </div>
  );
}

export default SharedDocumentPage;
