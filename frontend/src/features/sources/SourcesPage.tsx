import { useEffect, useState, type ComponentType } from 'react';
import { useSearchParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import type { Key } from 'react-aria-components';
import { Tab, TabList, TabPanel, Tabs } from '../../ui';
import { sourcesService } from '../../services/api';
import { PageHeader } from '../shell';
import { ConnectionsSection } from '../intake/connections/ConnectionsSection';
import { WatchSection } from '../intake/watch/WatchSection';
import styles from '../intake/Intake.module.css';

export const SOURCES_SECTION_IDS = ['connections', 'watch'] as const;
export type SourcesSectionId = (typeof SOURCES_SECTION_IDS)[number];

const PANELS: Record<SourcesSectionId, ComponentType> = {
  connections: ConnectionsSection,
  watch: WatchSection,
};

export function parseSourcesSection(value: string | null): SourcesSectionId {
  return (SOURCES_SECTION_IDS as readonly string[]).includes(value ?? '') ? (value as SourcesSectionId) : 'connections';
}

/** Number of configured connections for the header; null while loading or when it fails. */
function useConnectionCount(): number | null {
  const [count, setCount] = useState<number | null>(null);
  useEffect(() => {
    let alive = true;
    sourcesService
      .list()
      .then((res) => alive && setCount(Array.isArray(res.data) ? res.data.length : null))
      .catch(() => alive && setCount(null));
    return () => {
      alive = false;
    };
  }, []);
  return count;
}

/** Sources: the connections that sync documents in, and the watch folder. */
export default function SourcesPage() {
  const { t } = useTranslation();
  const [params, setParams] = useSearchParams();
  const section = params.get('source') === 'watch' ? 'watch' : parseSourcesSection(params.get('section'));
  const count = useConnectionCount();

  const labels: Record<SourcesSectionId, string> = {
    connections: t('sourcesPage.sections.connections', 'Connections'),
    watch: t('sourcesPage.sections.watch', 'Watch folder'),
  };

  const select = (key: Key) => {
    const next = parseSourcesSection(String(key));
    if (next !== section) setParams({ section: next });
  };

  return (
    <div className={styles.page}>
      <PageHeader
        title={t('sourcesPage.title', 'Sources')}
        figure={count !== null ? t('sourcesPage.meta.connections', '{{count}} connections', { count }) : undefined}
      />
      <Tabs selectedKey={section} onSelectionChange={select}>
        <div className={styles.tabScroller}>
          <TabList aria-label={t('sourcesPage.sections.label', 'Source sections')} className={styles.tabList}>
            {SOURCES_SECTION_IDS.map((id) => (
              <Tab key={id} id={id}>
                {labels[id]}
              </Tab>
            ))}
          </TabList>
        </div>
        {SOURCES_SECTION_IDS.map((id) => {
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
