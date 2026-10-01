import { useTranslation } from 'react-i18next';
import { buildExampleSyncUrl, type UrlPart, type UrlPreviewInput } from './sourceFormModel';
import styles from './SourceForm.module.css';

/** Example of the first file URL a sync would fetch, with each part named in a legend. */
export function UrlPreview({ form }: { form: UrlPreviewInput }) {
  const { t } = useTranslation();
  const url = buildExampleSyncUrl(form);
  if (!url) return null;

  const legend = (type: UrlPart['type']) => {
    switch (type) {
      case 'server':
        return t('intake.form.preview.server', 'Server URL');
      case 'path':
        return form.source_type === 'webdav'
          ? t('intake.form.preview.webdavPath', 'WebDAV path')
          : t('intake.form.preview.bucketPrefix', 'Bucket / prefix');
      case 'folder':
        return t('intake.form.preview.folder', 'Watch folder');
      case 'file':
      default:
        return t('intake.form.preview.file', 'Example file');
    }
  };
  const types = Array.from(new Set(url.parts.map((p) => p.type)));

  return (
    <figure className={styles.preview} aria-label={t('intake.form.preview.title', 'Example sync URL')}>
      <figcaption className={styles.previewTitle}>{t('intake.form.preview.title', 'Example sync URL')}</figcaption>
      <output className={styles.previewUrl}>
        {url.parts.map((part, i) =>
          part.text ? (
            <span key={i} className={styles.previewPart} data-part={part.type}>
              {part.text}
            </span>
          ) : null,
        )}
      </output>
      <ul className={styles.previewLegend}>
        {types.map((type) => (
          <li key={type}>
            <span className={styles.previewPart} data-part={type}>
              {legend(type)}
            </span>
          </li>
        ))}
      </ul>
    </figure>
  );
}
