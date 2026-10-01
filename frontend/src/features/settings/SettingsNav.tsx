import { useMemo } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import type { Key } from 'react-aria-components';
import { SearchField, Select, SelectItem } from '../../ui';
import { sectionPath, type SectionDef, type SectionId } from './sections';
import styles from './SettingsPage.module.css';

export interface SettingsNavProps {
  sections: readonly SectionDef[];
  activeId: SectionId;
  query: string;
  onQueryChange: (q: string) => void;
  /** Narrow screens replace the link list with a Select. */
  isNarrow: boolean;
}

interface Match {
  section: SectionDef;
  title: string;
  /** Settings inside the section whose label matches (empty when only the title matched). */
  hits: { group: string; label: string }[];
}

/** Sections (and the settings within them) whose label contains the query, case-insensitively. */
export function useSectionMatches(sections: readonly SectionDef[], query: string): Match[] {
  const { t } = useTranslation();
  return useMemo(() => {
    const q = query.trim().toLocaleLowerCase();
    return sections.flatMap((section) => {
      const title = t(section.label, section.fallback);
      if (!q) return [{ section, title, hits: [] }];
      const seen = new Set<string>();
      const hits = section.entries
        .map((entry) => ({ group: entry.group, label: t(entry.label, entry.fallback) }))
        .filter((hit) => {
          if (seen.has(hit.label) || !hit.label.toLocaleLowerCase().includes(q)) return false;
          seen.add(hit.label);
          return true;
        });
      const titleMatch = title.toLocaleLowerCase().includes(q);
      return titleMatch || hits.length > 0 ? [{ section, title, hits }] : [];
    });
  }, [sections, query, t]);
}

export function SettingsNav({ sections, activeId, query, onQueryChange, isNarrow }: SettingsNavProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const matches = useSectionMatches(sections, query);
  const searching = query.trim() !== '';

  const hitList = (m: Match) =>
    m.hits.length > 0 ? (
      <ul className={styles.hits}>
        {m.hits.map((hit) => (
          <li key={`${hit.group}:${hit.label}`}>
            <Link className={styles.hit} to={`${sectionPath(m.section.id)}#${hit.group}`}>
              {hit.label}
            </Link>
          </li>
        ))}
      </ul>
    ) : null;

  return (
    <nav className={styles.nav} aria-label={t('settings.nav.label', 'Settings sections')}>
      <SearchField
        aria-label={t('settings.nav.search', 'Search settings')}
        placeholder={t('settings.nav.searchPlaceholder', 'Filter by name')}
        value={query}
        onChange={onQueryChange}
      />
      {isNarrow ? (
        <Select
          label={t('settings.nav.section', 'Section')}
          selectedKey={activeId}
          onSelectionChange={(key: Key | null) => {
            if (key !== null) navigate(sectionPath(String(key) as SectionId));
          }}
        >
          {sections.map((s) => (
            <SelectItem key={s.id} id={s.id} textValue={t(s.label, s.fallback)}>
              {t(s.label, s.fallback)}
            </SelectItem>
          ))}
        </Select>
      ) : null}
      {!isNarrow || searching ? (
        <ul className={styles.links}>
          {matches.map((m) => (
            <li key={m.section.id}>
              <Link
                className={styles.link}
                to={sectionPath(m.section.id)}
                aria-current={m.section.id === activeId ? 'page' : undefined}
              >
                {m.title}
              </Link>
              {hitList(m)}
            </li>
          ))}
        </ul>
      ) : null}
      {searching ? (
        <p className={styles.searchStatus} role="status">
          {matches.length === 0
            ? t('settings.nav.noMatches', 'No settings match “{{query}}”', { query: query.trim() })
            : t('settings.nav.matches', '{{count}} sections match', { count: matches.length })}
        </p>
      ) : null}
    </nav>
  );
}
