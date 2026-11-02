import { useCallback, useId, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { IconButton, StatusMark } from '../../../ui';
import { ExpandLess, ExpandMore } from '../../../ui/icons';
import type { ConnectionStatus, SyncProgressManager } from '../../../services/syncProgress';
import { Notice } from '../shared/parts';
import { ConnectionStatusIndicator } from './ConnectionStatusIndicator';
import { ProgressStatistics } from './ProgressStatistics';
import { useSyncProgress } from './useSyncProgress';
import styles from './Sync.module.css';

export interface SyncProgressDisplayProps {
  sourceId: string;
  sourceName: string;
  isVisible: boolean;
  onClose?: () => void;
  /** Inject a manager (tests); defaults to the WebSocket manager. */
  manager?: SyncProgressManager;
}

/** Live progress of one connection's sync, fed by the sync-progress WebSocket. */
export function SyncProgressDisplay({ sourceId, sourceName, isVisible, manager }: SyncProgressDisplayProps) {
  const { t } = useTranslation();
  const headingId = useId();
  const bodyId = useId();
  const [isExpanded, setIsExpanded] = useState(true);

  const handleError = useCallback((error: Error) => {
    console.error('Sync progress connection error:', error);
  }, []);
  const handleStatus = useCallback((_status: ConnectionStatus) => undefined, []);

  const { progressInfo, connectionStatus, isConnected, reconnect } = useSyncProgress({
    sourceId,
    enabled: isVisible && !!sourceId,
    onError: handleError,
    onConnectionStatusChange: handleStatus,
    manager,
  });

  if (!isVisible || (!progressInfo && connectionStatus === 'disconnected' && !isConnected)) {
    return null;
  }

  return (
    <section className={styles.display} aria-labelledby={headingId} data-active={progressInfo?.is_active ? 'true' : undefined}>
      <div className={styles.head}>
        <div className={styles.titles}>
          <h3 id={headingId} className={styles.title}>
            {t('intake.sync.title', '{{name}} sync progress', { name: sourceName })}
          </h3>
          {progressInfo?.is_active ? <StatusMark state="syncing" size="sm" /> : null}
        </div>
        <div className={styles.headActions}>
          <ConnectionStatusIndicator
            connectionStatus={connectionStatus}
            isActive={progressInfo?.is_active}
            onReconnect={reconnect}
          />
          <IconButton
            size="sm"
            label={isExpanded ? t('intake.sync.collapse', 'Collapse') : t('intake.sync.expand', 'Expand')}
            icon={isExpanded ? <ExpandLess fontSize="inherit" /> : <ExpandMore fontSize="inherit" />}
            aria-expanded={isExpanded}
            aria-controls={bodyId}
            onPress={() => setIsExpanded((v) => !v)}
          />
        </div>
      </div>
      <div id={bodyId} hidden={!isExpanded}>
        {progressInfo ? (
          <ProgressStatistics progressInfo={progressInfo} />
        ) : (
          <Notice>{t('intake.sync.waiting', 'Waiting for sync progress information…')}</Notice>
        )}
      </div>
    </section>
  );
}

export default SyncProgressDisplay;
