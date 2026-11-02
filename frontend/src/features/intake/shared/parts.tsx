import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { Button, ChangeTag, Dialog, Notice as UiNotice, ProgressBar } from '../../../ui';
import type { LitReason } from '../../board/litStore';
import { litTagOf } from '../../board/litTag';
import styles from './Shared.module.css';

export { styles as sharedStyles };

export type NoticeTone = 'info' | 'danger' | 'ok';

export interface NoticeProps {
  tone?: NoticeTone;
  title?: ReactNode;
  children?: ReactNode;
  /** Announce the notice when it appears. `alert` for failures, `status` for results. */
  live?: 'alert' | 'status';
}

/** Inline message; the tone is carried by an icon and a hidden word as well as the colour. */
export function Notice({ tone = 'info', title, children, live }: NoticeProps) {
  const { t } = useTranslation();
  const word =
    tone === 'danger'
      ? t('intake.notice.danger', 'Error:')
      : tone === 'ok'
        ? t('intake.notice.ok', 'Done:')
        : t('intake.notice.info', 'Note:');
  return (
    <UiNotice tone={tone} title={title} live={live ?? 'off'} prefix={word}>
      {children}
    </UiNotice>
  );
}

/** Visible marker for rows that changed since the user last looked. */
export function ChangedTag({ reason }: { reason?: LitReason | string }) {
  const { t } = useTranslation();
  return (
    <ChangeTag>{litTagOf(reason) === 'new' ? t('intake.tag.new', 'New') : t('intake.tag.changed', 'Changed')}</ChangeTag>
  );
}

/** Row name cell: text plus the tag when the row is marked. */
export function NameCell({ name, tag }: { name: ReactNode; tag?: LitReason | string | null }) {
  return (
    <span className={styles.name}>
      <span className={styles.nameText}>{name}</span>
      {tag ? <ChangedTag reason={tag} /> : null}
    </span>
  );
}

/** Mono percentage plus a thin bar (the ui ProgressBar). */
export function ProgressCell({ value, label }: { value: number; label: string }) {
  return <ProgressBar value={value} label={label} showValue />;
}

export interface ConfirmDialogProps {
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
  title: ReactNode;
  children?: ReactNode;
  confirmLabel: string;
  onConfirm: () => void | Promise<void>;
  tone?: 'danger' | 'primary';
  isPending?: boolean;
}

/** Confirmation for anything destructive or expensive. Cancel is the default focus target. */
export function ConfirmDialog({
  isOpen,
  onOpenChange,
  title,
  children,
  confirmLabel,
  onConfirm,
  tone = 'danger',
  isPending,
}: ConfirmDialogProps) {
  const { t } = useTranslation();
  return (
    <Dialog
      isOpen={isOpen}
      onOpenChange={onOpenChange}
      title={title}
      size="sm"
      role="alertdialog"
      actions={
        <>
          <Button variant="ghost" onPress={() => onOpenChange(false)} isDisabled={isPending}>
            {t('intake.actions.cancel', 'Cancel')}
          </Button>
          <Button variant={tone} onPress={() => void onConfirm()} isPending={isPending}>
            {confirmLabel}
          </Button>
        </>
      }
    >
      {children}
    </Dialog>
  );
}

/** A labelled list of removable values (folders, extensions). */
export function ChipList({
  items,
  label,
  onRemove,
  removeLabel,
}: {
  items: string[];
  label: string;
  onRemove?: (item: string) => void;
  removeLabel?: (item: string) => string;
}) {
  if (items.length === 0) return null;
  return (
    <ul className={styles.chips} aria-label={label}>
      {items.map((item) => (
        <li key={item} className={styles.chip}>
          <span className={styles.chipText}>{item}</span>
          {onRemove ? (
            <Button
              variant="ghost"
              size="sm"
              aria-label={removeLabel ? removeLabel(item) : item}
              onPress={() => onRemove(item)}
            >
              ×
            </Button>
          ) : null}
        </li>
      ))}
    </ul>
  );
}
