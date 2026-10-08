import type { ReactNode } from 'react';
import { Dialog as RACDialog, Heading, Modal, ModalOverlay } from 'react-aria-components';
import { cx } from '../shared/FieldParts';
import styles from './Dialog.module.css';

export interface DialogProps {
  title: ReactNode;
  children?: ReactNode;
  /** Footer content, typically `Button`s. */
  actions?: ReactNode;
  isOpen?: boolean;
  defaultOpen?: boolean;
  onOpenChange?: (isOpen: boolean) => void;
  size?: 'sm' | 'md' | 'lg';
  /** Close on outside click. Default true. Esc always closes. */
  isDismissable?: boolean;
  /** Use `alertdialog` semantics for destructive confirmations. */
  role?: 'dialog' | 'alertdialog';
  /** Icon shown in an accent tile beside the title. */
  icon?: ReactNode;
  /** Link (or text) on the left of the footer, e.g. "What does this do?". */
  helpLink?: ReactNode;
}

/** Modal dialog, anchored near the top. Traps focus and restores it to the trigger on close. */
export function Dialog({
  title,
  children,
  actions,
  isOpen,
  defaultOpen,
  onOpenChange,
  size = 'md',
  isDismissable = true,
  role = 'dialog',
  icon,
  helpLink,
}: DialogProps) {
  return (
    <ModalOverlay
      className={styles.overlay}
      isOpen={isOpen}
      defaultOpen={defaultOpen}
      onOpenChange={onOpenChange}
      isDismissable={isDismissable}
    >
      <Modal className={cx(styles.modal, styles[size])}>
        <RACDialog className={styles.dialog} role={role}>
          <div className={styles.header}>
            {icon ? (
              <span className={styles.iconTile} data-icon-tile="" aria-hidden="true">
                {icon}
              </span>
            ) : null}
            <Heading slot="title" className={styles.title}>
              {title}
            </Heading>
          </div>
          <div className={styles.body}>{children}</div>
          {actions || helpLink ? (
            <div className={styles.actions}>
              {helpLink ? <span className={styles.help}>{helpLink}</span> : null}
              {actions}
            </div>
          ) : null}
        </RACDialog>
      </Modal>
    </ModalOverlay>
  );
}
