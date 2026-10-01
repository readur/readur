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
}

/** Modal dialog. Traps focus and restores it to the trigger on close. */
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
          <Heading slot="title" className={styles.title}>
            {title}
          </Heading>
          <div className={styles.body}>{children}</div>
          {actions ? <div className={styles.actions}>{actions}</div> : null}
        </RACDialog>
      </Modal>
    </ModalOverlay>
  );
}
