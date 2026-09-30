import { useContext, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { BoardTable, Button, Dialog, EmptyState, IconButton, type BoardColumn } from '../../../ui';
import { Add, CreateNewFolder, Delete, Edit, RemoveCircle, Visibility } from '../../../ui/icons';
import { FeatureFlagsContext } from '../../../contexts/FeatureFlagsContext';
import { Notice } from '../shared/Notice';
import { Tag } from '../shared/Facts';
import shared from '../shared/shared.module.css';
import { useCurrentUserId } from '../shared/useIsAdmin';
import { UserDialog } from './UserDialog';
import { UserStatusCell } from './UserStatusCell';
import { useUsers, type UserRow } from './useUsers';
import { useWatchDirectories } from './useWatchDirectories';

type Confirm = { kind: 'delete'; user: UserRow } | { kind: 'removeDir'; user: UserRow } | null;

const formatDate = (iso?: string) => (iso ? new Date(iso).toLocaleDateString() : '—');

/** Admin user management: table, create/edit dialog, confirmed delete and per-user watch folders. */
export default function UsersSection() {
  const { t } = useTranslation();
  const currentUserId = useCurrentUserId();
  const perUserWatch = useContext(FeatureFlagsContext)?.flags.enablePerUserWatch ?? false;
  const { users, isLoading, loadError, saveUser, deleteUser, setUserActive } = useUsers();
  const [statusBusy, setStatusBusy] = useState<string | null>(null);
  const inactiveCount = users.filter((u) => u.is_active === false).length;
  const watch = useWatchDirectories(users, perUserWatch);
  const [editing, setEditing] = useState<{ user: UserRow | null } | null>(null);
  const [confirm, setConfirm] = useState<Confirm>(null);
  const [confirming, setConfirming] = useState(false);

  const watchCell = (user: UserRow) => {
    const info = watch.dirs.get(user.id);
    if (watch.busy.has(user.id)) return <span className={shared.meta}>{t('settings.userManagement.watchDirectory.loading')}</span>;
    if (!info) return <span className={shared.meta}>{t('settings.userManagement.watchDirectory.statusUnknown')}</span>;
    const [word, tone] =
      info.exists && info.enabled
        ? [t('settings.userManagement.watchDirectory.statusActive'), 'ok' as const]
        : info.exists
          ? [t('settings.userManagement.watchDirectory.statusDisabled'), 'default' as const]
          : [t('settings.userManagement.watchDirectory.statusNotCreated'), 'danger' as const];
    return (
      <span className={shared.row}>
        <Tag tone={tone}>{word}</Tag>
        <span className={`${shared.mono} ${shared.meta}`}>{info.watch_directory_path}</span>
      </span>
    );
  };

  const actions = (user: UserRow) => {
    const info = watch.dirs.get(user.id);
    const dirBusy = watch.busy.has(user.id);
    const isSelf = user.id === currentUserId;
    return (
      <span className={shared.row}>
        {perUserWatch && (!info || !info.exists) ? (
          <IconButton
            size="sm"
            label={t('settings.users.createDirFor', 'Create watch directory for {{name}}', { name: user.username })}
            icon={<CreateNewFolder fontSize="inherit" />}
            onPress={() => void watch.create(user.id)}
            isDisabled={dirBusy}
          />
        ) : null}
        {perUserWatch && info?.exists ? (
          <>
            <IconButton
              size="sm"
              label={t('settings.users.viewDirFor', 'View watch directory for {{name}}', { name: user.username })}
              icon={<Visibility fontSize="inherit" />}
              onPress={() => watch.view(info.watch_directory_path)}
              isDisabled={dirBusy}
            />
            <IconButton
              size="sm"
              label={t('settings.users.removeDirFor', 'Remove watch directory for {{name}}', { name: user.username })}
              icon={<RemoveCircle fontSize="inherit" />}
              onPress={() => setConfirm({ kind: 'removeDir', user })}
              isDisabled={dirBusy}
            />
          </>
        ) : null}
        <IconButton
          size="sm"
          label={t('settings.users.editNamed', 'Edit {{name}}', { name: user.username })}
          icon={<Edit fontSize="inherit" />}
          onPress={() => setEditing({ user })}
        />
        <IconButton
          size="sm"
          label={
            isSelf
              ? t('settings.messages.cannotDeleteSelf')
              : t('settings.users.deleteNamed', 'Delete {{name}}', { name: user.username })
          }
          icon={<Delete fontSize="inherit" />}
          onPress={() => setConfirm({ kind: 'delete', user })}
          isDisabled={isSelf}
        />
      </span>
    );
  };

  const columns = useMemo<BoardColumn<UserRow>[]>(() => {
    const cols: BoardColumn<UserRow>[] = [
      { id: 'username', label: t('settings.userManagement.tableHeaders.username'), render: (u) => u.username },
      { id: 'email', hideOnNarrow: true, label: t('settings.userManagement.tableHeaders.email'), render: (u) => u.email },
      {
        id: 'created',
        hideOnNarrow: true,
        label: t('settings.userManagement.tableHeaders.createdAt'),
        mono: true,
        width: 140,
        render: (u) => formatDate(u.created_at),
      },
    ];
    cols.push({
      id: 'status',
      label: t('settings.userManagement.tableHeaders.status', 'Status'),
      render: (u) => (
        <UserStatusCell
          user={u}
          isSelf={u.id === currentUserId}
          isDisabled={statusBusy === u.id}
          onChange={(active) => {
            setStatusBusy(u.id);
            void setUserActive(u, active).finally(() => setStatusBusy(null));
          }}
        />
      ),
    });
    if (perUserWatch) {
      cols.push({ id: 'watch', hideOnNarrow: true, label: t('settings.userManagement.tableHeaders.watchDirectory'), render: watchCell });
    }
    cols.push({ id: 'actions', label: t('settings.userManagement.tableHeaders.actions'), width: perUserWatch ? 200 : 110, render: actions });
    return cols;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [t, perUserWatch, watch.dirs, watch.busy, currentUserId, statusBusy]);

  const runConfirm = async () => {
    if (!confirm) return;
    setConfirming(true);
    try {
      if (confirm.kind === 'delete') await deleteUser(confirm.user);
      else await watch.remove(confirm.user.id);
    } finally {
      setConfirming(false);
      setConfirm(null);
    }
  };

  return (
    <div className={shared.stack}>
      <div className={shared.sectionHead}>
        <p className={shared.sectionIntro}>{t('settings.users.intro', 'People who can sign in to this Readur server.')}</p>
        <Button variant="primary" icon={<Add fontSize="inherit" />} onPress={() => setEditing({ user: null })}>
          {t('settings.userManagement.addUser')}
        </Button>
      </div>
      {loadError ? <Notice tone="danger">{loadError}</Notice> : null}
      {inactiveCount > 0 ? (
        <Notice tone="warning">
          {t('settings.userManagement.pendingApprovalNotice', {
            count: inactiveCount,
            defaultValue: '{{count}} account(s) are disabled or awaiting approval. Enable an account to let that user sign in.',
          })}
        </Notice>
      ) : null}
      <BoardTable
        aria-label={t('settings.userManagement.title')}
        columns={columns}
        rows={users}
        getRowId={(u) => u.id}
        isLoading={isLoading}
        emptyState={<EmptyState headingAs="h3" title={t('settings.users.empty', 'No users yet')} />}
      />
      <UserDialog
        isOpen={editing !== null}
        user={editing?.user ?? null}
        onClose={() => setEditing(null)}
        onSubmit={saveUser}
      />
      <Dialog
        role="alertdialog"
        size="sm"
        isOpen={confirm !== null}
        onOpenChange={(open) => !open && !confirming && setConfirm(null)}
        title={
          confirm?.kind === 'removeDir'
            ? t('settings.userManagement.confirmRemoveDirectory.title')
            : t('settings.users.deleteTitle', 'Delete user')
        }
        actions={
          <>
            <Button variant="ghost" onPress={() => setConfirm(null)} isDisabled={confirming}>
              {t('common.actions.cancel')}
            </Button>
            <Button variant="danger" onPress={() => void runConfirm()} isPending={confirming}>
              {confirm?.kind === 'removeDir'
                ? t('settings.userManagement.confirmRemoveDirectory.removeButton')
                : t('common.actions.delete')}
            </Button>
          </>
        }
      >
        <p>
          {confirm?.kind === 'removeDir'
            ? t('settings.userManagement.confirmRemoveDirectory.message', { username: confirm.user.username })
            : confirm
              ? t('settings.messages.confirmDeleteUser', { username: confirm.user.username, email: confirm.user.email })
              : null}
        </p>
      </Dialog>
    </div>
  );
}
