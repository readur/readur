import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { IconButton } from '../../../ui';
import { Check, ContentCopy } from '../../../ui/icons';
import { Notice } from '../shared/Notice';
import styles from './ApiKeys.module.css';

/**
 * A read-only value with a copy button, a screen-reader status and a fallback notice when the
 * clipboard is unavailable. `label` names the value; the button is "Copy <label>" unless
 * `copyLabel` overrides it.
 */
export function CopyBlock({
  label,
  value,
  copyLabel,
  multiline = false,
}: {
  label: string;
  value: string;
  copyLabel?: string;
  multiline?: boolean;
}) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    setCopied(false);
    setFailed(false);
  }, [value]);

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(value);
      setCopied(true);
      setFailed(false);
    } catch {
      setFailed(true);
    }
  };

  const Value = multiline ? 'pre' : 'code';
  return (
    <>
      <div className={styles.secret}>
        <Value className={multiline ? styles.snippetBlock : styles.secretValue} aria-label={label}>
          {value}
        </Value>
        <IconButton
          label={
            copied
              ? t('settings.apiKeys.copied', 'Copied')
              : (copyLabel ?? t('settings.apiKeys.mcp.copyNamed', 'Copy {{name}}', { name: label }))
          }
          icon={copied ? <Check fontSize="inherit" /> : <ContentCopy fontSize="inherit" />}
          onPress={() => void copy()}
        />
      </div>
      <p className={styles.srStatus} role="status">
        {copied ? t('settings.apiKeys.copiedStatus', 'Copied to clipboard') : ''}
      </p>
      {failed ? (
        <Notice tone="danger">
          {t(
            'settings.apiKeys.copyFailed',
            'Failed to copy to clipboard. Please select and copy manually.',
          )}
        </Notice>
      ) : null}
    </>
  );
}
