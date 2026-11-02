import { useTranslation } from 'react-i18next';
import { Button, Dialog } from '../../../ui';
import { SharedLinksManager } from './SharedLinksManager';

export interface SharedLinksDialogProps {
  documentId: string;
  isOpen: boolean;
  onOpenChange: (isOpen: boolean) => void;
  /** Shown in the dialog title when given. */
  filename?: string;
}

/**
 * Modal for sharing one document: create a link (optional password, expiry, view limit) and copy
 * or revoke existing links. Reusable anywhere a document id is at hand.
 */
export function SharedLinksDialog({ documentId, isOpen, onOpenChange, filename }: SharedLinksDialogProps) {
  const { t } = useTranslation();
  const title = filename
    ? t('document.share.titleNamed', { name: filename, defaultValue: 'Share {{name}}' })
    : t('document.share.title', 'Share document');
  return (
    <Dialog
      title={title}
      isOpen={isOpen}
      onOpenChange={onOpenChange}
      size="md"
      actions={
        <Button variant="secondary" onPress={() => onOpenChange(false)}>
          {t('document.share.done', 'Done')}
        </Button>
      }
    >
      {isOpen ? <SharedLinksManager documentId={documentId} /> : null}
    </Dialog>
  );
}

export default SharedLinksDialog;
