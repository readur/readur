import { useLayoutEffect, useRef } from 'react';
import { Button as RACButton } from 'react-aria-components';
import { useTranslation } from 'react-i18next';
import { IconButton, Kbd } from '../../ui';
import { Search } from '../../ui/icons';
import styles from './AppShell.module.css';

const SHORTCUTS = 'Meta+K Control+K';

export function isMacPlatform(): boolean {
  if (typeof navigator === 'undefined') return false;
  const nav = navigator as Navigator & { userAgentData?: { platform?: string } };
  const platform = nav.userAgentData?.platform ?? nav.platform ?? nav.userAgent ?? '';
  return /mac|iphone|ipad|ipod/i.test(platform);
}

export interface SearchTriggerProps {
  onOpen: () => void;
  compact: boolean;
}

/** Opens the command palette: a search-field lookalike, or an icon button on narrow screens. */
export function SearchTrigger({ onOpen, compact }: SearchTriggerProps) {
  const { t } = useTranslation();
  const label = t('shell.search.open', 'Search documents');
  const ref = useRef<HTMLButtonElement>(null);

  // RAC drops aria-keyshortcuts, so it is set on the DOM node directly.
  useLayoutEffect(() => {
    ref.current?.setAttribute('aria-keyshortcuts', SHORTCUTS);
  }, [compact]);

  if (compact) {
    return (
      <IconButton
        label={label}
        icon={<Search fontSize="inherit" />}
        onPress={onOpen}
        ref={ref}
        className={styles.toolIcon}
      />
    );
  }

  const mac = isMacPlatform();
  return (
    <RACButton
      className={styles.searchTrigger}
      onPress={onOpen}
      aria-label={label}
      ref={ref}
    >
      <span className={styles.searchIcon} aria-hidden="true">
        <Search fontSize="inherit" />
      </span>
      <span className={styles.searchText}>{t('shell.search.placeholder', 'Search documents…')}</span>
      <Kbd aria-hidden="true" className={styles.searchKbd}>
        {mac ? '⌘K' : 'Ctrl K'}
      </Kbd>
    </RACButton>
  );
}
