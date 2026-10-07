import type { ReactNode } from 'react';

export interface CommandItem {
  id: string;
  title: string;
  subtitle?: string;
  icon?: ReactNode;
  /** Shown in the preview pane while this result is focused (e.g. a thumbnail and OCR snippet). */
  preview?: ReactNode;
  onSelect: () => void;
}

export interface CommandSource {
  id: string;
  /** Group heading shown above this source's results. */
  label: string;
  search: (query: string) => Promise<CommandItem[]>;
  /**
   * Also query this source with an empty string (for example to list default commands when the
   * palette opens). Off by default, so API-backed sources are not hit before the user types.
   */
  searchesEmpty?: boolean;
}
