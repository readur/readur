import { useTranslation } from 'react-i18next';
import { humanizeFailureReason } from '../../../lib/failureReason';
import { humanizeConnectionFailure } from '../connections/connectionFailure';
import styles from './Shared.module.css';

export interface HumanReasonProps {
  /** The raw message from the server. */
  raw: string | null | undefined;
  /** Extra technical text (a path, say) kept behind the details disclosure. */
  extra?: string | null;
  /** The server's failure code, used when the text itself says nothing known. */
  code?: string | null;
  /** Show only the plain-language summary, with no disclosure. */
  summaryOnly?: boolean;
  /** `connection` reads server, sign-in and network errors; `document` (default) reads file errors. */
  kind?: 'document' | 'connection';
}

/** A failure in plain words; the raw text stays one click away. */
export function HumanReason({ raw, extra, code, summaryOnly, kind = 'document' }: HumanReasonProps) {
  const { t } = useTranslation();
  const text = (raw ?? '').trim();
  if (!text) return null;
  const { summary, detail } = (kind === 'connection' ? humanizeConnectionFailure : humanizeFailureReason)(text, code);
  const technical = [detail, extra].filter((part): part is string => Boolean(part && part.trim()));
  if (summaryOnly || technical.length === 0) return <span className={styles.reasonSummary}>{summary}</span>;
  return (
    <span className={styles.reason}>
      <span className={styles.reasonSummary}>{summary}</span>
      <details className={styles.reasonDetails}>
        <summary>{t('intake.reason.details', 'Technical details')}</summary>
        <pre className={styles.codeBlock}>{technical.join('\n')}</pre>
      </details>
    </span>
  );
}
