import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { Key } from 'react-aria-components';
import { Button, Dialog, IconButton, Select, SelectItem, TextField } from '../../../ui';
import { Check, ContentCopy } from '../../../ui/icons';
import type { ApiKey } from '../../../services/api';
import { Notice } from '../shared/Notice';
import formStyles from '../form/form.module.css';
import styles from './ApiKeys.module.css';
import type { ExpirationOption } from './useApiKeys';

const EXPIRATIONS: { id: ExpirationOption; label: string; fallback: string }[] = [
  { id: '7', label: 'settings.apiKeys.expiry.7', fallback: '7 days' },
  { id: '30', label: 'settings.apiKeys.expiry.30', fallback: '30 days' },
  { id: '90', label: 'settings.apiKeys.expiry.90', fallback: '90 days' },
  { id: '180', label: 'settings.apiKeys.expiry.180', fallback: '180 days' },
  { id: '365', label: 'settings.apiKeys.expiry.365', fallback: '1 year' },
  { id: 'never', label: 'settings.apiKeys.expiry.never', fallback: 'Never (not recommended)' },
];

export function CreateKeyDialog({
  isOpen,
  onClose,
  onCreate,
}: {
  isOpen: boolean;
  onClose: () => void;
  onCreate: (name: string, expiration: ExpirationOption) => Promise<void>;
}) {
  const { t } = useTranslation();
  const [name, setName] = useState('');
  const [expiration, setExpiration] = useState<ExpirationOption>('90');
  const [nameError, setNameError] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);

  useEffect(() => {
    if (!isOpen) return;
    setName('');
    setExpiration('90');
    setNameError(null);
    setError(null);
  }, [isOpen]);

  const submit = async () => {
    const trimmed = name.trim();
    if (!trimmed) {
      setNameError(t('settings.apiKeys.nameRequired', 'Please give this key a name.'));
      return;
    }
    setCreating(true);
    setError(null);
    try {
      await onCreate(trimmed, expiration);
    } catch (err) {
      setError(err instanceof Error ? err.message : t('settings.apiKeys.createFailed', 'Failed to create API key.'));
    } finally {
      setCreating(false);
    }
  };

  return (
    <Dialog
      isOpen={isOpen}
      size="sm"
      onOpenChange={(open) => !open && !creating && onClose()}
      title={t('settings.apiKeys.createTitle', 'Create API key')}
      actions={
        <>
          <Button variant="ghost" onPress={onClose} isDisabled={creating}>
            {t('common.actions.cancel')}
          </Button>
          <Button variant="primary" onPress={() => void submit()} isPending={creating}>
            {t('common.actions.create')}
          </Button>
        </>
      }
    >
      <div className={formStyles.form}>
        <TextField
          label={t('settings.apiKeys.name', 'Name')}
          description={t('settings.apiKeys.nameHelp', "A label you'll recognize later.")}
          placeholder={t('settings.apiKeys.namePlaceholder', 'e.g. backup-script')}
          value={name}
          onChange={(v) => {
            setName(v);
            if (nameError) setNameError(null);
          }}
          maxLength={100}
          isRequired
          isInvalid={Boolean(nameError)}
          errorMessage={nameError}
          validationBehavior="aria"
          autoFocus
        />
        <Select
          label={t('settings.apiKeys.expiration', 'Expiration')}
          selectedKey={expiration}
          onSelectionChange={(k: Key | null) => k !== null && setExpiration(String(k) as ExpirationOption)}
        >
          {EXPIRATIONS.map((o) => (
            <SelectItem key={o.id} id={o.id} textValue={t(o.label, o.fallback)}>
              {t(o.label, o.fallback)}
            </SelectItem>
          ))}
        </Select>
        {error ? <Notice tone="danger">{error}</Notice> : null}
      </div>
    </Dialog>
  );
}

/**
 * Shows the new key exactly once. Only "I've saved it" closes it: Esc and outside clicks are
 * ignored because this is the last chance to copy the key.
 */
