import type { ComponentType } from 'react';
import { Navigate, useParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import SettingsPage from '../../pages/SettingsPage';
import LabelsPage from '../../pages/LabelsPage';
import DebugPage from '../../pages/DebugPage';
import { PageHeader, SETTINGS_SECTIONS, SectionNav } from '../shell';

const SECTION_PAGES: Record<string, ComponentType> = {
  general: SettingsPage,
  labels: LabelsPage,
  debug: DebugPage,
};

/** Temporary Settings container: `/settings/:section?` picks one of the existing pages. */
export default function SettingsInterim() {
  const { t } = useTranslation();
  const { section = 'general' } = useParams();

  if (!(section in SECTION_PAGES)) return <Navigate to="/settings" replace />;
  const Page = SECTION_PAGES[section];

  return (
    <>
      <PageHeader title={t('shell.settings.title', 'Settings')} />
      <SectionNav
        label={t('shell.settings.sections', 'Settings sections')}
        activeId={section}
        items={SETTINGS_SECTIONS.map((s) => ({ id: s.id, label: t(s.labelKey, s.fallback), to: s.path }))}
      />
      <Page />
    </>
  );
}
