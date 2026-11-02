import { lazy, Suspense, useMemo, useState, type ComponentType, type LazyExoticComponent } from 'react';
import { Navigate, useParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { EmptyState, Skeleton } from '../../ui';
import { Lock } from '../../ui/icons';
import { PageHeader, useIsNarrow } from '../shell';
import { DEFAULT_SECTION, findSection, SECTIONS, type SectionId } from './sections';
import { SettingsNav } from './SettingsNav';
import { useIsAdmin } from './shared/useIsAdmin';
import { RequireAdmin } from '../../auth/RequireAdmin';
import styles from './SettingsPage.module.css';

const SECTION_COMPONENTS: Record<SectionId, LazyExoticComponent<ComponentType>> = {
  general: lazy(() => import('./general/GeneralSection')),
  account: lazy(() => import('./account/AccountSection')),
  ocr: lazy(() => import('./ocr/OcrSection')),
  users: lazy(() => import('./users/UsersSection')),
  server: lazy(() => import('./server/ServerSection')),
  'api-keys': lazy(() => import('./apiKeys/ApiKeysSection')),
  labels: lazy(() => import('./labels/LabelsSection')),
  debug: lazy(() => import('./debug/DebugSection')),
  appearance: lazy(() => import('./appearance/AppearanceSection')),
};

/** `/settings/:section?`: searchable section nav on the left, the section's setting groups on the right. */
export default function SettingsPage() {
  const { t } = useTranslation();
  const { section: param } = useParams();
  const isAdmin = useIsAdmin();
  const isNarrow = useIsNarrow();
  const [query, setQuery] = useState('');

  const visible = useMemo(() => SECTIONS.filter((s) => isAdmin || !s.adminOnly), [isAdmin]);
  const section = findSection(param ?? DEFAULT_SECTION);

  if (!section) return <Navigate to="/settings" replace />;

  const Section = SECTION_COMPONENTS[section.id];
  const content = (
    <Suspense fallback={<Skeleton lines={4} label={t('common.status.loading', 'Loading...')} />}>
      <Section />
    </Suspense>
  );
  const adminOnly = (
    <EmptyState
      headingAs="h3"
      icon={<Lock fontSize="inherit" />}
      title={t('settings.adminOnly.title', 'Admins only')}
      description={t('settings.adminOnly.description', 'Ask an administrator if you need to change these settings.')}
    />
  );

  return (
    <>
      <PageHeader title={t('settings.title', 'Settings')} />
      <div className={styles.layout}>
        <aside className={styles.aside}>
          <SettingsNav
            sections={visible}
            activeId={section.id}
            query={query}
            onQueryChange={setQuery}
            isNarrow={isNarrow}
          />
        </aside>
        <div className={styles.content}>
          <h2 className={styles.sectionTitle}>{t(section.label, section.fallback)}</h2>
          {section.adminOnly ? (
            <RequireAdmin fallback={adminOnly}>{content}</RequireAdmin>
          ) : (
            content
          )}
        </div>
      </div>
    </>
  );
}
