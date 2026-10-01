import { useTranslation } from 'react-i18next';
import { Pass, PassCell } from '../../../ui';
import type { SyncProgressInfo } from '../../../services/api';
import { formatBytes, formatDuration, humanize } from '../shared/format';
import { ProgressCell } from '../shared/parts';
import styles from './Sync.module.css';

export interface ProgressStatisticsProps {
  progressInfo: SyncProgressInfo;
}

/** Phase, files, bytes, rate and time remaining of a running sync, as labelled mono cells. */
export function ProgressStatistics({ progressInfo: p }: ProgressStatisticsProps) {
  const { t } = useTranslation();
  const eta = p.estimated_time_remaining_secs;
  const hasEta = eta !== null && eta !== undefined && eta > 0;

  return (
    <div className={styles.stats}>
      <Pass aria-label={t('intake.sync.statsLabel', 'Sync figures')}>
        <PassCell label={t('intake.sync.phase', 'Phase')}>{p.phase_description || humanize(p.phase)}</PassCell>
        <PassCell label={t('intake.sync.files', 'Files')} mono>
          {t('intake.sync.filesValue', '{{processed}} / {{found}} files ({{percent}}%)', {
            processed: p.files_processed,
            found: p.files_found,
            percent: p.files_progress_percent.toFixed(1),
          })}
        </PassCell>
        <PassCell label={t('intake.sync.directories', 'Directories')} mono>
          {`${p.directories_processed} / ${p.directories_found}`}
        </PassCell>
      </Pass>
      {p.files_found > 0 ? (
        <ProgressCell
          value={p.files_progress_percent}
          label={t('intake.sync.filesProgress', 'Files progress')}
        />
      ) : null}
      <Pass>
        <PassCell label={t('intake.sync.bytes', 'Data processed')} mono>
          {formatBytes(p.bytes_processed)}
        </PassCell>
        <PassCell label={t('intake.sync.rate', 'Rate')} mono>
          {t('intake.sync.rateValue', '{{rate}} files/sec', { rate: p.processing_rate_files_per_sec.toFixed(1) })}
        </PassCell>
        <PassCell label={t('intake.sync.elapsed', 'Elapsed')} mono>
          {formatDuration(p.elapsed_time_secs)}
        </PassCell>
        {hasEta ? (
          <PassCell label={t('intake.sync.eta', 'Time remaining')} mono>
            {formatDuration(eta)}
          </PassCell>
        ) : null}
      </Pass>
      {p.current_directory ? (
        <Pass>
          <PassCell label={t('intake.sync.currentDirectory', 'Current directory')} mono>
            {p.current_directory}
          </PassCell>
          {p.current_file ? (
            <PassCell label={t('intake.sync.currentFile', 'Current file')} mono>
              {p.current_file}
            </PassCell>
          ) : null}
        </Pass>
      ) : null}
      {p.errors > 0 || p.warnings > 0 ? (
        <p className={styles.problems}>
          {p.errors > 0 ? (
            <span className={`${styles.problem} ${styles.problemError}`}>
              <span aria-hidden="true">▲</span>
              <span>
                {p.errors === 1
                  ? t('intake.sync.errorOne', '1 error')
                  : t('intake.sync.errorMany', '{{count}} errors', { count: p.errors })}
              </span>
            </span>
          ) : null}
          {p.warnings > 0 ? (
            <span className={styles.problem}>
              <span aria-hidden="true">◆</span>
              <span>
                {p.warnings === 1
                  ? t('intake.sync.warningOne', '1 warning')
                  : t('intake.sync.warningMany', '{{count}} warnings', { count: p.warnings })}
              </span>
            </span>
          ) : null}
        </p>
      ) : null}
    </div>
  );
}

export default ProgressStatistics;
