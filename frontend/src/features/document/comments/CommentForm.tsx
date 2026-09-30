import { useState, type FormEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { Button, TextField } from '../../../ui';
import styles from './Comments.module.css';

export interface CommentFormProps {
  onSubmit: (content: string) => Promise<void>;
  /** Accessible name of the text box. */
  label: string;
  placeholder?: string;
  submitLabel?: string;
  initialValue?: string;
  autoFocus?: boolean;
  onCancel?: () => void;
}

/** Text box plus submit (and optional cancel) used for new comments, replies and edits. */
export function CommentForm({
  onSubmit,
  label,
  placeholder,
  submitLabel,
  initialValue = '',
  autoFocus,
  onCancel,
}: CommentFormProps) {
  const { t } = useTranslation();
  const [content, setContent] = useState(initialValue);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = async (e?: FormEvent) => {
    e?.preventDefault();
    const trimmed = content.trim();
    if (!trimmed || pending) return;
    setPending(true);
    setError(null);
    try {
      await onSubmit(trimmed);
      setContent('');
    } catch {
      setError(t('document.comments.saveFailed', "Couldn't save the comment. Try again."));
    } finally {
      setPending(false);
    }
  };

  return (
    <form className={styles.form} onSubmit={submit}>
      <TextField
        aria-label={label}
        placeholder={placeholder}
        multiline
        rows={2}
        value={content}
        onChange={setContent}
        autoFocus={autoFocus}
        isDisabled={pending}
        isInvalid={Boolean(error)}
        errorMessage={error ?? undefined}
        onKeyDown={(e) => {
          if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) void submit();
        }}
      />
      <div className={styles.formActions}>
        {onCancel ? (
          <Button size="sm" variant="ghost" onPress={onCancel} isDisabled={pending}>
            {t('document.comments.cancel', 'Cancel')}
          </Button>
        ) : null}
        <Button size="sm" variant="primary" type="submit" isPending={pending} isDisabled={!content.trim()}>
          {submitLabel ?? t('document.comments.post', 'Post')}
        </Button>
      </div>
    </form>
  );
}
