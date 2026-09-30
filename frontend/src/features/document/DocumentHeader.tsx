import type { Key } from 'react-aria-components';
import { useTranslation } from 'react-i18next';
import { Link } from 'react-router-dom';
import type { Document } from '../../services/api';
import { Button, IconButton, Menu, MenuItem, MenuTrigger, TruncatedText } from '../../ui';
import { ChatBubbleOutline, Delete, Download, MoreVert, PhotoFilter, Refresh, Share } from '../../ui/icons';
import { PageHeader } from '../shell';
import { fileKind, ocrState } from './format';
import styles from './DocumentPage.module.css';

export interface DocumentHeaderProps {
  document: Document;
  libraryHref: string;
  isRetrying: boolean;
  isDownloading: boolean;
  isCommentsOpen: boolean;
  onDownload: () => void;
  onShare: () => void;
  onRetry: () => void;
  onDelete: () => void;
  onViewProcessed: () => void;
  onToggleComments: () => void;
}

/** Breadcrumb, filename title and the page's actions. */
export function DocumentHeader({
  document: doc,
  libraryHref,
  isRetrying,
  isDownloading,
  isCommentsOpen,
  onDownload,
  onShare,
  onRetry,
  onDelete,
  onViewProcessed,
  onToggleComments,
}: DocumentHeaderProps) {
  const { t } = useTranslation();
  const failed = ocrState(doc.ocr_status) === 'failed';
  const hasProcessedImage = fileKind(doc.mime_type) === 'image';

  const onMore = (key: Key) => {
    if (key === 'delete') onDelete();
    if (key === 'processed') onViewProcessed();
  };

  return (
    <>
      <nav aria-label={t('document.breadcrumb', 'Breadcrumb')} className={styles.breadcrumb}>
        <ol>
          <li>
            <Link to={libraryHref}>{t('document.library', 'Library')}</Link>
          </li>
          <li aria-current="page" className={styles.crumbCurrent}>
            <TruncatedText>{doc.original_filename}</TruncatedText>
          </li>
        </ol>
      </nav>
      <PageHeader
        className={styles.pageHeader}
        title={<TruncatedText className={styles.title}>{doc.original_filename}</TruncatedText>}
        actions={
          <>
            {failed ? (
              <Button variant="primary" icon={<Refresh fontSize="inherit" />} isPending={isRetrying} onPress={onRetry}>
                {t('document.actions.retry', 'Retry OCR')}
              </Button>
            ) : null}
            <Button icon={<Download fontSize="inherit" />} isPending={isDownloading} onPress={onDownload}>
              {t('document.actions.download', 'Download')}
            </Button>
            <Button icon={<Share fontSize="inherit" />} onPress={onShare}>
              {t('document.actions.share', 'Share')}
            </Button>
            <Button
              icon={<ChatBubbleOutline fontSize="inherit" />}
              aria-pressed={isCommentsOpen}
              onPress={onToggleComments}
            >
              {t('document.actions.comments', 'Comments')}
            </Button>
            <MenuTrigger>
              <IconButton label={t('document.actions.more', 'More actions')} icon={<MoreVert fontSize="inherit" />} />
              <Menu aria-label={t('document.actions.more', 'More actions')} onAction={onMore}>
                {hasProcessedImage ? (
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
          </>
        }
      />
    </>
  );
}
