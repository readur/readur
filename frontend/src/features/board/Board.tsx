import { Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { PageHeader } from '../shell';
import { ArrivalsPanel } from './ArrivalsPanel';
import { AttentionStrip } from './AttentionStrip';
import { ConnectionsPanel } from './ConnectionsPanel';
import { fetchFailedOcr, fetchSources, POLL_MS } from './data';
import { MarkAllSeen } from './MarkAllSeen';
import { ProcessingPanel } from './ProcessingPanel';
import { TotalsRow } from './TotalsRow';
import { useResource } from './useResource';
import styles from './Board.module.css';

const BOARD_KINDS = ['document', 'attention'] as const;

/** Home: what arrived, what is processing, what needs you. */
export default function Board() {
  const { t } = useTranslation();
  const failed = useResource(fetchFailedOcr, POLL_MS);
  const sources = useResource(fetchSources, POLL_MS);

  return (
    <>
      <PageHeader
        title={t('board.title', 'Board')}
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
        {/* One segmented pass strip: the queue, then the library. */}
        <div className={styles.strip}>
          <div className={styles.stripInner}>
            <ProcessingPanel failed={failed} />
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
