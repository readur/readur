import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { sharedLinksService, type SharedLinkData } from '../../../services/api';
import { Button, Skeleton } from '../../../ui';
import { apiErrorMessage } from '../format';
import { CreateSharedLinkForm } from './CreateSharedLinkForm';
import { SharedLinksList } from './SharedLinksList';
import styles from './Sharing.module.css';

/** Create a share link for a document and manage the existing ones. */
export function SharedLinksManager({ documentId }: { documentId: string }) {
  const { t } = useTranslation();
  const [links, setLinks] = useState<SharedLinkData[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchLinks = useCallback(async () => {
    try {
      const res = await sharedLinksService.listByDocument(documentId);
      setLinks(Array.isArray(res.data) ? res.data : []);
      setError(null);
    } catch (err) {
      setError(apiErrorMessage(err) ?? t('document.share.loadFailed', "Couldn't load share links."));
    } finally {
      setLoading(false);
    }
  }, [documentId, t]);

  useEffect(() => {
    setLoading(true);
    void fetchLinks();
  }, [fetchLinks]);

  return (
    <div className={styles.manager}>
      <CreateSharedLinkForm documentId={documentId} onCreated={() => void fetchLinks()} />
      <section className={styles.section} aria-labelledby={`share-existing-${documentId}`}>
        <h3 id={`share-existing-${documentId}`} className={styles.sectionTitle}>
          {t('document.share.existing', 'Existing links')}
        </h3>
        {loading ? (
          <Skeleton lines={2} label={t('document.share.loading', 'Loading share links')} />
        ) : error ? (
          <div className={styles.error} role="alert">
            <span>{error}</span>
            <Button size="sm" variant="ghost" onPress={() => void fetchLinks()}>
              {t('document.retry', 'Try again')}
            </Button>
          </div>
        ) : (
          <SharedLinksList links={links} onRevoked={() => void fetchLinks()} />
        )}
      </section>
    </div>
  );
}
