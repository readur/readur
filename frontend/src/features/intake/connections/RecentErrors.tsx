import { useTranslation } from 'react-i18next';
import { StatusMark } from '../../../ui';
import { sourceErrorService } from '../../../services/api';
import { formatRelative } from '../shared/format';
import { HumanReason } from '../shared/HumanReason';
import { Notice, sharedStyles } from '../shared/parts';
import { useLoader } from '../shared/useLoader';
import { errorTypeLabel, severityLabel, severityState } from './sourceErrorLabels';
import styles from './Connections.module.css';

export const RECENT_ERROR_LIMIT = 5;

/** The latest unresolved scan failures recorded for one connection. */
export function RecentErrors({ sourceId, headingId }: { sourceId: string; headingId: string }) {
  const { t, i18n } = useTranslation();
  const failures = useLoader(async () => {
    const res = await sourceErrorService.listFailures({ source_id: sourceId, limit: RECENT_ERROR_LIMIT });
    return Array.isArray(res.data) ? res.data : [];
  }, [sourceId]);

  if (failures.isLoading && !failures.data) {
    return <p className={sharedStyles.meta}>{t('intake.detail.errorsLoading', 'Loading recent errors…')}</p>;
  }
  if (failures.error) {
    return <Notice tone="danger">{t('intake.detail.errorsFailed', 'Could not load recent errors.')}</Notice>;
  }
  const items = failures.data ?? [];
  if (items.length === 0) {
    return <p className={sharedStyles.meta}>{t('intake.detail.noErrors', 'No recent errors.')}</p>;
  }
  return (
    <ul className={styles.errorList} aria-labelledby={headingId}>
      {items.map((f) => (
        <li key={f.id} className={styles.errorItem}>
          <div className={styles.errorHead}>
            <StatusMark state={severityState(f.error_severity)} size="sm" />
            <span className={styles.errorType}>{errorTypeLabel(t, f.error_type)}</span>
            <span className={sharedStyles.meta}>
              {t('intake.detail.severity', 'Severity: {{severity}}', { severity: severityLabel(t, f.error_severity) })}
            </span>
          </div>
          <span className={`${sharedStyles.mono} ${styles.errorPath}`}>{f.resource_path}</span>
          {f.error_message ? (
            <span className={styles.errorMessage}>
              <HumanReason kind="connection" raw={f.error_message} />
            </span>
          ) : null}
          <span className={sharedStyles.meta}>
            {t('intake.detail.failureCount', '{{count}} failures · last {{when}}', {
              count: f.failure_count,
              when: formatRelative(f.last_failure_at, i18n.language),
            })}
          </span>
        </li>
      ))}
    </ul>
  );
}
