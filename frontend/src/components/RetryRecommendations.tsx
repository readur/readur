import React, { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button, Skeleton } from '../ui';
import { documentService, type BulkOcrRetryResponse, type OcrRetryRecommendation } from '../services/api';
import styles from './RetryModals.module.css';

interface RetryRecommendationsProps {
  onRetrySuccess?: (result: BulkOcrRetryResponse) => void;
  onRetryClick?: (recommendation: OcrRetryRecommendation) => void;
}

export function successRateLevel(rate: number): 'high' | 'medium' | 'low' {
  const pct = Math.round(rate * 100);
  if (pct >= 70) return 'high';
  if (pct >= 40) return 'medium';
  return 'low';
}

/** Groups of failed documents that are likely to succeed if retried now, each with a one-press retry. */
export const RetryRecommendations: React.FC<RetryRecommendationsProps> = ({ onRetrySuccess, onRetryClick }) => {
  const { t } = useTranslation();
  const [recommendations, setRecommendations] = useState<OcrRetryRecommendation[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [retrying, setRetrying] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const response = await documentService.getRetryRecommendations();
      setRecommendations(response.data?.recommendations ?? []);
    } catch (err: unknown) {
      const message = (err as { response?: { data?: { message?: string } } })?.response?.data?.message;
      setError(message || t('intake.recommend.loadFailed', 'Could not load retry recommendations'));
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => {
    void load();
  }, [load]);

  const retry = async (recommendation: OcrRetryRecommendation) => {
    if (onRetryClick) {
      onRetryClick(recommendation);
      return;
    }
    setRetrying(recommendation.reason);
    try {
      const response = await documentService.bulkRetryOcr({
        mode: 'filter',
        filter: recommendation.filter,
        preview_only: false,
      });
      onRetrySuccess?.(response.data);
      void load();
    } catch (err: unknown) {
      const message = (err as { response?: { data?: { message?: string } } })?.response?.data?.message;
      setError(message || t('intake.recommend.retryFailed', 'Could not start the retry'));
    } finally {
      setRetrying(null);
    }
  };

  const rateText = (rate: number) => {
    const pct = Math.round(rate * 100);
    switch (successRateLevel(rate)) {
      case 'high':
        return t('intake.recommend.rateHigh', '{{pct}}% likely (high)', { pct });
      case 'medium':
        return t('intake.recommend.rateMedium', '{{pct}}% likely (medium)', { pct });
      default:
        return t('intake.recommend.rateLow', '{{pct}}% likely (low)', { pct });
    }
  };

  const list = recommendations ?? [];
  return (
    <section className={styles.stack} aria-label={t('intake.recommend.title', 'Retry recommendations')}>
      <div className={styles.headRow}>
        <h3 className={styles.heading}>{t('intake.recommend.title', 'Retry recommendations')}</h3>
        <Button size="sm" variant="ghost" onPress={() => void load()} isPending={loading}>
          {t('intake.actions.refresh', 'Refresh')}
        </Button>
      </div>
      {error ? (
        <p className={styles.error} role="alert">
          {error}
        </p>
      ) : null}
      {loading && list.length === 0 ? (
        <Skeleton lines={2} label={t('intake.recommend.loading', 'Looking for failure patterns…')} />
      ) : list.length === 0 ? (
        <div className={styles.muted}>
          <p>{t('intake.recommend.empty', 'No retry recommendations right now. This usually means:')}</p>
          <ul className={styles.bullets}>
            <li>{t('intake.recommend.emptyRetried', 'Failed documents have already been retried several times')}</li>
            <li>{t('intake.recommend.emptyPattern', 'No failure pattern suggests a retry would succeed')}</li>
            <li>{t('intake.recommend.emptyTypes', 'No failures of a kind that usually succeeds on retry')}</li>
          </ul>
        </div>
      ) : (
        <ul className={styles.cards}>
          {list.map((r) => (
            <li key={r.reason} className={styles.card}>
              <div className={styles.headRow}>
                <span className={styles.cardTitle}>{r.title}</span>
                <span className={styles.mono}>{rateText(r.estimated_success_rate)}</span>
              </div>
              <p className={styles.muted}>{r.description}</p>
              <p className={styles.mono}>
                {t('intake.recommend.count', '{{count}} documents · {{pattern}}', {
                  count: r.document_count,
                  pattern: r.reason.replace(/_/g, ' '),
                })}
              </p>
              <ul className={styles.criteria} aria-label={t('intake.recommend.criteria', 'Criteria')}>
                {r.filter.failure_reasons?.map((reason) => <li key={reason}>{reason.replace(/_/g, ' ')}</li>)}
                {r.filter.mime_types?.map((type) => <li key={type}>{(type.split('/')[1] ?? type).toUpperCase()}</li>)}
                {r.filter.max_file_size ? <li>{`< ${Math.round(r.filter.max_file_size / (1024 * 1024))} MB`}</li> : null}
              </ul>
              <div>
                <Button
                  variant="primary"
                  size="sm"
                  onPress={() => void retry(r)}
                  isPending={retrying === r.reason}
                  isDisabled={retrying !== null && retrying !== r.reason}
                >
                  {t('intake.recommend.retry', 'Retry {{count}} documents', { count: r.document_count })}
                </Button>
              </div>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
};
