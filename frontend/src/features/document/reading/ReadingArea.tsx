import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { Tab, TabList, TabPanel, Tabs } from '../../../ui';
import { useMediaQuery } from '../../shell';
import styles from '../DocumentPage.module.css';

/** At this width and up the preview and the text sit side by side. */
export const WIDE_QUERY = '(min-width: 1100px)';

export interface ReadingAreaProps {
  preview: ReactNode;
  text: ReactNode;
  /** Rendered collapsible under the columns on wide screens, as the third tab otherwise. */
  details: (layout: 'wide' | 'tabs') => ReactNode;
  /** Tab to show first on narrow screens. */
  defaultTab?: 'preview' | 'text' | 'details';
}

/** Two columns (preview | text) on wide screens; Preview / Text / Details tabs below 1100px. */
export function ReadingArea({ preview, text, details, defaultTab = 'preview' }: ReadingAreaProps) {
  const { t } = useTranslation();
  const wide = useMediaQuery(WIDE_QUERY);

  if (wide) {
    return (
      <>
        <div className={styles.columns}>
          <section className={styles.previewColumn} aria-label={t('document.tabs.preview', 'Preview')}>
            {preview}
          </section>
          <div className={styles.textColumn}>{text}</div>
        </div>
        {details('wide')}
      </>
    );
  }

  return (
    <Tabs defaultSelectedKey={defaultTab} className={styles.tabs}>
      <TabList aria-label={t('document.tabs.label', 'Document views')}>
        <Tab id="preview">{t('document.tabs.preview', 'Preview')}</Tab>
        <Tab id="text">{t('document.tabs.text', 'Text')}</Tab>
        <Tab id="details">{t('document.tabs.details', 'Details')}</Tab>
      </TabList>
      <TabPanel id="preview">{preview}</TabPanel>
      <TabPanel id="text">{text}</TabPanel>
      <TabPanel id="details">{details('tabs')}</TabPanel>
    </Tabs>
  );
}
