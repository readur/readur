import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { BoardTable, Button, EmptyState, StatusMark, type BoardColumn, type BoardSort } from '../../../ui';
import { Add, Refresh } from '../../../ui/icons';
import type { SourceResponse } from '../../../services/api';
import { useAuth } from '../../../contexts/AuthContext';
import { acknowledge, isLit, useLitCount } from '../../board/litStore';
import { formatCount, formatRelative } from '../shared/format';
import { NameCell, Notice, sharedStyles } from '../shared/parts';
import { sourceTypeLabel } from '../shared/sourceTypes';
import { SourceForm } from './form/SourceForm';
import { OcrControls } from './OcrControls';
import { SourceDetailPanel } from './SourceDetailPanel';
import { nextSyncAt, sourceState } from './sourceModel';
import { useSourceActions } from './useSourceActions';
import { useSources } from './useSources';

type SortKey = 'name' | 'lastSync' | 'files';

function compare(a: SourceResponse, b: SourceResponse, key: SortKey): number {
  if (key === 'files') return a.total_files_synced - b.total_files_synced;
  if (key === 'lastSync') return (Date.parse(a.last_sync_at ?? '') || 0) - (Date.parse(b.last_sync_at ?? '') || 0);
  return a.name.localeCompare(b.name);
}

/** Every connection with its health; a row opens its details and actions. */
export function ConnectionsSection() {
  const { t, i18n } = useTranslation();
  const { user } = useAuth();
  const sources = useSources();
  useLitCount('source'); // re-render when a row is acknowledged elsewhere
  const [sort, setSort] = useState<BoardSort>({ column: 'name', direction: 'ascending' });
  const [openId, setOpenId] = useState<string | null>(null);
  const [formOpen, setFormOpen] = useState(false);
  const [editing, setEditing] = useState<SourceResponse | null>(null);
  const actions = useSourceActions(() => void sources.reload(), (id) => {
    if (openId === id) setOpenId(null);
  });

  const rows = useMemo(() => {
    const list = [...(sources.data ?? [])];
    list.sort((a, b) => compare(a, b, sort.column as SortKey) * (sort.direction === 'ascending' ? 1 : -1));
    return list;
  }, [sources.data, sort]);
  const open = rows.find((s) => s.id === openId) ?? null;
  const lng = i18n.language;

  const columns: BoardColumn<SourceResponse>[] = [
    {
      id: 'name',
      label: t('intake.connections.col.name', 'Name'),
      sortable: true,
      render: (s) => <NameCell name={s.name} tag={isLit('source', s.id) ? 'changed' : null} />,
    },
    { id: 'type', label: t('intake.connections.col.type', 'Type'), width: 120, render: (s) => sourceTypeLabel(t, s.source_type) },
    { id: 'status', label: t('intake.connections.col.status', 'Status'), width: 130, render: (s) => <StatusMark state={sourceState(s)} size="sm" /> },
    {
      id: 'lastSync',
      label: t('intake.connections.col.lastSync', 'Last sync'),
      sortable: true,
      mono: true,
      width: 140,
      render: (s) => (s.last_sync_at ? formatRelative(s.last_sync_at, lng) : t('intake.detail.never', 'never')),
    },
    { id: 'files', label: t('intake.connections.col.files', 'Files'), sortable: true, align: 'end', width: 100, render: (s) => formatCount(s.total_files_synced, lng) },
    {
      id: 'next',
      label: t('intake.connections.col.next', 'Next'),
      mono: true,
      width: 130,
      render: (s) => {
        const next = nextSyncAt(s);
        return next ? formatRelative(next, lng) : '—';
      },
    },
  ];

  const openRow = (id: string) => {
    acknowledge('source', id);
    setOpenId(id);
  };
  const startCreate = () => {
    setEditing(null);
    setFormOpen(true);
  };
  const startEdit = (source: SourceResponse) => {
    setEditing(source);
    setFormOpen(true);
  };

  return (
    <div className={sharedStyles.section}>
      <div className={sharedStyles.toolbar}>
        <Button variant="primary" icon={<Add fontSize="inherit" />} onPress={startCreate}>
          {t('intake.connections.add', 'Add connection')}
        </Button>
        <Button
          icon={<Refresh fontSize="inherit" />}
          onPress={() => void sources.reload()}
          isPending={sources.isLoading && Boolean(sources.data)}
        >
          {sources.isAutoRefreshing ? t('intake.connections.autoRefreshing', 'Refreshing while syncing') : t('intake.actions.refresh', 'Refresh')}
        </Button>
        {user?.role === 'Admin' ? (
          <div className={sharedStyles.toolbarEnd}>
            <OcrControls />
          </div>
        ) : null}
      </div>

      {sources.error && !sources.data ? (
        <Notice tone="danger" live="alert" title={t('intake.connections.loadFailed', 'Could not load connections.')}>
          <Button size="sm" onPress={() => void sources.reload()}>
            {t('intake.actions.retry', 'Retry')}
          </Button>
        </Notice>
      ) : (
        <BoardTable
          aria-label={t('intake.connections.label', 'Connections')}
          columns={columns}
          rows={rows}
          getRowId={(s) => s.id}
          sort={sort}
          onSortChange={setSort}
          onRowAction={openRow}
          isRowLit={(s) => isLit('source', s.id)}
          isLoading={sources.isLoading && !sources.data}
          emptyState={
            <EmptyState
              headingAs="h3"
              title={t('intake.connections.emptyTitle', 'No connections yet')}
              description={t(
                'intake.connections.emptyBody',
                'Connect a WebDAV server, a local folder or S3 storage and Readur imports new files on its own.',
              )}
              action={
                <Button variant="primary" onPress={startCreate}>
                  {t('intake.connections.addFirst', 'Add your first connection')}
                </Button>
              }
            />
          }
        />
      )}

      <SourceDetailPanel
        source={open}
        isOpen={Boolean(open)}
        onOpenChange={(isOpen) => !isOpen && setOpenId(null)}
        onEdit={startEdit}
        actions={actions}
      />
      <SourceForm
        isOpen={formOpen}
        onOpenChange={setFormOpen}
        source={editing}
        onSaved={() => void sources.reload()}
      />
    </div>
  );
}
