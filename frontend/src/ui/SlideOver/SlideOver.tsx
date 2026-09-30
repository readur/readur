import { useEffect, useLayoutEffect, useRef, type CSSProperties, type ReactNode } from 'react';
import { Dialog as RACDialog, Heading, Modal, ModalOverlay } from 'react-aria-components';
import { useTranslation } from 'react-i18next';
import { IconButton } from '../IconButton';
import { Close } from '../icons';
import { cx } from '../shared/FieldParts';
import styles from './SlideOver.module.css';

export interface SlideOverProps {
  title: ReactNode;
  isOpen: boolean;
  onOpenChange: (isOpen: boolean) => void;
  children?: ReactNode;
  /** Sticky footer, typically `Button`s. */
  footer?: ReactNode;
  /** Panel width on desktop. Default 440px. Always full width below 720px. */
  width?: number | string;
  /**
   * Called for ↑/↓ when focus is not in a text field or a list, so the parent can show the
   * previous or next record without closing the panel.
   */
  onNavigate?: (direction: 'previous' | 'next') => void;
  className?: string;
}

const EDITABLE =
  'input, textarea, select, [contenteditable="true"], [role="listbox"], [role="menu"], [role="grid"], [role="slider"]';

/**
 * Right-anchored detail panel. Focus moves into the panel, Esc closes it and focus returns to the
 * trigger. The parent may swap its content while it is open; focus stays inside the panel.
 */
export function SlideOver({
  title,
  isOpen,
  onOpenChange,
  children,
  footer,
  width,
  onNavigate,
  className,
}: SlideOverProps) {
  const { t } = useTranslation();
  const dialogRef = useRef<HTMLElement>(null);
  const navigateRef = useRef(onNavigate);
  navigateRef.current = onNavigate;

  // When swapped content removes the focused element, keep focus inside the panel.
  useLayoutEffect(() => {
    const dialog = dialogRef.current;
    if (!isOpen || !dialog) return;
    const active = document.activeElement;
    if (!active || active === document.body || !active.isConnected) {
      dialog.focus({ preventScroll: true });
    }
  });

  // RAC's Dialog does not forward keyboard handlers, so listen natively. This also catches keys
  // pressed while the panel itself holds focus.
  useEffect(() => {
    const dialog = dialogRef.current;
    if (!isOpen || !dialog) return undefined;
    const onKeyDown = (e: KeyboardEvent) => {
      const navigate = navigateRef.current;
      if (!navigate || (e.key !== 'ArrowUp' && e.key !== 'ArrowDown')) return;
      if (e.altKey || e.ctrlKey || e.metaKey || e.shiftKey || e.defaultPrevented) return;
      const target = e.target as Element | null;
      if (target?.closest?.(EDITABLE)) return;
      e.preventDefault();
      navigate(e.key === 'ArrowUp' ? 'previous' : 'next');
    };
    dialog.addEventListener('keydown', onKeyDown);
    return () => dialog.removeEventListener('keydown', onKeyDown);
  }, [isOpen]);

  const style =
    width !== undefined
      ? ({ '--slideover-width': typeof width === 'number' ? `${width}px` : width } as CSSProperties)
      : undefined;

  return (
    <ModalOverlay className={styles.overlay} isOpen={isOpen} onOpenChange={onOpenChange} isDismissable>
      <Modal className={cx(styles.panel, className)} style={style}>
        <RACDialog ref={dialogRef} className={styles.dialog}>
          <header className={styles.header}>
            <Heading slot="title" className={styles.title}>
              {title}
            </Heading>
            <IconButton
              size="sm"
              label={t('ui.close', 'Close')}
              icon={<Close fontSize="small" />}
              onPress={() => onOpenChange(false)}
            />
          </header>
          <div className={styles.body}>{children}</div>
          {footer ? <footer className={styles.footer}>{footer}</footer> : null}
        </RACDialog>
      </Modal>
    </ModalOverlay>
  );
}
