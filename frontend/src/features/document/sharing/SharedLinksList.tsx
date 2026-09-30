import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { sharedLinksService, type SharedLinkData } from '../../../services/api';
import { Button, IconButton, useToast } from '../../../ui';
import { ContentCopy, LinkOff } from '../../../ui/icons';
import { formatRelativeTime } from '../../../lib/relativeTime';
import { apiErrorMessage, formatDateTime } from '../format';
import styles from './Sharing.module.css';

export interface SharedLinksListProps {
  links: SharedLinkData[];
  onRevoked: () => void;
}

type LinkStatus = 'active' | 'expired' | 'revoked';

const GLYPH: Record<LinkStatus, string> = { active: '■', expired: '◆', revoked: '—' };

function statusOf(link: SharedLinkData): LinkStatus {
  if (link.is_revoked) return 'revoked';
  if (link.is_expired) return 'expired';
  return 'active';
}

function shortToken(token: string): string {
  return token.length <= 12 ? token : `${token.slice(0, 6)}…${token.slice(-4)}`;
}

/** Existing share links with copy and a confirm-in-place revoke. */
export function SharedLinksList({ links, onRevoked }: SharedLinksListProps) {
  const { t, i18n } = useTranslation();
  const toast = useToast();
  const [confirming, setConfirming] = useState<string | null>(null);
  const [revoking, setRevoking] = useState(false);

  const statusWord: Record<LinkStatus, string> = {
    active: t('document.share.active', 'Active'),
    expired: t('document.share.expired', 'Expired'),
    revoked: t('document.share.revoked', 'Revoked'),
  };

  const copy = async (link: SharedLinkData) => {
    try {
      await navigator.clipboard.writeText(link.url);
      toast.show({ title: t('document.share.copied', 'Link copied'), tone: 'success' });
    } catch {
      toast.show({ title: t('document.share.copyFailed', "Couldn't copy the link"), tone: 'danger' });
    }
  };

  const revoke = async (link: SharedLinkData) => {
    setRevoking(true);
    try {
      await sharedLinksService.revoke(link.id);
      setConfirming(null);
      toast.show({ title: t('document.share.revokedToast', 'Link revoked'), tone: 'success' });
      onRevoked();
    } catch (err) {
      toast.show({
        title: t('document.share.revokeFailed', "Couldn't revoke the link"),
        description: apiErrorMessage(err) ?? undefined,
        tone: 'danger',
      });
    } finally {
      setRevoking(false);
    }
  };

  if (links.length === 0) {
    return <p className={styles.hint}>{t('document.share.none', 'No share links yet.')}</p>;
  }

  return (
    <ul className={styles.links} aria-label={t('document.share.existing', 'Existing links')}>
      {links.map((link) => {
        const status = statusOf(link);
        const views = link.max_views != null ? `${link.view_count} / ${link.max_views}` : String(link.view_count);
        return (
          <li key={link.id} className={styles.link}>
            <div className={styles.linkMain}>
              <span className={styles.token} title={link.token}>
                {shortToken(link.token)}
              </span>
              <span className={styles.status} data-status={status}>
                <span aria-hidden="true">{GLYPH[status]}</span> {statusWord[status]}
              </span>
            </div>
            <dl className={styles.linkMeta}>
              <div>
                <dt>{t('document.share.views', 'Views')}</dt>
                <dd>{views}</dd>
              </div>
              <div>
                <dt>{t('document.share.expiresShort', 'Expires')}</dt>
                <dd title={link.expires_at ? formatDateTime(link.expires_at, i18n.language) : undefined}>
                  {link.expires_at
                    ? formatRelativeTime(link.expires_at, { locale: i18n.language })
                    : t('document.share.never', 'Never')}
                </dd>
              </div>
              <div>
                <dt>{t('document.share.createdShort', 'Created')}</dt>
                <dd title={formatDateTime(link.created_at, i18n.language)}>
                  {formatRelativeTime(link.created_at, { locale: i18n.language })}
                </dd>
              </div>
              {link.has_password ? (
                <div>
                  <dt>{t('document.share.protected', 'Password')}</dt>
                  <dd>{t('document.details.yes', 'Yes')}</dd>
                </div>
              ) : null}
            </dl>
            {confirming === link.id ? (
              <div className={styles.confirm} role="group" aria-label={t('document.share.confirmRevoke', 'Revoke this link?')}>
                <span>{t('document.share.confirmRevokeText', 'Revoke this link? It stops working for everyone who has it.')}</span>
                <Button size="sm" variant="ghost" onPress={() => setConfirming(null)} isDisabled={revoking}>
                  {t('document.share.keep', 'Keep')}
                </Button>
                <Button size="sm" variant="danger" isPending={revoking} onPress={() => revoke(link)}>
                  {t('document.share.revoke', 'Revoke')}
                </Button>
              </div>
            ) : (
              <div className={styles.linkActions}>
                <IconButton
                  size="sm"
                  label={t('document.share.copy', 'Copy link')}
                  icon={<ContentCopy fontSize="inherit" />}
                  isDisabled={status !== 'active'}
                  onPress={() => copy(link)}
                />
                <IconButton
                  size="sm"
                  label={t('document.share.revokeLink', 'Revoke link')}
                  icon={<LinkOff fontSize="inherit" />}
                  isDisabled={status === 'revoked'}
                  onPress={() => setConfirming(link.id)}
                />
              </div>
            )}
          </li>
        );
      })}
    </ul>
  );
}
