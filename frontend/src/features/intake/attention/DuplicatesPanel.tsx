import { useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { BoardTable, Button, EmptyState, Pagination, useToast, type BoardColumn } from '../../../ui';
import { documentService } from '../../../services/api';
import { formatBytes, formatDateTime, shortHash } from '../shared/format';
import { Notice, sharedStyles } from '../shared/parts';
import { useLoader } from '../shared/useLoader';

export const DUPLICATES_PAGE_SIZE = 25;

interface DuplicateDocument {
  id: string;
  filename: string;
  original_filename: string;
  file_size: number;
  mime_type: string;
  created_at: string;
}

interface DuplicateGroup {
  file_hash: string;
  duplicate_count: number;
  first_uploaded: string;
  last_uploaded: string;
  documents: DuplicateDocument[];
}

interface DuplicatesResponse {
  duplicates: DuplicateGroup[];
  pagination?: { total: number; limit: number; offset: number; has_more: boolean };
  statistics?: { total_duplicate_groups: number };
}

interface DuplicateRow {
  key: string;
  doc: DuplicateDocument;
  group: DuplicateGroup;
  position: number;
}

/** Documents with identical content, one row per file, grouped by content hash. */
export function DuplicatesPanel() {
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const toast = useToast();
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(DUPLICATES_PAGE_SIZE);
  const result = useLoader(
    async () => (await documentService.getDuplicates(pageSize, (page - 1) * pageSize)).data as DuplicatesResponse,
    [page, pageSize],
  );

  const groups = result.data?.duplicates ?? [];
  const rows = useMemo<DuplicateRow[]>(
    () =>
      groups.flatMap((group) =>
        (group.documents ?? []).map((doc, position) => ({ key: `${group.file_hash}:${doc.id}`, doc, group, position })),
      ),
    [groups],
  );
  const totalGroups = result.data?.statistics?.total_duplicate_groups ?? groups.length;
  const total = result.data?.pagination?.total ?? totalGroups;

  const download = async (doc: DuplicateDocument) => {
    try {
      await documentService.downloadFile(doc.id, doc.original_filename || doc.filename);
    } catch {
      toast.show({ title: t('intake.attention.downloadFailed', 'Could not download the file'), tone: 'danger' });
    }
  };

  const columns: BoardColumn<DuplicateRow>[] = [
    { id: 'name', label: t('intake.duplicates.col.name', 'Name'), render: (r) => r.doc.filename },
    {
      id: 'group',
      hideOnNarrow: true,
      label: t('intake.duplicates.col.group', 'Group'),
      mono: true,
      width: 220,
      render: (r) => `${shortHash(r.group.file_hash, 12)} ${r.position + 1}/${r.group.duplicate_count}`,
    },
    { id: 'size', hideOnNarrow: true, label: t('intake.duplicates.col.size', 'Size'), align: 'end', width: 100, render: (r) => formatBytes(r.doc.file_size, 2) },
    { id: 'type', hideOnNarrow: true, label: t('intake.duplicates.col.type', 'Type'), mono: true, width: 150, render: (r) => r.doc.mime_type },
    { id: 'added', hideOnNarrow: true, label: t('intake.duplicates.col.added', 'Added'), mono: true, width: 170, render: (r) => formatDateTime(r.doc.created_at, i18n.language) },
    {
      id: 'actions',
      label: <span className={sharedStyles.visuallyHidden}>{t('intake.duplicates.col.actions', 'Actions')}</span>,
      textValue: t('intake.duplicates.col.actions', 'Actions'),
      align: 'end',
      width: 190,
      render: (r) => (
        <span className={sharedStyles.rowActions}>
          <Button
            size="sm"
            variant="ghost"
            aria-label={t('intake.duplicates.viewFile', 'View {{name}}', { name: r.doc.filename })}
            onPress={() => navigate(`/documents/${r.doc.id}`)}
          >
            {t('intake.duplicates.view', 'View')}
          </Button>
          <Button
            size="sm"
            variant="ghost"
            aria-label={t('intake.duplicates.downloadFile', 'Download {{name}}', { name: r.doc.filename })}
            onPress={() => void download(r.doc)}
          >
            {t('intake.attention.download', 'Download')}
          </Button>
        </span>
      ),
    },
  ];

  return (
    <div className={sharedStyles.section}>
      <p className={sharedStyles.lead}>
        {t(
          'intake.duplicates.lead',
          'These files have identical content under different names. Open each one to decide which to keep; the first upload of a group is listed first.',
        )}
      </p>
      {result.data ? (
        <p className={sharedStyles.meta}>
          {t('intake.duplicates.count', '{{count}} groups of identical files', { count: totalGroups })}
        </p>
      ) : null}
      {result.error && !result.data ? (
        <Notice tone="danger" live="alert" title={t('intake.duplicates.loadFailed', 'Could not load duplicates.')}>
          <Button size="sm" onPress={() => void result.reload()}>{t('intake.actions.retry', 'Retry')}</Button>
        </Notice>
      ) : (
        <BoardTable
          aria-label={t('intake.duplicates.label', 'Duplicate documents')}
          columns={columns}
          rows={rows}
          getRowId={(r) => r.key}
          onRowAction={(key) => {
            const row = rows.find((r) => r.key === key);
            if (row) navigate(`/documents/${row.doc.id}`);
          }}
          renderRowDetail={(r) =>
            r.doc.original_filename && r.doc.original_filename !== r.doc.filename
              ? t('intake.duplicates.original', 'Original name: {{name}}', { name: r.doc.original_filename })
              : null
          }
          isLoading={result.isLoading}
          emptyState={
            <EmptyState
              headingAs="h3"
              title={t('intake.duplicates.emptyTitle', 'No duplicates')}
              description={t('intake.duplicates.emptyBody', 'Every file in the library has unique content.')}
            />
          }
        />
      )}
      {total > pageSize ? (
        <Pagination page={page} pageSize={pageSize} total={total} onChange={(p, size) => { setPage(p); setPageSize(size); }} />
      ) : null}
    </div>
  );
}
