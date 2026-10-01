import type { ReactNode } from 'react';
import { Dialog as RACDialog, Modal, ModalOverlay } from 'react-aria-components';
import { useTranslation } from 'react-i18next';
import { IconButton } from '../../ui';
import { Close } from '../../ui/icons';
import styles from './AppShell.module.css';

export interface MobileDrawerProps {
  isOpen: boolean;
  onOpenChange: (isOpen: boolean) => void;
  children: ReactNode;
}

/** The sidebar as a left-hand drawer on phones and small tablets. Esc or the scrim closes it. */
export function MobileDrawer({ isOpen, onOpenChange, children }: MobileDrawerProps) {
  const { t } = useTranslation();
  return (
    <ModalOverlay className={styles.drawerOverlay} isOpen={isOpen} onOpenChange={onOpenChange} isDismissable>
      <Modal className={styles.drawer}>
        <RACDialog className={styles.drawerDialog} aria-label={t('shell.menu.label', 'Menu')}>
          {children}
          {/* After the content so it paints above it without a z-index. */}
          <div className={styles.drawerClose}>
            <IconButton
              size="sm"
              label={t('shell.menu.close', 'Close menu')}
              icon={<Close fontSize="small" />}
              onPress={() => onOpenChange(false)}
            />
          </div>
        </RACDialog>
      </Modal>
    </ModalOverlay>
  );
}
