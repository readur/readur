import { useMemo, useState } from 'react';
import { Link, useSearchParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import {
  BoardTable,
  BulkActionBar,
  Button,
  EmptyState,
  Pagination,
  Pass,
  PassCell,
  SearchField,
  Select,
  SelectItem,
  useToast,
  type BoardColumn,
  type Selection,
} from '../../../ui';
import { sourcesService, type SourceResponse } from '../../../services/api';
import { ignoredFilesService, type IgnoredFileResponse } from '../../../services/api/ignoredFiles';
import { serverMessage } from '../shared/errors';
import { formatBytes, formatDateTime, formatRelative } from '../shared/format';
import { ConfirmDialog, Notice, sharedStyles } from '../shared/parts';
import { SOURCE_TYPES, isSourceType, sourceTypeLabel } from '../shared/sourceTypes';
import { useLoader } from '../shared/useLoader';
import styles from './Ignored.module.css';

export const IGNORED_PAGE_SIZE = 25;
const ALL = 'all';

/** Files skipped during syncs. Un-ignoring one lets the next sync import it. */
export function IgnoredSection() {
  const { t, i18n } = useTranslation();
  const toast = useToast();
  const [params, setParams] = useSearchParams();
  const sourceId = params.get('sourceId') ?? '';
  const sourceName = params.get('sourceName') ?? '';
  const typeParam = params.get('sourceType');
  const [sourceType, setSourceType] = useState<string>(isSourceType(typeParam) ? typeParam : ALL);
  const [search, setSearch] = useState('');
  const [reason, setReason] = useState<string>(ALL);
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(IGNORED_PAGE_SIZE);
  const [selected, setSelected] = useState<Selection>(new Set());
  const [confirm, setConfirm] = useState<{ ids: string[]; name?: string } | null>(null);
  const [removing, setRemoving] = useState(false);
  const lng = i18n.language;

  const list = useLoader(
    async () =>
      (
        await ignoredFilesService.list({
          limit: pageSize,
          offset: (page - 1) * pageSize,
          filename: search.trim() || undefined,
          source_type: sourceType === ALL ? undefined : sourceType,
          source_identifier: sourceId || undefined,
        })
      ).data,
    [page, pageSize, search, sourceType, sourceId],
  );
  const stats = useLoader(async () => (await ignoredFilesService.stats()).data, []);
  const sources = useLoader(async () => (await sourcesService.list()).data ?? [], []);

  const files = list.data?.ignored_files ?? [];
  const reasons = useMemo(() => Array.from(new Set(files.map((f) => f.reason).filter((r): r is string => Boolean(r)))), [files]);
  const rows = reason === ALL ? files : files.filter((f) => f.reason === reason);
  const selectedIds = selected === 'all' ? rows.map((f) => f.id) : rows.filter((f) => selected.has(f.id)).map((f) => f.id);

  const sourceNameOf = (f: IgnoredFileResponse): string | null => {
    const match = (sources.data ?? []).find((s: SourceResponse) => s.id === f.source_identifier);
    return match?.name ?? f.source_identifier ?? null;
  };

  const resetPage = () => {
    setPage(1);
    setSelected(new Set());
  };

  const clearSourceFilter = () => {
    setParams({ section: 'ignored' });
    setSourceType(ALL);
    resetPage();
  };

  const unignore = async () => {
    if (!confirm) return;
    setRemoving(true);
    try {
      const res =
        confirm.ids.length === 1
          ? await ignoredFilesService.remove(confirm.ids[0])
          : await ignoredFilesService.bulkRemove(confirm.ids);
      toast.show({
        title: t('intake.ignored.removed', 'Removed from the ignored list'),
        description: res.data?.message,
        tone: 'success',
      });
      setConfirm(null);
      setSelected(new Set());
      await Promise.all([list.reload(), stats.reload()]);
    } catch (error) {
      toast.show({ title: t('intake.ignored.removeFailed', 'Could not update the ignored list'), description: serverMessage(error), tone: 'danger' });
    } finally {
      setRemoving(false);
    }
  };

  const columns: BoardColumn<IgnoredFileResponse>[] = [
    { id: 'name', label: t('intake.ignored.col.name', 'Name'), render: (f) => f.filename },
    {
      id: 'source',
      hideOnNarrow: true,
      label: t('intake.ignored.col.source', 'Source'),
      width: 180,
      render: (f) => [sourceTypeLabel(t, f.source_type), sourceNameOf(f)].filter(Boolean).join(' · '),
    },
    { id: 'size', hideOnNarrow: true, label: t('intake.ignored.col.size', 'Size'), align: 'end', width: 100, render: (f) => formatBytes(f.file_size, 2) },
    { id: 'ignored', hideOnNarrow: true, label: t('intake.ignored.col.ignored', 'Ignored'), mono: true, width: 170, render: (f) => formatDateTime(f.ignored_at, lng) },
    { id: 'reason', hideOnNarrow: true, label: t('intake.ignored.col.reason', 'Reason'), width: 180, render: (f) => f.reason || t('intake.ignored.noReason', 'No reason given') },
    {
      id: 'actions',
      label: <span className={sharedStyles.visuallyHidden}>{t('intake.ignored.col.actions', 'Actions')}</span>,
      textValue: t('intake.ignored.col.actions', 'Actions'),
      width: 120,
      align: 'end',
      render: (f) => (
        <Button
          size="sm"
          onPress={() => setConfirm({ ids: [f.id], name: f.filename })}
          aria-label={t('intake.ignored.unignoreFile', 'Un-ignore {{name}}', { name: f.filename })}
        >
          {t('intake.ignored.unignore', 'Un-ignore')}
        </Button>
      ),
    },
  ];

  const filteredBySource = Boolean(sourceId || sourceName || typeParam);
  const total = list.data?.total ?? 0;

  return (
    <div className={sharedStyles.section}>
      <nav aria-label={t('intake.ignored.breadcrumbs', 'Breadcrumbs')}>
        <ol className={styles.crumbs}>
          <li>
            <Link to="/intake?section=connections">{t('intake.sections.connections', 'Connections')}</Link>
          </li>
          {filteredBySource ? (
            <li>
              <span>{sourceName || sourceTypeLabel(t, typeParam)}</span>
            </li>
          ) : null}
          <li aria-current="page">{t('intake.ignored.title', 'Ignored files')}</li>
        </ol>
      </nav>

      {stats.data ? (
        <Pass aria-label={t('intake.ignored.summary', 'Ignored files summary')}>
          <PassCell label={t('intake.ignored.total', 'Ignored files')} mono>{String(stats.data.total_ignored_files)}</PassCell>
          <PassCell label={t('intake.ignored.totalSize', 'Total size')} mono>{formatBytes(stats.data.total_size_bytes, 2)}</PassCell>
          <PassCell label={t('intake.ignored.latest', 'Most recent')} mono>
            {stats.data.most_recent_ignored_at ? formatRelative(stats.data.most_recent_ignored_at, lng) : '—'}
          </PassCell>
        </Pass>
      ) : null}

      <div className={sharedStyles.toolbar}>
        <SearchField
          className={styles.search}
          label={t('intake.ignored.search', 'Search file names')}
          value={search}
          onChange={(v) => {
            setSearch(v);
            resetPage();
          }}
        />
        <Select
          label={t('intake.ignored.sourceType', 'Source type')}
          selectedKey={sourceType}
          onSelectionChange={(key) => {
            setSourceType(String(key));
            resetPage();
          }}
        >
          <SelectItem id={ALL}>{t('intake.ignored.allSources', 'All sources')}</SelectItem>
          {SOURCE_TYPES.map((type) => (
            <SelectItem key={type} id={type}>
              {sourceTypeLabel(t, type)}
            </SelectItem>
          ))}
        </Select>
        <Select
          label={t('intake.ignored.reason', 'Reason')}
          selectedKey={reason}
          onSelectionChange={(key) => setReason(String(key))}
        >
          <SelectItem id={ALL}>{t('intake.ignored.allReasons', 'All reasons')}</SelectItem>
          {reasons.map((r) => (
            <SelectItem key={r} id={r}>
              {r}
            </SelectItem>
          ))}
        </Select>
        {filteredBySource ? (
          <Button variant="ghost" onPress={clearSourceFilter}>
            {t('intake.ignored.clearSource', 'Show all sources')}
          </Button>
        ) : null}
        <div className={sharedStyles.toolbarEnd}>
          <Button
            onPress={() => {
              void list.reload();
              void stats.reload();
            }}
          >
            {t('intake.actions.refresh', 'Refresh')}
          </Button>
        </div>
      </div>

      {list.error && !list.data ? (
        <Notice tone="danger" live="alert" title={t('intake.ignored.loadFailed', 'Could not load ignored files.')}>
          <Button size="sm" onPress={() => void list.reload()}>
            {t('intake.actions.retry', 'Retry')}
          </Button>
        </Notice>
      ) : (
        <BoardTable
          aria-label={t('intake.ignored.title', 'Ignored files')}
          columns={columns}
          rows={rows}
          getRowId={(f) => f.id}
          selectionMode="multiple"
          selectedKeys={selected}
          onSelectionChange={setSelected}
          renderRowDetail={(f) => <span className={sharedStyles.mono}>{f.source_path || f.file_path}</span>}
          isLoading={list.isLoading}
          emptyState={
            <EmptyState
              headingAs="h3"
              title={t('intake.ignored.emptyTitle', 'Nothing is ignored')}
              description={t('intake.ignored.emptyBody', 'Every file your connections found is being imported.')}
            />
          }
        />
      )}

      {total > pageSize ? (
        <Pagination
          page={page}
          pageSize={pageSize}
          total={total}
          onChange={(p, size) => {
            setPage(p);
            setPageSize(size);
            setSelected(new Set());
          }}
        />
      ) : null}

      <BulkActionBar
        count={selectedIds.length}
        onClear={() => setSelected(new Set())}
        actions={[
          {
            id: 'unignore',
            label: t('intake.ignored.unignoreSelected', 'Un-ignore'),
            onPress: () =>
              setConfirm({ ids: selectedIds, name: rows.find((f) => f.id === selectedIds[0])?.filename }),
          },
        ]}
      />

      <ConfirmDialog
        isOpen={confirm !== null}
        onOpenChange={(open) => !open && setConfirm(null)}
        tone="primary"
        title={
          confirm?.ids.length === 1
            ? t('intake.ignored.confirmOne', 'Un-ignore “{{name}}”?', { name: confirm.name ?? '' })
            : t('intake.ignored.confirmMany', 'Un-ignore {{count}} files?', { count: confirm?.ids.length ?? 0 })
        }
        confirmLabel={t('intake.ignored.confirm', 'Un-ignore')}
        isPending={removing}
        onConfirm={unignore}
      >
        <p>
          {t(
            'intake.ignored.confirmBody',
            'These files are removed from the ignored list and will be imported again the next time their connection syncs.',
          )}
        </p>
      </ConfirmDialog>
    </div>
  );
}
