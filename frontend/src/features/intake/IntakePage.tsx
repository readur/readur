import { useLayoutEffect, useRef, type ComponentType } from 'react';
import { Navigate, useSearchParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import type { Key } from 'react-aria-components';
import { Tab, TabList, TabPanel, Tabs } from '../../ui';
import { PageHeader } from '../shell';
import { AttentionSection } from './attention/AttentionSection';
import { IgnoredSection } from './ignored/IgnoredSection';
import { sharedStyles } from './shared/parts';
import { UploadSection } from './upload/UploadSection';
import { useIntakeSummary } from './useIntakeSummary';
import styles from './Intake.module.css';

export const INTAKE_SECTION_IDS = ['upload', 'attention', 'ignored'] as const;
export type IntakeSectionId = (typeof INTAKE_SECTION_IDS)[number];
export const DEFAULT_INTAKE_SECTION: IntakeSectionId = 'upload';

const PANELS: Record<IntakeSectionId, ComponentType> = {
  upload: UploadSection,
  attention: AttentionSection,
  ignored: IgnoredSection,
};

/** Scroll the tab row sideways (never the page) so the selected tab is fully in view. */
export function revealSelectedTab(scroller: HTMLElement | null) {
  const tab = scroller?.querySelector<HTMLElement>('[role="tab"][aria-selected="true"]');
  if (!scroller || !tab) return;
  const box = scroller.getBoundingClientRect();
  const rect = tab.getBoundingClientRect();
  if (rect.left < box.left) scroller.scrollLeft -= box.left - rect.left;
  else if (rect.right > box.right) scroller.scrollLeft += rect.right - box.right;
}

/** The uploads key in `?source=` leads to the upload section. */
export function sectionForSource(source: string | null): IntakeSectionId | null {
  if (source === 'upload' || source === 'uploads') return 'upload';
  return null;
}

/**
 * Connections and the watch folder moved to their own Sources page. An old Intake link to
 * either (`?section=connections|watch`, or `?source=watch`) returns its new address.
 */
export function sourcesRedirect(params: URLSearchParams): string | null {
  const section = params.get('section');
  const source = params.get('source');
  if (source === 'watch' || section === 'watch') return '/sources?section=watch';
  if (section !== 'connections') return null;
  const next = new URLSearchParams({ section: 'connections' });
  if (source) next.set('source', source);
  return `/sources?${next.toString()}`;
}

export function parseSection(value: string | null): IntakeSectionId {
  return (INTAKE_SECTION_IDS as readonly string[]).includes(value ?? '') ? (value as IntakeSectionId) : DEFAULT_INTAKE_SECTION;
}

/** Intake: add documents, check connections, and clear what needs attention. */
export default function IntakePage() {
  const [params] = useSearchParams();
  const moved = sourcesRedirect(params);
  if (moved) return <Navigate replace to={moved} />;
  return <IntakeTabs />;
}

function IntakeTabs() {
  const { t } = useTranslation();
  const [params, setParams] = useSearchParams();
  const section = sectionForSource(params.get('source')) ?? parseSection(params.get('section'));
  const summary = useIntakeSummary(section);
  const tabScroller = useRef<HTMLDivElement>(null);

  // On a phone the tab row is wider than the screen; keep the current section's tab visible,
  // also when the row's width changes afterwards (a count badge arriving, fonts loading).
  useLayoutEffect(() => {
    const scroller = tabScroller.current;
    revealSelectedTab(scroller);
    const list = scroller?.firstElementChild;
    if (!list || typeof ResizeObserver !== 'function') return undefined;
    const observer = new ResizeObserver(() => revealSelectedTab(scroller));
    observer.observe(list);
    return () => observer.disconnect();
  }, [section]);

  const labels: Record<IntakeSectionId, string> = {
    upload: t('intake.sections.upload', 'Add documents'),
    attention: t('intake.sections.attention', 'Needs attention'),
    ignored: t('intake.sections.ignored', 'Ignored'),
  };

  const select = (key: Key) => {
    const next = parseSection(String(key));
    if (next === section) return;
    // Filters that belong to one section (e.g. the Ignored source filter) don't carry over.
    setParams({ section: next });
  };

  const connections =
    summary.connections !== null
      ? t('intake.meta.connections', '{{count}} connections', { count: summary.connections })
      : null;
  const attention =
    summary.attention !== null ? t('intake.meta.attention', '{{count}} need attention', { count: summary.attention }) : null;
  const processing =
    summary.processing !== null ? t('intake.meta.processing', '{{count}} processing', { count: summary.processing }) : null;

  // The headline states one number; the line under it carries the others, never the same one twice.
  const figure = summary.attention ? attention : connections;
  const meta = [connections, attention, processing].filter((part): part is string => Boolean(part) && part !== figure);

  return (
    <div className={styles.page}>
      <PageHeader
        title={t('intake.title', 'Intake')}
        figure={figure ?? undefined}
        meta={meta.length > 0 ? <span>{meta.join(' · ')}</span> : undefined}
      />
      <Tabs selectedKey={section} onSelectionChange={select}>
        <div ref={tabScroller} className={styles.tabScroller}>
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
