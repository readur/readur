import { useMemo, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import { useDocumentDrawer } from '../document/drawer/DocumentDrawerContext';
import { useTranslation } from 'react-i18next';
import type { CommandItem, CommandSource } from '../../ui';
import { Description, History, Search } from '../../ui/icons';
import { documentService } from '../../services/api';
import { DESTINATIONS, INTAKE_SECTIONS, SETTINGS_SECTIONS, SOURCES_SECTIONS } from './destinations';
import { readRecentSearches, saveRecentSearch } from './recentSearches';

export const DOCUMENT_RESULT_LIMIT = 8;

const matches = (text: string, query: string) =>
  text.toLocaleLowerCase().includes(query.trim().toLocaleLowerCase());

const searchPath = (q: string) => `/search?q=${encodeURIComponent(q)}`;

/** The shell's command palette sources: navigation, documents, recent searches. */
export function usePaletteSources(): CommandSource[] {
  const navigate = useNavigate();
  const { t } = useTranslation();
  // Read at selection time: the drawer opens over whatever page is showing by then.
  const openDrawer = useDocumentDrawer();
  const drawer = useRef(openDrawer);
  drawer.current = openDrawer;

  return useMemo<CommandSource[]>(() => {
    const intakeTitle = t('shell.nav.intake', 'Intake');
    const sourcesTitle = t('shell.nav.sources', 'Sources');
    const settingsTitle = t('shell.nav.settings', 'Settings');

    const navItems: CommandItem[] = [
      ...DESTINATIONS.map((d) => ({
        id: d.id,
        title: t(d.labelKey, d.fallback),
        icon: d.icon,
        onSelect: () => navigate(d.path),
      })),
      ...INTAKE_SECTIONS.map((s) => ({
        id: `intake-${s.id}`,
        title: t(s.labelKey, s.fallback),
        subtitle: intakeTitle,
        onSelect: () => navigate(s.path),
      })),
      ...SOURCES_SECTIONS.map((s) => ({
        id: `sources-${s.id}`,
        title: t(s.labelKey, s.fallback),
        subtitle: sourcesTitle,
        onSelect: () => navigate(s.path),
      })),
      ...SETTINGS_SECTIONS.map((s) => ({
        id: `settings-${s.id}`,
        title: t(s.labelKey, s.fallback),
        subtitle: settingsTitle,
        onSelect: () => navigate(s.path),
      })),
    ];

    const navigation: CommandSource = {
      id: 'navigation',
      label: t('shell.palette.navigation', 'Go to'),
      searchesEmpty: true,
      search: async (q) =>
        q.trim() ? navItems.filter((i) => matches(i.title, q) || matches(i.subtitle ?? '', q)) : navItems,
    };

    const documents: CommandSource = {
      id: 'documents',
      label: t('shell.palette.documents', 'Documents'),
      search: async (q) => {
        const query = q.trim();
        if (!query) return [];
        const showAll: CommandItem = {
          id: 'show-all',
          title: t('shell.palette.showAll', { query, defaultValue: 'Show all results for “{{query}}”' }),
          icon: <Search fontSize="inherit" />,
          onSelect: () => {
            saveRecentSearch(query);
            navigate(searchPath(query));
          },
        };
        let docs: CommandItem[] = [];
        try {
          const res = await documentService.enhancedSearch({
            query,
            limit: DOCUMENT_RESULT_LIMIT,
            include_snippets: true,
            snippet_length: 80,
            search_mode: query.length < 4 ? 'fuzzy' : 'simple',
          });
          docs = (res.data?.documents ?? []).slice(0, DOCUMENT_RESULT_LIMIT).map((doc) => ({
            id: doc.id,
            title: doc.original_filename || doc.filename,
            subtitle: doc.snippets?.[0]?.text,
            icon: <Description fontSize="inherit" />,
            onSelect: () => {
              saveRecentSearch(query);
              drawer.current.open(doc.id);
            },
          }));
        } catch {
          /* search failed: still offer the full results page */
        }
        return [...docs, showAll];
      },
    };

    const recent: CommandSource = {
      id: 'recent',
      label: t('shell.palette.recent', 'Recent searches'),
      searchesEmpty: true,
      search: async (q) =>
        readRecentSearches()
          .filter((s) => !q.trim() || matches(s, q))
          .map((s) => ({
            id: s,
            title: s,
            icon: <History fontSize="inherit" />,
            onSelect: () => {
              saveRecentSearch(s);
              navigate(searchPath(s));
            },
          })),
    };

    return [navigation, documents, recent];
  }, [navigate, t]);
}
