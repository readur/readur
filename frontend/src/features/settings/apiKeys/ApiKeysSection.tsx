import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { BoardTable, Button, EmptyState, IconButton, type BoardColumn } from '../../../ui';
import { Add, Delete, Key } from '../../../ui/icons';
import type { ApiKey } from '../../../services/api';
import { Tag } from '../shared/Facts';
import { Notice } from '../shared/Notice';
import shared from '../shared/shared.module.css';
import { CreateKeyDialog, RevealKeyDialog, RevokeKeyDialog } from './ApiKeyDialogs';
import styles from './ApiKeys.module.css';
import { useApiKeys } from './useApiKeys';

function formatDate(value: string | null): string {
  if (!value) return '—';
  return new Date(value).toLocaleString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

/** Personal API keys: create (plaintext shown once), list and revoke with confirmation. */
export default function ApiKeysSection() {
  const { t } = useTranslation();
  const { keys, isLoading, error, setError, create, revoke } = useApiKeys();
  const [createOpen, setCreateOpen] = useState(false);
  const [revealed, setRevealed] = useState<{ name: string; plaintext: string } | null>(null);
  const [revokeTarget, setRevokeTarget] = useState<ApiKey | null>(null);

  const columns = useMemo<BoardColumn<ApiKey>[]>(
    () => [
      { id: 'name', label: t('settings.apiKeys.name', 'Name'), render: (k) => k.name },
      {
        id: 'prefix',
        label: t('settings.apiKeys.prefix', 'Prefix'),
        width: 150,
        render: (k) => <span className={styles.prefix}>{k.key_prefix}…</span>,
      },
      {
        id: 'status',
        label: t('settings.apiKeys.status', 'Status'),
        width: 120,
        render: (k) =>
          k.revoked_at ? (
            <Tag>
              <span aria-hidden="true">—&nbsp;</span>
              {t('settings.apiKeys.revoked', 'Revoked')}
            </Tag>
          ) : k.is_expired ? (
            <Tag>
              <span aria-hidden="true">◆&nbsp;</span>
              {t('settings.apiKeys.expired', 'Expired')}
            </Tag>
          ) : (
            <Tag tone="ok">
              <span aria-hidden="true">■&nbsp;</span>
              {t('settings.apiKeys.active', 'Active')}
            </Tag>
          ),
      },
      { id: 'lastUsed', label: t('settings.apiKeys.lastUsed', 'Last used'), mono: true, render: (k) => formatDate(k.last_used_at) },
      {
        id: 'expires',
        label: t('settings.apiKeys.expires', 'Expires'),
        mono: true,
        render: (k) => (k.expires_at ? formatDate(k.expires_at) : t('settings.apiKeys.never', 'Never')),
      },
      { id: 'created', label: t('settings.apiKeys.created', 'Created'), mono: true, render: (k) => formatDate(k.created_at) },
      {
        id: 'actions',
        label: t('settings.userManagement.tableHeaders.actions'),
        width: 72,
        render: (k) => (
          <IconButton
            size="sm"
            label={
              k.revoked_at
                ? t('settings.apiKeys.alreadyRevoked', 'Already revoked')
                : t('settings.apiKeys.revokeNamed', 'Revoke {{name}}', { name: k.name })
            }
            icon={<Delete fontSize="inherit" />}
            isDisabled={Boolean(k.revoked_at)}
            onPress={() => setRevokeTarget(k)}
          />
        ),
      },
    ],
    [t],
  );

  return (
    <div className={shared.stack}>
      <div className={shared.sectionHead}>
        <p className={shared.sectionIntro}>
          {t(
            'settings.apiKeys.intro',
            'Use an API key in the Authorization: Bearer header to authenticate scripts and integrations. Keys carry your full account permissions — treat them like passwords.',
          )}
        </p>
        <Button variant="primary" icon={<Add fontSize="inherit" />} onPress={() => setCreateOpen(true)}>
          {t('settings.apiKeys.create', 'Create API key')}
        </Button>
      </div>
      {error ? (
        <Notice tone="danger" onDismiss={() => setError(null)}>
          {error}
        </Notice>
      ) : null}
      <BoardTable
        aria-label={t('settings.sections.apiKeys', 'API keys')}
        columns={columns}
        rows={keys}
        getRowId={(k) => k.id}
        isLoading={isLoading}
        density="compact"
        emptyState={
          <EmptyState
            headingAs="h3"
            icon={<Key fontSize="inherit" />}
            title={t('settings.apiKeys.empty', "You don't have any API keys yet.")}
          />
        }
      />
      <CreateKeyDialog
        isOpen={createOpen}
        onClose={() => setCreateOpen(false)}
        onCreate={async (name, expiration) => {
          const result = await create(name, expiration);
          setCreateOpen(false);
          setRevealed(result);
        }}
      />
      <RevealKeyDialog revealed={revealed} onDone={() => setRevealed(null)} />
      <RevokeKeyDialog
        target={revokeTarget}
        onCancel={() => setRevokeTarget(null)}
        onRevoke={async (key) => {
          await revoke(key);
          setRevokeTarget(null);
        }}
      />
    </div>
  );
}
