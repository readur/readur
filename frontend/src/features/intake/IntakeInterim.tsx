import type { ComponentType } from 'react';
import { useSearchParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import UploadPage from '../../pages/UploadPage';
import SourcesPage from '../../pages/SourcesPage';
import WatchFolderPage from '../../pages/WatchFolderPage';
import DocumentManagementPage from '../../pages/DocumentManagementPage';
import IgnoredFilesPage from '../../pages/IgnoredFilesPage';
import { INTAKE_SECTIONS, PageHeader, SectionNav } from '../shell';

const SECTION_PAGES: Record<string, ComponentType> = {
  upload: UploadPage,
  connections: SourcesPage,
  watch: WatchFolderPage,
  attention: DocumentManagementPage,
  ignored: IgnoredFilesPage,
};

export const DEFAULT_INTAKE_SECTION = 'upload';

/** Temporary Intake container: `?section=` picks one of the existing pages. */
export default function IntakeInterim() {
  const { t } = useTranslation();
  const [params] = useSearchParams();
  const requested = params.get('section') ?? '';
  const section = requested in SECTION_PAGES ? requested : DEFAULT_INTAKE_SECTION;
  const Page = SECTION_PAGES[section];

  return (
    <>
      <PageHeader title={t('shell.intake.title', 'Intake')} />
      <SectionNav
        label={t('shell.intake.sections', 'Intake sections')}
        activeId={section}
        items={INTAKE_SECTIONS.map((s) => ({ id: s.id, label: t(s.labelKey, s.fallback), to: s.path }))}
      />
      <Page />
    </>
  );
}
