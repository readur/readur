import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { BoardTable, Button, Pass, PassCell, useToast, type BoardColumn } from '../../../../ui';
import { sourcesService, type WebDAVCrawlEstimate } from '../../../../services/api';
import type { WebDAVFolderInfo } from '../../../../types/generated';
import { formatCount } from '../../shared/format';
import styles from './SourceForm.module.css';

export interface CrawlEstimateProps {
  sourceId: string;
}

/** Counts and time for a full crawl of a saved WebDAV connection. */
export function CrawlEstimate({ sourceId }: CrawlEstimateProps) {
  const { t, i18n } = useTranslation();
  const toast = useToast();
  const [estimate, setEstimate] = useState<WebDAVCrawlEstimate | null>(null);
  const [isPending, setPending] = useState(false);

  const run = async () => {
    setPending(true);
    try {
      const res = await sourcesService.estimate(sourceId);
      setEstimate(res.data);
      toast.show({ title: t('intake.form.estimate.done', 'Estimate ready'), tone: 'success' });
    } catch {
      toast.show({ title: t('intake.form.estimate.failed', 'Could not estimate the crawl'), tone: 'danger' });
    } finally {
      setPending(false);
    }
  };

  const columns: BoardColumn<WebDAVFolderInfo>[] = [
    { id: 'path', label: t('intake.form.estimate.folder', 'Folder'), render: (f) => f.path },
    { id: 'files', label: t('intake.form.estimate.files', 'Files'), align: 'end', render: (f) => formatCount(f.total_files, i18n.language) },
    { id: 'supported', label: t('intake.form.estimate.supported', 'Supported'), align: 'end', render: (f) => formatCount(f.supported_files, i18n.language) },
    { id: 'time', label: t('intake.form.estimate.time', 'Time'), align: 'end', render: (f) => `${f.estimated_time_hours.toFixed(1)}h` },
    { id: 'size', label: t('intake.form.estimate.sizeMb', 'Size (MB)'), align: 'end', render: (f) => f.total_size_mb.toFixed(1) },
  ];

  return (
    <fieldset className={styles.group}>
      <legend className={styles.legend}>{t('intake.form.estimate.title', 'Crawl estimate')}</legend>
      <p className={styles.hint}>
        {t('intake.form.estimate.hint', 'Estimate how many files a full sync would process and how long it would take.')}
      </p>
      <div>
        <Button onPress={run} isPending={isPending}>
          {isPending ? t('intake.form.estimate.running', 'Estimating…') : t('intake.form.estimate.run', 'Estimate crawl')}
        </Button>
      </div>
      {estimate ? (
        <>
          <Pass aria-label={t('intake.form.estimate.results', 'Estimate results')}>
            <PassCell label={t('intake.form.estimate.totalFiles', 'Total files')} mono>
              {formatCount(estimate.total_files, i18n.language)}
            </PassCell>
            <PassCell label={t('intake.form.estimate.supportedFiles', 'Supported files')} mono>
              {formatCount(estimate.total_supported_files, i18n.language)}
            </PassCell>
            <PassCell label={t('intake.form.estimate.estimatedTime', 'Estimated time')} mono>
              {`${estimate.total_estimated_time_hours.toFixed(1)}h`}
            </PassCell>
            <PassCell label={t('intake.form.estimate.totalSize', 'Total size')} mono>
              {`${(estimate.total_size_mb / 1024).toFixed(1)} GB`}
            </PassCell>
          </Pass>
          {estimate.folders.length > 0 ? (
            <BoardTable
              aria-label={t('intake.form.estimate.byFolder', 'Estimate by folder')}
              columns={columns}
              rows={estimate.folders}
              getRowId={(f) => f.path}
              density="compact"
            />
          ) : null}
        </>
      ) : null}
    </fieldset>
  );
}