export function RevealKeyDialog({ revealed, onDone }: { revealed: { name: string; plaintext: string } | null; onDone: () => void }) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);
  const [copyError, setCopyError] = useState(false);

  useEffect(() => {
    setCopied(false);
    setCopyError(false);
  }, [revealed]);

  const copy = async () => {
    if (!revealed) return;
    try {
      await navigator.clipboard.writeText(revealed.plaintext);
      setCopied(true);
      setCopyError(false);
    } catch {
      setCopyError(true);
    }
  };

  return (
    <Dialog
      isOpen={revealed !== null}
      isDismissable={false}
      onOpenChange={() => undefined}
      title={t('settings.apiKeys.revealTitle', 'Your new API key')}
      actions={
        <Button variant="primary" onPress={onDone}>
          {t('settings.apiKeys.saved', "I've saved it")}
        </Button>
      }
    >
      <div className={formStyles.form}>
        <Notice tone="warning">
          {t(
            'settings.apiKeys.revealWarning',
            "Copy this key now. For your security it will not be shown again — if you lose it you'll need to create a new one.",
          )}
        </Notice>
        {revealed ? (
          <>
            <p className={formStyles.help}>
              {t('settings.apiKeys.keyName', 'Key name:')} <strong>{revealed.name}</strong>
            </p>
            <div className={styles.secret}>
              <code className={styles.secretValue}>{revealed.plaintext}</code>
              <IconButton
                label={copied ? t('settings.apiKeys.copied', 'Copied') : t('settings.apiKeys.copy', 'Copy API key')}
                icon={copied ? <Check fontSize="inherit" /> : <ContentCopy fontSize="inherit" />}
                onPress={() => void copy()}
              />
            </div>
            <p className={styles.srStatus} role="status">
              {copied ? t('settings.apiKeys.copiedStatus', 'Copied to clipboard') : ''}
            </p>
            {copyError ? (
              <Notice tone="danger">
                {t('settings.apiKeys.copyFailed', 'Failed to copy to clipboard. Please select and copy manually.')}
              </Notice>
            ) : null}
            <p className={formStyles.help}>
              {t('settings.apiKeys.usage', 'Use it like:')}{' '}
              <code className={styles.inlineCode}>curl -H "Authorization: Bearer {revealed.plaintext.slice(0, 15)}…"</code>
            </p>
          </>
        ) : null}
      </div>
    </Dialog>
  );
}

export function RevokeKeyDialog({
  target,
  onCancel,
  onRevoke,
}: {
  target: ApiKey | null;
  onCancel: () => void;
  onRevoke: (key: ApiKey) => Promise<void>;
}) {
  const { t } = useTranslation();
  const [revoking, setRevoking] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => setError(null), [target]);

  const run = async () => {
    if (!target) return;
    setRevoking(true);
    setError(null);
    try {
      await onRevoke(target);
    } catch (err) {
      // Shown inside the dialog, where the user is looking.
      setError(err instanceof Error ? err.message : t('settings.apiKeys.revokeFailed', 'Failed to revoke API key.'));
    } finally {
      setRevoking(false);
    }
  };

  return (
    <Dialog
      role="alertdialog"
      size="sm"
      isOpen={target !== null}
      onOpenChange={(open) => !open && !revoking && onCancel()}
      title={t('settings.apiKeys.revokeTitle', 'Revoke API key')}
      actions={
        <>
          <Button variant="ghost" onPress={onCancel} isDisabled={revoking}>
            {t('common.actions.cancel')}
          </Button>
          <Button variant="danger" onPress={() => void run()} isPending={revoking}>
            {t('settings.apiKeys.revoke', 'Revoke')}
          </Button>
        </>
      }
    >
      <div className={formStyles.form}>
        <p>
          {t(
            'settings.apiKeys.revokeMessage',
            'Revoke {{name}}? Any scripts or integrations using this key will immediately stop working. This cannot be undone.',
            { name: target?.name ?? '' },
          )}
        </p>
        {error ? <Notice tone="danger" onDismiss={() => setError(null)}>{error}</Notice> : null}
      </div>
    </Dialog>
  );
}
