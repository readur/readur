import { Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { PageHeader } from '../shell';
import { ArrivalsPanel } from './ArrivalsPanel';
import { AttentionStrip } from './AttentionStrip';
import { ConnectionsPanel } from './ConnectionsPanel';
import { fetchFailedOcr, fetchSources, POLL_MS } from './data';
import { ProcessingPanel } from './ProcessingPanel';
import { TotalsRow } from './TotalsRow';
import { useResource } from './useResource';
import styles from './Board.module.css';

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
          <Link className={styles.linkButton} to="/intake?section=upload">
            {t('board.addDocuments', 'Add documents')}
          </Link>
        }
      />
      <div className={styles.grid}>
        <AttentionStrip failed={failed} sources={sources} />
        <ArrivalsPanel />
        <div className={styles.side}>
          <ProcessingPanel />
          <ConnectionsPanel sources={sources} />
        </div>
        <TotalsRow />
      </div>
    </>
  );
}
