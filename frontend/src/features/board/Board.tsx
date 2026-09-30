import { Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { PageHeader } from '../shell';
import { ArrivalsPanel } from './ArrivalsPanel';
import { AttentionStrip } from './AttentionStrip';
import { ConnectionsPanel } from './ConnectionsPanel';
import { fetchFailedOcr, fetchQueueFigures, fetchSources, POLL_MS } from './data';
import { MarkAllSeen } from './MarkAllSeen';
import { ProcessingPanel } from './ProcessingPanel';
import { TotalsRow } from './TotalsRow';
import { formatCount } from './format';
import { useResource } from './useResource';
import styles from './Board.module.css';

const BOARD_KINDS = ['document', 'attention'] as const;

/** Home: what arrived, what is processing, what needs you. */
export default function Board() {
  const { t, i18n } = useTranslation();
  const failed = useResource(fetchFailedOcr, POLL_MS);
  const sources = useResource(fetchSources, POLL_MS);
  const stats = useResource(fetchQueueFigures, POLL_MS);
  // Headline figure: what is waiting on OCR. Absent until known (and for users who cannot see the queue).
  const inQueue = stats.data ? stats.data.pending + stats.data.processing : null;
  const figure =
    inQueue === null
      ? undefined
      : inQueue === 0
        ? t('board.figure.zero', '0 in queue')
        : t('board.figure.inQueue', '{{formatted}} in queue', { formatted: formatCount(inQueue, i18n.language) });

  return (
    <>
      <PageHeader
        title={t('board.title', 'Board')}
        figure={figure}
        actions={
          <>
            <MarkAllSeen kinds={BOARD_KINDS} />
            <Link className={styles.linkButton} to="/intake?section=upload">
              {t('board.addDocuments', 'Add documents')}
            </Link>
          </>
        }
      />
      <div className={styles.grid}>
        <AttentionStrip failed={failed} sources={sources} />
        {/* One segmented strip: the queue, then the library. */}
        <div className={styles.strip}>
          <div className={styles.stripInner}>
            <ProcessingPanel failed={failed} stats={stats} />
            <TotalsRow />
          </div>
        </div>
        <ArrivalsPanel />
        <div className={styles.side}>
          <ConnectionsPanel sources={sources} />
        </div>
      </div>
    </>
  );
}
