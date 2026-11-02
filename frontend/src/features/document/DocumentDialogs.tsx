import { useEffect, useState } from 'react';
import { Trans, useTranslation } from 'react-i18next';
import { documentService } from '../../services/api';
import { Button, Dialog, Skeleton } from '../../ui';
import styles from './DocumentDialogs.module.css';

export interface DeleteDocumentDialogProps {
  filename: string;
  isOpen: boolean;
  isDeleting: boolean;
  onOpenChange: (open: boolean) => void;
  onConfirm: () => void;
}

/** Destructive confirmation before a document is removed. */
export function DeleteDocumentDialog({ filename, isOpen, isDeleting, onOpenChange, onConfirm }: DeleteDocumentDialogProps) {
  const { t } = useTranslation();
  return (
    <Dialog
      role="alertdialog"
      size="sm"
      title={t('document.delete.title', 'Delete this document?')}
      isOpen={isOpen}
      onOpenChange={(open) => !isDeleting && onOpenChange(open)}
      actions={
        <>
          <Button variant="ghost" isDisabled={isDeleting} onPress={() => onOpenChange(false)}>
            {t('document.delete.cancel', 'Cancel')}
          </Button>
          <Button variant="danger" isPending={isDeleting} onPress={onConfirm}>
            {t('document.delete.confirm', 'Delete')}
          </Button>
        </>
      }
    >
      <p className={styles.dialogText}>
        <Trans
          i18nKey="document.delete.message"
          values={{ filename }}
          defaults="<0>{{filename}}</0> and its extracted text, labels, comments and share links will be removed. This can't be undone."
          components={[<strong key="name" />]}
        />
      </p>
    </Dialog>
  );
}

export interface ProcessedImageDialogProps {
  documentId: string;
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
}

/** Shows the cleaned-up image that was fed to OCR. */
export function ProcessedImageDialog({ documentId, isOpen, onOpenChange }: ProcessedImageDialogProps) {
  const { t } = useTranslation();
  const [state, setState] = useState<{ status: 'loading' | 'missing' } | { status: 'ready'; url: string }>({
    status: 'loading',
  });

  useEffect(() => {
    if (!isOpen) return undefined;
    let cancelled = false;
    let url: string | null = null;
    setState({ status: 'loading' });
    documentService
      .getProcessedImage(documentId)
      .then((res) => {
        if (cancelled) return;
        url = URL.createObjectURL(new Blob([res.data], { type: 'image/png' }));
        setState({ status: 'ready', url });
      })
      .catch(() => {
        if (!cancelled) setState({ status: 'missing' });
      });
    return () => {
      cancelled = true;
      if (url) URL.revokeObjectURL(url);
    };
  }, [documentId, isOpen]);

  return (
    <Dialog
      size="lg"
      title={t('document.processed.title', 'Processed image')}
      isOpen={isOpen}
      onOpenChange={onOpenChange}
      actions={<Button onPress={() => onOpenChange(false)}>{t('document.processed.close', 'Close')}</Button>}
    >
      {state.status === 'loading' ? <Skeleton height={320} label={t('document.processed.loading', 'Loading image')} /> : null}
      {state.status === 'missing' ? (
        <p className={styles.dialogText}>{t('document.processed.missing', 'No processed image is stored for this document.')}</p>
      ) : null}
      {state.status === 'ready' ? (
        <figure className={styles.processed}>
          <img src={state.url} alt={t('document.processed.alt', 'Image as prepared for OCR')} />
          <figcaption>{t('document.processed.caption', 'This is the cleaned-up image the OCR engine read.')}</figcaption>
        </figure>
      ) : null}
    </Dialog>
  );
}
