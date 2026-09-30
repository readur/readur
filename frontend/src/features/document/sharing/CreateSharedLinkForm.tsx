import { useState, type FormEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { sharedLinksService, type CreateSharedLinkRequest, type SharedLinkData } from '../../../services/api';
import { Button, IconButton, TextField, useToast } from '../../../ui';
import { ContentCopy, Visibility, VisibilityOff } from '../../../ui/icons';
import { apiErrorMessage } from '../format';
import styles from './Sharing.module.css';

export interface CreateSharedLinkFormProps {
  documentId: string;
  onCreated: (link: SharedLinkData) => void;
}

/** Creates a share link with an optional password, expiry and view limit, then shows its URL. */
export function CreateSharedLinkForm({ documentId, onCreated }: CreateSharedLinkFormProps) {
  const { t } = useTranslation();
  const toast = useToast();
  const [password, setPassword] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const [expiresAt, setExpiresAt] = useState('');
  const [maxViews, setMaxViews] = useState('');
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [created, setCreated] = useState<SharedLinkData | null>(null);

  const reset = () => {
    setPassword('');
    setExpiresAt('');
    setMaxViews('');
    setError(null);
    setCreated(null);
  };

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setPending(true);
    setError(null);
    const request: CreateSharedLinkRequest = { document_id: documentId };
    if (password.trim()) request.password = password.trim();
    if (expiresAt) request.expires_at = new Date(expiresAt).toISOString();
    const views = parseInt(maxViews, 10);
    if (views > 0) request.max_views = views;
    try {
      const res = await sharedLinksService.create(request);
      setCreated(res.data);
      onCreated(res.data);
    } catch (err) {
      setError(apiErrorMessage(err) ?? t('document.share.createFailed', "Couldn't create the link. Try again."));
    } finally {
      setPending(false);
    }
  };

  const copy = async () => {
    if (!created) return;
    try {
      await navigator.clipboard.writeText(created.url);
      toast.show({ title: t('document.share.copied', 'Link copied'), tone: 'success' });
    } catch {
      toast.show({ title: t('document.share.copyFailed', "Couldn't copy the link"), tone: 'danger' });
    }
  };

  if (created) {
    return (
      <div className={styles.created}>
        <p className={styles.hint}>{t('document.share.ready', 'Anyone with this link can open the document.')}</p>
        <div className={styles.urlRow}>
          <TextField
            aria-label={t('document.share.url', 'Share link')}
            value={created.url}
            isReadOnly
            className={styles.url}
          />
          <IconButton
            label={t('document.share.copy', 'Copy link')}
            icon={<ContentCopy fontSize="inherit" />}
            variant="secondary"
            onPress={copy}
          />
        </div>
        {created.has_password ? (
          <p className={styles.hint}>{t('document.share.passwordNote', 'People will need the password to open it.')}</p>
        ) : null}
        <Button size="sm" variant="ghost" onPress={reset}>
          {t('document.share.another', 'Create another link')}
        </Button>
      </div>
    );
  }

  return (
    <form className={styles.form} onSubmit={submit} aria-label={t('document.share.newLink', 'New share link')}>
      {error ? (
        <p className={styles.error} role="alert">
          {error}
        </p>
      ) : null}
      <div className={styles.passwordRow}>
        <TextField
          label={t('document.share.password', 'Password (optional)')}
          description={t('document.share.passwordHint', 'Leave empty for open access')}
          type={showPassword ? 'text' : 'password'}
          value={password}
          onChange={setPassword}
          isDisabled={pending}
          className={styles.grow}
        />
        <IconButton
          label={showPassword ? t('document.share.hidePassword', 'Hide password') : t('document.share.showPassword', 'Show password')}
          icon={showPassword ? <VisibilityOff fontSize="inherit" /> : <Visibility fontSize="inherit" />}
          onPress={() => setShowPassword((v) => !v)}
        />
      </div>
      <TextField
        label={t('document.share.expires', 'Expires (optional)')}
        description={t('document.share.expiresHint', 'Leave empty for no expiry')}
        type="datetime-local"
        value={expiresAt}
        onChange={setExpiresAt}
        isDisabled={pending}
      />
      <TextField
        label={t('document.share.maxViews', 'View limit (optional)')}
        description={t('document.share.maxViewsHint', 'Leave empty for unlimited views')}
        type="number"
        inputMode="numeric"
        value={maxViews}
        onChange={setMaxViews}
        isDisabled={pending}
      />
      <div className={styles.formActions}>
        <Button type="submit" variant="primary" isPending={pending}>
          {t('document.share.create', 'Create link')}
        </Button>
      </div>
    </form>
  );
}
