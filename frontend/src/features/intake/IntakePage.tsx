import type { ComponentType } from 'react';
import { useSearchParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import type { Key } from 'react-aria-components';
import { Tab, TabList, TabPanel, Tabs } from '../../ui';
import { PageHeader } from '../shell';
import { AttentionSection } from './attention/AttentionSection';
import { ConnectionsSection } from './connections/ConnectionsSection';
import { IgnoredSection } from './ignored/IgnoredSection';
import { sharedStyles } from './shared/parts';
import { UploadSection } from './upload/UploadSection';
import { useIntakeSummary } from './useIntakeSummary';
import { WatchSection } from './watch/WatchSection';
import styles from './Intake.module.css';

export const INTAKE_SECTION_IDS = ['upload', 'connections', 'watch', 'attention', 'ignored'] as const;
export type IntakeSectionId = (typeof INTAKE_SECTION_IDS)[number];
export const DEFAULT_INTAKE_SECTION: IntakeSectionId = 'upload';

const PANELS: Record<IntakeSectionId, ComponentType> = {
  upload: UploadSection,
  connections: ConnectionsSection,
  watch: WatchSection,
  attention: AttentionSection,
  ignored: IgnoredSection,
};

export function parseSection(value: string | null): IntakeSectionId {
  return (INTAKE_SECTION_IDS as readonly string[]).includes(value ?? '') ? (value as IntakeSectionId) : DEFAULT_INTAKE_SECTION;
}

/** Intake: add documents, check connections, and clear what needs attention. */
export default function IntakePage() {
  const { t } = useTranslation();
  const [params, setParams] = useSearchParams();
  const section = parseSection(params.get('section'));
  const summary = useIntakeSummary(section);

  const labels: Record<IntakeSectionId, string> = {
    upload: t('intake.sections.upload', 'Add documents'),
    connections: t('intake.sections.connections', 'Connections'),
    watch: t('intake.sections.watch', 'Watch folder'),
    attention: t('intake.sections.attention', 'Needs attention'),
    ignored: t('intake.sections.ignored', 'Ignored'),
  };

  const select = (key: Key) => {
    const next = parseSection(String(key));
    if (next === section) return;
    // Filters that belong to one section (e.g. the Ignored source filter) don't carry over.
    setParams({ section: next });
  };

  const meta = [
    summary.connections !== null
      ? t('intake.meta.connections', '{{count}} connections', { count: summary.connections })
      : null,
    summary.attention !== null ? t('intake.meta.attention', '{{count}} need attention', { count: summary.attention }) : null,
    summary.processing !== null ? t('intake.meta.processing', '{{count}} processing', { count: summary.processing }) : null,
  ].filter(Boolean);

  return (
    <div className={styles.page}>
      <PageHeader
        title={t('intake.title', 'Intake')}
        meta={meta.length > 0 ? <span>{meta.join(' · ')}</span> : undefined}
      />
      <Tabs selectedKey={section} onSelectionChange={select}>
        <div className={styles.tabScroller}>
          <TabList aria-label={t('intake.sections.label', 'Intake sections')} className={styles.tabList}>
            {INTAKE_SECTION_IDS.map((id) => (
              <Tab key={id} id={id}>
                {labels[id]}
                {id === 'attention' && summary.attention ? (
                  <span className={sharedStyles.badge}>
                    <span className={sharedStyles.visuallyHidden}>, </span>
                    {summary.attention}
                  </span>
                ) : null}
              </Tab>
            ))}
          </TabList>
        </div>
        {INTAKE_SECTION_IDS.map((id) => {
          const Panel = PANELS[id];
          return (
            <TabPanel key={id} id={id} className={styles.panel}>
              <Panel />
            </TabPanel>
          );
        })}
      </Tabs>
    </div>
  );
}
