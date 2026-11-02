import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { Key } from 'react-aria-components';
import { Tab, TabList, TabPanel, Tabs } from '../../../ui';
import { Notice } from '../shared/Notice';
import shared from '../shared/shared.module.css';
import { SearchPanel, UploadPanel } from './DebugPanels';
import { Diagnostics } from './Diagnostics';
import { PipelineStepper } from './PipelineStepper';
import { STATE_GLYPH, stepState } from './types';
import { useDebugSession } from './useDebugSession';
import styles from './Debug.module.css';

type TabId = 'upload' | 'search' | 'results';

/** Admin OCR pipeline debugger: upload or look up a document, then read its diagnostics. */
export default function DebugSection() {
  const { t } = useTranslation();
  const session = useDebugSession();
  const { debugInfo, error } = session;
  const [tab, setTab] = useState<TabId>('upload');

  // New results open the results tab; losing them falls back to the first tab.
  useEffect(() => {
    if (debugInfo) setTab('results');
  }, [debugInfo]);
  useEffect(() => {
    if (!debugInfo && tab === 'results') setTab('upload');
  }, [debugInfo, tab]);

  const overall = debugInfo ? stepState(debugInfo.overall_status, debugInfo.overall_status === 'success') : 'other';

  return (
    <div className={shared.stack}>
      <p className={shared.sectionIntro}>{t('debug.subtitle')}</p>
      <Tabs selectedKey={tab} onSelectionChange={(k: Key) => setTab(k as TabId)}>
        <TabList aria-label={t('debug.title')}>
          <Tab id="upload">{t('debug.tabs.uploadAndDebug')}</Tab>
          <Tab id="search">{t('debug.tabs.searchExisting')}</Tab>
          {debugInfo ? <Tab id="results">{t('debug.tabs.debugResults')}</Tab> : null}
        </TabList>
        <TabPanel id="upload" className={styles.tabPanel}>
          <UploadPanel session={session} onShowResults={() => setTab('results')} />
        </TabPanel>
        <TabPanel id="search" className={styles.tabPanel}>
          <SearchPanel session={session} />
        </TabPanel>
        {debugInfo ? (
          <TabPanel id="results" className={styles.tabPanel}>
            <div className={shared.stack}>
              <header className={styles.resultHead}>
                <h3 className={styles.panelTitle}>{t('debug.document.title', { filename: debugInfo.filename })}</h3>
                <p className={styles.overall} data-state={overall}>
                  <span aria-hidden="true">{STATE_GLYPH[overall]} </span>
                  {t('debug.document.status', { status: debugInfo.overall_status })}
                </p>
                <p className={shared.meta}>
                  {t('debug.document.debugRunAt', { timestamp: new Date(debugInfo.debug_timestamp).toLocaleString() })}
                </p>
              </header>
              <PipelineStepper steps={debugInfo.pipeline_steps || []} />
              <Diagnostics info={debugInfo} />
            </div>
          </TabPanel>
        ) : null}
      </Tabs>
      {error ? (
        <Notice tone="danger" title={t('debug.errors.debugError')}>
          {error}
        </Notice>
      ) : null}
    </div>
  );
}
