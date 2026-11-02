import { Fragment } from 'react';
import { Toolbar, type Key } from 'react-aria-components';
import { useTranslation } from 'react-i18next';
import type { Document, OcrResponse } from '../../../services/api';
import { Button, IconButton, Menu, MenuItem, MenuTrigger, ProgressBar, StatusMark } from '../../../ui';
import { Delete, Download, MoreVert, PhotoFilter, Refresh, Share } from '../../../ui/icons';
import { fileKind, ocrState } from '../format';
import { metaParts } from '../meta';
import styles from './DocumentDrawer.module.css';

export interface DrawerHeadProps {
  document: Document;
  ocr: OcrResponse | null;
  sourceName: string | null;
}

/** The document's status and facts on one line. The drawer's heading names it. */
export function DrawerHead({ document: doc, ocr, sourceName }: DrawerHeadProps) {
  const { t, i18n } = useTranslation();
  const state = ocrState(doc.ocr_status);
  const total = doc.ocr_progress_total ?? 0;
  const current = doc.ocr_progress_current ?? 0;
  const hasProgress = state === 'processing' && total > 0;
  const facts = metaParts({ document: doc, ocr, sourceName, t, lng: i18n.language });

  return (
    <div className={styles.head}>
      <div className={styles.factsRow} role="group" aria-label={t('document.pass.label', 'Document summary')}>
        <StatusMark state={state} progress={hasProgress ? { current, total } : undefined} />
        {hasProgress ? (
          <ProgressBar
            className={styles.progress}
            label={t('document.pass.progress', 'OCR progress')}
            value={(Math.min(current, total) / total) * 100}
          />
        ) : null}
        <p className={styles.facts}>
          {facts.map((fact, i) => (
            <Fragment key={i}>
              {i > 0 ? ' ' : null}
              <span className={styles.fact}>{fact}</span>
            </Fragment>
          ))}
        </p>
      </div>
    </div>
  );
}

export interface DrawerActionsProps {
  document: Document;
  isRetrying: boolean;
  isDownloading: boolean;
  onDownload: () => void;
  onShare: () => void;
  onRetry: () => void;
  onDelete: () => void;
  onViewProcessed: () => void;
}

/** The document's actions, in a bar along the bottom of the drawer. */
export function DrawerActions({
  document: doc,
  isRetrying,
  isDownloading,
  onDownload,
  onShare,
  onRetry,
  onDelete,
  onViewProcessed,
}: DrawerActionsProps) {
  const { t } = useTranslation();
  const failed = ocrState(doc.ocr_status) === 'failed';

  const onMore = (key: Key) => {
    if (key === 'delete') onDelete();
    if (key === 'processed') onViewProcessed();
  };

  return (
    <Toolbar className={styles.actionBar} aria-label={t('document.drawer.actions', 'Document actions')}>
      <Button variant="primary" size="sm" icon={<Download fontSize="inherit" />} isPending={isDownloading} onPress={onDownload}>
        {t('document.actions.download', 'Download')}
      </Button>
      <Button size="sm" icon={<Share fontSize="inherit" />} onPress={onShare}>
        {t('document.actions.share', 'Share')}
      </Button>
      {failed ? (
        <Button size="sm" icon={<Refresh fontSize="inherit" />} isPending={isRetrying} onPress={onRetry}>
          {t('document.actions.retry', 'Retry OCR')}
        </Button>
      ) : null}
      <MenuTrigger>
        <IconButton size="sm" label={t('document.actions.more', 'More actions')} icon={<MoreVert fontSize="inherit" />} />
        <Menu aria-label={t('document.actions.more', 'More actions')} onAction={onMore}>
          {fileKind(doc.mime_type) === 'image' ? (
            <MenuItem id="processed" textValue={t('document.actions.processedImage', 'View processed image')}>
              <PhotoFilter fontSize="inherit" aria-hidden="true" />
              {t('document.actions.processedImage', 'View processed image')}
            </MenuItem>
          ) : null}
          <MenuItem id="delete" danger textValue={t('document.actions.delete', 'Delete document')}>
            <Delete fontSize="inherit" aria-hidden="true" />
            {t('document.actions.delete', 'Delete document')}
          </MenuItem>
        </Menu>
      </MenuTrigger>
    </Toolbar>
  );
}
