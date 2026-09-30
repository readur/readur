import { useTranslation } from 'react-i18next';
import { IconButton } from '../../../ui';
import { Refresh } from '../../../ui/icons';
import type { ConnectionStatus } from '../../../services/syncProgress';
import styles from './Sync.module.css';

export interface ConnectionStatusIndicatorProps {
  connectionStatus: ConnectionStatus;
  isActive?: boolean;
  onReconnect?: () => void;
}

type Tone = 'ok' | 'neutral' | 'danger';

/** Live-connection state of the progress feed: a glyph and a word, plus Reconnect after a failure. */
export function ConnectionStatusIndicator({ connectionStatus, isActive = false, onReconnect }: ConnectionStatusIndicatorProps) {
  const { t } = useTranslation();

  let glyph = '○';
  let tone: Tone = 'neutral';
  let word: string;
  switch (connectionStatus) {
    case 'connecting':
      glyph = '◐';
      word = t('intake.sync.connection.connecting', 'Connecting…');
      break;
    case 'reconnecting':
      glyph = '◐';
      word = t('intake.sync.connection.reconnecting', 'Reconnecting…');
      break;
    case 'connected':
      glyph = '■';
      tone = isActive ? 'ok' : 'neutral';
      word = isActive ? t('intake.sync.connection.live', 'Live') : t('intake.sync.connection.connected', 'Connected');
      break;
    case 'disconnected':
    case 'error':
      glyph = '▲';
      tone = 'danger';
      word = t('intake.sync.connection.disconnected', 'Disconnected');
      break;
    case 'failed':
      glyph = '▲';
      tone = 'danger';
      word = t('intake.sync.connection.failed', 'Connection failed');
      break;
    default:
      word = t('intake.sync.connection.unknown', 'Unknown');
  }
  const showReconnect = (connectionStatus === 'failed' || connectionStatus === 'error') && onReconnect;

  return (
    <span className={styles.connection}>
      <span className={styles.connectionWord} data-tone={tone} role="status">
        <span aria-hidden="true">{glyph}</span>
        <span>{word}</span>
      </span>
      {showReconnect ? (
        <IconButton
          size="sm"
          label={t('intake.sync.connection.reconnect', 'Reconnect')}
          icon={<Refresh fontSize="inherit" />}
          onPress={onReconnect}
        />
      ) : null}
    </span>
  );
}

export default ConnectionStatusIndicator;
