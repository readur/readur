/* eslint-disable @typescript-eslint/no-explicit-any */
import { useTranslation } from 'react-i18next';
import { Tag } from '../shared/Facts';
import shared from '../shared/shared.module.css';
import { when } from './types';
import styles from './Debug.module.css';

const statusTone = (status: string) => (status === 'completed' ? 'ok' : status === 'failed' ? 'danger' : 'default');

/** OCR queue history (step 2) or, with `detailed`, every processing attempt with timings and errors. */
export function HistoryTable({ title, rows, detailed = false }: { title: string; rows: any[]; detailed?: boolean }) {
  const { t } = useTranslation();
  const Q = detailed ? 'debug.processingLogs' : 'debug.steps.queueStatus';
  return (
    <div className={shared.stack}>
      <p className={styles.blockTitle}>{title}</p>
      <div className={shared.tableWrap}>
        <table className={shared.table}>
          <thead>
            <tr>
              {detailed ? <th scope="col">{t(`${Q}.attempt`)}</th> : null}
              <th scope="col">{t(`${Q}.status`)}</th>
              <th scope="col">{t(`${Q}.priority`)}</th>
              <th scope="col">{t(`${Q}.created`)}</th>
              <th scope="col">{t(`${Q}.started`)}</th>
              <th scope="col">{t(`${Q}.completed`)}</th>
              {detailed ? <th scope="col">{t(`${Q}.duration`)}</th> : null}
              {detailed ? <th scope="col">{t(`${Q}.waitTime`)}</th> : null}
              <th scope="col">{t(`${Q}.attempts`)}</th>
              <th scope="col">{t(`${Q}.worker`)}</th>
              {detailed ? <th scope="col">{t(`${Q}.error`)}</th> : null}
            </tr>
          </thead>
          <tbody>
            {rows.map((row, index) => (
              <tr key={row.id ?? index}>
                {detailed ? <td className={shared.num}>{index + 1}</td> : null}
                <td>
                  <Tag tone={statusTone(row.status)}>{row.status}</Tag>
                </td>
                <td className={shared.num}>{row.priority}</td>
                <td className={shared.num}>{when(row.created_at)}</td>
                <td className={shared.num}>{when(row.started_at)}</td>
                <td className={shared.num}>{when(row.completed_at)}</td>
                {detailed ? <td className={shared.num}>{row.processing_duration_ms ? `${row.processing_duration_ms}ms` : '-'}</td> : null}
                {detailed ? <td className={shared.num}>{row.queue_wait_time_ms ? `${row.queue_wait_time_ms}ms` : '-'}</td> : null}
                <td className={shared.num}>{detailed ? row.attempts || 0 : row.attempts}</td>
                <td className={shared.num}>{row.worker_id || '-'}</td>
                {detailed ? <td className={styles.errorCell}>{row.error_message || '-'}</td> : null}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
