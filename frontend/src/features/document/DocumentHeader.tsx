import { Fragment, type Ref } from 'react';
import type { Key } from 'react-aria-components';
import { useTranslation } from 'react-i18next';
import { Link } from 'react-router-dom';
import type { Document, OcrResponse } from '../../services/api';
import { Button, IconButton, Menu, MenuItem, MenuTrigger, StatusMark, TruncatedText } from '../../ui';
import { ChatBubbleOutline, Delete, Download, MoreVert, PhotoFilter, Refresh, Share } from '../../ui/icons';
import { fileKind, ocrState } from './format';
import { metaParts } from './meta';
import styles from './DocumentHeader.module.css';

export interface DocumentHeaderProps {
  document: Document;
  ocr: OcrResponse | null;
  sourceName: string | null;
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
  ref?: Ref<HTMLElement>;
}

/** Breadcrumb, the filename, one line of facts and the page's actions. */
export function DocumentHeader({
  document: doc,
  ocr,
  sourceName,
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
  ref,
}: DocumentHeaderProps) {
  const { t, i18n } = useTranslation();
  const state = ocrState(doc.ocr_status);
  const failed = state === 'failed';
  const hasProcessedImage = fileKind(doc.mime_type) === 'image';
  const total = doc.ocr_progress_total ?? 0;
  const current = doc.ocr_progress_current ?? 0;
  const hasProgress = state === 'processing' && total > 0;
  const percent = hasProgress ? Math.round((Math.min(current, total) / total) * 100) : 0;
  const facts = metaParts({ document: doc, ocr, sourceName, t, lng: i18n.language });

  const onMore = (key: Key) => {
    if (key === 'delete') onDelete();
    if (key === 'processed') onViewProcessed();
  };

  return (
    <header ref={ref} className={styles.header}>
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

      <div className={styles.titleRow}>
        <h1 className={styles.title}>
          <TruncatedText lines={2}>{doc.original_filename}</TruncatedText>
        </h1>
        <div className={styles.actions}>
          {failed ? (
            <Button className={styles.action} icon={<Refresh fontSize="inherit" />} isPending={isRetrying} onPress={onRetry}>
              {t('document.actions.retry', 'Retry OCR')}
            </Button>
          ) : null}
          <Button
            className={styles.action}
            variant="primary"
            icon={<Download fontSize="inherit" />}
            isPending={isDownloading}
            onPress={onDownload}
          >
            {t('document.actions.download', 'Download')}
          </Button>
          <Button className={styles.action} icon={<Share fontSize="inherit" />} onPress={onShare}>
            <span className={styles.actionLabel}>{t('document.actions.share', 'Share')}</span>
          </Button>
          <Button
            className={styles.action}
            variant={isCommentsOpen ? 'secondary' : 'ghost'}
            icon={<ChatBubbleOutline fontSize="inherit" />}
            aria-pressed={isCommentsOpen}
            onPress={onToggleComments}
          >
            <span className={styles.actionLabel}>{t('document.actions.comments', 'Comments')}</span>
          </Button>
          <MenuTrigger>
            <IconButton
              className={styles.action}
              label={t('document.actions.more', 'More actions')}
              icon={<MoreVert fontSize="inherit" />}
            />
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
        </div>
      </div>

      <div className={styles.metaRow} role="group" aria-label={t('document.pass.label', 'Document summary')}>
        <span className={styles.status}>
          <StatusMark state={state} progress={hasProgress ? { current, total } : undefined} />
          {hasProgress ? (
            <span
              className={styles.progress}
              role="progressbar"
              aria-label={t('document.pass.progress', 'OCR progress')}
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={percent}
            >
              <span className={styles.progressFill} style={{ width: `${percent}%` }} />
            </span>
          ) : null}
        </span>
        <p className={styles.meta}>
          {facts.map((fact, i) => (
            <Fragment key={i}>
              {i > 0 ? ' ' : null}
              <span className={styles.fact}>{fact}</span>
            </Fragment>
          ))}
        </p>
      </div>
    </header>
  );
}
