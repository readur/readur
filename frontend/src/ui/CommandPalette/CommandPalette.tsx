import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from 'react';
import {
  Autocomplete,
  Dialog as RACDialog,
  Header,
  Menu,
  MenuItem,
  MenuSection,
  Modal,
  ModalOverlay,
  Text,
} from 'react-aria-components';
import { useTranslation } from 'react-i18next';
import { Kbd } from '../Kbd';
import { SearchField } from '../SearchField';
import type { CommandItem, CommandSource } from './types';
import { useSourceSearch } from './useSourceSearch';
import styles from './CommandPalette.module.css';

export interface CommandPaletteProps {
  isOpen: boolean;
  onOpenChange: (isOpen: boolean) => void;
  sources: CommandSource[];
  placeholder?: string;
}

/**
 * Centred search-anything dialog. Sources are queried with a 150ms debounce and results are
 * grouped by source. ↑/↓ move, Enter selects, Esc closes. Only results for the text currently in
 * the field are shown (and selectable); while they load, the list shows a loading state.
 */
export function CommandPalette({ isOpen, onOpenChange, sources, placeholder }: CommandPaletteProps) {
  const { t } = useTranslation();
  const [query, setQuery] = useState('');
  const { results, isLoading } = useSourceSearch(sources, query, isOpen);
  const [active, setActive] = useState<CommandItem | null>(null);

  useEffect(() => {
    if (!isOpen) setQuery('');
  }, [isOpen]);

  const frameRef = useRef<HTMLDivElement>(null);
  const itemsByKey = useMemo(() => {
    const map = new Map<string, CommandItem>();
    for (const { source, items } of results) for (const item of items) map.set(`${source.id}:${item.id}`, item);
    return map;
  }, [results]);

  // The palette uses virtual focus (the caret stays in the field), so follow React Aria's
  // data-focused marker on the result list to know which result to preview.
  useEffect(() => {
    const frame = frameRef.current;
    if (!frame) return;
    const sync = () => {
      const key = frame.querySelector('[role="menuitem"][data-focused]')?.getAttribute('data-key');
      setActive(key ? (itemsByKey.get(key) ?? null) : null);
    };
    sync();
    const observer = new MutationObserver(sync);
    observer.observe(frame, { subtree: true, attributes: true, attributeFilter: ['data-focused'], childList: true });
    return () => observer.disconnect();
  }, [itemsByKey, isOpen]);

  // Esc always closes, even when the field has text (the field would otherwise clear first).
  const onKeyDownCapture = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      onOpenChange(false);
    }
  };

  const label = t('ui.commandPalette.label', 'Command palette');
  const hasResults = results.length > 0;
  const hasPreview = results.some(({ items }) => items.some((item) => item.preview));

  return (
    <ModalOverlay className={styles.overlay} isOpen={isOpen} onOpenChange={onOpenChange} isDismissable>
      <Modal className={styles.modal}>
        <RACDialog className={styles.dialog} aria-label={label}>
          <div ref={frameRef} className={styles.frame} onKeyDownCapture={onKeyDownCapture} data-has-preview={hasPreview || undefined}>
            <Autocomplete inputValue={query} onInputChange={setQuery}>
              <SearchField
                autoFocus
                aria-label={t('ui.commandPalette.search', 'Search')}
                placeholder={placeholder ?? t('ui.commandPalette.placeholder', 'Search documents, labels, pages…')}
                className={styles.search}
              />
              <div className={styles.status} role="status" aria-live="polite">
                {isLoading ? t('ui.commandPalette.loading', 'Searching…') : null}
              </div>
              <Menu
                className={styles.menu}
                aria-label={t('ui.commandPalette.results', 'Results')}
                renderEmptyState={() => (
                  <p className={styles.empty}>
                    {isLoading
                      ? t('ui.commandPalette.loading', 'Searching…')
                      : query.trim() === ''
                        ? t('ui.commandPalette.hint', 'Type to search')
                        : t('ui.commandPalette.noResults', 'No results')}
                  </p>
                )}
              >
                {hasResults
                  ? results.map(({ source, items }) => (
                      <MenuSection key={source.id} id={source.id} className={styles.section}>
                        <Header className={styles.heading}>{source.label}</Header>
                        {items.map((item) => (
                          <MenuItem
                            key={item.id}
                            id={`${source.id}:${item.id}`}
                            textValue={item.title}
                            className={styles.item}
                            onAction={() => {
                              item.onSelect();
                              onOpenChange(false);
                            }}
                          >
                            {/* Always present, so titles line up whether or not an item has an icon. */}
                            <span className={styles.icon} aria-hidden="true">
                              {item.icon ?? null}
                            </span>
                            <span className={styles.text}>
                              <Text slot="label" className={styles.title}>
                                {item.title}
                              </Text>
                              {item.subtitle ? (
                                <Text slot="description" className={styles.subtitle}>
                                  {item.subtitle}
                                </Text>
                              ) : null}
                            </span>
                          </MenuItem>
                        ))}
                      </MenuSection>
                    ))
                  : []}
              </Menu>
            </Autocomplete>
            {hasPreview ? (
              <aside className={styles.preview}>
                {active?.preview ?? null}
              </aside>
            ) : null}
            <div className={styles.footer} aria-hidden="true">
              <span>
                <Kbd>↑↓</Kbd> {t('ui.palette.navigate', 'navigate')}
              </span>
              <span>
                <Kbd>↵</Kbd> {t('ui.palette.open', 'open')}
              </span>
            </div>
          </div>
        </RACDialog>
      </Modal>
    </ModalOverlay>
  );
}
