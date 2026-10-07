import { Button as RACButton } from 'react-aria-components';
import { useTranslation } from 'react-i18next';
import { ChevronLeft, ChevronRight } from '../icons';
import { IconButton } from '../IconButton';
import { Select, SelectItem } from '../Select';
import styles from './Pagination.module.css';

export interface PaginationProps {
  /** 1-based current page. */
  page: number;
  pageSize: number;
  total: number;
  /** Called with the new page (and page size when it changed; the page resets to 1 then). */
  onChange: (page: number, pageSize: number) => void;
  pageSizeOptions?: number[];
}

/** Page numbers to show: first, last, and the current page with its neighbours; gaps elsewhere. */
export function pageItems(page: number, count: number): Array<number | 'gap'> {
  if (count <= 7) return Array.from({ length: count }, (_, i) => i + 1);
  const set = new Set([1, count, page - 1, page, page + 1].filter((n) => n >= 1 && n <= count));
  const sorted = [...set].sort((a, b) => a - b);
  const out: Array<number | 'gap'> = [];
  sorted.forEach((n, i) => {
    if (i > 0 && n - sorted[i - 1] > 1) out.push('gap');
    out.push(n);
  });
  return out;
}

export function Pagination({ page, pageSize, total, onChange, pageSizeOptions }: PaginationProps) {
  const { t } = useTranslation();
  const pageCount = Math.max(1, Math.ceil(total / pageSize));
  const start = total === 0 ? 0 : (page - 1) * pageSize + 1;
  const end = Math.min(total, page * pageSize);
  const sizeItems = (pageSizeOptions ?? []).map((n) => ({ id: String(n), n }));

  return (
    <nav className={styles.pagination} aria-label={t('ui.pagination.label', 'Pagination')}>
      <span className={styles.readout} aria-live="polite">
        {t('ui.pagination.range', '{{start}}–{{end}} of {{total}}', { start, end, total })}
      </span>
      <div className={styles.controls}>
        {sizeItems.length > 0 ? (
          <div className={styles.size}>
            <Select
              className={styles.sizeSelect}
              aria-label={t('ui.pagination.pageSize', 'Rows per page')}
              items={sizeItems}
              selectedKey={String(pageSize)}
              onSelectionChange={(key) => onChange(1, Number(key))}
            >
              {(item) => <SelectItem id={item.id}>{item.n}</SelectItem>}
            </Select>
          </div>
        ) : null}
        <IconButton
          variant="secondary"
          size="sm"
          label={t('ui.pagination.previous', 'Previous page')}
          icon={<ChevronLeft fontSize="small" />}
          isDisabled={page <= 1}
          onPress={() => onChange(page - 1, pageSize)}
        />
        <span className={styles.pages}>
          {pageItems(page, pageCount).map((p, i) =>
            p === 'gap' ? (
              <span key={`gap-${i}`} className={styles.gap} aria-hidden="true">
                …
              </span>
            ) : (
              <RACButton
                key={p}
                className={styles.page}
                aria-label={t('ui.pagination.page', { defaultValue: 'Page {{n}}', n: p })}
                aria-current={p === page ? 'page' : undefined}
                onPress={() => onChange(p, pageSize)}
              >
                {p}
              </RACButton>
            ),
          )}
        </span>
        <IconButton
          variant="secondary"
          size="sm"
          label={t('ui.pagination.next', 'Next page')}
          icon={<ChevronRight fontSize="small" />}
          isDisabled={page >= pageCount}
          onPress={() => onChange(page + 1, pageSize)}
        />
      </div>
    </nav>
  );
}
